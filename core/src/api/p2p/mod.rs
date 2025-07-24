//! Client P2P pour ArchiveChain
//!
//! Implémente la communication peer-to-peer entre nœuds ArchiveChain,
//! incluant la découverte de pairs, la synchronisation et le gossip.

pub mod client;
pub mod discovery;
pub mod gossip;
pub mod sync;
pub mod messages;

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::{ApiResult, server::ServerState};

// Re-exports
pub use client::*;
pub use discovery::*;
pub use gossip::*;
pub use sync::*;
pub use messages::*;

/// Configuration P2P
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PConfig {
    /// Port d'écoute P2P
    pub listen_port: u16,
    /// Adresse d'écoute (0.0.0.0 pour toutes les interfaces)
    pub listen_addr: String,
    /// Nombre maximum de pairs connectés
    pub max_peers: usize,
    /// Nombre minimum de pairs requis
    pub min_peers: usize,
    /// Timeout de connexion (en secondes)
    pub connection_timeout: u64,
    /// Intervalle de ping (en secondes)
    pub ping_interval: u64,
    /// Timeout pour les requêtes (en secondes)
    pub request_timeout: u64,
    /// Liste des nœuds bootstrap
    pub bootstrap_nodes: Vec<String>,
    /// Active le protocole de découverte automatique
    pub enable_discovery: bool,
    /// Intervalle de découverte (en secondes)
    pub discovery_interval: u64,
    /// Taille maximum des messages
    pub max_message_size: usize,
    /// Buffer size pour les messages
    pub message_buffer_size: usize,
    /// Active la compression des messages
    pub enable_compression: bool,
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            listen_port: 8000,
            listen_addr: "0.0.0.0".to_string(),
            max_peers: 50,
            min_peers: 3,
            connection_timeout: 10,
            ping_interval: 30,
            request_timeout: 30,
            bootstrap_nodes: vec![],
            enable_discovery: true,
            discovery_interval: 60,
            max_message_size: 1024 * 1024, // 1MB
            message_buffer_size: 1000,
            enable_compression: true,
        }
    }
}

/// Informations sur un pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfo {
    /// ID unique du pair
    pub peer_id: String,
    /// Adresse réseau
    pub addr: SocketAddr,
    /// Version du protocole
    pub protocol_version: String,
    /// Version du client
    pub client_version: String,
    /// Hauteur de bloc actuelle
    pub block_height: u64,
    /// Hash du meilleur bloc
    pub best_block_hash: String,
    /// Latence moyenne
    pub latency_ms: u64,
    /// Heure de dernière activité
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Statut de la connexion
    pub status: PeerStatus,
    /// Région géographique
    pub region: Option<String>,
    /// Capacités supportées
    pub capabilities: HashSet<String>,
}

/// Statut d'un pair
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerStatus {
    Connecting,
    Connected,
    Disconnected,
    Banned,
    Syncing,
}

/// Gestionnaire P2P principal
#[derive(Clone)]
pub struct P2PManager {
    /// Configuration P2P
    config: P2PConfig,
    /// État du serveur
    server_state: ServerState,
    /// Client P2P
    client: Arc<P2PClient>,
    /// Service de découverte
    discovery: Arc<DiscoveryService>,
    /// Service de gossip
    gossip: Arc<GossipService>,
    /// Service de synchronisation
    sync: Arc<SyncService>,
    /// Pairs connectés
    peers: Arc<RwLock<HashMap<String, PeerInfo>>>,
    /// Statistiques P2P
    stats: Arc<RwLock<P2PStats>>,
}

/// Statistiques P2P
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct P2PStats {
    /// Nombre total de pairs connectés
    pub connected_peers: usize,
    /// Nombre de pairs actifs (récente activité)
    pub active_peers: usize,
    /// Messages envoyés
    pub messages_sent: u64,
    /// Messages reçus
    pub messages_received: u64,
    /// Bytes envoyés
    pub bytes_sent: u64,
    /// Bytes reçus
    pub bytes_received: u64,
    /// Connexions établies
    pub connections_established: u64,
    /// Connexions fermées
    pub connections_closed: u64,
    /// Erreurs de connexion
    pub connection_errors: u64,
    /// Temps de fonctionnement
    pub uptime_seconds: u64,
}

impl P2PManager {
    /// Crée un nouveau gestionnaire P2P
    pub async fn new(config: P2PConfig, server_state: ServerState) -> ApiResult<Self> {
        let client = Arc::new(P2PClient::new(config.clone()).await?);
        let discovery = Arc::new(DiscoveryService::new(config.clone()));
        let gossip = Arc::new(GossipService::new(config.clone()));
        let sync_service = Arc::new(SyncService::new(config.clone(), server_state.blockchain.clone()));

        Ok(Self {
            config,
            server_state,
            client,
            discovery,
            gossip,
            sync: sync_service,
            peers: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(P2PStats::default())),
        })
    }

    /// Démarre le gestionnaire P2P
    pub async fn start(&self) -> ApiResult<()> {
        tracing::info!("Starting P2P manager on port {}", self.config.listen_port);

        // Démarre le client P2P
        self.client.start().await?;

        // Démarre les services
        if self.config.enable_discovery {
            self.discovery.start().await?;
        }
        self.gossip.start().await?;
        self.sync.start().await?;

        // Connecte aux nœuds bootstrap
        self.connect_bootstrap_nodes().await?;

        // Démarre les tâches de maintenance
        self.start_maintenance_tasks().await;

        tracing::info!("P2P manager started successfully");
        Ok(())
    }

    /// Arrête le gestionnaire P2P
    pub async fn stop(&self) -> ApiResult<()> {
        tracing::info!("Stopping P2P manager");

        // Arrête les services
        self.sync.stop().await?;
        self.gossip.stop().await?;
        if self.config.enable_discovery {
            self.discovery.stop().await?;
        }

        // Arrête le client
        self.client.stop().await?;

        tracing::info!("P2P manager stopped");
        Ok(())
    }

    /// Connecte aux nœuds bootstrap
    async fn connect_bootstrap_nodes(&self) -> ApiResult<()> {
        for bootstrap_addr in &self.config.bootstrap_nodes {
            if let Ok(addr) = bootstrap_addr.parse::<SocketAddr>() {
                if let Err(e) = self.client.connect_to_peer(addr).await {
                    tracing::warn!("Failed to connect to bootstrap node {}: {}", addr, e);
                }
            }
        }
        Ok(())
    }

    /// Démarre les tâches de maintenance
    async fn start_maintenance_tasks(&self) {
        let peers = self.peers.clone();
        let config = self.config.clone();
        
        // Tâche de nettoyage des pairs inactifs
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(
                tokio::time::Duration::from_secs(config.ping_interval)
            );
            
            loop {
                interval.tick().await;
                
                let mut peers_guard = peers.write().await;
                let cutoff = chrono::Utc::now() - chrono::Duration::seconds(config.ping_interval as i64 * 3);
                
                peers_guard.retain(|_, peer| {
                    if peer.last_seen < cutoff {
                        tracing::debug!("Removing inactive peer: {}", peer.peer_id);
                        false
                    } else {
                        true
                    }
                });
            }
        });

        // Tâche de mise à jour des statistiques
        let stats = self.stats.clone();
        let start_time = chrono::Utc::now();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
            
            loop {
                interval.tick().await;
                
                let mut stats_guard = stats.write().await;
                stats_guard.uptime_seconds = (chrono::Utc::now() - start_time).num_seconds() as u64;
            }
        });
    }

    /// Ajoute un nouveau pair
    pub async fn add_peer(&self, peer_info: PeerInfo) -> ApiResult<()> {
        let mut peers = self.peers.write().await;
        let mut stats = self.stats.write().await;
        
        if peers.len() >= self.config.max_peers {
            return Err(crate::api::ApiError::internal("Maximum number of peers reached"));
        }

        peers.insert(peer_info.peer_id.clone(), peer_info);
        stats.connected_peers = peers.len();
        stats.connections_established += 1;

        Ok(())
    }

    /// Supprime un pair
    pub async fn remove_peer(&self, peer_id: &str) -> ApiResult<()> {
        let mut peers = self.peers.write().await;
        let mut stats = self.stats.write().await;
        
        if peers.remove(peer_id).is_some() {
            stats.connected_peers = peers.len();
            stats.connections_closed += 1;
        }

        Ok(())
    }

    /// Récupère la liste des pairs connectés
    pub async fn get_peers(&self) -> Vec<PeerInfo> {
        let peers = self.peers.read().await;
        peers.values().cloned().collect()
    }

    /// Récupère les statistiques P2P
    pub async fn get_stats(&self) -> P2PStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Diffuse un message à tous les pairs
    pub async fn broadcast_message(&self, message: P2PMessage) -> ApiResult<usize> {
        let peers = self.peers.read().await;
        let mut sent_count = 0;

        for peer in peers.values() {
            if peer.status == PeerStatus::Connected {
                if let Ok(_) = self.client.send_message(&peer.peer_id, message.clone()).await {
                    sent_count += 1;
                }
            }
        }

        // Met à jour les statistiques
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += sent_count as u64;
        }

        Ok(sent_count)
    }

    /// Envoie un message à un pair spécifique
    pub async fn send_to_peer(&self, peer_id: &str, message: P2PMessage) -> ApiResult<()> {
        self.client.send_message(peer_id, message).await?;
        
        // Met à jour les statistiques
        {
            let mut stats = self.stats.write().await;
            stats.messages_sent += 1;
        }

        Ok(())
    }

    /// Vérifie si le réseau a suffisamment de pairs
    pub async fn has_sufficient_peers(&self) -> bool {
        let peers = self.peers.read().await;
        peers.len() >= self.config.min_peers
    }

    /// Récupère le meilleur pair pour la synchronisation
    pub async fn get_best_sync_peer(&self) -> Option<PeerInfo> {
        let peers = self.peers.read().await;
        
        peers.values()
            .filter(|peer| peer.status == PeerStatus::Connected)
            .max_by_key(|peer| peer.block_height)
            .cloned()
    }
}

/// Erreurs P2P
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    
    #[error("Peer not found: {0}")]
    PeerNotFound(String),
    
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Timeout")]
    Timeout,
    
    #[error("Peer banned: {0}")]
    PeerBanned(String),
    
    #[error("Invalid message format")]
    InvalidMessage,
    
    #[error("Service unavailable")]
    ServiceUnavailable,
}

impl From<P2PError> for crate::api::ApiError {
    fn from(err: P2PError) -> Self {
        match err {
            P2PError::Timeout => crate::api::ApiError::internal("P2P timeout"),
            P2PError::ServiceUnavailable => crate::api::ApiError::ServiceUnavailable("P2P service unavailable".to_string()),
            _ => crate::api::ApiError::internal(err.to_string()),
        }
    }
}

/// Type de résultat P2P
pub type P2PResult<T> = Result<T, P2PError>;

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_p2p_config_default() {
        let config = P2PConfig::default();
        assert_eq!(config.listen_port, 8000);
        assert_eq!(config.max_peers, 50);
        assert_eq!(config.min_peers, 3);
        assert!(config.enable_discovery);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_peer_info_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let peer_info = PeerInfo {
            peer_id: "peer_123".to_string(),
            addr,
            protocol_version: "1.0".to_string(),
            client_version: "archivechain-0.1.0".to_string(),
            block_height: 12345,
            best_block_hash: "0x123456".to_string(),
            latency_ms: 50,
            last_seen: chrono::Utc::now(),
            status: PeerStatus::Connected,
            region: Some("us-east".to_string()),
            capabilities: HashSet::new(),
        };

        assert_eq!(peer_info.peer_id, "peer_123");
        assert_eq!(peer_info.status, PeerStatus::Connected);
        assert_eq!(peer_info.block_height, 12345);
    }

    #[test]
    fn test_peer_status() {
        assert_eq!(PeerStatus::Connected, PeerStatus::Connected);
        assert_ne!(PeerStatus::Connected, PeerStatus::Disconnected);
    }

    #[test]
    fn test_p2p_stats_default() {
        let stats = P2PStats::default();
        assert_eq!(stats.connected_peers, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.messages_received, 0);
    }

    #[test]
    fn test_p2p_error_conversion() {
        let p2p_err = P2PError::Timeout;
        let api_err: crate::api::ApiError = p2p_err.into();
        
        match api_err {
            crate::api::ApiError::Internal(_) => (),
            _ => panic!("Expected Internal error"),
        }
    }

    #[test]
    fn test_peer_capabilities() {
        let mut capabilities = HashSet::new();
        capabilities.insert("sync".to_string());
        capabilities.insert("gossip".to_string());
        
        assert!(capabilities.contains("sync"));
        assert!(capabilities.contains("gossip"));
        assert!(!capabilities.contains("invalid"));
    }
}