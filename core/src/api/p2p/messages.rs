//! Messages P2P pour ArchiveChain
//!
//! Définit tous les types de messages échangés entre les nœuds du réseau P2P.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Messages P2P principaux
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum P2PMessage {
    /// Handshake initial entre pairs
    Handshake {
        peer_id: String,
        protocol_version: String,
        client_version: String,
        block_height: u64,
        best_block_hash: String,
        capabilities: Vec<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Réponse au handshake
    HandshakeResponse {
        peer_id: String,
        protocol_version: String,
        client_version: String,
        block_height: u64,
        best_block_hash: String,
        capabilities: Vec<String>,
        accepted: bool,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Message de ping pour maintenir la connexion
    Ping {
        nonce: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Réponse au ping
    Pong {
        nonce: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Annonce d'un nouveau bloc
    BlockAnnouncement {
        block_hash: String,
        block_height: u64,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Demande d'un bloc
    BlockRequest {
        block_hash: String,
        request_id: String,
    },

    /// Réponse avec un bloc
    BlockResponse {
        block: Option<BlockData>,
        request_id: String,
    },

    /// Demande d'inventaire (hashes de blocs)
    InventoryRequest {
        start_height: u64,
        count: u32,
        request_id: String,
    },

    /// Réponse d'inventaire
    InventoryResponse {
        block_hashes: Vec<String>,
        request_id: String,
    },

    /// Annonce d'une nouvelle transaction
    TransactionAnnouncement {
        tx_hash: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Demande d'une transaction
    TransactionRequest {
        tx_hash: String,
        request_id: String,
    },

    /// Réponse avec une transaction
    TransactionResponse {
        transaction: Option<TransactionData>,
        request_id: String,
    },

    /// Annonce d'une nouvelle archive
    ArchiveAnnouncement {
        archive_id: String,
        url: String,
        status: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Demande de pairs connectés
    PeerRequest {
        max_peers: u32,
        request_id: String,
    },

    /// Réponse avec la liste des pairs
    PeerResponse {
        peers: Vec<PeerAddress>,
        request_id: String,
    },

    /// Demande de synchronisation
    SyncRequest {
        start_height: u64,
        end_height: Option<u64>,
        request_id: String,
    },

    /// Début de synchronisation
    SyncStart {
        start_height: u64,
        end_height: u64,
        request_id: String,
    },

    /// Données de synchronisation (batch de blocs)
    SyncData {
        blocks: Vec<BlockData>,
        request_id: String,
        is_last: bool,
    },

    /// Fin de synchronisation
    SyncEnd {
        request_id: String,
        success: bool,
        message: Option<String>,
    },

    /// Message de gossip générique
    Gossip {
        topic: String,
        data: serde_json::Value,
        ttl: u32,
        timestamp: chrono::DateTime<chrono::Utc>,
    },

    /// Demande de statut du réseau
    NetworkStatusRequest {
        request_id: String,
    },

    /// Réponse avec le statut du réseau
    NetworkStatusResponse {
        active_peers: u32,
        block_height: u64,
        network_hash_rate: Option<String>,
        request_id: String,
    },

    /// Message d'erreur
    Error {
        code: u32,
        message: String,
        request_id: Option<String>,
    },

    /// Fermeture de connexion
    Disconnect {
        reason: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
}

/// Données de bloc pour P2P
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockData {
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub transactions: Vec<TransactionData>,
    pub merkle_root: String,
    pub validator: String,
    pub signature: String,
}

/// Données de transaction pour P2P
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionData {
    pub hash: String,
    pub from: String,
    pub to: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub nonce: u64,
    pub signature: String,
    pub data: Option<Vec<u8>>,
}

/// Adresse de pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerAddress {
    pub peer_id: String,
    pub address: String,
    pub port: u16,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Types de messages par catégorie
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageCategory {
    Handshake,
    KeepAlive,
    Blockchain,
    Transaction,
    Archive,
    Peer,
    Sync,
    Gossip,
    Status,
    Error,
}

impl P2PMessage {
    /// Retourne la catégorie du message
    pub fn category(&self) -> MessageCategory {
        match self {
            P2PMessage::Handshake { .. } | P2PMessage::HandshakeResponse { .. } => MessageCategory::Handshake,
            P2PMessage::Ping { .. } | P2PMessage::Pong { .. } => MessageCategory::KeepAlive,
            P2PMessage::BlockAnnouncement { .. } | P2PMessage::BlockRequest { .. } | P2PMessage::BlockResponse { .. } | P2PMessage::InventoryRequest { .. } | P2PMessage::InventoryResponse { .. } => MessageCategory::Blockchain,
            P2PMessage::TransactionAnnouncement { .. } | P2PMessage::TransactionRequest { .. } | P2PMessage::TransactionResponse { .. } => MessageCategory::Transaction,
            P2PMessage::ArchiveAnnouncement { .. } => MessageCategory::Archive,
            P2PMessage::PeerRequest { .. } | P2PMessage::PeerResponse { .. } => MessageCategory::Peer,
            P2PMessage::SyncRequest { .. } | P2PMessage::SyncStart { .. } | P2PMessage::SyncData { .. } | P2PMessage::SyncEnd { .. } => MessageCategory::Sync,
            P2PMessage::Gossip { .. } => MessageCategory::Gossip,
            P2PMessage::NetworkStatusRequest { .. } | P2PMessage::NetworkStatusResponse { .. } => MessageCategory::Status,
            P2PMessage::Error { .. } | P2PMessage::Disconnect { .. } => MessageCategory::Error,
        }
    }

    /// Retourne l'ID de requête si applicable
    pub fn request_id(&self) -> Option<&str> {
        match self {
            P2PMessage::BlockRequest { request_id, .. } |
            P2PMessage::BlockResponse { request_id, .. } |
            P2PMessage::InventoryRequest { request_id, .. } |
            P2PMessage::InventoryResponse { request_id, .. } |
            P2PMessage::TransactionRequest { request_id, .. } |
            P2PMessage::TransactionResponse { request_id, .. } |
            P2PMessage::PeerRequest { request_id, .. } |
            P2PMessage::PeerResponse { request_id, .. } |
            P2PMessage::SyncRequest { request_id, .. } |
            P2PMessage::SyncStart { request_id, .. } |
            P2PMessage::SyncData { request_id, .. } |
            P2PMessage::SyncEnd { request_id, .. } |
            P2PMessage::NetworkStatusRequest { request_id, .. } |
            P2PMessage::NetworkStatusResponse { request_id, .. } => Some(request_id),
            P2PMessage::Error { request_id, .. } => request_id.as_deref(),
            _ => None,
        }
    }

    /// Vérifie si le message nécessite une réponse
    pub fn requires_response(&self) -> bool {
        matches!(self,
            P2PMessage::Handshake { .. } |
            P2PMessage::Ping { .. } |
            P2PMessage::BlockRequest { .. } |
            P2PMessage::InventoryRequest { .. } |
            P2PMessage::TransactionRequest { .. } |
            P2PMessage::PeerRequest { .. } |
            P2PMessage::SyncRequest { .. } |
            P2PMessage::NetworkStatusRequest { .. }
        )
    }

    /// Retourne la priorité du message (0 = haute priorité)
    pub fn priority(&self) -> u8 {
        match self.category() {
            MessageCategory::Handshake => 0,
            MessageCategory::KeepAlive => 1,
            MessageCategory::Error => 1,
            MessageCategory::Sync => 2,
            MessageCategory::Blockchain => 3,
            MessageCategory::Transaction => 4,
            MessageCategory::Archive => 5,
            MessageCategory::Peer => 6,
            MessageCategory::Gossip => 7,
            MessageCategory::Status => 8,
        }
    }
}

/// Builder pour créer facilement des messages P2P
pub struct MessageBuilder;

impl MessageBuilder {
    /// Crée un message de handshake
    pub fn handshake(
        peer_id: String,
        protocol_version: String,
        client_version: String,
        block_height: u64,
        best_block_hash: String,
        capabilities: Vec<String>,
    ) -> P2PMessage {
        P2PMessage::Handshake {
            peer_id,
            protocol_version,
            client_version,
            block_height,
            best_block_hash,
            capabilities,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée une réponse de handshake
    pub fn handshake_response(
        peer_id: String,
        protocol_version: String,
        client_version: String,
        block_height: u64,
        best_block_hash: String,
        capabilities: Vec<String>,
        accepted: bool,
    ) -> P2PMessage {
        P2PMessage::HandshakeResponse {
            peer_id,
            protocol_version,
            client_version,
            block_height,
            best_block_hash,
            capabilities,
            accepted,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de ping
    pub fn ping(nonce: u64) -> P2PMessage {
        P2PMessage::Ping {
            nonce,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de pong
    pub fn pong(nonce: u64) -> P2PMessage {
        P2PMessage::Pong {
            nonce,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée une annonce de bloc
    pub fn block_announcement(block_hash: String, block_height: u64) -> P2PMessage {
        P2PMessage::BlockAnnouncement {
            block_hash,
            block_height,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée une demande de bloc
    pub fn block_request(block_hash: String, request_id: String) -> P2PMessage {
        P2PMessage::BlockRequest {
            block_hash,
            request_id,
        }
    }

    /// Crée une réponse de bloc
    pub fn block_response(block: Option<BlockData>, request_id: String) -> P2PMessage {
        P2PMessage::BlockResponse {
            block,
            request_id,
        }
    }

    /// Crée un message de gossip
    pub fn gossip(topic: String, data: serde_json::Value, ttl: u32) -> P2PMessage {
        P2PMessage::Gossip {
            topic,
            data,
            ttl,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message d'erreur
    pub fn error(code: u32, message: String, request_id: Option<String>) -> P2PMessage {
        P2PMessage::Error {
            code,
            message,
            request_id,
        }
    }

    /// Crée un message de déconnexion
    pub fn disconnect(reason: String) -> P2PMessage {
        P2PMessage::Disconnect {
            reason,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Validateur de messages P2P
pub struct MessageValidator;

impl MessageValidator {
    /// Valide un message P2P
    pub fn validate(message: &P2PMessage) -> Result<(), String> {
        match message {
            P2PMessage::Handshake { peer_id, protocol_version, client_version, .. } => {
                if peer_id.is_empty() {
                    return Err("Peer ID cannot be empty".to_string());
                }
                if protocol_version.is_empty() {
                    return Err("Protocol version cannot be empty".to_string());
                }
                if client_version.is_empty() {
                    return Err("Client version cannot be empty".to_string());
                }
            }
            P2PMessage::BlockRequest { block_hash, request_id } => {
                if block_hash.is_empty() {
                    return Err("Block hash cannot be empty".to_string());
                }
                if request_id.is_empty() {
                    return Err("Request ID cannot be empty".to_string());
                }
                if block_hash.len() != 64 {
                    return Err("Invalid block hash length".to_string());
                }
            }
            P2PMessage::SyncRequest { start_height, end_height, .. } => {
                if let Some(end) = end_height {
                    if start_height >= end {
                        return Err("Start height must be less than end height".to_string());
                    }
                    if end - start_height > 1000 {
                        return Err("Sync range too large (max 1000 blocks)".to_string());
                    }
                }
            }
            P2PMessage::Gossip { topic, ttl, .. } => {
                if topic.is_empty() {
                    return Err("Gossip topic cannot be empty".to_string());
                }
                if *ttl == 0 {
                    return Err("Gossip TTL must be greater than 0".to_string());
                }
                if *ttl > 100 {
                    return Err("Gossip TTL too large (max 100)".to_string());
                }
            }
            _ => {} // Autres messages valides par construction
        }
        
        Ok(())
    }

    /// Valide les données de bloc
    pub fn validate_block_data(block: &BlockData) -> Result<(), String> {
        if block.hash.is_empty() {
            return Err("Block hash cannot be empty".to_string());
        }
        if block.hash.len() != 64 {
            return Err("Invalid block hash length".to_string());
        }
        if block.previous_hash.len() != 64 {
            return Err("Invalid previous block hash length".to_string());
        }
        if block.merkle_root.len() != 64 {
            return Err("Invalid merkle root length".to_string());
        }
        if block.validator.is_empty() {
            return Err("Validator cannot be empty".to_string());
        }
        if block.signature.is_empty() {
            return Err("Block signature cannot be empty".to_string());
        }
        
        Ok(())
    }

    /// Valide les données de transaction
    pub fn validate_transaction_data(tx: &TransactionData) -> Result<(), String> {
        if tx.hash.is_empty() {
            return Err("Transaction hash cannot be empty".to_string());
        }
        if tx.hash.len() != 64 {
            return Err("Invalid transaction hash length".to_string());
        }
        if tx.from.is_empty() {
            return Err("From address cannot be empty".to_string());
        }
        if tx.signature.is_empty() {
            return Err("Transaction signature cannot be empty".to_string());
        }
        
        Ok(())
    }
}

/// Codes d'erreur standard
pub mod error_codes {
    pub const PROTOCOL_VERSION_MISMATCH: u32 = 1000;
    pub const INVALID_MESSAGE_FORMAT: u32 = 1001;
    pub const MESSAGE_TOO_LARGE: u32 = 1002;
    pub const RATE_LIMIT_EXCEEDED: u32 = 1003;
    pub const PEER_BANNED: u32 = 1004;
    pub const RESOURCE_NOT_FOUND: u32 = 1005;
    pub const SYNC_ERROR: u32 = 1006;
    pub const INTERNAL_ERROR: u32 = 1999;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_category() {
        let ping = MessageBuilder::ping(12345);
        assert_eq!(ping.category(), MessageCategory::KeepAlive);

        let handshake = MessageBuilder::handshake(
            "peer_123".to_string(),
            "1.0".to_string(),
            "archivechain-0.1.0".to_string(),
            12345,
            "0x123456".to_string(),
            vec!["sync".to_string()],
        );
        assert_eq!(handshake.category(), MessageCategory::Handshake);
    }

    #[test]
    fn test_message_priority() {
        let handshake = MessageBuilder::handshake(
            "peer_123".to_string(),
            "1.0".to_string(),
            "archivechain-0.1.0".to_string(),
            12345,
            "0x123456".to_string(),
            vec![],
        );
        assert_eq!(handshake.priority(), 0);

        let gossip = MessageBuilder::gossip(
            "test".to_string(),
            serde_json::json!({"key": "value"}),
            10,
        );
        assert_eq!(gossip.priority(), 7);
    }

    #[test]
    fn test_message_requires_response() {
        let ping = MessageBuilder::ping(12345);
        assert!(ping.requires_response());

        let pong = MessageBuilder::pong(12345);
        assert!(!pong.requires_response());

        let block_request = MessageBuilder::block_request("0x123456".to_string(), "req_1".to_string());
        assert!(block_request.requires_response());
    }

    #[test]
    fn test_message_request_id() {
        let block_request = MessageBuilder::block_request("0x123456".to_string(), "req_1".to_string());
        assert_eq!(block_request.request_id(), Some("req_1"));

        let ping = MessageBuilder::ping(12345);
        assert_eq!(ping.request_id(), None);
    }

    #[test]
    fn test_message_validation() {
        let valid_handshake = MessageBuilder::handshake(
            "peer_123".to_string(),
            "1.0".to_string(),
            "archivechain-0.1.0".to_string(),
            12345,
            "0x123456".to_string(),
            vec![],
        );
        assert!(MessageValidator::validate(&valid_handshake).is_ok());

        let invalid_block_request = P2PMessage::BlockRequest {
            block_hash: "".to_string(), // Vide
            request_id: "req_1".to_string(),
        };
        assert!(MessageValidator::validate(&invalid_block_request).is_err());
    }

    #[test]
    fn test_block_data_validation() {
        let valid_block = BlockData {
            height: 12345,
            hash: "a".repeat(64),
            previous_hash: "b".repeat(64),
            timestamp: chrono::Utc::now(),
            transactions: vec![],
            merkle_root: "c".repeat(64),
            validator: "validator_123".to_string(),
            signature: "signature_123".to_string(),
        };
        assert!(MessageValidator::validate_block_data(&valid_block).is_ok());

        let invalid_block = BlockData {
            height: 12345,
            hash: "".to_string(), // Vide
            previous_hash: "b".repeat(64),
            timestamp: chrono::Utc::now(),
            transactions: vec![],
            merkle_root: "c".repeat(64),
            validator: "validator_123".to_string(),
            signature: "signature_123".to_string(),
        };
        assert!(MessageValidator::validate_block_data(&invalid_block).is_err());
    }

    #[test]
    fn test_transaction_data_validation() {
        let valid_tx = TransactionData {
            hash: "a".repeat(64),
            from: "from_address".to_string(),
            to: Some("to_address".to_string()),
            amount: 1000,
            fee: 10,
            nonce: 1,
            signature: "signature_123".to_string(),
            data: None,
        };
        assert!(MessageValidator::validate_transaction_data(&valid_tx).is_ok());

        let invalid_tx = TransactionData {
            hash: "".to_string(), // Vide
            from: "from_address".to_string(),
            to: Some("to_address".to_string()),
            amount: 1000,
            fee: 10,
            nonce: 1,
            signature: "signature_123".to_string(),
            data: None,
        };
        assert!(MessageValidator::validate_transaction_data(&invalid_tx).is_err());
    }

    #[test]
    fn test_message_serialization() {
        let ping = MessageBuilder::ping(12345);
        let serialized = serde_json::to_string(&ping).unwrap();
        let deserialized: P2PMessage = serde_json::from_str(&serialized).unwrap();
        
        match (ping, deserialized) {
            (P2PMessage::Ping { nonce: n1, .. }, P2PMessage::Ping { nonce: n2, .. }) => {
                assert_eq!(n1, n2);
            }
            _ => panic!("Serialization/deserialization failed"),
        }
    }
}