//! Structure principale de la blockchain ArchiveChain

use std::collections::HashMap;
use crate::crypto::{Hash, HashAlgorithm};
use crate::block::{Block, BlockBuilder};
use crate::transaction::{Transaction, TransactionPool};
use crate::state::{StateMachine, StateStorage, MemoryStateStorage};
use crate::error::{CoreError, Result};

/// Configuration de la blockchain
#[derive(Debug, Clone)]
pub struct BlockchainConfig {
    /// Algorithme de hachage principal
    pub hash_algorithm: HashAlgorithm,
    /// Difficulté initiale
    pub initial_difficulty: u64,
    /// Taille maximum des blocs
    pub max_block_size: usize,
    /// Nombre maximum de transactions par bloc
    pub max_transactions_per_block: usize,
    /// Temps cible entre les blocs (en secondes)
    pub target_block_time: u64,
}

impl Default for BlockchainConfig {
    fn default() -> Self {
        Self {
            hash_algorithm: HashAlgorithm::Blake3,
            initial_difficulty: 1000,
            max_block_size: 1024 * 1024 * 4, // 4MB
            max_transactions_per_block: 1000,
            target_block_time: 60, // 1 minute
        }
    }
}

/// Structure principale de la blockchain ArchiveChain
#[derive(Debug)]
pub struct Blockchain {
    /// Configuration de la blockchain
    config: BlockchainConfig,
    
    /// Chaîne de blocs indexée par hash
    blocks: HashMap<Hash, Block>,
    
    /// Index des blocs par hauteur
    blocks_by_height: HashMap<u64, Hash>,
    
    /// Hash du bloc genesis
    genesis_hash: Hash,
    
    /// Hash du dernier bloc (tête de chaîne)
    head_hash: Hash,
    
    /// Hauteur actuelle de la chaîne
    current_height: u64,
    
    /// Pool de transactions en attente
    transaction_pool: TransactionPool,
    
    /// Machine d'état actuelle
    state: StateMachine,
    
    /// Stockage d'état
    state_storage: Box<dyn StateStorage>,
    
    /// Difficulté actuelle
    current_difficulty: u64,
}

impl Blockchain {
    /// Crée une nouvelle blockchain avec le bloc genesis
    pub fn new(config: BlockchainConfig) -> Result<Self> {
        let mut blockchain = Self {
            config: config.clone(),
            blocks: HashMap::new(),
            blocks_by_height: HashMap::new(),
            genesis_hash: Hash::zero(),
            head_hash: Hash::zero(),
            current_height: 0,
            transaction_pool: TransactionPool::default(),
            state: StateMachine::new(),
            state_storage: Box::new(MemoryStateStorage::new()),
            current_difficulty: config.initial_difficulty,
        };

        // Crée et ajoute le bloc genesis
        let genesis_block = blockchain.create_genesis_block()?;
        blockchain.add_block(genesis_block)?;

        Ok(blockchain)
    }

    /// Crée le bloc genesis
    fn create_genesis_block(&self) -> Result<Block> {
        let genesis_block = BlockBuilder::new(0, Hash::zero(), self.config.hash_algorithm)
            .difficulty(self.current_difficulty)
            .nonce(0)
            .build()?;

        Ok(genesis_block)
    }

    /// Ajoute un nouveau bloc à la chaîne
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // Valide le bloc
        if !self.validate_block(&block)? {
            return Err(CoreError::Validation {
                message: "Bloc invalide".to_string(),
            });
        }

        // Vérifie l'ordre séquentiel
        if block.height() != self.current_height {
            return Err(CoreError::Validation {
                message: format!(
                    "Hauteur de bloc incorrecte: attendue {}, reçue {}",
                    self.current_height, block.height()
                ),
            });
        }

        let block_hash = block.hash().clone();

        // Ajoute le bloc aux index
        self.blocks.insert(block_hash.clone(), block);
        self.blocks_by_height.insert(self.current_height, block_hash.clone());

        // Met à jour la tête de chaîne
        if self.current_height == 0 {
            self.genesis_hash = block_hash.clone();
        }
        self.head_hash = block_hash;
        self.current_height += 1;

        // Retire les transactions du pool
        if let Some(block) = self.blocks.get(&self.head_hash) {
            for transaction in block.transactions() {
                self.transaction_pool.remove_transaction(transaction.hash());
            }
        }

        Ok(())
    }

    /// Valide un bloc
    pub fn validate_block(&self, block: &Block) -> Result<bool> {
        // Validation de base du bloc
        if !block.is_valid(self.config.hash_algorithm)? {
            return Ok(false);
        }

        // Vérifie que le bloc précédent existe (sauf pour genesis)
        if block.height() > 0 {
            if !block.previous_hash().is_zero() && !self.blocks.contains_key(block.previous_hash()) {
                return Ok(false);
            }

            // Vérifie que le hash précédent correspond à notre tête
            if block.previous_hash() != &self.head_hash {
                return Ok(false);
            }
        }

        // Vérifie la taille du bloc
        if block.size_bytes() > self.config.max_block_size {
            return Ok(false);
        }

        // Vérifie le nombre de transactions
        if block.transaction_count() > self.config.max_transactions_per_block {
            return Ok(false);
        }

        Ok(true)
    }

    /// Obtient un bloc par son hash
    pub fn get_block(&self, hash: &Hash) -> Option<&Block> {
        self.blocks.get(hash)
    }

    /// Obtient un bloc par sa hauteur
    pub fn get_block_by_height(&self, height: u64) -> Option<&Block> {
        self.blocks_by_height
            .get(&height)
            .and_then(|hash| self.blocks.get(hash))
    }

    /// Obtient le dernier bloc
    pub fn get_head_block(&self) -> Option<&Block> {
        if self.head_hash.is_zero() {
            None
        } else {
            self.blocks.get(&self.head_hash)
        }
    }

    /// Obtient le bloc genesis
    pub fn get_genesis_block(&self) -> Option<&Block> {
        if self.genesis_hash.is_zero() {
            None
        } else {
            self.blocks.get(&self.genesis_hash)
        }
    }

    /// Ajoute une transaction au pool
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        self.transaction_pool.add_transaction(transaction)
    }

    /// Obtient les transactions en attente
    pub fn pending_transactions(&self) -> Vec<&Transaction> {
        self.transaction_pool.pending_transactions()
    }

    /// Mine un nouveau bloc avec les transactions en attente
    pub fn mine_block(&mut self) -> Result<Block> {
        let pending_txs: Vec<Transaction> = self.transaction_pool
            .pending_transactions()
            .into_iter()
            .cloned()
            .collect();

        let new_block = BlockBuilder::new(
            self.current_height,
            self.head_hash.clone(),
            self.config.hash_algorithm,
        )
        .add_transactions(pending_txs)
        .difficulty(self.current_difficulty)
        .build()?;

        Ok(new_block)
    }

    /// Obtient la hauteur actuelle de la chaîne
    pub fn height(&self) -> u64 {
        self.current_height
    }

    /// Obtient le hash de la tête de chaîne
    pub fn head_hash(&self) -> &Hash {
        &self.head_hash
    }

    /// Obtient la difficulté actuelle
    pub fn difficulty(&self) -> u64 {
        self.current_difficulty
    }

    /// Calcule la difficulté pour le prochain bloc
    pub fn calculate_next_difficulty(&self) -> u64 {
        // Implémentation simple - peut être étendue avec un algorithme d'ajustement
        if self.current_height < 10 {
            return self.current_difficulty;
        }

        // Obtient les 10 derniers blocs pour calculer le temps moyen
        let mut total_time = 0u64;
        let mut valid_intervals = 0;

        for i in 1..=10 {
            if let (Some(current), Some(previous)) = (
                self.get_block_by_height(self.current_height.saturating_sub(i)),
                self.get_block_by_height(self.current_height.saturating_sub(i + 1)),
            ) {
                let time_diff = (current.timestamp() - previous.timestamp()).num_seconds();
                if time_diff > 0 {
                    total_time += time_diff as u64;
                    valid_intervals += 1;
                }
            }
        }

        if valid_intervals == 0 {
            return self.current_difficulty;
        }

        let average_time = total_time / valid_intervals;
        let target_time = self.config.target_block_time;

        // Ajuste la difficulté basé sur le temps moyen vs cible
        if average_time > target_time * 2 {
            // Trop lent, réduit la difficulté
            self.current_difficulty * 9 / 10
        } else if average_time < target_time / 2 {
            // Trop rapide, augmente la difficulté
            self.current_difficulty * 11 / 10
        } else {
            // Dans la plage acceptable
            self.current_difficulty
        }
    }

    /// Met à jour la difficulté
    pub fn update_difficulty(&mut self) {
        self.current_difficulty = self.calculate_next_difficulty();
    }

    /// Obtient des statistiques sur la blockchain
    pub fn stats(&self) -> BlockchainStats {
        BlockchainStats {
            height: self.current_height,
            total_blocks: self.blocks.len(),
            pending_transactions: self.transaction_pool.size(),
            difficulty: self.current_difficulty,
            head_hash: self.head_hash.clone(),
        }
    }

    /// Vérifie l'intégrité de toute la chaîne
    pub fn verify_chain(&self) -> Result<bool> {
        for height in 0..self.current_height {
            if let Some(block) = self.get_block_by_height(height) {
                if !block.is_valid(self.config.hash_algorithm)? {
                    return Ok(false);
                }

                // Vérifie le chaînage
                if height > 0 {
                    if let Some(prev_block) = self.get_block_by_height(height - 1) {
                        if block.previous_hash() != prev_block.hash() {
                            return Ok(false);
                        }
                    } else {
                        return Ok(false);
                    }
                }
            } else {
                return Ok(false);
            }
        }

        Ok(true)
    }
}

/// Statistiques de la blockchain
#[derive(Debug, Clone)]
pub struct BlockchainStats {
    /// Hauteur actuelle
    pub height: u64,
    /// Nombre total de blocs
    pub total_blocks: usize,
    /// Transactions en attente
    pub pending_transactions: usize,
    /// Difficulté actuelle
    pub difficulty: u64,
    /// Hash de la tête
    pub head_hash: Hash,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_blockchain_creation() {
        let config = BlockchainConfig::default();
        let blockchain = Blockchain::new(config).unwrap();
        
        assert_eq!(blockchain.height(), 1); // Genesis block
        assert!(blockchain.get_genesis_block().is_some());
        assert_eq!(blockchain.blocks.len(), 1);
    }

    #[test]
    fn test_blockchain_add_block() {
        let config = BlockchainConfig::default();
        let mut blockchain = Blockchain::new(config).unwrap();
        
        let new_block = blockchain.mine_block().unwrap();
        blockchain.add_block(new_block).unwrap();
        
        assert_eq!(blockchain.height(), 2);
    }

    #[test]
    fn test_blockchain_verification() {
        let config = BlockchainConfig::default();
        let blockchain = Blockchain::new(config).unwrap();
        
        assert!(blockchain.verify_chain().unwrap());
    }

    #[test]
    fn test_difficulty_calculation() {
        let config = BlockchainConfig::default();
        let blockchain = Blockchain::new(config).unwrap();
        
        let next_difficulty = blockchain.calculate_next_difficulty();
        assert_eq!(next_difficulty, blockchain.difficulty()); // Should be same for short chain
    }
}