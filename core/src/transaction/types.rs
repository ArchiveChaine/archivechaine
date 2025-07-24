//! Types de transactions pour ArchiveChain

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, HashAlgorithm, Signature, PublicKey, compute_hash};
use crate::error::{TransactionError, Result};

/// Types de transactions supportées
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// Transaction de transfert de tokens
    Transfer,
    /// Transaction d'archivage (paiement pour archiver)
    Archive,
    /// Transaction de stake/délégation
    Stake,
    /// Transaction de gouvernance
    Governance,
}

/// Entrée d'une transaction (UTXO)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionInput {
    /// Hash de la transaction précédente
    pub previous_tx: Hash,
    /// Index de l'output dans la transaction précédente
    pub output_index: u32,
    /// Script de déverrouillage (simplifié pour le core)
    pub unlock_script: Vec<u8>,
    /// Signature de l'input
    pub signature: Signature,
}

/// Sortie d'une transaction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransactionOutput {
    /// Montant en tokens (en unité atomique)
    pub amount: u64,
    /// Clé publique du destinataire
    pub recipient: PublicKey,
    /// Script de verrouillage (simplifié)
    pub lock_script: Vec<u8>,
}

/// Transaction complète
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    /// Identifiant unique de la transaction
    pub tx_id: Hash,
    /// Type de transaction
    pub tx_type: TransactionType,
    /// Entrées de la transaction
    pub inputs: Vec<TransactionInput>,
    /// Sorties de la transaction
    pub outputs: Vec<TransactionOutput>,
    /// Frais de transaction
    pub fee: u64,
    /// Nonce pour éviter les replays
    pub nonce: u64,
    /// Timestamp de création
    pub timestamp: DateTime<Utc>,
    /// Données additionnelles (pour les contrats, etc.)
    pub data: Vec<u8>,
    /// Signature de la transaction complète
    pub signature: Signature,
}

impl Transaction {
    /// Crée une nouvelle transaction
    pub fn new(
        from: Hash,
        to: Hash,
        amount: u64,
        data: Vec<u8>,
    ) -> Self {
        let timestamp = Utc::now();
        let tx_id = Self::calculate_tx_id(&from, &to, amount, timestamp);
        
        Self {
            tx_id,
            tx_type: TransactionType::Transfer,
            inputs: Vec::new(),
            outputs: Vec::new(),
            fee: 0,
            nonce: 0,
            timestamp,
            data,
            signature: Signature::zero(),
        }
    }

    /// Calcule l'ID de la transaction
    fn calculate_tx_id(from: &Hash, to: &Hash, amount: u64, timestamp: DateTime<Utc>) -> Hash {
        let mut data = Vec::new();
        data.extend_from_slice(from.as_bytes());
        data.extend_from_slice(to.as_bytes());
        data.extend_from_slice(&amount.to_le_bytes());
        data.extend_from_slice(&timestamp.timestamp().to_le_bytes());
        
        compute_hash(&data, HashAlgorithm::Blake3)
    }

    /// Calcule le hash de la transaction
    pub fn hash(&self) -> &Hash {
        &self.tx_id
    }

    /// Recalcule le hash de la transaction
    pub fn calculate_hash(&self, algorithm: HashAlgorithm) -> Hash {
        let data = self.serialize_for_hash();
        compute_hash(&data, algorithm)
    }

    /// Sérialise la transaction pour le calcul de hash
    fn serialize_for_hash(&self) -> Vec<u8> {
        // Sérialise tous les champs sauf la signature finale
        let mut data = Vec::new();
        
        // Type de transaction
        data.push(match self.tx_type {
            TransactionType::Transfer => 0,
            TransactionType::Archive => 1,
            TransactionType::Stake => 2,
            TransactionType::Governance => 3,
        });
        
        // Inputs
        data.extend_from_slice(&(self.inputs.len() as u32).to_le_bytes());
        for input in &self.inputs {
            data.extend_from_slice(input.previous_tx.as_bytes());
            data.extend_from_slice(&input.output_index.to_le_bytes());
            data.extend_from_slice(&input.unlock_script);
        }
        
        // Outputs
        data.extend_from_slice(&(self.outputs.len() as u32).to_le_bytes());
        for output in &self.outputs {
            data.extend_from_slice(&output.amount.to_le_bytes());
            data.extend_from_slice(output.recipient.as_bytes());
            data.extend_from_slice(&output.lock_script);
        }
        
        // Autres champs
        data.extend_from_slice(&self.fee.to_le_bytes());
        data.extend_from_slice(&self.nonce.to_le_bytes());
        data.extend_from_slice(&self.timestamp.timestamp().to_le_bytes());
        data.extend_from_slice(&self.data);
        
        data
    }

    /// Vérifie si la transaction est valide
    pub fn is_valid(&self) -> Result<bool> {
        // Vérifications de base
        if self.inputs.is_empty() && self.tx_type != TransactionType::Archive {
            return Ok(false);
        }

        if self.outputs.is_empty() {
            return Ok(false);
        }

        // Vérifie que le montant total des outputs + fee <= inputs
        let total_output: u64 = self.outputs.iter().map(|o| o.amount).sum();
        if self.tx_type == TransactionType::Transfer {
            // Pour les transferts simples, vérification UTXO basique
            // (implémentation complète nécessiterait l'état de la chaîne)
            if total_output == 0 {
                return Ok(false);
            }
        }

        // Vérifie le timestamp (ne doit pas être dans le futur)
        if self.timestamp > Utc::now() {
            return Ok(false);
        }

        // Vérifie que les montants sont cohérents
        if total_output > u64::MAX - self.fee {
            return Ok(false);
        }

        Ok(true)
    }

    /// Ajoute une entrée à la transaction
    pub fn add_input(&mut self, input: TransactionInput) {
        self.inputs.push(input);
        // Recalcule le TX ID
        self.tx_id = self.calculate_hash(HashAlgorithm::Blake3);
    }

    /// Ajoute une sortie à la transaction
    pub fn add_output(&mut self, output: TransactionOutput) {
        self.outputs.push(output);
        // Recalcule le TX ID
        self.tx_id = self.calculate_hash(HashAlgorithm::Blake3);
    }

    /// Définit les frais de transaction
    pub fn set_fee(&mut self, fee: u64) {
        self.fee = fee;
        self.tx_id = self.calculate_hash(HashAlgorithm::Blake3);
    }

    /// Obtient le montant total des entrées
    pub fn total_input_amount(&self) -> u64 {
        // Dans une implémentation complète, ceci nécessiterait l'accès à l'état
        // Pour l'instant, retourne 0
        0
    }

    /// Obtient le montant total des sorties
    pub fn total_output_amount(&self) -> u64 {
        self.outputs.iter().map(|o| o.amount).sum()
    }

    /// Vérifie si c'est une transaction coinbase (génération de nouveaux tokens)
    pub fn is_coinbase(&self) -> bool {
        self.inputs.is_empty() && self.tx_type == TransactionType::Archive
    }

    /// Obtient la taille de la transaction en bytes
    pub fn size_bytes(&self) -> usize {
        bincode::serialized_size(self).unwrap_or(0) as usize
    }

    /// Calcule les frais par byte
    pub fn fee_per_byte(&self) -> f64 {
        let size = self.size_bytes();
        if size == 0 {
            0.0
        } else {
            self.fee as f64 / size as f64
        }
    }
}

/// Builder pour créer des transactions de manière fluide
#[derive(Debug)]
pub struct TransactionBuilder {
    tx_type: TransactionType,
    inputs: Vec<TransactionInput>,
    outputs: Vec<TransactionOutput>,
    fee: u64,
    nonce: u64,
    data: Vec<u8>,
}

impl TransactionBuilder {
    /// Crée un nouveau builder
    pub fn new(tx_type: TransactionType) -> Self {
        Self {
            tx_type,
            inputs: Vec::new(),
            outputs: Vec::new(),
            fee: 0,
            nonce: 0,
            data: Vec::new(),
        }
    }

    /// Ajoute une entrée
    pub fn add_input(mut self, input: TransactionInput) -> Self {
        self.inputs.push(input);
        self
    }

    /// Ajoute une sortie
    pub fn add_output(mut self, output: TransactionOutput) -> Self {
        self.outputs.push(output);
        self
    }

    /// Définit les frais
    pub fn fee(mut self, fee: u64) -> Self {
        self.fee = fee;
        self
    }

    /// Définit le nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = nonce;
        self
    }

    /// Ajoute des données
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Construit la transaction
    pub fn build(self) -> Transaction {
        let timestamp = Utc::now();
        let tx_id = Hash::zero(); // Sera recalculé
        
        let mut tx = Transaction {
            tx_id,
            tx_type: self.tx_type,
            inputs: self.inputs,
            outputs: self.outputs,
            fee: self.fee,
            nonce: self.nonce,
            timestamp,
            data: self.data,
            signature: Signature::zero(),
        };
        
        // Calcule le vrai TX ID
        tx.tx_id = tx.calculate_hash(HashAlgorithm::Blake3);
        tx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{Hash, generate_keypair};

    #[test]
    fn test_transaction_creation() {
        let tx = Transaction::new(
            Hash::zero(),
            Hash::zero(),
            1000,
            Vec::new(),
        );
        
        assert!(!tx.tx_id.is_zero());
        assert_eq!(tx.tx_type, TransactionType::Transfer);
    }

    #[test]
    fn test_transaction_builder() {
        let keypair = generate_keypair().unwrap();
        
        let output = TransactionOutput {
            amount: 1000,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let tx = TransactionBuilder::new(TransactionType::Transfer)
            .add_output(output)
            .fee(10)
            .nonce(1)
            .build();
        
        assert_eq!(tx.outputs.len(), 1);
        assert_eq!(tx.fee, 10);
        assert_eq!(tx.nonce, 1);
    }

    #[test]
    fn test_transaction_validation() {
        let keypair = generate_keypair().unwrap();
        
        let output = TransactionOutput {
            amount: 1000,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let tx = TransactionBuilder::new(TransactionType::Archive)
            .add_output(output)
            .build();
        
        assert!(tx.is_valid().unwrap());
    }

    #[test]
    fn test_coinbase_transaction() {
        let keypair = generate_keypair().unwrap();
        
        let output = TransactionOutput {
            amount: 1000,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let tx = TransactionBuilder::new(TransactionType::Archive)
            .add_output(output)
            .build();
        
        assert!(tx.is_coinbase());
    }

    #[test]
    fn test_transaction_amounts() {
        let keypair = generate_keypair().unwrap();
        
        let output1 = TransactionOutput {
            amount: 500,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let output2 = TransactionOutput {
            amount: 300,
            recipient: keypair.public_key().clone(),
            lock_script: Vec::new(),
        };
        
        let tx = TransactionBuilder::new(TransactionType::Transfer)
            .add_output(output1)
            .add_output(output2)
            .fee(10)
            .build();
        
        assert_eq!(tx.total_output_amount(), 800);
        assert_eq!(tx.fee, 10);
    }
}