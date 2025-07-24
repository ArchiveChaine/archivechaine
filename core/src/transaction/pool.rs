//! Pool de transactions pour ArchiveChain

use std::collections::HashMap;
use crate::crypto::Hash;
use crate::error::{TransactionError, Result};
use super::types::Transaction;

/// Pool de transactions en attente
#[derive(Debug, Clone)]
pub struct TransactionPool {
    /// Transactions en attente, indexées par hash
    pending: HashMap<Hash, Transaction>,
    /// Nombre maximum de transactions dans le pool
    max_size: usize,
}

impl TransactionPool {
    /// Crée un nouveau pool
    pub fn new(max_size: usize) -> Self {
        Self {
            pending: HashMap::new(),
            max_size,
        }
    }

    /// Ajoute une transaction au pool
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        if self.pending.len() >= self.max_size {
            return Err(TransactionError::Invalid.into());
        }

        if !transaction.is_valid()? {
            return Err(TransactionError::Invalid.into());
        }

        self.pending.insert(transaction.tx_id.clone(), transaction);
        Ok(())
    }

    /// Retire une transaction du pool
    pub fn remove_transaction(&mut self, tx_id: &Hash) -> Option<Transaction> {
        self.pending.remove(tx_id)
    }

    /// Obtient une transaction par son ID
    pub fn get_transaction(&self, tx_id: &Hash) -> Option<&Transaction> {
        self.pending.get(tx_id)
    }

    /// Obtient toutes les transactions en attente
    pub fn pending_transactions(&self) -> Vec<&Transaction> {
        self.pending.values().collect()
    }

    /// Vide le pool
    pub fn clear(&mut self) {
        self.pending.clear();
    }

    /// Retourne la taille du pool
    pub fn size(&self) -> usize {
        self.pending.len()
    }

    /// Vérifie si le pool est plein
    pub fn is_full(&self) -> bool {
        self.pending.len() >= self.max_size
    }
}

impl Default for TransactionPool {
    fn default() -> Self {
        Self::new(10000) // Pool par défaut de 10k transactions
    }
}