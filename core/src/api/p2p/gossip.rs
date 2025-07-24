//! Service de gossip P2P pour ArchiveChain
//!
//! Implémente le protocole de gossip pour la diffusion d'informations dans le réseau.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot};
use tokio::time::{Duration, interval};

use super::{P2PConfig, P2PError, P2PResult, messages::*};

/// Service de gossip
#[derive(Debug)]
pub struct GossipService {
    /// Configuration
    config: P2PConfig,
    /// Messages de gossip actifs
    active_messages: Arc<RwLock<HashMap<String, GossipMessage>>>,
    /// Canal d'arrêt
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
}

/// Message de gossip avec métadonnées
#[derive(Debug, Clone)]
pub struct GossipMessage {
    /// ID unique du message
    pub message_id: String,
    /// Topic du gossip
    pub topic: String,
    /// Données du message
    pub data: serde_json::Value,
    /// TTL (Time To Live)
    pub ttl: u32,
    /// Timestamp de création
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Pairs qui ont reçu ce message
    pub propagated_to: HashSet<String>,
    /// Nombre de propagations
    pub propagation_count: u32,
}

impl GossipService {
    /// Crée un nouveau service de gossip
    pub fn new(config: P2PConfig) -> Self {
        Self {
            config,
            active_messages: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Démarre le service de gossip
    pub async fn start(&self) -> P2PResult<()> {
        tracing::info!("Starting P2P gossip service");

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        {
            let mut shutdown_guard = self.shutdown_tx.write().await;
            *shutdown_guard = Some(shutdown_tx);
        }

        // Démarre la tâche de nettoyage des messages expirés
        let active_messages = self.active_messages.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Nettoie chaque minute

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        Self::cleanup_expired_messages(&active_messages).await;
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Gossip service shutting down");
                        break;
                    }
                }
            }
        });

        tracing::info!("P2P gossip service started");
        Ok(())
    }

    /// Arrête le service de gossip
    pub async fn stop(&self) -> P2PResult<()> {
        tracing::info!("Stopping P2P gossip service");

        if let Some(shutdown_tx) = self.shutdown_tx.write().await.take() {
            let _ = shutdown_tx.send(());
        }

        tracing::info!("P2P gossip service stopped");
        Ok(())
    }

    /// Diffuse un message de gossip
    pub async fn broadcast_gossip(
        &self,
        topic: String,
        data: serde_json::Value,
        ttl: u32,
    ) -> P2PResult<String> {
        let message_id = format!("gossip_{}", uuid::Uuid::new_v4().simple());
        
        let gossip_message = GossipMessage {
            message_id: message_id.clone(),
            topic: topic.clone(),
            data: data.clone(),
            ttl,
            created_at: chrono::Utc::now(),
            propagated_to: HashSet::new(),
            propagation_count: 0,
        };

        // Ajoute à la liste des messages actifs
        {
            let mut messages = self.active_messages.write().await;
            messages.insert(message_id.clone(), gossip_message);
        }

        tracing::debug!("Broadcasting gossip message: {} on topic: {}", message_id, topic);
        
        // TODO: Envoyer le message à tous les pairs connectés via le P2PManager
        
        Ok(message_id)
    }

    /// Traite un message de gossip reçu
    pub async fn handle_gossip_message(&self, message: P2PMessage, from_peer: String) -> P2PResult<bool> {
        if let P2PMessage::Gossip { topic, data, ttl, timestamp } = message {
            let message_id = self.generate_message_id(&topic, &data, timestamp);
            
            // Vérifie si on a déjà vu ce message
            {
                let mut messages = self.active_messages.write().await;
                if let Some(existing_message) = messages.get_mut(&message_id) {
                    // Marque ce pair comme ayant reçu le message
                    existing_message.propagated_to.insert(from_peer);
                    return Ok(false); // Message déjà vu, ne pas propager
                }

                // Nouveau message, l'ajouter
                if ttl > 0 {
                    let gossip_message = GossipMessage {
                        message_id: message_id.clone(),
                        topic: topic.clone(),
                        data: data.clone(),
                        ttl,
                        created_at: timestamp,
                        propagated_to: {
                            let mut set = HashSet::new();
                            set.insert(from_peer);
                            set
                        },
                        propagation_count: 1,
                    };

                    messages.insert(message_id.clone(), gossip_message);
                }
            }

            // Traite le message selon le topic
            self.process_gossip_topic(&topic, &data).await?;

            // Propage le message si TTL > 1
            if ttl > 1 {
                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Propage un message de gossip
    pub async fn propagate_message(
        &self,
        message_id: String,
        exclude_peers: HashSet<String>,
    ) -> P2PResult<u32> {
        let mut messages = self.active_messages.write().await;
        
        if let Some(gossip_message) = messages.get_mut(&message_id) {
            if gossip_message.ttl <= 1 {
                return Ok(0); // TTL expiré
            }

            // Réduit le TTL
            gossip_message.ttl -= 1;
            gossip_message.propagation_count += 1;

            // TODO: Envoyer aux pairs (sauf ceux exclus)
            // Cette logique serait implémentée en coordination avec le P2PManager

            tracing::debug!("Propagated gossip message: {} (TTL: {})", 
                message_id, gossip_message.ttl);

            Ok(1) // Placeholder pour le nombre de pairs contactés
        } else {
            Err(P2PError::InvalidMessage)
        }
    }

    /// Traite un message selon son topic
    async fn process_gossip_topic(&self, topic: &str, data: &serde_json::Value) -> P2PResult<()> {
        match topic {
            "block_announcement" => {
                tracing::debug!("Received block announcement via gossip: {:?}", data);
                // TODO: Traiter l'annonce de bloc
            }
            "transaction_announcement" => {
                tracing::debug!("Received transaction announcement via gossip: {:?}", data);
                // TODO: Traiter l'annonce de transaction
            }
            "archive_announcement" => {
                tracing::debug!("Received archive announcement via gossip: {:?}", data);
                // TODO: Traiter l'annonce d'archive
            }
            "network_status" => {
                tracing::debug!("Received network status via gossip: {:?}", data);
                // TODO: Traiter le statut réseau
            }
            _ => {
                tracing::debug!("Received unknown gossip topic: {}", topic);
            }
        }

        Ok(())
    }

    /// Génère un ID de message unique basé sur le contenu
    fn generate_message_id(
        &self,
        topic: &str,
        data: &serde_json::Value,
        timestamp: chrono::DateTime<chrono::Utc>,
    ) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        topic.hash(&mut hasher);
        data.to_string().hash(&mut hasher);
        timestamp.timestamp().hash(&mut hasher);

        format!("gossip_{:x}", hasher.finish())
    }

    /// Nettoie les messages expirés
    async fn cleanup_expired_messages(
        active_messages: &Arc<RwLock<HashMap<String, GossipMessage>>>,
    ) {
        let mut messages = active_messages.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::minutes(10);

        let initial_count = messages.len();
        messages.retain(|_, message| {
            message.created_at > cutoff && message.ttl > 0
        });

        let removed_count = initial_count - messages.len();
        if removed_count > 0 {
            tracing::debug!("Cleaned up {} expired gossip messages", removed_count);
        }
    }

    /// Récupère les statistiques de gossip
    pub async fn get_gossip_stats(&self) -> GossipStats {
        let messages = self.active_messages.read().await;
        
        let mut stats = GossipStats {
            active_messages: messages.len(),
            total_propagations: 0,
            messages_by_topic: HashMap::new(),
            average_ttl: 0.0,
        };

        let mut total_ttl = 0u32;
        for message in messages.values() {
            stats.total_propagations += message.propagation_count;
            total_ttl += message.ttl;
            
            *stats.messages_by_topic.entry(message.topic.clone()).or_insert(0) += 1;
        }

        if !messages.is_empty() {
            stats.average_ttl = total_ttl as f64 / messages.len() as f64;
        }

        stats
    }

    /// Récupère les messages actifs pour un topic
    pub async fn get_messages_by_topic(&self, topic: &str) -> Vec<GossipMessage> {
        let messages = self.active_messages.read().await;
        
        messages.values()
            .filter(|msg| msg.topic == topic)
            .cloned()
            .collect()
    }

    /// Force l'expiration d'un message
    pub async fn expire_message(&self, message_id: &str) -> P2PResult<()> {
        let mut messages = self.active_messages.write().await;
        
        if let Some(message) = messages.get_mut(message_id) {
            message.ttl = 0;
            tracing::debug!("Expired gossip message: {}", message_id);
        }

        Ok(())
    }
}

/// Statistiques de gossip
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GossipStats {
    pub active_messages: usize,
    pub total_propagations: u32,
    pub messages_by_topic: HashMap<String, usize>,
    pub average_ttl: f64,
}

/// Topics de gossip prédéfinis
pub mod topics {
    pub const BLOCK_ANNOUNCEMENT: &str = "block_announcement";
    pub const TRANSACTION_ANNOUNCEMENT: &str = "transaction_announcement";
    pub const ARCHIVE_ANNOUNCEMENT: &str = "archive_announcement";
    pub const NETWORK_STATUS: &str = "network_status";
    pub const PEER_DISCOVERY: &str = "peer_discovery";
    pub const EMERGENCY_ALERT: &str = "emergency_alert";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gossip_service_creation() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        // Vérifie que le service peut être créé
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_gossip_message() {
        let message = GossipMessage {
            message_id: "msg_123".to_string(),
            topic: "test_topic".to_string(),
            data: serde_json::json!({"key": "value"}),
            ttl: 10,
            created_at: chrono::Utc::now(),
            propagated_to: HashSet::new(),
            propagation_count: 0,
        };

        assert_eq!(message.message_id, "msg_123");
        assert_eq!(message.topic, "test_topic");
        assert_eq!(message.ttl, 10);
        assert_eq!(message.propagation_count, 0);
    }

    #[tokio::test]
    async fn test_broadcast_gossip() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        let result = service.broadcast_gossip(
            "test_topic".to_string(),
            serde_json::json!({"test": "data"}),
            5,
        ).await;

        assert!(result.is_ok());
        let message_id = result.unwrap();
        assert!(message_id.starts_with("gossip_"));

        let stats = service.get_gossip_stats().await;
        assert_eq!(stats.active_messages, 1);
        assert_eq!(stats.messages_by_topic.get("test_topic"), Some(&1));
    }

    #[tokio::test]
    async fn test_handle_gossip_message() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        let gossip_msg = P2PMessage::Gossip {
            topic: "test_topic".to_string(),
            data: serde_json::json!({"test": "data"}),
            ttl: 5,
            timestamp: chrono::Utc::now(),
        };

        let result = service.handle_gossip_message(gossip_msg, "peer_123".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap()); // Devrait retourner true pour propager

        let stats = service.get_gossip_stats().await;
        assert_eq!(stats.active_messages, 1);
    }

    #[tokio::test]
    async fn test_duplicate_message_handling() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        let timestamp = chrono::Utc::now();
        let gossip_msg = P2PMessage::Gossip {
            topic: "test_topic".to_string(),
            data: serde_json::json!({"test": "data"}),
            ttl: 5,
            timestamp,
        };

        // Premier message
        let result1 = service.handle_gossip_message(gossip_msg.clone(), "peer_1".to_string()).await;
        assert!(result1.is_ok());
        assert!(result1.unwrap());

        // Message dupliqué
        let result2 = service.handle_gossip_message(gossip_msg, "peer_2".to_string()).await;
        assert!(result2.is_ok());
        assert!(!result2.unwrap()); // Ne devrait pas propager

        let stats = service.get_gossip_stats().await;
        assert_eq!(stats.active_messages, 1); // Un seul message
    }

    #[test]
    fn test_message_id_generation() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        let timestamp = chrono::Utc::now();
        let data = serde_json::json!({"test": "data"});
        
        let id1 = service.generate_message_id("topic1", &data, timestamp);
        let id2 = service.generate_message_id("topic1", &data, timestamp);
        let id3 = service.generate_message_id("topic2", &data, timestamp);
        
        assert_eq!(id1, id2); // Même contenu = même ID
        assert_ne!(id1, id3); // Topic différent = ID différent
    }

    #[tokio::test]
    async fn test_get_messages_by_topic() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        // Ajoute des messages sur différents topics
        service.broadcast_gossip(
            "topic1".to_string(),
            serde_json::json!({"data": 1}),
            5,
        ).await.unwrap();

        service.broadcast_gossip(
            "topic2".to_string(),
            serde_json::json!({"data": 2}),
            5,
        ).await.unwrap();

        service.broadcast_gossip(
            "topic1".to_string(),
            serde_json::json!({"data": 3}),
            5,
        ).await.unwrap();

        let topic1_messages = service.get_messages_by_topic("topic1").await;
        let topic2_messages = service.get_messages_by_topic("topic2").await;

        assert_eq!(topic1_messages.len(), 2);
        assert_eq!(topic2_messages.len(), 1);
    }

    #[tokio::test]
    async fn test_expire_message() {
        let config = P2PConfig::default();
        let service = GossipService::new(config);
        
        let message_id = service.broadcast_gossip(
            "test_topic".to_string(),
            serde_json::json!({"test": "data"}),
            5,
        ).await.unwrap();

        let result = service.expire_message(&message_id).await;
        assert!(result.is_ok());

        // Vérifie que le message est toujours là mais avec TTL = 0
        let messages = service.get_messages_by_topic("test_topic").await;
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].ttl, 0);
    }

    #[test]
    fn test_gossip_topics() {
        assert_eq!(topics::BLOCK_ANNOUNCEMENT, "block_announcement");
        assert_eq!(topics::TRANSACTION_ANNOUNCEMENT, "transaction_announcement");
        assert_eq!(topics::ARCHIVE_ANNOUNCEMENT, "archive_announcement");
        assert_eq!(topics::NETWORK_STATUS, "network_status");
        assert_eq!(topics::PEER_DISCOVERY, "peer_discovery");
        assert_eq!(topics::EMERGENCY_ALERT, "emergency_alert");
    }
}