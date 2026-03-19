//! Transaction mempool: pending transactions awaiting block inclusion.

use std::collections::{BTreeMap, HashMap, HashSet};
use types::{Hash, SignedTransaction};
use crypto::verify;

/// Mempool error types.
#[derive(Debug, Clone, thiserror::Error)]
pub enum MempoolError {
    #[error("duplicate transaction")]
    DuplicateTransaction,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("pool is full")]
    PoolFull,
    #[error("nonce too low (expected >= {expected}, got {got})")]
    NonceTooLow { expected: u64, got: u64 },
}

/// Transaction mempool.
pub struct Mempool {
    /// All transactions by hash.
    txs: HashMap<Hash, SignedTransaction>,
    /// Transactions by sender, ordered by nonce.
    by_sender: HashMap<[u8; 32], BTreeMap<u64, Hash>>,
    /// Set of transaction hashes for fast lookup.
    tx_hashes: HashSet<Hash>,
    /// Maximum pool size.
    max_size: usize,
    /// Minimum nonce per sender (to reject old txs).
    min_nonces: HashMap<[u8; 32], u64>,
}

impl Mempool {
    /// Create a new mempool with given maximum size.
    pub fn new(max_size: usize) -> Self {
        Self {
            txs: HashMap::new(),
            by_sender: HashMap::new(),
            tx_hashes: HashSet::new(),
            max_size,
            min_nonces: HashMap::new(),
        }
    }

    /// Get the number of pending transactions.
    pub fn len(&self) -> usize {
        self.txs.len()
    }

    /// Check if the mempool is empty.
    pub fn is_empty(&self) -> bool {
        self.txs.is_empty()
    }

    /// Set the minimum nonce for a sender (after block inclusion).
    pub fn set_min_nonce(&mut self, sender: [u8; 32], nonce: u64) {
        self.min_nonces.insert(sender, nonce);

        // Remove any transactions with lower nonces
        if let Some(sender_txs) = self.by_sender.get_mut(&sender) {
            let to_remove: Vec<u64> = sender_txs
                .range(..nonce)
                .map(|(n, _)| *n)
                .collect();

            for n in to_remove {
                if let Some(hash) = sender_txs.remove(&n) {
                    self.txs.remove(&hash);
                    self.tx_hashes.remove(&hash);
                }
            }
        }
    }

    /// Add a transaction to the pool.
    pub fn add(&mut self, tx: SignedTransaction) -> Result<Hash, MempoolError> {
        let hash = tx.hash();

        // Check for duplicate
        if self.tx_hashes.contains(&hash) {
            return Err(MempoolError::DuplicateTransaction);
        }

        // Check pool size
        if self.txs.len() >= self.max_size {
            return Err(MempoolError::PoolFull);
        }

        // Verify signature
        let signing_hash = tx.tx.signing_hash();
        if !verify(&tx.public_key, &signing_hash, &tx.signature) {
            return Err(MempoolError::InvalidSignature);
        }

        // Check nonce
        let sender = tx.sender();
        let min_nonce = self.min_nonces.get(&sender).copied().unwrap_or(0);
        if tx.tx.nonce < min_nonce {
            return Err(MempoolError::NonceTooLow {
                expected: min_nonce,
                got: tx.tx.nonce,
            });
        }

        // Add to structures
        self.tx_hashes.insert(hash);
        self.by_sender
            .entry(sender)
            .or_default()
            .insert(tx.tx.nonce, hash);
        self.txs.insert(hash, tx);

        Ok(hash)
    }

    /// Remove transactions by hash.
    pub fn remove(&mut self, tx_hashes: &[Hash]) {
        for hash in tx_hashes {
            if let Some(tx) = self.txs.remove(hash) {
                self.tx_hashes.remove(hash);
                let sender = tx.sender();
                if let Some(sender_txs) = self.by_sender.get_mut(&sender) {
                    sender_txs.remove(&tx.tx.nonce);
                    if sender_txs.is_empty() {
                        self.by_sender.remove(&sender);
                    }
                }
            }
        }
    }

    /// Check if a transaction exists in the pool.
    pub fn contains(&self, hash: &Hash) -> bool {
        self.tx_hashes.contains(hash)
    }

    /// Get a transaction by hash.
    pub fn get(&self, hash: &Hash) -> Option<&SignedTransaction> {
        self.txs.get(hash)
    }

    /// Get pending transactions for block building.
    ///
    /// Returns transactions ordered by sender, then by nonce (for proper sequencing).
    pub fn get_pending(&self, limit: usize) -> Vec<SignedTransaction> {
        let mut result = Vec::with_capacity(limit.min(self.txs.len()));

        // Iterate through senders and get transactions in nonce order
        for (_sender, nonce_map) in &self.by_sender {
            for (_nonce, hash) in nonce_map {
                if result.len() >= limit {
                    return result;
                }
                if let Some(tx) = self.txs.get(hash) {
                    result.push(tx.clone());
                }
            }
        }

        result
    }

    /// Get all transaction hashes.
    pub fn all_hashes(&self) -> Vec<Hash> {
        self.tx_hashes.iter().copied().collect()
    }

    /// Get the next available nonce for a sender.
    ///
    /// This considers both the minimum nonce (from confirmed transactions)
    /// and any pending transactions in the mempool.
    pub fn next_nonce(&self, sender: &[u8; 32], chain_nonce: u64) -> u64 {
        let min_nonce = self.min_nonces.get(sender).copied().unwrap_or(0);
        let base_nonce = chain_nonce.max(min_nonce);

        // Check if there are pending transactions for this sender
        if let Some(sender_txs) = self.by_sender.get(sender) {
            if let Some((&max_pending_nonce, _)) = sender_txs.last_key_value() {
                // Next nonce is one more than the highest pending
                return (max_pending_nonce + 1).max(base_nonce);
            }
        }

        base_nonce
    }

    /// Clear all transactions.
    pub fn clear(&mut self) {
        self.txs.clear();
        self.by_sender.clear();
        self.tx_hashes.clear();
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use types::{Transaction, TxType, CHAIN_ID};
    use crypto::Keypair;

    fn create_signed_tx(keypair: &Keypair, nonce: u64) -> SignedTransaction {
        let tx = Transaction {
            tx_type: TxType::ContractCall {
                contract_id: 1,
                method: 1,
                calldata: vec![],
            },
            nonce,
            chain_id: CHAIN_ID,
        };
        let signing_hash = tx.signing_hash();
        let signature = keypair.sign(&signing_hash);
        SignedTransaction::new(tx, signature, keypair.public_key())
    }

    #[test]
    fn test_add_and_get() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = create_signed_tx(&kp, 0);
        let hash = pool.add(tx.clone()).unwrap();

        assert!(pool.contains(&hash));
        assert_eq!(pool.len(), 1);
        assert_eq!(pool.get(&hash).unwrap(), &tx);
    }

    #[test]
    fn test_duplicate_rejected() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = create_signed_tx(&kp, 0);

        pool.add(tx.clone()).unwrap();
        let result = pool.add(tx);
        assert!(matches!(result, Err(MempoolError::DuplicateTransaction)));
    }

    #[test]
    fn test_invalid_signature_rejected() {
        let mut pool = Mempool::new(100);
        let kp1 = Keypair::generate();
        let kp2 = Keypair::generate();

        // Sign with kp1 but use kp2's public key
        let tx = Transaction {
            tx_type: TxType::ContractCall {
                contract_id: 1,
                method: 1,
                calldata: vec![],
            },
            nonce: 0,
            chain_id: CHAIN_ID,
        };
        let signature = kp1.sign(&tx.signing_hash());
        let bad_tx = SignedTransaction::new(tx, signature, kp2.public_key());

        let result = pool.add(bad_tx);
        assert!(matches!(result, Err(MempoolError::InvalidSignature)));
    }

    #[test]
    fn test_pool_full() {
        let mut pool = Mempool::new(2);
        let kp = Keypair::generate();

        pool.add(create_signed_tx(&kp, 0)).unwrap();
        pool.add(create_signed_tx(&kp, 1)).unwrap();

        let result = pool.add(create_signed_tx(&kp, 2));
        assert!(matches!(result, Err(MempoolError::PoolFull)));
    }

    #[test]
    fn test_remove() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();
        let tx = create_signed_tx(&kp, 0);
        let hash = pool.add(tx).unwrap();

        assert!(pool.contains(&hash));
        pool.remove(&[hash]);
        assert!(!pool.contains(&hash));
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_get_pending_ordered() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();

        // Add in reverse order
        pool.add(create_signed_tx(&kp, 2)).unwrap();
        pool.add(create_signed_tx(&kp, 0)).unwrap();
        pool.add(create_signed_tx(&kp, 1)).unwrap();

        let pending = pool.get_pending(10);
        assert_eq!(pending.len(), 3);
        // Should be sorted by nonce
        assert_eq!(pending[0].tx.nonce, 0);
        assert_eq!(pending[1].tx.nonce, 1);
        assert_eq!(pending[2].tx.nonce, 2);
    }

    #[test]
    fn test_nonce_too_low() {
        let mut pool = Mempool::new(100);
        let kp = Keypair::generate();

        pool.set_min_nonce(kp.public_key(), 5);

        let result = pool.add(create_signed_tx(&kp, 3));
        assert!(matches!(result, Err(MempoolError::NonceTooLow { .. })));

        // Nonce 5 should work
        pool.add(create_signed_tx(&kp, 5)).unwrap();
    }
}
