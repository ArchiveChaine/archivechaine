//! Module de smart contracts pour ArchiveChain
//!
//! Ce module implémente le système de smart contracts avec support WASM,
//! incluant les contrats d'archivage automatisé, les bounties et les pools
//! de préservation.

pub mod runtime;
pub mod context;
pub mod gas;
pub mod abi;
pub mod manager;
pub mod archive_bounty;
pub mod preservation_pool;
pub mod content_verification;

// Re-exports pour l'interface publique
pub use runtime::{WasmRuntime, ContractExecution, ExecutionResult};
pub use context::{ContractContext, ContextProvider};
pub use gas::{GasManager, GasCost, GasLimit};
pub use abi::{ContractAbi, ContractCall, ContractEvent, ContractError as AbiError};
pub use manager::{ContractManager, ContractRegistry, ContractDeployment};
pub use archive_bounty::{ArchiveBountyContract, ArchiveBounty, BountyStatus, QualityLevel};
pub use preservation_pool::{PreservationPoolContract, PreservationPool, PoolParticipant};
pub use content_verification::{ContentVerificationContract, ContentVerification, VerificationRules};

use serde::{Deserialize, Serialize};
use crate::crypto::Hash;
use crate::error::CoreError;

/// Type de résultat pour les opérations de contrats
pub type ContractResult<T> = std::result::Result<T, ContractError>;

/// Erreurs spécifiques aux smart contracts
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ContractError {
    #[error("Erreur d'exécution WASM: {message}")]
    WasmExecution { message: String },

    #[error("Gas insuffisant: requis {required}, disponible {available}")]
    InsufficientGas { required: u64, available: u64 },

    #[error("Contrat non trouvé: {address}")]
    ContractNotFound { address: Hash },

    #[error("Fonction de contrat non trouvée: {function}")]
    FunctionNotFound { function: String },

    #[error("Paramètres invalides: {message}")]
    InvalidParameters { message: String },

    #[error("État de contrat invalide: {message}")]
    InvalidState { message: String },

    #[error("Erreur de sérialisation: {message}")]
    Serialization { message: String },

    #[error("Erreur d'autorisation: {message}")]
    Unauthorized { message: String },

    #[error("Deadline expirée")]
    DeadlineExpired,

    #[error("Fonds insuffisants: requis {required}, disponible {available}")]
    InsufficientFunds { required: u64, available: u64 },

    #[error("Contrat déjà complété")]
    AlreadyCompleted,

    #[error("Qualité insuffisante: requis {required:?}, fourni {provided:?}")]
    InsufficientQuality { required: QualityLevel, provided: QualityLevel },

    #[error("Consensus insuffisant: requis {required}, atteint {achieved}")]
    InsufficientConsensus { required: f64, achieved: f64 },
}

impl From<ContractError> for CoreError {
    fn from(err: ContractError) -> Self {
        CoreError::Internal { 
            message: format!("Contract error: {}", err) 
        }
    }
}

/// Adresse d'un smart contract
pub type ContractAddress = Hash;

/// Version d'un smart contract
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContractVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl ContractVersion {
    pub fn new(major: u32, minor: u32, patch: u32) -> Self {
        Self { major, minor, patch }
    }
}

impl std::fmt::Display for ContractVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Métadonnées d'un smart contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractMetadata {
    pub name: String,
    pub version: ContractVersion,
    pub description: String,
    pub author: String,
    pub license: String,
    pub abi_hash: Hash,
}

/// Interface principale pour les smart contracts
pub trait SmartContract {
    type State: Serialize + for<'de> Deserialize<'de>;
    type CallData: Serialize + for<'de> Deserialize<'de>;
    type ReturnData: Serialize + for<'de> Deserialize<'de>;

    /// Initialise le contrat avec l'état initial
    fn initialize(&mut self, context: &mut ContractContext) -> ContractResult<()>;

    /// Exécute un appel de fonction sur le contrat
    fn call(
        &mut self,
        function: &str,
        call_data: Self::CallData,
        context: &mut ContractContext,
    ) -> ContractResult<Self::ReturnData>;

    /// Obtient l'état actuel du contrat
    fn get_state(&self) -> &Self::State;

    /// Met à jour l'état du contrat
    fn set_state(&mut self, state: Self::State);

    /// Obtient les métadonnées du contrat
    fn metadata(&self) -> ContractMetadata;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_contract_version() {
        let version = ContractVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_contract_error_conversion() {
        let contract_err = ContractError::ContractNotFound { 
            address: Hash::zero() 
        };
        let core_err: CoreError = contract_err.into();
        assert!(matches!(core_err, CoreError::Internal { .. }));
    }
}