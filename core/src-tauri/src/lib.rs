use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager, State};
use tabula_blockchain::{AppState, Block};
use serde::Serialize;

// Mining state structure
pub struct MiningState {
    is_mining: AtomicBool,
    current_nonce: AtomicU64,
    start_time: std::sync::Mutex<Option<Instant>>,
}

impl MiningState {
    pub fn new() -> Self {
        MiningState {
            is_mining: AtomicBool::new(false),
            current_nonce: AtomicU64::new(0),
            start_time: std::sync::Mutex::new(None),
        }
    }
}

impl Default for MiningState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Serialize)]
pub struct MiningStatus {
    is_mining: bool,
    current_nonce: u64,
    hash_rate: f64,
    elapsed_ms: u64,
    target_difficulty: usize,
}

#[derive(Clone, Serialize)]
pub struct MiningProgress {
    nonce: u64,
    #[serde(rename = "hashRate")]
    hash_rate: f64,
    elapsed: u64,
}

#[tauri::command]
fn get_blockchain(state: State<AppState>) -> Result<Vec<Block>, String> {
    let blockchain = state.blockchain.lock().unwrap();
    Ok(blockchain.get_chain())
}

#[tauri::command]
fn add_block(data: String, state: State<AppState>) -> Result<Vec<Block>, String> {
    let mut blockchain = state.blockchain.lock().unwrap();
    blockchain.add_block(data);
    Ok(blockchain.get_chain())
}

#[tauri::command]
fn validate_chain(state: State<AppState>) -> Result<bool, String> {
    let blockchain = state.blockchain.lock().unwrap();
    Ok(blockchain.is_chain_valid())
}

#[tauri::command]
fn get_difficulty(state: State<AppState>) -> Result<usize, String> {
    let blockchain = state.blockchain.lock().unwrap();
    Ok(blockchain.difficulty)
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! Welcome to Tabula Blockchain!", name)
}

#[tauri::command]
async fn start_mining(
    data: String,
    app: AppHandle,
    blockchain_state: State<'_, AppState>,
    mining_state: State<'_, Arc<MiningState>>,
) -> Result<(), String> {
    // Check if already mining
    if mining_state.is_mining.load(Ordering::SeqCst) {
        return Err("Mining already in progress".to_string());
    }

    // Set mining state
    mining_state.is_mining.store(true, Ordering::SeqCst);
    mining_state.current_nonce.store(0, Ordering::SeqCst);
    *mining_state.start_time.lock().unwrap() = Some(Instant::now());

    // Clone necessary data for the async task
    let mining_state_clone = Arc::clone(&mining_state);
    let blockchain_mutex = blockchain_state.blockchain.lock().unwrap();
    let difficulty = blockchain_mutex.difficulty;
    let previous_hash = blockchain_mutex.get_latest_block().hash.clone();
    let index = blockchain_mutex.get_latest_block().index + 1;
    drop(blockchain_mutex);

    // Get the app handle to access state later
    let app_handle = app.clone();

    // Spawn mining task
    tauri::async_runtime::spawn(async move {
        use sha2::{Sha256, Digest};
        use chrono::Utc;

        let timestamp = Utc::now().timestamp();
        let target = "0".repeat(difficulty);
        let mut nonce: u64 = 0;
        let start = Instant::now();
        let mut last_emit = Instant::now();

        loop {
            // Check if mining was cancelled
            if !mining_state_clone.is_mining.load(Ordering::SeqCst) {
                break;
            }

            // Calculate hash
            let data_str = format!("{}{}{}{}{}", index, timestamp, data, previous_hash, nonce);
            let mut hasher = Sha256::new();
            hasher.update(data_str.as_bytes());
            let hash = format!("{:x}", hasher.finalize());

            // Update nonce counter
            mining_state_clone.current_nonce.store(nonce, Ordering::SeqCst);

            // Check if we found a valid hash
            if hash.starts_with(&target) {
                // Create the new block
                let new_block = Block {
                    index,
                    timestamp,
                    data: data.clone(),
                    previous_hash: previous_hash.clone(),
                    hash: hash.clone(),
                    nonce,
                };

                // Add block to chain via app state
                let blockchain_state = app_handle.state::<AppState>();
                {
                    let mut blockchain = blockchain_state.blockchain.lock().unwrap();
                    blockchain.chain.push(new_block.clone());
                }

                // Emit block mined event
                let _ = app_handle.emit("block:mined", new_block.clone());

                // Emit chain updated event
                {
                    let blockchain = blockchain_state.blockchain.lock().unwrap();
                    let _ = app_handle.emit("chain:updated", blockchain.get_chain());
                }

                // Reset mining state
                mining_state_clone.is_mining.store(false, Ordering::SeqCst);
                break;
            }

            // Emit progress every 100ms
            if last_emit.elapsed().as_millis() >= 100 {
                let elapsed_ms = start.elapsed().as_millis() as u64;
                let hash_rate = if elapsed_ms > 0 {
                    (nonce as f64 / elapsed_ms as f64) * 1000.0
                } else {
                    0.0
                };

                let progress = MiningProgress {
                    nonce,
                    hash_rate,
                    elapsed: elapsed_ms,
                };

                let _ = app_handle.emit("mining:progress", progress);
                last_emit = Instant::now();
            }

            nonce += 1;

            // Yield to prevent blocking
            if nonce % 10000 == 0 {
                tokio::task::yield_now().await;
            }
        }
    });

    Ok(())
}

#[tauri::command]
fn stop_mining(mining_state: State<Arc<MiningState>>) -> Result<(), String> {
    mining_state.is_mining.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
fn get_mining_status(
    blockchain_state: State<AppState>,
    mining_state: State<Arc<MiningState>>,
) -> Result<MiningStatus, String> {
    let is_mining = mining_state.is_mining.load(Ordering::SeqCst);
    let current_nonce = mining_state.current_nonce.load(Ordering::SeqCst);
    let blockchain = blockchain_state.blockchain.lock().unwrap();
    let difficulty = blockchain.difficulty;

    let (elapsed_ms, hash_rate) = if let Some(start) = *mining_state.start_time.lock().unwrap() {
        let elapsed = start.elapsed().as_millis() as u64;
        let rate = if elapsed > 0 {
            (current_nonce as f64 / elapsed as f64) * 1000.0
        } else {
            0.0
        };
        (elapsed, rate)
    } else {
        (0, 0.0)
    };

    Ok(MiningStatus {
        is_mining,
        current_nonce,
        hash_rate,
        elapsed_ms,
        target_difficulty: difficulty,
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let blockchain_state = AppState::new(2);
            app.manage(blockchain_state);

            let mining_state = Arc::new(MiningState::new());
            app.manage(mining_state);

            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_liquid_glass::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            get_blockchain,
            add_block,
            validate_chain,
            get_difficulty,
            start_mining,
            stop_mining,
            get_mining_status
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
