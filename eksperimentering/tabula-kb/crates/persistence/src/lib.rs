//! Persistent storage for blockchain data using sled.

use std::collections::HashMap;
use types::{Block, Hash};
use store::{KV, PartitionedKV};
use subreddit::SubredditId;

/// Database error types.
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("sled error: {0}")]
    Sled(#[from] sled::Error),
    #[error("decode error: {0}")]
    Decode(String),
    #[error("not found")]
    NotFound,
}

/// Key prefixes for different data types.
mod keys {
    pub const BLOCK: &[u8] = b"block:";
    pub const STATE: &[u8] = b"state";
    pub const TIP: &[u8] = b"tip";
    pub const HEIGHT: &[u8] = b"height";
    pub const NONCE: &[u8] = b"nonce:";
    // Per-subreddit storage
    pub const SUBREDDIT_STATE: &[u8] = b"sub_state:";
    pub const GLOBAL_REGISTRY: &[u8] = b"global_registry";
    pub const SUBREDDIT_LIST: &[u8] = b"subreddit_list";
}

/// Persistent database for blockchain storage.
pub struct Database {
    db: sled::Db,
}

impl Database {
    /// Open or create a database at the given path.
    pub fn open(path: &str) -> Result<Self, DbError> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Open an in-memory database (for testing).
    pub fn open_temp() -> Result<Self, DbError> {
        let config = sled::Config::new().temporary(true);
        let db = config.open()?;
        Ok(Self { db })
    }

    // ========================================================================
    // Block storage
    // ========================================================================

    /// Store a block.
    pub fn put_block(&self, block: &Block) -> Result<(), DbError> {
        let hash = block.hash();
        let mut key = keys::BLOCK.to_vec();
        key.extend_from_slice(&hash);

        let value = block.to_bytes();
        self.db.insert(key, value)?;
        Ok(())
    }

    /// Get a block by hash.
    pub fn get_block(&self, hash: &Hash) -> Result<Option<Block>, DbError> {
        let mut key = keys::BLOCK.to_vec();
        key.extend_from_slice(hash);

        match self.db.get(key)? {
            Some(data) => {
                let block = Block::from_bytes(&data)
                    .map_err(|e| DbError::Decode(e.to_string()))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    /// Get all blocks.
    pub fn get_all_blocks(&self) -> Result<HashMap<Hash, Block>, DbError> {
        let mut blocks = HashMap::new();

        for result in self.db.scan_prefix(keys::BLOCK) {
            let (key, value) = result?;
            if key.len() >= keys::BLOCK.len() + 32 {
                let block = Block::from_bytes(&value)
                    .map_err(|e| DbError::Decode(e.to_string()))?;
                blocks.insert(block.hash(), block);
            }
        }

        Ok(blocks)
    }

    // ========================================================================
    // Chain metadata
    // ========================================================================

    /// Get the current tip hash.
    pub fn get_tip(&self) -> Result<Option<Hash>, DbError> {
        match self.db.get(keys::TIP)? {
            Some(data) if data.len() == 32 => {
                let hash: Hash = data.as_ref().try_into()
                    .map_err(|_| DbError::Decode("invalid tip hash".into()))?;
                Ok(Some(hash))
            }
            Some(_) => Err(DbError::Decode("invalid tip hash length".into())),
            None => Ok(None),
        }
    }

    /// Set the current tip hash.
    pub fn set_tip(&self, hash: &Hash) -> Result<(), DbError> {
        self.db.insert(keys::TIP, hash.as_slice())?;
        Ok(())
    }

    /// Get the current height.
    pub fn get_height(&self) -> Result<u64, DbError> {
        match self.db.get(keys::HEIGHT)? {
            Some(data) if data.len() == 8 => {
                let height = u64::from_be_bytes(data.as_ref().try_into().unwrap());
                Ok(height)
            }
            _ => Ok(0),
        }
    }

    /// Set the current height.
    pub fn set_height(&self, height: u64) -> Result<(), DbError> {
        self.db.insert(keys::HEIGHT, &height.to_be_bytes())?;
        Ok(())
    }

    // ========================================================================
    // State storage
    // ========================================================================

    /// Save the current state.
    pub fn save_state(&self, state: &KV) -> Result<(), DbError> {
        let data = state.to_bytes();
        self.db.insert(keys::STATE, data)?;
        Ok(())
    }

    /// Load the state.
    pub fn load_state(&self) -> Result<Option<KV>, DbError> {
        match self.db.get(keys::STATE)? {
            Some(data) => {
                let state = KV::from_bytes(&data)
                    .ok_or_else(|| DbError::Decode("invalid state data".into()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    // ========================================================================
    // Partitioned state storage (per-subreddit)
    // ========================================================================

    /// Save a single subreddit's state.
    pub fn save_subreddit_state(&self, subreddit: &SubredditId, state: &KV) -> Result<(), DbError> {
        let mut key = keys::SUBREDDIT_STATE.to_vec();
        key.extend_from_slice(subreddit.as_bytes());
        self.db.insert(key, state.to_bytes())?;
        Ok(())
    }

    /// Load a single subreddit's state.
    pub fn load_subreddit_state(&self, subreddit: &SubredditId) -> Result<Option<KV>, DbError> {
        let mut key = keys::SUBREDDIT_STATE.to_vec();
        key.extend_from_slice(subreddit.as_bytes());

        match self.db.get(key)? {
            Some(data) => {
                let state = KV::from_bytes(&data)
                    .ok_or_else(|| DbError::Decode("invalid subreddit state".into()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// Delete a subreddit's state (for pruning).
    pub fn delete_subreddit_state(&self, subreddit: &SubredditId) -> Result<(), DbError> {
        let mut key = keys::SUBREDDIT_STATE.to_vec();
        key.extend_from_slice(subreddit.as_bytes());
        self.db.remove(key)?;
        Ok(())
    }

    /// Save the global registry state.
    pub fn save_global_registry(&self, state: &KV) -> Result<(), DbError> {
        self.db.insert(keys::GLOBAL_REGISTRY, state.to_bytes())?;
        Ok(())
    }

    /// Load the global registry state.
    pub fn load_global_registry(&self) -> Result<Option<KV>, DbError> {
        match self.db.get(keys::GLOBAL_REGISTRY)? {
            Some(data) => {
                let state = KV::from_bytes(&data)
                    .ok_or_else(|| DbError::Decode("invalid global registry".into()))?;
                Ok(Some(state))
            }
            None => Ok(None),
        }
    }

    /// List all stored subreddit IDs.
    pub fn list_stored_subreddits(&self) -> Result<Vec<SubredditId>, DbError> {
        let mut subreddits = Vec::new();

        for result in self.db.scan_prefix(keys::SUBREDDIT_STATE) {
            let (key, _) = result?;
            if key.len() == keys::SUBREDDIT_STATE.len() + 32 {
                let id_bytes: [u8; 32] = key[keys::SUBREDDIT_STATE.len()..]
                    .try_into()
                    .map_err(|_| DbError::Decode("invalid subreddit id".into()))?;
                subreddits.push(SubredditId::from_bytes(id_bytes));
            }
        }

        Ok(subreddits)
    }

    /// Save the entire partitioned state.
    pub fn save_partitioned_state(&self, state: &PartitionedKV) -> Result<(), DbError> {
        // Save global registry
        self.save_global_registry(&state.global)?;

        // Save each subreddit partition
        for subreddit_id in state.subreddit_ids() {
            if let Some(partition) = state.partition(&subreddit_id) {
                self.save_subreddit_state(&subreddit_id, partition)?;
            }
        }

        Ok(())
    }

    /// Load the entire partitioned state.
    /// If `subreddits` is Some, only load those specific subreddits.
    pub fn load_partitioned_state(
        &self,
        subreddits: Option<&[SubredditId]>,
    ) -> Result<PartitionedKV, DbError> {
        let mut state = PartitionedKV::new();

        // Load global registry
        if let Some(global) = self.load_global_registry()? {
            state.global = global;
        }

        // Load subreddit partitions
        let subreddit_ids = match subreddits {
            Some(ids) => ids.to_vec(),
            None => self.list_stored_subreddits()?,
        };

        for subreddit_id in subreddit_ids {
            if let Some(partition) = self.load_subreddit_state(&subreddit_id)? {
                let target = state.partition_mut(&subreddit_id);
                target.merge(&partition);
            }
        }

        Ok(state)
    }

    /// Check if a specific subreddit is stored locally.
    pub fn has_subreddit(&self, subreddit: &SubredditId) -> Result<bool, DbError> {
        let mut key = keys::SUBREDDIT_STATE.to_vec();
        key.extend_from_slice(subreddit.as_bytes());
        Ok(self.db.contains_key(key)?)
    }

    // ========================================================================
    // Account nonces
    // ========================================================================

    /// Get an account nonce.
    pub fn get_nonce(&self, account: &[u8; 32]) -> Result<u64, DbError> {
        let mut key = keys::NONCE.to_vec();
        key.extend_from_slice(account);

        match self.db.get(key)? {
            Some(data) if data.len() == 8 => {
                Ok(u64::from_be_bytes(data.as_ref().try_into().unwrap()))
            }
            _ => Ok(0),
        }
    }

    /// Set an account nonce.
    pub fn set_nonce(&self, account: &[u8; 32], nonce: u64) -> Result<(), DbError> {
        let mut key = keys::NONCE.to_vec();
        key.extend_from_slice(account);
        self.db.insert(key, &nonce.to_be_bytes())?;
        Ok(())
    }

    /// Get all account nonces.
    pub fn get_all_nonces(&self) -> Result<HashMap<[u8; 32], u64>, DbError> {
        let mut nonces = HashMap::new();

        for result in self.db.scan_prefix(keys::NONCE) {
            let (key, value) = result?;
            if key.len() == keys::NONCE.len() + 32 && value.len() == 8 {
                let account: [u8; 32] = key[keys::NONCE.len()..].try_into().unwrap();
                let nonce = u64::from_be_bytes(value.as_ref().try_into().unwrap());
                nonces.insert(account, nonce);
            }
        }

        Ok(nonces)
    }

    /// Save all account nonces.
    pub fn save_all_nonces(&self, nonces: &HashMap<[u8; 32], u64>) -> Result<(), DbError> {
        for (account, nonce) in nonces {
            self.set_nonce(account, *nonce)?;
        }
        Ok(())
    }

    // ========================================================================
    // Utility
    // ========================================================================

    /// Flush all pending writes to disk.
    pub fn flush(&self) -> Result<(), DbError> {
        self.db.flush()?;
        Ok(())
    }

    /// Check if the database is empty (no blocks).
    pub fn is_empty(&self) -> Result<bool, DbError> {
        Ok(self.get_tip()?.is_none())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use types::{BlockHeader, ZERO_HASH};

    fn create_test_block(height: u64) -> Block {
        let header = BlockHeader::new(
            height,
            1234567890,
            0x00_ff_ff_ff_ff_ff_ff_ff,
            ZERO_HASH,
            ZERO_HASH,
            ZERO_HASH,
        );
        Block::new(header, vec![])
    }

    #[test]
    fn test_block_roundtrip() {
        let db = Database::open_temp().unwrap();
        let block = create_test_block(0);
        let hash = block.hash();

        db.put_block(&block).unwrap();
        let loaded = db.get_block(&hash).unwrap().unwrap();

        assert_eq!(loaded.hash(), hash);
    }

    #[test]
    fn test_tip_roundtrip() {
        let db = Database::open_temp().unwrap();

        assert!(db.get_tip().unwrap().is_none());

        let tip = [42u8; 32];
        db.set_tip(&tip).unwrap();

        assert_eq!(db.get_tip().unwrap(), Some(tip));
    }

    #[test]
    fn test_state_roundtrip() {
        let db = Database::open_temp().unwrap();

        let mut state = KV::new();
        state.put(b"key".to_vec(), b"value".to_vec());

        db.save_state(&state).unwrap();
        let loaded = db.load_state().unwrap().unwrap();

        assert_eq!(loaded.get(b"key"), Some(&b"value".to_vec()));
    }

    #[test]
    fn test_nonce_roundtrip() {
        let db = Database::open_temp().unwrap();
        let account = [1u8; 32];

        assert_eq!(db.get_nonce(&account).unwrap(), 0);

        db.set_nonce(&account, 42).unwrap();
        assert_eq!(db.get_nonce(&account).unwrap(), 42);
    }

    #[test]
    fn test_get_all_blocks() {
        let db = Database::open_temp().unwrap();

        let block0 = create_test_block(0);
        let block1 = create_test_block(1);

        db.put_block(&block0).unwrap();
        db.put_block(&block1).unwrap();

        let all = db.get_all_blocks().unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_subreddit_state_roundtrip() {
        let db = Database::open_temp().unwrap();
        let sub_id = SubredditId::derive(&[1u8; 32], "test", 0);

        let mut state = KV::new();
        state.put(b"key".to_vec(), b"value".to_vec());

        db.save_subreddit_state(&sub_id, &state).unwrap();
        let loaded = db.load_subreddit_state(&sub_id).unwrap().unwrap();

        assert_eq!(loaded.get(b"key"), Some(&b"value".to_vec()));
    }

    #[test]
    fn test_partitioned_state_roundtrip() {
        let db = Database::open_temp().unwrap();

        let sub1 = SubredditId::derive(&[1u8; 32], "science", 0);
        let sub2 = SubredditId::derive(&[1u8; 32], "cooking", 0);

        let mut state = PartitionedKV::new();
        state.global.put(b"registry_key".to_vec(), b"registry_value".to_vec());
        state.put(&sub1, b"science_key".to_vec(), b"science_data".to_vec());
        state.put(&sub2, b"cooking_key".to_vec(), b"cooking_data".to_vec());

        db.save_partitioned_state(&state).unwrap();
        let loaded = db.load_partitioned_state(None).unwrap();

        assert_eq!(loaded.global.get(b"registry_key"), Some(&b"registry_value".to_vec()));
        assert_eq!(loaded.get(&sub1, b"science_key"), Some(&b"science_data".to_vec()));
        assert_eq!(loaded.get(&sub2, b"cooking_key"), Some(&b"cooking_data".to_vec()));
    }

    #[test]
    fn test_selective_subreddit_loading() {
        let db = Database::open_temp().unwrap();

        let sub1 = SubredditId::derive(&[1u8; 32], "science", 0);
        let sub2 = SubredditId::derive(&[1u8; 32], "cooking", 0);

        let mut state = PartitionedKV::new();
        state.put(&sub1, b"k1".to_vec(), b"v1".to_vec());
        state.put(&sub2, b"k2".to_vec(), b"v2".to_vec());

        db.save_partitioned_state(&state).unwrap();

        // Only load sub1
        let loaded = db.load_partitioned_state(Some(&[sub1])).unwrap();

        assert!(loaded.has_subreddit(&sub1));
        assert!(!loaded.has_subreddit(&sub2)); // sub2 not loaded
    }

    #[test]
    fn test_list_stored_subreddits() {
        let db = Database::open_temp().unwrap();

        let sub1 = SubredditId::derive(&[1u8; 32], "a", 0);
        let sub2 = SubredditId::derive(&[1u8; 32], "b", 0);

        let mut state = PartitionedKV::new();
        state.put(&sub1, b"k".to_vec(), b"v".to_vec());
        state.put(&sub2, b"k".to_vec(), b"v".to_vec());

        db.save_partitioned_state(&state).unwrap();

        let list = db.list_stored_subreddits().unwrap();
        assert_eq!(list.len(), 2);
        assert!(list.contains(&sub1));
        assert!(list.contains(&sub2));
    }

    #[test]
    fn test_delete_subreddit_state() {
        let db = Database::open_temp().unwrap();
        let sub_id = SubredditId::derive(&[1u8; 32], "delete_me", 0);

        let mut state = KV::new();
        state.put(b"key".to_vec(), b"value".to_vec());

        db.save_subreddit_state(&sub_id, &state).unwrap();
        assert!(db.has_subreddit(&sub_id).unwrap());

        db.delete_subreddit_state(&sub_id).unwrap();
        assert!(!db.has_subreddit(&sub_id).unwrap());
    }
}
