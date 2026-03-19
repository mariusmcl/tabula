//! Core blockchain types: Block, Transaction, BlockHeader, SignedTransaction.
//!
//! All types use canonical binary encoding (not serde) for determinism.
//! Serde is only used for network serialization (gossip/sync).

use serde::{Serialize, Deserialize, Serializer, Deserializer};
use sha2::{Sha256, Digest};
use std::fmt;

pub use subreddit::SubredditId;

/// Serde support for [u8; 64] (signatures)
mod signature_serde {
    use serde::{Serialize, Deserialize, Serializer, Deserializer};

    pub fn serialize<S>(data: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serde::Serialize::serialize(&data[..], serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if vec.len() != 64 {
            return Err(serde::de::Error::custom(format!(
                "expected 64 bytes, got {}",
                vec.len()
            )));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&vec);
        Ok(arr)
    }
}

/// 32-byte hash type (SHA-256 output)
pub type Hash = [u8; 32];

/// Zero hash constant
pub const ZERO_HASH: Hash = [0u8; 32];

/// Chain ID for replay protection
pub const CHAIN_ID: u64 = 1;

// ============================================================================
// Block Header
// ============================================================================

/// Block header with PoW fields.
///
/// Canonical encoding order (v1, 132 bytes):
/// version(4) | height(8) | timestamp(8) | difficulty(8) | nonce(8) |
/// parent_hash(32) | state_root(32) | tx_root(32)
///
/// Canonical encoding order (v2, 164 bytes):
/// version(4) | height(8) | timestamp(8) | difficulty(8) | nonce(8) |
/// parent_hash(32) | state_root(32) | tx_root(32) | subreddit_roots_root(32) | miner(32)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    pub version: u32,
    pub height: u64,
    pub timestamp: u64,       // Unix timestamp in seconds
    pub difficulty: u64,      // PoW target (higher = easier)
    pub nonce: u64,           // PoW nonce
    pub parent_hash: Hash,
    pub state_root: Hash,     // Combined state root (global + subreddits)
    pub tx_root: Hash,        // Merkle root of transactions
    /// Merkle root of all subreddit roots (for partial validation).
    /// This allows nodes to verify blocks without having all subreddit data.
    /// Zero for v1 blocks.
    pub subreddit_roots_root: Hash,
    /// Miner's public key (receives block reward). Zero for v1/v2 blocks.
    #[serde(default)]
    pub miner: [u8; 32],
}

impl BlockHeader {
    /// Create a new v1 block header (legacy, no subreddit support).
    pub fn new(
        height: u64,
        timestamp: u64,
        difficulty: u64,
        parent_hash: Hash,
        state_root: Hash,
        tx_root: Hash,
    ) -> Self {
        Self {
            version: 1,
            height,
            timestamp,
            difficulty,
            nonce: 0,
            parent_hash,
            state_root,
            tx_root,
            subreddit_roots_root: ZERO_HASH,
            miner: [0u8; 32],
        }
    }

    /// Create a new v2 block header (with subreddit support).
    pub fn new_v2(
        height: u64,
        timestamp: u64,
        difficulty: u64,
        parent_hash: Hash,
        state_root: Hash,
        tx_root: Hash,
        subreddit_roots_root: Hash,
    ) -> Self {
        Self {
            version: 2,
            height,
            timestamp,
            difficulty,
            nonce: 0,
            parent_hash,
            state_root,
            tx_root,
            subreddit_roots_root,
            miner: [0u8; 32],
        }
    }

    /// Create a new v3 block header (with miner for block rewards).
    pub fn new_v3(
        height: u64,
        timestamp: u64,
        difficulty: u64,
        parent_hash: Hash,
        state_root: Hash,
        tx_root: Hash,
        subreddit_roots_root: Hash,
        miner: [u8; 32],
    ) -> Self {
        Self {
            version: 3,
            height,
            timestamp,
            difficulty,
            nonce: 0,
            parent_hash,
            state_root,
            tx_root,
            subreddit_roots_root,
            miner,
        }
    }

    /// Check if this is a v2 header (with subreddit support).
    pub fn is_v2(&self) -> bool {
        self.version >= 2
    }

    /// Compute the block hash (SHA-256 of canonical encoding).
    pub fn hash(&self) -> Hash {
        let bytes = self.to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }

    /// Canonical binary encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let capacity = if self.version >= 3 { 196 } else if self.version >= 2 { 164 } else { 132 };
        let mut buf = Vec::with_capacity(capacity);
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&self.height.to_be_bytes());
        buf.extend_from_slice(&self.timestamp.to_be_bytes());
        buf.extend_from_slice(&self.difficulty.to_be_bytes());
        buf.extend_from_slice(&self.nonce.to_be_bytes());
        buf.extend_from_slice(&self.parent_hash);
        buf.extend_from_slice(&self.state_root);
        buf.extend_from_slice(&self.tx_root);
        if self.version >= 2 {
            buf.extend_from_slice(&self.subreddit_roots_root);
        }
        if self.version >= 3 {
            buf.extend_from_slice(&self.miner);
        }
        buf
    }

    /// Decode from canonical bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 132 {
            return Err(DecodeError::InsufficientData);
        }
        let version = u32::from_be_bytes(data[0..4].try_into().unwrap());

        // V2 headers have subreddit_roots_root
        let subreddit_roots_root = if version >= 2 {
            if data.len() < 164 {
                return Err(DecodeError::InsufficientData);
            }
            data[132..164].try_into().unwrap()
        } else {
            ZERO_HASH
        };

        // V3 headers have miner
        let miner = if version >= 3 {
            if data.len() < 196 {
                return Err(DecodeError::InsufficientData);
            }
            data[164..196].try_into().unwrap()
        } else {
            [0u8; 32]
        };

        Ok(Self {
            version,
            height: u64::from_be_bytes(data[4..12].try_into().unwrap()),
            timestamp: u64::from_be_bytes(data[12..20].try_into().unwrap()),
            difficulty: u64::from_be_bytes(data[20..28].try_into().unwrap()),
            nonce: u64::from_be_bytes(data[28..36].try_into().unwrap()),
            parent_hash: data[36..68].try_into().unwrap(),
            state_root: data[68..100].try_into().unwrap(),
            tx_root: data[100..132].try_into().unwrap(),
            subreddit_roots_root,
            miner,
        })
    }
}

// ============================================================================
// Transaction
// ============================================================================

/// Transaction type enum.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxType {
    /// Call a contract method
    ContractCall {
        contract_id: u32,
        method: u32,
        calldata: Vec<u8>,
    },

    /// Create a new subreddit (requires fee)
    CreateSubreddit {
        name: String,
        description: String,
        fee_amount: u64,
    },

    /// Put a value in a subreddit's KV store
    SubredditPut {
        subreddit: SubredditId,
        entity_type: u32,
        entity_key: String,
        property: u32,
        value: Vec<u8>,
    },

    /// Delete a value from a subreddit's KV store
    SubredditDelete {
        subreddit: SubredditId,
        entity_type: u32,
        entity_key: String,
        property: u32,
    },

    // =========================================================================
    // Token & Staking Transactions
    // =========================================================================

    /// Transfer tokens to another account
    Transfer {
        to: [u8; 32],
        amount: u64,
    },

    /// Endorse existing data by staking tokens on its truth
    Endorse {
        subreddit: SubredditId,
        entity_type: u32,
        entity_key: String,
        property: u32,
        value_hash: Hash,      // Hash of the value being endorsed
        stake_amount: u64,     // Tokens to stake
        lock_blocks: u64,      // How long to lock the stake
    },

    /// Challenge data by staking tokens against it
    Challenge {
        subreddit: SubredditId,
        entity_type: u32,
        entity_key: String,
        property: u32,
        value_hash: Hash,      // Hash of the value being challenged
        stake_amount: u64,     // Tokens to stake on the challenge
        evidence: Vec<u8>,     // Evidence/reasoning for challenge
    },

    /// Withdraw unlocked stakes
    WithdrawStake {
        stake_id: Hash,        // ID of the stake to withdraw
    },

    /// Submit alternative value (dispute)
    Dispute {
        subreddit: SubredditId,
        entity_type: u32,
        entity_key: String,
        property: u32,
        new_value: Vec<u8>,    // The competing value
        stake_amount: u64,     // Must exceed current leader's stake
    },
}

impl TxType {
    /// Type tag for encoding.
    fn tag(&self) -> u8 {
        match self {
            TxType::ContractCall { .. } => 0x01,
            TxType::CreateSubreddit { .. } => 0x10,
            TxType::SubredditPut { .. } => 0x11,
            TxType::SubredditDelete { .. } => 0x12,
            // Token & staking tags
            TxType::Transfer { .. } => 0x20,
            TxType::Endorse { .. } => 0x21,
            TxType::Challenge { .. } => 0x22,
            TxType::WithdrawStake { .. } => 0x23,
            TxType::Dispute { .. } => 0x24,
        }
    }

    /// Canonical encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            TxType::ContractCall { contract_id, method, calldata } => {
                let mut buf = Vec::with_capacity(1 + 4 + 4 + 4 + calldata.len());
                buf.push(self.tag());
                buf.extend_from_slice(&contract_id.to_be_bytes());
                buf.extend_from_slice(&method.to_be_bytes());
                buf.extend_from_slice(&(calldata.len() as u32).to_be_bytes());
                buf.extend_from_slice(calldata);
                buf
            }
            TxType::CreateSubreddit { name, description, fee_amount } => {
                // tag(1) + fee(8) + name_len(2) + name + desc_len(2) + desc
                let mut buf = Vec::with_capacity(1 + 8 + 2 + name.len() + 2 + description.len());
                buf.push(self.tag());
                buf.extend_from_slice(&fee_amount.to_be_bytes());
                buf.extend_from_slice(&(name.len() as u16).to_be_bytes());
                buf.extend_from_slice(name.as_bytes());
                buf.extend_from_slice(&(description.len() as u16).to_be_bytes());
                buf.extend_from_slice(description.as_bytes());
                buf
            }
            TxType::SubredditPut { subreddit, entity_type, entity_key, property, value } => {
                // tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_len(4) + value
                let mut buf = Vec::with_capacity(1 + 32 + 4 + 2 + entity_key.len() + 4 + 4 + value.len());
                buf.push(self.tag());
                buf.extend_from_slice(subreddit.as_bytes());
                buf.extend_from_slice(&entity_type.to_be_bytes());
                buf.extend_from_slice(&(entity_key.len() as u16).to_be_bytes());
                buf.extend_from_slice(entity_key.as_bytes());
                buf.extend_from_slice(&property.to_be_bytes());
                buf.extend_from_slice(&(value.len() as u32).to_be_bytes());
                buf.extend_from_slice(value);
                buf
            }
            TxType::SubredditDelete { subreddit, entity_type, entity_key, property } => {
                // tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4)
                let mut buf = Vec::with_capacity(1 + 32 + 4 + 2 + entity_key.len() + 4);
                buf.push(self.tag());
                buf.extend_from_slice(subreddit.as_bytes());
                buf.extend_from_slice(&entity_type.to_be_bytes());
                buf.extend_from_slice(&(entity_key.len() as u16).to_be_bytes());
                buf.extend_from_slice(entity_key.as_bytes());
                buf.extend_from_slice(&property.to_be_bytes());
                buf
            }
            TxType::Transfer { to, amount } => {
                // tag(1) + to(32) + amount(8)
                let mut buf = Vec::with_capacity(41);
                buf.push(self.tag());
                buf.extend_from_slice(to);
                buf.extend_from_slice(&amount.to_be_bytes());
                buf
            }
            TxType::Endorse { subreddit, entity_type, entity_key, property, value_hash, stake_amount, lock_blocks } => {
                // tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_hash(32) + stake(8) + lock(8)
                let mut buf = Vec::with_capacity(1 + 32 + 4 + 2 + entity_key.len() + 4 + 32 + 8 + 8);
                buf.push(self.tag());
                buf.extend_from_slice(subreddit.as_bytes());
                buf.extend_from_slice(&entity_type.to_be_bytes());
                buf.extend_from_slice(&(entity_key.len() as u16).to_be_bytes());
                buf.extend_from_slice(entity_key.as_bytes());
                buf.extend_from_slice(&property.to_be_bytes());
                buf.extend_from_slice(value_hash);
                buf.extend_from_slice(&stake_amount.to_be_bytes());
                buf.extend_from_slice(&lock_blocks.to_be_bytes());
                buf
            }
            TxType::Challenge { subreddit, entity_type, entity_key, property, value_hash, stake_amount, evidence } => {
                // tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_hash(32) + stake(8) + evidence_len(4) + evidence
                let mut buf = Vec::with_capacity(1 + 32 + 4 + 2 + entity_key.len() + 4 + 32 + 8 + 4 + evidence.len());
                buf.push(self.tag());
                buf.extend_from_slice(subreddit.as_bytes());
                buf.extend_from_slice(&entity_type.to_be_bytes());
                buf.extend_from_slice(&(entity_key.len() as u16).to_be_bytes());
                buf.extend_from_slice(entity_key.as_bytes());
                buf.extend_from_slice(&property.to_be_bytes());
                buf.extend_from_slice(value_hash);
                buf.extend_from_slice(&stake_amount.to_be_bytes());
                buf.extend_from_slice(&(evidence.len() as u32).to_be_bytes());
                buf.extend_from_slice(evidence);
                buf
            }
            TxType::WithdrawStake { stake_id } => {
                // tag(1) + stake_id(32)
                let mut buf = Vec::with_capacity(33);
                buf.push(self.tag());
                buf.extend_from_slice(stake_id);
                buf
            }
            TxType::Dispute { subreddit, entity_type, entity_key, property, new_value, stake_amount } => {
                // tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_len(4) + value + stake(8)
                let mut buf = Vec::with_capacity(1 + 32 + 4 + 2 + entity_key.len() + 4 + 4 + new_value.len() + 8);
                buf.push(self.tag());
                buf.extend_from_slice(subreddit.as_bytes());
                buf.extend_from_slice(&entity_type.to_be_bytes());
                buf.extend_from_slice(&(entity_key.len() as u16).to_be_bytes());
                buf.extend_from_slice(entity_key.as_bytes());
                buf.extend_from_slice(&property.to_be_bytes());
                buf.extend_from_slice(&(new_value.len() as u32).to_be_bytes());
                buf.extend_from_slice(new_value);
                buf.extend_from_slice(&stake_amount.to_be_bytes());
                buf
            }
        }
    }

    /// Decode from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), DecodeError> {
        if data.is_empty() {
            return Err(DecodeError::InsufficientData);
        }
        match data[0] {
            0x01 => {
                if data.len() < 13 {
                    return Err(DecodeError::InsufficientData);
                }
                let contract_id = u32::from_be_bytes(data[1..5].try_into().unwrap());
                let method = u32::from_be_bytes(data[5..9].try_into().unwrap());
                let calldata_len = u32::from_be_bytes(data[9..13].try_into().unwrap()) as usize;
                if data.len() < 13 + calldata_len {
                    return Err(DecodeError::InsufficientData);
                }
                let calldata = data[13..13 + calldata_len].to_vec();
                Ok((TxType::ContractCall { contract_id, method, calldata }, 13 + calldata_len))
            }
            0x10 => {
                // CreateSubreddit: tag(1) + fee(8) + name_len(2) + name + desc_len(2) + desc
                if data.len() < 13 {
                    return Err(DecodeError::InsufficientData);
                }
                let fee_amount = u64::from_be_bytes(data[1..9].try_into().unwrap());
                let name_len = u16::from_be_bytes(data[9..11].try_into().unwrap()) as usize;
                if data.len() < 11 + name_len + 2 {
                    return Err(DecodeError::InsufficientData);
                }
                let name = String::from_utf8(data[11..11 + name_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let desc_start = 11 + name_len;
                let desc_len = u16::from_be_bytes(data[desc_start..desc_start + 2].try_into().unwrap()) as usize;
                if data.len() < desc_start + 2 + desc_len {
                    return Err(DecodeError::InsufficientData);
                }
                let description = String::from_utf8(data[desc_start + 2..desc_start + 2 + desc_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let total_len = desc_start + 2 + desc_len;
                Ok((TxType::CreateSubreddit { name, description, fee_amount }, total_len))
            }
            0x11 => {
                // SubredditPut: tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_len(4) + value
                if data.len() < 43 {
                    return Err(DecodeError::InsufficientData);
                }
                let subreddit = SubredditId::from_bytes(data[1..33].try_into().unwrap());
                let entity_type = u32::from_be_bytes(data[33..37].try_into().unwrap());
                let key_len = u16::from_be_bytes(data[37..39].try_into().unwrap()) as usize;
                if data.len() < 39 + key_len + 8 {
                    return Err(DecodeError::InsufficientData);
                }
                let entity_key = String::from_utf8(data[39..39 + key_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let prop_start = 39 + key_len;
                let property = u32::from_be_bytes(data[prop_start..prop_start + 4].try_into().unwrap());
                let value_len = u32::from_be_bytes(data[prop_start + 4..prop_start + 8].try_into().unwrap()) as usize;
                if data.len() < prop_start + 8 + value_len {
                    return Err(DecodeError::InsufficientData);
                }
                let value = data[prop_start + 8..prop_start + 8 + value_len].to_vec();
                let total_len = prop_start + 8 + value_len;
                Ok((TxType::SubredditPut { subreddit, entity_type, entity_key, property, value }, total_len))
            }
            0x12 => {
                // SubredditDelete: tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4)
                if data.len() < 43 {
                    return Err(DecodeError::InsufficientData);
                }
                let subreddit = SubredditId::from_bytes(data[1..33].try_into().unwrap());
                let entity_type = u32::from_be_bytes(data[33..37].try_into().unwrap());
                let key_len = u16::from_be_bytes(data[37..39].try_into().unwrap()) as usize;
                if data.len() < 39 + key_len + 4 {
                    return Err(DecodeError::InsufficientData);
                }
                let entity_key = String::from_utf8(data[39..39 + key_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let prop_start = 39 + key_len;
                let property = u32::from_be_bytes(data[prop_start..prop_start + 4].try_into().unwrap());
                let total_len = prop_start + 4;
                Ok((TxType::SubredditDelete { subreddit, entity_type, entity_key, property }, total_len))
            }
            0x20 => {
                // Transfer: tag(1) + to(32) + amount(8)
                if data.len() < 41 {
                    return Err(DecodeError::InsufficientData);
                }
                let to: [u8; 32] = data[1..33].try_into().unwrap();
                let amount = u64::from_be_bytes(data[33..41].try_into().unwrap());
                Ok((TxType::Transfer { to, amount }, 41))
            }
            0x21 => {
                // Endorse: tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_hash(32) + stake(8) + lock(8)
                if data.len() < 91 {
                    return Err(DecodeError::InsufficientData);
                }
                let subreddit = SubredditId::from_bytes(data[1..33].try_into().unwrap());
                let entity_type = u32::from_be_bytes(data[33..37].try_into().unwrap());
                let key_len = u16::from_be_bytes(data[37..39].try_into().unwrap()) as usize;
                if data.len() < 39 + key_len + 52 {
                    return Err(DecodeError::InsufficientData);
                }
                let entity_key = String::from_utf8(data[39..39 + key_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let mut offset = 39 + key_len;
                let property = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
                offset += 4;
                let value_hash: Hash = data[offset..offset + 32].try_into().unwrap();
                offset += 32;
                let stake_amount = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;
                let lock_blocks = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;
                Ok((TxType::Endorse { subreddit, entity_type, entity_key, property, value_hash, stake_amount, lock_blocks }, offset))
            }
            0x22 => {
                // Challenge: tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_hash(32) + stake(8) + evidence_len(4) + evidence
                if data.len() < 87 {
                    return Err(DecodeError::InsufficientData);
                }
                let subreddit = SubredditId::from_bytes(data[1..33].try_into().unwrap());
                let entity_type = u32::from_be_bytes(data[33..37].try_into().unwrap());
                let key_len = u16::from_be_bytes(data[37..39].try_into().unwrap()) as usize;
                if data.len() < 39 + key_len + 48 {
                    return Err(DecodeError::InsufficientData);
                }
                let entity_key = String::from_utf8(data[39..39 + key_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let mut offset = 39 + key_len;
                let property = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
                offset += 4;
                let value_hash: Hash = data[offset..offset + 32].try_into().unwrap();
                offset += 32;
                let stake_amount = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;
                let evidence_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;
                if data.len() < offset + evidence_len {
                    return Err(DecodeError::InsufficientData);
                }
                let evidence = data[offset..offset + evidence_len].to_vec();
                offset += evidence_len;
                Ok((TxType::Challenge { subreddit, entity_type, entity_key, property, value_hash, stake_amount, evidence }, offset))
            }
            0x23 => {
                // WithdrawStake: tag(1) + stake_id(32)
                if data.len() < 33 {
                    return Err(DecodeError::InsufficientData);
                }
                let stake_id: Hash = data[1..33].try_into().unwrap();
                Ok((TxType::WithdrawStake { stake_id }, 33))
            }
            0x24 => {
                // Dispute: tag(1) + subreddit(32) + entity_type(4) + key_len(2) + key + property(4) + value_len(4) + value + stake(8)
                if data.len() < 55 {
                    return Err(DecodeError::InsufficientData);
                }
                let subreddit = SubredditId::from_bytes(data[1..33].try_into().unwrap());
                let entity_type = u32::from_be_bytes(data[33..37].try_into().unwrap());
                let key_len = u16::from_be_bytes(data[37..39].try_into().unwrap()) as usize;
                if data.len() < 39 + key_len + 16 {
                    return Err(DecodeError::InsufficientData);
                }
                let entity_key = String::from_utf8(data[39..39 + key_len].to_vec())
                    .map_err(|_| DecodeError::InvalidData)?;
                let mut offset = 39 + key_len;
                let property = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap());
                offset += 4;
                let value_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
                offset += 4;
                if data.len() < offset + value_len + 8 {
                    return Err(DecodeError::InsufficientData);
                }
                let new_value = data[offset..offset + value_len].to_vec();
                offset += value_len;
                let stake_amount = u64::from_be_bytes(data[offset..offset + 8].try_into().unwrap());
                offset += 8;
                Ok((TxType::Dispute { subreddit, entity_type, entity_key, property, new_value, stake_amount }, offset))
            }
            _ => Err(DecodeError::UnknownTag),
        }
    }
}

/// Unsigned transaction.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    pub tx_type: TxType,
    pub nonce: u64,      // Sender's transaction count (replay protection)
    pub chain_id: u64,   // Chain ID (replay protection across chains)
}

impl Transaction {
    /// Create a new transaction.
    pub fn new(tx_type: TxType, nonce: u64) -> Self {
        Self {
            tx_type,
            nonce,
            chain_id: CHAIN_ID,
        }
    }

    /// Canonical encoding for signing.
    pub fn to_bytes(&self) -> Vec<u8> {
        let tx_bytes = self.tx_type.to_bytes();
        let mut buf = Vec::with_capacity(16 + tx_bytes.len());
        buf.extend_from_slice(&self.nonce.to_be_bytes());
        buf.extend_from_slice(&self.chain_id.to_be_bytes());
        buf.extend_from_slice(&tx_bytes);
        buf
    }

    /// Decode from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<(Self, usize), DecodeError> {
        if data.len() < 16 {
            return Err(DecodeError::InsufficientData);
        }
        let nonce = u64::from_be_bytes(data[0..8].try_into().unwrap());
        let chain_id = u64::from_be_bytes(data[8..16].try_into().unwrap());
        let (tx_type, tx_len) = TxType::from_bytes(&data[16..])?;
        Ok((Self { tx_type, nonce, chain_id }, 16 + tx_len))
    }

    /// Hash for signing (SHA-256 of canonical encoding).
    pub fn signing_hash(&self) -> Hash {
        let bytes = self.to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }
}

// ============================================================================
// Signed Transaction
// ============================================================================

/// Signed transaction with Ed25519 signature.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedTransaction {
    pub tx: Transaction,
    #[serde(with = "signature_serde")]
    pub signature: [u8; 64],   // Ed25519 signature
    pub public_key: [u8; 32],  // Sender's public key
}

impl SignedTransaction {
    /// Create a signed transaction (signature must be valid).
    pub fn new(tx: Transaction, signature: [u8; 64], public_key: [u8; 32]) -> Self {
        Self { tx, signature, public_key }
    }

    /// Get the transaction hash (for deduplication).
    pub fn hash(&self) -> Hash {
        let bytes = self.to_bytes();
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        hasher.finalize().into()
    }

    /// Sender address (just the public key for now).
    pub fn sender(&self) -> [u8; 32] {
        self.public_key
    }

    /// Canonical encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let tx_bytes = self.tx.to_bytes();
        let mut buf = Vec::with_capacity(96 + tx_bytes.len());
        buf.extend_from_slice(&self.signature);
        buf.extend_from_slice(&self.public_key);
        buf.extend_from_slice(&tx_bytes);
        buf
    }

    /// Decode from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        if data.len() < 96 {
            return Err(DecodeError::InsufficientData);
        }
        let signature: [u8; 64] = data[0..64].try_into().unwrap();
        let public_key: [u8; 32] = data[64..96].try_into().unwrap();
        let (tx, _) = Transaction::from_bytes(&data[96..])?;
        Ok(Self { tx, signature, public_key })
    }
}

// ============================================================================
// Block
// ============================================================================

/// Full block with header and transactions.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<SignedTransaction>,
}

impl Block {
    /// Create a new block.
    pub fn new(header: BlockHeader, transactions: Vec<SignedTransaction>) -> Self {
        Self { header, transactions }
    }

    /// Get the block hash.
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Compute Merkle root of transactions.
    pub fn compute_tx_root(transactions: &[SignedTransaction]) -> Hash {
        if transactions.is_empty() {
            return ZERO_HASH;
        }
        let mut hasher = Sha256::new();
        for tx in transactions {
            let tx_hash = tx.hash();
            hasher.update(&tx_hash);
        }
        hasher.finalize().into()
    }

    /// Canonical encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let header_bytes = self.header.to_bytes();
        let mut buf = Vec::new();
        buf.extend_from_slice(&header_bytes);
        buf.extend_from_slice(&(self.transactions.len() as u32).to_be_bytes());
        for tx in &self.transactions {
            let tx_bytes = tx.to_bytes();
            buf.extend_from_slice(&(tx_bytes.len() as u32).to_be_bytes());
            buf.extend_from_slice(&tx_bytes);
        }
        buf
    }

    /// Decode from bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, DecodeError> {
        let header = BlockHeader::from_bytes(data)?;
        // Header size depends on version
        let mut offset = if header.version >= 2 { 164 } else { 132 };

        if data.len() < offset + 4 {
            return Err(DecodeError::InsufficientData);
        }
        let tx_count = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
        offset += 4;

        let mut transactions = Vec::with_capacity(tx_count);
        for _ in 0..tx_count {
            if data.len() < offset + 4 {
                return Err(DecodeError::InsufficientData);
            }
            let tx_len = u32::from_be_bytes(data[offset..offset + 4].try_into().unwrap()) as usize;
            offset += 4;

            if data.len() < offset + tx_len {
                return Err(DecodeError::InsufficientData);
            }
            let tx = SignedTransaction::from_bytes(&data[offset..offset + tx_len])?;
            transactions.push(tx);
            offset += tx_len;
        }

        Ok(Self { header, transactions })
    }
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone, thiserror::Error)]
pub enum DecodeError {
    #[error("insufficient data")]
    InsufficientData,
    #[error("unknown tag")]
    UnknownTag,
    #[error("invalid data")]
    InvalidData,
}

// ============================================================================
// Display implementations
// ============================================================================

impl fmt::Display for BlockHeader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Block #{} ({})",
            self.height,
            hex::encode(&self.hash()[..4])
        )
    }
}

/// Simple hex encoding (avoid external dependency).
mod hex {
    pub fn encode(data: &[u8]) -> String {
        data.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_header_roundtrip() {
        let header = BlockHeader::new(
            1,
            1234567890,
            0x00_00_ff_ff_ff_ff_ff_ff,
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
        );
        let bytes = header.to_bytes();
        let decoded = BlockHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_transaction_roundtrip() {
        let tx = Transaction::new(
            TxType::ContractCall {
                contract_id: 1,
                method: 2,
                calldata: b"hello".to_vec(),
            },
            42,
        );
        let bytes = tx.to_bytes();
        let (decoded, _) = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_signed_transaction_roundtrip() {
        let tx = Transaction::new(
            TxType::ContractCall {
                contract_id: 1,
                method: 1,
                calldata: vec![],
            },
            0,
        );
        let signed = SignedTransaction::new(tx, [0u8; 64], [1u8; 32]);
        let bytes = signed.to_bytes();
        let decoded = SignedTransaction::from_bytes(&bytes).unwrap();
        assert_eq!(signed, decoded);
    }

    #[test]
    fn test_block_roundtrip() {
        let header = BlockHeader::new(0, 0, 0x00_ff_ff_ff_ff_ff_ff_ff, ZERO_HASH, ZERO_HASH, ZERO_HASH);
        let block = Block::new(header, vec![]);
        let bytes = block.to_bytes();
        let decoded = Block::from_bytes(&bytes).unwrap();
        assert_eq!(block, decoded);
    }

    #[test]
    fn test_hash_determinism() {
        let header = BlockHeader::new(1, 100, 1000, [0u8; 32], [0u8; 32], [0u8; 32]);
        let hash1 = header.hash();
        let hash2 = header.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_create_subreddit_roundtrip() {
        let tx = Transaction::new(
            TxType::CreateSubreddit {
                name: "science".to_string(),
                description: "A place for science discussion".to_string(),
                fee_amount: 1000,
            },
            1,
        );
        let bytes = tx.to_bytes();
        let (decoded, _) = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_subreddit_put_roundtrip() {
        let sub_id = SubredditId::derive(&[1u8; 32], "test", 100);
        let tx = Transaction::new(
            TxType::SubredditPut {
                subreddit: sub_id,
                entity_type: 1,
                entity_key: "apple".to_string(),
                property: 100,
                value: b"red fruit".to_vec(),
            },
            2,
        );
        let bytes = tx.to_bytes();
        let (decoded, _) = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_subreddit_delete_roundtrip() {
        let sub_id = SubredditId::derive(&[2u8; 32], "test", 200);
        let tx = Transaction::new(
            TxType::SubredditDelete {
                subreddit: sub_id,
                entity_type: 1,
                entity_key: "old_entry".to_string(),
                property: 50,
            },
            3,
        );
        let bytes = tx.to_bytes();
        let (decoded, _) = Transaction::from_bytes(&bytes).unwrap();
        assert_eq!(tx, decoded);
    }

    #[test]
    fn test_v2_block_header_roundtrip() {
        let subreddit_root = [42u8; 32];
        let header = BlockHeader::new_v2(
            100,
            1234567890,
            0x00_ff_ff_ff_ff_ff_ff_ff,
            [1u8; 32],
            [2u8; 32],
            [3u8; 32],
            subreddit_root,
        );

        assert!(header.is_v2());
        assert_eq!(header.version, 2);
        assert_eq!(header.subreddit_roots_root, subreddit_root);

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 164); // v2 headers are 164 bytes

        let decoded = BlockHeader::from_bytes(&bytes).unwrap();
        assert_eq!(header, decoded);
    }

    #[test]
    fn test_v1_block_header_has_zero_subreddit_root() {
        let header = BlockHeader::new(
            0,
            0,
            0x00_ff_ff_ff_ff_ff_ff_ff,
            ZERO_HASH,
            ZERO_HASH,
            ZERO_HASH,
        );

        assert!(!header.is_v2());
        assert_eq!(header.version, 1);
        assert_eq!(header.subreddit_roots_root, ZERO_HASH);

        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 132); // v1 headers are 132 bytes
    }

    #[test]
    fn test_v2_block_roundtrip() {
        let header = BlockHeader::new_v2(
            50,
            1000000,
            0x00_ff_ff_ff_ff_ff_ff_ff,
            ZERO_HASH,
            [1u8; 32],
            ZERO_HASH,
            [99u8; 32],
        );
        let block = Block::new(header, vec![]);

        let bytes = block.to_bytes();
        let decoded = Block::from_bytes(&bytes).unwrap();

        assert_eq!(block, decoded);
        assert!(decoded.header.is_v2());
        assert_eq!(decoded.header.subreddit_roots_root, [99u8; 32]);
    }
}
