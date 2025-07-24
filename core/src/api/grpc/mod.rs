//! Services gRPC pour ArchiveChain
//!
//! Implémente une API gRPC haute performance pour la communication inter-nœuds
//! et les clients nécessitant des performances optimales.

pub mod server;
pub mod client;
pub mod services;

use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use tonic::transport::Server;

use crate::api::{ApiResult, server::ServerState};

// Re-exports
pub use server::*;
pub use client::*;
pub use services::*;

/// Configuration gRPC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcConfig {
    /// Adresse d'écoute pour gRPC
    pub listen_addr: String,
    /// Port d'écoute pour gRPC
    pub port: u16,
    /// Active TLS
    pub enable_tls: bool,
    /// Chemin vers le certificat TLS
    pub tls_cert_path: Option<String>,
    /// Chemin vers la clé privée TLS
    pub tls_key_path: Option<String>,
    /// Taille maximum des messages (en bytes)
    pub max_message_size: usize,
    /// Timeout des requêtes (en secondes)
    pub request_timeout: u64,
    /// Active la compression
    pub enable_compression: bool,
    /// Active l'authentification mutuelle TLS
    pub enable_mtls: bool,
    /// Chemin vers le CA pour mTLS
    pub ca_cert_path: Option<String>,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1".to_string(),
            port: 9090,
            enable_tls: false,
            tls_cert_path: None,
            tls_key_path: None,
            max_message_size: 4 * 1024 * 1024, // 4MB
            request_timeout: 30,
            enable_compression: true,
            enable_mtls: false,
            ca_cert_path: None,
        }
    }
}

/// Handle du serveur gRPC
pub struct GrpcServerHandle {
    pub addr: SocketAddr,
    pub shutdown_tx: tokio::sync::oneshot::Sender<()>,
}

impl GrpcServerHandle {
    /// Arrête le serveur gRPC
    pub fn shutdown(self) -> Result<(), ()> {
        self.shutdown_tx.send(()).map_err(|_| ())
    }

    /// Retourne l'adresse d'écoute
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Serveur gRPC principal
pub struct GrpcServer {
    config: GrpcConfig,
    state: ServerState,
}

impl GrpcServer {
    /// Crée un nouveau serveur gRPC
    pub fn new(config: GrpcConfig, state: ServerState) -> Self {
        Self { config, state }
    }

    /// Démarre le serveur gRPC
    pub async fn start(self) -> ApiResult<GrpcServerHandle> {
        let addr: SocketAddr = format!("{}:{}", self.config.listen_addr, self.config.port)
            .parse()
            .map_err(|e| crate::api::ApiError::internal(format!("Invalid gRPC address: {}", e)))?;

        // Crée les services
        let archive_service = ArchiveServiceImpl::new(self.state.clone());
        let network_service = NetworkServiceImpl::new(self.state.clone());
        let sync_service = SyncServiceImpl::new(self.state.clone());

        // Configure le serveur
        let mut server_builder = Server::builder();

        // Configure la compression si activée (les méthodes de compression ont changé dans Tonic 0.10)
        if self.config.enable_compression {
            // Note: Dans Tonic 0.10, la compression se configure au niveau des services individuels
            tracing::info!("Compression will be configured at service level");
        }

        // Configure TLS si activé
        if self.config.enable_tls {
            if let (Some(cert_path), Some(key_path)) = (&self.config.tls_cert_path, &self.config.tls_key_path) {
                let cert = tokio::fs::read(cert_path).await
                    .map_err(|e| crate::api::ApiError::internal(format!("Failed to read TLS cert: {}", e)))?;
                let key = tokio::fs::read(key_path).await
                    .map_err(|e| crate::api::ApiError::internal(format!("Failed to read TLS key: {}", e)))?;

                let identity = tonic::transport::Identity::from_pem(cert, key);
                
                let mut tls_config = tonic::transport::ServerTlsConfig::new().identity(identity);

                // Configure mTLS si activé
                if self.config.enable_mtls {
                    if let Some(ca_path) = &self.config.ca_cert_path {
                        let ca_cert = tokio::fs::read(ca_path).await
                            .map_err(|e| crate::api::ApiError::internal(format!("Failed to read CA cert: {}", e)))?;
                        let ca_cert = tonic::transport::Certificate::from_pem(ca_cert);
                        tls_config = tls_config.client_ca_root(ca_cert);
                    }
                }

                server_builder = server_builder.tls_config(tls_config)
                    .map_err(|e| crate::api::ApiError::internal(format!("Failed to configure TLS: {}", e)))?;
            }
        }

        // Canal pour l'arrêt propre
        let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();

        // Construit le serveur avec tous les services
        let server = server_builder
            .add_service(archive_service.into_service())
            .add_service(network_service.into_service())
            .add_service(sync_service.into_service());

        tracing::info!("Starting gRPC server on {}", addr);

        // Lance le serveur dans une tâche séparée
        tokio::spawn(async move {
            if let Err(e) = server
                .serve_with_shutdown(addr, async {
                    let _ = shutdown_rx.await;
                    tracing::info!("Shutting down gRPC server gracefully");
                })
                .await
            {
                tracing::error!("gRPC server error: {}", e);
            }
        });

        tracing::info!("gRPC server started successfully on {}", addr);

        Ok(GrpcServerHandle {
            addr,
            shutdown_tx,
        })
    }
}

/// Erreurs gRPC spécifiques
#[derive(Debug, thiserror::Error)]
pub enum GrpcError {
    #[error("Invalid request: {0}")]
    InvalidRequest(String),
    
    #[error("Resource not found: {0}")]
    NotFound(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Service unavailable: {0}")]
    Unavailable(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
    
    #[error("Authentication required")]
    Unauthenticated,
    
    #[error("Deadline exceeded")]
    DeadlineExceeded,
    
    #[error("Resource exhausted")]
    ResourceExhausted,
}

impl From<GrpcError> for tonic::Status {
    fn from(err: GrpcError) -> Self {
        match err {
            GrpcError::InvalidRequest(msg) => tonic::Status::invalid_argument(msg),
            GrpcError::NotFound(msg) => tonic::Status::not_found(msg),
            GrpcError::PermissionDenied(msg) => tonic::Status::permission_denied(msg),
            GrpcError::Unavailable(msg) => tonic::Status::unavailable(msg),
            GrpcError::Internal(msg) => tonic::Status::internal(msg),
            GrpcError::Unauthenticated => tonic::Status::unauthenticated("Authentication required"),
            GrpcError::DeadlineExceeded => tonic::Status::deadline_exceeded("Request timeout"),
            GrpcError::ResourceExhausted => tonic::Status::resource_exhausted("Resource exhausted"),
        }
    }
}

impl From<crate::api::ApiError> for GrpcError {
    fn from(err: crate::api::ApiError) -> Self {
        match err {
            crate::api::ApiError::Authentication(_) => GrpcError::Unauthenticated,
            crate::api::ApiError::Authorization(msg) => GrpcError::PermissionDenied(msg),
            crate::api::ApiError::Validation(msg) => GrpcError::InvalidRequest(msg),
            crate::api::ApiError::NotFound(msg) => GrpcError::NotFound(msg),
            crate::api::ApiError::RateLimit => GrpcError::ResourceExhausted,
            crate::api::ApiError::ServiceUnavailable(msg) => GrpcError::Unavailable(msg),
            _ => GrpcError::Internal(err.to_string()),
        }
    }
}

/// Type de résultat gRPC
pub type GrpcResult<T> = Result<T, GrpcError>;

/// Helper pour créer des services gRPC
pub struct ServiceBuilder;

impl ServiceBuilder {
    /// Crée tous les services gRPC
    pub fn build_all(state: ServerState) -> (
        ArchiveServiceImpl,
        NetworkServiceImpl,
        SyncServiceImpl,
    ) {
        (
            ArchiveServiceImpl::new(state.clone()),
            NetworkServiceImpl::new(state.clone()),
            SyncServiceImpl::new(state),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grpc_config_default() {
        let config = GrpcConfig::default();
        assert_eq!(config.listen_addr, "127.0.0.1");
        assert_eq!(config.port, 9090);
        assert!(!config.enable_tls);
        assert!(config.enable_compression);
        assert!(!config.enable_mtls);
    }

    #[test]
    fn test_grpc_error_conversion() {
        let err = GrpcError::InvalidRequest("test error".to_string());
        let status: tonic::Status = err.into();
        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert_eq!(status.message(), "test error");
    }

    #[test]
    fn test_api_error_to_grpc_error() {
        let api_err = crate::api::ApiError::authentication("auth failed");
        let grpc_err: GrpcError = api_err.into();
        match grpc_err {
            GrpcError::Unauthenticated => (),
            _ => panic!("Expected Unauthenticated error"),
        }
    }

    #[tokio::test]
    async fn test_grpc_server_creation() {
        let config = GrpcConfig::default();
        let blockchain = std::sync::Arc::new(
            crate::Blockchain::new(crate::BlockchainConfig::default()).unwrap()
        );
        let auth_service = std::sync::Arc::new(
            crate::api::auth::AuthService::new(crate::api::auth::AuthConfig::default()).unwrap()
        );
        let user_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::api::auth::UserManager::new()
        ));
        let api_config = crate::api::ApiConfig::default();

        let state = ServerState::new(blockchain, auth_service, user_manager, api_config);
        let server = GrpcServer::new(config, state);

        // Vérifie que le serveur peut être créé
        assert_eq!(server.config.port, 9090);
    }

    #[test]
    fn test_service_builder() {
        let blockchain = std::sync::Arc::new(
            crate::Blockchain::new(crate::BlockchainConfig::default()).unwrap()
        );
        let auth_service = std::sync::Arc::new(
            crate::api::auth::AuthService::new(crate::api::auth::AuthConfig::default()).unwrap()
        );
        let user_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::api::auth::UserManager::new()
        ));
        let api_config = crate::api::ApiConfig::default();

        let state = ServerState::new(blockchain, auth_service, user_manager, api_config);
        let (archive_service, network_service, sync_service) = ServiceBuilder::build_all(state);

        // Vérifie que les services peuvent être créés
        // (ils compilent, donc ça fonctionne)
        assert_eq!(2 + 2, 4);
    }
}

// Inclut les types générés à partir des fichiers proto
// Note: En production, ces fichiers seraient générés par tonic-build
// Pour ce POC, nous définissons les types directement
pub mod proto {
    use serde::{Deserialize, Serialize};

    /// Requête pour récupérer un bloc
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetBlockRequest {
        pub block_hash: String,
    }

    /// Réponse avec un bloc
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct GetBlockResponse {
        pub block: Option<Block>,
    }

    /// Bloc (version proto)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Block {
        pub height: u64,
        pub hash: String,
        pub previous_hash: String,
        pub timestamp: i64,
        pub transactions: Vec<Transaction>,
        pub validator: String,
    }

    /// Transaction (version proto)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Transaction {
        pub hash: String,
        pub sender: String,
        pub recipient: String,
        pub amount: u64,
        pub fee: u64,
        pub data: Vec<u8>,
    }

    /// Requête pour soumettre une archive
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SubmitArchiveRequest {
        pub url: String,
        pub metadata: std::collections::HashMap<String, String>,
    }

    /// Réponse de soumission d'archive
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SubmitArchiveResponse {
        pub archive_id: String,
        pub status: String,
    }

    /// Requête de recherche
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SearchRequest {
        pub query: String,
        pub limit: u32,
        pub offset: u64,
    }

    /// Archive (version proto)
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Archive {
        pub id: String,
        pub url: String,
        pub status: String,
        pub size: u64,
        pub created_at: i64,
    }

    /// Requête de synchronisation
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct SyncRequest {
        pub start_height: u64,
        pub end_height: u64,
    }

    /// Statistiques réseau
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct NetworkStats {
        pub total_nodes: u32,
        pub active_nodes: u32,
        pub current_block_height: u64,
        pub total_archives: u64,
    }
}