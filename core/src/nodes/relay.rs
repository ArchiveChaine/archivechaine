//! Implémentation du Relay Node
//!
//! Les Relay Nodes facilitent les communications P2P dans le réseau ArchiveChain :
//! - Routage optimisé des messages réseau
//! - Facilitation des connexions entre nœuds
//! - Participation au consensus sans stockage massif
//! - Monitoring et métriques réseau en temps réel
//! - Support de la découverte de nœuds

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::net::SocketAddr;
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;

use crate::crypto::{Hash, PublicKey, PrivateKey, Signature};
use crate::consensus::NodeId;
use crate::error::Result;
use super::{
    Node, NodeType, NodeConfiguration, NetworkMessage, MessageType,
    NodeHealth, NodeMetrics, GeneralNodeMetrics, HealthStatus
};

/// Configuration spécifique aux Relay Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayNodeConfig {
    /// Configuration générale du nœud
    pub node_config: NodeConfiguration,
    /// Capacité de bande passante (bytes/sec)
    pub bandwidth_capacity: u64,
    /// Nombre maximum de connexions simultanées
    pub max_connections: u32,
    /// Participation au consensus
    pub consensus_participation: bool,
    /// Poids dans le consensus (réduit)
    pub consensus_weight: f64,
    /// Configuration du routage
    pub routing_config: RoutingConfiguration,
    /// Configuration de la découverte de nœuds
    pub discovery_config: DiscoveryConfiguration,
    /// Configuration du monitoring
    pub monitoring_config: MonitoringConfiguration,
    /// Taille du cache de stockage minimal
    pub minimal_cache_size: u64,
}

/// Configuration du routage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfiguration {
    /// Algorithme de routage
    pub routing_algorithm: RoutingAlgorithm,
    /// TTL maximum pour les messages
    pub max_message_ttl: u32,
    /// Taille maximum de la file d'attente
    pub max_queue_size: usize,
    /// Timeout de routage
    pub routing_timeout: Duration,
    /// Nombre maximum de sauts
    pub max_hops: u32,
    /// Table de routage activée
    pub enable_routing_table: bool,
}

/// Algorithmes de routage supportés
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingAlgorithm {
    /// Routage par inondation (flooding)
    Flooding,
    /// Routage basé sur la distance
    DistanceVector,
    /// Routage par état de liens
    LinkState,
    /// Routage adaptatif
    Adaptive,
}

/// Configuration de la découverte de nœuds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfiguration {
    /// Découverte activée
    pub enabled: bool,
    /// Intervalle de découverte
    pub discovery_interval: Duration,
    /// Ping interval vers les pairs
    pub ping_interval: Duration,
    /// Timeout de ping
    pub ping_timeout: Duration,
    /// Nombre maximum de pairs à découvrir
    pub max_discovery_peers: u32,
}

/// Configuration du monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfiguration {
    /// Monitoring activé
    pub enabled: bool,
    /// Intervalle de collecte des métriques
    pub metrics_collection_interval: Duration,
    /// Historique des métriques (en secondes)
    pub metrics_history_duration: Duration,
    /// Seuils d'alerte
    pub alert_thresholds: RelayAlertThresholds,
}

/// Seuils d'alerte pour les Relay Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayAlertThresholds {
    /// Seuil de latence critique (ms)
    pub critical_latency_threshold: u64,
    /// Seuil d'utilisation de bande passante (%)
    pub bandwidth_usage_threshold: f64,
    /// Seuil de connexions critiques
    pub critical_connections_threshold: u32,
    /// Seuil de perte de paquets (%)
    pub packet_loss_threshold: f64,
}

/// Statut d'un Relay Node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelayNodeStatus {
    /// Initialisation en cours
    Initializing,
    /// Configuration du routage
    ConfiguringRouting,
    /// Découverte de pairs
    DiscoveringPeers,
    /// Opérationnel
    Operational,
    /// Optimisation du routage
    OptimizingRouting,
    /// Surcharge détectée
    Overloaded,
    /// Problème de connectivité
    ConnectivityIssue,
    /// Arrêt en cours
    Stopping,
}

/// Informations de connexion P2P
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerConnection {
    /// Identifiant du pair
    pub peer_id: NodeId,
    /// Adresse réseau
    pub address: SocketAddr,
    /// Statut de la connexion
    pub status: ConnectionStatus,
    /// Latence mesurée
    pub latency: Duration,
    /// Bande passante disponible
    pub available_bandwidth: u64,
    /// Timestamp de dernière activité
    pub last_activity: SystemTime,
    /// Nombre de messages routés
    pub messages_routed: u64,
    /// Score de fiabilité
    pub reliability_score: f64,
}

/// Statut d'une connexion
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConnectionStatus {
    /// Connexion en cours d'établissement
    Connecting,
    /// Connexion active
    Connected,
    /// Connexion authentifiée
    Authenticated,
    /// Connexion en cours de fermeture
    Disconnecting,
    /// Connexion fermée
    Disconnected,
    /// Erreur de connexion
    Error,
}

/// Routeur de messages
#[derive(Debug)]
pub struct MessageRouter {
    /// Table de routage
    routing_table: Arc<RwLock<HashMap<NodeId, RouteEntry>>>,
    /// File d'attente des messages
    message_queue: Arc<Mutex<VecDeque<QueuedMessage>>>,
    /// Cache des messages récents (pour éviter les boucles)
    message_cache: Arc<RwLock<HashSet<Hash>>>,
    /// Configuration de routage
    config: RoutingConfiguration,
    /// Métriques de routage
    metrics: Arc<RwLock<RoutingMetrics>>,
}

/// Entrée de la table de routage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    /// Nœud de destination
    pub destination: NodeId,
    /// Prochain saut
    pub next_hop: NodeId,
    /// Coût de la route
    pub cost: u32,
    /// Nombre de sauts
    pub hops: u32,
    /// Timestamp de dernière mise à jour
    pub last_updated: SystemTime,
    /// Fiabilité de la route
    pub reliability: f64,
}

/// Message en file d'attente
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    /// Message à router
    pub message: NetworkMessage,
    /// Tentatives de routage
    pub retry_count: u32,
    /// Timestamp d'ajout à la file
    pub queued_at: SystemTime,
    /// Priorité du message
    pub priority: MessagePriority,
}

/// Priorité des messages
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MessagePriority {
    /// Priorité basse
    Low,
    /// Priorité normale
    Normal,
    /// Priorité haute
    High,
    /// Priorité critique
    Critical,
}

/// Métriques réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Métriques générales
    pub general: GeneralNodeMetrics,
    /// Nombre de connexions actives
    pub active_connections: u32,
    /// Bande passante utilisée entrante (bytes/sec)
    pub bandwidth_in_used: u64,
    /// Bande passante utilisée sortante (bytes/sec)
    pub bandwidth_out_used: u64,
    /// Messages routés avec succès
    pub messages_routed_success: u64,
    /// Messages échoués
    pub messages_routed_failed: u64,
    /// Latence moyenne de routage
    pub average_routing_latency: Duration,
    /// Taux de perte de paquets
    pub packet_loss_rate: f64,
    /// Nombre de pairs découverts
    pub peers_discovered: u32,
    /// Score de connectivité
    pub connectivity_score: f64,
}

/// Métriques de routage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMetrics {
    /// Messages traités
    pub messages_processed: u64,
    /// Messages routés avec succès
    pub messages_routed: u64,
    /// Messages abandonnés
    pub messages_dropped: u64,
    /// Taille moyenne de la file d'attente
    pub average_queue_size: f64,
    /// Temps moyen de traitement
    pub average_processing_time: Duration,
    /// Entrées dans la table de routage
    pub routing_table_size: usize,
}

impl NodeMetrics for NetworkMetrics {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn general_metrics(&self) -> GeneralNodeMetrics {
        self.general.clone()
    }
}

/// Relay Node - Nœud de relais pour les communications P2P
pub struct RelayNode {
    /// Configuration du nœud
    config: RelayNodeConfig,
    /// Identifiant du nœud
    node_id: NodeId,
    /// Clés cryptographiques
    keypair: (PublicKey, PrivateKey),
    /// Statut actuel
    status: Arc<RwLock<RelayNodeStatus>>,
    /// Connexions P2P actives
    peer_connections: Arc<RwLock<HashMap<NodeId, PeerConnection>>>,
    /// Routeur de messages
    message_router: Arc<Mutex<MessageRouter>>,
    /// Métriques réseau
    metrics: Arc<RwLock<NetworkMetrics>>,
    /// Cache minimal pour les métadonnées
    minimal_cache: Arc<RwLock<HashMap<Hash, CachedMetadata>>>,
    /// Heure de démarrage
    start_time: SystemTime,
}

/// Métadonnées en cache minimal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMetadata {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Nœuds qui possèdent ce contenu
    pub storage_nodes: Vec<NodeId>,
    /// Timestamp de mise en cache
    pub cached_at: SystemTime,
    /// Nombre d'accès
    pub access_count: u64,
}

impl Default for RelayNodeConfig {
    fn default() -> Self {
        Self {
            node_config: NodeConfiguration {
                node_id: NodeId::from(Hash::zero()),
                node_type: NodeType::Relay {
                    bandwidth_capacity: 1_000_000_000, // 1 GB/s
                    max_connections: 1000,
                },
                region: "us-east-1".to_string(),
                listen_address: "0.0.0.0".to_string(),
                listen_port: 8082,
                bootstrap_nodes: Vec::new(),
                storage_config: None,
                network_config: super::NetworkConfiguration::default(),
                security_config: super::SecurityConfiguration::default(),
            },
            bandwidth_capacity: 1_000_000_000, // 1 GB/s
            max_connections: 1000,
            consensus_participation: true,
            consensus_weight: 0.3,
            routing_config: RoutingConfiguration::default(),
            discovery_config: DiscoveryConfiguration::default(),
            monitoring_config: MonitoringConfiguration::default(),
            minimal_cache_size: 1_000_000_000, // 1GB
        }
    }
}

impl Default for RoutingConfiguration {
    fn default() -> Self {
        Self {
            routing_algorithm: RoutingAlgorithm::Adaptive,
            max_message_ttl: 64,
            max_queue_size: 10000,
            routing_timeout: Duration::from_secs(30),
            max_hops: 16,
            enable_routing_table: true,
        }
    }
}

impl Default for DiscoveryConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            discovery_interval: Duration::from_secs(60),
            ping_interval: Duration::from_secs(30),
            ping_timeout: Duration::from_secs(5),
            max_discovery_peers: 100,
        }
    }
}

impl Default for MonitoringConfiguration {
    fn default() -> Self {
        Self {
            enabled: true,
            metrics_collection_interval: Duration::from_secs(10),
            metrics_history_duration: Duration::from_secs(3600), // 1 heure
            alert_thresholds: RelayAlertThresholds::default(),
        }
    }
}

impl Default for RelayAlertThresholds {
    fn default() -> Self {
        Self {
            critical_latency_threshold: 1000, // 1 seconde
            bandwidth_usage_threshold: 80.0, // 80%
            critical_connections_threshold: 10, // Minimum 10 connexions
            packet_loss_threshold: 5.0, // 5%
        }
    }
}

impl MessageRouter {
    /// Crée un nouveau routeur de messages
    pub fn new(config: RoutingConfiguration) -> Self {
        Self {
            routing_table: Arc::new(RwLock::new(HashMap::new())),
            message_queue: Arc::new(Mutex::new(VecDeque::new())),
            message_cache: Arc::new(RwLock::new(HashSet::new())),
            config,
            metrics: Arc::new(RwLock::new(RoutingMetrics {
                messages_processed: 0,
                messages_routed: 0,
                messages_dropped: 0,
                average_queue_size: 0.0,
                average_processing_time: Duration::ZERO,
                routing_table_size: 0,
            })),
        }
    }

    /// Route un message vers sa destination
    pub async fn route_message(&self, message: NetworkMessage) -> Result<RoutingResult> {
        let start_time = SystemTime::now();

        // Vérifie si le message a déjà été traité (évite les boucles)
        {
            let mut cache = self.message_cache.write().await;
            if cache.contains(&message.message_id) {
                return Ok(RoutingResult::Duplicate);
            }
            cache.insert(message.message_id);
            
            // Nettoie le cache si trop grand
            if cache.len() > 10000 {
                cache.clear();
            }
        }

        // Vérifie le TTL
        if message.ttl == 0 {
            return Ok(RoutingResult::Expired);
        }

        // Ajoute à la file d'attente
        {
            let mut queue = self.message_queue.lock().await;
            if queue.len() >= self.config.max_queue_size {
                return Ok(RoutingResult::QueueFull);
            }

            queue.push_back(QueuedMessage {
                message,
                retry_count: 0,
                queued_at: SystemTime::now(),
                priority: MessagePriority::Normal,
            });
        }

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.messages_processed += 1;
            let processing_time = start_time.elapsed().unwrap_or(Duration::ZERO);
            metrics.average_processing_time = 
                (metrics.average_processing_time + processing_time) / 2;
        }

        Ok(RoutingResult::Queued)
    }

    /// Traite la file d'attente des messages
    pub async fn process_queue(&self, peer_connections: &HashMap<NodeId, PeerConnection>) -> Result<u32> {
        let mut processed = 0;
        let mut queue = self.message_queue.lock().await;
        
        while let Some(queued_message) = queue.pop_front() {
            let route_result = self.find_route(&queued_message.message, peer_connections).await;
            
            match route_result {
                Some(next_hop) => {
                    // Simule l'envoi du message
                    tracing::debug!("Routage message {:?} vers {:?}", 
                        queued_message.message.message_id, next_hop);
                    
                    let mut metrics = self.metrics.write().await;
                    metrics.messages_routed += 1;
                    processed += 1;
                },
                None => {
                    // Aucune route trouvée
                    if queued_message.retry_count < 3 {
                        // Remet en queue pour retry
                        let mut retry_message = queued_message;
                        retry_message.retry_count += 1;
                        queue.push_back(retry_message);
                    } else {
                        // Abandonne le message
                        let mut metrics = self.metrics.write().await;
                        metrics.messages_dropped += 1;
                    }
                }
            }
        }

        Ok(processed)
    }

    /// Trouve la meilleure route pour un message
    async fn find_route(&self, message: &NetworkMessage, peer_connections: &HashMap<NodeId, PeerConnection>) -> Option<NodeId> {
        match self.config.routing_algorithm {
            RoutingAlgorithm::Flooding => {
                // Envoie à tous les pairs connectés
                peer_connections.keys().next().cloned()
            },
            RoutingAlgorithm::DistanceVector => {
                // Utilise la table de routage pour trouver la meilleure route
                let routing_table = self.routing_table.read().await;
                if let Some(recipient) = &message.recipient {
                    routing_table.get(recipient).map(|entry| entry.next_hop.clone())
                } else {
                    peer_connections.keys().next().cloned()
                }
            },
            RoutingAlgorithm::Adaptive => {
                // Sélectionne la connexion avec la meilleure latence
                peer_connections.values()
                    .filter(|conn| conn.status == ConnectionStatus::Connected)
                    .min_by_key(|conn| conn.latency)
                    .map(|conn| conn.peer_id.clone())
            },
            _ => peer_connections.keys().next().cloned(),
        }
    }

    /// Met à jour la table de routage
    pub async fn update_routing_table(&self, destination: NodeId, next_hop: NodeId, cost: u32) {
        if !self.config.enable_routing_table {
            return;
        }

        let mut routing_table = self.routing_table.write().await;
        routing_table.insert(destination, RouteEntry {
            destination: destination.clone(),
            next_hop,
            cost,
            hops: cost, // Simplification : coût = nombre de sauts
            last_updated: SystemTime::now(),
            reliability: 0.9, // Score par défaut
        });

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.routing_table_size = routing_table.len();
        }
    }
}

impl RelayNode {
    /// Crée une nouvelle instance de Relay Node
    pub fn new(
        config: RelayNodeConfig,
        keypair: (PublicKey, PrivateKey),
    ) -> Result<Self> {
        // Valide la configuration
        config.validate()?;

        let node_id = config.node_config.node_id.clone();
        let start_time = SystemTime::now();

        let message_router = MessageRouter::new(config.routing_config.clone());

        let initial_metrics = NetworkMetrics {
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
            active_connections: 0,
            bandwidth_in_used: 0,
            bandwidth_out_used: 0,
            messages_routed_success: 0,
            messages_routed_failed: 0,
            average_routing_latency: Duration::ZERO,
            packet_loss_rate: 0.0,
            peers_discovered: 0,
            connectivity_score: 0.0,
        };

        Ok(Self {
            config,
            node_id,
            keypair,
            status: Arc::new(RwLock::new(RelayNodeStatus::Initializing)),
            peer_connections: Arc::new(RwLock::new(HashMap::new())),
            message_router: Arc::new(Mutex::new(message_router)),
            metrics: Arc::new(RwLock::new(initial_metrics)),
            minimal_cache: Arc::new(RwLock::new(HashMap::new())),
            start_time,
        })
    }

    /// Ajoute une connexion P2P
    pub async fn add_peer_connection(&self, peer_connection: PeerConnection) -> Result<()> {
        {
            let mut connections = self.peer_connections.write().await;
            connections.insert(peer_connection.peer_id.clone(), peer_connection);
        }

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            let connections = self.peer_connections.read().await;
            metrics.active_connections = connections.len() as u32;
        }

        Ok(())
    }

    /// Supprime une connexion P2P
    pub async fn remove_peer_connection(&self, peer_id: &NodeId) -> Result<()> {
        {
            let mut connections = self.peer_connections.write().await;
            connections.remove(peer_id);
        }

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            let connections = self.peer_connections.read().await;
            metrics.active_connections = connections.len() as u32;
        }

        Ok(())
    }

    /// Découvre de nouveaux pairs
    pub async fn discover_peers(&self) -> Result<PeerDiscoveryResult> {
        if !self.config.discovery_config.enabled {
            return Ok(PeerDiscoveryResult {
                peers_discovered: 0,
                peers_connected: 0,
                discovery_duration: Duration::ZERO,
            });
        }

        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::DiscoveringPeers;
        }

        let discovery_start = SystemTime::now();
        let mut peers_discovered = 0;
        let mut peers_connected = 0;

        // Simulation de découverte de pairs
        // Dans la réalité, on utiliserait des mécanismes comme mDNS, DHT, etc.

        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::Operational;
        }

        {
            let mut metrics = self.metrics.write().await;
            metrics.peers_discovered += peers_discovered;
        }

        Ok(PeerDiscoveryResult {
            peers_discovered,
            peers_connected,
            discovery_duration: discovery_start.elapsed().unwrap_or(Duration::ZERO),
        })
    }

    /// Ping vers un pair pour mesurer la latence
    pub async fn ping_peer(&self, peer_id: &NodeId) -> Result<Duration> {
        let ping_start = SystemTime::now();

        // Simulation de ping
        // Dans la réalité, on enverrait un message ping et attendrait le pong
        tokio::time::sleep(Duration::from_millis(10)).await;

        let latency = ping_start.elapsed().unwrap_or(Duration::ZERO);

        // Met à jour la latence dans la connexion
        {
            let mut connections = self.peer_connections.write().await;
            if let Some(connection) = connections.get_mut(peer_id) {
                connection.latency = latency;
                connection.last_activity = SystemTime::now();
            }
        }

        Ok(latency)
    }

    /// Traite les messages en file d'attente
    pub async fn process_message_queue(&self) -> Result<u32> {
        let connections = self.peer_connections.read().await;
        let router = self.message_router.lock().await;
        router.process_queue(&connections).await
    }

    /// Optimise le routage
    pub async fn optimize_routing(&self) -> Result<RoutingOptimizationResult> {
        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::OptimizingRouting;
        }

        let optimization_start = SystemTime::now();
        let mut routes_optimized = 0;

        // Optimise les routes basées sur les métriques de latence
        {
            let connections = self.peer_connections.read().await;
            let router = self.message_router.lock().await;
            
            for (peer_id, connection) in connections.iter() {
                if connection.status == ConnectionStatus::Connected {
                    router.update_routing_table(
                        peer_id.clone(),
                        peer_id.clone(),
                        connection.latency.as_millis() as u32,
                    ).await;
                    routes_optimized += 1;
                }
            }
        }

        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::Operational;
        }

        Ok(RoutingOptimizationResult {
            routes_optimized,
            optimization_duration: optimization_start.elapsed().unwrap_or(Duration::ZERO),
        })
    }

    /// Obtient les statistiques de connectivité
    pub async fn get_connectivity_stats(&self) -> ConnectivityStats {
        let connections = self.peer_connections.read().await;
        let metrics = self.metrics.read().await;

        let connected_peers = connections.values()
            .filter(|conn| conn.status == ConnectionStatus::Connected)
            .count() as u32;

        let average_latency = if !connections.is_empty() {
            let total_latency: Duration = connections.values()
                .map(|conn| conn.latency)
                .sum();
            total_latency / connections.len() as u32
        } else {
            Duration::ZERO
        };

        ConnectivityStats {
            total_connections: connections.len() as u32,
            connected_peers,
            average_latency,
            bandwidth_utilization: (metrics.bandwidth_in_used + metrics.bandwidth_out_used) as f64 
                / self.config.bandwidth_capacity as f64,
            messages_routed: metrics.messages_routed_success,
            routing_errors: metrics.messages_routed_failed,
        }
    }
}

#[async_trait]
impl Node for RelayNode {
    fn node_type(&self) -> NodeType {
        self.config.node_config.node_type.clone()
    }

    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn start(&mut self) -> Result<()> {
        tracing::info!("Démarrage du Relay Node: {:?}", self.node_id);

        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::ConfiguringRouting;
        }

        // Démarre la découverte de pairs
        self.discover_peers().await?;

        // Optimise le routage initial
        self.optimize_routing().await?;

        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::Operational;
        }

        tracing::info!("Relay Node démarré avec succès");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("Arrêt du Relay Node: {:?}", self.node_id);

        {
            let mut status = self.status.write().await;
            *status = RelayNodeStatus::Stopping;
        }

        // Ferme toutes les connexions
        {
            let mut connections = self.peer_connections.write().await;
            for (_, mut connection) in connections.iter_mut() {
                connection.status = ConnectionStatus::Disconnected;
            }
            connections.clear();
        }

        // Vide le cache
        {
            let mut cache = self.minimal_cache.write().await;
            cache.clear();
        }

        tracing::info!("Relay Node arrêté");
        Ok(())
    }

    async fn health_check(&self) -> Result<NodeHealth> {
        let status = self.status.read().await;
        let metrics = self.metrics.read().await;
        let connections = self.peer_connections.read().await;

        let connected_peers = connections.values()
            .filter(|conn| conn.status == ConnectionStatus::Connected)
            .count() as u32;

        let health_status = match *status {
            RelayNodeStatus::Operational => {
                if connected_peers < self.config.monitoring_config.alert_thresholds.critical_connections_threshold {
                    HealthStatus::Warning
                } else {
                    HealthStatus::Healthy
                }
            },
            RelayNodeStatus::Overloaded | RelayNodeStatus::ConnectivityIssue => HealthStatus::Critical,
            _ => HealthStatus::Warning,
        };

        Ok(NodeHealth {
            status: health_status,
            uptime: self.start_time.elapsed().unwrap_or(Duration::ZERO),
            cpu_usage: metrics.general.cpu_usage,
            memory_usage: metrics.general.memory_usage,
            storage_usage: metrics.general.storage_usage,
            network_latency: metrics.average_routing_latency,
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
                // Mesure la latence et répond avec un Pong
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
            MessageType::NodeDiscovery => {
                // Traite une demande de découverte de nœud
                self.discover_peers().await?;
                Ok(None)
            },
            _ => {
                // Route le message vers sa destination
                let router = self.message_router.lock().await;
                router.route_message(message).await?;
                Ok(None)
            }
        }
    }

    async fn sync_with_network(&mut self) -> Result<()> {
        // Traite la file d'attente des messages
        self.process_message_queue().await?;
        
        // Découvre de nouveaux pairs
        self.discover_peers().await?;
        
        Ok(())
    }

    async fn update_config(&mut self, config: super::NodeConfiguration) -> Result<()> {
        self.config.node_config = config;
        Ok(())
    }
}

/// Résultat de routage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoutingResult {
    /// Message mis en file d'attente
    Queued,
    /// Message routé avec succès
    Routed,
    /// Message dupliqué (déjà traité)
    Duplicate,
    /// Message expiré (TTL = 0)
    Expired,
    /// File d'attente pleine
    QueueFull,
    /// Aucune route trouvée
    NoRoute,
}

/// Résultat de découverte de pairs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerDiscoveryResult {
    /// Pairs découverts
    pub peers_discovered: u32,
    /// Pairs connectés
    pub peers_connected: u32,
    /// Durée de découverte
    pub discovery_duration: Duration,
}

/// Résultat d'optimisation du routage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingOptimizationResult {
    /// Routes optimisées
    pub routes_optimized: u32,
    /// Durée d'optimisation
    pub optimization_duration: Duration,
}

/// Statistiques de connectivité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectivityStats {
    /// Nombre total de connexions
    pub total_connections: u32,
    /// Pairs connectés
    pub connected_peers: u32,
    /// Latence moyenne
    pub average_latency: Duration,
    /// Utilisation de la bande passante (0.0-1.0)
    pub bandwidth_utilization: f64,
    /// Messages routés
    pub messages_routed: u64,
    /// Erreurs de routage
    pub routing_errors: u64,
}

impl RelayNodeConfig {
    /// Valide la configuration
    pub fn validate(&self) -> Result<()> {
        if self.max_connections == 0 {
            return Err(crate::error::CoreError::Validation {
                message: "Nombre maximum de connexions doit être supérieur à 0".to_string(),
            });
        }

        if self.bandwidth_capacity == 0 {
            return Err(crate::error::CoreError::Validation {
                message: "Capacité de bande passante doit être supérieure à 0".to_string(),
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

    #[test]
    fn test_relay_node_config_validation() {
        let mut config = RelayNodeConfig::default();
        assert!(config.validate().is_ok());

        // Test max_connections invalide
        config.max_connections = 0;
        assert!(config.validate().is_err());

        // Test bandwidth_capacity invalide
        config.max_connections = 100;
        config.bandwidth_capacity = 0;
        assert!(config.validate().is_err());

        // Test consensus_weight invalide
        config.bandwidth_capacity = 1_000_000;
        config.consensus_weight = 1.5; // > 1.0
        assert!(config.validate().is_err());

        config.consensus_weight = 0.3; // Valide
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_relay_node_creation() {
        let config = RelayNodeConfig::default();
        let keypair = generate_keypair().unwrap();

        let node = RelayNode::new(config, keypair);
        assert!(node.is_ok());
    }

    #[tokio::test]
    async fn test_message_router() {
        let config = RoutingConfiguration::default();
        let router = MessageRouter::new(config);

        let message = NetworkMessage {
            message_id: Hash::zero(),
            sender: NodeId::from(Hash::zero()),
            recipient: None,
            message_type: MessageType::Ping,
            payload: Vec::new(),
            timestamp: chrono::Utc::now(),
            ttl: 60,
        };

        let result = router.route_message(message).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), RoutingResult::Queued);
    }

    #[test]
    fn test_routing_algorithms() {
        assert_eq!(RoutingAlgorithm::Flooding, RoutingAlgorithm::Flooding);
        assert_ne!(RoutingAlgorithm::Flooding, RoutingAlgorithm::Adaptive);
    }

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus::Connected;
        assert_eq!(status, ConnectionStatus::Connected);
        assert_ne!(status, ConnectionStatus::Disconnected);
    }
}