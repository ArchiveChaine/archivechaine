//! Gestionnaire d'événements WebSocket pour ArchiveChain
//!
//! Gère la diffusion d'événements en temps réel aux clients WebSocket connectés.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, interval};

use crate::api::types::*;
use super::{
    connection::ConnectionManager,
    messages::*,
};

/// Gestionnaire d'événements WebSocket
#[derive(Clone)]
pub struct EventManager {
    /// Gestionnaire de connexions
    connection_manager: Arc<RwLock<ConnectionManager>>,
    /// Cache des derniers événements par type
    event_cache: Arc<RwLock<HashMap<String, CachedEvent>>>,
}

/// Événement mis en cache
#[derive(Debug, Clone)]
struct CachedEvent {
    message: WsMessage,
    timestamp: chrono::DateTime<chrono::Utc>,
    topic: String,
}

impl EventManager {
    /// Crée un nouveau gestionnaire d'événements
    pub fn new(connection_manager: Arc<RwLock<ConnectionManager>>) -> Self {
        let event_manager = Self {
            connection_manager,
            event_cache: Arc::new(RwLock::new(HashMap::new())),
        };

        // Démarre la tâche de nettoyage du cache
        event_manager.start_cache_cleanup_task();

        event_manager
    }

    /// Démarre la tâche de nettoyage du cache
    fn start_cache_cleanup_task(&self) {
        let cache = self.event_cache.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(300)); // 5 minutes
            
            loop {
                interval.tick().await;
                
                let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
                let mut cache_guard = cache.write().await;
                
                cache_guard.retain(|_, event| event.timestamp > cutoff);
            }
        });
    }

    /// Diffuse un événement de nouvelle archive
    pub async fn broadcast_new_archive(&self, archive: ArchiveDto) -> Result<usize, String> {
        let archive_update = ArchiveUpdate {
            archive_id: archive.archive_id.clone(),
            url: archive.url,
            status: format!("{:?}", archive.status),
            size: Some(archive.size),
            replicas: Some(archive.storage_info.replicas),
            integrity_score: Some(archive.storage_info.integrity_score),
            metadata: Some(ArchiveMetadataUpdate {
                title: archive.metadata.title,
                content_type: Some(archive.metadata.mime_type),
                language: archive.metadata.language,
                tags: Some(archive.metadata.tags),
            }),
        };

        let message = MessageBuilder::new_archive(archive_update);
        self.broadcast_to_topic("new_archives", message).await
    }

    /// Diffuse un événement de mise à jour d'archive
    pub async fn broadcast_archive_update(
        &self,
        archive_id: String,
        status: ArchiveStatus,
        progress: Option<f64>,
        additional_data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<usize, String> {
        let message = MessageBuilder::archive_update(
            archive_id,
            format!("{:?}", status),
            progress,
            additional_data,
        );

        self.broadcast_to_topic("archive_updates", message).await
    }

    /// Diffuse un événement de nouveau bloc
    pub async fn broadcast_new_block(&self, block: BlockDto) -> Result<usize, String> {
        let block_update = BlockUpdate {
            height: block.height,
            hash: block.hash,
            timestamp: block.timestamp,
            transactions: block.transactions.len() as u32,
            archives: block.archive_count,
            validator: block.validator,
            size: 0, // TODO: Calculer la vraie taille du bloc
        };

        let message = MessageBuilder::new_block(block_update);
        self.broadcast_to_topic("new_blocks", message).await
    }

    /// Diffuse des statistiques réseau mises à jour
    pub async fn broadcast_network_stats(&self, stats: NetworkStats) -> Result<usize, String> {
        let stats_update = NetworkStatsUpdate {
            active_nodes: stats.network.active_nodes as u32,
            total_storage: stats.network.total_storage,
            current_block_height: stats.network.current_block_height,
            network_latency: Some(stats.performance.network_latency),
            total_archives: Some(stats.archives.total_archives),
            archives_today: Some(stats.archives.archives_today as u32),
        };

        let message = MessageBuilder::network_stats(stats_update);
        self.broadcast_to_topic("network_stats", message).await
    }

    /// Diffuse un changement de statut de nœud
    pub async fn broadcast_node_status_change(
        &self,
        node_id: String,
        status: NodeStatus,
        region: Option<String>,
    ) -> Result<usize, String> {
        let message = WsMessage::NodeStatusChange {
            node_id,
            status: format!("{:?}", status),
            region,
            timestamp: chrono::Utc::now(),
        };

        self.broadcast_to_topic("node_status_change", message).await
    }

    /// Diffuse une mise à jour de bounty
    pub async fn broadcast_bounty_update(
        &self,
        bounty_id: String,
        status: String,
        data: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<usize, String> {
        let message = WsMessage::BountyUpdate {
            bounty_id,
            status,
            data,
            timestamp: chrono::Utc::now(),
        };

        self.broadcast_to_topic("bounty_updates", message).await
    }

    /// Diffuse un événement de contrat intelligent
    pub async fn broadcast_contract_event(
        &self,
        contract_id: String,
        event_type: String,
        data: serde_json::Value,
        block_height: u64,
        transaction_hash: String,
    ) -> Result<usize, String> {
        let message = WsMessage::ContractEvent {
            contract_id,
            event_type,
            data,
            block_height,
            transaction_hash,
            timestamp: chrono::Utc::now(),
        };

        self.broadcast_to_topic("contract_events", message).await
    }

    /// Diffuse un message à tous les abonnés d'un topic
    async fn broadcast_to_topic(&self, topic: &str, message: WsMessage) -> Result<usize, String> {
        // Met en cache l'événement
        {
            let mut cache = self.event_cache.write().await;
            let cache_key = format!("{}_{}", topic, chrono::Utc::now().timestamp_millis());
            cache.insert(cache_key, CachedEvent {
                message: message.clone(),
                timestamp: chrono::Utc::now(),
                topic: topic.to_string(),
            });
        }

        // Diffuse le message
        let mut manager = self.connection_manager.write().await;
        manager.broadcast_to_topic(topic, message).await
            .map_err(|e| format!("Failed to broadcast to topic {}: {}", topic, e))
    }

    /// Récupère les événements récents pour un topic
    pub async fn get_recent_events(
        &self,
        topic: &str,
        limit: usize,
    ) -> Vec<WsMessage> {
        let cache = self.event_cache.read().await;
        let mut events: Vec<_> = cache.values()
            .filter(|event| event.topic == topic)
            .collect();

        events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        events.into_iter()
            .take(limit)
            .map(|event| event.message.clone())
            .collect()
    }

    /// Récupère les statistiques d'événements
    pub async fn get_event_stats(&self) -> EventStats {
        let cache = self.event_cache.read().await;
        let manager = self.connection_manager.read().await;

        let mut events_by_topic = HashMap::new();
        for event in cache.values() {
            *events_by_topic.entry(event.topic.clone()).or_insert(0) += 1;
        }

        let connection_stats = manager.get_stats().await;

        EventStats {
            total_cached_events: cache.len(),
            events_by_topic,
            active_subscribers: connection_stats.subscriptions_by_topic,
            last_hour_events: cache.values()
                .filter(|event| {
                    let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
                    event.timestamp > cutoff
                })
                .count(),
        }
    }

    /// Envoie les événements récents à une nouvelle connexion
    pub async fn send_recent_events_to_connection(
        &self,
        connection_id: &str,
        topics: &[String],
        limit: usize,
    ) -> Result<(), String> {
        let mut manager = self.connection_manager.write().await;

        for topic in topics {
            let recent_events = self.get_recent_events(topic, limit).await;
            
            for event in recent_events {
                if let Err(e) = manager.send_to_connection(connection_id, event).await {
                    return Err(format!("Failed to send recent events: {}", e));
                }
            }
        }

        Ok(())
    }
}

/// Statistiques d'événements
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EventStats {
    pub total_cached_events: usize,
    pub events_by_topic: HashMap<String, usize>,
    pub active_subscribers: HashMap<String, usize>,
    pub last_hour_events: usize,
}

/// Helper pour créer des événements de test
#[cfg(test)]
pub struct EventTestHelper;

#[cfg(test)]
impl EventTestHelper {
    pub fn create_test_archive_dto() -> ArchiveDto {
        ArchiveDto {
            archive_id: "arc_test_123".to_string(),
            url: "https://example.com/test".to_string(),
            status: ArchiveStatus::Completed,
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            size: 1024,
            metadata: ArchiveMetadataDto {
                title: Some("Test Archive".to_string()),
                description: Some("A test archive".to_string()),
                mime_type: "text/html".to_string(),
                language: Some("en".to_string()),
                author: Some("Test Author".to_string()),
                published_at: Some(chrono::Utc::now()),
                tags: vec!["test".to_string(), "example".to_string()],
            },
            storage_info: StorageInfo {
                replicas: 3,
                locations: vec!["us-east".to_string(), "eu-west".to_string()],
                integrity_score: 0.99,
                last_verified: chrono::Utc::now(),
            },
            access_urls: AccessUrls::new("https://gateway.example.com", "arc_test_123"),
        }
    }

    pub fn create_test_block_dto() -> BlockDto {
        BlockDto {
            height: 12345,
            hash: "0x1234567890abcdef".to_string(),
            previous_hash: "0x0987654321fedcba".to_string(),
            timestamp: chrono::Utc::now(),
            transactions: vec![],
            archive_count: 5,
            validator: "validator_123".to_string(),
        }
    }

    pub fn create_test_network_stats() -> NetworkStats {
        NetworkStats {
            network: NetworkInfo {
                total_nodes: 100,
                active_nodes: 95,
                total_storage: "15.7 TB".to_string(),
                available_storage: "8.3 TB".to_string(),
                current_block_height: 12345,
            },
            archives: ArchiveStats {
                total_archives: 50000,
                archives_today: 150,
                total_size: "12.4 TB".to_string(),
                average_replication: 4.2,
            },
            performance: PerformanceStats {
                average_archive_time: "2.3 minutes".to_string(),
                network_latency: "45ms".to_string(),
                success_rate: 0.987,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::websocket::{WebSocketConfig, connection::ConnectionManager};
    use std::sync::Arc;
    use tokio::sync::{RwLock, mpsc};

    #[tokio::test]
    async fn test_event_manager_creation() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        let stats = event_manager.get_event_stats().await;
        assert_eq!(stats.total_cached_events, 0);
    }

    #[tokio::test]
    async fn test_broadcast_new_archive() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager.clone());

        // Ajoute une connexion mockée
        {
            let mut manager = connection_manager.write().await;
            let (tx, _) = mpsc::unbounded_channel();
            manager.add_connection("test_conn".to_string(), tx, None, None).await.unwrap();
            manager.subscribe_to_topic("test_conn", "new_archives").await.unwrap();
        }

        let archive = EventTestHelper::create_test_archive_dto();
        let result = event_manager.broadcast_new_archive(archive).await;

        // Devrait échouer car la connexion n'est pas authentifiée
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_broadcast_archive_update() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        let result = event_manager.broadcast_archive_update(
            "arc_123".to_string(),
            ArchiveStatus::Completed,
            Some(100.0),
            None,
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // Aucun abonné
    }

    #[tokio::test]
    async fn test_broadcast_new_block() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        let block = EventTestHelper::create_test_block_dto();
        let result = event_manager.broadcast_new_block(block).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // Aucun abonné
    }

    #[tokio::test]
    async fn test_broadcast_network_stats() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        let stats = EventTestHelper::create_test_network_stats();
        let result = event_manager.broadcast_network_stats(stats).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // Aucun abonné
    }

    #[tokio::test]
    async fn test_broadcast_node_status_change() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        let result = event_manager.broadcast_node_status_change(
            "node_123".to_string(),
            NodeStatus::Active,
            Some("us-east".to_string()),
        ).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 0); // Aucun abonné
    }

    #[tokio::test]
    async fn test_get_recent_events() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        // Diffuse quelques événements
        let _ = event_manager.broadcast_archive_update(
            "arc_1".to_string(),
            ArchiveStatus::Completed,
            Some(100.0),
            None,
        ).await;

        let _ = event_manager.broadcast_archive_update(
            "arc_2".to_string(),
            ArchiveStatus::Processing,
            Some(50.0),
            None,
        ).await;

        // Récupère les événements récents
        let events = event_manager.get_recent_events("archive_updates", 10).await;
        assert_eq!(events.len(), 2);
    }

    #[tokio::test]
    async fn test_event_stats() {
        let config = WebSocketConfig::default();
        let connection_manager = Arc::new(RwLock::new(ConnectionManager::new(config)));
        let event_manager = EventManager::new(connection_manager);

        // Diffuse quelques événements
        let _ = event_manager.broadcast_archive_update(
            "arc_1".to_string(),
            ArchiveStatus::Completed,
            None,
            None,
        ).await;

        let stats = event_manager.get_event_stats().await;
        assert_eq!(stats.total_cached_events, 1);
        assert_eq!(stats.events_by_topic.get("archive_updates"), Some(&1));
    }

    #[test]
    fn test_event_test_helper() {
        let archive = EventTestHelper::create_test_archive_dto();
        assert_eq!(archive.archive_id, "arc_test_123");
        assert_eq!(archive.status, ArchiveStatus::Completed);

        let block = EventTestHelper::create_test_block_dto();
        assert_eq!(block.height, 12345);
        assert_eq!(block.archive_count, 5);

        let stats = EventTestHelper::create_test_network_stats();
        assert_eq!(stats.network.total_nodes, 100);
        assert_eq!(stats.archives.total_archives, 50000);
    }
}