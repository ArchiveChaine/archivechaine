//! Types d'erreurs pour ArchiveChain Core

use thiserror::Error;

/// Type de résultat standard pour le module core
pub type Result<T> = std::result::Result<T, CoreError>;

/// Erreurs principales du module core
#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Erreur cryptographique: {0}")]
    Crypto(#[from] CryptoError),

    #[error("Erreur de bloc: {0}")]
    Block(#[from] BlockError),

    #[error("Erreur de transaction: {0}")]
    Transaction(#[from] TransactionError),

    #[error("Erreur d'état: {0}")]
    State(#[from] StateError),

    #[error("Erreur de consensus: {0}")]
    Consensus(#[from] ConsensusError),

    #[error("Erreur de sérialisation: {0}")]
    Serialization(#[from] SerializationError),

    #[error("Erreur de validation: {message}")]
    Validation { message: String },

    #[error("Erreur interne: {message}")]
    Internal { message: String },

    #[error("Entrée invalide: {0}")]
    InvalidInput(String),

    #[error("Élément non trouvé: {message}")]
    NotFound { message: String },
}

/// Alias pour CoreError pour compatibilité
pub type ArchiveChainError = CoreError;

/// Erreurs cryptographiques
#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Signature invalide")]
    InvalidSignature,

    #[error("Clé publique invalide")]
    InvalidPublicKey,

    #[error("Clé privée invalide")]
    InvalidPrivateKey,

    #[error("Hash invalide: longueur attendue {expected}, reçue {actual}")]
    InvalidHashLength { expected: usize, actual: usize },

    #[error("Erreur de génération aléatoire: {0}")]
    RandomGeneration(String),

    #[error("Erreur de décodage hexadécimal: {0}")]
    HexDecode(#[from] hex::FromHexError),
}

/// Erreurs de bloc
#[derive(Error, Debug)]
pub enum BlockError {
    #[error("Hash de bloc invalide")]
    InvalidHash,

    #[error("En-tête de bloc invalide")]
    InvalidHeader,

    #[error("Timestamp invalide")]
    InvalidTimestamp,

    #[error("Nonce invalide")]
    InvalidNonce,

    #[error("Métadonnées d'archive invalides")]
    InvalidArchiveMetadata,

    #[error("Index de contenu invalide")]
    InvalidContentIndex,

    #[error("Preuve de stockage invalide")]
    InvalidStorageProof,
}

/// Erreurs de transaction
#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("Transaction invalide")]
    Invalid,

    #[error("Signature de transaction invalide")]
    InvalidSignature,

    #[error("Solde insuffisant")]
    InsufficientBalance,

    #[error("Nonce invalide")]
    InvalidNonce,
}

/// Erreurs d'état
#[derive(Error, Debug)]
pub enum StateError {
    #[error("Racine Merkle invalide")]
    InvalidMerkleRoot,

    #[error("Nœud Merkle introuvable")]
    MerkleNodeNotFound,

    #[error("État inconsistant")]
    InconsistentState,
}

/// Erreurs de sérialisation
#[derive(Error, Debug)]
pub enum SerializationError {
    #[error("Erreur bincode: {0}")]
    Bincode(#[from] bincode::Error),

    #[error("Erreur CBOR: {0}")]
    Cbor(String),

    #[error("Erreur JSON: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Format non supporté: {format}")]
    UnsupportedFormat { format: String },
}

impl From<cbor4ii::serde::SerializeError> for SerializationError {
    fn from(err: cbor4ii::serde::SerializeError) -> Self {
        SerializationError::Cbor(err.to_string())
    }
}

impl From<cbor4ii::serde::DeserializeError> for SerializationError {
    fn from(err: cbor4ii::serde::DeserializeError) -> Self {
        SerializationError::Cbor(err.to_string())
    }
}

/// Erreurs de consensus
#[derive(Error, Debug)]
pub enum ConsensusError {
    #[error("Score de consensus insuffisant")]
    InsufficientScore,

    #[error("Nœud non autorisé pour le consensus")]
    UnauthorizedNode,

    #[error("Preuve de stockage invalide: {0}")]
    InvalidStorageProof(String),

    #[error("Preuve de bande passante invalide: {0}")]
    InvalidBandwidthProof(String),

    #[error("Preuve de longévité invalide: {0}")]
    InvalidLongevityProof(String),

    #[error("Sélection de leader échouée: {0}")]
    LeaderSelectionFailed(String),

    #[error("Validation de consensus échouée: {0}")]
    ValidationFailed(String),

    #[error("Pool de récompenses insuffisant")]
    InsufficientRewardPool,

    #[error("Configuration de consensus invalide: {0}")]
    InvalidConfiguration(String),

    #[error("Epoch de consensus invalide")]
    InvalidEpoch,

    #[error("Timeout de consensus atteint")]
    ConsensusTimeout,

    #[error("Défis de consensus expirés")]
    ExpiredChallenge,
}