//! Proof-of-Work consensus: mining, difficulty adjustment, block validation.

use types::{Block, BlockHeader, Hash, SignedTransaction, ZERO_HASH};
use crypto::verify;

/// PoW difficulty configuration.
#[derive(Clone, Debug)]
pub struct DifficultyConfig {
    /// Initial difficulty target (higher value = easier mining).
    pub initial_difficulty: u64,
    /// Target block time in seconds.
    pub target_block_time_secs: u64,
    /// Number of blocks between difficulty adjustments.
    pub adjustment_interval: u64,
    /// Maximum adjustment factor per interval (e.g., 4 means 4x up or 4x down).
    pub max_adjustment_factor: u64,
}

impl Default for DifficultyConfig {
    fn default() -> Self {
        Self {
            // Easy initial difficulty for testing
            initial_difficulty: 0x00_00_ff_ff_ff_ff_ff_ff,
            target_block_time_secs: 5,   // 5 seconds per block
            adjustment_interval: 10,      // Adjust every 10 blocks
            max_adjustment_factor: 4,
        }
    }
}

/// Check if a hash meets the difficulty target.
///
/// The hash (interpreted as big-endian) must be <= difficulty.
/// Higher difficulty value = easier to mine.
pub fn hash_meets_target(hash: &Hash, difficulty: u64) -> bool {
    // Compare first 8 bytes as big-endian u64
    let hash_prefix = u64::from_be_bytes(hash[0..8].try_into().unwrap());
    hash_prefix <= difficulty
}

/// Mine a block by finding a valid nonce.
///
/// Returns `true` if mining succeeded within max_iterations.
/// The header's nonce field is updated in place.
pub fn mine_block(header: &mut BlockHeader, max_iterations: u64) -> bool {
    for nonce in 0..max_iterations {
        header.nonce = nonce;
        let hash = header.hash();
        if hash_meets_target(&hash, header.difficulty) {
            return true;
        }
    }
    false
}

/// Mine a block with no iteration limit (runs until found).
pub fn mine_block_unlimited(header: &mut BlockHeader) {
    let mut nonce: u64 = 0;
    loop {
        header.nonce = nonce;
        let hash = header.hash();
        if hash_meets_target(&hash, header.difficulty) {
            return;
        }
        nonce = nonce.wrapping_add(1);
    }
}

/// Calculate new difficulty based on actual vs expected block times.
pub fn adjust_difficulty(
    current_difficulty: u64,
    actual_time_secs: u64,
    expected_time_secs: u64,
    config: &DifficultyConfig,
) -> u64 {
    // Prevent division by zero
    let actual = actual_time_secs.max(1);
    let expected = expected_time_secs.max(1);

    // If blocks are coming too fast, make it harder (lower difficulty value)
    // If blocks are coming too slow, make it easier (higher difficulty value)
    let new_difficulty = if actual < expected {
        // Blocks too fast - make harder
        let ratio = expected / actual;
        let ratio = ratio.min(config.max_adjustment_factor);
        current_difficulty.saturating_div(ratio)
    } else {
        // Blocks too slow - make easier
        let ratio = actual / expected;
        let ratio = ratio.min(config.max_adjustment_factor);
        current_difficulty.saturating_mul(ratio)
    };

    // Don't let difficulty go to zero or max
    new_difficulty.max(1).min(u64::MAX - 1)
}

/// Consensus errors.
#[derive(Debug, Clone, thiserror::Error)]
pub enum ConsensusError {
    #[error("invalid proof of work")]
    InvalidPoW,
    #[error("invalid parent hash")]
    InvalidParentHash,
    #[error("timestamp too old (must be > parent timestamp)")]
    TimestampTooOld,
    #[error("timestamp too far in future")]
    TimestampTooFuture,
    #[error("invalid difficulty")]
    InvalidDifficulty,
    #[error("invalid height (expected {expected}, got {got})")]
    InvalidHeight { expected: u64, got: u64 },
    #[error("invalid transaction root")]
    InvalidTxRoot,
    #[error("invalid transaction signature")]
    InvalidTxSignature,
    #[error("duplicate transaction")]
    DuplicateTx,
}

/// Validate a block header.
pub fn validate_header(
    header: &BlockHeader,
    parent: Option<&BlockHeader>,
    current_time: u64,
    _config: &DifficultyConfig,
) -> Result<(), ConsensusError> {
    // Check PoW
    let hash = header.hash();
    if !hash_meets_target(&hash, header.difficulty) {
        return Err(ConsensusError::InvalidPoW);
    }

    // Check parent link
    if let Some(parent) = parent {
        if header.parent_hash != parent.hash() {
            return Err(ConsensusError::InvalidParentHash);
        }
        if header.height != parent.height + 1 {
            return Err(ConsensusError::InvalidHeight {
                expected: parent.height + 1,
                got: header.height,
            });
        }
        if header.timestamp <= parent.timestamp {
            return Err(ConsensusError::TimestampTooOld);
        }
    } else {
        // Genesis block checks
        if header.height != 0 {
            return Err(ConsensusError::InvalidHeight { expected: 0, got: header.height });
        }
        if header.parent_hash != ZERO_HASH {
            return Err(ConsensusError::InvalidParentHash);
        }
    }

    // Check timestamp not too far in future (allow 600 seconds drift for fast mining)
    if header.timestamp > current_time + 600 {
        return Err(ConsensusError::TimestampTooFuture);
    }

    Ok(())
}

/// Validate a full block including transactions.
pub fn validate_block(
    block: &Block,
    parent: Option<&BlockHeader>,
    current_time: u64,
    config: &DifficultyConfig,
) -> Result<(), ConsensusError> {
    // Validate header
    validate_header(&block.header, parent, current_time, config)?;

    // Validate transaction root
    let computed_tx_root = Block::compute_tx_root(&block.transactions);
    if block.header.tx_root != computed_tx_root {
        return Err(ConsensusError::InvalidTxRoot);
    }

    // Validate transaction signatures
    for tx in &block.transactions {
        let signing_hash = tx.tx.signing_hash();
        if !verify(&tx.public_key, &signing_hash, &tx.signature) {
            return Err(ConsensusError::InvalidTxSignature);
        }
    }

    // Check for duplicate transactions
    let mut seen = std::collections::HashSet::new();
    for tx in &block.transactions {
        let hash = tx.hash();
        if !seen.insert(hash) {
            return Err(ConsensusError::DuplicateTx);
        }
    }

    Ok(())
}

/// Create a genesis block.
pub fn create_genesis_block(
    state_root: Hash,
    difficulty: u64,
    timestamp: u64,
) -> Block {
    let mut header = BlockHeader::new(
        0,           // height
        timestamp,
        difficulty,
        ZERO_HASH,   // parent_hash
        state_root,
        ZERO_HASH,   // tx_root (no transactions)
    );

    // Mine the genesis block
    mine_block_unlimited(&mut header);

    Block::new(header, vec![])
}

/// Create a new block template for mining.
pub fn create_block_template(
    parent: &BlockHeader,
    transactions: Vec<SignedTransaction>,
    state_root: Hash,
    difficulty: u64,
    timestamp: u64,
) -> Block {
    let tx_root = Block::compute_tx_root(&transactions);

    let header = BlockHeader::new(
        parent.height + 1,
        timestamp,
        difficulty,
        parent.hash(),
        state_root,
        tx_root,
    );

    Block::new(header, transactions)
}

/// Create a new block template with miner for block rewards (v3).
pub fn create_block_template_v3(
    parent: &BlockHeader,
    transactions: Vec<SignedTransaction>,
    state_root: Hash,
    difficulty: u64,
    timestamp: u64,
    miner: [u8; 32],
) -> Block {
    let tx_root = Block::compute_tx_root(&transactions);

    let header = BlockHeader::new_v3(
        parent.height + 1,
        timestamp,
        difficulty,
        parent.hash(),
        state_root,
        tx_root,
        types::ZERO_HASH, // subreddit_roots_root
        miner,
    );

    Block::new(header, transactions)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_meets_target() {
        // Easy target (all bits set in prefix)
        let easy_target = 0xff_ff_ff_ff_ff_ff_ff_ff;
        let hard_target = 0x00_00_00_00_00_00_00_01;

        let easy_hash = [0xff; 32];
        let hard_hash = [0x00; 32];

        assert!(hash_meets_target(&hard_hash, easy_target));
        assert!(hash_meets_target(&hard_hash, hard_target));
        assert!(hash_meets_target(&easy_hash, easy_target));
        assert!(!hash_meets_target(&easy_hash, hard_target));
    }

    #[test]
    fn test_mine_block() {
        let config = DifficultyConfig::default();
        let mut header = BlockHeader::new(
            0,
            1234567890,
            config.initial_difficulty,
            ZERO_HASH,
            ZERO_HASH,
            ZERO_HASH,
        );

        assert!(mine_block(&mut header, 1_000_000));
        assert!(hash_meets_target(&header.hash(), header.difficulty));
    }

    #[test]
    fn test_create_genesis_block() {
        let config = DifficultyConfig::default();
        let genesis = create_genesis_block(ZERO_HASH, config.initial_difficulty, 0);

        assert_eq!(genesis.header.height, 0);
        assert_eq!(genesis.header.parent_hash, ZERO_HASH);
        assert!(hash_meets_target(&genesis.hash(), genesis.header.difficulty));
    }

    #[test]
    fn test_adjust_difficulty_blocks_too_fast() {
        let config = DifficultyConfig::default();
        let current = 0x00_00_ff_ff_ff_ff_ff_ff;

        // Blocks coming 2x faster than expected
        let actual_time = 25;  // 5 blocks in 25 seconds
        let expected_time = 50; // should have taken 50 seconds

        let new_diff = adjust_difficulty(current, actual_time, expected_time, &config);
        // Should be harder (lower value)
        assert!(new_diff < current);
    }

    #[test]
    fn test_adjust_difficulty_blocks_too_slow() {
        let config = DifficultyConfig::default();
        let current = 0x00_00_ff_ff_ff_ff_ff_ff;

        // Blocks coming 2x slower than expected
        let actual_time = 100; // 5 blocks in 100 seconds
        let expected_time = 50; // should have taken 50 seconds

        let new_diff = adjust_difficulty(current, actual_time, expected_time, &config);
        // Should be easier (higher value)
        assert!(new_diff > current);
    }

    #[test]
    fn test_validate_genesis_header() {
        let config = DifficultyConfig::default();
        let genesis = create_genesis_block(ZERO_HASH, config.initial_difficulty, 1000);

        let result = validate_header(&genesis.header, None, 2000, &config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_header_invalid_pow() {
        let config = DifficultyConfig::default();
        let header = BlockHeader::new(
            0,
            1000,
            0x00_00_00_00_00_00_00_01, // Very hard difficulty
            ZERO_HASH,
            ZERO_HASH,
            ZERO_HASH,
        );
        // Don't mine - nonce is 0

        let result = validate_header(&header, None, 2000, &config);
        assert!(matches!(result, Err(ConsensusError::InvalidPoW)));
    }
}
