//! Serveur HTTP principal pour ArchiveChain API
//!
//! Ce module contient le serveur HTTP principal qui orchestre toutes les APIs :
//! REST, GraphQL, WebSocket et gRPC. Il configure le routage, les middlewares
//! et gère le cycle de vie du serveur.

use crate::api::{
    ApiConfig, ApiError, ApiResult, ApiVersion, HealthStatus,
    auth::{AuthService, UserManager},
    middleware::{MiddlewareState, RateLimiters, cors_middleware, compression_middleware, tracing_middleware},
    rest,
    graphql,
    websocket,
};
use crate::{Blockchain, BlockchainConfig};
use axum::{
    extract::{State, Path},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::{net::SocketAddr, sync::Arc, time::SystemTime};
use tokio::net::TcpListener;
use tower::ServiceBuilder;
use tower_http::timeout::TimeoutLayer;
use tracing::{info, error};

/// Configuration du serveur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Adresse d'écoute
    pub host: String,
    /// Port d'écoute
    pub port: u16,
    /// Timeout des requêtes (en secondes)
    pub request_timeout: u64,
    /// Taille maximum du body (en bytes)
    pub max_body_size: usize,
    /// Configuration TLS
    pub tls: Option<TlsConfig>,
    /// Mode de développement
    pub dev_mode: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 8080,
            request_timeout: 30,
            max_body_size: 16 * 1024 * 1024, // 16MB
            tls: None,
            dev_mode: false,
        }
    }
}

/// Configuration TLS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub cert_path: String,
    pub key_path: String,
    pub ca_cert_path: Option<String>,
}

/// État partagé du serveur
#[derive(Clone)]
pub struct ServerState {
    pub blockchain: Arc<Blockchain>,
    pub auth_service: Arc<AuthService>,
    pub user_manager: Arc<tokio::sync::RwLock<UserManager>>,
    pub config: ApiConfig,
    pub start_time: SystemTime,
    pub version: ApiVersion,
}

impl ServerState {
    pub fn new(
        blockchain: Arc<Blockchain>,
        auth_service: Arc<AuthService>,
        user_manager: Arc<tokio::sync::RwLock<UserManager>>,
        config: ApiConfig,
    ) -> Self {
        Self {
            blockchain,
            auth_service,
            user_manager,
            config,
            start_time: SystemTime::now(),
            version: ApiVersion::default(),
        }
    }
}

/// Handle du serveur pour le contrôler
pub struct ServerHandle {
    pub addr: SocketAddr,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl ServerHandle {
    /// Arrête le serveur proprement
    pub fn shutdown(self) -> Result<(), ()> {
        self.shutdown_tx.send(()).map_err(|_| ())
    }

    /// Retourne l'adresse d'écoute du serveur
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Serveur API principal
pub struct ApiServer {
    config: ApiConfig,
    state: ServerState,
}

impl ApiServer {
    /// Crée un nouveau serveur API
    pub async fn new(
        config: ApiConfig,
        blockchain: Arc<Blockchain>,
    ) -> ApiResult<Self> {
        // Initialise l'authentification
        let auth_service = Arc::new(AuthService::new(config.auth.clone())?);
        let user_manager = Arc::new(tokio::sync::RwLock::new(UserManager::new()));

        // Crée l'état du serveur
        let state = ServerState::new(
            blockchain,
            auth_service,
            user_manager,
            config.clone(),
        );

        Ok(Self { config, state })
    }

    /// Démarre le serveur
    pub async fn start(self) -> ApiResult<ServerHandle> {
        let addr = SocketAddr::from((
            self.config.server.host.parse::<std::net::IpAddr>()
                .map_err(|e| ApiError::internal(format!("Invalid host: {}", e)))?,
            self.config.server.port,
        ));

        // Crée l'application avec tous les routes
        let app = self.create_app().await?;

        // Crée le listener
        let listener = TcpListener::bind(addr).await
            .map_err(|e| ApiError::internal(format!("Failed to bind to {}: {}", addr, e)))?;

        let actual_addr = listener.local_addr()
            .map_err(|e| ApiError::internal(format!("Failed to get local address: {}", e)))?;

        info!("API server starting on {}", actual_addr);

        // Canal pour l'arrêt propre
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::oneshot::channel();

        // Lance le serveur
        let server_future = async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.recv().await;
                    info!("Shutting down API server gracefully");
                })
                .await
        };

        // Lance le serveur dans une tâche séparée
        tokio::spawn(async move {
            if let Err(e) = server_future.await {
                error!("Server error: {}", e);
            }
        });

        info!("API server started successfully on {}", actual_addr);

        Ok(ServerHandle {
            addr: actual_addr,
            shutdown_tx,
        })
    }

    /// Crée l'application Axum avec tous les routes et middlewares
    async fn create_app(&self) -> ApiResult<Router> {
        // État pour les middlewares
        let middleware_state = MiddlewareState {
            auth_service: self.state.auth_service.clone(),
            rate_limiters: Arc::new(RateLimiters::new(&self.config.middleware.rate_limit)),
            config: self.config.middleware.clone(),
        };

        // Routes publiques (sans authentification)
        let public_routes = Router::new()
            .route("/health", get(health_check))
            .route("/version", get(version_info))
            .route("/metrics", get(metrics));

        // Routes API avec authentification
        let api_routes = Router::new()
            .nest("/rest", rest::create_routes().await?)
            .nest("/graphql", graphql::create_routes().await?)
            .nest("/ws", websocket::create_routes().await?)
            .layer(axum::middleware::from_fn_with_state(
                middleware_state.clone(),
                crate::api::middleware::auth_middleware,
            ));

        // Combine tous les routes
        let app = Router::new()
            .nest("/api/v1", api_routes)
            .merge(public_routes)
            .with_state(self.state.clone())
            // Middlewares globaux
            .layer(
                ServiceBuilder::new()
                    .layer(TimeoutLayer::new(std::time::Duration::from_secs(self.config.server.request_timeout)))
                    .layer(axum::middleware::from_fn(crate::api::middleware::request_id_middleware))
                    .layer(axum::middleware::from_fn(crate::api::middleware::logging_middleware))
                    .layer(axum::middleware::from_fn(crate::api::middleware::error_handler_middleware))
                    .layer(axum::middleware::from_fn_with_state(
                        middleware_state,
                        crate::api::middleware::rate_limit_middleware,
                    ))
            );

        // Ajoute CORS si configuré
        let app = if let Some(cors_layer) = cors_middleware(&self.config.middleware.cors) {
            app.layer(cors_layer)
        } else {
            app
        };

        // Ajoute compression si configurée
        let app = if let Some(compression_layer) = compression_middleware(&self.config.middleware.compression) {
            app.layer(compression_layer)
        } else {
            app
        };

        // Ajoute tracing si configuré
        let app = if let Some(tracing_layer) = tracing_middleware(&self.config.middleware.logging) {
            app.layer(tracing_layer)
        } else {
            app
        };

        Ok(app)
    }
}

/// Handler pour le health check
async fn health_check(State(state): State<ServerState>) -> Json<HealthStatus> {
    let mut health = HealthStatus::healthy();
    
    // Calcule l'uptime
    if let Ok(uptime) = state.start_time.elapsed() {
        health.uptime = format_duration(uptime);
    }

    // Vérifie l'état de la blockchain
    match state.blockchain.get_stats() {
        Ok(_stats) => {
            health.checks.insert("blockchain".to_string(), "healthy".to_string());
        }
        Err(_) => {
            health.status = "degraded".to_string();
            health.checks.insert("blockchain".to_string(), "unhealthy".to_string());
        }
    }

    Json(health)
}

/// Handler pour les informations de version
async fn version_info(State(state): State<ServerState>) -> Json<ApiVersion> {
    Json(state.version.clone())
}

/// Handler pour les métriques Prometheus
async fn metrics() -> Result<String, ApiError> {
    // Ici on pourrait intégrer des métriques Prometheus
    // Pour l'instant, on retourne un placeholder
    Ok(format!(
        "# HELP api_requests_total Total number of API requests\n\
         # TYPE api_requests_total counter\n\
         api_requests_total{{method=\"GET\",endpoint=\"/health\",status=\"200\"}} 1\n\
         \n\
         # HELP api_request_duration_seconds API request duration\n\
         # TYPE api_request_duration_seconds histogram\n\
         api_request_duration_seconds_bucket{{le=\"0.1\"}} 100\n\
         api_request_duration_seconds_bucket{{le=\"0.5\"}} 1000\n\
         api_request_duration_seconds_bucket{{le=\"+Inf\"}} 1000\n\
         api_request_duration_seconds_sum 45.0\n\
         api_request_duration_seconds_count 1000\n"
    ))
}

/// Formate une durée en format lisible
fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let days = total_seconds / 86400;
    let hours = (total_seconds % 86400) / 3600;
    let minutes = (total_seconds % 3600) / 60;
    let seconds = total_seconds % 60;

    if days > 0 {
        format!("{}d {}h {}m {}s", days, hours, minutes, seconds)
    } else if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, seconds)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}

/// Builder pour la configuration du serveur
#[derive(Debug, Default)]
pub struct ServerBuilder {
    config: ApiConfig,
    blockchain_config: Option<BlockchainConfig>,
}

impl ServerBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_config(mut self, config: ApiConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_host(mut self, host: String) -> Self {
        self.config.server.host = host;
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.config.server.port = port;
        self
    }

    pub fn with_dev_mode(mut self, dev_mode: bool) -> Self {
        self.config.server.dev_mode = dev_mode;
        self
    }

    pub fn with_blockchain_config(mut self, blockchain_config: BlockchainConfig) -> Self {
        self.blockchain_config = Some(blockchain_config);
        self
    }

    pub async fn build(self) -> ApiResult<ApiServer> {
        // Crée la blockchain
        let blockchain_config = self.blockchain_config.unwrap_or_default();
        let blockchain = Arc::new(
            Blockchain::new(blockchain_config)
                .map_err(|e| ApiError::internal(format!("Failed to create blockchain: {}", e)))?
        );

        ApiServer::new(self.config, blockchain).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BlockchainConfig;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.request_timeout, 30);
        assert!(!config.dev_mode);
    }

    #[test]
    fn test_format_duration() {
        let duration = std::time::Duration::from_secs(3661); // 1h 1m 1s
        assert_eq!(format_duration(duration), "1h 1m 1s");

        let duration = std::time::Duration::from_secs(90061); // 1d 1h 1m 1s
        assert_eq!(format_duration(duration), "1d 1h 1m 1s");

        let duration = std::time::Duration::from_secs(61); // 1m 1s
        assert_eq!(format_duration(duration), "1m 1s");

        let duration = std::time::Duration::from_secs(30); // 30s
        assert_eq!(format_duration(duration), "30s");
    }

    #[test]
    fn test_server_builder() {
        let builder = ServerBuilder::new()
            .with_host("0.0.0.0".to_string())
            .with_port(9090)
            .with_dev_mode(true);

        assert_eq!(builder.config.server.host, "0.0.0.0");
        assert_eq!(builder.config.server.port, 9090);
        assert!(builder.config.server.dev_mode);
    }

    #[tokio::test]
    async fn test_server_state_creation() {
        let blockchain_config = BlockchainConfig::default();
        let blockchain = Arc::new(Blockchain::new(blockchain_config).unwrap());
        let auth_config = crate::api::auth::AuthConfig::default();
        let auth_service = Arc::new(AuthService::new(auth_config).unwrap());
        let user_manager = Arc::new(tokio::sync::RwLock::new(UserManager::new()));
        let api_config = ApiConfig::default();

        let state = ServerState::new(
            blockchain,
            auth_service,
            user_manager,
            api_config,
        );

        assert_eq!(state.version.version, env!("CARGO_PKG_VERSION"));
    }

    #[tokio::test]
    async fn test_health_status() {
        let health = HealthStatus::healthy();
        assert_eq!(health.status, "healthy");
        assert!(health.checks.contains_key("database"));
        assert!(health.checks.contains_key("blockchain"));
        assert!(health.checks.contains_key("storage"));
        assert!(health.checks.contains_key("network"));
    }

    #[test]
    fn test_tls_config() {
        let tls_config = TlsConfig {
            cert_path: "/path/to/cert.pem".to_string(),
            key_path: "/path/to/key.pem".to_string(),
            ca_cert_path: Some("/path/to/ca.pem".to_string()),
        };

        assert_eq!(tls_config.cert_path, "/path/to/cert.pem");
        assert_eq!(tls_config.key_path, "/path/to/key.pem");
        assert_eq!(tls_config.ca_cert_path, Some("/path/to/ca.pem".to_string()));
    }
}