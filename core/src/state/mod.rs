//! Module d'état pour ArchiveChain
//!
//! Gère l'état de la blockchain via des arbres de Merkle et une machine d'état

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub mod machine;
pub mod merkle;
pub mod storage;

pub use machine::{StateMachine, StateTransition};
pub use merkle::{MerkleTree, MerkleProof, MerkleNode};
pub use storage::{StateKey, StateValue};

use crate::crypto::Hash;
use crate::error::{CoreError, Result};

/// Type pour une racine d'état
pub type StateRoot = Hash;

/// Trait pour le stockage d'état - doit être Send + Sync pour la concurrence
#[async_trait]
pub trait StateStorage: Send + Sync {
    /// Lit une valeur depuis le stockage
    async fn get(&self, key: &StateKey) -> Result<Option<StateValue>>;
    
    /// Écrit une valeur dans le stockage
    async fn set(&mut self, key: StateKey, value: StateValue) -> Result<()>;
    
    /// Supprime une valeur du stockage
    async fn remove(&mut self, key: &StateKey) -> Result<bool>;
    
    /// Vérifie si une clé existe
    async fn contains(&self, key: &StateKey) -> Result<bool>;
    
    /// Obtient toutes les clés
    async fn keys(&self) -> Result<Vec<StateKey>>;
    
    /// Vide complètement le stockage
    async fn clear(&mut self) -> Result<()>;
    
    /// Calcule la racine d'état actuelle
    async fn calculate_state_root(&self) -> Result<StateRoot>;
    
    /// Crée un snapshot de l'état actuel
    async fn create_snapshot(&self) -> Result<StateSnapshot>;
    
    /// Restaure depuis un snapshot
    async fn restore_snapshot(&mut self, snapshot: StateSnapshot) -> Result<()>;
}

/// Structure pour un snapshot d'état
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSnapshot {
    /// Racine d'état au moment du snapshot
    pub state_root: StateRoot,
    /// Timestamp du snapshot
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Données sérialisées de l'état
    pub data: Vec<u8>,
}

/// Implémentation en mémoire du stockage d'état
#[derive(Debug)]
pub struct MemoryStateStorage {
    /// Stockage interne thread-safe
    storage: Arc<RwLock<HashMap<StateKey, StateValue>>>,
}

impl MemoryStateStorage {
    /// Crée une nouvelle instance de stockage en mémoire
    pub fn new() -> Self {
        Self {
            storage: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Obtient le nombre d'éléments stockés
    pub fn len(&self) -> usize {
        self.storage.read().unwrap().len()
    }
    
    /// Vérifie si le stockage est vide
    pub fn is_empty(&self) -> bool {
        self.storage.read().unwrap().is_empty()
    }
}

impl Default for MemoryStateStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StateStorage for MemoryStateStorage {
    async fn get(&self, key: &StateKey) -> Result<Option<StateValue>> {
        let storage = self.storage.read()
            .map_err(|_| CoreError::State("Failed to acquire read lock".to_string()))?;
        Ok(storage.get(key).cloned())
    }
    
    async fn set(&mut self, key: StateKey, value: StateValue) -> Result<()> {
        let mut storage = self.storage.write()
            .map_err(|_| CoreError::State("Failed to acquire write lock".to_string()))?;
        storage.insert(key, value);
        Ok(())
    }
    
    async fn remove(&mut self, key: &StateKey) -> Result<bool> {
        let mut storage = self.storage.write()
            .map_err(|_| CoreError::State("Failed to acquire write lock".to_string()))?;
        Ok(storage.remove(key).is_some())
    }
    
    async fn contains(&self, key: &StateKey) -> Result<bool> {
        let storage = self.storage.read()
            .map_err(|_| CoreError::State("Failed to acquire read lock".to_string()))?;
        Ok(storage.contains_key(key))
    }
    
    async fn keys(&self) -> Result<Vec<StateKey>> {
        let storage = self.storage.read()
            .map_err(|_| CoreError::State("Failed to acquire read lock".to_string()))?;
        Ok(storage.keys().cloned().collect())
    }
    
    async fn clear(&mut self) -> Result<()> {
        let mut storage = self.storage.write()
            .map_err(|_| CoreError::State("Failed to acquire write lock".to_string()))?;
        storage.clear();
        Ok(())
    }
    
    async fn calculate_state_root(&self) -> Result<StateRoot> {
        use crate::crypto::{compute_blake3, HashAlgorithm};
        
        let storage = self.storage.read()
            .map_err(|_| CoreError::State("Failed to acquire read lock".to_string()))?;
        
        if storage.is_empty() {
            return Ok(Hash::zero());
        }
        
        // Crée une représentation déterministe de l'état
        let mut pairs: Vec<(&StateKey, &StateValue)> = storage.iter().collect();
        pairs.sort_by_key(|(key, _)| *key);
        
        let mut state_data = Vec::new();
        for (key, value) in pairs {
            state_data.extend_from_slice(&key.to_bytes());
            state_data.extend_from_slice(&value.to_bytes());
        }
        
        Ok(Hash::from_bytes(compute_blake3(&state_data)))
    }
    
    async fn create_snapshot(&self) -> Result<StateSnapshot> {
        let storage = self.storage.read()
            .map_err(|_| CoreError::State("Failed to acquire read lock".to_string()))?;
        
        let state_root = self.calculate_state_root().await?;
        let timestamp = chrono::Utc::now();
        
        // Sérialise le stockage
        let data = bincode::serialize(&*storage)
            .map_err(|e| CoreError::Serialization(format!("Failed to serialize state: {}", e)))?;
        
        Ok(StateSnapshot {
            state_root,
            timestamp,
            data,
        })
    }
    
    async fn restore_snapshot(&mut self, snapshot: StateSnapshot) -> Result<()> {
        // Désérialise les données
        let storage_data: HashMap<StateKey, StateValue> = bincode::deserialize(&snapshot.data)
            .map_err(|e| CoreError::Serialization(format!("Failed to deserialize state: {}", e)))?;
        
        let mut storage = self.storage.write()
            .map_err(|_| CoreError::State("Failed to acquire write lock".to_string()))?;
        
        *storage = storage_data;
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_module_basic() {
        // Test basique pour vérifier que le module se compile
        assert!(true);
    }
}