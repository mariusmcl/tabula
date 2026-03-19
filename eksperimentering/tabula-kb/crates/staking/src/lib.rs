//! Staking system for truth-weighted data.
//!
//! This module handles:
//! - Token balances (available and locked)
//! - Stake records for endorsements
//! - Confidence score calculation
//! - Challenge resolution

use std::collections::HashMap;
use crypto::sha256;
use serde::{Deserialize, Serialize};
use store::KV;
use subreddit::SubredditId;
use types::Hash;

/// Initial token supply minted to miners per block
pub const BLOCK_REWARD: u64 = 1000;

/// Minimum stake required for endorsement
pub const MIN_ENDORSEMENT_STAKE: u64 = 10;

/// Minimum stake required for challenge
pub const MIN_CHALLENGE_STAKE: u64 = 100;

/// Minimum lock duration for stakes (blocks)
pub const MIN_LOCK_BLOCKS: u64 = 100;

/// Staking error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum StakingError {
    #[error("insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },
    #[error("stake amount too low: minimum is {minimum}")]
    StakeTooLow { minimum: u64 },
    #[error("lock duration too short: minimum is {minimum} blocks")]
    LockTooShort { minimum: u64 },
    #[error("stake not found: {0:?}")]
    StakeNotFound(Hash),
    #[error("stake still locked until block {unlock_height}")]
    StakeStillLocked { unlock_height: u64 },
    #[error("data not found")]
    DataNotFound,
    #[error("value hash mismatch")]
    ValueHashMismatch,
}

/// A single stake record
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stake {
    /// Unique stake ID (hash of staker + data_key + timestamp)
    pub id: Hash,
    /// Account that created the stake
    pub staker: [u8; 32],
    /// Amount staked
    pub amount: u64,
    /// Block when stake was created
    pub created_at: u64,
    /// Block when stake can be withdrawn
    pub unlock_at: u64,
    /// Hash of the value being endorsed
    pub value_hash: Hash,
    /// Data location key
    pub data_key: Vec<u8>,
    /// Whether this is an endorsement (true) or challenge (false)
    pub is_endorsement: bool,
}

impl Stake {
    /// Create a new stake
    pub fn new(
        staker: [u8; 32],
        amount: u64,
        current_block: u64,
        lock_blocks: u64,
        value_hash: Hash,
        data_key: Vec<u8>,
        is_endorsement: bool,
    ) -> Self {
        // Generate unique ID
        let mut id_data = Vec::new();
        id_data.extend_from_slice(&staker);
        id_data.extend_from_slice(&data_key);
        id_data.extend_from_slice(&current_block.to_be_bytes());
        let id = sha256(&id_data);

        Self {
            id,
            staker,
            amount,
            created_at: current_block,
            unlock_at: current_block + lock_blocks,
            value_hash,
            data_key,
            is_endorsement,
        }
    }

    /// Encode stake to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(32 + 32 + 8 + 8 + 8 + 32 + 4 + self.data_key.len() + 1);
        buf.extend_from_slice(&self.id);
        buf.extend_from_slice(&self.staker);
        buf.extend_from_slice(&self.amount.to_be_bytes());
        buf.extend_from_slice(&self.created_at.to_be_bytes());
        buf.extend_from_slice(&self.unlock_at.to_be_bytes());
        buf.extend_from_slice(&self.value_hash);
        buf.extend_from_slice(&(self.data_key.len() as u32).to_be_bytes());
        buf.extend_from_slice(&self.data_key);
        buf.push(if self.is_endorsement { 1 } else { 0 });
        buf
    }

    /// Decode stake from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 121 {
            return None;
        }
        let id: Hash = data[0..32].try_into().ok()?;
        let staker: [u8; 32] = data[32..64].try_into().ok()?;
        let amount = u64::from_be_bytes(data[64..72].try_into().ok()?);
        let created_at = u64::from_be_bytes(data[72..80].try_into().ok()?);
        let unlock_at = u64::from_be_bytes(data[80..88].try_into().ok()?);
        let value_hash: Hash = data[88..120].try_into().ok()?;
        let key_len = u32::from_be_bytes(data[120..124].try_into().ok()?) as usize;
        if data.len() < 124 + key_len + 1 {
            return None;
        }
        let data_key = data[124..124 + key_len].to_vec();
        let is_endorsement = data[124 + key_len] == 1;

        Some(Self {
            id,
            staker,
            amount,
            created_at,
            unlock_at,
            value_hash,
            data_key,
            is_endorsement,
        })
    }
}

/// Aggregated confidence score for a data entry
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ConfidenceScore {
    /// Total stake endorsing this value
    pub endorsement_stake: u64,
    /// Number of unique endorsers
    pub endorser_count: u32,
    /// Total stake challenging this value
    pub challenge_stake: u64,
    /// Number of unique challengers
    pub challenger_count: u32,
    /// Block height of first endorsement
    pub first_endorsement: u64,
    /// Block height of latest endorsement
    pub latest_endorsement: u64,
}

impl ConfidenceScore {
    /// Calculate net confidence (endorsements - challenges)
    pub fn net_stake(&self) -> i64 {
        self.endorsement_stake as i64 - self.challenge_stake as i64
    }

    /// Calculate confidence ratio (0.0 to 1.0)
    pub fn confidence_ratio(&self) -> f64 {
        let total = self.endorsement_stake + self.challenge_stake;
        if total == 0 {
            return 0.0;
        }
        self.endorsement_stake as f64 / total as f64
    }

    /// Encode to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(40);
        buf.extend_from_slice(&self.endorsement_stake.to_be_bytes());
        buf.extend_from_slice(&self.endorser_count.to_be_bytes());
        buf.extend_from_slice(&self.challenge_stake.to_be_bytes());
        buf.extend_from_slice(&self.challenger_count.to_be_bytes());
        buf.extend_from_slice(&self.first_endorsement.to_be_bytes());
        buf.extend_from_slice(&self.latest_endorsement.to_be_bytes());
        buf
    }

    /// Decode from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 40 {
            return None;
        }
        Some(Self {
            endorsement_stake: u64::from_be_bytes(data[0..8].try_into().ok()?),
            endorser_count: u32::from_be_bytes(data[8..12].try_into().ok()?),
            challenge_stake: u64::from_be_bytes(data[12..20].try_into().ok()?),
            challenger_count: u32::from_be_bytes(data[20..24].try_into().ok()?),
            first_endorsement: u64::from_be_bytes(data[24..32].try_into().ok()?),
            latest_endorsement: u64::from_be_bytes(data[32..40].try_into().ok()?),
        })
    }
}

// ============================================================================
// State Key Helpers
// ============================================================================

/// Key prefix for token balances
const BALANCE_PREFIX: &[u8] = b"bal:";

/// Key prefix for locked balances
const LOCKED_PREFIX: &[u8] = b"lok:";

/// Key prefix for stake records
const STAKE_PREFIX: &[u8] = b"stk:";

/// Key prefix for confidence scores
const CONFIDENCE_PREFIX: &[u8] = b"cnf:";

/// Key prefix for stakes by data
const STAKES_BY_DATA_PREFIX: &[u8] = b"sbd:";

/// Build balance key
pub fn balance_key(account: &[u8; 32]) -> Vec<u8> {
    let mut key = Vec::with_capacity(BALANCE_PREFIX.len() + 32);
    key.extend_from_slice(BALANCE_PREFIX);
    key.extend_from_slice(account);
    key
}

/// Build locked balance key
pub fn locked_key(account: &[u8; 32]) -> Vec<u8> {
    let mut key = Vec::with_capacity(LOCKED_PREFIX.len() + 32);
    key.extend_from_slice(LOCKED_PREFIX);
    key.extend_from_slice(account);
    key
}

/// Build stake record key
pub fn stake_key(stake_id: &Hash) -> Vec<u8> {
    let mut key = Vec::with_capacity(STAKE_PREFIX.len() + 32);
    key.extend_from_slice(STAKE_PREFIX);
    key.extend_from_slice(stake_id);
    key
}

/// Build confidence score key for a data entry
pub fn confidence_key(
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
    value_hash: &Hash,
) -> Vec<u8> {
    let mut key = Vec::with_capacity(CONFIDENCE_PREFIX.len() + 32 + 4 + entity_key.len() + 4 + 32);
    key.extend_from_slice(CONFIDENCE_PREFIX);
    key.extend_from_slice(subreddit.as_bytes());
    key.extend_from_slice(&entity_type.to_be_bytes());
    key.extend_from_slice(entity_key.as_bytes());
    key.extend_from_slice(&property.to_be_bytes());
    key.extend_from_slice(value_hash);
    key
}

/// Build data key (without confidence prefix, for stake records)
pub fn data_key(
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
) -> Vec<u8> {
    let mut key = Vec::with_capacity(32 + 4 + entity_key.len() + 4);
    key.extend_from_slice(subreddit.as_bytes());
    key.extend_from_slice(&entity_type.to_be_bytes());
    key.extend_from_slice(entity_key.as_bytes());
    key.extend_from_slice(&property.to_be_bytes());
    key
}

// ============================================================================
// State Operations
// ============================================================================

/// Get account balance from state
pub fn get_balance(state: &KV, account: &[u8; 32]) -> u64 {
    let key = balance_key(account);
    state.get(&key)
        .and_then(|v| <[u8; 8]>::try_from(v.as_slice()).ok().map(u64::from_be_bytes))
        .unwrap_or(0)
}

/// Get locked balance from state
pub fn get_locked_balance(state: &KV, account: &[u8; 32]) -> u64 {
    let key = locked_key(account);
    state.get(&key)
        .and_then(|v| <[u8; 8]>::try_from(v.as_slice()).ok().map(u64::from_be_bytes))
        .unwrap_or(0)
}

/// Get available (unlocked) balance
pub fn get_available_balance(state: &KV, account: &[u8; 32]) -> u64 {
    get_balance(state, account).saturating_sub(get_locked_balance(state, account))
}

/// Set account balance
pub fn set_balance(state: &mut KV, account: &[u8; 32], amount: u64) {
    let key = balance_key(account);
    state.put(key, amount.to_be_bytes().to_vec());
}

/// Set locked balance
pub fn set_locked_balance(state: &mut KV, account: &[u8; 32], amount: u64) {
    let key = locked_key(account);
    state.put(key, amount.to_be_bytes().to_vec());
}

/// Add to balance (for mining rewards, transfers)
pub fn add_balance(state: &mut KV, account: &[u8; 32], amount: u64) {
    let current = get_balance(state, account);
    set_balance(state, account, current + amount);
}

/// Get stake record
pub fn get_stake(state: &KV, stake_id: &Hash) -> Option<Stake> {
    let key = stake_key(stake_id);
    state.get(&key).and_then(|v| Stake::from_bytes(&v))
}

/// Save stake record
pub fn save_stake(state: &mut KV, stake: &Stake) {
    let key = stake_key(&stake.id);
    state.put(key, stake.to_bytes());
}

/// Delete stake record
pub fn delete_stake(state: &mut KV, stake_id: &Hash) {
    let key = stake_key(stake_id);
    state.delete(&key);
}

/// Get confidence score for a value
pub fn get_confidence(
    state: &KV,
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
    value_hash: &Hash,
) -> ConfidenceScore {
    let key = confidence_key(subreddit, entity_type, entity_key, property, value_hash);
    state.get(&key)
        .and_then(|v| ConfidenceScore::from_bytes(&v))
        .unwrap_or_default()
}

/// Save confidence score
pub fn save_confidence(
    state: &mut KV,
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
    value_hash: &Hash,
    score: &ConfidenceScore,
) {
    let key = confidence_key(subreddit, entity_type, entity_key, property, value_hash);
    state.put(key, score.to_bytes());
}

// ============================================================================
// Transaction Execution
// ============================================================================

/// Execute a transfer transaction
pub fn execute_transfer(
    state: &mut KV,
    from: &[u8; 32],
    to: &[u8; 32],
    amount: u64,
) -> Result<(), StakingError> {
    let available = get_available_balance(state, from);
    if available < amount {
        return Err(StakingError::InsufficientBalance { have: available, need: amount });
    }

    let from_balance = get_balance(state, from);
    let to_balance = get_balance(state, to);

    set_balance(state, from, from_balance - amount);
    set_balance(state, to, to_balance + amount);

    Ok(())
}

/// Execute an endorsement transaction
pub fn execute_endorse(
    state: &mut KV,
    staker: &[u8; 32],
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
    value_hash: Hash,
    stake_amount: u64,
    lock_blocks: u64,
    current_block: u64,
) -> Result<Hash, StakingError> {
    // Validate stake amount
    if stake_amount < MIN_ENDORSEMENT_STAKE {
        return Err(StakingError::StakeTooLow { minimum: MIN_ENDORSEMENT_STAKE });
    }

    // Validate lock duration
    if lock_blocks < MIN_LOCK_BLOCKS {
        return Err(StakingError::LockTooShort { minimum: MIN_LOCK_BLOCKS });
    }

    // Check balance
    let available = get_available_balance(state, staker);
    if available < stake_amount {
        return Err(StakingError::InsufficientBalance { have: available, need: stake_amount });
    }

    // Lock tokens
    let locked = get_locked_balance(state, staker);
    set_locked_balance(state, staker, locked + stake_amount);

    // Create stake record
    let data_key = data_key(subreddit, entity_type, entity_key, property);
    let stake = Stake::new(
        *staker,
        stake_amount,
        current_block,
        lock_blocks,
        value_hash,
        data_key,
        true, // is_endorsement
    );
    save_stake(state, &stake);

    // Update confidence score
    let mut score = get_confidence(state, subreddit, entity_type, entity_key, property, &value_hash);
    score.endorsement_stake += stake_amount;
    score.endorser_count += 1;
    if score.first_endorsement == 0 {
        score.first_endorsement = current_block;
    }
    score.latest_endorsement = current_block;
    save_confidence(state, subreddit, entity_type, entity_key, property, &value_hash, &score);

    Ok(stake.id)
}

/// Execute a challenge transaction
pub fn execute_challenge(
    state: &mut KV,
    staker: &[u8; 32],
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
    value_hash: Hash,
    stake_amount: u64,
    current_block: u64,
) -> Result<Hash, StakingError> {
    // Validate stake amount
    if stake_amount < MIN_CHALLENGE_STAKE {
        return Err(StakingError::StakeTooLow { minimum: MIN_CHALLENGE_STAKE });
    }

    // Check balance
    let available = get_available_balance(state, staker);
    if available < stake_amount {
        return Err(StakingError::InsufficientBalance { have: available, need: stake_amount });
    }

    // Lock tokens (challenges lock for 2x minimum)
    let locked = get_locked_balance(state, staker);
    set_locked_balance(state, staker, locked + stake_amount);

    // Create stake record
    let data_key = data_key(subreddit, entity_type, entity_key, property);
    let stake = Stake::new(
        *staker,
        stake_amount,
        current_block,
        MIN_LOCK_BLOCKS * 2, // Challenges lock longer
        value_hash,
        data_key,
        false, // is_endorsement = false (challenge)
    );
    save_stake(state, &stake);

    // Update confidence score
    let mut score = get_confidence(state, subreddit, entity_type, entity_key, property, &value_hash);
    score.challenge_stake += stake_amount;
    score.challenger_count += 1;
    save_confidence(state, subreddit, entity_type, entity_key, property, &value_hash, &score);

    Ok(stake.id)
}

/// Execute stake withdrawal
pub fn execute_withdraw(
    state: &mut KV,
    staker: &[u8; 32],
    stake_id: &Hash,
    current_block: u64,
) -> Result<u64, StakingError> {
    // Get stake
    let stake = get_stake(state, stake_id)
        .ok_or(StakingError::StakeNotFound(*stake_id))?;

    // Verify ownership
    if stake.staker != *staker {
        return Err(StakingError::StakeNotFound(*stake_id));
    }

    // Check if unlocked
    if current_block < stake.unlock_at {
        return Err(StakingError::StakeStillLocked { unlock_height: stake.unlock_at });
    }

    // Unlock tokens
    let locked = get_locked_balance(state, staker);
    set_locked_balance(state, staker, locked.saturating_sub(stake.amount));

    // Delete stake record
    delete_stake(state, stake_id);

    Ok(stake.amount)
}

/// Mint block reward to miner
pub fn mint_block_reward(state: &mut KV, miner: &[u8; 32]) {
    add_balance(state, miner, BLOCK_REWARD);
}

/// Hash a value for endorsement/challenge
pub fn hash_value(value: &[u8]) -> Hash {
    sha256(value)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_state() -> KV {
        KV::new()
    }

    #[test]
    fn test_balance_operations() {
        let mut state = test_state();
        let account = [1u8; 32];

        assert_eq!(get_balance(&state, &account), 0);

        add_balance(&mut state, &account, 1000);
        assert_eq!(get_balance(&state, &account), 1000);
        assert_eq!(get_available_balance(&state, &account), 1000);

        set_locked_balance(&mut state, &account, 300);
        assert_eq!(get_available_balance(&state, &account), 700);
    }

    #[test]
    fn test_transfer() {
        let mut state = test_state();
        let alice = [1u8; 32];
        let bob = [2u8; 32];

        add_balance(&mut state, &alice, 1000);

        execute_transfer(&mut state, &alice, &bob, 400).unwrap();
        assert_eq!(get_balance(&state, &alice), 600);
        assert_eq!(get_balance(&state, &bob), 400);
    }

    #[test]
    fn test_transfer_insufficient() {
        let mut state = test_state();
        let alice = [1u8; 32];
        let bob = [2u8; 32];

        add_balance(&mut state, &alice, 100);

        let result = execute_transfer(&mut state, &alice, &bob, 200);
        assert!(matches!(result, Err(StakingError::InsufficientBalance { .. })));
    }

    #[test]
    fn test_endorse() {
        let mut state = test_state();
        let staker = [1u8; 32];
        let subreddit = SubredditId::LEGACY;

        add_balance(&mut state, &staker, 1000);

        let value_hash = hash_value(b"Tokyo");
        let stake_id = execute_endorse(
            &mut state,
            &staker,
            &subreddit,
            2, // country
            "Japan",
            101, // capital
            value_hash,
            100,
            MIN_LOCK_BLOCKS,
            1,
        ).unwrap();

        // Check balance locked
        assert_eq!(get_available_balance(&state, &staker), 900);
        assert_eq!(get_locked_balance(&state, &staker), 100);

        // Check stake created
        let stake = get_stake(&state, &stake_id).unwrap();
        assert_eq!(stake.amount, 100);
        assert!(stake.is_endorsement);

        // Check confidence updated
        let score = get_confidence(&state, &subreddit, 2, "Japan", 101, &value_hash);
        assert_eq!(score.endorsement_stake, 100);
        assert_eq!(score.endorser_count, 1);
    }

    #[test]
    fn test_withdraw_after_unlock() {
        let mut state = test_state();
        let staker = [1u8; 32];
        let subreddit = SubredditId::LEGACY;

        add_balance(&mut state, &staker, 1000);

        let value_hash = hash_value(b"Tokyo");
        let stake_id = execute_endorse(
            &mut state,
            &staker,
            &subreddit,
            2,
            "Japan",
            101,
            value_hash,
            100,
            MIN_LOCK_BLOCKS,
            1,
        ).unwrap();

        // Can't withdraw before unlock
        let result = execute_withdraw(&mut state, &staker, &stake_id, 50);
        assert!(matches!(result, Err(StakingError::StakeStillLocked { .. })));

        // Can withdraw after unlock
        let amount = execute_withdraw(&mut state, &staker, &stake_id, MIN_LOCK_BLOCKS + 2).unwrap();
        assert_eq!(amount, 100);
        assert_eq!(get_available_balance(&state, &staker), 1000);
    }

    #[test]
    fn test_stake_roundtrip() {
        let stake = Stake::new(
            [1u8; 32],
            500,
            100,
            200,
            [2u8; 32],
            b"test_key".to_vec(),
            true,
        );

        let bytes = stake.to_bytes();
        let decoded = Stake::from_bytes(&bytes).unwrap();

        assert_eq!(stake.id, decoded.id);
        assert_eq!(stake.staker, decoded.staker);
        assert_eq!(stake.amount, decoded.amount);
        assert_eq!(stake.data_key, decoded.data_key);
    }

    #[test]
    fn test_confidence_score_roundtrip() {
        let score = ConfidenceScore {
            endorsement_stake: 5000,
            endorser_count: 42,
            challenge_stake: 100,
            challenger_count: 2,
            first_endorsement: 1000,
            latest_endorsement: 5000,
        };

        let bytes = score.to_bytes();
        let decoded = ConfidenceScore::from_bytes(&bytes).unwrap();

        assert_eq!(score.endorsement_stake, decoded.endorsement_stake);
        assert_eq!(score.endorser_count, decoded.endorser_count);
        assert_eq!(score.confidence_ratio(), decoded.confidence_ratio());
    }
}
