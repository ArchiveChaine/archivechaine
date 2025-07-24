//! API REST pour ArchiveChain
//!
//! Ce module implémente l'API REST complète selon les spécifications,
//! incluant tous les endpoints pour les archives, la recherche, les statistiques
//! du réseau et la gestion des nœuds.

pub mod routes;
pub mod handlers;
pub mod validation;

use axum::Router;
use serde::{Deserialize, Serialize};
use crate::api::{ApiResult, server::ServerState};

// Re-exports
pub use routes::create_routes;
pub use handlers::*;
pub use validation::*;

/// Configuration de l'API REST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestConfig {
    /// URL de base pour les liens
    pub base_url: String,
    /// URL de base pour les gateways d'accès aux archives
    pub gateway_url: String,
    /// Pagination par défaut
    pub default_page_size: u32,
    /// Taille maximum de page
    pub max_page_size: u32,
    /// Timeout pour les opérations d'archivage
    pub archive_timeout: u64,
    /// Activation de la documentation OpenAPI
    pub enable_openapi: bool,
}

impl Default for RestConfig {
    fn default() -> Self {
        Self {
            base_url: "https://api.archivechain.org/v1".to_string(),
            gateway_url: "https://gateway.archivechain.org".to_string(),
            default_page_size: 20,
            max_page_size: 100,
            archive_timeout: 300, // 5 minutes
            enable_openapi: true,
        }
    }
}

/// Paramètres de pagination standard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationParams {
    #[serde(default = "default_page")]
    pub page: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_page() -> u32 {
    1
}

fn default_limit() -> u32 {
    20
}

impl PaginationParams {
    pub fn validate(&self, max_limit: u32) -> Result<(), String> {
        if self.page == 0 {
            return Err("Page must be greater than 0".to_string());
        }
        if self.limit == 0 {
            return Err("Limit must be greater than 0".to_string());
        }
        if self.limit > max_limit {
            return Err(format!("Limit cannot exceed {}", max_limit));
        }
        Ok(())
    }

    pub fn offset(&self) -> u64 {
        ((self.page - 1) * self.limit) as u64
    }
}

/// Réponse paginée standard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub pagination: crate::api::types::PaginationInfo,
}

impl<T> PaginatedResponse<T> {
    pub fn new(data: Vec<T>, pagination: crate::api::types::PaginationInfo) -> Self {
        Self { data, pagination }
    }
}

/// Métadonnées de réponse
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMetadata {
    pub request_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub duration_ms: u64,
    pub version: String,
}

impl ResponseMetadata {
    pub fn new(request_id: String, duration_ms: u64) -> Self {
        Self {
            request_id,
            timestamp: chrono::Utc::now(),
            duration_ms,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

/// Réponse API standard avec métadonnées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<ResponseMetadata>,
}

impl<T> ApiResponse<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            metadata: None,
        }
    }

    pub fn with_metadata(data: T, metadata: ResponseMetadata) -> Self {
        Self {
            data,
            metadata: Some(metadata),
        }
    }
}

/// Extensions pour les extracteurs Axum
pub mod extractors {
    use axum::{
        extract::{FromRequestParts, Query},
        http::request::Parts,
    };
    use async_trait::async_trait;
    use serde::de::DeserializeOwned;
    
    use crate::api::{ApiError, middleware::AuthInfo, auth::ApiScope};
    use super::PaginationParams;

    /// Extracteur pour la pagination validée
    pub struct ValidatedPagination(pub PaginationParams);

    #[async_trait]
    impl<S> FromRequestParts<S> for ValidatedPagination
    where
        S: Send + Sync,
    {
        type Rejection = ApiError;

        async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
            let Query(params): Query<PaginationParams> = Query::from_request_parts(parts, state).await
                .map_err(|e| ApiError::validation(format!("Invalid pagination parameters: {}", e)))?;

            params.validate(100)
                .map_err(|e| ApiError::validation(e))?;

            Ok(ValidatedPagination(params))
        }
    }

    /// Extracteur pour l'authentification avec scope requis
    pub struct RequireScope<const SCOPE: u8>;

    // Helper pour convertir les scopes en constantes
    impl RequireScope<0> {
        pub const ARCHIVES_READ: u8 = 0;
    }
    impl RequireScope<1> {
        pub const ARCHIVES_WRITE: u8 = 1;
    }
    impl RequireScope<2> {
        pub const ARCHIVES_DELETE: u8 = 2;
    }
    impl RequireScope<3> {
        pub const SEARCH_READ: u8 = 3;
    }
    impl RequireScope<4> {
        pub const NETWORK_READ: u8 = 4;
    }
    impl RequireScope<5> {
        pub const NODE_MANAGE: u8 = 5;
    }
    impl RequireScope<6> {
        pub const ADMIN_ALL: u8 = 6;
    }

    fn scope_from_const(scope_id: u8) -> ApiScope {
        match scope_id {
            0 => ApiScope::ArchivesRead,
            1 => ApiScope::ArchivesWrite,
            2 => ApiScope::ArchivesDelete,
            3 => ApiScope::SearchRead,
            4 => ApiScope::NetworkRead,
            5 => ApiScope::NodeManage,
            6 => ApiScope::AdminAll,
            _ => ApiScope::ArchivesRead, // default
        }
    }

    #[async_trait]
    impl<S, const SCOPE: u8> FromRequestParts<S> for RequireScope<SCOPE>
    where
        S: Send + Sync,
    {
        type Rejection = ApiError;

        async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
            let auth_info = parts.extensions.get::<AuthInfo>()
                .ok_or_else(|| ApiError::authentication("Authentication required"))?;

            let required_scope = scope_from_const(SCOPE);
            
            if !auth_info.scopes.contains(&required_scope) && !auth_info.scopes.contains(&ApiScope::AdminAll) {
                return Err(ApiError::authorization(format!("Required scope: {}", required_scope.as_str())));
            }

            Ok(RequireScope)
        }
    }

    /// Extracteur pour les paramètres de requête validés
    pub struct ValidatedQuery<T>(pub T);

    #[async_trait]
    impl<T, S> FromRequestParts<S> for ValidatedQuery<T>
    where
        T: DeserializeOwned + Validate + Send,
        S: Send + Sync,
    {
        type Rejection = ApiError;

        async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
            let Query(params): Query<T> = Query::from_request_parts(parts, state).await
                .map_err(|e| ApiError::validation(format!("Invalid query parameters: {}", e)))?;

            params.validate()
                .map_err(|e| ApiError::validation(e))?;

            Ok(ValidatedQuery(params))
        }
    }

    /// Trait pour la validation des paramètres
    pub trait Validate {
        fn validate(&self) -> Result<(), String>;
    }

    impl Validate for PaginationParams {
        fn validate(&self) -> Result<(), String> {
            self.validate(100)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rest_config_default() {
        let config = RestConfig::default();
        assert_eq!(config.default_page_size, 20);
        assert_eq!(config.max_page_size, 100);
        assert!(config.enable_openapi);
        assert_eq!(config.archive_timeout, 300);
    }

    #[test]
    fn test_pagination_params() {
        let params = PaginationParams {
            page: 2,
            limit: 50,
        };

        assert!(params.validate(100).is_ok());
        assert_eq!(params.offset(), 50);

        let invalid_params = PaginationParams {
            page: 0,
            limit: 10,
        };
        assert!(invalid_params.validate(100).is_err());

        let invalid_limit = PaginationParams {
            page: 1,
            limit: 200,
        };
        assert!(invalid_limit.validate(100).is_err());
    }

    #[test]
    fn test_api_response() {
        let data = vec![1, 2, 3];
        let response = ApiResponse::new(data.clone());
        assert_eq!(response.data, data);
        assert!(response.metadata.is_none());

        let metadata = ResponseMetadata::new("test-123".to_string(), 100);
        let response_with_metadata = ApiResponse::with_metadata(data.clone(), metadata);
        assert_eq!(response_with_metadata.data, data);
        assert!(response_with_metadata.metadata.is_some());
    }

    #[test]
    fn test_paginated_response() {
        let data = vec![1, 2, 3];
        let pagination = crate::api::types::PaginationInfo::new(1, 10, 100);
        let response = PaginatedResponse::new(data.clone(), pagination);
        
        assert_eq!(response.data, data);
        assert_eq!(response.pagination.page, 1);
        assert_eq!(response.pagination.total, 100);
    }

    #[test]
    fn test_response_metadata() {
        let metadata = ResponseMetadata::new("test-123".to_string(), 150);
        assert_eq!(metadata.request_id, "test-123");
        assert_eq!(metadata.duration_ms, 150);
        assert_eq!(metadata.version, env!("CARGO_PKG_VERSION"));
    }
}