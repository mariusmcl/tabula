use std::collections::BTreeMap;
use sha2::{Sha256, Digest};
use subreddit::SubredditId;

#[derive(Clone)]
pub struct KV {
    inner: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl KV {
    pub fn new() -> Self { Self { inner: BTreeMap::new() } }

    pub fn put(&mut self, k: Vec<u8>, v: Vec<u8>) { self.inner.insert(k, v); }

    pub fn get(&self, k: &[u8]) -> Option<&Vec<u8>> { self.inner.get(k) }

    pub fn delete(&mut self, k: &[u8]) -> Option<Vec<u8>> { self.inner.remove(k) }

    pub fn ordered_merkle_root(&self) -> [u8;32] {
        let mut h = Sha256::new();
        for (k, v) in &self.inner {
            h.update(&(k.len() as u64).to_be_bytes());
            h.update(k);
            h.update(&(v.len() as u64).to_be_bytes());
            h.update(v);
        }
        h.finalize().into()
    }

    pub fn keys_count(&self) -> usize { self.inner.len() }

    /// Create a snapshot (clone) for speculative execution.
    pub fn snapshot(&self) -> Self { self.clone() }

    /// Iterate over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&Vec<u8>, &Vec<u8>)> {
        self.inner.iter()
    }

    /// Serialize the KV store to bytes for persistence.
    ///
    /// Format: count(8) | [len(k)(8) | k | len(v)(8) | v]*
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        buf.extend_from_slice(&(self.inner.len() as u64).to_be_bytes());
        for (k, v) in &self.inner {
            buf.extend_from_slice(&(k.len() as u64).to_be_bytes());
            buf.extend_from_slice(k);
            buf.extend_from_slice(&(v.len() as u64).to_be_bytes());
            buf.extend_from_slice(v);
        }
        buf
    }

    /// Deserialize from bytes.
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }
        let count = u64::from_be_bytes(data[0..8].try_into().ok()?) as usize;
        let mut offset = 8;
        let mut inner = BTreeMap::new();

        for _ in 0..count {
            if data.len() < offset + 8 {
                return None;
            }
            let k_len = u64::from_be_bytes(data[offset..offset + 8].try_into().ok()?) as usize;
            offset += 8;

            if data.len() < offset + k_len {
                return None;
            }
            let k = data[offset..offset + k_len].to_vec();
            offset += k_len;

            if data.len() < offset + 8 {
                return None;
            }
            let v_len = u64::from_be_bytes(data[offset..offset + 8].try_into().ok()?) as usize;
            offset += 8;

            if data.len() < offset + v_len {
                return None;
            }
            let v = data[offset..offset + v_len].to_vec();
            offset += v_len;

            inner.insert(k, v);
        }

        Some(Self { inner })
    }

    /// Merge all entries from another KV into this one.
    pub fn merge(&mut self, other: &KV) {
        for (k, v) in &other.inner {
            self.inner.insert(k.clone(), v.clone());
        }
    }
}

impl Default for KV {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// PartitionedKV - Per-subreddit key-value storage
// ============================================================================

/// Partitioned KV store with per-subreddit state.
///
/// Each subreddit has its own KV store and merkle root. The combined state
/// is computed as a merkle tree of all subreddit roots.
#[derive(Clone)]
pub struct PartitionedKV {
    /// Per-subreddit state partitions.
    partitions: BTreeMap<SubredditId, KV>,
    /// Global registry (subreddit definitions, entity types).
    /// Uses SubredditId::GLOBAL as its partition.
    pub global: KV,
}

impl PartitionedKV {
    /// Create a new partitioned KV store.
    pub fn new() -> Self {
        Self {
            partitions: BTreeMap::new(),
            global: KV::new(),
        }
    }

    /// Get or create a partition for a subreddit.
    pub fn partition_mut(&mut self, subreddit: &SubredditId) -> &mut KV {
        self.partitions.entry(*subreddit).or_insert_with(KV::new)
    }

    /// Get a partition for reading (if it exists).
    pub fn partition(&self, subreddit: &SubredditId) -> Option<&KV> {
        self.partitions.get(subreddit)
    }

    /// Put a value in a subreddit's partition.
    pub fn put(&mut self, subreddit: &SubredditId, key: Vec<u8>, value: Vec<u8>) {
        self.partition_mut(subreddit).put(key, value);
    }

    /// Get a value from a subreddit's partition.
    pub fn get(&self, subreddit: &SubredditId, key: &[u8]) -> Option<&Vec<u8>> {
        self.partitions.get(subreddit)?.get(key)
    }

    /// Delete a value from a subreddit's partition.
    pub fn delete(&mut self, subreddit: &SubredditId, key: &[u8]) -> Option<Vec<u8>> {
        self.partitions.get_mut(subreddit)?.delete(key)
    }

    /// Compute the merkle root for a specific subreddit.
    pub fn subreddit_root(&self, subreddit: &SubredditId) -> [u8; 32] {
        self.partitions
            .get(subreddit)
            .map(|kv| kv.ordered_merkle_root())
            .unwrap_or([0u8; 32])
    }

    /// Compute the combined state root.
    ///
    /// This is a merkle tree of:
    /// - Global registry root
    /// - All subreddit roots (sorted by SubredditId)
    pub fn combined_state_root(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        // Include global registry
        hasher.update(b"global:");
        hasher.update(&self.global.ordered_merkle_root());

        // Include all subreddit roots in sorted order
        for (sub_id, kv) in &self.partitions {
            hasher.update(sub_id.as_bytes());
            hasher.update(&kv.ordered_merkle_root());
        }

        hasher.finalize().into()
    }

    /// Get the merkle root of all subreddit roots (for partial validation).
    ///
    /// This allows nodes to verify the subreddit_roots_root in block headers
    /// without having all subreddit data.
    pub fn subreddit_roots_root(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();

        for (sub_id, kv) in &self.partitions {
            hasher.update(sub_id.as_bytes());
            hasher.update(&kv.ordered_merkle_root());
        }

        hasher.finalize().into()
    }

    /// List all subreddit IDs that have data.
    pub fn subreddit_ids(&self) -> Vec<SubredditId> {
        self.partitions.keys().copied().collect()
    }

    /// Check if a subreddit has any data.
    pub fn has_subreddit(&self, subreddit: &SubredditId) -> bool {
        self.partitions.contains_key(subreddit)
    }

    /// Get the total number of entries across all partitions.
    pub fn total_entries(&self) -> usize {
        self.global.keys_count()
            + self.partitions.values().map(|kv| kv.keys_count()).sum::<usize>()
    }

    /// Create a snapshot for speculative execution.
    pub fn snapshot(&self) -> Self {
        Self {
            partitions: self.partitions.clone(),
            global: self.global.clone(),
        }
    }

    /// Get the legacy partition (SubredditId::LEGACY).
    /// This is used for backward compatibility with pre-subreddit data.
    pub fn legacy_partition(&self) -> Option<&KV> {
        self.partitions.get(&SubredditId::LEGACY)
    }

    /// Get or create the legacy partition.
    pub fn legacy_partition_mut(&mut self) -> &mut KV {
        self.partition_mut(&SubredditId::LEGACY)
    }

    /// Migrate a flat KV store to the legacy partition.
    /// Used for upgrading existing data to the partitioned model.
    pub fn migrate_from_flat(&mut self, flat: &KV) {
        let legacy = self.legacy_partition_mut();
        legacy.merge(flat);
    }
}

impl Default for PartitionedKV {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut kv = KV::new();
        kv.put(b"key1".to_vec(), b"value1".to_vec());
        kv.put(b"key2".to_vec(), b"value2".to_vec());

        let bytes = kv.to_bytes();
        let restored = KV::from_bytes(&bytes).unwrap();

        assert_eq!(restored.get(b"key1"), Some(&b"value1".to_vec()));
        assert_eq!(restored.get(b"key2"), Some(&b"value2".to_vec()));
        assert_eq!(kv.ordered_merkle_root(), restored.ordered_merkle_root());
    }

    #[test]
    fn test_snapshot() {
        let mut kv = KV::new();
        kv.put(b"key".to_vec(), b"value".to_vec());

        let snapshot = kv.snapshot();
        kv.put(b"key2".to_vec(), b"value2".to_vec());

        // Original has both keys
        assert_eq!(kv.keys_count(), 2);
        // Snapshot only has original key
        assert_eq!(snapshot.keys_count(), 1);
    }

    #[test]
    fn test_partitioned_kv_basic() {
        let mut pkv = PartitionedKV::new();
        let sub1 = SubredditId::derive(&[1u8; 32], "science", 100);
        let sub2 = SubredditId::derive(&[1u8; 32], "cooking", 100);

        // Put values in different partitions
        pkv.put(&sub1, b"key1".to_vec(), b"science_data".to_vec());
        pkv.put(&sub2, b"key1".to_vec(), b"cooking_data".to_vec());

        // Values are isolated by partition
        assert_eq!(pkv.get(&sub1, b"key1"), Some(&b"science_data".to_vec()));
        assert_eq!(pkv.get(&sub2, b"key1"), Some(&b"cooking_data".to_vec()));
        assert_eq!(pkv.get(&sub1, b"key2"), None);
    }

    #[test]
    fn test_partitioned_kv_roots() {
        let mut pkv = PartitionedKV::new();
        let sub1 = SubredditId::derive(&[1u8; 32], "test1", 0);
        let sub2 = SubredditId::derive(&[1u8; 32], "test2", 0);

        // Empty partition has zero root
        assert_eq!(pkv.subreddit_root(&sub1), [0u8; 32]);

        // Add data to sub1
        pkv.put(&sub1, b"key".to_vec(), b"value".to_vec());
        let root1 = pkv.subreddit_root(&sub1);
        assert_ne!(root1, [0u8; 32]);

        // Add data to sub2 - different root
        pkv.put(&sub2, b"key".to_vec(), b"value".to_vec());
        let root2 = pkv.subreddit_root(&sub2);
        assert_eq!(root1, root2); // Same content = same root

        // Combined root should be deterministic
        let combined1 = pkv.combined_state_root();
        let combined2 = pkv.combined_state_root();
        assert_eq!(combined1, combined2);
    }

    #[test]
    fn test_partitioned_kv_snapshot() {
        let mut pkv = PartitionedKV::new();
        let sub = SubredditId::derive(&[1u8; 32], "test", 0);

        pkv.put(&sub, b"key1".to_vec(), b"value1".to_vec());
        let snapshot = pkv.snapshot();

        // Modify original
        pkv.put(&sub, b"key2".to_vec(), b"value2".to_vec());

        // Snapshot should be unchanged
        assert_eq!(snapshot.get(&sub, b"key2"), None);
        assert_eq!(pkv.get(&sub, b"key2"), Some(&b"value2".to_vec()));
    }

    #[test]
    fn test_legacy_migration() {
        let mut flat = KV::new();
        flat.put(b"old_key".to_vec(), b"old_value".to_vec());

        let mut pkv = PartitionedKV::new();
        pkv.migrate_from_flat(&flat);

        // Data should be in legacy partition
        assert!(pkv.has_subreddit(&SubredditId::LEGACY));
        assert_eq!(
            pkv.get(&SubredditId::LEGACY, b"old_key"),
            Some(&b"old_value".to_vec())
        );
    }
}