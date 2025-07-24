//! Subscriptions GraphQL pour ArchiveChain
//!
//! Gère les subscriptions en temps réel via WebSocket pour les mises à jour
//! d'archives, statistiques réseau et événements de la blockchain.

use async_graphql::{Result as GraphQLResult, Error as GraphQLError};
use futures_util::{Stream, StreamExt};
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{broadcast, RwLock};
use tokio_stream::wrappers::BroadcastStream;

use super::schema::*;

/// Gestionnaire de subscriptions
#[derive(Clone)]
pub struct SubscriptionManager {
    /// Canal pour les mises à jour d'archives
    archive_updates: broadcast::Sender<ArchiveUpdateEvent>,
    /// Canal pour les nouvelles archives
    new_archives: broadcast::Sender<Archive>,
    /// Canal pour les statistiques réseau
    network_stats: broadcast::Sender<NetworkStats>,
    /// Canal pour les nouveaux blocs
    new_blocks: broadcast::Sender<Block>,
    /// Canal pour les événements de contrats
    contract_events: broadcast::Sender<ContractEvent>,
    /// Abonnements actifs par utilisateur
    active_subscriptions: Arc<RwLock<HashMap<String, Vec<SubscriptionInfo>>>>,
}

/// Information sur une subscription active
#[derive(Debug, Clone)]
pub struct SubscriptionInfo {
    pub subscription_id: String,
    pub subscription_type: SubscriptionType,
    pub user_id: String,
    pub filter: Option<serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Types de subscriptions disponibles
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SubscriptionType {
    ArchiveUpdates,
    NewArchives,
    NetworkStats,
    NewBlocks,
    ContractEvents,
}

/// Événement de mise à jour d'archive
#[derive(Debug, Clone)]
pub struct ArchiveUpdateEvent {
    pub archive_id: String,
    pub status: ArchiveStatus,
    pub progress: Option<f64>,
    pub message: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Événement de contrat intelligent
#[derive(Debug, Clone)]
pub struct ContractEvent {
    pub contract_id: String,
    pub event_type: String,
    pub data: serde_json::Value,
    pub block_height: u64,
    pub transaction_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SubscriptionManager {
    /// Crée un nouveau gestionnaire de subscriptions
    pub fn new() -> Self {
        // Crée les canaux avec une capacité appropriée
        let (archive_updates, _) = broadcast::channel(1000);
        let (new_archives, _) = broadcast::channel(1000);
        let (network_stats, _) = broadcast::channel(100);
        let (new_blocks, _) = broadcast::channel(1000);
        let (contract_events, _) = broadcast::channel(1000);

        Self {
            archive_updates,
            new_archives,
            network_stats,
            new_blocks,
            contract_events,
            active_subscriptions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Démarre les tâches de background pour les subscriptions
    pub async fn start_background_tasks(&self) {
        // Démarre la tâche de statistiques réseau périodiques
        let stats_sender = self.network_stats.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                
                // TODO: Récupérer les vraies statistiques
                let stats = NetworkStats {
                    total_nodes: 100,
                    active_nodes: 95,
                    total_storage: "15.7 TB".to_string(),
                    available_storage: "8.3 TB".to_string(),
                    current_block_height: 12345,
                    total_archives: 50000,
                    archives_today: 150,
                    average_archive_time: "2.3 minutes".to_string(),
                    success_rate: 0.987,
                };

                if let Err(_) = stats_sender.send(stats) {
                    tracing::warn!("No subscribers for network stats");
                }
            }
        });

        // Démarre la tâche de nettoyage des subscriptions expirées
        let subscriptions = self.active_subscriptions.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // 5 minutes
            loop {
                interval.tick().await;
                Self::cleanup_expired_subscriptions(&subscriptions).await;
            }
        });
    }

    /// Nettoyage des subscriptions expirées
    async fn cleanup_expired_subscriptions(
        subscriptions: &Arc<RwLock<HashMap<String, Vec<SubscriptionInfo>>>>
    ) {
        let mut subs = subscriptions.write().await;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);
        
        subs.retain(|_, user_subs| {
            user_subs.retain(|sub| sub.created_at > cutoff);
            !user_subs.is_empty()
        });
    }

    /// Enregistre une nouvelle subscription
    pub async fn register_subscription(&self, info: SubscriptionInfo) {
        let mut subs = self.active_subscriptions.write().await;
        subs.entry(info.user_id.clone())
            .or_insert_with(Vec::new)
            .push(info);
    }

    /// Désenregistre une subscription
    pub async fn unregister_subscription(&self, user_id: &str, subscription_id: &str) {
        let mut subs = self.active_subscriptions.write().await;
        if let Some(user_subs) = subs.get_mut(user_id) {
            user_subs.retain(|sub| sub.subscription_id != subscription_id);
            if user_subs.is_empty() {
                subs.remove(user_id);
            }
        }
    }

    /// Publie une mise à jour d'archive
    pub async fn publish_archive_update(&self, event: ArchiveUpdateEvent) -> Result<(), String> {
        self.archive_updates.send(event)
            .map_err(|e| format!("Failed to publish archive update: {}", e))?;
        Ok(())
    }

    /// Publie une nouvelle archive
    pub async fn publish_new_archive(&self, archive: Archive) -> Result<(), String> {
        self.new_archives.send(archive)
            .map_err(|e| format!("Failed to publish new archive: {}", e))?;
        Ok(())
    }

    /// Publie un nouveau bloc
    pub async fn publish_new_block(&self, block: Block) -> Result<(), String> {
        self.new_blocks.send(block)
            .map_err(|e| format!("Failed to publish new block: {}", e))?;
        Ok(())
    }

    /// Publie un événement de contrat
    pub async fn publish_contract_event(&self, event: ContractEvent) -> Result<(), String> {
        self.contract_events.send(event)
            .map_err(|e| format!("Failed to publish contract event: {}", e))?;
        Ok(())
    }

    /// Crée un stream pour les mises à jour d'archives
    pub fn archive_updates_stream(
        &self, 
        archive_id: Option<String>
    ) -> Pin<Box<dyn Stream<Item = Archive> + Send>> {
        let receiver = self.archive_updates.subscribe();
        let stream = BroadcastStream::new(receiver)
            .filter_map(move |result| {
                let archive_id = archive_id.clone();
                async move {
                    match result {
                        Ok(event) => {
                            // Filtre par archive_id si spécifié
                            if let Some(ref id) = archive_id {
                                if event.archive_id != *id {
                                    return None;
                                }
                            }

                            // Convertit l'événement en Archive
                            // TODO: Récupérer l'archive complète depuis la base de données
                            Some(Archive {
                                id: event.archive_id,
                                url: "https://example.com".to_string(), // Placeholder
                                status: event.status,
                                metadata: ArchiveMetadata {
                                    title: None,
                                    description: None,
                                    tags: vec![],
                                    content_type: "unknown".to_string(),
                                    language: None,
                                    author: None,
                                    published_at: None,
                                },
                                storage_info: StorageInfo {
                                    replicas: 0,
                                    locations: vec![],
                                    integrity_score: 0.0,
                                    last_verified: chrono::Utc::now(),
                                },
                                created_at: chrono::Utc::now(),
                                completed_at: None,
                                size: 0,
                                cost: TokenAmount {
                                    amount: "0".to_string(),
                                    currency: "ARC".to_string(),
                                },
                            })
                        }
                        Err(_) => None,
                    }
                }
            });

        Box::pin(stream)
    }

    /// Crée un stream pour les nouvelles archives
    pub fn new_archives_stream(&self) -> Pin<Box<dyn Stream<Item = Archive> + Send>> {
        let receiver = self.new_archives.subscribe();
        let stream = BroadcastStream::new(receiver)
            .filter_map(|result| async move {
                result.ok()
            });

        Box::pin(stream)
    }

    /// Crée un stream pour les statistiques réseau
    pub fn network_stats_stream(&self) -> Pin<Box<dyn Stream<Item = NetworkStats> + Send>> {
        let receiver = self.network_stats.subscribe();
        let stream = BroadcastStream::new(receiver)
            .filter_map(|result| async move {
                result.ok()
            });

        Box::pin(stream)
    }

    /// Crée un stream pour les nouveaux blocs
    pub fn new_blocks_stream(&self) -> Pin<Box<dyn Stream<Item = Block> + Send>> {
        let receiver = self.new_blocks.subscribe();
        let stream = BroadcastStream::new(receiver)
            .filter_map(|result| async move {
                result.ok()
            });

        Box::pin(stream)
    }

    /// Crée un stream pour les événements de contrats
    pub fn contract_events_stream(
        &self,
        contract_id: Option<String>
    ) -> Pin<Box<dyn Stream<Item = ContractEvent> + Send>> {
        let receiver = self.contract_events.subscribe();
        let stream = BroadcastStream::new(receiver)
            .filter_map(move |result| {
                let contract_id = contract_id.clone();
                async move {
                    match result {
                        Ok(event) => {
                            // Filtre par contract_id si spécifié
                            if let Some(ref id) = contract_id {
                                if event.contract_id != *id {
                                    return None;
                                }
                            }
                            Some(event)
                        }
                        Err(_) => None,
                    }
                }
            });

        Box::pin(stream)
    }

    /// Récupère les statistiques des subscriptions
    pub async fn get_subscription_stats(&self) -> SubscriptionStats {
        let subs = self.active_subscriptions.read().await;
        let total_users = subs.len();
        let total_subscriptions: usize = subs.values().map(|v| v.len()).sum();
        
        let mut by_type = HashMap::new();
        for user_subs in subs.values() {
            for sub in user_subs {
                *by_type.entry(sub.subscription_type.clone()).or_insert(0) += 1;
            }
        }

        SubscriptionStats {
            total_users,
            total_subscriptions,
            by_type,
            active_channels: 5, // archive_updates, new_archives, network_stats, new_blocks, contract_events
        }
    }
}

/// Statistiques des subscriptions
#[derive(Debug, Clone)]
pub struct SubscriptionStats {
    pub total_users: usize,
    pub total_subscriptions: usize,
    pub by_type: HashMap<SubscriptionType, usize>,
    pub active_channels: usize,
}

/// Instance globale du gestionnaire de subscriptions
static SUBSCRIPTION_MANAGER: tokio::sync::OnceCell<SubscriptionManager> = tokio::sync::OnceCell::const_new();

/// Récupère l'instance globale du gestionnaire
pub async fn get_subscription_manager() -> &'static SubscriptionManager {
    SUBSCRIPTION_MANAGER.get_or_init(|| async {
        let manager = SubscriptionManager::new();
        manager.start_background_tasks().await;
        manager
    }).await
}

/// Helpers pour les resolvers
pub struct SubscriptionHelpers;

impl SubscriptionHelpers {
    /// Crée un stream pour les mises à jour d'archive spécifique
    pub async fn archive_updates_for_id(archive_id: String) -> GraphQLResult<Pin<Box<dyn Stream<Item = Archive> + Send>>> {
        let manager = get_subscription_manager().await;
        
        // Enregistre la subscription
        let sub_info = SubscriptionInfo {
            subscription_id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::ArchiveUpdates,
            user_id: "anonymous".to_string(), // TODO: Récupérer l'ID utilisateur du contexte
            filter: Some(serde_json::json!({"archive_id": archive_id})),
            created_at: chrono::Utc::now(),
        };
        
        manager.register_subscription(sub_info).await;
        
        Ok(manager.archive_updates_stream(Some(archive_id)))
    }

    /// Crée un stream pour toutes les nouvelles archives
    pub async fn all_new_archives() -> GraphQLResult<Pin<Box<dyn Stream<Item = Archive> + Send>>> {
        let manager = get_subscription_manager().await;
        
        let sub_info = SubscriptionInfo {
            subscription_id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::NewArchives,
            user_id: "anonymous".to_string(),
            filter: None,
            created_at: chrono::Utc::now(),
        };
        
        manager.register_subscription(sub_info).await;
        
        Ok(manager.new_archives_stream())
    }

    /// Crée un stream pour les statistiques réseau
    pub async fn network_statistics() -> GraphQLResult<Pin<Box<dyn Stream<Item = NetworkStats> + Send>>> {
        let manager = get_subscription_manager().await;
        
        let sub_info = SubscriptionInfo {
            subscription_id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::NetworkStats,
            user_id: "anonymous".to_string(),
            filter: None,
            created_at: chrono::Utc::now(),
        };
        
        manager.register_subscription(sub_info).await;
        
        Ok(manager.network_stats_stream())
    }

    /// Crée un stream pour les nouveaux blocs
    pub async fn new_blockchain_blocks() -> GraphQLResult<Pin<Box<dyn Stream<Item = Block> + Send>>> {
        let manager = get_subscription_manager().await;
        
        let sub_info = SubscriptionInfo {
            subscription_id: uuid::Uuid::new_v4().to_string(),
            subscription_type: SubscriptionType::NewBlocks,
            user_id: "anonymous".to_string(),
            filter: None,
            created_at: chrono::Utc::now(),
        };
        
        manager.register_subscription(sub_info).await;
        
        Ok(manager.new_blocks_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_subscription_manager_creation() {
        let manager = SubscriptionManager::new();
        
        // Vérifie que les canaux sont créés
        assert_eq!(manager.archive_updates.receiver_count(), 0);
        assert_eq!(manager.new_archives.receiver_count(), 0);
        assert_eq!(manager.network_stats.receiver_count(), 0);
    }

    #[tokio::test]
    async fn test_subscription_registration() {
        let manager = SubscriptionManager::new();
        
        let sub_info = SubscriptionInfo {
            subscription_id: "test-123".to_string(),
            subscription_type: SubscriptionType::ArchiveUpdates,
            user_id: "user-456".to_string(),
            filter: None,
            created_at: chrono::Utc::now(),
        };
        
        manager.register_subscription(sub_info.clone()).await;
        
        let stats = manager.get_subscription_stats().await;
        assert_eq!(stats.total_users, 1);
        assert_eq!(stats.total_subscriptions, 1);
        assert_eq!(stats.by_type.get(&SubscriptionType::ArchiveUpdates), Some(&1));
    }

    #[tokio::test]
    async fn test_subscription_unregistration() {
        let manager = SubscriptionManager::new();
        
        let sub_info = SubscriptionInfo {
            subscription_id: "test-123".to_string(),
            subscription_type: SubscriptionType::ArchiveUpdates,
            user_id: "user-456".to_string(),
            filter: None,
            created_at: chrono::Utc::now(),
        };
        
        manager.register_subscription(sub_info.clone()).await;
        manager.unregister_subscription("user-456", "test-123").await;
        
        let stats = manager.get_subscription_stats().await;
        assert_eq!(stats.total_users, 0);
        assert_eq!(stats.total_subscriptions, 0);
    }

    #[tokio::test]
    async fn test_archive_update_publishing() {
        let manager = SubscriptionManager::new();
        
        let event = ArchiveUpdateEvent {
            archive_id: "arc_123".to_string(),
            status: ArchiveStatus::Completed,
            progress: Some(100.0),
            message: Some("Archive completed".to_string()),
            timestamp: chrono::Utc::now(),
        };
        
        // Test que la publication ne fait pas d'erreur même sans subscribers
        let result = manager.publish_archive_update(event).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_new_archive_publishing() {
        let manager = SubscriptionManager::new();
        
        let archive = Archive {
            id: "arc_123".to_string(),
            url: "https://example.com".to_string(),
            status: ArchiveStatus::Completed,
            metadata: ArchiveMetadata {
                title: Some("Test Archive".to_string()),
                description: None,
                tags: vec![],
                content_type: "text/html".to_string(),
                language: None,
                author: None,
                published_at: None,
            },
            storage_info: StorageInfo {
                replicas: 3,
                locations: vec!["us-east".to_string()],
                integrity_score: 0.99,
                last_verified: chrono::Utc::now(),
            },
            created_at: chrono::Utc::now(),
            completed_at: Some(chrono::Utc::now()),
            size: 1024,
            cost: TokenAmount {
                amount: "0.001".to_string(),
                currency: "ARC".to_string(),
            },
        };
        
        let result = manager.publish_new_archive(archive).await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_subscription_type_equality() {
        assert_eq!(SubscriptionType::ArchiveUpdates, SubscriptionType::ArchiveUpdates);
        assert_ne!(SubscriptionType::ArchiveUpdates, SubscriptionType::NewArchives);
    }

    #[test]
    fn test_subscription_info_creation() {
        let info = SubscriptionInfo {
            subscription_id: "test".to_string(),
            subscription_type: SubscriptionType::NetworkStats,
            user_id: "user".to_string(),
            filter: Some(serde_json::json!({"test": "value"})),
            created_at: chrono::Utc::now(),
        };
        
        assert_eq!(info.subscription_id, "test");
        assert_eq!(info.subscription_type, SubscriptionType::NetworkStats);
        assert!(info.filter.is_some());
    }
}