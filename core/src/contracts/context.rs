//! Contexte d'exécution pour les smart contracts

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, PublicKey};
use crate::contracts::{ContractError, ContractResult, ContractEvent, StateChange};
use crate::transaction::Transaction;
use crate::block::Block;

/// Fournisseur de contexte pour l'accès aux données de la blockchain
pub trait ContextProvider {
    /// Obtient les informations d'un bloc par son hash
    fn get_block(&self, block_hash: Hash) -> ContractResult<Option<Block>>;
    
    /// Obtient le bloc courant
    fn get_current_block(&self) -> ContractResult<Block>;
    
    /// Obtient une transaction par son hash
    fn get_transaction(&self, tx_hash: Hash) -> ContractResult<Option<Transaction>>;
    
    /// Obtient le solde d'une adresse
    fn get_balance(&self, address: &PublicKey) -> ContractResult<u64>;
    
    /// Lit une valeur du storage d'un contrat
    fn read_storage(&self, contract_address: Hash, key: &[u8]) -> ContractResult<Option<Vec<u8>>>;
    
    /// Écrit une valeur dans le storage d'un contrat
    fn write_storage(&mut self, contract_address: Hash, key: &[u8], value: &[u8]) -> ContractResult<()>;
    
    /// Vérifie si un contrat existe
    fn contract_exists(&self, address: Hash) -> ContractResult<bool>;
    
    /// Obtient le code d'un contrat
    fn get_contract_code(&self, address: Hash) -> ContractResult<Option<Vec<u8>>>;
}

/// Informations sur l'environnement d'exécution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionEnvironment {
    /// Hash du bloc courant
    pub block_hash: Hash,
    /// Numéro du bloc courant
    pub block_number: u64,
    /// Timestamp du bloc courant
    pub block_timestamp: DateTime<Utc>,
    /// Hash de la transaction courante
    pub transaction_hash: Hash,
    /// Adresse de l'expéditeur de la transaction
    pub transaction_sender: PublicKey,
    /// Adresse du contrat en cours d'exécution
    pub contract_address: Hash,
    /// Adresse qui a appelé le contrat
    pub caller_address: PublicKey,
    /// Valeur envoyée avec l'appel (en tokens ARC)
    pub value_sent: u64,
    /// Gas limite pour cette exécution
    pub gas_limit: u64,
    /// Prix du gas
    pub gas_price: u64,
}

/// Contexte d'exécution d'un smart contract
#[derive(Debug)]
pub struct ContractContext {
    /// Environnement d'exécution
    pub environment: ExecutionEnvironment,
    /// Fournisseur d'accès aux données blockchain
    provider: Box<dyn ContextProvider + Send + Sync>,
    /// Storage temporaire pour les modifications
    temp_storage: HashMap<(Hash, Vec<u8>), Vec<u8>>,
    /// Logs émis par le contrat
    logs: Vec<String>,
    /// Events émis par le contrat
    events: Vec<ContractEvent>,
    /// Changements d'état effectués
    state_changes: Vec<StateChange>,
    /// Transferts de tokens effectués
    token_transfers: Vec<TokenTransfer>,
}

/// Transfert de tokens effectué par un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenTransfer {
    pub from: PublicKey,
    pub to: PublicKey,
    pub amount: u64,
    pub timestamp: DateTime<Utc>,
}

impl ContractContext {
    /// Crée un nouveau contexte d'exécution
    pub fn new(
        environment: ExecutionEnvironment,
        provider: Box<dyn ContextProvider + Send + Sync>,
    ) -> Self {
        Self {
            environment,
            provider,
            temp_storage: HashMap::new(),
            logs: Vec::new(),
            events: Vec::new(),
            state_changes: Vec::new(),
            token_transfers: Vec::new(),
        }
    }

    /// Lit une valeur du storage du contrat courant
    pub fn storage_read(&self, key: &[u8]) -> ContractResult<Option<Vec<u8>>> {
        // Vérifie d'abord dans le storage temporaire
        let storage_key = (self.environment.contract_address, key.to_vec());
        if let Some(value) = self.temp_storage.get(&storage_key) {
            return Ok(Some(value.clone()));
        }

        // Sinon, lit depuis la blockchain
        self.provider.read_storage(self.environment.contract_address, key)
    }

    /// Écrit une valeur dans le storage du contrat courant
    pub fn storage_write(&mut self, key: &[u8], value: &[u8]) -> ContractResult<()> {
        // Lit l'ancienne valeur pour l'historique
        let old_value = self.storage_read(key)?;

        // Écrit dans le storage temporaire
        let storage_key = (self.environment.contract_address, key.to_vec());
        self.temp_storage.insert(storage_key, value.to_vec());

        // Enregistre le changement d'état
        self.state_changes.push(StateChange {
            key: key.to_vec(),
            old_value,
            new_value: Some(value.to_vec()),
        });

        Ok(())
    }

    /// Supprime une valeur du storage
    pub fn storage_delete(&mut self, key: &[u8]) -> ContractResult<()> {
        let old_value = self.storage_read(key)?;
        
        if old_value.is_some() {
            let storage_key = (self.environment.contract_address, key.to_vec());
            self.temp_storage.remove(&storage_key);

            self.state_changes.push(StateChange {
                key: key.to_vec(),
                old_value,
                new_value: None,
            });
        }

        Ok(())
    }

    /// Émet un log
    pub fn emit_log(&mut self, message: String) {
        self.logs.push(message);
    }

    /// Émet un event
    pub fn emit_event(&mut self, name: String, data: Vec<u8>, topics: Vec<Hash>) {
        self.events.push(ContractEvent {
            name,
            data,
            topics,
        });
    }

    /// Effectue un transfert de tokens
    pub fn transfer_tokens(&mut self, to: PublicKey, amount: u64) -> ContractResult<()> {
        // Vérifie que le contrat a suffisamment de fonds
        let contract_balance = self.get_contract_balance()?;
        if contract_balance < amount {
            return Err(ContractError::InsufficientFunds {
                required: amount,
                available: contract_balance,
            });
        }

        // Enregistre le transfert (sera exécuté à la fin de l'exécution)
        self.token_transfers.push(TokenTransfer {
            from: self.environment.caller_address.clone(),
            to,
            amount,
            timestamp: Utc::now(),
        });

        Ok(())
    }

    /// Obtient le solde du contrat courant
    pub fn get_contract_balance(&self) -> ContractResult<u64> {
        // Simule une adresse publique à partir du hash du contrat
        // Dans une vraie implémentation, il faudrait un mapping approprié
        let contract_pubkey = PublicKey::from_bytes(&self.environment.contract_address.as_bytes()[..32])
            .map_err(|_| ContractError::InvalidState {
                message: "Invalid contract address for balance lookup".to_string(),
            })?;
        
        self.provider.get_balance(&contract_pubkey)
    }

    /// Obtient le solde d'une adresse
    pub fn get_balance(&self, address: &PublicKey) -> ContractResult<u64> {
        self.provider.get_balance(address)
    }

    /// Obtient les informations du bloc courant
    pub fn get_current_block(&self) -> ContractResult<Block> {
        self.provider.get_current_block()
    }

    /// Obtient une transaction par son hash
    pub fn get_transaction(&self, tx_hash: Hash) -> ContractResult<Option<Transaction>> {
        self.provider.get_transaction(tx_hash)
    }

    /// Appelle un autre contrat
    pub fn call_contract(
        &mut self,
        contract_address: Hash,
        function_name: &str,
        args: &[u8],
        gas_limit: u64,
    ) -> ContractResult<Vec<u8>> {
        // Vérifie que le contrat existe
        if !self.provider.contract_exists(contract_address)? {
            return Err(ContractError::ContractNotFound { address: contract_address });
        }

        // TODO: Implémenter l'appel récursif de contrat
        // Pour l'instant, retourne un résultat vide
        Ok(Vec::new())
    }

    /// Vérifie une signature
    pub fn verify_signature(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &PublicKey,
    ) -> ContractResult<bool> {
        // TODO: Implémenter la vérification de signature
        // en utilisant les fonctions crypto existantes
        Ok(true)
    }

    /// Calcule un hash
    pub fn compute_hash(&self, data: &[u8]) -> ContractResult<Hash> {
        use crate::crypto::{compute_blake3};
        Ok(compute_blake3(data))
    }

    /// Obtient le timestamp courant
    pub fn get_timestamp(&self) -> u64 {
        self.environment.block_timestamp.timestamp() as u64
    }

    /// Obtient l'adresse de l'appelant
    pub fn get_caller(&self) -> &PublicKey {
        &self.environment.caller_address
    }

    /// Obtient l'adresse du contrat
    pub fn get_contract_address(&self) -> Hash {
        self.environment.contract_address
    }

    /// Obtient la valeur envoyée avec l'appel
    pub fn get_value(&self) -> u64 {
        self.environment.value_sent
    }

    /// Finalise le contexte et applique les changements
    pub fn finalize(mut self) -> ContractResult<Vec<StateChange>> {
        // Applique tous les changements de storage à la blockchain
        for ((contract_address, key), value) in self.temp_storage {
            self.provider.write_storage(contract_address, &key, &value)?;
        }

        // TODO: Exécuter les transferts de tokens
        // TODO: Persister les events et logs

        Ok(self.state_changes)
    }

    /// Accès en lecture seule aux logs
    pub fn get_logs(&self) -> &Vec<String> {
        &self.logs
    }

    /// Accès en lecture seule aux events
    pub fn get_events(&self) -> &Vec<ContractEvent> {
        &self.events
    }

    /// Accès en lecture seule aux changements d'état
    pub fn get_state_changes(&self) -> &Vec<StateChange> {
        &self.state_changes
    }

    /// Accès en lecture seule aux transferts de tokens
    pub fn get_token_transfers(&self) -> &Vec<TokenTransfer> {
        &self.token_transfers
    }
}

/// Implémentation par défaut du ContextProvider pour les tests
#[cfg(test)]
pub struct MockContextProvider {
    blocks: HashMap<Hash, Block>,
    transactions: HashMap<Hash, Transaction>,
    balances: HashMap<PublicKey, u64>,
    storage: HashMap<(Hash, Vec<u8>), Vec<u8>>,
    contracts: HashMap<Hash, Vec<u8>>,
}

#[cfg(test)]
impl MockContextProvider {
    pub fn new() -> Self {
        Self {
            blocks: HashMap::new(),
            transactions: HashMap::new(),
            balances: HashMap::new(),
            storage: HashMap::new(),
            contracts: HashMap::new(),
        }
    }

    pub fn set_balance(&mut self, address: PublicKey, balance: u64) {
        self.balances.insert(address, balance);
    }

    pub fn set_storage(&mut self, contract: Hash, key: Vec<u8>, value: Vec<u8>) {
        self.storage.insert((contract, key), value);
    }
}

#[cfg(test)]
impl ContextProvider for MockContextProvider {
    fn get_block(&self, block_hash: Hash) -> ContractResult<Option<Block>> {
        Ok(self.blocks.get(&block_hash).cloned())
    }

    fn get_current_block(&self) -> ContractResult<Block> {
        // Retourne un bloc par défaut pour les tests
        Err(ContractError::InvalidState {
            message: "No current block set in mock".to_string(),
        })
    }

    fn get_transaction(&self, tx_hash: Hash) -> ContractResult<Option<Transaction>> {
        Ok(self.transactions.get(&tx_hash).cloned())
    }

    fn get_balance(&self, address: &PublicKey) -> ContractResult<u64> {
        Ok(self.balances.get(address).copied().unwrap_or(0))
    }

    fn read_storage(&self, contract_address: Hash, key: &[u8]) -> ContractResult<Option<Vec<u8>>> {
        let storage_key = (contract_address, key.to_vec());
        Ok(self.storage.get(&storage_key).cloned())
    }

    fn write_storage(&mut self, contract_address: Hash, key: &[u8], value: &[u8]) -> ContractResult<()> {
        let storage_key = (contract_address, key.to_vec());
        self.storage.insert(storage_key, value.to_vec());
        Ok(())
    }

    fn contract_exists(&self, address: Hash) -> ContractResult<bool> {
        Ok(self.contracts.contains_key(&address))
    }

    fn get_contract_code(&self, address: Hash) -> ContractResult<Option<Vec<u8>>> {
        Ok(self.contracts.get(&address).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair};

    #[test]
    fn test_context_creation() {
        let keypair = generate_keypair().unwrap();
        let environment = ExecutionEnvironment {
            block_hash: Hash::zero(),
            block_number: 1,
            block_timestamp: Utc::now(),
            transaction_hash: Hash::zero(),
            transaction_sender: keypair.public_key().clone(),
            contract_address: Hash::zero(),
            caller_address: keypair.public_key().clone(),
            value_sent: 0,
            gas_limit: 1000000,
            gas_price: 1,
        };

        let provider = Box::new(MockContextProvider::new());
        let context = ContractContext::new(environment, provider);

        assert_eq!(context.environment.block_number, 1);
        assert_eq!(context.logs.len(), 0);
        assert_eq!(context.events.len(), 0);
    }

    #[test]
    fn test_storage_operations() {
        let keypair = generate_keypair().unwrap();
        let environment = ExecutionEnvironment {
            block_hash: Hash::zero(),
            block_number: 1,
            block_timestamp: Utc::now(),
            transaction_hash: Hash::zero(),
            transaction_sender: keypair.public_key().clone(),
            contract_address: Hash::zero(),
            caller_address: keypair.public_key().clone(),
            value_sent: 0,
            gas_limit: 1000000,
            gas_price: 1,
        };

        let provider = Box::new(MockContextProvider::new());
        let mut context = ContractContext::new(environment, provider);

        // Test écriture/lecture
        let key = b"test_key";
        let value = b"test_value";
        
        context.storage_write(key, value).unwrap();
        let read_value = context.storage_read(key).unwrap();
        
        assert_eq!(read_value, Some(value.to_vec()));
        assert_eq!(context.state_changes.len(), 1);
    }

    #[test]
    fn test_event_emission() {
        let keypair = generate_keypair().unwrap();
        let environment = ExecutionEnvironment {
            block_hash: Hash::zero(),
            block_number: 1,
            block_timestamp: Utc::now(),
            transaction_hash: Hash::zero(),
            transaction_sender: keypair.public_key().clone(),
            contract_address: Hash::zero(),
            caller_address: keypair.public_key().clone(),
            value_sent: 0,
            gas_limit: 1000000,
            gas_price: 1,
        };

        let provider = Box::new(MockContextProvider::new());
        let mut context = ContractContext::new(environment, provider);

        context.emit_event("TestEvent".to_string(), vec![1, 2, 3], vec![Hash::zero()]);
        context.emit_log("Test log message".to_string());

        assert_eq!(context.events.len(), 1);
        assert_eq!(context.logs.len(), 1);
        assert_eq!(context.events[0].name, "TestEvent");
        assert_eq!(context.logs[0], "Test log message");
    }
}