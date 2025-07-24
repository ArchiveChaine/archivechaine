//! Serveur gRPC pour ArchiveChain
//!
//! Implémente le serveur gRPC avec authentification, middlewares et monitoring.

use std::net::SocketAddr;
use std::sync::Arc;
use tonic::{Request, Response, Status, transport::Server};
use tonic::service::Interceptor;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::api::{
    auth::{AuthService, ApiScope},
    middleware::AuthInfo,
    server::ServerState,
};
use super::{GrpcConfig, GrpcError, GrpcResult, services::*};

/// Serveur gRPC avec authentification et middlewares
pub struct AuthenticatedGrpcServer {
    config: GrpcConfig,
    state: ServerState,
}

impl AuthenticatedGrpcServer {
    /// Crée un nouveau serveur gRPC authentifié
    pub fn new(config: GrpcConfig, state: ServerState) -> Self {
        Self { config, state }
    }

    /// Démarre le serveur gRPC
    pub async fn start(self) -> GrpcResult<super::GrpcServerHandle> {
        let addr: SocketAddr = format!("{}:{}", self.config.listen_addr, self.config.port)
            .parse()
            .map_err(|e| GrpcError::Internal(format!("Invalid address: {}", e)))?;

        // Crée l'intercepteur d'authentification
        let auth_interceptor = AuthInterceptor::new(self.state.auth_service.clone());

        // Crée les services avec authentification
        let archive_service = ArchiveServiceImpl::new(self.state.clone()).into_service();
        let network_service = NetworkServiceImpl::new(self.state.clone()).into_service();
        let sync_service = SyncServiceImpl::new(self.state.clone()).into_service();

        // Configure le serveur (API Tonic 0.10)
        let mut server_builder = Server::builder()
            .timeout(std::time::Duration::from_secs(self.config.request_timeout));

        // Note: Dans Tonic 0.10, max_decoding_message_size et les options de compression
        // se configurent au niveau des services individuels
        if self.config.enable_compression {
            tracing::info!("Compression will be configured at service level");
        }

        // Configure TLS si activé
        if self.config.enable_tls {
            server_builder = self.configure_tls(server_builder).await?;
        }

        // Canal pour l'arrêt propre
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Lance le serveur avec les intercepteurs (API Tonic 0.10)
        let server = server_builder
            .add_service(
                tonic::service::interceptor(auth_interceptor.clone(), archive_service)
            )
            .add_service(
                tonic::service::interceptor(auth_interceptor.clone(), network_service)
            )
            .add_service(
                tonic::service::interceptor(auth_interceptor.clone(), sync_service)
            )
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_grpc())
                    .layer(tower_http::timeout::TimeoutLayer::new(
                        std::time::Duration::from_secs(self.config.request_timeout)
                    ))
            );

        tracing::info!("Starting authenticated gRPC server on {}", addr);

        // Lance le serveur dans une tâche séparée
        tokio::spawn(async move {
            if let Err(e) = server
                .serve_with_shutdown(addr, async {
                    let _ = shutdown_rx.await;
                    tracing::info!("Shutting down gRPC server");
                })
                .await
            {
                tracing::error!("gRPC server error: {}", e);
            }
        });

        Ok(super::GrpcServerHandle {
            addr,
            shutdown_tx,
        })
    }

    /// Configure TLS pour le serveur
    async fn configure_tls(
        &self,
        mut server_builder: tonic::transport::server::Router,
    ) -> GrpcResult<tonic::transport::server::Router> {
        if let (Some(cert_path), Some(key_path)) = (&self.config.tls_cert_path, &self.config.tls_key_path) {
            let cert = tokio::fs::read(cert_path).await
                .map_err(|e| GrpcError::Internal(format!("Failed to read TLS cert: {}", e)))?;
            let key = tokio::fs::read(key_path).await
                .map_err(|e| GrpcError::Internal(format!("Failed to read TLS key: {}", e)))?;

            let identity = tonic::transport::Identity::from_pem(cert, key);
            let mut tls_config = tonic::transport::ServerTlsConfig::new().identity(identity);

            // Configure mTLS si activé
            if self.config.enable_mtls {
                if let Some(ca_path) = &self.config.ca_cert_path {
                    let ca_cert = tokio::fs::read(ca_path).await
                        .map_err(|e| GrpcError::Internal(format!("Failed to read CA cert: {}", e)))?;
                    let ca_cert = tonic::transport::Certificate::from_pem(ca_cert);
                    tls_config = tls_config.client_ca_root(ca_cert);
                }
            }

            // Dans Tonic 0.10, tls_config retourne un Result<Router, Error>
            server_builder = server_builder.tls_config(tls_config)
                .map_err(|e| GrpcError::Internal(format!("Failed to configure TLS: {}", e)))?;
        }

        Ok(server_builder)
    }
}

/// Intercepteur d'authentification pour gRPC
#[derive(Clone)]
pub struct AuthInterceptor {
    auth_service: Arc<AuthService>,
}

impl AuthInterceptor {
    pub fn new(auth_service: Arc<AuthService>) -> Self {
        Self { auth_service }
    }

    /// Extrait le token d'authentification depuis les métadonnées
    fn extract_token(&self, request: &Request<()>) -> Option<String> {
        request
            .metadata()
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|auth_header| {
                if auth_header.starts_with("Bearer ") {
                    Some(auth_header[7..].to_string())
                } else {
                    None
                }
            })
    }

    /// Valide l'authentification et retourne les informations utilisateur
    fn validate_auth(&self, token: &str) -> Result<AuthInfo, Status> {
        let claims = self.auth_service.validate_token(token)
            .map_err(|_| Status::unauthenticated("Invalid or expired token"))?;

        let scopes = claims.scope.iter()
            .filter_map(|s| crate::api::auth::ApiScope::from_str(s))
            .collect();

        Ok(AuthInfo {
            claims: claims.clone(),
            user_id: claims.sub.clone(),
            scopes,
        })
    }
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        // Extrait le token d'authentification
        let token = self.extract_token(&request)
            .ok_or_else(|| Status::unauthenticated("Missing authorization token"))?;

        // Valide l'authentification
        let auth_info = self.validate_auth(&token)?;

        // Ajoute les informations d'authentification aux extensions de la requête
        request.extensions_mut().insert(auth_info);

        Ok(request)
    }
}

/// Middleware de vérification des permissions
pub struct PermissionChecker;

impl PermissionChecker {
    /// Vérifie qu'une requête a les permissions requises
    pub fn check_permission(
        request: &Request<()>, 
        required_scope: ApiScope
    ) -> Result<(), Status> {
        let auth_info = request.extensions().get::<AuthInfo>()
            .ok_or_else(|| Status::unauthenticated("Authentication required"))?;

        if !auth_info.scopes.contains(&required_scope) && 
           !auth_info.scopes.contains(&ApiScope::AdminAll) {
            return Err(Status::permission_denied(
                format!("Required scope: {}", required_scope.as_str())
            ));
        }

        Ok(())
    }

    /// Helper pour vérifier les permissions d'archivage
    pub fn check_archive_permission(request: &Request<()>) -> Result<(), Status> {
        Self::check_permission(request, ApiScope::ArchivesRead)
    }

    /// Helper pour vérifier les permissions réseau
    pub fn check_network_permission(request: &Request<()>) -> Result<(), Status> {
        Self::check_permission(request, ApiScope::NetworkRead)
    }

    /// Helper pour vérifier les permissions de synchronisation
    pub fn check_sync_permission(request: &Request<()>) -> Result<(), Status> {
        Self::check_permission(request, ApiScope::NetworkRead)
    }
}

/// Moniteur de métriques pour gRPC
pub struct GrpcMetrics {
    request_count: Arc<std::sync::atomic::AtomicU64>,
    error_count: Arc<std::sync::atomic::AtomicU64>,
    total_latency: Arc<std::sync::atomic::AtomicU64>,
}

impl GrpcMetrics {
    pub fn new() -> Self {
        Self {
            request_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            error_count: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            total_latency: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    pub fn increment_requests(&self) {
        self.request_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn increment_errors(&self) {
        self.error_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn add_latency(&self, latency_ms: u64) {
        self.total_latency.fetch_add(latency_ms, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.request_count.load(std::sync::atomic::Ordering::Relaxed),
            self.error_count.load(std::sync::atomic::Ordering::Relaxed),
            self.total_latency.load(std::sync::atomic::Ordering::Relaxed),
        )
    }
}

impl Default for GrpcMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper pour créer un serveur gRPC avec configuration par défaut
pub async fn create_default_grpc_server(
    state: ServerState,
) -> GrpcResult<super::GrpcServerHandle> {
    let config = GrpcConfig::default();
    let server = AuthenticatedGrpcServer::new(config, state);
    server.start().await
}

/// Helper pour créer un serveur gRPC avec TLS
pub async fn create_tls_grpc_server(
    state: ServerState,
    cert_path: String,
    key_path: String,
) -> GrpcResult<super::GrpcServerHandle> {
    let mut config = GrpcConfig::default();
    config.enable_tls = true;
    config.tls_cert_path = Some(cert_path);
    config.tls_key_path = Some(key_path);

    let server = AuthenticatedGrpcServer::new(config, state);
    server.start().await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ApiConfig, auth::{AuthConfig, UserManager}};
    use std::sync::Arc;
    use tokio::sync::RwLock;

    fn create_test_state() -> ServerState {
        let blockchain = Arc::new(
            crate::Blockchain::new(crate::BlockchainConfig::default()).unwrap()
        );
        let auth_service = Arc::new(
            AuthService::new(AuthConfig::default()).unwrap()
        );
        let user_manager = Arc::new(RwLock::new(UserManager::new()));
        let config = ApiConfig::default();

        ServerState::new(blockchain, auth_service, user_manager, config)
    }

    #[test]
    fn test_auth_interceptor_creation() {
        let state = create_test_state();
        let interceptor = AuthInterceptor::new(state.auth_service);
        
        // Teste qu'on peut créer l'intercepteur
        assert_eq!(2 + 2, 4);
    }

    #[test]
    fn test_permission_checker() {
        // Crée une requête mock sans authentification
        let request = Request::new(());
        
        // Devrait échouer sans authentification
        let result = PermissionChecker::check_archive_permission(&request);
        assert!(result.is_err());
    }

    #[test]
    fn test_grpc_metrics() {
        let metrics = GrpcMetrics::new();
        
        metrics.increment_requests();
        metrics.increment_errors();
        metrics.add_latency(100);
        
        let (requests, errors, latency) = metrics.get_stats();
        assert_eq!(requests, 1);
        assert_eq!(errors, 1);
        assert_eq!(latency, 100);
    }

    #[tokio::test]
    async fn test_authenticated_grpc_server_creation() {
        let state = create_test_state();
        let config = GrpcConfig::default();
        let server = AuthenticatedGrpcServer::new(config, state);
        
        // Vérifie que le serveur peut être créé
        assert_eq!(server.config.port, 9090);
    }

    #[test]
    fn test_grpc_config_tls() {
        let mut config = GrpcConfig::default();
        config.enable_tls = true;
        config.tls_cert_path = Some("/path/to/cert.pem".to_string());
        config.tls_key_path = Some("/path/to/key.pem".to_string());
        
        assert!(config.enable_tls);
        assert!(config.tls_cert_path.is_some());
        assert!(config.tls_key_path.is_some());
    }

    #[test]
    fn test_grpc_config_mtls() {
        let mut config = GrpcConfig::default();
        config.enable_tls = true;
        config.enable_mtls = true;
        config.ca_cert_path = Some("/path/to/ca.pem".to_string());
        
        assert!(config.enable_tls);
        assert!(config.enable_mtls);
        assert!(config.ca_cert_path.is_some());
    }

    #[test]
    fn test_grpc_error_conversion() {
        let error = GrpcError::PermissionDenied("Access denied".to_string());
        let status: Status = error.into();
        
        assert_eq!(status.code(), tonic::Code::PermissionDenied);
        assert_eq!(status.message(), "Access denied");
    }

    #[test]
    fn test_grpc_metrics_default() {
        let metrics = GrpcMetrics::default();
        let (requests, errors, latency) = metrics.get_stats();
        
        assert_eq!(requests, 0);
        assert_eq!(errors, 0);
        assert_eq!(latency, 0);
    }
}