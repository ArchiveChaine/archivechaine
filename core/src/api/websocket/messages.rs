//! Messages WebSocket pour ArchiveChain
//!
//! Définit tous les types de messages WebSocket selon les spécifications API.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Types de messages WebSocket principaux
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsMessage {
    /// Authentification avec token JWT
    Auth {
        token: String,
    },
    
    /// Réponse d'authentification
    AuthResponse {
        success: bool,
        user_id: Option<String>,
        scopes: Vec<String>,
        message: Option<String>,
    },
    
    /// Souscription à un topic
    Subscribe {
        topics: Vec<String>,
        filters: Option<HashMap<String, serde_json::Value>>,
    },
    
    /// Désouscription d'un topic
    Unsubscribe {
        topics: Vec<String>,
    },
    
    /// Confirmation de souscription
    SubscriptionConfirmed {
        topics: Vec<String>,
        subscription_id: String,
    },
    
    /// Erreur de souscription
    SubscriptionError {
        topic: String,
        error: String,
    },
    
    /// Nouveau bloc
    NewBlock {
        block: BlockUpdate,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Nouvelle archive
    NewArchive {
        archive: ArchiveUpdate,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Mise à jour d'archive
    ArchiveUpdate {
        archive_id: String,
        status: String,
        progress: Option<f64>,
        data: Option<HashMap<String, serde_json::Value>>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Mise à jour des statistiques réseau
    NetworkStats {
        data: NetworkStatsUpdate,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Changement de statut de nœud
    NodeStatusChange {
        node_id: String,
        status: String,
        region: Option<String>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Mise à jour de bounty
    BountyUpdate {
        bounty_id: String,
        status: String,
        data: Option<HashMap<String, serde_json::Value>>,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Événement de contrat intelligent
    ContractEvent {
        contract_id: String,
        event_type: String,
        data: serde_json::Value,
        block_height: u64,
        transaction_hash: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Message de ping
    Ping {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Message de pong
    Pong {
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Message d'erreur générique
    Error {
        code: String,
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Message de succès générique
    Success {
        message: String,
        timestamp: chrono::DateTime<chrono::Utc>,
    },
    
    /// Requête de statut de connexion
    ConnectionStatus,
    
    /// Réponse de statut de connexion
    ConnectionStatusResponse {
        connection_id: String,
        authenticated: bool,
        subscriptions: Vec<String>,
        connected_since: chrono::DateTime<chrono::Utc>,
        stats: ConnectionStatsData,
    },
}

/// Mise à jour de bloc
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockUpdate {
    pub height: u64,
    pub hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub transactions: u32,
    pub archives: u32,
    pub validator: String,
    pub size: u64,
}

/// Mise à jour d'archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveUpdate {
    pub archive_id: String,
    pub url: String,
    pub status: String,
    pub size: Option<u64>,
    pub replicas: Option<u32>,
    pub integrity_score: Option<f64>,
    pub metadata: Option<ArchiveMetadataUpdate>,
}

/// Métadonnées d'archive mise à jour
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadataUpdate {
    pub title: Option<String>,
    pub content_type: Option<String>,
    pub language: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Mise à jour des statistiques réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatsUpdate {
    pub active_nodes: u32,
    pub total_storage: String,
    pub current_block_height: u64,
    pub network_latency: Option<String>,
    pub total_archives: Option<u64>,
    pub archives_today: Option<u32>,
}

/// Données de statistiques de connexion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStatsData {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub last_activity: chrono::DateTime<chrono::Utc>,
}

/// Topics de souscription disponibles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionTopic {
    /// Mises à jour d'archives
    ArchiveUpdates,
    /// Nouvelles archives
    NewArchives,
    /// Nouveaux blocs
    NewBlocks,
    /// Statistiques réseau
    NetworkStats,
    /// Changements de statut des nœuds
    NodeStatusChange,
    /// Mises à jour de bounties
    BountyUpdates,
    /// Événements de contrats
    ContractEvents,
    /// Toutes les mises à jour (admin seulement)
    All,
}

impl SubscriptionTopic {
    /// Retourne la chaîne de caractères pour le topic
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArchiveUpdates => "archive_updates",
            Self::NewArchives => "new_archives",
            Self::NewBlocks => "new_blocks",
            Self::NetworkStats => "network_stats",
            Self::NodeStatusChange => "node_status_change",
            Self::BountyUpdates => "bounty_updates",
            Self::ContractEvents => "contract_events",
            Self::All => "all",
        }
    }

    /// Parse une chaîne vers un topic
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "archive_updates" => Some(Self::ArchiveUpdates),
            "new_archives" => Some(Self::NewArchives),
            "new_blocks" => Some(Self::NewBlocks),
            "network_stats" => Some(Self::NetworkStats),
            "node_status_change" => Some(Self::NodeStatusChange),
            "bounty_updates" => Some(Self::BountyUpdates),
            "contract_events" => Some(Self::ContractEvents),
            "all" => Some(Self::All),
            _ => None,
        }
    }

    /// Retourne tous les topics disponibles
    pub fn all_topics() -> Vec<Self> {
        vec![
            Self::ArchiveUpdates,
            Self::NewArchives,
            Self::NewBlocks,
            Self::NetworkStats,
            Self::NodeStatusChange,
            Self::BountyUpdates,
            Self::ContractEvents,
        ]
    }

    /// Vérifie si un topic nécessite une authentification
    pub fn requires_auth(&self) -> bool {
        match self {
            Self::NetworkStats => false, // Public
            Self::All => true, // Admin seulement
            _ => true, // Nécessite authentification
        }
    }

    /// Vérifie si un topic nécessite des permissions spéciales
    pub fn required_scope(&self) -> Option<&'static str> {
        match self {
            Self::ArchiveUpdates | Self::NewArchives => Some("archives:read"),
            Self::NewBlocks | Self::NetworkStats => Some("network:read"),
            Self::NodeStatusChange => Some("node:manage"),
            Self::BountyUpdates => Some("bounties:read"),
            Self::ContractEvents => Some("contracts:read"),
            Self::All => Some("admin:all"),
        }
    }
}

/// Builder pour créer facilement des messages WebSocket
pub struct MessageBuilder;

impl MessageBuilder {
    /// Crée un message d'authentification
    pub fn auth(token: String) -> WsMessage {
        WsMessage::Auth { token }
    }

    /// Crée une réponse d'authentification réussie
    pub fn auth_success(user_id: String, scopes: Vec<String>) -> WsMessage {
        WsMessage::AuthResponse {
            success: true,
            user_id: Some(user_id),
            scopes,
            message: Some("Authentication successful".to_string()),
        }
    }

    /// Crée une réponse d'authentification échouée
    pub fn auth_failure(message: String) -> WsMessage {
        WsMessage::AuthResponse {
            success: false,
            user_id: None,
            scopes: vec![],
            message: Some(message),
        }
    }

    /// Crée un message de souscription
    pub fn subscribe(topics: Vec<String>, filters: Option<HashMap<String, serde_json::Value>>) -> WsMessage {
        WsMessage::Subscribe { topics, filters }
    }

    /// Crée un message de désouscription
    pub fn unsubscribe(topics: Vec<String>) -> WsMessage {
        WsMessage::Unsubscribe { topics }
    }

    /// Crée une confirmation de souscription
    pub fn subscription_confirmed(topics: Vec<String>, subscription_id: String) -> WsMessage {
        WsMessage::SubscriptionConfirmed { topics, subscription_id }
    }

    /// Crée une erreur de souscription
    pub fn subscription_error(topic: String, error: String) -> WsMessage {
        WsMessage::SubscriptionError { topic, error }
    }

    /// Crée un message de nouveau bloc
    pub fn new_block(block: BlockUpdate) -> WsMessage {
        WsMessage::NewBlock {
            block,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de nouvelle archive
    pub fn new_archive(archive: ArchiveUpdate) -> WsMessage {
        WsMessage::NewArchive {
            archive,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de mise à jour d'archive
    pub fn archive_update(
        archive_id: String,
        status: String,
        progress: Option<f64>,
        data: Option<HashMap<String, serde_json::Value>>,
    ) -> WsMessage {
        WsMessage::ArchiveUpdate {
            archive_id,
            status,
            progress,
            data,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de statistiques réseau
    pub fn network_stats(data: NetworkStatsUpdate) -> WsMessage {
        WsMessage::NetworkStats {
            data,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de ping
    pub fn ping() -> WsMessage {
        WsMessage::Ping {
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de pong
    pub fn pong() -> WsMessage {
        WsMessage::Pong {
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message d'erreur
    pub fn error(code: String, message: String) -> WsMessage {
        WsMessage::Error {
            code,
            message,
            timestamp: chrono::Utc::now(),
        }
    }

    /// Crée un message de succès
    pub fn success(message: String) -> WsMessage {
        WsMessage::Success {
            message,
            timestamp: chrono::Utc::now(),
        }
    }
}

/// Validation des messages WebSocket
pub struct MessageValidator;

impl MessageValidator {
    /// Valide un message WebSocket
    pub fn validate(message: &WsMessage) -> Result<(), String> {
        match message {
            WsMessage::Auth { token } => {
                if token.trim().is_empty() {
                    return Err("Token cannot be empty".to_string());
                }
                if !token.starts_with("eyJ") { // JWT basique check
                    return Err("Invalid token format".to_string());
                }
            }
            WsMessage::Subscribe { topics, .. } => {
                if topics.is_empty() {
                    return Err("At least one topic is required".to_string());
                }
                for topic in topics {
                    if SubscriptionTopic::from_str(topic).is_none() {
                        return Err(format!("Invalid topic: {}", topic));
                    }
                }
            }
            WsMessage::Unsubscribe { topics } => {
                if topics.is_empty() {
                    return Err("At least one topic is required".to_string());
                }
            }
            _ => {} // Autres messages sont valides par construction
        }
        Ok(())
    }

    /// Valide qu'un topic est accessible avec les permissions données
    pub fn validate_topic_access(topic: &str, scopes: &[String]) -> Result<(), String> {
        let topic_enum = SubscriptionTopic::from_str(topic)
            .ok_or_else(|| format!("Invalid topic: {}", topic))?;

        if let Some(required_scope) = topic_enum.required_scope() {
            if !scopes.contains(&required_scope.to_string()) && !scopes.contains(&"admin:all".to_string()) {
                return Err(format!("Insufficient permissions for topic: {}", topic));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscription_topic_conversion() {
        assert_eq!(SubscriptionTopic::ArchiveUpdates.as_str(), "archive_updates");
        assert_eq!(SubscriptionTopic::from_str("archive_updates"), Some(SubscriptionTopic::ArchiveUpdates));
        assert_eq!(SubscriptionTopic::from_str("invalid"), None);
    }

    #[test]
    fn test_topic_permissions() {
        assert!(SubscriptionTopic::NetworkStats.requires_auth() == false);
        assert!(SubscriptionTopic::ArchiveUpdates.requires_auth());
        assert_eq!(SubscriptionTopic::ArchiveUpdates.required_scope(), Some("archives:read"));
        assert_eq!(SubscriptionTopic::All.required_scope(), Some("admin:all"));
    }

    #[test]
    fn test_message_builder_auth() {
        let msg = MessageBuilder::auth("test_token".to_string());
        match msg {
            WsMessage::Auth { token } => assert_eq!(token, "test_token"),
            _ => panic!("Expected Auth message"),
        }
    }

    #[test]
    fn test_message_builder_auth_success() {
        let msg = MessageBuilder::auth_success("user123".to_string(), vec!["archives:read".to_string()]);
        match msg {
            WsMessage::AuthResponse { success, user_id, scopes, .. } => {
                assert!(success);
                assert_eq!(user_id, Some("user123".to_string()));
                assert_eq!(scopes, vec!["archives:read".to_string()]);
            }
            _ => panic!("Expected AuthResponse message"),
        }
    }

    #[test]
    fn test_message_builder_subscribe() {
        let topics = vec!["archive_updates".to_string()];
        let msg = MessageBuilder::subscribe(topics.clone(), None);
        match msg {
            WsMessage::Subscribe { topics: msg_topics, filters } => {
                assert_eq!(msg_topics, topics);
                assert!(filters.is_none());
            }
            _ => panic!("Expected Subscribe message"),
        }
    }

    #[test]
    fn test_message_validation_auth() {
        let valid_msg = WsMessage::Auth { token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string() };
        assert!(MessageValidator::validate(&valid_msg).is_ok());

        let invalid_msg = WsMessage::Auth { token: "".to_string() };
        assert!(MessageValidator::validate(&invalid_msg).is_err());

        let invalid_token = WsMessage::Auth { token: "invalid_token".to_string() };
        assert!(MessageValidator::validate(&invalid_token).is_err());
    }

    #[test]
    fn test_message_validation_subscribe() {
        let valid_msg = WsMessage::Subscribe {
            topics: vec!["archive_updates".to_string()],
            filters: None,
        };
        assert!(MessageValidator::validate(&valid_msg).is_ok());

        let empty_topics = WsMessage::Subscribe {
            topics: vec![],
            filters: None,
        };
        assert!(MessageValidator::validate(&empty_topics).is_err());

        let invalid_topic = WsMessage::Subscribe {
            topics: vec!["invalid_topic".to_string()],
            filters: None,
        };
        assert!(MessageValidator::validate(&invalid_topic).is_err());
    }

    #[test]
    fn test_topic_access_validation() {
        let scopes = vec!["archives:read".to_string()];
        
        assert!(MessageValidator::validate_topic_access("archive_updates", &scopes).is_ok());
        assert!(MessageValidator::validate_topic_access("new_blocks", &scopes).is_err());
        
        let admin_scopes = vec!["admin:all".to_string()];
        assert!(MessageValidator::validate_topic_access("all", &admin_scopes).is_ok());
        assert!(MessageValidator::validate_topic_access("archive_updates", &admin_scopes).is_ok());
    }

    #[test]
    fn test_all_topics() {
        let topics = SubscriptionTopic::all_topics();
        assert!(topics.len() > 0);
        assert!(topics.contains(&SubscriptionTopic::ArchiveUpdates));
        assert!(topics.contains(&SubscriptionTopic::NetworkStats));
    }

    #[test]
    fn test_block_update_creation() {
        let block = BlockUpdate {
            height: 12345,
            hash: "0x123".to_string(),
            timestamp: chrono::Utc::now(),
            transactions: 10,
            archives: 5,
            validator: "validator1".to_string(),
            size: 1024,
        };
        
        assert_eq!(block.height, 12345);
        assert_eq!(block.transactions, 10);
        assert_eq!(block.archives, 5);
    }

    #[test]
    fn test_archive_update_creation() {
        let archive = ArchiveUpdate {
            archive_id: "arc_123".to_string(),
            url: "https://example.com".to_string(),
            status: "completed".to_string(),
            size: Some(1024),
            replicas: Some(3),
            integrity_score: Some(0.99),
            metadata: None,
        };
        
        assert_eq!(archive.archive_id, "arc_123");
        assert_eq!(archive.size, Some(1024));
        assert_eq!(archive.replicas, Some(3));
    }

    #[test]
    fn test_message_serialization() {
        let msg = MessageBuilder::ping();
        let serialized = serde_json::to_string(&msg).unwrap();
        let deserialized: WsMessage = serde_json::from_str(&serialized).unwrap();
        
        match (msg, deserialized) {
            (WsMessage::Ping { .. }, WsMessage::Ping { .. }) => (),
            _ => panic!("Serialization/deserialization failed"),
        }
    }
}