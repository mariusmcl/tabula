//! Blockchain state management: blocks, chain tip, state transitions.

use std::collections::HashMap;
use types::{Block, BlockHeader, Hash, SignedTransaction, TxType};
use consensus::{validate_block, ConsensusError, DifficultyConfig};
use store::KV;
use staking::{self, StakingError};
use crypto;
use vm::Contract;

/// Chain error types.
#[derive(Debug, thiserror::Error)]
pub enum ChainError {
    #[error("consensus error: {0}")]
    Consensus(#[from] ConsensusError),
    #[error("block not found: {0:?}")]
    BlockNotFound(Hash),
    #[error("orphan block (parent not found)")]
    OrphanBlock,
    #[error("invalid state root (expected {expected:?}, got {got:?})")]
    InvalidStateRoot { expected: Hash, got: Hash },
    #[error("invalid nonce for sender (expected {expected}, got {got})")]
    InvalidNonce { expected: u64, got: u64 },
    #[error("contract not found: {0}")]
    ContractNotFound(u32),
    #[error("staking error: {0}")]
    Staking(#[from] StakingError),
}

/// Blockchain state and storage.
pub struct Chain {
    /// Block storage (hash -> block).
    blocks: HashMap<Hash, Block>,
    /// Height to hash mapping (canonical chain).
    height_to_hash: HashMap<u64, Hash>,
    /// Current chain tip hash.
    tip: Option<Hash>,
    /// Current state (after applying tip block).
    state: KV,
    /// Account nonces for replay protection.
    account_nonces: HashMap<[u8; 32], u64>,
    /// Consensus configuration.
    config: DifficultyConfig,
}

impl Chain {
    /// Create a new chain with a genesis block.
    pub fn new(genesis: Block, config: DifficultyConfig) -> Result<Self, ChainError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Validate genesis
        validate_block(&genesis, None, now, &config)?;

        let hash = genesis.hash();
        let mut chain = Self {
            blocks: HashMap::new(),
            height_to_hash: HashMap::new(),
            tip: None,
            state: KV::new(),
            account_nonces: HashMap::new(),
            config,
        };

        chain.blocks.insert(hash, genesis);
        chain.height_to_hash.insert(0, hash);
        chain.tip = Some(hash);

        Ok(chain)
    }

    /// Create a chain from existing state (for persistence).
    pub fn from_state(
        state: KV,
        blocks: HashMap<Hash, Block>,
        tip: Hash,
        account_nonces: HashMap<[u8; 32], u64>,
        config: DifficultyConfig,
    ) -> Self {
        let mut height_to_hash = HashMap::new();
        for (hash, block) in &blocks {
            height_to_hash.insert(block.header.height, *hash);
        }

        Self {
            blocks,
            height_to_hash,
            tip: Some(tip),
            state,
            account_nonces,
            config,
        }
    }

    /// Get the current tip block.
    pub fn tip(&self) -> Option<&Block> {
        self.tip.as_ref().and_then(|h| self.blocks.get(h))
    }

    /// Get the tip block hash.
    pub fn tip_hash(&self) -> Option<Hash> {
        self.tip
    }

    /// Get the tip block header.
    pub fn tip_header(&self) -> Option<&BlockHeader> {
        self.tip().map(|b| &b.header)
    }

    /// Get the current height.
    pub fn height(&self) -> u64 {
        self.tip().map(|b| b.header.height).unwrap_or(0)
    }

    /// Get a block by hash.
    pub fn get_block(&self, hash: &Hash) -> Option<&Block> {
        self.blocks.get(hash)
    }

    /// Get a block by height.
    pub fn get_block_at_height(&self, height: u64) -> Option<&Block> {
        self.height_to_hash.get(&height).and_then(|h| self.blocks.get(h))
    }

    /// Get the current state root.
    pub fn state_root(&self) -> Hash {
        self.state.ordered_merkle_root()
    }

    /// Get the current state (for queries).
    pub fn state(&self) -> &KV {
        &self.state
    }

    /// Get mutable state (for direct manipulation).
    pub fn state_mut(&mut self) -> &mut KV {
        &mut self.state
    }

    /// Get the nonce for an account.
    pub fn get_nonce(&self, account: &[u8; 32]) -> u64 {
        self.account_nonces.get(account).copied().unwrap_or(0)
    }

    /// Get all account nonces (for persistence).
    pub fn account_nonces(&self) -> &HashMap<[u8; 32], u64> {
        &self.account_nonces
    }

    /// Get all blocks (for persistence).
    pub fn blocks(&self) -> &HashMap<Hash, Block> {
        &self.blocks
    }

    /// Get the consensus config.
    pub fn config(&self) -> &DifficultyConfig {
        &self.config
    }

    /// Get the current difficulty.
    pub fn current_difficulty(&self) -> u64 {
        self.tip_header()
            .map(|h| h.difficulty)
            .unwrap_or(self.config.initial_difficulty)
    }

    /// Check if a block exists.
    pub fn has_block(&self, hash: &Hash) -> bool {
        self.blocks.contains_key(hash)
    }

    /// Apply a new block to the chain.
    ///
    /// This validates the block, executes transactions, and updates state.
    pub fn apply_block(
        &mut self,
        block: Block,
        contracts: &mut dyn ContractExecutor,
    ) -> Result<Hash, ChainError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Get parent
        let parent = if block.header.height == 0 {
            None
        } else {
            let parent = self.blocks.get(&block.header.parent_hash)
                .ok_or(ChainError::OrphanBlock)?;
            Some(&parent.header)
        };

        // Validate block
        validate_block(&block, parent, now, &self.config)?;

        // Execute transactions and compute state root (including block reward)
        let state_root = self.execute_transactions(&block.transactions, block.header.height, &block.header.miner, contracts)?;

        // Verify state root matches
        if block.header.state_root != state_root {
            return Err(ChainError::InvalidStateRoot {
                expected: block.header.state_root,
                got: state_root,
            });
        }

        // Store block and update tip
        let hash = block.hash();
        self.blocks.insert(hash, block.clone());
        self.height_to_hash.insert(block.header.height, hash);
        self.tip = Some(hash);

        Ok(hash)
    }

    /// Execute transactions and update state.
    fn execute_transactions(
        &mut self,
        txs: &[SignedTransaction],
        block_height: u64,
        miner: &[u8; 32],
        contracts: &mut dyn ContractExecutor,
    ) -> Result<Hash, ChainError> {
        // Mint block reward to miner (only if miner is non-zero, i.e., v3+ block)
        if miner != &[0u8; 32] {
            staking::mint_block_reward(&mut self.state, miner);
        }

        for tx in txs {
            let sender = tx.sender();
            let expected_nonce = self.get_nonce(&sender);

            // Check nonce
            if tx.tx.nonce != expected_nonce {
                return Err(ChainError::InvalidNonce {
                    expected: expected_nonce,
                    got: tx.tx.nonce,
                });
            }

            // Execute transaction
            match &tx.tx.tx_type {
                TxType::ContractCall { contract_id, method, calldata } => {
                    contracts.execute(&mut self.state, *contract_id, *method, calldata)?;
                }
                TxType::CreateSubreddit { name, description, fee_amount: _ } => {
                    // Store subreddit metadata in state
                    // Key: "subreddit:" + name
                    let key = format!("subreddit:{}", name.to_lowercase());
                    let meta = subreddit::SubredditMeta::new(
                        sender,
                        name.clone(),
                        description.clone(),
                        block_height,
                    );
                    self.state.put(key.into_bytes(), meta.to_bytes());
                }
                TxType::SubredditPut { subreddit, entity_type, entity_key, property, value } => {
                    // Build the storage key with subreddit prefix
                    let storage_key = subreddit::prefixed_key(
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                    );
                    self.state.put(storage_key, value.clone());
                }
                TxType::SubredditDelete { subreddit, entity_type, entity_key, property } => {
                    let storage_key = subreddit::prefixed_key(
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                    );
                    self.state.delete(&storage_key);
                }
                TxType::Transfer { to, amount } => {
                    staking::execute_transfer(&mut self.state, &sender, to, *amount)?;
                }
                TxType::Endorse { subreddit, entity_type, entity_key, property, value_hash, stake_amount, lock_blocks } => {
                    staking::execute_endorse(
                        &mut self.state,
                        &sender,
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                        *value_hash,
                        *stake_amount,
                        *lock_blocks,
                        block_height,
                    )?;
                }
                TxType::Challenge { subreddit, entity_type, entity_key, property, value_hash, stake_amount, evidence: _ } => {
                    staking::execute_challenge(
                        &mut self.state,
                        &sender,
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                        *value_hash,
                        *stake_amount,
                        block_height,
                    )?;
                }
                TxType::WithdrawStake { stake_id } => {
                    staking::execute_withdraw(&mut self.state, &sender, stake_id, block_height)?;
                }
                TxType::Dispute { subreddit, entity_type, entity_key, property, new_value, stake_amount } => {
                    // Compute hash of the new value being proposed
                    let new_value_hash = crypto::sha256(new_value);
                    staking::execute_endorse(
                        &mut self.state,
                        &sender,
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                        new_value_hash,
                        *stake_amount,
                        staking::MIN_LOCK_BLOCKS,
                        block_height,
                    )?;
                }
            }

            // Increment nonce
            self.account_nonces.insert(sender, expected_nonce + 1);
        }

        Ok(self.state.ordered_merkle_root())
    }

    /// Speculatively execute transactions without committing.
    ///
    /// Returns the resulting state root.
    pub fn speculative_execute(
        &self,
        txs: &[SignedTransaction],
        miner: &[u8; 32],
        contracts: &mut dyn ContractExecutor,
    ) -> Result<Hash, ChainError> {
        // Clone state for speculative execution
        let mut temp_state = self.state.clone();
        let mut temp_nonces = self.account_nonces.clone();

        // Mint block reward to miner (only if miner is non-zero, i.e., v3+ block)
        if miner != &[0u8; 32] {
            staking::mint_block_reward(&mut temp_state, miner);
        }

        for tx in txs {
            let sender = tx.sender();
            let expected_nonce = temp_nonces.get(&sender).copied().unwrap_or(0);

            if tx.tx.nonce != expected_nonce {
                return Err(ChainError::InvalidNonce {
                    expected: expected_nonce,
                    got: tx.tx.nonce,
                });
            }

            match &tx.tx.tx_type {
                TxType::ContractCall { contract_id, method, calldata } => {
                    contracts.execute(&mut temp_state, *contract_id, *method, calldata)?;
                }
                TxType::CreateSubreddit { name, description, fee_amount: _ } => {
                    let key = format!("subreddit:{}", name.to_lowercase());
                    let meta = subreddit::SubredditMeta::new(
                        sender,
                        name.clone(),
                        description.clone(),
                        self.height() + 1, // Next block height
                    );
                    temp_state.put(key.into_bytes(), meta.to_bytes());
                }
                TxType::SubredditPut { subreddit, entity_type, entity_key, property, value } => {
                    let storage_key = subreddit::prefixed_key(
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                    );
                    temp_state.put(storage_key, value.clone());
                }
                TxType::SubredditDelete { subreddit, entity_type, entity_key, property } => {
                    let storage_key = subreddit::prefixed_key(
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                    );
                    temp_state.delete(&storage_key);
                }
                TxType::Transfer { to, amount } => {
                    staking::execute_transfer(&mut temp_state, &sender, to, *amount)?;
                }
                TxType::Endorse { subreddit, entity_type, entity_key, property, value_hash, stake_amount, lock_blocks } => {
                    let block_height = self.height() + 1;
                    staking::execute_endorse(
                        &mut temp_state,
                        &sender,
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                        *value_hash,
                        *stake_amount,
                        *lock_blocks,
                        block_height,
                    )?;
                }
                TxType::Challenge { subreddit, entity_type, entity_key, property, value_hash, stake_amount, evidence: _ } => {
                    let block_height = self.height() + 1;
                    staking::execute_challenge(
                        &mut temp_state,
                        &sender,
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                        *value_hash,
                        *stake_amount,
                        block_height,
                    )?;
                }
                TxType::WithdrawStake { stake_id } => {
                    let block_height = self.height() + 1;
                    staking::execute_withdraw(&mut temp_state, &sender, stake_id, block_height)?;
                }
                TxType::Dispute { subreddit, entity_type, entity_key, property, new_value, stake_amount } => {
                    let block_height = self.height() + 1;
                    let new_value_hash = crypto::sha256(new_value);
                    staking::execute_endorse(
                        &mut temp_state,
                        &sender,
                        subreddit,
                        *entity_type,
                        entity_key,
                        *property,
                        new_value_hash,
                        *stake_amount,
                        staking::MIN_LOCK_BLOCKS,
                        block_height,
                    )?;
                }
            }

            temp_nonces.insert(sender, expected_nonce + 1);
        }

        Ok(temp_state.ordered_merkle_root())
    }
}

/// Trait for executing contract calls.
pub trait ContractExecutor {
    fn execute(
        &mut self,
        state: &mut KV,
        contract_id: u32,
        method: u32,
        calldata: &[u8],
    ) -> Result<Vec<u8>, ChainError>;
}

/// Simple contract executor using a map of contracts.
pub struct SimpleExecutor {
    contracts: HashMap<u32, Box<dyn Contract>>,
}

impl SimpleExecutor {
    pub fn new() -> Self {
        Self {
            contracts: HashMap::new(),
        }
    }

    pub fn register(&mut self, id: u32, contract: Box<dyn Contract>) {
        self.contracts.insert(id, contract);
    }
}

impl Default for SimpleExecutor {
    fn default() -> Self {
        Self::new()
    }
}

impl ContractExecutor for SimpleExecutor {
    fn execute(
        &mut self,
        state: &mut KV,
        contract_id: u32,
        method: u32,
        calldata: &[u8],
    ) -> Result<Vec<u8>, ChainError> {
        let contract = self.contracts.get_mut(&contract_id)
            .ok_or(ChainError::ContractNotFound(contract_id))?;
        Ok(contract.call(state, method, calldata))
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use types::ZERO_HASH;
    use consensus::create_genesis_block;

    #[test]
    fn test_new_chain_with_genesis() {
        let config = DifficultyConfig::default();
        let genesis = create_genesis_block(ZERO_HASH, config.initial_difficulty, 0);
        let chain = Chain::new(genesis.clone(), config).unwrap();

        assert_eq!(chain.height(), 0);
        assert!(chain.tip().is_some());
        assert_eq!(chain.tip().unwrap().hash(), genesis.hash());
    }

    #[test]
    fn test_get_block_by_height() {
        let config = DifficultyConfig::default();
        let genesis = create_genesis_block(ZERO_HASH, config.initial_difficulty, 0);
        let chain = Chain::new(genesis.clone(), config).unwrap();

        let block = chain.get_block_at_height(0).unwrap();
        assert_eq!(block.hash(), genesis.hash());
    }

    #[test]
    fn test_has_block() {
        let config = DifficultyConfig::default();
        let genesis = create_genesis_block(ZERO_HASH, config.initial_difficulty, 0);
        let hash = genesis.hash();
        let chain = Chain::new(genesis, config).unwrap();

        assert!(chain.has_block(&hash));
        assert!(!chain.has_block(&ZERO_HASH));
    }
}
