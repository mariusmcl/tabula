//! Tabula blockchain node.
//!
//! A P2P PoW blockchain with persistence, mining, and networking.

use axum::{
    extract::{State, Query},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tracing::{info, warn, debug};

use chain::{Chain, SimpleExecutor, ContractExecutor};
use consensus::{create_genesis_block, create_block_template, DifficultyConfig};
use contracts::KbContract;
use crypto::{Keypair, hex_encode};
use mempool::Mempool;
use network::{NetworkConfig, NetworkEvent, start_network, PeerId};
use persistence::Database;
use staking;
use store::KV;
use subreddit::SubredditId;
use types::{Block, Hash, SignedTransaction, Transaction, TxType};

/// Tabula blockchain node
#[derive(Parser)]
#[command(name = "tabula-node")]
#[command(about = "A PoW blockchain node for the Tabula knowledge base")]
struct Args {
    /// Data directory for blockchain storage
    #[arg(short, long, default_value = "./data")]
    data_dir: String,

    /// Port to listen on (0 = random)
    #[arg(short, long, default_value = "0")]
    port: u16,

    /// Enable mining
    #[arg(short, long)]
    mine: bool,

    /// Create genesis block (for new chain)
    #[arg(long)]
    genesis: bool,

    /// Seed KB demo data at genesis (only works with --genesis)
    #[arg(long)]
    seed_genesis: bool,

    /// Mining key seed (32 hex chars, for deterministic key)
    #[arg(long)]
    key_seed: Option<String>,

    /// Bootstrap peer addresses (multiaddr format)
    #[arg(long)]
    bootstrap: Vec<String>,

    /// Disable mDNS peer discovery
    #[arg(long)]
    no_mdns: bool,

    /// Subreddits to store (comma-separated hex IDs, or "all" for full node)
    #[arg(long, default_value = "all")]
    subreddits: String,

    /// HTTP API port (0 = disabled)
    #[arg(long, default_value = "8080")]
    api_port: u16,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the node (default)
    Run,
    /// Query the knowledge base
    Query {
        /// Query string (e.g., food:"Salad".EnergyPerServing)
        query: String,
    },
    /// Submit a transaction
    Submit {
        /// Contract ID
        #[arg(long, default_value = "1")]
        contract: u32,
        /// Method ID
        #[arg(long)]
        method: u32,
        /// Calldata (as UTF-8 string)
        #[arg(long, default_value = "")]
        data: String,
    },
    /// Show chain status
    Status,
    /// Seed demo data (submits a transaction to seed the KB)
    Seed,
    /// Put data into a subreddit (anyone can add data!)
    Put {
        /// Subreddit name or "legacy" for default
        #[arg(long, default_value = "legacy")]
        subreddit: String,
        /// Entity type (e.g., "person", "place", "thing", "event")
        #[arg(long, short = 't')]
        entity_type: String,
        /// Entity key (e.g., "Elon_Musk", "Paris", "Bitcoin")
        #[arg(long, short = 'k')]
        key: String,
        /// Property name (e.g., "description", "founded", "population")
        #[arg(long, short = 'p')]
        property: String,
        /// Value to store
        #[arg(long, short = 'v')]
        value: String,
    },
    /// Create a new subreddit
    CreateSub {
        /// Subreddit name (lowercase, alphanumeric + underscore)
        name: String,
        /// Description
        #[arg(long, default_value = "")]
        description: String,
        /// Fee amount (tokens to burn)
        #[arg(long, default_value = "100")]
        fee: u64,
    },
    /// Get data from a subreddit
    Get {
        /// Subreddit name or "legacy"
        #[arg(long, default_value = "legacy")]
        subreddit: String,
        /// Entity type
        #[arg(long, short = 't')]
        entity_type: String,
        /// Entity key
        #[arg(long, short = 'k')]
        key: String,
        /// Property name
        #[arg(long, short = 'p')]
        property: String,
    },
    /// List all data in the chain
    List {
        /// Limit number of entries
        #[arg(long, default_value = "50")]
        limit: usize,
    },
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

// ============================================================================
// Simple Entity Type & Property Registry
// ============================================================================

/// Convert entity type name to ID. Anyone can define their own types!
fn entity_type_to_id(name: &str) -> u32 {
    // Common types have fixed IDs for interoperability
    match name.to_lowercase().as_str() {
        "food" => 1,
        "country" => 2,
        "person" => 3,
        "place" => 4,
        "thing" => 5,
        "event" => 6,
        "organization" => 7,
        "concept" => 8,
        "artwork" => 9,
        "software" => 10,
        // For custom types, hash the name
        _ => {
            let hash = crypto::sha256(name.to_lowercase().as_bytes());
            u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) | 0x8000_0000
        }
    }
}

/// Convert property name to ID
fn property_to_id(name: &str) -> u32 {
    match name.to_lowercase().as_str() {
        // Common properties
        "name" => 1,
        "description" => 2,
        "created" => 3,
        "modified" => 4,
        "author" => 5,
        "source" => 6,
        "url" => 7,
        "image" => 8,
        "tags" => 9,
        "category" => 10,
        // Domain-specific
        "population" => 100,
        "capital" => 101,
        "area" => 102,
        "currency" => 103,
        "language" => 104,
        "founded" => 105,
        "location" => 106,
        "birthdate" => 107,
        "occupation" => 108,
        "nationality" => 109,
        // For custom properties, hash the name
        _ => {
            let hash = crypto::sha256(name.to_lowercase().as_bytes());
            u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) | 0x8000_0000
        }
    }
}

/// Parse subreddit argument to SubredditId
fn parse_subreddit_arg(name: &str, height: u64, creator: &[u8; 32]) -> SubredditId {
    match name.to_lowercase().as_str() {
        "legacy" => SubredditId::LEGACY,
        "global" => SubredditId::GLOBAL,
        _ => SubredditId::derive(creator, name, height),
    }
}

fn setup_logging() {
    tracing_subscriber::fmt()
        .with_target(false)
        .with_thread_ids(false)
        .with_level(true)
        .init();
}

fn get_keypair(seed: Option<&str>) -> Keypair {
    match seed {
        Some(s) => {
            let mut seed_bytes = [0u8; 32];
            let decoded = crypto::hex_decode(s).unwrap_or_else(|_| {
                // If not valid hex, hash the string
                crypto::sha256(s.as_bytes()).to_vec()
            });
            let len = decoded.len().min(32);
            seed_bytes[..len].copy_from_slice(&decoded[..len]);
            Keypair::from_seed(&seed_bytes)
        }
        None => Keypair::generate(),
    }
}

fn create_executor() -> SimpleExecutor {
    let mut executor = SimpleExecutor::new();
    executor.register(1, Box::new(KbContract::new()));
    executor
}

fn initialize_chain(
    db: &Database,
    config: DifficultyConfig,
    force_genesis: bool,
    seed_genesis: bool,
) -> Result<Chain, Box<dyn std::error::Error>> {
    // Check if we have existing data
    if !force_genesis {
        if let Some(tip) = db.get_tip()? {
            info!("Loading existing chain from database...");
            let blocks = db.get_all_blocks()?;
            let state = db.load_state()?.unwrap_or_else(KV::new);
            let nonces = db.get_all_nonces()?;

            let chain = Chain::from_state(state, blocks, tip, nonces, config);
            info!("Loaded chain at height {}", chain.height());
            return Ok(chain);
        }
    }

    // Create initial state (optionally seeded)
    let mut initial_state = KV::new();
    if seed_genesis {
        info!("Seeding KB demo data into genesis state...");
        let mut executor = create_executor();
        executor.execute(&mut initial_state, 1, 1, b"")?;
        info!("Seeded {} state keys", initial_state.keys_count());
    }

    let state_root = initial_state.ordered_merkle_root();

    // Create genesis
    info!("Creating genesis block...");
    let genesis = create_genesis_block(state_root, config.initial_difficulty, current_timestamp());
    info!("Genesis block: {}", hex_encode(&genesis.hash()[..8]));

    // Store genesis
    db.put_block(&genesis)?;
    db.set_tip(&genesis.hash())?;
    db.set_height(0)?;
    db.save_state(&initial_state)?;
    db.flush()?;

    // Create chain with initial state
    let mut chain = Chain::new(genesis, config)?;
    // Replace state with the seeded one
    *chain.state_mut() = initial_state;

    Ok(chain)
}

fn mine_next_block(
    chain: &Chain,
    mempool: &mut Mempool,
    executor: &mut SimpleExecutor,
    miner: &[u8; 32],
) -> Result<Block, Box<dyn std::error::Error>> {
    let tip = chain.tip_header().unwrap();

    // Get pending transactions
    let pending_txs = mempool.get_pending(100);
    info!("Mining block {} with {} transactions...", tip.height + 1, pending_txs.len());

    // Compute expected state root (including block reward for miner)
    let state_root = if pending_txs.is_empty() && miner == &[0u8; 32] {
        chain.state_root()
    } else {
        chain.speculative_execute(&pending_txs, miner, executor)?
    };

    // Ensure timestamp is strictly greater than parent
    let timestamp = current_timestamp().max(tip.timestamp + 1);

    // Create block template with miner for v3
    let mut block = consensus::create_block_template_v3(
        tip,
        pending_txs,
        state_root,
        chain.current_difficulty(),
        timestamp,
        *miner,
    );

    // Mine!
    let start = std::time::Instant::now();
    consensus::mine_block_unlimited(&mut block.header);
    let elapsed = start.elapsed();

    info!(
        "Mined block {} in {:?} (nonce: {}, hash: {})",
        block.header.height,
        elapsed,
        block.header.nonce,
        hex_encode(&block.hash()[..8])
    );

    Ok(block)
}

fn apply_block(
    chain: &mut Chain,
    block: Block,
    mempool: &mut Mempool,
    executor: &mut SimpleExecutor,
    db: &Database,
) -> Result<Hash, Box<dyn std::error::Error>> {
    let tx_hashes: Vec<Hash> = block.transactions.iter().map(|tx| tx.hash()).collect();

    // Apply block to chain
    let hash = chain.apply_block(block.clone(), executor)?;

    // Remove included transactions from mempool
    mempool.remove(&tx_hashes);

    // Update minimum nonces in mempool
    for tx in &block.transactions {
        mempool.set_min_nonce(tx.sender(), tx.tx.nonce + 1);
    }

    // Persist
    db.put_block(&block)?;
    db.set_tip(&hash)?;
    db.set_height(chain.height())?;
    db.save_state(chain.state())?;
    db.save_all_nonces(chain.account_nonces())?;
    db.flush()?;

    info!(
        "Applied block {} (state root: {})",
        chain.height(),
        hex_encode(&chain.state_root()[..8])
    );

    Ok(hash)
}

fn run_query(chain: &Chain, query: &str) {
    use entity::Value;

    let mut executor = create_executor();
    let mut state = chain.state().clone();

    // Execute query (method 2)
    let result = executor.execute(&mut state, 1, 2, query.as_bytes());

    match result {
        Ok(data) if !data.is_empty() => {
            let v = entity::codec::decode(&data);
            match v {
                Value::Quantity(qty) => println!("{} -> {}", query, qty),
                Value::Integer(i) => println!("{} -> {}", query, i),
                Value::Text(s) => println!("{} -> {}", query, s),
                Value::EntityRef(r) => println!("{} -> {}", query, r),
                _ => println!("{} -> {:?}", query, v),
            }
        }
        Ok(_) => println!("{} -> (no result)", query),
        Err(e) => println!("{} -> error: {}", query, e),
    }
}

fn show_status(chain: &Chain, mempool: &Mempool, peer_count: usize) {
    println!("=== Tabula Blockchain Status ===");
    println!("Height:      {}", chain.height());

    if let Some(tip) = chain.tip() {
        println!("Tip hash:    {}", hex_encode(&tip.hash()[..16]));
        println!("Timestamp:   {}", tip.header.timestamp);
        println!("Difficulty:  {:016x}", tip.header.difficulty);
        println!("Nonce:       {}", tip.header.nonce);
    }

    println!("State root:  {}", hex_encode(&chain.state_root()[..16]));
    println!("State keys:  {}", chain.state().keys_count());
    println!("Mempool:     {} pending txs", mempool.len());
    println!("Peers:       {}", peer_count);
}

fn submit_transaction(
    chain: &Chain,
    mempool: &mut Mempool,
    keypair: &Keypair,
    contract_id: u32,
    method: u32,
    calldata: &[u8],
) -> Result<SignedTransaction, Box<dyn std::error::Error>> {
    let sender = keypair.public_key();
    let nonce = chain.get_nonce(&sender);

    let tx = Transaction::new(
        TxType::ContractCall {
            contract_id,
            method,
            calldata: calldata.to_vec(),
        },
        nonce,
    );

    let signing_hash = tx.signing_hash();
    let signature = keypair.sign(&signing_hash);
    let signed_tx = SignedTransaction::new(tx, signature, sender);

    let hash = mempool.add(signed_tx.clone())?;
    info!("Transaction submitted: {}", hex_encode(&hash[..8]));

    Ok(signed_tx)
}

/// Handle a block received from the network.
fn handle_received_block(
    block: Block,
    from: PeerId,
    chain: &mut Chain,
    mempool: &mut Mempool,
    executor: &mut SimpleExecutor,
    db: &Database,
) -> Result<bool, Box<dyn std::error::Error>> {
    let block_hash = block.hash();
    let height = block.header.height;

    // Check if we already have this block
    if chain.has_block(&block_hash) {
        debug!("Already have block {} from {}", hex_encode(&block_hash[..8]), from);
        return Ok(false);
    }

    // Check if it extends our chain
    let our_height = chain.height();
    if height != our_height + 1 {
        debug!(
            "Block {} at height {} doesn't extend our chain (height {})",
            hex_encode(&block_hash[..8]),
            height,
            our_height
        );
        // Could be a fork or we're behind - for now just ignore
        return Ok(false);
    }

    // Validate and apply block
    info!("Received new block {} at height {} from {}", hex_encode(&block_hash[..8]), height, from);

    match apply_block(chain, block, mempool, executor, db) {
        Ok(_) => Ok(true),
        Err(e) => {
            warn!("Failed to apply block from {}: {}", from, e);
            Ok(false)
        }
    }
}

/// Handle a transaction received from the network.
fn handle_received_transaction(
    tx: SignedTransaction,
    from: PeerId,
    mempool: &mut Mempool,
) -> bool {
    let tx_hash = tx.hash();

    if mempool.contains(&tx_hash) {
        debug!("Already have transaction {} from {}", hex_encode(&tx_hash[..8]), from);
        return false;
    }

    match mempool.add(tx) {
        Ok(_) => {
            info!("Added transaction {} from {}", hex_encode(&tx_hash[..8]), from);
            true
        }
        Err(e) => {
            debug!("Rejected transaction from {}: {}", from, e);
            false
        }
    }
}

// ============================================================================
// HTTP API
// ============================================================================

/// Shared application state for HTTP handlers
struct AppState {
    chain: RwLock<Chain>,
    mempool: RwLock<Mempool>,
    stored_subreddits: Vec<SubredditId>,
    keypair: Keypair,
}

#[derive(Serialize)]
struct StatusResponse {
    height: u64,
    tip_hash: String,
    state_root: String,
    state_keys: usize,
    mempool_size: usize,
    stored_subreddits: Vec<String>,
}

#[derive(Serialize)]
struct QueryResponse {
    query: String,
    result: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct QueryRequest {
    query: String,
}

#[derive(Serialize)]
struct SubredditInfo {
    id: String,
    name: Option<String>,
    stored: bool,
}

#[derive(Serialize)]
struct SubredditsResponse {
    subreddits: Vec<SubredditInfo>,
}

#[derive(Deserialize)]
struct PutRequest {
    subreddit: Option<String>,
    entity_type: String,
    key: String,
    property: String,
    value: String,
}

#[derive(Serialize)]
struct TxResponse {
    tx_hash: String,
    subreddit: String,
    entity_type: u32,
    key: String,
    property: u32,
    mempool_size: usize,
}

#[derive(Deserialize)]
struct TransferRequest {
    to: String,      // hex-encoded 32-byte public key
    amount: u64,
}

async fn api_status(State(state): State<Arc<AppState>>) -> Json<StatusResponse> {
    let chain = state.chain.read().await;
    let mempool = state.mempool.read().await;

    let tip = chain.tip();
    let tip_hash = tip.map(|b| hex_encode(&b.hash())).unwrap_or_default();

    Json(StatusResponse {
        height: chain.height(),
        tip_hash,
        state_root: hex_encode(&chain.state_root()),
        state_keys: chain.state().keys_count(),
        mempool_size: mempool.len(),
        stored_subreddits: state.stored_subreddits.iter()
            .map(|s| s.to_string())
            .collect(),
    })
}

async fn api_query(
    State(state): State<Arc<AppState>>,
    Json(req): Json<QueryRequest>,
) -> Json<QueryResponse> {
    use entity::Value;

    let chain = state.chain.read().await;
    let mut executor = create_executor();
    let mut kv_state = chain.state().clone();

    let result = executor.execute(&mut kv_state, 1, 2, req.query.as_bytes());

    match result {
        Ok(data) if !data.is_empty() => {
            let v = entity::codec::decode(&data);
            let result_str = match v {
                Value::Quantity(qty) => format!("{}", qty),
                Value::Integer(i) => format!("{}", i),
                Value::Text(s) => s,
                Value::EntityRef(r) => format!("{}", r),
                _ => format!("{:?}", v),
            };
            Json(QueryResponse {
                query: req.query,
                result: Some(result_str),
                error: None,
            })
        }
        Ok(_) => Json(QueryResponse {
            query: req.query,
            result: None,
            error: Some("No result".to_string()),
        }),
        Err(e) => Json(QueryResponse {
            query: req.query,
            result: None,
            error: Some(e.to_string()),
        }),
    }
}

async fn api_subreddits(State(state): State<Arc<AppState>>) -> Json<SubredditsResponse> {
    let subreddits: Vec<SubredditInfo> = state.stored_subreddits.iter()
        .map(|id| SubredditInfo {
            id: hex_encode(id.as_bytes()),
            name: if id.is_legacy() {
                Some("legacy".to_string())
            } else if id.is_global() {
                Some("global".to_string())
            } else {
                None
            },
            stored: true,
        })
        .collect();

    Json(SubredditsResponse { subreddits })
}

/// Submit a put transaction via HTTP API
async fn api_put(
    State(state): State<Arc<AppState>>,
    Json(req): Json<PutRequest>,
) -> Result<Json<TxResponse>, (StatusCode, String)> {
    let chain = state.chain.read().await;
    let mut mempool = state.mempool.write().await;

    let sender = state.keypair.public_key();
    let chain_nonce = chain.get_nonce(&sender);
    let nonce = mempool.next_nonce(&sender, chain_nonce);
    let subreddit_name = req.subreddit.unwrap_or_else(|| "legacy".to_string());
    let sub_id = parse_subreddit_arg(&subreddit_name, chain.height(), &sender);
    let entity_type_id = entity_type_to_id(&req.entity_type);
    let property_id = property_to_id(&req.property);

    let tx = Transaction::new(
        TxType::SubredditPut {
            subreddit: sub_id,
            entity_type: entity_type_id,
            entity_key: req.key.clone(),
            property: property_id,
            value: req.value.as_bytes().to_vec(),
        },
        nonce,
    );

    let signing_hash = tx.signing_hash();
    let signature = state.keypair.sign(&signing_hash);
    let signed_tx = SignedTransaction::new(tx, signature, sender);

    let hash = mempool.add(signed_tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    Ok(Json(TxResponse {
        tx_hash: hex_encode(&hash),
        subreddit: subreddit_name,
        entity_type: entity_type_id,
        key: req.key,
        property: property_id,
        mempool_size: mempool.len(),
    }))
}

/// Get data from chain state via HTTP API
async fn api_get(
    State(state): State<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    let chain = state.chain.read().await;
    let sender = state.keypair.public_key();

    let subreddit = params.get("subreddit").map(|s| s.as_str()).unwrap_or("legacy");
    let entity_type = params.get("type")
        .ok_or((StatusCode::BAD_REQUEST, "Missing 'type' parameter".to_string()))?;
    let key = params.get("key")
        .ok_or((StatusCode::BAD_REQUEST, "Missing 'key' parameter".to_string()))?;
    let property = params.get("property")
        .ok_or((StatusCode::BAD_REQUEST, "Missing 'property' parameter".to_string()))?;

    let sub_id = parse_subreddit_arg(subreddit, chain.height(), &sender);
    let entity_type_id = entity_type_to_id(entity_type);
    let property_id = property_to_id(property);

    let storage_key = subreddit::prefixed_key(&sub_id, entity_type_id, key, property_id);

    match chain.state().get(&storage_key) {
        Some(value) => {
            let value_str = String::from_utf8_lossy(&value).to_string();
            Ok(Json(serde_json::json!({
                "subreddit": subreddit,
                "type": entity_type,
                "key": key,
                "property": property,
                "value": value_str
            })))
        }
        None => Ok(Json(serde_json::json!({
            "subreddit": subreddit,
            "type": entity_type,
            "key": key,
            "property": property,
            "value": null
        }))),
    }
}

/// POST /api/transfer - Transfer tokens to another account
async fn api_transfer(
    State(state): State<Arc<AppState>>,
    Json(req): Json<TransferRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    // Parse destination address
    let to_bytes = crypto::hex_decode(&req.to)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid 'to' address: must be hex".to_string()))?;
    if to_bytes.len() != 32 {
        return Err((StatusCode::BAD_REQUEST, "Invalid 'to' address: must be 32 bytes".to_string()));
    }
    let mut to = [0u8; 32];
    to.copy_from_slice(&to_bytes);

    // Create transfer transaction
    let tx = Transaction {
        nonce: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos() as u64,
        chain_id: 1, // Default chain ID
        tx_type: TxType::Transfer { to, amount: req.amount },
    };

    // Sign the transaction
    let signing_hash = tx.signing_hash();
    let signature = state.keypair.sign(&signing_hash);
    let sender = state.keypair.public_key();
    let signed_tx = SignedTransaction::new(tx, signature, sender);
    let tx_hash = signed_tx.hash();

    // Add to mempool
    let mut mempool = state.mempool.write().await;
    mempool.add(signed_tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    let mempool_size = mempool.len();
    drop(mempool);

    info!("Transfer submitted: {} tokens to {}", req.amount, req.to);

    Ok(Json(serde_json::json!({
        "tx_hash": hex_encode(&tx_hash),
        "to": req.to,
        "amount": req.amount,
        "mempool_size": mempool_size
    })))
}

/// GET /api/balance?account=<hex> - Get token balance for an account
/// If no account is provided, returns the node's own balance
async fn api_balance(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let chain = state.chain.read().await;

    // Get account from query param or use node's own public key
    let account: [u8; 32] = if let Some(account_hex) = params.get("account") {
        if let Ok(bytes) = crypto::hex_decode(account_hex) {
            if bytes.len() == 32 {
                let mut arr = [0u8; 32];
                arr.copy_from_slice(&bytes);
                arr
            } else {
                return Json(serde_json::json!({ "error": "Invalid account: must be 32 bytes hex" }));
            }
        } else {
            return Json(serde_json::json!({ "error": "Invalid account hex" }));
        }
    } else {
        state.keypair.public_key()
    };

    let balance = staking::get_balance(chain.state(), &account);
    let locked = staking::get_locked_balance(chain.state(), &account);
    let available = balance.saturating_sub(locked);

    Json(serde_json::json!({
        "account": hex_encode(&account),
        "balance": balance,
        "locked": locked,
        "available": available
    }))
}

fn create_api_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/api/status", get(api_status))
        .route("/api/query", post(api_query))
        .route("/api/subreddits", get(api_subreddits))
        .route("/api/put", post(api_put))
        .route("/api/get", get(api_get))
        .route("/api/balance", get(api_balance))
        .route("/api/transfer", post(api_transfer))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Parse subreddit IDs from CLI arg
fn parse_subreddits(arg: &str) -> Vec<SubredditId> {
    if arg == "all" {
        // Full node - store everything (represented by empty vec for now)
        vec![SubredditId::LEGACY, SubredditId::GLOBAL]
    } else {
        arg.split(',')
            .filter_map(|s| {
                let s = s.trim();
                if s == "legacy" {
                    Some(SubredditId::LEGACY)
                } else if s == "global" {
                    Some(SubredditId::GLOBAL)
                } else if s.len() == 64 {
                    // Full hex ID
                    let bytes = crypto::hex_decode(s).ok()?;
                    if bytes.len() == 32 {
                        Some(SubredditId::from_bytes(bytes.try_into().ok()?))
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    setup_logging();

    // Open database
    let db = Database::open(&args.data_dir)?;
    info!("Database opened at {}", args.data_dir);

    // Configuration
    let config = DifficultyConfig::default();

    // Initialize chain
    let mut chain = initialize_chain(&db, config, args.genesis, args.seed_genesis)?;

    // Initialize mempool
    let mut mempool = Mempool::new(10000);

    // Initialize executor
    let mut executor = create_executor();

    // Get keypair
    let keypair = get_keypair(args.key_seed.as_deref());
    info!("Node public key: {}", hex_encode(&keypair.public_key()[..16]));

    match args.command {
        Some(Commands::Query { query }) => {
            run_query(&chain, &query);
        }
        Some(Commands::Status) => {
            show_status(&chain, &mempool, 0);
        }
        Some(Commands::Submit { contract, method, data }) => {
            submit_transaction(&chain, &mut mempool, &keypair, contract, method, data.as_bytes())?;
        }
        Some(Commands::Seed) => {
            // Submit seed transaction (method 1 on contract 1)
            submit_transaction(&chain, &mut mempool, &keypair, 1, 1, b"")?;
            info!("Seed transaction submitted. Mine a block to execute it.");
        }
        Some(Commands::Put { subreddit, entity_type, key, property, value }) => {
            let sender = keypair.public_key();
            let nonce = chain.get_nonce(&sender);
            let sub_id = parse_subreddit_arg(&subreddit, chain.height(), &sender);
            let entity_type_id = entity_type_to_id(&entity_type);
            let property_id = property_to_id(&property);

            let tx = Transaction::new(
                TxType::SubredditPut {
                    subreddit: sub_id,
                    entity_type: entity_type_id,
                    entity_key: key.clone(),
                    property: property_id,
                    value: value.as_bytes().to_vec(),
                },
                nonce,
            );

            let signing_hash = tx.signing_hash();
            let signature = keypair.sign(&signing_hash);
            let signed_tx = SignedTransaction::new(tx, signature, sender);

            let hash = mempool.add(signed_tx)?;
            println!("Put transaction submitted: {}", hex_encode(&hash[..8]));
            println!("  Subreddit: {}", subreddit);
            println!("  Type: {} (id: {})", entity_type, entity_type_id);
            println!("  Key: {}", key);
            println!("  Property: {} (id: {})", property, property_id);
            println!("  Value: {}", value);
            println!("\nMine a block to confirm: ./target/release/node -d {} --mine", args.data_dir);
        }
        Some(Commands::CreateSub { name, description, fee }) => {
            let sender = keypair.public_key();
            let nonce = chain.get_nonce(&sender);

            // Validate name
            if let Err(e) = subreddit::SubredditMeta::validate_name(&name) {
                eprintln!("Invalid subreddit name: {}", e);
                return Ok(());
            }

            let tx = Transaction::new(
                TxType::CreateSubreddit {
                    name: name.clone(),
                    description: description.clone(),
                    fee_amount: fee,
                },
                nonce,
            );

            let signing_hash = tx.signing_hash();
            let signature = keypair.sign(&signing_hash);
            let signed_tx = SignedTransaction::new(tx, signature, sender);

            let hash = mempool.add(signed_tx)?;
            let sub_id = SubredditId::derive(&sender, &name, chain.height() + 1);
            println!("CreateSubreddit transaction submitted: {}", hex_encode(&hash[..8]));
            println!("  Name: {}", name);
            println!("  Description: {}", description);
            println!("  Fee: {} tokens", fee);
            println!("  Expected ID: {}", sub_id);
            println!("\nMine a block to confirm.");
        }
        Some(Commands::Get { subreddit, entity_type, key, property }) => {
            let sender = keypair.public_key();
            let sub_id = parse_subreddit_arg(&subreddit, chain.height(), &sender);
            let entity_type_id = entity_type_to_id(&entity_type);
            let property_id = property_to_id(&property);

            // Build the key
            let storage_key = subreddit::prefixed_key(&sub_id, entity_type_id, &key, property_id);

            // Look up in state
            match chain.state().get(&storage_key) {
                Some(value) => {
                    // Try to interpret as UTF-8 string
                    match String::from_utf8(value.clone()) {
                        Ok(s) => println!("{}", s),
                        Err(_) => println!("{:?}", value),
                    }
                }
                None => println!("(not found)"),
            }
        }
        Some(Commands::List { limit }) => {
            println!("=== Blockchain State ({} entries) ===", chain.state().keys_count());
            let mut count = 0;
            for (key, value) in chain.state().iter() {
                if count >= limit {
                    println!("... ({} more entries)", chain.state().keys_count() - limit);
                    break;
                }
                // Try to decode key
                if let Some(sub_id) = subreddit::extract_subreddit(key) {
                    let value_str = String::from_utf8_lossy(value);
                    let value_display = if value_str.len() > 50 {
                        format!("{}...", &value_str[..50])
                    } else {
                        value_str.to_string()
                    };
                    println!("[{}] {} = {}", sub_id, hex_encode(&key[33..41]), value_display);
                } else {
                    // Old format key
                    let value_str = String::from_utf8_lossy(value);
                    let value_display = if value_str.len() > 50 {
                        format!("{}...", &value_str[..50])
                    } else {
                        value_str.to_string()
                    };
                    println!("{} = {}", hex_encode(&key[..8]), value_display);
                }
                count += 1;
            }
        }
        Some(Commands::Run) | None => {
            // Parse subreddits to store
            let stored_subreddits = parse_subreddits(&args.subreddits);
            info!("Storing {} subreddits: {:?}", stored_subreddits.len(),
                stored_subreddits.iter().map(|s| s.to_string()).collect::<Vec<_>>());

            // Start network
            let bootstrap_peers: Vec<_> = args.bootstrap
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect();

            let net_config = NetworkConfig {
                listen_addr: format!("/ip4/0.0.0.0/tcp/{}", args.port).parse().unwrap(),
                bootstrap_peers,
                enable_mdns: !args.no_mdns,
                keypair_seed: args.key_seed.as_ref().map(|s| {
                    let mut seed = [0u8; 32];
                    let decoded = crypto::hex_decode(s).unwrap_or_else(|_| {
                        crypto::sha256(s.as_bytes()).to_vec()
                    });
                    let len = decoded.len().min(32);
                    seed[..len].copy_from_slice(&decoded[..len]);
                    seed
                }),
            };

            let mut network = start_network(net_config).await?;
            info!("Network started. Peer ID: {}", network.peer_id);

            // Get miner public key for block rewards
            let miner_pubkey = keypair.public_key();

            // Start HTTP API if enabled
            let api_port = args.api_port;
            if api_port > 0 {
                let app_state = Arc::new(AppState {
                    chain: RwLock::new(chain),
                    mempool: RwLock::new(mempool),
                    stored_subreddits: stored_subreddits.clone(),
                    keypair: keypair.clone(),
                });

                let router = create_api_router(app_state.clone());
                let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", api_port)).await?;
                info!("HTTP API listening on http://localhost:{}", api_port);
                info!("  GET  /api/status     - Chain status");
                info!("  POST /api/query      - Query KB (JSON: {{\"query\": \"...\"}})");
                info!("  GET  /api/subreddits - List stored subreddits");
                info!("  POST /api/put        - Submit data (JSON: {{\"entity_type\": \"..\", \"key\": \"..\", \"property\": \"..\", \"value\": \"..\"}})");
                info!("  GET  /api/get        - Get data (?type=..&key=..&property=..)");

                // Run API server in background
                tokio::spawn(async move {
                    axum::serve(listener, router).await.ok();
                });

                // Get chain/mempool back from state for the main loop
                let chain_lock = &app_state.chain;
                let mempool_lock = &app_state.mempool;

                let mut peer_count: usize = 0;
                {
                    let chain = chain_lock.read().await;
                    let mempool = mempool_lock.read().await;
                    show_status(&chain, &mempool, peer_count);
                }

                if args.mine {
                    info!("Mining enabled. Starting mining loop...");
                    info!("Press Ctrl+C to stop.");
                } else {
                    info!("Mining disabled. Listening for blocks and transactions.");
                    info!("Use --mine to enable mining.");
                }

                // Main event loop with shared state
                let mining_enabled = args.mine;
                let mut mining_interval = tokio::time::interval(Duration::from_millis(100));

                loop {
                    tokio::select! {
                        Some(event) = network.recv() => {
                            let mut chain = chain_lock.write().await;
                            let mut mempool = mempool_lock.write().await;
                            match event {
                                NetworkEvent::BlockReceived { block, from } => {
                                    if let Ok(true) = handle_received_block(
                                        block, from, &mut chain, &mut mempool, &mut executor, &db,
                                    ) {}
                                }
                                NetworkEvent::TransactionReceived { tx, from } => {
                                    if handle_received_transaction(tx.clone(), from, &mut mempool) {
                                        network.broadcast_transaction(tx).await;
                                    }
                                }
                                NetworkEvent::PeerConnected(peer_id) => {
                                    peer_count += 1;
                                    info!("Peer connected: {} (total: {})", peer_id, peer_count);
                                    network.request_tip().await;
                                }
                                NetworkEvent::PeerDisconnected(peer_id) => {
                                    peer_count = peer_count.saturating_sub(1);
                                    info!("Peer disconnected: {} (total: {})", peer_id, peer_count);
                                }
                                NetworkEvent::TipRequested { from } => {
                                    if let Some(tip) = chain.tip() {
                                        debug!("Sending tip to {}", from);
                                        network.send_tip_response(tip.hash(), tip.header.height).await;
                                    }
                                }
                                NetworkEvent::TipResponseReceived { hash, height, from } => {
                                    debug!("Tip from {}: height {} hash {}", from, height, hex_encode(&hash[..8]));
                                    if height > chain.height() {
                                        info!("Peer {} is ahead (height {}), requesting blocks...", from, height);
                                        network.request_block(hash).await;
                                    }
                                }
                                NetworkEvent::BlockRequested { hash, from } => {
                                    debug!("Block {} requested by {}", hex_encode(&hash[..8]), from);
                                    let block = chain.get_block(&hash).cloned();
                                    network.send_block_response(block).await;
                                }
                                NetworkEvent::BlockResponseReceived { block, from } => {
                                    if let Some(block) = block {
                                        debug!("Received block response from {}", from);
                                        let _ = handle_received_block(
                                            block, from, &mut chain, &mut mempool, &mut executor, &db,
                                        );
                                    }
                                }
                            }
                        }
                        _ = mining_interval.tick(), if mining_enabled => {
                            let mut chain = chain_lock.write().await;
                            let mut mempool = mempool_lock.write().await;
                            match mine_next_block(&chain, &mut mempool, &mut executor, &miner_pubkey) {
                                Ok(block) => {
                                    network.broadcast_block(block.clone()).await;
                                    if let Err(e) = apply_block(&mut chain, block, &mut mempool, &mut executor, &db) {
                                        warn!("Failed to apply our own block: {}", e);
                                    }
                                }
                                Err(e) => {
                                    warn!("Mining error: {}", e);
                                }
                            }
                        }
                    }
                }
            } else {
                // No API - use original code path
                let mut peer_count: usize = 0;
                show_status(&chain, &mempool, peer_count);

                if args.mine {
                    info!("Mining enabled. Starting mining loop...");
                    info!("Press Ctrl+C to stop.");
                } else {
                    info!("Mining disabled. Listening for blocks and transactions.");
                    info!("Use --mine to enable mining.");
                }

                // Main event loop
                let mining_enabled = args.mine;
                let mut mining_interval = tokio::time::interval(Duration::from_millis(100));

            loop {
                tokio::select! {
                    // Handle network events
                    Some(event) = network.recv() => {
                        match event {
                            NetworkEvent::BlockReceived { block, from } => {
                                if let Ok(true) = handle_received_block(
                                    block,
                                    from,
                                    &mut chain,
                                    &mut mempool,
                                    &mut executor,
                                    &db,
                                ) {
                                    // Block was new and valid - no need to re-broadcast
                                    // (gossipsub handles propagation)
                                }
                            }
                            NetworkEvent::TransactionReceived { tx, from } => {
                                if handle_received_transaction(tx.clone(), from, &mut mempool) {
                                    // Transaction was new - broadcast to peers
                                    network.broadcast_transaction(tx).await;
                                }
                            }
                            NetworkEvent::PeerConnected(peer_id) => {
                                peer_count += 1;
                                info!("Peer connected: {} (total: {})", peer_id, peer_count);

                                // Request tip from new peer for sync
                                network.request_tip().await;
                            }
                            NetworkEvent::PeerDisconnected(peer_id) => {
                                peer_count = peer_count.saturating_sub(1);
                                info!("Peer disconnected: {} (total: {})", peer_id, peer_count);
                            }
                            NetworkEvent::TipRequested { from } => {
                                // Respond with our tip
                                if let Some(tip) = chain.tip() {
                                    debug!("Sending tip to {}", from);
                                    network.send_tip_response(tip.hash(), tip.header.height).await;
                                }
                            }
                            NetworkEvent::TipResponseReceived { hash, height, from } => {
                                debug!("Tip from {}: height {} hash {}", from, height, hex_encode(&hash[..8]));
                                // Simple sync: if they're ahead, request their latest block
                                if height > chain.height() {
                                    info!("Peer {} is ahead (height {}), requesting blocks...", from, height);
                                    // For now, just request the block at height we need
                                    // A proper sync would request ranges
                                    network.request_block(hash).await;
                                }
                            }
                            NetworkEvent::BlockRequested { hash, from } => {
                                debug!("Block {} requested by {}", hex_encode(&hash[..8]), from);
                                let block = chain.get_block(&hash).cloned();
                                network.send_block_response(block).await;
                            }
                            NetworkEvent::BlockResponseReceived { block, from } => {
                                if let Some(block) = block {
                                    debug!("Received block response from {}", from);
                                    let _ = handle_received_block(
                                        block,
                                        from,
                                        &mut chain,
                                        &mut mempool,
                                        &mut executor,
                                        &db,
                                    );
                                }
                            }
                        }
                    }

                    // Mining tick
                    _ = mining_interval.tick(), if mining_enabled => {
                        // Mine a block
                        match mine_next_block(&chain, &mut mempool, &mut executor, &miner_pubkey) {
                            Ok(block) => {
                                // Broadcast to peers first
                                network.broadcast_block(block.clone()).await;

                                // Then apply locally
                                if let Err(e) = apply_block(&mut chain, block, &mut mempool, &mut executor, &db) {
                                    warn!("Failed to apply our own block: {}", e);
                                }
                            }
                            Err(e) => {
                                warn!("Mining error: {}", e);
                            }
                        }
                    }
                }
            }
            } // end else (no API)
        }
    }

    Ok(())
}
