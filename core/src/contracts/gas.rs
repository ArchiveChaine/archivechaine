//! Système de gas pour les smart contracts ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::contracts::{ContractError, ContractResult};

/// Limite de gas par défaut
pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;

/// Prix de gas minimum
pub const MIN_GAS_PRICE: u64 = 1;

/// Coûts de gas pour différentes opérations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GasCost {
    /// Opération de base (addition, multiplication, etc.)
    Basic = 1,
    /// Accès mémoire
    Memory = 2,
    /// Lecture du storage
    StorageRead = 50,
    /// Écriture dans le storage
    StorageWrite = 200,
    /// Suppression du storage
    StorageDelete = 100,
    /// Émission d'un log
    Log = 25,
    /// Émission d'un event
    Event = 100,
    /// Appel de fonction
    FunctionCall = 10,
    /// Transfert de tokens
    Transfer = 300,
    /// Calcul de hash
    Hash = 30,
    /// Vérification de signature
    SignatureVerify = 500,
    /// Appel de contrat externe
    ExternalCall = 1000,
    /// Création de contrat
    ContractCreation = 5000,
}

/// Métriques de gas pour une opération
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasMetrics {
    /// Gas consommé par opération
    pub operations: HashMap<String, u64>,
    /// Gas total consommé
    pub total_consumed: u64,
    /// Gas restant
    pub remaining: u64,
    /// Nombre d'opérations par type
    pub operation_counts: HashMap<String, u32>,
}

/// Gestionnaire de gas pour l'exécution des contrats
#[derive(Debug, Clone)]
pub struct GasManager {
    /// Gas initial disponible
    initial_gas: u64,
    /// Gas restant
    remaining_gas: u64,
    /// Prix du gas
    gas_price: u64,
    /// Historique des consommations
    consumption_log: Vec<GasConsumption>,
    /// Métriques par type d'opération
    metrics: HashMap<String, u64>,
}

/// Enregistrement d'une consommation de gas
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasConsumption {
    /// Type d'opération
    pub operation: String,
    /// Coût en gas
    pub cost: u64,
    /// Gas restant après l'opération
    pub remaining_after: u64,
    /// Timestamp de l'opération
    pub timestamp: std::time::Instant,
}

/// Limite de gas configurable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasLimit {
    /// Limite pour les opérations de base
    pub basic_operations: u64,
    /// Limite pour les accès storage
    pub storage_operations: u64,
    /// Limite pour les appels externes
    pub external_calls: u64,
    /// Limite totale par transaction
    pub total_limit: u64,
}

impl Default for GasLimit {
    fn default() -> Self {
        Self {
            basic_operations: 100_000,
            storage_operations: 500_000,
            external_calls: 300_000,
            total_limit: DEFAULT_GAS_LIMIT,
        }
    }
}

impl GasManager {
    /// Crée un nouveau gestionnaire de gas
    pub fn new(gas_limit: u64) -> Self {
        Self::with_price(gas_limit, MIN_GAS_PRICE)
    }

    /// Crée un nouveau gestionnaire avec un prix de gas spécifique
    pub fn with_price(gas_limit: u64, gas_price: u64) -> Self {
        Self {
            initial_gas: gas_limit,
            remaining_gas: gas_limit,
            gas_price,
            consumption_log: Vec::new(),
            metrics: HashMap::new(),
        }
    }

    /// Consomme du gas pour une opération
    pub fn consume(&mut self, cost: u64) -> ContractResult<()> {
        self.consume_with_name(cost, "unknown")
    }

    /// Consomme du gas avec le nom de l'opération
    pub fn consume_with_name(&mut self, cost: u64, operation: &str) -> ContractResult<()> {
        if self.remaining_gas < cost {
            return Err(ContractError::InsufficientGas {
                required: cost,
                available: self.remaining_gas,
            });
        }

        self.remaining_gas -= cost;

        // Enregistre la consommation
        self.consumption_log.push(GasConsumption {
            operation: operation.to_string(),
            cost,
            remaining_after: self.remaining_gas,
            timestamp: std::time::Instant::now(),
        });

        // Met à jour les métriques
        *self.metrics.entry(operation.to_string()).or_insert(0) += cost;

        Ok(())
    }

    /// Consomme du gas pour un type d'opération spécifique
    pub fn consume_operation(&mut self, operation: GasCost) -> ContractResult<()> {
        let cost = operation as u64;
        let name = match operation {
            GasCost::Basic => "basic",
            GasCost::Memory => "memory",
            GasCost::StorageRead => "storage_read",
            GasCost::StorageWrite => "storage_write",
            GasCost::StorageDelete => "storage_delete",
            GasCost::Log => "log",
            GasCost::Event => "event",
            GasCost::FunctionCall => "function_call",
            GasCost::Transfer => "transfer",
            GasCost::Hash => "hash",
            GasCost::SignatureVerify => "signature_verify",
            GasCost::ExternalCall => "external_call",
            GasCost::ContractCreation => "contract_creation",
        };

        self.consume_with_name(cost, name)
    }

    /// Obtient le gas restant
    pub fn remaining(&self) -> u64 {
        self.remaining_gas
    }

    /// Obtient le gas consommé
    pub fn consumed(&self) -> u64 {
        self.initial_gas - self.remaining_gas
    }

    /// Obtient le pourcentage de gas utilisé
    pub fn usage_percentage(&self) -> f64 {
        if self.initial_gas == 0 {
            0.0
        } else {
            (self.consumed() as f64 / self.initial_gas as f64) * 100.0
        }
    }

    /// Vérifie si on peut consommer une quantité de gas
    pub fn can_consume(&self, cost: u64) -> bool {
        self.remaining_gas >= cost
    }

    /// Calcule le coût en tokens ARC du gas consommé
    pub fn calculate_fee(&self) -> u64 {
        self.consumed() * self.gas_price
    }

    /// Obtient les métriques détaillées
    pub fn get_metrics(&self) -> GasMetrics {
        let mut operation_counts = HashMap::new();
        
        for consumption in &self.consumption_log {
            *operation_counts.entry(consumption.operation.clone()).or_insert(0) += 1;
        }

        GasMetrics {
            operations: self.metrics.clone(),
            total_consumed: self.consumed(),
            remaining: self.remaining_gas,
            operation_counts,
        }
    }

    /// Obtient l'historique des consommations
    pub fn get_consumption_log(&self) -> &Vec<GasConsumption> {
        &self.consumption_log
    }

    /// Réinitialise le gestionnaire avec une nouvelle limite
    pub fn reset(&mut self, new_limit: u64) {
        self.initial_gas = new_limit;
        self.remaining_gas = new_limit;
        self.consumption_log.clear();
        self.metrics.clear();
    }

    /// Estime le gas nécessaire pour une séquence d'opérations
    pub fn estimate_cost(operations: &[GasCost]) -> u64 {
        operations.iter().map(|op| *op as u64).sum()
    }

    /// Vérifie si le gas restant est suffisant pour une séquence d'opérations
    pub fn can_execute_sequence(&self, operations: &[GasCost]) -> bool {
        let total_cost = Self::estimate_cost(operations);
        self.can_consume(total_cost)
    }

    /// Consomme le gas pour une séquence d'opérations
    pub fn consume_sequence(&mut self, operations: &[GasCost]) -> ContractResult<()> {
        let total_cost = Self::estimate_cost(operations);
        
        if !self.can_consume(total_cost) {
            return Err(ContractError::InsufficientGas {
                required: total_cost,
                available: self.remaining_gas,
            });
        }

        for operation in operations {
            self.consume_operation(*operation)?;
        }

        Ok(())
    }

    /// Crée un point de sauvegarde du gas
    pub fn checkpoint(&self) -> GasCheckpoint {
        GasCheckpoint {
            remaining_gas: self.remaining_gas,
            consumption_count: self.consumption_log.len(),
        }
    }

    /// Restaure le gas à un point de sauvegarde
    pub fn restore_checkpoint(&mut self, checkpoint: GasCheckpoint) {
        self.remaining_gas = checkpoint.remaining_gas;
        self.consumption_log.truncate(checkpoint.consumption_count);
        
        // Recalcule les métriques
        self.metrics.clear();
        for consumption in &self.consumption_log {
            *self.metrics.entry(consumption.operation.clone()).or_insert(0) += consumption.cost;
        }
    }
}

/// Point de sauvegarde pour le gas
#[derive(Debug, Clone)]
pub struct GasCheckpoint {
    remaining_gas: u64,
    consumption_count: usize,
}

/// Calculateur de gas statique pour différents types d'opérations
pub struct GasCalculator;

impl GasCalculator {
    /// Calcule le gas pour un accès mémoire
    pub fn memory_access(bytes: usize) -> u64 {
        (GasCost::Memory as u64) * ((bytes / 32) + 1) as u64
    }

    /// Calcule le gas pour une écriture dans le storage
    pub fn storage_write(key_size: usize, value_size: usize) -> u64 {
        let base_cost = GasCost::StorageWrite as u64;
        let size_cost = ((key_size + value_size) / 32 + 1) as u64;
        base_cost + size_cost
    }

    /// Calcule le gas pour un log
    pub fn log_cost(data_size: usize) -> u64 {
        let base_cost = GasCost::Log as u64;
        let data_cost = (data_size / 32 + 1) as u64;
        base_cost + data_cost
    }

    /// Calcule le gas pour un event
    pub fn event_cost(data_size: usize, topics_count: usize) -> u64 {
        let base_cost = GasCost::Event as u64;
        let data_cost = (data_size / 32 + 1) as u64;
        let topics_cost = topics_count as u64 * 10;
        base_cost + data_cost + topics_cost
    }

    /// Calcule le gas pour un appel de contrat
    pub fn contract_call_cost(args_size: usize) -> u64 {
        let base_cost = GasCost::ExternalCall as u64;
        let args_cost = (args_size / 32 + 1) as u64;
        base_cost + args_cost
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_manager_creation() {
        let manager = GasManager::new(1000);
        assert_eq!(manager.remaining(), 1000);
        assert_eq!(manager.consumed(), 0);
    }

    #[test]
    fn test_gas_consumption() {
        let mut manager = GasManager::new(1000);
        
        assert!(manager.consume(100).is_ok());
        assert_eq!(manager.remaining(), 900);
        assert_eq!(manager.consumed(), 100);
    }

    #[test]
    fn test_insufficient_gas() {
        let mut manager = GasManager::new(100);
        
        let result = manager.consume(200);
        assert!(result.is_err());
        
        if let Err(ContractError::InsufficientGas { required, available }) = result {
            assert_eq!(required, 200);
            assert_eq!(available, 100);
        } else {
            panic!("Expected InsufficientGas error");
        }
    }

    #[test]
    fn test_operation_consumption() {
        let mut manager = GasManager::new(1000);
        
        assert!(manager.consume_operation(GasCost::StorageRead).is_ok());
        assert_eq!(manager.consumed(), 50);
        
        assert!(manager.consume_operation(GasCost::StorageWrite).is_ok());
        assert_eq!(manager.consumed(), 250);
    }

    #[test]
    fn test_gas_metrics() {
        let mut manager = GasManager::new(1000);
        
        manager.consume_operation(GasCost::StorageRead).unwrap();
        manager.consume_operation(GasCost::StorageWrite).unwrap();
        manager.consume_operation(GasCost::StorageRead).unwrap();
        
        let metrics = manager.get_metrics();
        assert_eq!(metrics.total_consumed, 300); // 50 + 200 + 50
        assert_eq!(metrics.operation_counts.get("storage_read"), Some(&2));
        assert_eq!(metrics.operation_counts.get("storage_write"), Some(&1));
    }

    #[test]
    fn test_sequence_execution() {
        let mut manager = GasManager::new(1000);
        
        let operations = vec![
            GasCost::StorageRead,
            GasCost::Basic,
            GasCost::StorageWrite,
        ];
        
        assert!(manager.can_execute_sequence(&operations));
        assert!(manager.consume_sequence(&operations).is_ok());
        assert_eq!(manager.consumed(), 251); // 50 + 1 + 200
    }

    #[test]
    fn test_checkpoint_restore() {
        let mut manager = GasManager::new(1000);
        
        manager.consume(100).unwrap();
        let checkpoint = manager.checkpoint();
        
        manager.consume(200).unwrap();
        assert_eq!(manager.consumed(), 300);
        
        manager.restore_checkpoint(checkpoint);
        assert_eq!(manager.consumed(), 100);
        assert_eq!(manager.remaining(), 900);
    }

    #[test]
    fn test_gas_calculator() {
        let memory_cost = GasCalculator::memory_access(64);
        assert_eq!(memory_cost, 4); // 2 * (64/32 + 1) = 2 * 2 = 4
        
        let storage_cost = GasCalculator::storage_write(32, 64);
        assert_eq!(storage_cost, 203); // 200 + (32+64)/32 + 1 = 200 + 3 = 203
        
        let log_cost = GasCalculator::log_cost(128);
        assert_eq!(log_cost, 30); // 25 + 128/32 + 1 = 25 + 5 = 30
    }

    #[test]
    fn test_usage_percentage() {
        let mut manager = GasManager::new(1000);
        
        manager.consume(250).unwrap();
        assert_eq!(manager.usage_percentage(), 25.0);
        
        manager.consume(250).unwrap();
        assert_eq!(manager.usage_percentage(), 50.0);
    }

    #[test]
    fn test_fee_calculation() {
        let mut manager = GasManager::with_price(1000, 5);
        
        manager.consume(200).unwrap();
        assert_eq!(manager.calculate_fee(), 1000); // 200 * 5 = 1000
    }
}