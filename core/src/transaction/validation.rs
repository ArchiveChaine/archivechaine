//! Validation des transactions pour ArchiveChain

use crate::error::{TransactionError, Result};
use super::types::Transaction;

/// Validateur de transactions
#[derive(Debug)]
pub struct TransactionValidator {
    /// Configuration de validation
    pub config: ValidationConfig,
}

/// Configuration pour la validation des transactions
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Frais minimum par transaction
    pub min_fee: u64,
    /// Taille maximum d'une transaction en bytes
    pub max_size: usize,
    /// Montant maximum par transaction
    pub max_amount: u64,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            min_fee: 1,
            max_size: 1024 * 1024, // 1MB
            max_amount: u64::MAX / 2,
        }
    }
}

impl TransactionValidator {
    /// Crée un nouveau validateur
    pub fn new(config: ValidationConfig) -> Self {
        Self { config }
    }

    /// Valide une transaction
    pub fn validate(&self, transaction: &Transaction) -> Result<bool> {
        // Validation de base
        if !transaction.is_valid()? {
            return Ok(false);
        }

        // Vérifie les frais minimum
        if transaction.fee < self.config.min_fee {
            return Ok(false);
        }

        // Vérifie la taille
        if transaction.size_bytes() > self.config.max_size {
            return Ok(false);
        }

        // Vérifie les montants
        if transaction.total_output_amount() > self.config.max_amount {
            return Ok(false);
        }

        Ok(true)
    }
}

impl Default for TransactionValidator {
    fn default() -> Self {
        Self::new(ValidationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transaction::types::{TransactionBuilder, TransactionType, TransactionOutput};
    use crate::crypto::generate_keypair;

    #[test]
    fn test_transaction_validation() {
        let validator = TransactionValidator::default();
        let keypair = generate_keypair().unwrap();
        
        let output = TransactionOutput {
            amount: 1000,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let tx = TransactionBuilder::new(TransactionType::Archive)
            .add_output(output)
            .fee(10)
            .build();
        
        assert!(validator.validate(&tx).unwrap());
    }
}

/// Trait pour les types qui peuvent être validés
pub trait Validatable {
    /// Valide l'objet et retourne true si valide
    fn is_valid(&self) -> Result<bool>;
    
    /// Valide l'objet avec une configuration spécifique
    fn validate_with_config(&self, config: &ValidationConfig) -> Result<bool>;
}