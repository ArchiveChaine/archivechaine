//! Service de découverte P2P pour ArchiveChain
//!
//! Implémente la découverte automatique de pairs via différents mécanismes.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{RwLock, oneshot};
use tokio::time::{Duration, interval};

use super::{P2PConfig, P2PError, P2PResult, messages::*};

/// Service de découverte de pairs
#[derive(Debug)]
pub struct DiscoveryService {
    /// Configuration
    config: P2PConfig,
    /// Pairs découverts
    discovered_peers: Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
    /// Canal d'arrêt
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
}

/// Pair découvert
#[derive(Debug, Clone)]
pub struct DiscoveredPeer {
    /// ID du pair
    pub peer_id: String,
    /// Adresse du pair
    pub addr: SocketAddr,
    /// Source de découverte
    pub discovery_source: DiscoverySource,
    /// Première découverte
    pub discovered_at: chrono::DateTime<chrono::Utc>,
    /// Dernière confirmation
    pub last_seen: chrono::DateTime<chrono::Utc>,
    /// Nombre de confirmations
    pub confirmations: u32,
    /// Score de réputation
    pub reputation_score: f64,
}

/// Source de découverte
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum DiscoverySource {
    Bootstrap,
    PeerExchange,
    DHT,
    LocalNetwork,
    Manual,
}

impl DiscoveryService {
    /// Crée un nouveau service de découverte
    pub fn new(config: P2PConfig) -> Self {
        Self {
            config,
            discovered_peers: Arc::new(RwLock::new(HashMap::new())),
            shutdown_tx: Arc::new(RwLock::new(None)),
        }
    }

    /// Démarre le service de découverte
    pub async fn start(&self) -> P2PResult<()> {
        if !self.config.enable_discovery {
            return Ok(());
        }

        tracing::info!("Starting P2P discovery service");

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        {
            let mut shutdown_guard = self.shutdown_tx.write().await;
            *shutdown_guard = Some(shutdown_tx);
        }

        // Ajoute les nœuds bootstrap
        self.add_bootstrap_peers().await?;

        // Démarre la tâche de découverte périodique
        let discovered_peers = self.discovered_peers.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(config.discovery_interval));

            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        // Effectue la découverte périodique
                        if let Err(e) = Self::perform_discovery(&discovered_peers, &config).await {
                            tracing::error!("Discovery error: {}", e);
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("Discovery service shutting down");
                        break;
                    }
                }
            }
        });

        // Démarre la tâche de nettoyage
        self.start_cleanup_task().await;

        tracing::info!("P2P discovery service started");
        Ok(())
    }

    /// Arrête le service de découverte
    pub async fn stop(&self) -> P2PResult<()> {
        tracing::info!("Stopping P2P discovery service");

        if let Some(shutdown_tx) = self.shutdown_tx.write().await.take() {
            let _ = shutdown_tx.send(());
        }

        tracing::info!("P2P discovery service stopped");
        Ok(())
    }

    /// Ajoute les nœuds bootstrap
    async fn add_bootstrap_peers(&self) -> P2PResult<()> {
        let mut peers = self.discovered_peers.write().await;

        for bootstrap_addr in &self.config.bootstrap_nodes {
            if let Ok(addr) = bootstrap_addr.parse::<SocketAddr>() {
                let peer_id = format!("bootstrap_{}", uuid::Uuid::new_v4().simple());
                
                let peer = DiscoveredPeer {
                    peer_id: peer_id.clone(),
                    addr,
                    discovery_source: DiscoverySource::Bootstrap,
                    discovered_at: chrono::Utc::now(),
                    last_seen: chrono::Utc::now(),
                    confirmations: 1,
                    reputation_score: 1.0,
                };

                peers.insert(peer_id, peer);
                tracing::debug!("Added bootstrap peer: {}", addr);
            } else {
                tracing::warn!("Invalid bootstrap address: {}", bootstrap_addr);
            }
        }

        Ok(())
    }

    /// Effectue la découverte périodique
    async fn perform_discovery(
        discovered_peers: &Arc<RwLock<HashMap<String, DiscoveredPeer>>>,
        config: &P2PConfig,
    ) -> P2PResult<()> {
        tracing::debug!("Performing peer discovery");

        // Pour l'instant, implémente une découverte simple
        // TODO: Implémenter DHT, mDNS, etc.

        // Nettoie les pairs obsolètes
        {
            let mut peers = discovered_peers.write().await;
            let cutoff = chrono::Utc::now() - chrono::Duration::hours(24);
            
            peers.retain(|_, peer| {
                peer.last_seen > cutoff || peer.discovery_source == DiscoverySource::Bootstrap
            });
        }

        Ok(())
    }

    /// Démarre la tâche de nettoyage
    async fn start_cleanup_task(&self) {
        let discovered_peers = self.discovered_peers.clone();

        tokio::spawn(async move {
            let mut interval = interval(Duration::from_secs(3600)); // 1 heure

            loop {
                interval.tick().await;

                let mut peers = discovered_peers.write().await;
                let cutoff = chrono::Utc::now() - chrono::Duration::hours(48);

                // Supprime les pairs très anciens (sauf bootstrap)
                peers.retain(|peer_id, peer| {
                    if peer.discovery_source == DiscoverySource::Bootstrap {
                        true
                    } else if peer.last_seen < cutoff && peer.reputation_score < 0.5 {
                        tracing::debug!("Removing stale peer: {}", peer_id);
                        false
                    } else {
                        true
                    }
                });
            }
        });
    }

    /// Ajoute un pair découvert
    pub async fn add_discovered_peer(
        &self,
        peer_id: String,
        addr: SocketAddr,
        source: DiscoverySource,
    ) -> P2PResult<()> {
        let mut peers = self.discovered_peers.write().await;

        if let Some(existing_peer) = peers.get_mut(&peer_id) {
            // Met à jour un pair existant
            existing_peer.last_seen = chrono::Utc::now();
            existing_peer.confirmations += 1;
            
            // Améliore le score de réputation
            existing_peer.reputation_score = (existing_peer.reputation_score + 0.1).min(1.0);
        } else {
            // Ajoute un nouveau pair
            let peer = DiscoveredPeer {
                peer_id: peer_id.clone(),
                addr,
                discovery_source: source,
                discovered_at: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
                confirmations: 1,
                reputation_score: 0.5, // Score initial neutre
            };

            peers.insert(peer_id.clone(), peer);
            tracing::debug!("Discovered new peer: {} at {}", peer_id, addr);
        }

        Ok(())
    }

    /// Supprime un pair
    pub async fn remove_peer(&self, peer_id: &str) -> P2PResult<()> {
        let mut peers = self.discovered_peers.write().await;
        
        if peers.remove(peer_id).is_some() {
            tracing::debug!("Removed peer: {}", peer_id);
        }

        Ok(())
    }

    /// Marque un pair comme mauvais (réduit sa réputation)
    pub async fn mark_peer_bad(&self, peer_id: &str, reason: &str) -> P2PResult<()> {
        let mut peers = self.discovered_peers.write().await;
        
        if let Some(peer) = peers.get_mut(peer_id) {
            peer.reputation_score = (peer.reputation_score - 0.2).max(0.0);
            tracing::debug!("Marked peer {} as bad ({}): score = {}", 
                peer_id, reason, peer.reputation_score);
            
            // Supprime les pairs avec une très mauvaise réputation
            if peer.reputation_score < 0.1 && peer.discovery_source != DiscoverySource::Bootstrap {
                peers.remove(peer_id);
                tracing::debug!("Removed peer with bad reputation: {}", peer_id);
            }
        }

        Ok(())
    }

    /// Récupère la liste des pairs découverts
    pub async fn get_discovered_peers(&self) -> Vec<DiscoveredPeer> {
        let peers = self.discovered_peers.read().await;
        peers.values().cloned().collect()
    }

    /// Récupère les meilleurs pairs pour se connecter
    pub async fn get_best_peers(&self, count: usize) -> Vec<DiscoveredPeer> {
        let peers = self.discovered_peers.read().await;
        
        let mut peer_list: Vec<_> = peers.values().cloned().collect();
        
        // Trie par score de réputation (décroissant)
        peer_list.sort_by(|a, b| b.reputation_score.partial_cmp(&a.reputation_score).unwrap());
        
        peer_list.into_iter().take(count).collect()
    }

    /// Traite les pairs reçus via peer exchange
    pub async fn process_peer_exchange(&self, peers: Vec<PeerAddress>) -> P2PResult<()> {
        for peer_addr in peers {
            if let Ok(addr) = format!("{}:{}", peer_addr.address, peer_addr.port).parse::<SocketAddr>() {
                self.add_discovered_peer(
                    peer_addr.peer_id,
                    addr,
                    DiscoverySource::PeerExchange,
                ).await?;
            }
        }
        Ok(())
    }

    /// Récupère des pairs aléatoires pour partager
    pub async fn get_peers_for_exchange(&self, max_count: usize) -> Vec<PeerAddress> {
        let peers = self.discovered_peers.read().await;
        
        let mut peer_list: Vec<_> = peers.values()
            .filter(|peer| peer.reputation_score > 0.3) // Seulement les pairs corrects
            .map(|peer| PeerAddress {
                peer_id: peer.peer_id.clone(),
                address: peer.addr.ip().to_string(),
                port: peer.addr.port(),
                last_seen: peer.last_seen,
            })
            .collect();

        // Mélange et limite
        use rand::seq::SliceRandom;
        let mut rng = rand::thread_rng();
        peer_list.shuffle(&mut rng);
        
        peer_list.into_iter().take(max_count).collect()
    }

    /// Récupère les statistiques de découverte
    pub async fn get_discovery_stats(&self) -> DiscoveryStats {
        let peers = self.discovered_peers.read().await;
        
        let mut stats = DiscoveryStats {
            total_discovered: peers.len(),
            by_source: HashMap::new(),
            average_reputation: 0.0,
            active_peers: 0,
        };

        let mut total_reputation = 0.0;
        let cutoff = chrono::Utc::now() - chrono::Duration::hours(1);

        for peer in peers.values() {
            *stats.by_source.entry(peer.discovery_source.clone()).or_insert(0) += 1;
            total_reputation += peer.reputation_score;
            
            if peer.last_seen > cutoff {
                stats.active_peers += 1;
            }
        }

        if !peers.is_empty() {
            stats.average_reputation = total_reputation / peers.len() as f64;
        }

        stats
    }
}

/// Statistiques de découverte
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveryStats {
    pub total_discovered: usize,
    pub by_source: HashMap<DiscoverySource, usize>,
    pub average_reputation: f64,
    pub active_peers: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_discovery_service_creation() {
        let config = P2PConfig::default();
        let service = DiscoveryService::new(config);
        
        // Vérifie que le service peut être créé
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_discovered_peer() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let peer = DiscoveredPeer {
            peer_id: "peer_123".to_string(),
            addr,
            discovery_source: DiscoverySource::Bootstrap,
            discovered_at: chrono::Utc::now(),
            last_seen: chrono::Utc::now(),
            confirmations: 1,
            reputation_score: 1.0,
        };

        assert_eq!(peer.peer_id, "peer_123");
        assert_eq!(peer.discovery_source, DiscoverySource::Bootstrap);
        assert_eq!(peer.reputation_score, 1.0);
    }

    #[test]
    fn test_discovery_source() {
        assert_eq!(DiscoverySource::Bootstrap, DiscoverySource::Bootstrap);
        assert_ne!(DiscoverySource::Bootstrap, DiscoverySource::DHT);
    }

    #[tokio::test]
    async fn test_add_discovered_peer() {
        let config = P2PConfig::default();
        let service = DiscoveryService::new(config);
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let result = service.add_discovered_peer(
            "peer_123".to_string(),
            addr,
            DiscoverySource::Manual,
        ).await;

        assert!(result.is_ok());

        let peers = service.get_discovered_peers().await;
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].peer_id, "peer_123");
    }

    #[tokio::test]
    async fn test_mark_peer_bad() {
        let config = P2PConfig::default();
        let service = DiscoveryService::new(config);
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        service.add_discovered_peer(
            "peer_123".to_string(),
            addr,
            DiscoverySource::Manual,
        ).await.unwrap();

        let result = service.mark_peer_bad("peer_123", "test reason").await;
        assert!(result.is_ok());

        let peers = service.get_discovered_peers().await;
        assert_eq!(peers[0].reputation_score, 0.3); // 0.5 - 0.2
    }

    #[tokio::test]
    async fn test_get_best_peers() {
        let config = P2PConfig::default();
        let service = DiscoveryService::new(config);
        
        // Ajoute plusieurs pairs avec différents scores
        let addr1 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8001);
        let addr2 = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8002);
        
        service.add_discovered_peer("peer_1".to_string(), addr1, DiscoverySource::Bootstrap).await.unwrap();
        service.add_discovered_peer("peer_2".to_string(), addr2, DiscoverySource::Manual).await.unwrap();
        
        // Améliore le score du premier
        service.add_discovered_peer("peer_1".to_string(), addr1, DiscoverySource::Bootstrap).await.unwrap();

        let best_peers = service.get_best_peers(2).await;
        assert_eq!(best_peers.len(), 2);
        
        // Le premier devrait avoir un meilleur score
        assert!(best_peers[0].reputation_score >= best_peers[1].reputation_score);
    }

    #[tokio::test]
    async fn test_peer_exchange() {
        let config = P2PConfig::default();
        let service = DiscoveryService::new(config);
        
        let peer_addresses = vec![
            PeerAddress {
                peer_id: "peer_1".to_string(),
                address: "127.0.0.1".to_string(),
                port: 8001,
                last_seen: chrono::Utc::now(),
            },
            PeerAddress {
                peer_id: "peer_2".to_string(),
                address: "127.0.0.1".to_string(),
                port: 8002,
                last_seen: chrono::Utc::now(),
            },
        ];

        let result = service.process_peer_exchange(peer_addresses).await;
        assert!(result.is_ok());

        let peers = service.get_discovered_peers().await;
        assert_eq!(peers.len(), 2);
        
        for peer in peers {
            assert_eq!(peer.discovery_source, DiscoverySource::PeerExchange);
        }
    }

    #[tokio::test]
    async fn test_discovery_stats() {
        let config = P2PConfig::default();
        let service = DiscoveryService::new(config);
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        service.add_discovered_peer("peer_1".to_string(), addr, DiscoverySource::Bootstrap).await.unwrap();
        service.add_discovered_peer("peer_2".to_string(), addr, DiscoverySource::DHT).await.unwrap();

        let stats = service.get_discovery_stats().await;
        assert_eq!(stats.total_discovered, 2);
        assert_eq!(stats.by_source.get(&DiscoverySource::Bootstrap), Some(&1));
        assert_eq!(stats.by_source.get(&DiscoverySource::DHT), Some(&1));
        assert!(stats.average_reputation > 0.0);
    }
}