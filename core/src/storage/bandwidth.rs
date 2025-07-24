//! Gestion de la bande passante pour ArchiveChain
//! 
//! Implémente :
//! - Gestion des limites de bande passante par nœud
//! - Files de priorité pour les transferts
//! - Politiques QoS (Quality of Service)
//! - Équilibrage de charge automatique
//! - Monitoring des performances réseau

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque, BinaryHeap};
use std::time::{Duration, SystemTime, Instant};
use std::cmp::Ordering;
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, sleep};
use crate::crypto::Hash;
use crate::consensus::NodeId;
use crate::error::Result;
use super::{ContentMetadata, StorageNodeInfo};

/// Configuration de gestion de bande passante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthConfig {
    /// Limite globale d'upload (bytes/sec)
    pub global_upload_limit: u64,
    /// Limite globale de download (bytes/sec)
    pub global_download_limit: u64,
    /// Limite par nœud d'upload (bytes/sec)
    pub per_node_upload_limit: u64,
    /// Limite par nœud de download (bytes/sec)
    pub per_node_download_limit: u64,
    /// Intervalle de mise à jour des métriques
    pub metrics_update_interval: Duration,
    /// Fenêtre de mesure de la bande passante
    pub bandwidth_measurement_window: Duration,
    /// Priorité par défaut pour les transferts
    pub default_priority: TransferPriority,
    /// Activation de l'équilibrage de charge
    pub load_balancing_enabled: bool,
    /// Seuil de congestion (utilisation > seuil = congestionné)
    pub congestion_threshold: f64,
}

impl Default for BandwidthConfig {
    fn default() -> Self {
        Self {
            global_upload_limit: 100 * 1024 * 1024, // 100 MB/s
            global_download_limit: 100 * 1024 * 1024, // 100 MB/s
            per_node_upload_limit: 10 * 1024 * 1024, // 10 MB/s
            per_node_download_limit: 10 * 1024 * 1024, // 10 MB/s
            metrics_update_interval: Duration::from_secs(1),
            bandwidth_measurement_window: Duration::from_secs(60), // 1 minute
            default_priority: TransferPriority::Normal,
            load_balancing_enabled: true,
            congestion_threshold: 0.8, // 80%
        }
    }
}

/// Priorité de transfert
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TransferPriority {
    /// Priorité très basse (nettoyage, maintenance)
    VeryLow = 0,
    /// Priorité basse (réplication en arrière-plan)
    Low = 1,
    /// Priorité normale (transferts standards)
    Normal = 2,
    /// Priorité haute (contenu populaire)
    High = 3,
    /// Priorité critique (urgence, récupération)
    Critical = 4,
}

impl TransferPriority {
    /// Facteur multiplicateur pour la bande passante allouée
    pub fn bandwidth_multiplier(&self) -> f64 {
        match self {
            TransferPriority::VeryLow => 0.1,
            TransferPriority::Low => 0.5,
            TransferPriority::Normal => 1.0,
            TransferPriority::High => 2.0,
            TransferPriority::Critical => 5.0,
        }
    }
}

/// Type de transfert
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferType {
    /// Upload de contenu
    Upload,
    /// Download de contenu
    Download,
    /// Synchronisation de métadonnées
    Sync,
    /// Réplication
    Replication,
}

/// Limites de bande passante pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthLimits {
    /// Limite d'upload (bytes/sec)
    pub upload_limit: u64,
    /// Limite de download (bytes/sec)
    pub download_limit: u64,
    /// Utilisation actuelle d'upload (bytes/sec)
    pub current_upload_usage: u64,
    /// Utilisation actuelle de download (bytes/sec)
    pub current_download_usage: u64,
    /// Limite d'upload par transfert
    pub per_transfer_upload_limit: u64,
    /// Limite de download par transfert
    pub per_transfer_download_limit: u64,
}

impl BandwidthLimits {
    /// Crée de nouvelles limites
    pub fn new(upload_limit: u64, download_limit: u64) -> Self {
        Self {
            upload_limit,
            download_limit,
            current_upload_usage: 0,
            current_download_usage: 0,
            per_transfer_upload_limit: upload_limit / 4, // 25% par transfert max
            per_transfer_download_limit: download_limit / 4,
        }
    }

    /// Vérifie si l'upload est disponible
    pub fn upload_available(&self, required_bandwidth: u64) -> bool {
        self.current_upload_usage + required_bandwidth <= self.upload_limit
    }

    /// Vérifie si le download est disponible
    pub fn download_available(&self, required_bandwidth: u64) -> bool {
        self.current_download_usage + required_bandwidth <= self.download_limit
    }

    /// Calcule l'utilisation d'upload en pourcentage
    pub fn upload_usage_percent(&self) -> f64 {
        if self.upload_limit == 0 {
            0.0
        } else {
            (self.current_upload_usage as f64 / self.upload_limit as f64) * 100.0
        }
    }

    /// Calcule l'utilisation de download en pourcentage
    pub fn download_usage_percent(&self) -> f64 {
        if self.download_limit == 0 {
            0.0
        } else {
            (self.current_download_usage as f64 / self.download_limit as f64) * 100.0
        }
    }
}

/// Demande de transfert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferRequest {
    /// ID unique du transfert
    pub transfer_id: Hash,
    /// Nœud source
    pub source_node: NodeId,
    /// Nœud destination
    pub destination_node: NodeId,
    /// Type de transfert
    pub transfer_type: TransferType,
    /// Priorité
    pub priority: TransferPriority,
    /// Taille des données à transférer
    pub data_size: u64,
    /// Bande passante estimée requise
    pub estimated_bandwidth: u64,
    /// Timestamp de création
    pub created_at: SystemTime,
    /// Deadline optionnelle
    pub deadline: Option<SystemTime>,
    /// Hash du contenu transféré
    pub content_hash: Hash,
    /// Métadonnées associées
    pub metadata: Option<ContentMetadata>,
}

impl TransferRequest {
    /// Crée une nouvelle demande de transfert
    pub fn new(
        source_node: NodeId,
        destination_node: NodeId,
        transfer_type: TransferType,
        priority: TransferPriority,
        data_size: u64,
        content_hash: Hash,
    ) -> Self {
        let transfer_id = Hash::from_bytes(&[
            source_node.hash().as_bytes(),
            destination_node.hash().as_bytes(),
            &content_hash.as_bytes(),
            &SystemTime::now().duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default().as_nanos().to_le_bytes(),
        ].concat()).unwrap_or_else(|_| Hash::zero());

        Self {
            transfer_id,
            source_node,
            destination_node,
            transfer_type,
            priority,
            data_size,
            estimated_bandwidth: data_size, // Estimation simple
            created_at: SystemTime::now(),
            deadline: None,
            content_hash,
            metadata: None,
        }
    }

    /// Définit une deadline
    pub fn with_deadline(mut self, deadline: SystemTime) -> Self {
        self.deadline = Some(deadline);
        self
    }

    /// Ajoute des métadonnées
    pub fn with_metadata(mut self, metadata: ContentMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Vérifie si le transfert est expiré
    pub fn is_expired(&self) -> bool {
        if let Some(deadline) = self.deadline {
            SystemTime::now() > deadline
        } else {
            false
        }
    }

    /// Calcule l'âge du transfert
    pub fn age(&self) -> Duration {
        SystemTime::now().duration_since(self.created_at).unwrap_or_default()
    }
}

/// Transfert actif
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTransfer {
    /// Demande de transfert
    pub request: TransferRequest,
    /// Timestamp de début
    pub started_at: SystemTime,
    /// Bytes transférés
    pub bytes_transferred: u64,
    /// Bande passante actuelle (bytes/sec)
    pub current_bandwidth: u64,
    /// Temps estimé de fin
    pub estimated_completion: SystemTime,
    /// Statut du transfert
    pub status: TransferStatus,
}

/// Statut d'un transfert
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransferStatus {
    /// En attente dans la queue
    Queued,
    /// En cours de transfert
    Active,
    /// En pause
    Paused,
    /// Terminé avec succès
    Completed,
    /// Échoué
    Failed,
    /// Annulé
    Cancelled,
}

impl Eq for TransferRequest {}

impl PartialEq for TransferRequest {
    fn eq(&self, other: &Self) -> bool {
        self.transfer_id == other.transfer_id
    }
}

impl Ord for TransferRequest {
    fn cmp(&self, other: &Self) -> Ordering {
        // Priorité plus élevée d'abord, puis deadline plus proche
        match self.priority.cmp(&other.priority).reverse() {
            Ordering::Equal => {
                match (self.deadline, other.deadline) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => Ordering::Less, // Avec deadline prioritaire
                    (None, Some(_)) => Ordering::Greater,
                    (None, None) => self.created_at.cmp(&other.created_at), // FIFO
                }
            }
            other => other,
        }
    }
}

impl PartialOrd for TransferRequest {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Files de priorité pour les transferts
#[derive(Debug)]
pub struct PriorityQueues {
    /// Queue de priorité pour uploads
    upload_queue: BinaryHeap<TransferRequest>,
    /// Queue de priorité pour downloads
    download_queue: BinaryHeap<TransferRequest>,
    /// Transferts actifs
    active_transfers: HashMap<Hash, ActiveTransfer>,
    /// Limite de transferts simultanés
    max_concurrent_transfers: usize,
}

impl PriorityQueues {
    /// Crée de nouvelles files de priorité
    pub fn new(max_concurrent_transfers: usize) -> Self {
        Self {
            upload_queue: BinaryHeap::new(),
            download_queue: BinaryHeap::new(),
            active_transfers: HashMap::new(),
            max_concurrent_transfers,
        }
    }

    /// Ajoute un transfert à la queue
    pub fn enqueue_transfer(&mut self, request: TransferRequest) {
        match request.transfer_type {
            TransferType::Upload | TransferType::Replication => {
                self.upload_queue.push(request);
            }
            TransferType::Download | TransferType::Sync => {
                self.download_queue.push(request);
            }
        }
    }

    /// Retire le prochain transfert prioritaire
    pub fn dequeue_next_transfer(&mut self) -> Option<TransferRequest> {
        if self.active_transfers.len() >= self.max_concurrent_transfers {
            return None;
        }

        // Équilibre entre uploads et downloads
        let upload_next = self.upload_queue.peek();
        let download_next = self.download_queue.peek();

        match (upload_next, download_next) {
            (Some(up), Some(down)) => {
                if up > down {
                    self.upload_queue.pop()
                } else {
                    self.download_queue.pop()
                }
            }
            (Some(_), None) => self.upload_queue.pop(),
            (None, Some(_)) => self.download_queue.pop(),
            (None, None) => None,
        }
    }

    /// Démarre un transfert
    pub fn start_transfer(&mut self, request: TransferRequest) -> Result<()> {
        let active_transfer = ActiveTransfer {
            request: request.clone(),
            started_at: SystemTime::now(),
            bytes_transferred: 0,
            current_bandwidth: 0,
            estimated_completion: SystemTime::now() + Duration::from_secs(60), // Estimation
            status: TransferStatus::Active,
        };

        self.active_transfers.insert(request.transfer_id, active_transfer);
        Ok(())
    }

    /// Met à jour le progrès d'un transfert
    pub fn update_transfer_progress(
        &mut self,
        transfer_id: &Hash,
        bytes_transferred: u64,
        current_bandwidth: u64,
    ) -> Result<()> {
        if let Some(transfer) = self.active_transfers.get_mut(transfer_id) {
            transfer.bytes_transferred = bytes_transferred;
            transfer.current_bandwidth = current_bandwidth;

            // Met à jour l'estimation de fin
            let remaining_bytes = transfer.request.data_size.saturating_sub(bytes_transferred);
            if current_bandwidth > 0 {
                let remaining_seconds = remaining_bytes / current_bandwidth;
                transfer.estimated_completion = SystemTime::now() + Duration::from_secs(remaining_seconds);
            }

            // Vérifie si le transfert est terminé
            if bytes_transferred >= transfer.request.data_size {
                transfer.status = TransferStatus::Completed;
            }
        }

        Ok(())
    }

    /// Termine un transfert
    pub fn complete_transfer(&mut self, transfer_id: &Hash, status: TransferStatus) -> Option<ActiveTransfer> {
        if let Some(mut transfer) = self.active_transfers.remove(transfer_id) {
            transfer.status = status;
            Some(transfer)
        } else {
            None
        }
    }

    /// Obtient les transferts actifs
    pub fn get_active_transfers(&self) -> &HashMap<Hash, ActiveTransfer> {
        &self.active_transfers
    }

    /// Nettoie les transferts expirés des queues
    pub fn cleanup_expired_transfers(&mut self) {
        // Nettoie les queues
        let mut new_upload_queue = BinaryHeap::new();
        while let Some(request) = self.upload_queue.pop() {
            if !request.is_expired() {
                new_upload_queue.push(request);
            }
        }
        self.upload_queue = new_upload_queue;

        let mut new_download_queue = BinaryHeap::new();
        while let Some(request) = self.download_queue.pop() {
            if !request.is_expired() {
                new_download_queue.push(request);
            }
        }
        self.download_queue = new_download_queue;
    }

    /// Obtient les statistiques des queues
    pub fn get_queue_stats(&self) -> QueueStats {
        QueueStats {
            upload_queue_size: self.upload_queue.len(),
            download_queue_size: self.download_queue.len(),
            active_transfers_count: self.active_transfers.len(),
            max_concurrent_transfers: self.max_concurrent_transfers,
        }
    }
}

/// Statistiques des queues
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueStats {
    /// Taille de la queue d'upload
    pub upload_queue_size: usize,
    /// Taille de la queue de download
    pub download_queue_size: usize,
    /// Nombre de transferts actifs
    pub active_transfers_count: usize,
    /// Limite de transferts simultanés
    pub max_concurrent_transfers: usize,
}

/// Politiques de Quality of Service (QoS)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QoSPolicies {
    /// Allocation de bande passante par priorité
    pub bandwidth_allocation: HashMap<TransferPriority, f64>,
    /// Latence maximale par priorité
    pub max_latency: HashMap<TransferPriority, Duration>,
    /// Stratégie de gestion de congestion
    pub congestion_strategy: CongestionStrategy,
    /// Préemption activée
    pub preemption_enabled: bool,
}

impl Default for QoSPolicies {
    fn default() -> Self {
        let mut bandwidth_allocation = HashMap::new();
        bandwidth_allocation.insert(TransferPriority::VeryLow, 0.05); // 5%
        bandwidth_allocation.insert(TransferPriority::Low, 0.15);     // 15%
        bandwidth_allocation.insert(TransferPriority::Normal, 0.40);  // 40%
        bandwidth_allocation.insert(TransferPriority::High, 0.25);    // 25%
        bandwidth_allocation.insert(TransferPriority::Critical, 0.15); // 15%

        let mut max_latency = HashMap::new();
        max_latency.insert(TransferPriority::VeryLow, Duration::from_secs(3600)); // 1h
        max_latency.insert(TransferPriority::Low, Duration::from_secs(1800));     // 30min
        max_latency.insert(TransferPriority::Normal, Duration::from_secs(300));   // 5min
        max_latency.insert(TransferPriority::High, Duration::from_secs(60));      // 1min
        max_latency.insert(TransferPriority::Critical, Duration::from_secs(10));  // 10s

        Self {
            bandwidth_allocation,
            max_latency,
            congestion_strategy: CongestionStrategy::ReduceLowPriority,
            preemption_enabled: true,
        }
    }
}

/// Stratégies de gestion de congestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionStrategy {
    /// Pause les transferts de basse priorité
    ReduceLowPriority,
    /// Équilibre proportionnellement
    ProportionalReduction,
    /// Différer les nouveaux transferts
    DeferNewTransfers,
    /// Augmenter les limites temporairement
    TemporaryBoost,
}

/// Gestionnaire de transfert
#[derive(Debug)]
pub struct TransferManager {
    /// Configuration
    config: BandwidthConfig,
    /// Files de priorité
    priority_queues: Mutex<PriorityQueues>,
    /// Métriques de transfert
    metrics: TransferMetrics,
}

/// Métriques de transfert
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TransferMetrics {
    /// Bytes totaux transférés (upload)
    pub total_uploaded: u64,
    /// Bytes totaux transférés (download)
    pub total_downloaded: u64,
    /// Nombre de transferts terminés
    pub completed_transfers: u64,
    /// Nombre de transferts échoués
    pub failed_transfers: u64,
    /// Bande passante moyenne (bytes/sec)
    pub average_bandwidth: u64,
    /// Latence moyenne des transferts
    pub average_latency: Duration,
}

impl TransferManager {
    /// Crée un nouveau gestionnaire de transfert
    pub fn new(config: BandwidthConfig) -> Self {
        Self {
            priority_queues: Mutex::new(PriorityQueues::new(10)), // 10 transferts simultanés
            metrics: TransferMetrics::default(),
            config,
        }
    }

    /// Planifie un nouveau transfert
    pub async fn schedule_transfer(&self, request: TransferRequest) -> Result<()> {
        let mut queues = self.priority_queues.lock().await;
        queues.enqueue_transfer(request);
        Ok(())
    }

    /// Traite les transferts en attente
    pub async fn process_pending_transfers(&self) -> Result<Vec<Hash>> {
        let mut queues = self.priority_queues.lock().await;
        let mut started_transfers = Vec::new();

        while let Some(request) = queues.dequeue_next_transfer() {
            queues.start_transfer(request.clone())?;
            started_transfers.push(request.transfer_id);
        }

        Ok(started_transfers)
    }

    /// Met à jour le progrès d'un transfert
    pub async fn update_progress(
        &mut self,
        transfer_id: Hash,
        bytes_transferred: u64,
        current_bandwidth: u64,
    ) -> Result<()> {
        let mut queues = self.priority_queues.lock().await;
        queues.update_transfer_progress(&transfer_id, bytes_transferred, current_bandwidth)?;

        // Met à jour les métriques
        self.metrics.average_bandwidth = 
            (self.metrics.average_bandwidth + current_bandwidth) / 2;

        Ok(())
    }

    /// Termine un transfert
    pub async fn complete_transfer(
        &mut self,
        transfer_id: Hash,
        status: TransferStatus,
    ) -> Result<Option<ActiveTransfer>> {
        let mut queues = self.priority_queues.lock().await;
        let completed_transfer = queues.complete_transfer(&transfer_id, status.clone());

        // Met à jour les métriques
        match status {
            TransferStatus::Completed => {
                self.metrics.completed_transfers += 1;
                if let Some(ref transfer) = completed_transfer {
                    match transfer.request.transfer_type {
                        TransferType::Upload | TransferType::Replication => {
                            self.metrics.total_uploaded += transfer.request.data_size;
                        }
                        TransferType::Download | TransferType::Sync => {
                            self.metrics.total_downloaded += transfer.request.data_size;
                        }
                    }
                }
            }
            TransferStatus::Failed | TransferStatus::Cancelled => {
                self.metrics.failed_transfers += 1;
            }
            _ => {}
        }

        Ok(completed_transfer)
    }

    /// Obtient les transferts actifs
    pub async fn get_active_transfers(&self) -> HashMap<Hash, ActiveTransfer> {
        let queues = self.priority_queues.lock().await;
        queues.get_active_transfers().clone()
    }

    /// Nettoie les transferts expirés
    pub async fn cleanup_expired(&self) {
        let mut queues = self.priority_queues.lock().await;
        queues.cleanup_expired_transfers();
    }

    /// Obtient les métriques
    pub fn get_metrics(&self) -> &TransferMetrics {
        &self.metrics
    }

    /// Obtient les statistiques des queues
    pub async fn get_queue_stats(&self) -> QueueStats {
        let queues = self.priority_queues.lock().await;
        queues.get_queue_stats()
    }
}

/// Équilibreur de charge
#[derive(Debug)]
pub struct LoadBalancer {
    /// Configuration
    config: BandwidthConfig,
    /// Métriques par nœud
    node_metrics: RwLock<HashMap<NodeId, NodeBandwidthMetrics>>,
    /// Politiques QoS
    qos_policies: QoSPolicies,
}

/// Métriques de bande passante par nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeBandwidthMetrics {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Bande passante d'upload utilisée (bytes/sec)
    pub upload_usage: u64,
    /// Bande passante de download utilisée (bytes/sec)
    pub download_usage: u64,
    /// Bande passante d'upload disponible (bytes/sec)
    pub upload_capacity: u64,
    /// Bande passante de download disponible (bytes/sec)
    pub download_capacity: u64,
    /// Latence moyenne (ms)
    pub average_latency: u32,
    /// Nombre de transferts actifs
    pub active_transfers: u32,
    /// Score de performance
    pub performance_score: f64,
    /// Timestamp de dernière mise à jour
    pub last_updated: SystemTime,
}

impl NodeBandwidthMetrics {
    /// Crée de nouvelles métriques pour un nœud
    pub fn new(node_id: NodeId, upload_capacity: u64, download_capacity: u64) -> Self {
        Self {
            node_id,
            upload_usage: 0,
            download_usage: 0,
            upload_capacity,
            download_capacity,
            average_latency: 0,
            active_transfers: 0,
            performance_score: 1.0,
            last_updated: SystemTime::now(),
        }
    }

    /// Calcule l'utilisation d'upload en pourcentage
    pub fn upload_utilization(&self) -> f64 {
        if self.upload_capacity == 0 {
            0.0
        } else {
            (self.upload_usage as f64 / self.upload_capacity as f64) * 100.0
        }
    }

    /// Calcule l'utilisation de download en pourcentage
    pub fn download_utilization(&self) -> f64 {
        if self.download_capacity == 0 {
            0.0
        } else {
            (self.download_usage as f64 / self.download_capacity as f64) * 100.0
        }
    }

    /// Vérifie si le nœud est congestionné
    pub fn is_congested(&self, threshold: f64) -> bool {
        self.upload_utilization() > threshold * 100.0 || 
        self.download_utilization() > threshold * 100.0
    }

    /// Calcule la bande passante disponible pour upload
    pub fn available_upload_bandwidth(&self) -> u64 {
        self.upload_capacity.saturating_sub(self.upload_usage)
    }

    /// Calcule la bande passante disponible pour download
    pub fn available_download_bandwidth(&self) -> u64 {
        self.download_capacity.saturating_sub(self.download_usage)
    }
}

impl LoadBalancer {
    /// Crée un nouveau équilibreur de charge
    pub fn new(config: BandwidthConfig, qos_policies: QoSPolicies) -> Self {
        Self {
            config,
            node_metrics: RwLock::new(HashMap::new()),
            qos_policies,
        }
    }

    /// Met à jour les métriques d'un nœud
    pub async fn update_node_metrics(&self, metrics: NodeBandwidthMetrics) {
        let mut node_metrics = self.node_metrics.write().await;
        node_metrics.insert(metrics.node_id.clone(), metrics);
    }

    /// Sélectionne le meilleur nœud pour un transfert
    pub async fn select_optimal_node(
        &self,
        candidates: &[NodeId],
        transfer_type: &TransferType,
        data_size: u64,
    ) -> Option<NodeId> {
        let node_metrics = self.node_metrics.read().await;
        let mut scored_nodes: Vec<(NodeId, f64)> = Vec::new();

        for node_id in candidates {
            if let Some(metrics) = node_metrics.get(node_id) {
                let score = self.calculate_node_score(metrics, transfer_type, data_size);
                scored_nodes.push((node_id.clone(), score));
            }
        }

        // Trie par score décroissant
        scored_nodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));

        scored_nodes.first().map(|(node_id, _)| node_id.clone())
    }

    /// Calcule le score d'un nœud pour un transfert
    fn calculate_node_score(
        &self,
        metrics: &NodeBandwidthMetrics,
        transfer_type: &TransferType,
        data_size: u64,
    ) -> f64 {
        let available_bandwidth = match transfer_type {
            TransferType::Upload | TransferType::Replication => {
                metrics.available_upload_bandwidth()
            }
            TransferType::Download | TransferType::Sync => {
                metrics.available_download_bandwidth()
            }
        };

        // Score basé sur la bande passante disponible
        let bandwidth_score = if available_bandwidth > data_size {
            1.0
        } else {
            available_bandwidth as f64 / data_size as f64
        };

        // Score basé sur la latence (latence plus faible = meilleur score)
        let latency_score = 1.0 / (1.0 + metrics.average_latency as f64 / 1000.0);

        // Score basé sur le nombre de transferts actifs
        let load_score = 1.0 / (1.0 + metrics.active_transfers as f64);

        // Score composite
        bandwidth_score * 0.5 + latency_score * 0.3 + load_score * 0.2
    }

    /// Vérifie les nœuds congestionnés
    pub async fn check_congestion(&self) -> Vec<NodeId> {
        let node_metrics = self.node_metrics.read().await;
        node_metrics.values()
            .filter(|metrics| metrics.is_congested(self.config.congestion_threshold))
            .map(|metrics| metrics.node_id.clone())
            .collect()
    }

    /// Applique les politiques QoS
    pub async fn apply_qos_policies(&self, transfers: &mut [TransferRequest]) {
        // Trie par priorité
        transfers.sort_by(|a, b| a.priority.cmp(&b.priority).reverse());

        // Applique les limites de latence
        let now = SystemTime::now();
        for transfer in transfers.iter_mut() {
            if let Some(max_latency) = self.qos_policies.max_latency.get(&transfer.priority) {
                if transfer.deadline.is_none() {
                    transfer.deadline = Some(now + *max_latency);
                }
            }
        }
    }

    /// Obtient les statistiques de l'équilibreur
    pub async fn get_load_balancer_stats(&self) -> LoadBalancerStats {
        let node_metrics = self.node_metrics.read().await;
        
        let total_nodes = node_metrics.len();
        let congested_nodes = node_metrics.values()
            .filter(|m| m.is_congested(self.config.congestion_threshold))
            .count();

        let total_upload_capacity: u64 = node_metrics.values()
            .map(|m| m.upload_capacity)
            .sum();
        let total_upload_usage: u64 = node_metrics.values()
            .map(|m| m.upload_usage)
            .sum();

        let total_download_capacity: u64 = node_metrics.values()
            .map(|m| m.download_capacity)
            .sum();
        let total_download_usage: u64 = node_metrics.values()
            .map(|m| m.download_usage)
            .sum();

        LoadBalancerStats {
            total_nodes: total_nodes as u32,
            congested_nodes: congested_nodes as u32,
            total_upload_capacity,
            total_upload_usage,
            total_download_capacity,
            total_download_usage,
            average_utilization: if total_nodes > 0 {
                node_metrics.values()
                    .map(|m| (m.upload_utilization() + m.download_utilization()) / 2.0)
                    .sum::<f64>() / total_nodes as f64
            } else {
                0.0
            },
        }
    }
}

/// Statistiques de l'équilibreur de charge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerStats {
    /// Nombre total de nœuds
    pub total_nodes: u32,
    /// Nombre de nœuds congestionnés
    pub congested_nodes: u32,
    /// Capacité totale d'upload
    pub total_upload_capacity: u64,
    /// Utilisation totale d'upload
    pub total_upload_usage: u64,
    /// Capacité totale de download
    pub total_download_capacity: u64,
    /// Utilisation totale de download
    pub total_download_usage: u64,
    /// Utilisation moyenne
    pub average_utilization: f64,
}

/// Gestionnaire principal de bande passante
pub struct BandwidthManager {
    /// Configuration
    config: BandwidthConfig,
    /// Limites par nœud
    node_limits: RwLock<HashMap<NodeId, BandwidthLimits>>,
    /// Gestionnaire de transfert
    transfer_manager: TransferManager,
    /// Équilibreur de charge
    load_balancer: LoadBalancer,
    /// Politiques QoS
    qos_policies: QoSPolicies,
}

impl BandwidthManager {
    /// Crée un nouveau gestionnaire de bande passante
    pub fn new(config: BandwidthConfig) -> Self {
        let qos_policies = QoSPolicies::default();
        let transfer_manager = TransferManager::new(config.clone());
        let load_balancer = LoadBalancer::new(config.clone(), qos_policies.clone());

        Self {
            config,
            node_limits: RwLock::new(HashMap::new()),
            transfer_manager,
            load_balancer,
            qos_policies,
        }
    }

    /// Configure les limites pour un nœud
    pub async fn set_node_limits(&self, node_id: NodeId, limits: BandwidthLimits) {
        let mut node_limits = self.node_limits.write().await;
        node_limits.insert(node_id, limits);
    }

    /// Planifie un transfert
    pub async fn schedule_transfer(&self, request: TransferRequest) -> Result<()> {
        self.transfer_manager.schedule_transfer(request).await
    }

    /// Sélectionne le meilleur nœud pour un transfert
    pub async fn select_transfer_node(
        &self,
        candidates: &[NodeId],
        transfer_type: TransferType,
        data_size: u64,
    ) -> Option<NodeId> {
        self.load_balancer.select_optimal_node(candidates, &transfer_type, data_size).await
    }

    /// Obtient les statistiques complètes
    pub async fn get_bandwidth_stats(&self) -> BandwidthStats {
        let transfer_metrics = self.transfer_manager.get_metrics().clone();
        let queue_stats = self.transfer_manager.get_queue_stats().await;
        let load_balancer_stats = self.load_balancer.get_load_balancer_stats().await;

        BandwidthStats {
            transfer_metrics,
            queue_stats,
            load_balancer_stats,
            global_limits: GlobalBandwidthLimits {
                upload_limit: self.config.global_upload_limit,
                download_limit: self.config.global_download_limit,
                current_upload_usage: load_balancer_stats.total_upload_usage,
                current_download_usage: load_balancer_stats.total_download_usage,
            },
        }
    }

    /// Vérifie et applique les politiques QoS
    pub async fn enforce_qos(&self) -> Result<()> {
        // Vérifie la congestion
        let congested_nodes = self.load_balancer.check_congestion().await;
        
        if !congested_nodes.is_empty() {
            // Applique la stratégie de gestion de congestion
            match self.qos_policies.congestion_strategy {
                CongestionStrategy::ReduceLowPriority => {
                    // Pause les transferts de basse priorité
                    // Implémentation simplifiée
                }
                CongestionStrategy::DeferNewTransfers => {
                    // Diffère les nouveaux transferts
                    // Implémentation simplifiée
                }
                _ => {
                    // Autres stratégies...
                }
            }
        }

        Ok(())
    }
}

/// Limites globales de bande passante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalBandwidthLimits {
    /// Limite globale d'upload
    pub upload_limit: u64,
    /// Limite globale de download
    pub download_limit: u64,
    /// Utilisation actuelle d'upload
    pub current_upload_usage: u64,
    /// Utilisation actuelle de download
    pub current_download_usage: u64,
}

/// Statistiques complètes de bande passante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthStats {
    /// Métriques de transfert
    pub transfer_metrics: TransferMetrics,
    /// Statistiques des queues
    pub queue_stats: QueueStats,
    /// Statistiques de l'équilibreur
    pub load_balancer_stats: LoadBalancerStats,
    /// Limites globales
    pub global_limits: GlobalBandwidthLimits,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    #[test]
    fn test_transfer_priority_ordering() {
        assert!(TransferPriority::Critical > TransferPriority::High);
        assert!(TransferPriority::High > TransferPriority::Normal);
        assert!(TransferPriority::Normal > TransferPriority::Low);
    }

    #[test]
    fn test_bandwidth_limits() {
        let limits = BandwidthLimits::new(1000, 2000);
        assert!(limits.upload_available(500));
        assert!(!limits.upload_available(1500));
        assert_eq!(limits.upload_usage_percent(), 0.0);
    }

    #[test]
    fn test_transfer_request_ordering() {
        let req1 = TransferRequest::new(
            NodeId::from(Hash::zero()),
            NodeId::from(Hash::zero()),
            TransferType::Upload,
            TransferPriority::Normal,
            1000,
            Hash::zero(),
        );

        let req2 = TransferRequest::new(
            NodeId::from(Hash::zero()),
            NodeId::from(Hash::zero()),
            TransferType::Upload,
            TransferPriority::High,
            1000,
            Hash::zero(),
        );

        assert!(req2 > req1); // Higher priority should come first
    }

    #[tokio::test]
    async fn test_priority_queues() {
        let mut queues = PriorityQueues::new(5);
        
        let request = TransferRequest::new(
            NodeId::from(Hash::zero()),
            NodeId::from(Hash::zero()),
            TransferType::Upload,
            TransferPriority::Normal,
            1000,
            Hash::zero(),
        );

        queues.enqueue_transfer(request.clone());
        let dequeued = queues.dequeue_next_transfer();
        assert!(dequeued.is_some());
        assert_eq!(dequeued.unwrap().transfer_id, request.transfer_id);
    }

    #[test]
    fn test_node_bandwidth_metrics() {
        let metrics = NodeBandwidthMetrics::new(
            NodeId::from(Hash::zero()),
            1000,
            2000,
        );

        assert_eq!(metrics.upload_utilization(), 0.0);
        assert_eq!(metrics.available_upload_bandwidth(), 1000);
        assert!(!metrics.is_congested(0.8));
    }

    #[tokio::test]
    async fn test_bandwidth_manager() {
        let config = BandwidthConfig::default();
        let manager = BandwidthManager::new(config);

        let request = TransferRequest::new(
            NodeId::from(Hash::zero()),
            NodeId::from(Hash::zero()),
            TransferType::Upload,
            TransferPriority::Normal,
            1000,
            Hash::zero(),
        );

        let result = manager.schedule_transfer(request).await;
        assert!(result.is_ok());
    }
}