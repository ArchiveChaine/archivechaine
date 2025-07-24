//! Implémentation du Gateway Node
//!
//! Les Gateway Nodes sont les points d'entrée publics du réseau ArchiveChain :
//! - Interface publique pour applications web (REST, GraphQL, WebSocket, gRPC)
//! - Load balancing et mise en cache intelligente
//! - Authentification et sécurité (JWT, OAuth)
//! - Rate limiting et protection DDoS
//! - Monitoring et métriques d'utilisation

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use std::net::SocketAddr;
use tokio::sync::{RwLock, Mutex};
use async_trait::async_trait;

use crate::crypto::{Hash, PublicKey, PrivateKey};
use crate::consensus::NodeId;
use crate::api::{ApiConfig, ApiError, ApiResult};
use crate::error::Result;
use super::{
    Node, NodeType, NodeConfiguration, NetworkMessage, MessageType, ApiType,
    NodeHealth, NodeMetrics, GeneralNodeMetrics, HealthStatus
};

/// Configuration spécifique aux Gateway Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayNodeConfig {
    /// Configuration générale du nœud
    pub node_config: NodeConfiguration,
    /// APIs exposées publiquement
    pub exposed_apis: Vec<ApiType>,
    /// Configuration du load balancer
    pub load_balancer_config: LoadBalancerConfig,
    /// Configuration du système de cache
    pub cache_config: CacheConfig,
    /// Configuration du rate limiter
    pub rate_limiter_config: RateLimiterConfig,
    /// Configuration de sécurité
    pub security_config: GatewaySecurityConfig,
    /// Configuration de monitoring
    pub monitoring_config: GatewayMonitoringConfig,
    /// Nœuds backend du réseau
    pub backend_nodes: Vec<BackendNodeInfo>,
}

/// Configuration du load balancer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerConfig {
    /// Algorithme de load balancing
    pub algorithm: LoadBalancingAlgorithm,
    /// Health check activé
    pub health_check_enabled: bool,
    /// Intervalle de health check
    pub health_check_interval: Duration,
    /// Timeout de health check
    pub health_check_timeout: Duration,
    /// Nombre de tentatives avant marquage comme indisponible
    pub max_retries: u32,
    /// Timeout de circuit breaker
    pub circuit_breaker_timeout: Duration,
}

/// Algorithmes de load balancing
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LoadBalancingAlgorithm {
    /// Round Robin
    RoundRobin,
    /// Least Connections
    LeastConnections,
    /// Least Response Time
    LeastResponseTime,
    /// Weighted Round Robin
    WeightedRoundRobin,
    /// Random
    Random,
    /// IP Hash
    IpHash,
}

/// Configuration du cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Cache activé
    pub enabled: bool,
    /// Taille maximale du cache (bytes)
    pub max_cache_size: u64,
    /// TTL par défaut
    pub default_ttl: Duration,
    /// Politique d'éviction
    pub eviction_policy: CacheEvictionPolicy,
    /// Cache des métadonnées
    pub cache_metadata: bool,
    /// Cache du contenu
    pub cache_content: bool,
    /// Compression du cache
    pub compress_cache: bool,
}

/// Politiques d'éviction du cache
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CacheEvictionPolicy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// Time-based expiration
    TTL,
    /// First In First Out
    FIFO,
}

/// Configuration du rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterConfig {
    /// Rate limiting activé
    pub enabled: bool,
    /// Requêtes par seconde par IP
    pub requests_per_second_per_ip: u32,
    /// Requêtes par minute par IP
    pub requests_per_minute_per_ip: u32,
    /// Burst allowance
    pub burst_allowance: u32,
    /// Whitelist d'IPs
    pub ip_whitelist: Vec<String>,
    /// Blacklist d'IPs
    pub ip_blacklist: Vec<String>,
    /// Rate limiting par API key
    pub api_key_limits: HashMap<String, RateLimit>,
}

/// Limite de taux pour une clé API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    /// Requêtes par seconde
    pub requests_per_second: u32,
    /// Requêtes par minute
    pub requests_per_minute: u32,
    /// Requêtes par heure
    pub requests_per_hour: u32,
    /// Burst allowance
    pub burst_allowance: u32,
}

/// Configuration de sécurité du Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewaySecurityConfig {
    /// CORS activé
    pub cors_enabled: bool,
    /// Origines CORS autorisées
    pub cors_allowed_origins: Vec<String>,
    /// HTTPS forcé
    pub force_https: bool,
    /// Configuration JWT
    pub jwt_config: JwtConfig,
    /// Protection DDoS activée
    pub ddos_protection_enabled: bool,
    /// Seuil de détection DDoS
    pub ddos_detection_threshold: u32,
    /// WAF (Web Application Firewall) activé
    pub waf_enabled: bool,
}

/// Configuration JWT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    /// JWT activé
    pub enabled: bool,
    /// Clé secrète pour signature
    pub secret_key: String,
    /// Durée de vie des tokens
    pub token_lifetime: Duration,
    /// Algorithme de signature
    pub signing_algorithm: String,
    /// Issuer
    pub issuer: String,
    /// Audience
    pub audience: String,
}

/// Configuration de monitoring du Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMonitoringConfig {
    /// Monitoring activé
    pub enabled: bool,
    /// Collecte des métriques détaillées
    pub detailed_metrics: bool,
    /// Logging des requêtes
    pub request_logging: bool,
    /// Niveau de logging
    pub log_level: String,
    /// Export des métriques Prometheus
    pub prometheus_metrics: bool,
    /// Alertes activées
    pub alerts_enabled: bool,
}

/// Informations sur un nœud backend
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendNodeInfo {
    /// Identifiant du nœud
    pub node_id: NodeId,
    /// Adresse du nœud
    pub address: SocketAddr,
    /// Type de nœud
    pub node_type: NodeType,
    /// Poids pour le load balancing
    pub weight: u32,
    /// Statut de santé
    pub health_status: BackendHealthStatus,
    /// Dernière vérification de santé
    pub last_health_check: SystemTime,
    /// Latence moyenne
    pub average_latency: Duration,
    /// Connexions actives
    pub active_connections: u32,
}

/// Statut de santé d'un backend
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendHealthStatus {
    /// Healthy
    Healthy,
    /// Dégradé
    Degraded,
    /// Indisponible
    Unhealthy,
    /// Non testé
    Unknown,
}

/// Point d'accès API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiEndpoint {
    /// Type d'API
    pub api_type: ApiType,
    /// Port d'écoute
    pub port: u16,
    /// Chemin de base
    pub base_path: String,
    /// Version de l'API
    pub version: String,
    /// SSL/TLS activé
    pub ssl_enabled: bool,
    /// Configuration spécifique
    pub config: ApiEndpointConfig,
}

/// Configuration spécifique d'un endpoint API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApiEndpointConfig {
    /// Configuration REST
    Rest {
        max_request_size: u64,
        timeout: Duration,
        compression: bool,
    },
    /// Configuration GraphQL
    GraphQL {
        introspection_enabled: bool,
        playground_enabled: bool,
        max_query_depth: u32,
        max_query_complexity: u32,
    },
    /// Configuration WebSocket
    WebSocket {
        max_connections: u32,
        ping_interval: Duration,
        message_size_limit: u64,
    },
    /// Configuration gRPC
    GRPC {
        max_message_size: u64,
        keepalive_time: Duration,
        keepalive_timeout: Duration,
    },
}

/// Load Balancer
#[derive(Debug)]
pub struct LoadBalancer {
    /// Configuration
    config: LoadBalancerConfig,
    /// Nœuds backend
    backend_nodes: Arc<RwLock<Vec<BackendNodeInfo>>>,
    /// Index actuel pour Round Robin
    current_index: Arc<Mutex<usize>>,
    /// Métriques
    metrics: Arc<RwLock<LoadBalancerMetrics>>,
}

/// Métriques du load balancer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadBalancerMetrics {
    /// Requêtes totales
    pub total_requests: u64,
    /// Requêtes réussies
    pub successful_requests: u64,
    /// Requêtes échouées
    pub failed_requests: u64,
    /// Temps de réponse moyen
    pub average_response_time: Duration,
    /// Distribution des requêtes par backend
    pub requests_per_backend: HashMap<NodeId, u64>,
}

/// Couche de cache
#[derive(Debug)]
pub struct CacheLayer {
    /// Configuration
    config: CacheConfig,
    /// Cache des métadonnées
    metadata_cache: Arc<RwLock<HashMap<Hash, CachedMetadata>>>,
    /// Cache du contenu
    content_cache: Arc<RwLock<HashMap<Hash, CachedContent>>>,
    /// Métriques du cache
    metrics: Arc<RwLock<CacheMetrics>>,
}

/// Métadonnées en cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedMetadata {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Métadonnées
    pub metadata: serde_json::Value,
    /// Timestamp de mise en cache
    pub cached_at: SystemTime,
    /// TTL
    pub ttl: Duration,
    /// Nombre d'accès
    pub access_count: u64,
    /// Dernière date d'accès
    pub last_accessed: SystemTime,
}

/// Contenu en cache
#[derive(Debug, Clone)]
pub struct CachedContent {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Données compressées
    pub compressed_data: Vec<u8>,
    /// Taille originale
    pub original_size: u64,
    /// Timestamp de mise en cache
    pub cached_at: SystemTime,
    /// TTL
    pub ttl: Duration,
    /// Nombre d'accès
    pub access_count: u64,
    /// Dernière date d'accès
    pub last_accessed: SystemTime,
}

/// Métriques du cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Hits du cache
    pub cache_hits: u64,
    /// Misses du cache
    pub cache_misses: u64,
    /// Évictions
    pub evictions: u64,
    /// Taille actuelle du cache
    pub current_cache_size: u64,
    /// Ratio de hit
    pub hit_ratio: f64,
}

/// Rate Limiter
#[derive(Debug)]
pub struct RateLimiter {
    /// Configuration
    config: RateLimiterConfig,
    /// Buckets par IP
    ip_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    /// Buckets par API key
    api_key_buckets: Arc<RwLock<HashMap<String, TokenBucket>>>,
    /// Métriques
    metrics: Arc<RwLock<RateLimiterMetrics>>,
}

/// Token bucket pour rate limiting
#[derive(Debug, Clone)]
pub struct TokenBucket {
    /// Tokens disponibles
    pub tokens: f64,
    /// Capacité maximale
    pub capacity: f64,
    /// Taux de remplissage (tokens/sec)
    pub refill_rate: f64,
    /// Dernière mise à jour
    pub last_refill: SystemTime,
}

/// Métriques du rate limiter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimiterMetrics {
    /// Requêtes autorisées
    pub allowed_requests: u64,
    /// Requêtes bloquées
    pub blocked_requests: u64,
    /// Taux de blocage
    pub block_rate: f64,
    /// IPs bloquées actuellement
    pub currently_blocked_ips: u32,
}

/// Stack de sécurité
#[derive(Debug)]
pub struct SecurityStack {
    /// Configuration
    config: GatewaySecurityConfig,
    /// Détecteur DDoS
    ddos_detector: Arc<RwLock<DDoSDetector>>,
    /// WAF
    waf: Arc<RwLock<WebApplicationFirewall>>,
    /// Métriques de sécurité
    metrics: Arc<RwLock<SecurityMetrics>>,
}

/// Détecteur DDoS
#[derive(Debug, Clone)]
pub struct DDoSDetector {
    /// Fenêtre de détection
    pub detection_window: Duration,
    /// Seuil de détection
    pub detection_threshold: u32,
    /// Requêtes par IP
    pub requests_per_ip: HashMap<String, VecDeque<SystemTime>>,
}

/// Web Application Firewall
#[derive(Debug, Clone)]
pub struct WebApplicationFirewall {
    /// Règles de filtrage
    pub rules: Vec<WafRule>,
    /// Patterns suspects
    pub suspicious_patterns: Vec<String>,
}

/// Règle WAF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafRule {
    /// Nom de la règle
    pub name: String,
    /// Pattern à détecter
    pub pattern: String,
    /// Action à prendre
    pub action: WafAction,
    /// Activée
    pub enabled: bool,
}

/// Actions WAF
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WafAction {
    /// Bloquer
    Block,
    /// Logger seulement
    Log,
    /// Challengère (CAPTCHA)
    Challenge,
}

/// Métriques de sécurité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityMetrics {
    /// Attaques détectées
    pub attacks_detected: u64,
    /// Attaques bloquées
    pub attacks_blocked: u64,
    /// Requêtes suspectes
    pub suspicious_requests: u64,
    /// IPs blacklistées
    pub blacklisted_ips: u32,
}

/// Statut d'un Gateway Node
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GatewayNodeStatus {
    /// Initialisation en cours
    Initializing,
    /// Configuration des APIs
    ConfiguringApis,
    /// Démarrage des services
    StartingServices,
    /// Opérationnel
    Operational,
    /// Surcharge
    Overloaded,
    /// Maintenance
    Maintenance,
    /// Problème de sécurité
    SecurityIssue,
    /// Arrêt en cours
    Stopping,
}

/// Métriques spécifiques aux Gateway Nodes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayMetrics {
    /// Métriques générales
    pub general: GeneralNodeMetrics,
    /// APIs actives
    pub active_apis: Vec<ApiType>,
    /// Requêtes par API
    pub requests_per_api: HashMap<ApiType, u64>,
    /// Temps de réponse par API
    pub response_time_per_api: HashMap<ApiType, Duration>,
    /// Métriques du load balancer
    pub load_balancer_metrics: LoadBalancerMetrics,
    /// Métriques du cache
    pub cache_metrics: CacheMetrics,
    /// Métriques du rate limiter
    pub rate_limiter_metrics: RateLimiterMetrics,
    /// Métriques de sécurité
    pub security_metrics: SecurityMetrics,
    /// Connexions WebSocket actives
    pub active_websocket_connections: u32,
    /// Clients authentifiés
    pub authenticated_clients: u32,
}

impl NodeMetrics for GatewayMetrics {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn general_metrics(&self) -> GeneralNodeMetrics {
        self.general.clone()
    }
}

/// Gateway Node - Nœud passerelle pour l'accès public
pub struct GatewayNode {
    /// Configuration du nœud
    config: GatewayNodeConfig,
    /// Identifiant du nœud
    node_id: NodeId,
    /// Clés cryptographiques
    keypair: (PublicKey, PrivateKey),
    /// Statut actuel
    status: Arc<RwLock<GatewayNodeStatus>>,
    /// Points d'accès API
    api_endpoints: Arc<RwLock<Vec<ApiEndpoint>>>,
    /// Load balancer
    load_balancer: Arc<Mutex<LoadBalancer>>,
    /// Couche de cache
    cache_layer: Arc<Mutex<CacheLayer>>,
    /// Rate limiter
    rate_limiter: Arc<Mutex<RateLimiter>>,
    /// Stack de sécurité
    security_stack: Arc<Mutex<SecurityStack>>,
    /// Métriques
    metrics: Arc<RwLock<GatewayMetrics>>,
    /// Heure de démarrage
    start_time: SystemTime,
}

impl Default for GatewayNodeConfig {
    fn default() -> Self {
        Self {
            node_config: NodeConfiguration {
                node_id: NodeId::from(Hash::zero()),
                node_type: NodeType::Gateway {
                    exposed_apis: vec![ApiType::Rest, ApiType::WebSocket],
                    rate_limit: 1000,
                },
                region: "us-east-1".to_string(),
                listen_address: "0.0.0.0".to_string(),
                listen_port: 8083,
                bootstrap_nodes: Vec::new(),
                storage_config: None,
                network_config: super::NetworkConfiguration::default(),
                security_config: super::SecurityConfiguration::default(),
            },
            exposed_apis: vec![ApiType::Rest, ApiType::WebSocket],
            load_balancer_config: LoadBalancerConfig::default(),
            cache_config: CacheConfig::default(),
            rate_limiter_config: RateLimiterConfig::default(),
            security_config: GatewaySecurityConfig::default(),
            monitoring_config: GatewayMonitoringConfig::default(),
            backend_nodes: Vec::new(),
        }
    }
}

impl Default for LoadBalancerConfig {
    fn default() -> Self {
        Self {
            algorithm: LoadBalancingAlgorithm::RoundRobin,
            health_check_enabled: true,
            health_check_interval: Duration::from_secs(30),
            health_check_timeout: Duration::from_secs(5),
            max_retries: 3,
            circuit_breaker_timeout: Duration::from_secs(60),
        }
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_cache_size: 1_000_000_000, // 1GB
            default_ttl: Duration::from_secs(3600), // 1 heure
            eviction_policy: CacheEvictionPolicy::LRU,
            cache_metadata: true,
            cache_content: true,
            compress_cache: true,
        }
    }
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            requests_per_second_per_ip: 100,
            requests_per_minute_per_ip: 1000,
            burst_allowance: 50,
            ip_whitelist: Vec::new(),
            ip_blacklist: Vec::new(),
            api_key_limits: HashMap::new(),
        }
    }
}

impl Default for GatewaySecurityConfig {
    fn default() -> Self {
        Self {
            cors_enabled: true,
            cors_allowed_origins: vec!["*".to_string()],
            force_https: false,
            jwt_config: JwtConfig::default(),
            ddos_protection_enabled: true,
            ddos_detection_threshold: 1000,
            waf_enabled: true,
        }
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            secret_key: "default-secret".to_string(),
            token_lifetime: Duration::from_secs(3600), // 1 heure
            signing_algorithm: "HS256".to_string(),
            issuer: "archivechain".to_string(),
            audience: "api".to_string(),
        }
    }
}

impl Default for GatewayMonitoringConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detailed_metrics: true,
            request_logging: true,
            log_level: "info".to_string(),
            prometheus_metrics: true,
            alerts_enabled: true,
        }
    }
}

impl LoadBalancer {
    /// Crée un nouveau load balancer
    pub fn new(config: LoadBalancerConfig, backend_nodes: Vec<BackendNodeInfo>) -> Self {
        Self {
            config,
            backend_nodes: Arc::new(RwLock::new(backend_nodes)),
            current_index: Arc::new(Mutex::new(0)),
            metrics: Arc::new(RwLock::new(LoadBalancerMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time: Duration::ZERO,
                requests_per_backend: HashMap::new(),
            })),
        }
    }

    /// Sélectionne un backend selon l'algorithme configuré
    pub async fn select_backend(&self, client_ip: Option<&str>) -> Option<NodeId> {
        let backends = self.backend_nodes.read().await;
        let healthy_backends: Vec<_> = backends.iter()
            .filter(|b| b.health_status == BackendHealthStatus::Healthy)
            .collect();

        if healthy_backends.is_empty() {
            return None;
        }

        match self.config.algorithm {
            LoadBalancingAlgorithm::RoundRobin => {
                let mut index = self.current_index.lock().await;
                let selected = &healthy_backends[*index % healthy_backends.len()];
                *index = (*index + 1) % healthy_backends.len();
                Some(selected.node_id.clone())
            },
            LoadBalancingAlgorithm::LeastConnections => {
                healthy_backends.iter()
                    .min_by_key(|b| b.active_connections)
                    .map(|b| b.node_id.clone())
            },
            LoadBalancingAlgorithm::LeastResponseTime => {
                healthy_backends.iter()
                    .min_by_key(|b| b.average_latency)
                    .map(|b| b.node_id.clone())
            },
            LoadBalancingAlgorithm::Random => {
                use rand::seq::SliceRandom;
                healthy_backends.choose(&mut rand::thread_rng())
                    .map(|b| b.node_id.clone())
            },
            LoadBalancingAlgorithm::IpHash => {
                if let Some(ip) = client_ip {
                    let hash = crate::crypto::compute_hash(
                        ip.as_bytes(),
                        crate::crypto::HashAlgorithm::Blake3
                    );
                    let index = hash.as_bytes()[0] as usize % healthy_backends.len();
                    Some(healthy_backends[index].node_id.clone())
                } else {
                    healthy_backends.first().map(|b| b.node_id.clone())
                }
            },
            _ => healthy_backends.first().map(|b| b.node_id.clone()),
        }
    }
}

impl CacheLayer {
    /// Crée une nouvelle couche de cache
    pub fn new(config: CacheConfig) -> Self {
        Self {
            config,
            metadata_cache: Arc::new(RwLock::new(HashMap::new())),
            content_cache: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(CacheMetrics {
                cache_hits: 0,
                cache_misses: 0,
                evictions: 0,
                current_cache_size: 0,
                hit_ratio: 0.0,
            })),
        }
    }

    /// Récupère du contenu depuis le cache
    pub async fn get_content(&self, content_hash: &Hash) -> Option<Vec<u8>> {
        if !self.config.cache_content {
            return None;
        }

        let mut cache = self.content_cache.write().await;
        if let Some(cached) = cache.get_mut(content_hash) {
            // Vérifie le TTL
            if cached.cached_at.elapsed().unwrap_or(Duration::ZERO) < cached.ttl {
                cached.access_count += 1;
                cached.last_accessed = SystemTime::now();
                
                // Met à jour les métriques
                let mut metrics = self.metrics.write().await;
                metrics.cache_hits += 1;
                
                // Décompresse si nécessaire
                return Some(cached.compressed_data.clone()); // Simplification
            } else {
                // Contenu expiré
                cache.remove(content_hash);
            }
        }

        // Cache miss
        let mut metrics = self.metrics.write().await;
        metrics.cache_misses += 1;
        None
    }

    /// Met en cache du contenu
    pub async fn cache_content(&self, content_hash: Hash, data: Vec<u8>, ttl: Option<Duration>) {
        if !self.config.cache_content {
            return;
        }

        let ttl = ttl.unwrap_or(self.config.default_ttl);
        let cached_content = CachedContent {
            content_hash,
            compressed_data: data.clone(), // Simplification - pas de compression
            original_size: data.len() as u64,
            cached_at: SystemTime::now(),
            ttl,
            access_count: 0,
            last_accessed: SystemTime::now(),
        };

        let mut cache = self.content_cache.write().await;
        cache.insert(content_hash, cached_content);

        // Met à jour les métriques
        let mut metrics = self.metrics.write().await;
        metrics.current_cache_size += data.len() as u64;
    }
}

impl RateLimiter {
    /// Crée un nouveau rate limiter
    pub fn new(config: RateLimiterConfig) -> Self {
        Self {
            config,
            ip_buckets: Arc::new(RwLock::new(HashMap::new())),
            api_key_buckets: Arc::new(RwLock::new(HashMap::new())),
            metrics: Arc::new(RwLock::new(RateLimiterMetrics {
                allowed_requests: 0,
                blocked_requests: 0,
                block_rate: 0.0,
                currently_blocked_ips: 0,
            })),
        }
    }

    /// Vérifie si une requête est autorisée
    pub async fn check_rate_limit(&self, client_ip: &str, api_key: Option<&str>) -> bool {
        if !self.config.enabled {
            return true;
        }

        // Vérifie la whitelist
        if self.config.ip_whitelist.contains(&client_ip.to_string()) {
            return true;
        }

        // Vérifie la blacklist
        if self.config.ip_blacklist.contains(&client_ip.to_string()) {
            let mut metrics = self.metrics.write().await;
            metrics.blocked_requests += 1;
            return false;
        }

        // Vérifie le rate limit par IP
        let ip_allowed = self.check_ip_rate_limit(client_ip).await;
        
        // Vérifie le rate limit par API key si présente
        let api_key_allowed = if let Some(key) = api_key {
            self.check_api_key_rate_limit(key).await
        } else {
            true
        };

        let allowed = ip_allowed && api_key_allowed;
        
        // Met à jour les métriques
        let mut metrics = self.metrics.write().await;
        if allowed {
            metrics.allowed_requests += 1;
        } else {
            metrics.blocked_requests += 1;
        }

        allowed
    }

    async fn check_ip_rate_limit(&self, ip: &str) -> bool {
        let mut buckets = self.ip_buckets.write().await;
        let bucket = buckets.entry(ip.to_string()).or_insert_with(|| {
            TokenBucket {
                tokens: self.config.requests_per_second_per_ip as f64,
                capacity: self.config.requests_per_second_per_ip as f64,
                refill_rate: self.config.requests_per_second_per_ip as f64,
                last_refill: SystemTime::now(),
            }
        });

        // Refill tokens
        let now = SystemTime::now();
        let elapsed = now.duration_since(bucket.last_refill).unwrap_or(Duration::ZERO);
        let tokens_to_add = elapsed.as_secs_f64() * bucket.refill_rate;
        bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity);
        bucket.last_refill = now;

        // Vérifie si des tokens sont disponibles
        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    async fn check_api_key_rate_limit(&self, api_key: &str) -> bool {
        if let Some(limit) = self.config.api_key_limits.get(api_key) {
            let mut buckets = self.api_key_buckets.write().await;
            let bucket = buckets.entry(api_key.to_string()).or_insert_with(|| {
                TokenBucket {
                    tokens: limit.requests_per_second as f64,
                    capacity: limit.requests_per_second as f64,
                    refill_rate: limit.requests_per_second as f64,
                    last_refill: SystemTime::now(),
                }
            });

            // Logique similaire à check_ip_rate_limit
            let now = SystemTime::now();
            let elapsed = now.duration_since(bucket.last_refill).unwrap_or(Duration::ZERO);
            let tokens_to_add = elapsed.as_secs_f64() * bucket.refill_rate;
            bucket.tokens = (bucket.tokens + tokens_to_add).min(bucket.capacity);
            bucket.last_refill = now;

            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                true
            } else {
                false
            }
        } else {
            true // Pas de limite pour cette API key
        }
    }
}

impl GatewayNode {
    /// Crée une nouvelle instance de Gateway Node
    pub fn new(
        config: GatewayNodeConfig,
        keypair: (PublicKey, PrivateKey),
    ) -> Result<Self> {
        // Valide la configuration
        config.validate()?;

        let node_id = config.node_config.node_id.clone();
        let start_time = SystemTime::now();

        let load_balancer = LoadBalancer::new(
            config.load_balancer_config.clone(),
            config.backend_nodes.clone(),
        );

        let cache_layer = CacheLayer::new(config.cache_config.clone());
        let rate_limiter = RateLimiter::new(config.rate_limiter_config.clone());

        let security_stack = SecurityStack {
            config: config.security_config.clone(),
            ddos_detector: Arc::new(RwLock::new(DDoSDetector {
                detection_window: Duration::from_secs(60),
                detection_threshold: config.security_config.ddos_detection_threshold,
                requests_per_ip: HashMap::new(),
            })),
            waf: Arc::new(RwLock::new(WebApplicationFirewall {
                rules: Vec::new(),
                suspicious_patterns: vec![
                    "<script".to_string(),
                    "union select".to_string(),
                    "../".to_string(),
                ],
            })),
            metrics: Arc::new(RwLock::new(SecurityMetrics {
                attacks_detected: 0,
                attacks_blocked: 0,
                suspicious_requests: 0,
                blacklisted_ips: 0,
            })),
        };

        let initial_metrics = GatewayMetrics {
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
            active_apis: config.exposed_apis.clone(),
            requests_per_api: HashMap::new(),
            response_time_per_api: HashMap::new(),
            load_balancer_metrics: LoadBalancerMetrics {
                total_requests: 0,
                successful_requests: 0,
                failed_requests: 0,
                average_response_time: Duration::ZERO,
                requests_per_backend: HashMap::new(),
            },
            cache_metrics: CacheMetrics {
                cache_hits: 0,
                cache_misses: 0,
                evictions: 0,
                current_cache_size: 0,
                hit_ratio: 0.0,
            },
            rate_limiter_metrics: RateLimiterMetrics {
                allowed_requests: 0,
                blocked_requests: 0,
                block_rate: 0.0,
                currently_blocked_ips: 0,
            },
            security_metrics: SecurityMetrics {
                attacks_detected: 0,
                attacks_blocked: 0,
                suspicious_requests: 0,
                blacklisted_ips: 0,
            },
            active_websocket_connections: 0,
            authenticated_clients: 0,
        };

        Ok(Self {
            config,
            node_id,
            keypair,
            status: Arc::new(RwLock::new(GatewayNodeStatus::Initializing)),
            api_endpoints: Arc::new(RwLock::new(Vec::new())),
            load_balancer: Arc::new(Mutex::new(load_balancer)),
            cache_layer: Arc::new(Mutex::new(cache_layer)),
            rate_limiter: Arc::new(Mutex::new(rate_limiter)),
            security_stack: Arc::new(Mutex::new(security_stack)),
            metrics: Arc::new(RwLock::new(initial_metrics)),
            start_time,
        })
    }

    /// Configure les endpoints API
    pub async fn configure_api_endpoints(&self) -> Result<()> {
        {
            let mut status = self.status.write().await;
            *status = GatewayNodeStatus::ConfiguringApis;
        }

        let mut endpoints = Vec::new();

        for api_type in &self.config.exposed_apis {
            let endpoint = match api_type {
                ApiType::Rest => ApiEndpoint {
                    api_type: ApiType::Rest,
                    port: 8080,
                    base_path: "/api/v1".to_string(),
                    version: "1.0".to_string(),
                    ssl_enabled: false,
                    config: ApiEndpointConfig::Rest {
                        max_request_size: 10 * 1024 * 1024, // 10MB
                        timeout: Duration::from_secs(30),
                        compression: true,
                    },
                },
                ApiType::GraphQL => ApiEndpoint {
                    api_type: ApiType::GraphQL,
                    port: 8081,
                    base_path: "/graphql".to_string(),
                    version: "1.0".to_string(),
                    ssl_enabled: false,
                    config: ApiEndpointConfig::GraphQL {
                        introspection_enabled: false,
                        playground_enabled: false,
                        max_query_depth: 10,
                        max_query_complexity: 1000,
                    },
                },
                ApiType::WebSocket => ApiEndpoint {
                    api_type: ApiType::WebSocket,
                    port: 8082,
                    base_path: "/ws".to_string(),
                    version: "1.0".to_string(),
                    ssl_enabled: false,
                    config: ApiEndpointConfig::WebSocket {
                        max_connections: 1000,
                        ping_interval: Duration::from_secs(30),
                        message_size_limit: 1024 * 1024, // 1MB
                    },
                },
                ApiType::GRPC => ApiEndpoint {
                    api_type: ApiType::GRPC,
                    port: 8083,
                    base_path: "/".to_string(),
                    version: "1.0".to_string(),
                    ssl_enabled: false,
                    config: ApiEndpointConfig::GRPC {
                        max_message_size: 4 * 1024 * 1024, // 4MB
                        keepalive_time: Duration::from_secs(30),
                        keepalive_timeout: Duration::from_secs(5),
                    },
                },
                _ => continue,
            };
            endpoints.push(endpoint);
        }

        {
            let mut api_endpoints = self.api_endpoints.write().await;
            *api_endpoints = endpoints;
        }

        Ok(())
    }

    /// Traite une requête HTTP
    pub async fn handle_http_request(
        &self,
        client_ip: &str,
        api_key: Option<&str>,
        request_data: &[u8],
    ) -> Result<Vec<u8>> {
        // Vérifie le rate limiting
        let rate_limiter = self.rate_limiter.lock().await;
        if !rate_limiter.check_rate_limit(client_ip, api_key).await {
            return Err(crate::error::CoreError::RateLimited {
                message: "Rate limit exceeded".to_string(),
            });
        }
        drop(rate_limiter);

        // Sélectionne un backend
        let load_balancer = self.load_balancer.lock().await;
        let backend = load_balancer.select_backend(Some(client_ip)).await;
        drop(load_balancer);

        let backend_id = backend.ok_or_else(|| crate::error::CoreError::ServiceUnavailable {
            message: "No healthy backend available".to_string(),
        })?;

        // Simule le traitement de la requête
        // Dans la réalité, on forwarderait vers le backend sélectionné
        let response = b"Gateway response".to_vec();

        // Met à jour les métriques
        {
            let mut metrics = self.metrics.write().await;
            metrics.general.messages_processed += 1;
        }

        Ok(response)
    }

    /// Obtient les statistiques du Gateway
    pub async fn get_gateway_stats(&self) -> GatewayStats {
        let metrics = self.metrics.read().await;
        let endpoints = self.api_endpoints.read().await;

        GatewayStats {
            active_apis: endpoints.len() as u32,
            total_requests: metrics.general.messages_processed,
            cache_hit_ratio: metrics.cache_metrics.hit_ratio,
            rate_limit_blocks: metrics.rate_limiter_metrics.blocked_requests,
            security_incidents: metrics.security_metrics.attacks_detected,
            backend_health: BackendHealthSummary {
                healthy_backends: 0, // À calculer depuis load_balancer
                total_backends: self.config.backend_nodes.len() as u32,
                average_response_time: metrics.load_balancer_metrics.average_response_time,
            },
        }
    }
}

#[async_trait]
impl Node for GatewayNode {
    fn node_type(&self) -> NodeType {
        self.config.node_config.node_type.clone()
    }

    fn node_id(&self) -> &NodeId {
        &self.node_id
    }

    async fn start(&mut self) -> Result<()> {
        tracing::info!("Démarrage du Gateway Node: {:?}", self.node_id);

        // Configure les APIs
        self.configure_api_endpoints().await?;

        {
            let mut status = self.status.write().await;
            *status = GatewayNodeStatus::StartingServices;
        }

        // Démarre les services (simulation)
        // Dans la réalité, on démarrerait les serveurs HTTP, WebSocket, etc.

        {
            let mut status = self.status.write().await;
            *status = GatewayNodeStatus::Operational;
        }

        tracing::info!("Gateway Node démarré avec succès");
        Ok(())
    }

    async fn stop(&mut self) -> Result<()> {
        tracing::info!("Arrêt du Gateway Node: {:?}", self.node_id);

        {
            let mut status = self.status.write().await;
            *status = GatewayNodeStatus::Stopping;
        }

        // Arrête les services API
        // Vide les caches
        {
            let cache = self.cache_layer.lock().await;
            let mut content_cache = cache.content_cache.write().await;
            content_cache.clear();
            let mut metadata_cache = cache.metadata_cache.write().await;
            metadata_cache.clear();
        }

        tracing::info!("Gateway Node arrêté");
        Ok(())
    }

    async fn health_check(&self) -> Result<NodeHealth> {
        let status = self.status.read().await;
        let metrics = self.metrics.read().await;

        let health_status = match *status {
            GatewayNodeStatus::Operational => HealthStatus::Healthy,
            GatewayNodeStatus::Overloaded | GatewayNodeStatus::SecurityIssue => HealthStatus::Critical,
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
            _ => {
                // Le Gateway ne traite généralement pas les messages P2P directement
                // Il se contente de servir les APIs publiques
                Ok(None)
            }
        }
    }

    async fn sync_with_network(&mut self) -> Result<()> {
        // Le Gateway n'a pas besoin de synchronisation réseau particulière
        Ok(())
    }

    async fn update_config(&mut self, config: super::NodeConfiguration) -> Result<()> {
        self.config.node_config = config;
        Ok(())
    }
}

/// Statistiques du Gateway
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayStats {
    /// APIs actives
    pub active_apis: u32,
    /// Requêtes totales
    pub total_requests: u64,
    /// Ratio de hit du cache
    pub cache_hit_ratio: f64,
    /// Blocs par rate limiting
    pub rate_limit_blocks: u64,
    /// Incidents de sécurité
    pub security_incidents: u64,
    /// Santé des backends
    pub backend_health: BackendHealthSummary,
}

/// Résumé de santé des backends
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendHealthSummary {
    /// Backends en bonne santé
    pub healthy_backends: u32,
    /// Total de backends
    pub total_backends: u32,
    /// Temps de réponse moyen
    pub average_response_time: Duration,
}

impl GatewayNodeConfig {
    /// Valide la configuration
    pub fn validate(&self) -> Result<()> {
        if self.exposed_apis.is_empty() {
            return Err(crate::error::CoreError::Validation {
                message: "Au moins une API doit être exposée".to_string(),
            });
        }

        if self.cache_config.max_cache_size == 0 && self.cache_config.enabled {
            return Err(crate::error::CoreError::Validation {
                message: "Taille de cache doit être supérieure à 0 si activé".to_string(),
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
    fn test_gateway_node_config_validation() {
        let mut config = GatewayNodeConfig::default();
        assert!(config.validate().is_ok());

        // Test APIs vides
        config.exposed_apis.clear();
        assert!(config.validate().is_err());

        // Test cache mal configuré
        config.exposed_apis.push(ApiType::Rest);
        config.cache_config.enabled = true;
        config.cache_config.max_cache_size = 0;
        assert!(config.validate().is_err());

        config.cache_config.max_cache_size = 1_000_000;
        assert!(config.validate().is_ok());
    }

    #[tokio::test]
    async fn test_gateway_node_creation() {
        let config = GatewayNodeConfig::default();
        let keypair = generate_keypair().unwrap();

        let node = GatewayNode::new(config, keypair);
        assert!(node.is_ok());
    }

    #[tokio::test]
    async fn test_load_balancer() {
        let config = LoadBalancerConfig::default();
        let backend_nodes = vec![
            BackendNodeInfo {
                node_id: NodeId::from(Hash::zero()),
                address: "127.0.0.1:8080".parse().unwrap(),
                node_type: NodeType::FullArchive {
                    storage_capacity: 1000,
                    replication_factor: 5,
                },
                weight: 1,
                health_status: BackendHealthStatus::Healthy,
                last_health_check: SystemTime::now(),
                average_latency: Duration::from_millis(50),
                active_connections: 10,
            }
        ];

        let load_balancer = LoadBalancer::new(config, backend_nodes);
        let selected = load_balancer.select_backend(Some("192.168.1.1")).await;
        assert!(selected.is_some());
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimiterConfig::default();
        let rate_limiter = RateLimiter::new(config);

        // Première requête devrait passer
        assert!(rate_limiter.check_rate_limit("192.168.1.1", None).await);
    }

    #[tokio::test]
    async fn test_cache_layer() {
        let config = CacheConfig::default();
        let cache_layer = CacheLayer::new(config);

        let content_hash = Hash::zero();
        let data = b"test data".to_vec();

        // Cache le contenu
        cache_layer.cache_content(content_hash, data.clone(), None).await;

        // Récupère depuis le cache
        let cached_data = cache_layer.get_content(&content_hash).await;
        assert_eq!(cached_data, Some(data));
    }
}