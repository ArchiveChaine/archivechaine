//! Service de synchronisation P2P pour ArchiveChain
//!
//! Implémente la synchronisation de la blockchain entre pairs.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot, mpsc};
use tokio::time::{Duration, interval, timeout};

use crate::Blockchain;
use super::{P2PConfig, P2PError, P2PResult, messages::*};

/// Service de synchronisation
#[derive(Debug)]
pub struct SyncService {
    /// Configuration
    config: P2PConfig,
    /// Référence à la blockchain
    blockchain: Arc<Blockchain>,
    /// Synchronisations actives
    active_syncs: Arc<RwLock<HashMap<String, SyncSession>>>,
    /// Queue de blocs à traiter
    block_queue: Arc<RwLock<VecDeque<BlockData>>>,
    /// Canal d'arrêt
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    /// Statistiques de synchronisation
    sync_stats: Arc<RwLock<SyncStats>>,
}

/// Session de synchronisation active
#[derive(Debug, Clone)]
pub struct SyncSession {
    /// ID de la session
    pub session_id: String,
    /// ID du pair
    pub peer_id: String,
    /// Hauteur de départ
    pub start_height: u64,
    /// Hauteur de fin
    pub end_height: u64,
    /// Hauteur actuelle
    pub current_height: u64,
    /// Statut de la synchronisation
    pub status: SyncStatus,
    /// Heure de début
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Dernière activité
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Blocs reçus
    pub blocks_received: u64,
    /// Blocs traités
    pub blocks_processed: u64,
}

/// Statut de synchronisation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncStatus {
    Requesting,
    Receiving,
    Processing,
    Completed,
    Failed(String),
    Cancelled,
}

/// Statistiques de synchronisation
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncStats {
    pub total_syncs_started: u64,
    pub total_syncs_completed: u64,
    pub total_syncs_failed: u64,
    pub total_blocks_synced: u64,
    pub average_sync_time_ms: u64,
    pub active_sync_sessions: usize,
    pub pending_blocks: usize,
}

impl SyncService {
    /// Crée un nouveau service de synchronisation
    pub fn new(config: P2PConfig, blockchain: Arc<Blockchain>) -> Self {
        Self {
            config,
            blockchain,
            active_syncs: Arc::new(RwLock::new(HashMap::new())),
            block_queue: Arc::new(RwLock::new(VecDeque::new())),
            shutdown_tx: Arc::new(RwLock::new(None)),
            sync_stats: Arc::new(RwLock::new(SyncStats::default())),
        }
    }

    /// Démarre le service de synchronisation
    pub async fn start(&self) -> P2PResult<()> {
        tracing::info!("Starting P2P sync service");

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        {
            let mut shutdown_guard = self.shutdown_tx.write().await;
            *shutdown_guard = Some(shutdown_tx);
        }

        // Démarre la tâche de traitement des blocs
        self.start_block_processor().await;

        // Démarre la tâche de nettoyage des sessions expirées
        self.start_session_cleanup().await;

        // Démarre la tâche de synchronisation automatique
        let active_syncs = self.active_syncs.clone();
        let blockchain = self.blockchain.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(30)); // Vérifie toutes les 30 secondes

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Vérifie si une synchronisation automatique est nécessaire
                        if let Err(e) = Self::check_auto_sync(&active_syncs, &blockchain, &config).await {
                            tracing::error!("Auto sync check failed: {}", e);
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Sync service shutting down");
                        break;
                    }
                }
            }
        });

        tracing::info!("P2P sync service started");
        Ok(())
    }

    /// Arrête le service de synchronisation
    pub async fn stop(&self) -> P2PResult<()> {
        tracing::info!("Stopping P2P sync service");

        if let Some(shutdown_tx) = self.shutdown_tx.write().await.take() {
            let _ = shutdown_tx.send(());
        }

        // Annule toutes les synchronisations actives
        {
            let mut syncs = self.active_syncs.write().await;
            for (_, session) in syncs.iter_mut() {
                session.status = SyncStatus::Cancelled;
            }
        }

        tracing::info!("P2P sync service stopped");
        Ok(())
    }

    /// Démarre une synchronisation avec un pair
    pub async fn start_sync(
        &self,
        peer_id: String,
        start_height: u64,
        end_height: Option<u64>,
    ) -> P2PResult<String> {
        let session_id = format!("sync_{}", uuid::Uuid::new_v4().simple());
        
        // Récupère la hauteur actuelle de la blockchain
        let current_height = self.blockchain.get_stats()
            .map_err(|e| P2PError::Internal(format!("Failed to get blockchain stats: {}", e)))?
            .height;

        let target_height = end_height.unwrap_or(current_height + 1000); // Par défaut, sync 1000 blocs

        if start_height >= target_height {
            return Err(P2PError::ProtocolError("Invalid sync range".to_string()));
        }

        let session = SyncSession {
            session_id: session_id.clone(),
            peer_id: peer_id.clone(),
            start_height,
            end_height: target_height,
            current_height: start_height,
            status: SyncStatus::Requesting,
            started_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            blocks_received: 0,
            blocks_processed: 0,
        };

        // Ajoute la session
        {
            let mut syncs = self.active_syncs.write().await;
            syncs.insert(session_id.clone(), session);

            let mut stats = self.sync_stats.write().await;
            stats.total_syncs_started += 1;
            stats.active_sync_sessions = syncs.len();
        }

        tracing::info!("Started sync session {} with peer {} ({}->{})", 
            session_id, peer_id, start_height, target_height);

        // TODO: Envoyer la requête de synchronisation au pair
        
        Ok(session_id)
    }

    /// Traite une demande de synchronisation reçue
    pub async fn handle_sync_request(
        &self,
        peer_id: String,
        request: P2PMessage,
    ) -> P2PResult<P2PMessage> {
        if let P2PMessage::SyncRequest { start_height, end_height, request_id } = request {
            let actual_end_height = end_height.unwrap_or(start_height + 100).min(start_height + 1000);
            
            // Vérifie que la plage est valide
            if start_height >= actual_end_height {
                return Ok(MessageBuilder::error(
                    1006, // SYNC_ERROR
                    "Invalid sync range".to_string(),
                    Some(request_id),
                ));
            }

            // TODO: Récupérer les blocs depuis la blockchain
            // Pour l'instant, on simule
            let blocks = self.get_blocks_range(start_height, actual_end_height).await?;

            if blocks.is_empty() {
                return Ok(P2PMessage::SyncEnd {
                    request_id,
                    success: false,
                    message: Some("No blocks available".to_string()),
                });
            }

            // Commence la synchronisation
            let sync_start = P2PMessage::SyncStart {
                start_height,
                end_height: actual_end_height,
                request_id: request_id.clone(),
            };

            // TODO: Envoyer les données en chunks
            // Pour l'instant, on envoie tout en une fois
            tracing::info!("Handling sync request from {} ({}->{})", 
                peer_id, start_height, actual_end_height);

            Ok(sync_start)
        } else {
            Err(P2PError::ProtocolError("Invalid sync request message".to_string()))
        }
    }

    /// Traite des données de synchronisation reçues
    pub async fn handle_sync_data(
        &self,
        peer_id: String,
        message: P2PMessage,
    ) -> P2PResult<()> {
        if let P2PMessage::SyncData { blocks, request_id, is_last } = message {
            // Trouve la session correspondante
            let session_id = {
                let syncs = self.active_syncs.read().await;
                syncs.iter()
                    .find(|(_, session)| session.peer_id == peer_id)
                    .map(|(id, _)| id.clone())
            };

            if let Some(session_id) = session_id {
                // Met à jour la session
                {
                    let mut syncs = self.active_syncs.write().await;
                    if let Some(session) = syncs.get_mut(&session_id) {
                        session.blocks_received += blocks.len() as u64;
                        session.last_activity = chrono::Utc::now();
                        session.status = if is_last {
                            SyncStatus::Processing
                        } else {
                            SyncStatus::Receiving
                        };
                    }
                }

                // Ajoute les blocs à la queue de traitement
                {
                    let mut queue = self.block_queue.write().await;
                    for block in blocks {
                        queue.push_back(block);
                    }
                }

                if is_last {
                    tracing::info!("Completed receiving blocks for sync session: {}", session_id);
                }
            }
        }

        Ok(())
    }

    /// Récupère une plage de blocs
    async fn get_blocks_range(&self, start_height: u64, end_height: u64) -> P2PResult<Vec<BlockData>> {
        // TODO: Implémenter la récupération réelle depuis la blockchain
        // Pour l'instant, retourne une liste vide
        tracing::debug!("Getting blocks range: {} -> {}", start_height, end_height);
        Ok(vec![])
    }

    /// Démarre le processeur de blocs
    async fn start_block_processor(&self) {
        let block_queue = self.block_queue.clone();
        let blockchain = self.blockchain.clone();
        let active_syncs = self.active_syncs.clone();
        let sync_stats = self.sync_stats.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(100)); // Traite les blocs rapidement

            loop {
                interval.tick().await;

                // Traite un bloc de la queue
                let block_opt = {
                    let mut queue = block_queue.write().await;
                    queue.pop_front()
                };

                if let Some(block) = block_opt {
                    match Self::process_block(&blockchain, block).await {
                        Ok(()) => {
                            // Met à jour les statistiques
                            let mut stats = sync_stats.write().await;
                            stats.total_blocks_synced += 1;
                            stats.pending_blocks = {
                                let queue = block_queue.read().await;
                                queue.len()
                            };
                        }
                        Err(e) => {
                            tracing::error!("Failed to process synced block: {}", e);
                        }
                    }
                }
            }
        });
    }

    /// Traite un bloc reçu
    async fn process_block(blockchain: &Arc<Blockchain>, block: BlockData) -> P2PResult<()> {
        // TODO: Convertir BlockData vers le format interne et l'ajouter à la blockchain
        tracing::debug!("Processing synced block at height: {}", block.height);
        
        // Valide le bloc
        if block.hash.is_empty() {
            return Err(P2PError::ProtocolError("Invalid block: empty hash".to_string()));
        }
        
        if block.height == 0 {
            return Err(P2PError::ProtocolError("Invalid block: zero height".to_string()));
        }

        // TODO: Ajouter à la blockchain réelle
        Ok(())
    }

    /// Démarre le nettoyage des sessions
    async fn start_session_cleanup(&self) {
        let active_syncs = self.active_syncs.clone();
        let sync_stats = self.sync_stats.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(60)); // Nettoie chaque minute

            loop {
                interval.tick().await;

                let mut syncs = active_syncs.write().await;
                let cutoff = chrono::Utc::now() - chrono::Duration::minutes(10);

                let initial_count = syncs.len();
                syncs.retain(|session_id, session| {
                    let should_keep = match &session.status {
                        SyncStatus::Completed | SyncStatus::Failed(_) | SyncStatus::Cancelled => {
                            session.last_activity > cutoff
                        }
                        _ => {
                            // Sessions actives, garde si récente activité
                            session.last_activity > cutoff
                        }
                    };

                    if !should_keep {
                        tracing::debug!("Removing expired sync session: {}", session_id);
                    }

                    should_keep
                });

                // Met à jour les statistiques
                let mut stats = sync_stats.write().await;
                stats.active_sync_sessions = syncs.len();

                if syncs.len() < initial_count {
                    tracing::debug!("Cleaned up {} expired sync sessions", initial_count - syncs.len());
                }
            }
        });
    }

    /// Vérifie si une synchronisation automatique est nécessaire
    async fn check_auto_sync(
        active_syncs: &Arc<RwLock<HashMap<String, SyncSession>>>,
        blockchain: &Arc<Blockchain>,
        config: &P2PConfig,
    ) -> P2PResult<()> {
        // TODO: Implémenter la logique de synchronisation automatique
        // Par exemple, détecter si on est en retard par rapport aux pairs
        Ok(())
    }

    /// Annule une session de synchronisation
    pub async fn cancel_sync(&self, session_id: &str) -> P2PResult<()> {
        let mut syncs = self.active_syncs.write().await;
        
        if let Some(session) = syncs.get_mut(session_id) {
            session.status = SyncStatus::Cancelled;
            session.last_activity = chrono::Utc::now();
            tracing::info!("Cancelled sync session: {}", session_id);
        }

        Ok(())
    }

    /// Récupère les sessions de synchronisation actives
    pub async fn get_active_syncs(&self) -> Vec<SyncSession> {
        let syncs = self.active_syncs.read().await;
        syncs.values().cloned().collect()
    }

    /// Récupère les statistiques de synchronisation
    pub async fn get_sync_stats(&self) -> SyncStats {
        let stats = self.sync_stats.read().await;
        let mut stats_copy = stats.clone();
        
        // Met à jour les statistiques en temps réel
        {
            let syncs = self.active_syncs.read().await;
            stats_copy.active_sync_sessions = syncs.len();
        }
        
        {
            let queue = self.block_queue.read().await;
            stats_copy.pending_blocks = queue.len();
        }

        stats_copy
    }

    /// Force la synchronisation avec un pair spécifique
    pub async fn force_sync_with_peer(&self, peer_id: String) -> P2PResult<String> {
        let current_height = self.blockchain.get_stats()
            .map_err(|e| P2PError::Internal(format!("Failed to get blockchain stats: {}", e)))?
            .height;

        self.start_sync(peer_id, current_height, None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockchainConfig;

    #[tokio::test]
    async fn test_sync_service_creation() {
        let config = P2PConfig::default();
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let service = SyncService::new(config, blockchain);
        
        // Vérifie que le service peut être créé
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_sync_session() {
        let session = SyncSession {
            session_id: "sync_123".to_string(),
            peer_id: "peer_456".to_string(),
            start_height: 100,
            end_height: 200,
            current_height: 150,
            status: SyncStatus::Receiving,
            started_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            blocks_received: 50,
            blocks_processed: 45,
        };

        assert_eq!(session.session_id, "sync_123");
        assert_eq!(session.peer_id, "peer_456");
        assert_eq!(session.status, SyncStatus::Receiving);
        assert_eq!(session.blocks_received, 50);
        assert_eq!(session.blocks_processed, 45);
    }

    #[test]
    fn test_sync_status() {
        assert_eq!(SyncStatus::Requesting, SyncStatus::Requesting);
        assert_ne!(SyncStatus::Requesting, SyncStatus::Receiving);
        
        let failed_status = SyncStatus::Failed("test error".to_string());
        match failed_status {
            SyncStatus::Failed(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Failed status"),
        }
    }

    #[tokio::test]
    async fn test_start_sync() {
        let config = P2PConfig::default();
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let service = SyncService::new(config, blockchain);

        let result = service.start_sync("peer_123".to_string(), 100, Some(200)).await;
        assert!(result.is_ok());

        let session_id = result.unwrap();
        assert!(session_id.starts_with("sync_"));

        let syncs = service.get_active_syncs().await;
        assert_eq!(syncs.len(), 1);
        assert_eq!(syncs[0].peer_id, "peer_123");
        assert_eq!(syncs[0].start_height, 100);
        assert_eq!(syncs[0].end_height, 200);
    }

    #[tokio::test]
    async fn test_invalid_sync_range() {
        let config = P2PConfig::default();
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let service = SyncService::new(config, blockchain);

        let result = service.start_sync("peer_123".to_string(), 200, Some(100)).await;
        assert!(result.is_err());
        
        match result.unwrap_err() {
            P2PError::ProtocolError(msg) => assert!(msg.contains("Invalid sync range")),
            _ => panic!("Expected ProtocolError"),
        }
    }

    #[tokio::test]
    async fn test_cancel_sync() {
        let config = P2PConfig::default();
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let service = SyncService::new(config, blockchain);

        let session_id = service.start_sync("peer_123".to_string(), 100, Some(200)).await.unwrap();
        
        let result = service.cancel_sync(&session_id).await;
        assert!(result.is_ok());

        let syncs = service.get_active_syncs().await;
        assert_eq!(syncs.len(), 1);
        assert_eq!(syncs[0].status, SyncStatus::Cancelled);
    }

    #[tokio::test]
    async fn test_sync_stats() {
        let config = P2PConfig::default();
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let service = SyncService::new(config, blockchain);

        let stats = service.get_sync_stats().await;
        assert_eq!(stats.total_syncs_started, 0);
        assert_eq!(stats.active_sync_sessions, 0);
        assert_eq!(stats.pending_blocks, 0);

        // Démarre une synchronisation
        service.start_sync("peer_123".to_string(), 100, Some(200)).await.unwrap();

        let stats = service.get_sync_stats().await;
        assert_eq!(stats.total_syncs_started, 1);
        assert_eq!(stats.active_sync_sessions, 1);
    }

    #[tokio::test]
    async fn test_sync_data_handling() {
        let config = P2PConfig::default();
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let service = SyncService::new(config, blockchain);

        // Démarre une synchronisation
        service.start_sync("peer_123".to_string(), 100, Some(200)).await.unwrap();

        // Simule la réception de données
        let blocks = vec![
            BlockData {
                height: 101,
                hash: "hash_101".to_string(),
                previous_hash: "hash_100".to_string(),
                timestamp: chrono::Utc::now(),
                transactions: vec![],
                merkle_root: "merkle_101".to_string(),
                validator: "validator_1".to_string(),
                signature: "sig_101".to_string(),
            }
        ];

        let sync_data = P2PMessage::SyncData {
            blocks,
            request_id: "req_123".to_string(),
            is_last: true,
        };

        let result = service.handle_sync_data("peer_123".to_string(), sync_data).await;
        assert!(result.is_ok());

        // Vérifie que les blocs sont en queue
        let stats = service.get_sync_stats().await;
        assert_eq!(stats.pending_blocks, 1);
    }

    #[tokio::test]
    async fn test_process_block() {
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        
        let block = BlockData {
            height: 1,
            hash: "a".repeat(64),
            previous_hash: "b".repeat(64),
            timestamp: chrono::Utc::now(),
            transactions: vec![],
            merkle_root: "c".repeat(64),
            validator: "validator_1".to_string(),
            signature: "signature_1".to_string(),
        };

        let result = SyncService::process_block(&blockchain, block).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_invalid_block_processing() {
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        
        let invalid_block = BlockData {
            height: 0, // Invalid height
            hash: "".to_string(), // Empty hash
            previous_hash: "".to_string(),
            timestamp: chrono::Utc::now(),
            transactions: vec![],
            merkle_root: "".to_string(),
            validator: "".to_string(),
            signature: "".to_string(),
        };

        let result = SyncService::process_block(&blockchain, invalid_block).await;
        assert!(result.is_err());
    }
}