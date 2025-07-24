//! Stockage d'état pour ArchiveChain

use std::collections::HashMap;
use crate::crypto::Hash;
use crate::error::{StateError, Result};

/// Clé d'état
pub type StateKey = Hash;

/// Valeur d'état
pub type StateValue = Vec<u8>;

/// Interface de stockage d'état
pub trait StateStorage {
    /// Lit une valeur
    fn get(&self, key: &StateKey) -> Result<Option<StateValue>>;
    
    /// Écrit une valeur
    fn set(&mut self, key: StateKey, value: StateValue) -> Result<()>;
    
    /// Supprime une valeur
    fn remove(&mut self, key: &StateKey) -> Result<Option<StateValue>>;
    
    /// Vérifie l'existence d'une clé
    fn contains(&self, key: &StateKey) -> bool;
    
    /// Obtient toutes les clés
    fn keys(&self) -> Vec<StateKey>;
}

/// Implémentation en mémoire du stockage d'état
#[derive(Debug, Clone)]
pub struct MemoryStateStorage {
    data: HashMap<StateKey, StateValue>,
}

impl MemoryStateStorage {
    /// Crée un nouveau stockage en mémoire
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
    
    /// Obtient le nombre d'entrées
    pub fn len(&self) -> usize {
        self.data.len()
    }
    
    /// Vérifie si le stockage est vide
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
    
    /// Vide le stockage
    pub fn clear(&mut self) {
        self.data.clear();
    }
}

impl StateStorage for MemoryStateStorage {
    fn get(&self, key: &StateKey) -> Result<Option<StateValue>> {
        Ok(self.data.get(key).cloned())
    }
    
    fn set(&mut self, key: StateKey, value: StateValue) -> Result<()> {
        self.data.insert(key, value);
        Ok(())
    }
    
    fn remove(&mut self, key: &StateKey) -> Result<Option<StateValue>> {
        Ok(self.data.remove(key))
    }
    
    fn contains(&self, key: &StateKey) -> bool {
        self.data.contains_key(key)
    }
    
    fn keys(&self) -> Vec<StateKey> {
        self.data.keys().cloned().collect()
    }
}

impl Default for MemoryStateStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    #[test]
    fn test_memory_storage() {
        let mut storage = MemoryStateStorage::new();
        let key = Hash::zero();
        let value = b"test value".to_vec();
        
        assert!(storage.get(&key).unwrap().is_none());
        
        storage.set(key.clone(), value.clone()).unwrap();
        assert_eq!(storage.get(&key).unwrap(), Some(value.clone()));
        
        let removed = storage.remove(&key).unwrap();
        assert_eq!(removed, Some(value));
        assert!(storage.get(&key).unwrap().is_none());
    }
}