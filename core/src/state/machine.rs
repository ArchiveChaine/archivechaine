//! Machine d'état pour ArchiveChain

use std::collections::HashMap;
use crate::crypto::Hash;
use crate::error::{StateError, Result};

/// Clé d'état dans la machine d'état
pub type StateKey = Hash;

/// Valeur d'état
pub type StateValue = Vec<u8>;

/// Transition d'état
#[derive(Debug, Clone)]
pub struct StateTransition {
    /// Clé modifiée
    pub key: StateKey,
    /// Ancienne valeur (None si nouvelle clé)
    pub old_value: Option<StateValue>,
    /// Nouvelle valeur (None si suppression)
    pub new_value: Option<StateValue>,
}

/// Machine d'état déterministe
#[derive(Debug, Clone)]
pub struct StateMachine {
    /// État actuel (en mémoire pour le core)
    state: HashMap<StateKey, StateValue>,
    /// Historique des transitions
    transitions: Vec<StateTransition>,
}

impl StateMachine {
    /// Crée une nouvelle machine d'état
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            transitions: Vec::new(),
        }
    }

    /// Applique une transition d'état
    pub fn apply_transition(&mut self, transition: StateTransition) -> Result<()> {
        let old_value = self.state.get(&transition.key).cloned();
        
        match &transition.new_value {
            Some(value) => {
                self.state.insert(transition.key.clone(), value.clone());
            }
            None => {
                self.state.remove(&transition.key);
            }
        }
        
        self.transitions.push(transition);
        Ok(())
    }

    /// Obtient une valeur d'état
    pub fn get(&self, key: &StateKey) -> Option<&StateValue> {
        self.state.get(key)
    }

    /// Définit une valeur d'état
    pub fn set(&mut self, key: StateKey, value: StateValue) -> Result<()> {
        let old_value = self.state.get(&key).cloned();
        let transition = StateTransition {
            key: key.clone(),
            old_value,
            new_value: Some(value.clone()),
        };
        
        self.apply_transition(transition)
    }

    /// Supprime une valeur d'état
    pub fn remove(&mut self, key: &StateKey) -> Result<Option<StateValue>> {
        let old_value = self.state.get(key).cloned();
        
        if old_value.is_some() {
            let transition = StateTransition {
                key: key.clone(),
                old_value: old_value.clone(),
                new_value: None,
            };
            self.apply_transition(transition)?;
        }
        
        Ok(old_value)
    }

    /// Obtient toutes les clés
    pub fn keys(&self) -> Vec<&StateKey> {
        self.state.keys().collect()
    }

    /// Obtient le nombre d'entrées
    pub fn len(&self) -> usize {
        self.state.len()
    }

    /// Vérifie si l'état est vide
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }
}

impl Default for StateMachine {
    fn default() -> Self {
        Self::new()
    }
}