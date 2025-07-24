//! Implémentation du Light Storage Node
//!
//! Les Light Storage Nodes forment l'épine dorsale du stockage distribué :
//! - Stockage spécialisé par catégorie (1-10TB)
//! - Participation sélective au consensus
//! - Cache intelligent des archives populaires
//! - Synchronisation partielle de la blockchain
//! - Optimisation selon la spécialisation

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;
use regex::Regex;

use crate::crypto::{Hash, PublicKey, PrivateKey};
use crate::consensus::{NodeId, ConsensusScore};
use crate::storage::{
    // StorageManager, StorageNodeInfo, ContentMetadata, DistributedStorage,
    // StorageType, NodeStatus
};
use crate::error::Result;
use super::{
    Node, NodeType, NodeConfiguration, NetworkMessage, MessageType,
    NodeHealth, NodeMetrics, GeneralNodeMetrics, HealthStatus
};

/// Types de spécialisation pour les Light Storage Nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StorageSpecialization {
    /// Spécialisation par domaine (ex: .gov, .edu, .org)
    Domain,
    /// Spécialisation par type de contenu (HTML, PDF, images)
    ContentType,
    /// Spécialisation géographique
    Geographic,
    /// Spécialisation temporelle (par période)
    Temporal,
    /// Spécialisation par langue
    Language,
    /// Spécialisation par popularité
    Popularity,
    /// Spécialisation personnalisée
    Custom { name: String, description: String },
}

/// Filtre de contenu pour la spécialisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFilter {
    /// Spécialisation du nœud
    pub specialization: StorageSpecialization,
    /// Filtres regex pour les URLs
    pub url_patterns: Vec<String>,
    /// Types MIME acceptés
    pub accepted_mime_types: Vec<String>,
    /// Domaines préférés
    pub preferred_domains: Vec<String>,
    /// Langues acceptées (codes ISO)
    pub accepted_languages: Vec<String>,
    /// Taille minimale de fichier (bytes)
    pub min_file_size: Option<u64>,
    /// Taille maximale de fichier (bytes)
    pub max_file_size: Option<u64>,
    /// Score de popularité minimum
    pub min_popularity_score: Option<u64>,
    /// Période temporelle (pour spécialisation temporelle)
    pub temporal_range: Option<TemporalRange>,
}

/// Période temporelle pour la spécialisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalRange {
    /// Date de début
    pub start_date: chrono::DateTime<chrono::Utc>,
    /// Date de fin
    pub end_date: chrono::DateTime<chrono::Utc>,
}

/// Configuration spécifique aux Light Storage Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightStorageConfig {
    /// Configuration générale du nœud
    pub node_config: NodeConfiguration,
    /// Capacité de stockage (1-10TB)
    pub storage_capacity: u64,
    /// Spécialisation du nœud
    pub specialization: StorageSpecialization,
    /// Filtre de contenu
    pub content_filter: ContentFilter,
    /// Participation au consensus
    pub consensus_participation: bool,
    /// Poids dans le consensus (réduit)
    pub consensus_weight: f64,
    /// Facteur de réplication (3-8)
    pub replication_factor: u32,
    /// Taille du cache populaire
    pub popular_cache_size: u64,
    /// Intervalle de nettoyage du cache
    pub cache_cleanup_interval: Duration,
    /// Seuil de popularité pour mise en cache
    pub popularity_threshold: u64,
    /// Configuration de synchronisation partielle
    pub partial_sync_config: PartialSyncConfig,
}

/// Configuration de synchronisation partielle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSyncConfig {
    /// Synchronisation activée
    pub enabled: bool,
    /// Intervalle de synchronisation
    pub sync_interval: Duration,
    /// Nombre maximum de blocs à synchroniser par cycle
    pub max_blocks_per_sync: u32,
    /// Synchronisation sélective basée sur la spécialisation
    pub selective_sync: bool,
}

/// Statut d'un Light Storage Node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LightStorageStatus {
    /// Initialisation en cours
    Initializing,
    /// Configuration de la spécialisation
    ConfiguringSpecialization,
    /// Synchronisation sélective
    SelectiveSync,
    /// Opérationnel
    Operational,
    /// Optimisation du cache
    CacheOptimization,
    /// Maintenance
    Maintenance,
    /// Échec de spécialisation
    SpecializationFailed,
    /// Arrêt en cours
    Stopping,
}

/// Métriques spécifiques aux Light Storage Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightStorageMetrics {
    /// Métriques générales
    pub general: GeneralNodeMetrics,
    /// Contenu spécialisé stocké
    pub specialized_content_count: u64,
    /// Taille du contenu spécialisé (bytes)
    pub specialized_content_size: u64,
    /// Taux de correspondance avec la spécialisation
    pub specialization_match_rate: f64,
    /// Contenu en cache populaire
    pub cached_popular_content: u64,
    /// Taux de hit du cache
    pub cache_hit_rate: f64,
    /// Nombre de requêtes servies
    pub requests_served: u64,
    /// Taux de participation au consensus
    pub consensus_participation_rate: f64,
    /// Score de spécialisation
    pub specialization_score: f64,
    /// Efficacité du stockage
    pub storage_efficiency: f64,
}

impl NodeMetrics for LightStorageMetrics {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn general_metrics(&self) -> GeneralNodeMetrics {
        self.general.clone()
    }
}

/// Light Storage Node - Nœud de stockage léger spécialisé
pub struct LightStorageNode {
    /// Configuration du nœud
    config: LightStorageConfig,
    /// Identifiant du nœud
    node_id: NodeId,
    /// Clés cryptographiques
    keypair: (PublicKey, PrivateKey),
    /// Statut actuel
    status: Arc<RwLock<LightStorageStatus>>,
    /// Gestionnaire de stockage
    storage_manager: Arc<Mutex<StorageManager>>,
    /// Index local des archives spécialisées
    local_archive_index: Arc<RwLock<HashMap<Hash, ArchiveMetadata>>>,
    /// Cache des archives populaires
    popular_cache: Arc<RwLock<HashMap<Hash, CachedArchive>>>,
    /// Métriques de performance
    metrics: Arc<RwLock<LightStorageMetrics>>,
    /// Filtres regex compilés
    compiled_filters: Arc<RwLock<Vec<Regex>>>,
    /// Dernière optimisation du cache
    last_cache_optimization: Arc<Mutex<SystemTime>>,
    /// Heure de démarrage
    start_time: SystemTime,
}

/// Métadonnées d'archive dans l'index local
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// Hash de l'archive
    pub content_hash: Hash,
    /// Métadonnées de contenu
    pub metadata: ContentMetadata,
    /// Score de correspondance avec la spécialisation
    pub specialization_score: f64,
    /// Dernière date d'accès
    pub last_accessed: chrono::DateTime<chrono::Utc>,
    /// Nombre d'accès
    pub access_count: u64,
    /// Stocké localement
    pub stored_locally: bool,
}

/// Archive en cache
#[derive(Debug, Clone)]
pub struct CachedArchive {
    /// Hash de l'archive
    pub content_hash: Hash,
    /// Données de l'archive
    pub data: Vec<u8>,
    /// Métadonnées
    pub metadata: ContentMetadata,
    /// Timestamp de mise en cache
    pub cached_at: SystemTime,
    /// Nombre d'accès depuis la mise en cache
    pub cache_hits: u64,
    /// Dernière date d'accès
    pub last_access: SystemTime,
}

impl Default for LightStorageConfig {
    fn default() -> Self {
        Self {
            node_config: NodeConfiguration {
                node_id: NodeId::from(Hash::zero()),
                node_type: NodeType::LightStorage {
                    storage_capacity: 5_000_000_000_000, // 5TB
                    specialization: StorageSpecialization::ContentType,
                },
                region: "us-east-1".to_string(),
                listen_address: "0.0.0.0".to_string(),
                listen_port: 8081,
                bootstrap_nodes: Vec::new(),
                storage_config: None,
                network_config: super::NetworkConfiguration::default(),
                security_config: super::SecurityConfiguration::default(),
            },
            storage_capacity: 5_000_000_000_000, // 5TB
            specialization: StorageSpecialization::ContentType,
            content_filter: ContentFilter::default(),
            consensus_participation: true,
            consensus_weight: 0.5,
            replication_factor: 5,
            popular_cache_size: 100_000_000_000, // 100GB
            cache_cleanup_interval: Duration::from_secs(3600), // 1 heure
            popularity_threshold: 100,
            partial_sync_config: PartialSyncConfig::default(),
        }
    }
}

impl Default for ContentFilter {
    fn default() -> Self {
        Self {
            specialization: StorageSpecialization::ContentType,
            url_patterns: vec![r".*\.html$".to_string(), r".*\.htm$".to_string()],
            accepted_mime_types: vec!["text/html".to_string()],
            preferred_domains: Vec::new(),
            accepted_languages: Vec::new(),
            min_file_size: None,
            max_file_size: None,
            min_popularity_score: None,
            temporal_range: None,
        }
    }
}

impl Default for PartialSyncConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sync_interval: Duration::from_secs(300), // 5 minutes
            max_blocks_per_sync: 100,
            selective_sync: true,
        }
    }
}

impl LightStorageNode {
    /// Crée une nouvelle instance de Light Storage Node
    pub fn new(
        config: LightStorageConfig,
        keypair: (PublicKey, PrivateKey),
        storage_manager: StorageManager,
    ) -> Result<Self> {
        // Valide la configuration
        config.validate()?;

        let node_id = config.node_config.node_id.clone();
        let start_time = SystemTime::now();

        // Compile les filtres regex
        let mut compiled_filters = Vec::new();
        for pattern in &config.content_filter.url_patterns {
            match Regex::new(pattern) {
                Ok(regex) => compiled_filters.push(regex),
                Err(e) => {
                    tracing::warn!("Erreur compilation regex '{}': {}", pattern, e);
                }
            }
        }

        let initial_metrics = LightStorageMetrics {
            general: GeneralNodeMetrics {
                uptime: Duration::ZERO,
                cpu_usage: 0.0,
                memory_usage: 0.0,
                storage_usage: 0.0,
                bandwidth_in: 0,
                bandwidth_out: 0,
                active_connections: 0,
                messages_processed: 0,
                error_count: 0,
                average_latency: Duration::ZERO,
            },
            specialized_content_count: 0,
            specialized_content_size: 0,
            specialization_match_rate: 0.0,
            cached_popular_content: 0,
            cache_hit_rate: 0.0,
            requests_served: 0,
            consensus_participation_rate: 0.0,
            specialization_score: 0.0,
            storage_efficiency: 0.0,
        };

        Ok(Self {
            config,
            node_id,
            keypair,
            status: Arc::new(RwLock::new(LightStorageStatus::Initializing)),
            storage_manager: Arc::new(Mutex::new(storage_manager)),
            local_archive_index: Arc::new(RwLock::new(HashMap::new())),
            popular_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(initial_metrics)),
            compiled_filters: Arc::new(RwLock::new(compiled_filters)),
            last_cache_optimization: Arc::new(Mutex::new(start_time)),
            start_time,
        })
    }

    /// Évalue si un contenu correspond à la spécialisation
    pub async fn evaluate_content_match(&self, metadata: &ContentMetadata) -> f64 {
        let filter = &self.config.content_filter;
        let mut score = 0.0;
        let mut total_criteria = 0.0;

        // Vérifie les types MIME
        if !filter.accepted_mime_types.is_empty() {
            total_criteria += 1.0;
            if filter.accepted_mime_types.contains(&metadata.content_type) {
                score += 1.0;
            }
        }

        // Vérifie la taille du fichier
        if let Some(min_size) = filter.min_file_size {
            total_criteria += 1.0;
            if metadata.size >= min_size {
                score += 1.0;
            }
        }

        if let Some(max_size) = filter.max_file_size {
            total_criteria += 1.0;
            if metadata.size <= max_size {
                score += 1.0;
            }
        }

        // Vérifie la popularité
        if let Some(min_popularity) = filter.min_popularity_score {
            total_criteria += 1.0;
            if metadata.popularity >= min_popularity {
                score += 1.0;
            }
        }

        // Vérifie la période temporelle
        if let Some(temporal_range) = &filter.temporal_range {
            total_criteria += 1.0;
            if metadata.created_at >= temporal_range.start_date 
                && metadata.created_at <= temporal_range.end_date {
                score += 1.0;
            }
        }

        // Score final normalisé
        if total_criteria > 0.0 {
            score / total_criteria
        } else {
            1.0 // Pas de critères = accepte tout
        }
    }

    /// Stocke du contenu spécialisé
    pub async fn store_specialized_content(
        &mut self,
        content_hash: Hash,
        data: &[u8],
        metadata: ContentMetadata,
    ) -> Result<SpecializedStorageResult> {
        // Évalue la correspondance avec la spécialisation
        let match_score = self.evaluate_content_match(&metadata).await;
        
        if match_score < 0.5 {
            return Ok(SpecializedStorageResult {
                content_hash,
                stored: false,
                match_score,
                reason: "Contenu ne correspond pas à la spécialisation".to_string(),
                storage_duration: Duration::ZERO,
            });
        }

        let start_time = SystemTime::now();

        // Stocke via le gestionnaire de stockage
        let storage_result = {
            let mut storage = self.storage_manager.lock().await;
            storage.store_content(&content_hash, data, metadata.clone()).await?
        };

        // Met à jour l'index local
        {
            let mut index = self.local_archive_index.write().await;
            index.insert(content_hash, ArchiveMetadata {
                content_hash,
                metadata: metadata.clone(),
                specialization_score: match_score,
                last_accessed: chrono::Utc::now(),
                access_count: 0,
                stored_locally: true,
            });
        }

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.specialized_content_count += 1;
            metrics.specialized_content_size += data.len() as u64;
            
            // Recalcule le taux de correspondance
            let total_content = metrics.specialized_content_count as f64;
            metrics.specialization_match_rate = 
                (metrics.specialization_match_rate * (total_content - 1.0) + match_score) / total_content;
        }

        Ok(SpecializedStorageResult {
            content_hash,
            stored: true,
            match_score,
            reason: "Contenu stocké avec succès".to_string(),
            storage_duration: start_time.elapsed().unwrap_or(Duration::ZERO),
        })
    }

    /// Récupère du contenu depuis l'index local ou le cache
    pub async fn retrieve_specialized_content(&self, content_hash: &Hash) -> Result<Vec<u8>> {
        // Vérifie d'abord le cache populaire
        {
            let mut cache = self.popular_cache.write().await;
            if let Some(cached) = cache.get_mut(content_hash) {
                cached.cache_hits += 1;
                cached.last_access = SystemTime::now();
                
                // Met à jour les métriques de cache
                {
                    let mut metrics = self.metrics.write().await;
                    metrics.requests_served += 1;
                }
                
                return Ok(cached.data.clone());
            }
        }

        // Vérifie l'index local
        {
            let mut index = self.local_archive_index.write().await;
            if let Some(archive_meta) = index.get_mut(content_hash) {
                archive_meta.access_count += 1;
                archive_meta.last_accessed = chrono::Utc::now();
            } else {
                return Err(crate::error::CoreError::NotFound {
                    message: "Archive non trouvée dans cet index spécialisé".to_string(),
                });
            }
        }

        // Récupère depuis le stockage
        let storage = self.storage_manager.lock().await;
        let data = storage.retrieve_content(content_hash).await?;

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.requests_served += 1;
        }

        Ok(data)
    }

    /// Met en cache du contenu populaire
    pub async fn cache_popular_content(
        &self,
        content_hash: Hash,
        data: Vec<u8>,
        metadata: ContentMetadata,
    ) -> Result<()> {
        let cache_size_limit = self.config.popular_cache_size;
        
        {
            let mut cache = self.popular_cache.write().await;
            
            // Vérifie la limite de taille du cache
            let current_size: u64 = cache.values()
                .map(|cached| cached.data.len() as u64)
                .sum();
            
            if current_size + data.len() as u64 > cache_size_limit {
                // Éviction LRU
                self.evict_least_recently_used(&mut cache).await;
            }
            
            cache.insert(content_hash, CachedArchive {
                content_hash,
                data,
                metadata,
                cached_at: SystemTime::now(),
                cache_hits: 0,
                last_access: SystemTime::now(),
            });
        }

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.cached_popular_content += 1;
        }

        Ok(())
    }

    /// Éviction LRU du cache
    async fn evict_least_recently_used(&self, cache: &mut HashMap<Hash, CachedArchive>) {
        if cache.is_empty() {
            return;
        }

        // Trouve l'entrée la moins récemment utilisée
        let mut oldest_hash = None;
        let mut oldest_time = SystemTime::now();

        for (hash, cached) in cache.iter() {
            if cached.last_access < oldest_time {
                oldest_time = cached.last_access;
                oldest_hash = Some(*hash);
            }
        }

        if let Some(hash) = oldest_hash {
            cache.remove(&hash);
            tracing::debug!("Éviction LRU du cache: {:?}", hash);
        }
    }

    /// Optimise le cache
    pub async fn optimize_cache(&self) -> Result<CacheOptimizationResult> {
        let optimization_start = SystemTime::now();
        let mut evicted_items = 0;
        let mut total_space_freed = 0u64;

        {
            let mut cache = self.popular_cache.write().await;
            let cache_size_before = cache.len();
            
            // Supprime les entrées anciennes et peu utilisées
            let now = SystemTime::now();
            let max_age = Duration::from_secs(86400 * 7); // 7 jours
            
            cache.retain(|_, cached| {
                let age = now.duration_since(cached.cached_at).unwrap_or(Duration::ZERO);
                let keep = age < max_age && cached.cache_hits > 0;
                
                if !keep {
                    evicted_items += 1;
                    total_space_freed += cached.data.len() as u64;
                }
                
                keep
            });
            
            tracing::info!(
                "Optimisation cache: {} entrées supprimées, {:.2} MB libérés",
                evicted_items,
                total_space_freed as f64 / (1024.0 * 1024.0)
            );
        }

        // Met à jour le timestamp d'optimisation
        {
            let mut last_opt = self.last_cache_optimization.lock().await;
            *last_opt = SystemTime::now();
        }

        Ok(CacheOptimizationResult {
            items_evicted: evicted_items,
            space_freed: total_space_freed,
            optimization_duration: optimization_start.elapsed().unwrap_or(Duration::ZERO),
        })
    }

    /// Synchronisation partielle sélective
    pub async fn selective_sync(&mut self) -> Result<PartialSyncResult> {
        if !self.config.partial_sync_config.enabled {
            return Ok(PartialSyncResult {
                blocks_synced: 0,
                content_discovered: 0,
                specialization_matches: 0,
                sync_duration: Duration::ZERO,
            });
        }

        {
            let mut status = self.status.write().await;
            *status = LightStorageStatus::SelectiveSync;
        }

        let sync_start = SystemTime::now();
        let mut blocks_synced = 0;
        let mut content_discovered = 0;
        let mut specialization_matches = 0;

        // Simulation de synchronisation sélective
        // Dans la réalité, on interrogerait le réseau pour du contenu correspondant à notre spécialisation
        
        {
            let mut status = self.status.write().await;
            *status = LightStorageStatus::Operational;
        }

        Ok(PartialSyncResult {
            blocks_synced,
            content_discovered,
            specialization_matches,
            sync_duration: sync_start.elapsed().unwrap_or(Duration::ZERO),
        })
    }

    /// Obtient les statistiques de spécialisation
    pub async fn get_specialization_stats(&self) -> SpecializationStats {
        let metrics = self.metrics.read().await;
        let index = self.local_archive_index.read().await;
        let cache = self.popular_cache.read().await;

        SpecializationStats {
            specialization_type: self.config.specialization.clone(),
            total_content: index.len() as u64,
            specialized_content: metrics.specialized_content_count,
            match_rate: metrics.specialization_match_rate,
            cache_size: cache.len() as u64,
            cache_hit_rate: metrics.cache_hit_rate,
            storage_efficiency: metrics.storage_efficiency,
            consensus_participation: self.config.consensus_participation,
        }
    }
}

#[async_trait]
impl Node for LightStorageNode {
    fn node_type(&self) -> NodeType {
        self.config.node_config.node_type.clone()
    }

    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn start(&mut self) -> Result<()> {
        tracing::info!("Démarrage du Light Storage Node: {:?}", self.node_id);

        {
            let mut status = self.status.write().await;
            *status = LightStorageStatus::ConfiguringSpecialization;
        }

        // Configure le gestionnaire de stockage
        {
            let mut storage = self.storage_manager.lock().await;
            let node_info = StorageNodeInfo {
                node_id: self.node_id.clone(),
                node_type: self.config.node_config.node_type.to_storage_node_type(),
                region: self.config.node_config.region.clone(),
                total_capacity: self.config.storage_capacity,
                used_capacity: 0,
                supported_storage_types: vec![StorageType::Hot, StorageType::Warm],
                available_bandwidth: 100_000_000, // 100 MB/s
                average_latency: 30, // ms
                reliability_score: 0.85,
                last_seen: chrono::Utc::now(),
                status: NodeStatus::Active,
            };
            
            storage.update_node_info(self.node_id.clone(), node_info).await?;
        }

        // Effectue une synchronisation sélective initiale
        self.selective_sync().await?;

        {
            let mut status = self.status.write().await;
            *status = LightStorageStatus::Operational;
        }

        tracing::info!("Light Storage Node démarré avec succès");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("Arrêt du Light Storage Node: {:?}", self.node_id);

        {
            let mut status = self.status.write().await;
            *status = LightStorageStatus::Stopping;
        }

        // Optimise le cache une dernière fois
        self.optimize_cache().await?;

        // Vide le cache
        {
            let mut cache = self.popular_cache.write().await;
            cache.clear();
        }

        tracing::info!("Light Storage Node arrêté");
        Ok(())
    }

    async fn health_check(&self) -> Result<NodeHealth> {
        let status = self.status.read().await;
        let metrics = self.metrics.read().await;

        let health_status = match *status {
            LightStorageStatus::Operational => {
                if metrics.specialization_match_rate < 0.3 {
                    HealthStatus::Warning
                } else {
                    HealthStatus::Healthy
                }
            },
            LightStorageStatus::SpecializationFailed => HealthStatus::Critical,
            _ => HealthStatus::Warning,
        };

        Ok(NodeHealth {
            status: health_status,
            uptime: self.start_time.elapsed().unwrap_or(Duration::ZERO),
            cpu_usage: metrics.general.cpu_usage,
            memory_usage: metrics.general.memory_usage,
            storage_usage: metrics.general.storage_usage,
            network_latency: metrics.general.average_latency,
            error_rate: if metrics.general.messages_processed > 0 {
                metrics.general.error_count as f64 / metrics.general.messages_processed as f64
            } else {
                0.0
            },
            last_check: SystemTime::now(),
        })
    }

    async fn get_metrics(&self) -> Result<Box<dyn NodeMetrics>> {
        let metrics = self.metrics.read().await;
        Ok(Box::new(metrics.clone()))
    }

    async fn handle_message(&mut self, message: NetworkMessage) -> Result<Option<NetworkMessage>> {
        {
            let mut metrics = self.metrics.write().await;
            metrics.general.messages_processed += 1;
        }

        match message.message_type {
            MessageType::Ping => {
                Ok(Some(NetworkMessage {
                    message_id: crate::crypto::compute_hash(
                        &message.message_id.as_bytes(),
                        crate::crypto::HashAlgorithm::Blake3
                    ),
                    sender: self.node_id.clone(),
                    recipient: Some(message.sender),
                    message_type: MessageType::Pong,
                    payload: Vec::new(),
                    timestamp: chrono::Utc::now(),
                    ttl: 60,
                }))
            },
            MessageType::ContentStore => {
                // Évalue si le contenu correspond à notre spécialisation
                // Dans la réalité, on déserialiserait le payload pour obtenir les métadonnées
                Ok(None)
            },
            MessageType::ContentRetrieve => {
                // Vérifie si nous avons le contenu spécialisé demandé
                Ok(None)
            },
            _ => Ok(None),
        }
    }

    async fn sync_with_network(&mut self) -> Result<()> {
        self.selective_sync().await?;
        Ok(())
    }

    async fn update_config(&mut self, config: super::NodeConfiguration) -> Result<()> {
        self.config.node_config = config;
        Ok(())
    }
}

/// Résultat du stockage spécialisé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializedStorageResult {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Contenu stocké avec succès
    pub stored: bool,
    /// Score de correspondance avec la spécialisation
    pub match_score: f64,
    /// Raison du résultat
    pub reason: String,
    /// Durée du stockage
    pub storage_duration: Duration,
}

/// Résultat de synchronisation partielle
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSyncResult {
    /// Blocs synchronisés
    pub blocks_synced: u64,
    /// Contenu découvert
    pub content_discovered: u64,
    /// Correspondances de spécialisation
    pub specialization_matches: u64,
    /// Durée de synchronisation
    pub sync_duration: Duration,
}

/// Résultat d'optimisation du cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheOptimizationResult {
    /// Éléments évincés
    pub items_evicted: u32,
    /// Espace libéré (bytes)
    pub space_freed: u64,
    /// Durée d'optimisation
    pub optimization_duration: Duration,
}

/// Statistiques de spécialisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecializationStats {
    /// Type de spécialisation
    pub specialization_type: StorageSpecialization,
    /// Contenu total dans l'index
    pub total_content: u64,
    /// Contenu spécialisé stocké
    pub specialized_content: u64,
    /// Taux de correspondance
    pub match_rate: f64,
    /// Taille du cache
    pub cache_size: u64,
    /// Taux de hit du cache
    pub cache_hit_rate: f64,
    /// Efficacité du stockage
    pub storage_efficiency: f64,
    /// Participation au consensus
    pub consensus_participation: bool,
}

impl LightStorageConfig {
    /// Valide la configuration
    pub fn validate(&self) -> Result<()> {
        if self.storage_capacity < 1_000_000_000_000 || self.storage_capacity > 10_000_000_000_000 {
            return Err(crate::error::CoreError::Validation {
                message: "Capacité Light Storage Node doit être entre 1TB et 10TB".to_string(),
            });
        }

        if self.replication_factor < 3 || self.replication_factor > 8 {
            return Err(crate::error::CoreError::Validation {
                message: "Facteur de réplication doit être entre 3 et 8".to_string(),
            });
        }

        if self.consensus_weight < 0.0 || self.consensus_weight > 1.0 {
            return Err(crate::error::CoreError::Validation {
                message: "Poids consensus doit être entre 0.0 et 1.0".to_string(),
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;
    // use crate::storage::StorageConfig;

    #[test]
    fn test_content_filter_default() {
        let filter = ContentFilter::default();
        assert_eq!(filter.specialization, StorageSpecialization::ContentType);
        assert!(filter.accepted_mime_types.contains(&"text/html".to_string()));
    }

    #[test]
    fn test_specialization_types() {
        let domain_spec = StorageSpecialization::Domain;
        let content_spec = StorageSpecialization::ContentType;
        let custom_spec = StorageSpecialization::Custom {
            name: "PDFs académiques".to_string(),
            description: "Spécialisation pour les PDFs d'articles académiques".to_string(),
        };

        assert_eq!(domain_spec, StorageSpecialization::Domain);
        assert_ne!(domain_spec, content_spec);
        
        if let StorageSpecialization::Custom { name, .. } = custom_spec {
            assert_eq!(name, "PDFs académiques");
        }
    }

    #[tokio::test]
    async fn test_light_storage_node_creation() {
        let config = LightStorageConfig::default();
        let keypair = generate_keypair().unwrap();
        
        let storage_config = StorageConfig::default();
        let storage_manager = StorageManager::new(
            storage_config,
            crate::storage::manager::StoragePolicy {
                default_replication_strategy: crate::storage::replication::ReplicationStrategy::Fixed { 
                    replica_count: 3 
                },
                node_preferences: HashMap::new(),
                retention_policies: Vec::new(),
                alert_thresholds: crate::storage::manager::AlertThresholds::default(),
            }
        ).await.unwrap();

        let node = LightStorageNode::new(config, keypair, storage_manager);
        assert!(node.is_ok());
    }

    #[test]
    fn test_light_storage_config_validation() {
        let mut config = LightStorageConfig::default();
        assert!(config.validate().is_ok());

        // Test capacité invalide
        config.storage_capacity = 500_000_000_000; // 500GB trop peu
        assert!(config.validate().is_err());

        config.storage_capacity = 15_000_000_000_000; // 15TB trop
        assert!(config.validate().is_err());

        // Test facteur de réplication invalide
        config.storage_capacity = 5_000_000_000_000; // 5TB valide
        config.replication_factor = 2; // Trop peu
        assert!(config.validate().is_err());

        config.replication_factor = 10; // Trop
        assert!(config.validate().is_err());

        config.replication_factor = 5; // Valide
        assert!(config.validate().is_ok());
    }
}