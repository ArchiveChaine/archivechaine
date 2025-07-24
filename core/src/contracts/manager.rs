//! Gestionnaire de smart contracts pour ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, PublicKey};
use crate::contracts::{
    ContractError, ContractResult, ContractContext, ContractMetadata, 
    ContractVersion, WasmRuntime, WasmRuntimeConfig, ContractExecution,
    ArchiveBountyContract, PreservationPoolContract, ContentVerificationContract,
    SmartContract
};

/// Types de contrats supportés
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractType {
    /// Contract d'Archive Bounty
    ArchiveBounty,
    /// Contract de Preservation Pool
    PreservationPool,
    /// Contract de Content Verification
    ContentVerification,
    /// Contract WASM personnalisé
    CustomWasm,
}

/// Statut d'un contrat déployé
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContractStatus {
    /// Contrat actif
    Active,
    /// Contrat suspendu
    Suspended,
    /// Contrat désactivé
    Disabled,
    /// Contrat en cours de mise à jour
    Upgrading,
    /// Contrat détruit
    Destroyed,
}

/// Permissions d'un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractPermissions {
    /// Propriétaire du contrat
    pub owner: PublicKey,
    /// Administrateurs autorisés
    pub admins: Vec<PublicKey>,
    /// Utilisateurs autorisés (si liste blanche)
    pub authorized_users: Option<Vec<PublicKey>>,
    /// Le contrat est-il public
    pub is_public: bool,
    /// Permissions spécifiques
    pub specific_permissions: HashMap<String, Vec<PublicKey>>,
}

impl Default for ContractPermissions {
    fn default() -> Self {
        Self {
            owner: PublicKey::from_bytes(&[0u8; 32]).unwrap(),
            admins: Vec::new(),
            authorized_users: None,
            is_public: true,
            specific_permissions: HashMap::new(),
        }
    }
}

/// Configuration de déploiement d'un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractDeployment {
    /// Adresse du contrat déployé
    pub address: Hash,
    /// Type de contrat
    pub contract_type: ContractType,
    /// Métadonnées du contrat
    pub metadata: ContractMetadata,
    /// Bytecode WASM (pour les contrats personnalisés)
    pub wasm_bytecode: Option<Vec<u8>>,
    /// Statut du contrat
    pub status: ContractStatus,
    /// Permissions
    pub permissions: ContractPermissions,
    /// Date de déploiement
    pub deployed_at: DateTime<Utc>,
    /// Adresse du déployeur
    pub deployer: PublicKey,
    /// Configuration d'exécution
    pub runtime_config: WasmRuntimeConfig,
    /// Limite de gas par défaut
    pub default_gas_limit: u64,
    /// Version du contrat
    pub version: ContractVersion,
    /// Contrat précédent (pour les mises à jour)
    pub previous_version: Option<Hash>,
    /// Statistiques d'utilisation
    pub usage_stats: ContractUsageStats,
}

/// Statistiques d'utilisation d'un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractUsageStats {
    /// Nombre total d'appels
    pub total_calls: u64,
    /// Gas total consommé
    pub total_gas_consumed: u64,
    /// Dernier appel
    pub last_call: Option<DateTime<Utc>>,
    /// Erreurs totales
    pub total_errors: u64,
    /// Temps d'exécution moyen (ms)
    pub average_execution_time: u64,
}

impl Default for ContractUsageStats {
    fn default() -> Self {
        Self {
            total_calls: 0,
            total_gas_consumed: 0,
            last_call: None,
            total_errors: 0,
            average_execution_time: 0,
        }
    }
}

/// Registre de tous les contrats déployés
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRegistry {
    /// Contrats indexés par adresse
    pub contracts: HashMap<Hash, ContractDeployment>,
    /// Index par type de contrat
    pub contracts_by_type: HashMap<ContractType, Vec<Hash>>,
    /// Index par propriétaire
    pub contracts_by_owner: HashMap<PublicKey, Vec<Hash>>,
    /// Index par statut
    pub contracts_by_status: HashMap<ContractStatus, Vec<Hash>>,
    /// Compteur d'adresses de contrats
    pub next_contract_address: u64,
    /// Statistiques globales
    pub global_stats: GlobalContractStats,
}

/// Statistiques globales des contrats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalContractStats {
    pub total_contracts_deployed: u64,
    pub active_contracts: u64,
    pub total_contract_calls: u64,
    pub total_gas_consumed: u64,
    pub average_contract_age_days: u64,
    pub most_used_contract_type: Option<ContractType>,
}

impl Default for ContractRegistry {
    fn default() -> Self {
        Self {
            contracts: HashMap::new(),
            contracts_by_type: HashMap::new(),
            contracts_by_owner: HashMap::new(),
            contracts_by_status: HashMap::new(),
            next_contract_address: 1,
            global_stats: GlobalContractStats {
                total_contracts_deployed: 0,
                active_contracts: 0,
                total_contract_calls: 0,
                total_gas_consumed: 0,
                average_contract_age_days: 0,
                most_used_contract_type: None,
            },
        }
    }
}

/// Gestionnaire principal des smart contracts
pub struct ContractManager {
    /// Registre des contrats
    registry: ContractRegistry,
    /// Runtime WASM
    wasm_runtime: WasmRuntime,
    /// Instances des contrats natifs
    native_contracts: NativeContracts,
}

/// Instances des contrats natifs (non-WASM)
struct NativeContracts {
    archive_bounty: ArchiveBountyContract,
    preservation_pool: PreservationPoolContract,
    content_verification: ContentVerificationContract,
}

impl Default for NativeContracts {
    fn default() -> Self {
        Self {
            archive_bounty: ArchiveBountyContract::default(),
            preservation_pool: PreservationPoolContract::default(),
            content_verification: ContentVerificationContract::default(),
        }
    }
}

impl ContractManager {
    /// Crée un nouveau gestionnaire de contrats
    pub fn new() -> ContractResult<Self> {
        let runtime_config = WasmRuntimeConfig::default();
        let wasm_runtime = WasmRuntime::new(runtime_config)?;
        
        Ok(Self {
            registry: ContractRegistry::default(),
            wasm_runtime,
            native_contracts: NativeContracts::default(),
        })
    }

    /// Déploie un contrat natif
    pub fn deploy_native_contract(
        &mut self,
        contract_type: ContractType,
        deployer: PublicKey,
        permissions: ContractPermissions,
        context: &mut ContractContext,
    ) -> ContractResult<Hash> {
        // Génère une adresse unique pour le contrat
        let contract_address = self.generate_contract_address(contract_type.clone(), &deployer);

        // Crée les métadonnées selon le type
        let metadata = match contract_type {
            ContractType::ArchiveBounty => self.native_contracts.archive_bounty.metadata(),
            ContractType::PreservationPool => self.native_contracts.preservation_pool.metadata(),
            ContractType::ContentVerification => self.native_contracts.content_verification.metadata(),
            _ => return Err(ContractError::InvalidParameters {
                message: "Invalid native contract type".to_string(),
            }),
        };

        let deployment = ContractDeployment {
            address: contract_address,
            contract_type: contract_type.clone(),
            metadata,
            wasm_bytecode: None, // Pas de bytecode pour les contrats natifs
            status: ContractStatus::Active,
            permissions,
            deployed_at: Utc::now(),
            deployer: deployer.clone(),
            runtime_config: WasmRuntimeConfig::default(),
            default_gas_limit: 1_000_000,
            version: ContractVersion::new(1, 0, 0),
            previous_version: None,
            usage_stats: ContractUsageStats::default(),
        };

        // Initialise le contrat
        match contract_type {
            ContractType::ArchiveBounty => {
                self.native_contracts.archive_bounty.initialize(context)?;
            }
            ContractType::PreservationPool => {
                self.native_contracts.preservation_pool.initialize(context)?;
            }
            ContractType::ContentVerification => {
                self.native_contracts.content_verification.initialize(context)?;
            }
            _ => {}
        }

        // Enregistre le déploiement
        self.register_contract(deployment)?;

        // Émet un event
        context.emit_event(
            "ContractDeployed".to_string(),
            bincode::serialize(&contract_address).unwrap_or_default(),
            vec![
                context.compute_hash(&deployer.as_bytes())?,
                contract_address,
            ],
        );

        context.emit_log(format!(
            "Native contract {:?} deployed at address {:?} by {:?}",
            contract_type, contract_address, deployer
        ));

        Ok(contract_address)
    }

    /// Déploie un contrat WASM personnalisé
    pub fn deploy_wasm_contract(
        &mut self,
        wasm_bytecode: Vec<u8>,
        metadata: ContractMetadata,
        deployer: PublicKey,
        permissions: ContractPermissions,
        runtime_config: WasmRuntimeConfig,
        context: &mut ContractContext,
    ) -> ContractResult<Hash> {
        // Génère une adresse unique
        let contract_address = self.generate_contract_address(ContractType::CustomWasm, &deployer);

        // Compile le module WASM
        self.wasm_runtime.compile_module(&wasm_bytecode, contract_address)?;

        let deployment = ContractDeployment {
            address: contract_address,
            contract_type: ContractType::CustomWasm,
            metadata,
            wasm_bytecode: Some(wasm_bytecode),
            status: ContractStatus::Active,
            permissions,
            deployed_at: Utc::now(),
            deployer: deployer.clone(),
            runtime_config,
            default_gas_limit: 1_000_000,
            version: ContractVersion::new(1, 0, 0),
            previous_version: None,
            usage_stats: ContractUsageStats::default(),
        };

        // Enregistre le déploiement
        self.register_contract(deployment)?;

        // Émet un event
        context.emit_event(
            "WasmContractDeployed".to_string(),
            bincode::serialize(&contract_address).unwrap_or_default(),
            vec![
                context.compute_hash(&deployer.as_bytes())?,
                contract_address,
            ],
        );

        context.emit_log(format!(
            "WASM contract deployed at address {:?} by {:?}",
            contract_address, deployer
        ));

        Ok(contract_address)
    }

    /// Appelle une fonction d'un contrat
    pub fn call_contract(
        &mut self,
        contract_address: Hash,
        function_name: String,
        call_data: Vec<u8>,
        gas_limit: u64,
        context: &mut ContractContext,
    ) -> ContractResult<Vec<u8>> {
        let deployment = self.registry.contracts.get_mut(&contract_address)
            .ok_or(ContractError::ContractNotFound { address: contract_address })?;

        // Vérifie le statut du contrat
        if deployment.status != ContractStatus::Active {
            return Err(ContractError::InvalidState {
                message: format!("Contract is {:?}", deployment.status),
            });
        }

        // Vérifie les permissions
        self.check_permissions(deployment, context.get_caller())?;

        let start_time = std::time::Instant::now();
        let result = match deployment.contract_type {
            ContractType::ArchiveBounty => {
                self.call_native_contract(&mut self.native_contracts.archive_bounty, &function_name, call_data, context)
            }
            ContractType::PreservationPool => {
                self.call_native_contract(&mut self.native_contracts.preservation_pool, &function_name, call_data, context)
            }
            ContractType::ContentVerification => {
                self.call_native_contract(&mut self.native_contracts.content_verification, &function_name, call_data, context)
            }
            ContractType::CustomWasm => {
                self.call_wasm_contract(contract_address, &function_name, call_data, gas_limit, context)
            }
        };

        let execution_time = start_time.elapsed().as_millis() as u64;

        // Met à jour les statistiques
        deployment.usage_stats.total_calls += 1;
        deployment.usage_stats.last_call = Some(Utc::now());
        deployment.usage_stats.average_execution_time = 
            (deployment.usage_stats.average_execution_time + execution_time) / 2;

        match &result {
            Ok(_) => {
                // Succès - pas d'action particulière
            }
            Err(_) => {
                deployment.usage_stats.total_errors += 1;
            }
        }

        // Met à jour les statistiques globales
        self.registry.global_stats.total_contract_calls += 1;

        result
    }

    /// Appelle un contrat natif
    fn call_native_contract<T>(
        &self,
        contract: &mut T,
        function_name: &str,
        call_data: Vec<u8>,
        context: &mut ContractContext,
    ) -> ContractResult<Vec<u8>>
    where
        T: SmartContract,
        T::CallData: for<'de> serde::Deserialize<'de>,
        T::ReturnData: serde::Serialize,
    {
        // Désérialise les données d'appel
        let parsed_call_data: T::CallData = bincode::deserialize(&call_data)
            .map_err(|e| ContractError::InvalidParameters {
                message: format!("Failed to deserialize call data: {}", e),
            })?;

        // Appelle la fonction
        let return_data = contract.call(function_name, parsed_call_data, context)?;

        // Sérialise le résultat
        bincode::serialize(&return_data)
            .map_err(|e| ContractError::Serialization {
                message: format!("Failed to serialize return data: {}", e),
            })
    }

    /// Appelle un contrat WASM
    fn call_wasm_contract(
        &self,
        contract_address: Hash,
        function_name: &str,
        call_data: Vec<u8>,
        gas_limit: u64,
        context: &mut ContractContext,
    ) -> ContractResult<Vec<u8>> {
        // Charge le contrat WASM
        let mut execution = self.wasm_runtime.load_contract(
            contract_address,
            context.clone(), // Ici il faudrait implémenter Clone pour ContractContext
            gas_limit,
        )?;

        // Prépare les arguments pour WASM
        let args = vec![
            wasmer::Value::I32(call_data.as_ptr() as i32),
            wasmer::Value::I32(call_data.len() as i32),
        ];

        // Exécute la fonction
        let execution_result = execution.call_function(function_name, &args)?;

        // Vérifie le succès
        if !execution_result.success {
            return Err(ContractError::WasmExecution {
                message: execution_result.error_message.unwrap_or("Unknown error".to_string()),
            });
        }

        Ok(execution_result.return_data)
    }

    /// Vérifie les permissions d'accès
    fn check_permissions(
        &self,
        deployment: &ContractDeployment,
        caller: &PublicKey,
    ) -> ContractResult<()> {
        // Le propriétaire a toujours accès
        if deployment.permissions.owner == *caller {
            return Ok(());
        }

        // Les admins ont accès
        if deployment.permissions.admins.contains(caller) {
            return Ok(());
        }

        // Si c'est public, tout le monde a accès
        if deployment.permissions.is_public {
            return Ok(());
        }

        // Sinon, vérifie la liste blanche
        if let Some(ref authorized) = deployment.permissions.authorized_users {
            if authorized.contains(caller) {
                return Ok(());
            }
        }

        Err(ContractError::Unauthorized {
            message: "Access denied to contract".to_string(),
        })
    }

    /// Génère une adresse de contrat unique
    fn generate_contract_address(&mut self, contract_type: ContractType, deployer: &PublicKey) -> Hash {
        let address_data = bincode::serialize(&(
            contract_type,
            deployer,
            self.registry.next_contract_address,
            Utc::now().timestamp()
        )).unwrap_or_default();
        
        self.registry.next_contract_address += 1;
        crate::crypto::compute_blake3(&address_data)
    }

    /// Enregistre un nouveau contrat dans le registre
    fn register_contract(&mut self, deployment: ContractDeployment) -> ContractResult<()> {
        let address = deployment.address;
        let contract_type = deployment.contract_type.clone();
        let owner = deployment.deployer.clone();
        let status = deployment.status.clone();

        // Ajoute au registre principal
        self.registry.contracts.insert(address, deployment);

        // Met à jour les index
        self.registry.contracts_by_type
            .entry(contract_type)
            .or_insert_with(Vec::new)
            .push(address);

        self.registry.contracts_by_owner
            .entry(owner)
            .or_insert_with(Vec::new)
            .push(address);

        self.registry.contracts_by_status
            .entry(status)
            .or_insert_with(Vec::new)
            .push(address);

        // Met à jour les statistiques
        self.registry.global_stats.total_contracts_deployed += 1;
        if self.registry.contracts.get(&address).unwrap().status == ContractStatus::Active {
            self.registry.global_stats.active_contracts += 1;
        }

        Ok(())
    }

    /// Obtient les informations d'un contrat
    pub fn get_contract_info(&self, contract_address: Hash) -> ContractResult<&ContractDeployment> {
        self.registry.contracts.get(&contract_address)
            .ok_or(ContractError::ContractNotFound { address: contract_address })
    }

    /// Liste les contrats par type
    pub fn list_contracts_by_type(&self, contract_type: ContractType) -> Vec<Hash> {
        self.registry.contracts_by_type
            .get(&contract_type)
            .cloned()
            .unwrap_or_default()
    }

    /// Liste les contrats d'un propriétaire
    pub fn list_contracts_by_owner(&self, owner: &PublicKey) -> Vec<Hash> {
        self.registry.contracts_by_owner
            .get(owner)
            .cloned()
            .unwrap_or_default()
    }

    /// Suspend un contrat
    pub fn suspend_contract(
        &mut self,
        contract_address: Hash,
        caller: &PublicKey,
    ) -> ContractResult<()> {
        let deployment = self.registry.contracts.get_mut(&contract_address)
            .ok_or(ContractError::ContractNotFound { address: contract_address })?;

        // Vérifie les permissions (seul le propriétaire ou un admin peut suspendre)
        if deployment.permissions.owner != *caller && !deployment.permissions.admins.contains(caller) {
            return Err(ContractError::Unauthorized {
                message: "Only owner or admin can suspend contract".to_string(),
            });
        }

        deployment.status = ContractStatus::Suspended;
        Ok(())
    }

    /// Réactive un contrat suspendu
    pub fn reactivate_contract(
        &mut self,
        contract_address: Hash,
        caller: &PublicKey,
    ) -> ContractResult<()> {
        let deployment = self.registry.contracts.get_mut(&contract_address)
            .ok_or(ContractError::ContractNotFound { address: contract_address })?;

        // Vérifie les permissions
        if deployment.permissions.owner != *caller && !deployment.permissions.admins.contains(caller) {
            return Err(ContractError::Unauthorized {
                message: "Only owner or admin can reactivate contract".to_string(),
            });
        }

        if deployment.status == ContractStatus::Suspended {
            deployment.status = ContractStatus::Active;
        }

        Ok(())
    }

    /// Obtient les statistiques globales
    pub fn get_global_stats(&self) -> &GlobalContractStats {
        &self.registry.global_stats
    }

    /// Obtient le registre complet (pour les tests et la debug)
    pub fn get_registry(&self) -> &ContractRegistry {
        &self.registry
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::context::MockContextProvider;
    use crate::contracts::ExecutionEnvironment;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_contract_manager_creation() {
        let manager = ContractManager::new();
        assert!(manager.is_ok());
    }

    #[test]
    fn test_native_contract_deployment() {
        let mut manager = ContractManager::new().unwrap();
        let keypair = generate_keypair().unwrap();
        
        let env = ExecutionEnvironment {
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

        let provider = MockContextProvider::new();
        let mut context = ContractContext::new(env, Box::new(provider));

        let permissions = ContractPermissions {
            owner: keypair.public_key().clone(),
            ..Default::default()
        };

        let contract_address = manager.deploy_native_contract(
            ContractType::ArchiveBounty,
            keypair.public_key().clone(),
            permissions,
            &mut context,
        ).unwrap();

        assert!(!contract_address.is_zero());
        assert_eq!(manager.registry.contracts.len(), 1);
        assert_eq!(manager.registry.global_stats.total_contracts_deployed, 1);
    }

    #[test]
    fn test_contract_types() {
        assert_eq!(ContractType::ArchiveBounty, ContractType::ArchiveBounty);
        assert_ne!(ContractType::ArchiveBounty, ContractType::PreservationPool);
    }

    #[test]
    fn test_contract_permissions() {
        let permissions = ContractPermissions::default();
        assert!(permissions.is_public);
        assert!(permissions.admins.is_empty());
    }
}