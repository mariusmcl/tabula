//! Subreddit types for topic-based selective storage.
//!
//! Subreddits are user-created topic partitions that allow nodes to choose
//! which data to store. Each subreddit has its own state partition.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

// ============================================================================
// SubredditId
// ============================================================================

/// Unique identifier for a subreddit.
///
/// Derived from: sha256(b"subreddit:" || creator_pubkey || name || height)
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SubredditId(pub [u8; 32]);

impl SubredditId {
    /// The legacy subreddit ID (all zeros) for pre-subreddit data.
    pub const LEGACY: SubredditId = SubredditId([0u8; 32]);

    /// The global/system subreddit ID for registry data.
    /// Uses a specific pattern to distinguish from LEGACY.
    pub const GLOBAL: SubredditId = SubredditId([
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
        0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
    ]);

    /// Derive a subreddit ID from creator, name, and creation height.
    pub fn derive(creator: &[u8; 32], name: &str, height: u64) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"subreddit:");
        hasher.update(creator);
        hasher.update(name.to_ascii_lowercase().as_bytes());
        hasher.update(&height.to_be_bytes());
        SubredditId(hasher.finalize().into())
    }

    /// Check if this is the legacy subreddit.
    pub fn is_legacy(&self) -> bool {
        *self == Self::LEGACY
    }

    /// Check if this is the global/system subreddit.
    pub fn is_global(&self) -> bool {
        *self == Self::GLOBAL
    }

    /// Get the first 8 bytes as hex for topic names.
    pub fn short_hex(&self) -> String {
        hex_encode(&self.0[..8])
    }

    /// Convert to bytes.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Create from bytes.
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        SubredditId(bytes)
    }
}

impl fmt::Debug for SubredditId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_legacy() {
            write!(f, "SubredditId(LEGACY)")
        } else if self.is_global() {
            write!(f, "SubredditId(GLOBAL)")
        } else {
            write!(f, "SubredditId({})", self.short_hex())
        }
    }
}

impl fmt::Display for SubredditId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_legacy() {
            write!(f, "legacy")
        } else if self.is_global() {
            write!(f, "global")
        } else {
            write!(f, "{}", self.short_hex())
        }
    }
}

// ============================================================================
// SubredditMeta
// ============================================================================

/// Metadata about a subreddit stored in the global registry.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubredditMeta {
    /// Unique identifier.
    pub id: SubredditId,
    /// Human-readable name (lowercase, alphanumeric + underscore).
    pub name: String,
    /// Creator's public key.
    pub creator: [u8; 32],
    /// Block height when created.
    pub created_at: u64,
    /// Short description.
    pub description: String,
}

impl SubredditMeta {
    /// Create new subreddit metadata.
    pub fn new(
        creator: [u8; 32],
        name: String,
        description: String,
        created_at: u64,
    ) -> Self {
        let id = SubredditId::derive(&creator, &name, created_at);
        Self {
            id,
            name: name.to_ascii_lowercase(),
            creator,
            created_at,
            description,
        }
    }

    /// Validate the subreddit name.
    pub fn validate_name(name: &str) -> Result<(), SubredditError> {
        if name.is_empty() {
            return Err(SubredditError::EmptyName);
        }
        if name.len() > 32 {
            return Err(SubredditError::NameTooLong);
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(SubredditError::InvalidNameChars);
        }
        if name.starts_with('_') || name.ends_with('_') {
            return Err(SubredditError::InvalidNameFormat);
        }
        // Reserved names
        let reserved = ["legacy", "global", "system", "admin", "mod"];
        if reserved.contains(&name.to_ascii_lowercase().as_str()) {
            return Err(SubredditError::ReservedName);
        }
        Ok(())
    }

    /// Canonical binary encoding.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        // ID (32 bytes)
        buf.extend_from_slice(&self.id.0);
        // Creator (32 bytes)
        buf.extend_from_slice(&self.creator);
        // Created at (8 bytes)
        buf.extend_from_slice(&self.created_at.to_be_bytes());
        // Name length + name
        buf.extend_from_slice(&(self.name.len() as u16).to_be_bytes());
        buf.extend_from_slice(self.name.as_bytes());
        // Description length + description
        buf.extend_from_slice(&(self.description.len() as u16).to_be_bytes());
        buf.extend_from_slice(self.description.as_bytes());
        buf
    }

    /// Decode from canonical bytes.
    pub fn from_bytes(data: &[u8]) -> Result<Self, SubredditError> {
        if data.len() < 76 {
            return Err(SubredditError::InsufficientData);
        }

        let id = SubredditId::from_bytes(data[0..32].try_into().unwrap());
        let creator: [u8; 32] = data[32..64].try_into().unwrap();
        let created_at = u64::from_be_bytes(data[64..72].try_into().unwrap());

        let name_len = u16::from_be_bytes(data[72..74].try_into().unwrap()) as usize;
        if data.len() < 74 + name_len + 2 {
            return Err(SubredditError::InsufficientData);
        }
        let name = String::from_utf8(data[74..74 + name_len].to_vec())
            .map_err(|_| SubredditError::InvalidUtf8)?;

        let desc_start = 74 + name_len;
        let desc_len = u16::from_be_bytes(data[desc_start..desc_start + 2].try_into().unwrap()) as usize;
        if data.len() < desc_start + 2 + desc_len {
            return Err(SubredditError::InsufficientData);
        }
        let description = String::from_utf8(data[desc_start + 2..desc_start + 2 + desc_len].to_vec())
            .map_err(|_| SubredditError::InvalidUtf8)?;

        Ok(Self {
            id,
            name,
            creator,
            created_at,
            description,
        })
    }
}

// ============================================================================
// Key Format Helpers
// ============================================================================

/// Separator between subreddit ID and entity key.
pub const SUBREDDIT_SEPARATOR: u8 = 0x1E;

/// Separator between entity type, key, and property (from entity crate).
pub const FIELD_SEPARATOR: u8 = 0x1F;

/// Build a subreddit-prefixed key.
pub fn prefixed_key(
    subreddit: &SubredditId,
    entity_type: u32,
    entity_key: &str,
    property: u32,
) -> Vec<u8> {
    let mut key = Vec::with_capacity(32 + 1 + 4 + 1 + entity_key.len() + 1 + 4);
    key.extend_from_slice(&subreddit.0);
    key.push(SUBREDDIT_SEPARATOR);
    key.extend_from_slice(&entity_type.to_be_bytes());
    key.push(FIELD_SEPARATOR);
    key.extend_from_slice(entity_key.to_ascii_lowercase().as_bytes());
    key.push(FIELD_SEPARATOR);
    key.extend_from_slice(&property.to_be_bytes());
    key
}

/// Extract subreddit ID from a prefixed key.
pub fn extract_subreddit(key: &[u8]) -> Option<SubredditId> {
    if key.len() < 33 || key[32] != SUBREDDIT_SEPARATOR {
        return None;
    }
    Some(SubredditId::from_bytes(key[0..32].try_into().ok()?))
}

/// Check if a key belongs to a specific subreddit.
pub fn key_matches_subreddit(key: &[u8], subreddit: &SubredditId) -> bool {
    key.len() >= 33
        && key[32] == SUBREDDIT_SEPARATOR
        && &key[0..32] == subreddit.as_bytes()
}

// ============================================================================
// Errors
// ============================================================================

#[derive(Debug, Clone, thiserror::Error)]
pub enum SubredditError {
    #[error("subreddit name is empty")]
    EmptyName,
    #[error("subreddit name too long (max 32 chars)")]
    NameTooLong,
    #[error("subreddit name contains invalid characters")]
    InvalidNameChars,
    #[error("subreddit name format invalid")]
    InvalidNameFormat,
    #[error("subreddit name is reserved")]
    ReservedName,
    #[error("insufficient data for decoding")]
    InsufficientData,
    #[error("invalid UTF-8 in subreddit data")]
    InvalidUtf8,
}

// ============================================================================
// Hex Helpers
// ============================================================================

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02x}", b)).collect()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subreddit_id_derive() {
        let creator = [1u8; 32];
        let id1 = SubredditId::derive(&creator, "science", 100);
        let id2 = SubredditId::derive(&creator, "Science", 100); // case insensitive
        let id3 = SubredditId::derive(&creator, "science", 101); // different height

        assert_eq!(id1, id2);
        assert_ne!(id1, id3);
    }

    #[test]
    fn test_subreddit_id_special() {
        assert!(SubredditId::LEGACY.is_legacy());
        assert!(!SubredditId::LEGACY.is_global());
        assert!(SubredditId::GLOBAL.is_global());
        assert!(!SubredditId::GLOBAL.is_legacy());
    }

    #[test]
    fn test_subreddit_meta_roundtrip() {
        let meta = SubredditMeta::new(
            [42u8; 32],
            "test_subreddit".to_string(),
            "A test subreddit".to_string(),
            12345,
        );

        let bytes = meta.to_bytes();
        let decoded = SubredditMeta::from_bytes(&bytes).unwrap();

        assert_eq!(meta, decoded);
    }

    #[test]
    fn test_validate_name() {
        assert!(SubredditMeta::validate_name("science").is_ok());
        assert!(SubredditMeta::validate_name("food_recipes").is_ok());
        assert!(SubredditMeta::validate_name("test123").is_ok());

        assert!(SubredditMeta::validate_name("").is_err());
        assert!(SubredditMeta::validate_name("a".repeat(33).as_str()).is_err());
        assert!(SubredditMeta::validate_name("no spaces").is_err());
        assert!(SubredditMeta::validate_name("_invalid").is_err());
        assert!(SubredditMeta::validate_name("legacy").is_err());
    }

    #[test]
    fn test_prefixed_key() {
        let sub = SubredditId::derive(&[1u8; 32], "test", 0);
        let key = prefixed_key(&sub, 1, "apple", 100);

        assert!(key_matches_subreddit(&key, &sub));
        assert_eq!(extract_subreddit(&key), Some(sub));
    }
}
