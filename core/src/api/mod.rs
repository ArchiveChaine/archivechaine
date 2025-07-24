//! Module API pour ArchiveChain
//!
//! Ce module fournit toutes les interfaces API pour interagir avec ArchiveChain :
//! - REST API pour les intégrations tierces
//! - GraphQL API pour les requêtes flexibles
//! - WebSocket API pour la communication temps réel
//! - gRPC API pour la communication haute performance
//! - P2P Protocol pour la communication entre nœuds

pub mod types;
pub mod auth;
pub mod server;
pub mod middleware;
pub mod rest;
pub mod graphql;
pub mod websocket;
pub mod grpc;
pub mod p2p;
pub mod error;

// Re-exports publics
pub use types::*;
pub use auth::{AuthService, JwtClaims, AuthError, TokenInfo};
pub use server::{ApiServer, ServerConfig, ServerHandle};
pub use middleware::{
    MiddlewareConfig, CorsConfig, RateLimitConfig, CompressionConfig, LoggingConfig,
    cors_middleware, compression_middleware, tracing_middleware
};
pub use error::{ApiError, ApiResult};

// Configuration générale de l'API
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiConfig {
    /// Configuration du serveur HTTP
    pub server: ServerConfig,
    
    /// Configuration de l'authentification JWT
    pub auth: auth::AuthConfig,
    
    /// Configuration des middlewares
    pub middleware: middleware::MiddlewareConfig,
    
    /// Configuration REST API
    pub rest: rest::RestConfig,
    
    /// Configuration GraphQL
    pub graphql: graphql::GraphQLConfig,
    
    /// Configuration WebSocket
    pub websocket: websocket::WebSocketConfig,
    
    /// Configuration gRPC
    pub grpc: grpc::GrpcConfig,
    
    /// Configuration P2P
    pub p2p: p2p::P2PConfig,
}

impl Default for ApiConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: auth::AuthConfig::default(),
            middleware: middleware::MiddlewareConfig::default(),
            rest: rest::RestConfig::default(),
            graphql: graphql::GraphQLConfig::default(),
            websocket: websocket::WebSocketConfig::default(),
            grpc: grpc::GrpcConfig::default(),
            p2p: p2p::P2PConfig::default(),
        }
    }
}

/// Informations de version de l'API
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ApiVersion {
    pub version: String,
    pub build_date: String,
    pub commit_hash: String,
    pub supported_formats: Vec<String>,
}

impl Default for ApiVersion {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            build_date: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC").to_string(),
            commit_hash: "dev".to_string(),
            supported_formats: vec![
                "application/json".to_string(),
                "application/cbor".to_string(),
                "application/x-protobuf".to_string(),
            ],
        }
    }
}

/// Health check pour l'API
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub uptime: String,
    pub checks: std::collections::HashMap<String, String>,
}

impl HealthStatus {
    pub fn healthy() -> Self {
        let mut checks = std::collections::HashMap::new();
        checks.insert("database".to_string(), "healthy".to_string());
        checks.insert("blockchain".to_string(), "healthy".to_string());
        checks.insert("storage".to_string(), "healthy".to_string());
        checks.insert("network".to_string(), "healthy".to_string());
        
        Self {
            status: "healthy".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            timestamp: chrono::Utc::now(),
            uptime: "0s".to_string(), // À calculer dynamiquement
            checks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_config_default() {
        let config = ApiConfig::default();
        assert_eq!(config.server.host, "127.0.0.1");
    }

    #[test]
    fn test_api_version() {
        let version = ApiVersion::default();
        assert!(!version.version.is_empty());
        assert!(version.supported_formats.contains(&"application/json".to_string()));
    }

    #[test]
    fn test_health_status() {
        let health = HealthStatus::healthy();
        assert_eq!(health.status, "healthy");
        assert!(health.checks.contains_key("database"));
    }
}