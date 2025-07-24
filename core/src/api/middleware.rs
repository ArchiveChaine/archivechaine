//! Middlewares de sécurité pour l'API ArchiveChain
//!
//! Ce module contient tous les middlewares nécessaires pour sécuriser l'API :
//! - Authentification JWT
//! - Rate limiting
//! - CORS
//! - Compression
//! - Request ID
//! - Logging et monitoring

use crate::api::{ApiError, ApiResult, auth::{AuthService, JwtClaims, ApiScope}};
use axum::{
    extract::{Request, State},
    http::{HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::Duration,
};
use tower_http::{
    cors::{CorsLayer, Any},
    compression::CompressionLayer,
    trace::TraceLayer,
};
use tracing::{info, warn, error};

/// Configuration des middlewares
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiddlewareConfig {
    /// Configuration CORS
    pub cors: CorsConfig,
    /// Configuration rate limiting
    pub rate_limit: RateLimitConfig,
    /// Configuration de compression
    pub compression: CompressionConfig,
    /// Configuration de logging
    pub logging: LoggingConfig,
}

impl Default for MiddlewareConfig {
    fn default() -> Self {
        Self {
            cors: CorsConfig::default(),
            rate_limit: RateLimitConfig::default(),
            compression: CompressionConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

/// Configuration CORS
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age: Option<u64>,
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "OPTIONS".to_string(),
            ],
            allowed_headers: vec![
                "content-type".to_string(),
                "authorization".to_string(),
                "x-request-id".to_string(),
            ],
            expose_headers: vec![
                "x-request-id".to_string(),
                "x-rate-limit-remaining".to_string(),
                "x-rate-limit-reset".to_string(),
            ],
            max_age: Some(86400), // 24 heures
            allow_credentials: true,
        }
    }
}

/// Configuration du rate limiting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Limite globale par IP (requêtes par minute)
    pub global_per_ip: u32,
    /// Limite pour utilisateurs authentifiés (requêtes par minute)
    pub authenticated_per_user: u32,
    /// Limite pour utilisateurs premium (requêtes par minute)
    pub premium_per_user: u32,
    /// Fenêtre de temps pour le rate limiting (secondes)
    pub window_seconds: u64,
    /// Burst autorisé
    pub burst_size: u32,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            global_per_ip: 60,
            authenticated_per_user: 300,
            premium_per_user: 1000,
            window_seconds: 60,
            burst_size: 10,
        }
    }
}

/// Configuration de compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    pub enabled: bool,
    pub algorithms: Vec<String>,
    pub min_size: usize,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            algorithms: vec!["gzip".to_string(), "br".to_string(), "deflate".to_string()],
            min_size: 1024, // 1KB
        }
    }
}

/// Configuration de logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub log_requests: bool,
    pub log_responses: bool,
    pub log_errors: bool,
    pub include_headers: bool,
    pub include_body: bool,
    pub max_body_size: usize,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            log_requests: true,
            log_responses: false,
            log_errors: true,
            include_headers: false,
            include_body: false,
            max_body_size: 4096,
        }
    }
}

/// État partagé pour les middlewares
#[derive(Clone)]
pub struct MiddlewareState {
    pub auth_service: Arc<AuthService>,
    pub rate_limiters: Arc<RateLimiters>,
    pub config: MiddlewareConfig,
}

/// Gestionnaire de rate limiters
pub struct RateLimiters {
    pub ip_limiter: RateLimiter<IpAddr, governor::state::InMemoryState, governor::clock::DefaultClock>,
    pub user_limiters: Arc<tokio::sync::RwLock<HashMap<String, RateLimiter<String, governor::state::InMemoryState, governor::clock::DefaultClock>>>>,
}

impl RateLimiters {
    pub fn new(config: &RateLimitConfig) -> Self {
        let quota = Quota::per_minute(config.global_per_ip);
        let ip_limiter = RateLimiter::direct(quota);

        Self {
            ip_limiter,
            user_limiters: Arc::new(tokio::sync::RwLock::new(HashMap::new())),
        }
    }
}

/// Extension pour les informations d'authentification
#[derive(Debug, Clone)]
pub struct AuthInfo {
    pub claims: JwtClaims,
    pub user_id: String,
    pub scopes: Vec<ApiScope>,
}

/// Middleware d'authentification JWT
pub async fn auth_middleware(
    State(state): State<MiddlewareState>,
    mut req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Vérifie le header Authorization
    let auth_header = req.headers()
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or_else(|| ApiError::authentication("Missing authorization header"))?;

    // Valide le token JWT
    let claims = state.auth_service.extract_token_from_header(auth_header)?;

    // Vérifie que l'utilisateur est actif
    if !claims.user_metadata.get("is_active").unwrap_or(&serde_json::Value::Bool(true)).as_bool().unwrap_or(true) {
        return Err(ApiError::authentication("User account is deactivated"));
    }

    // Convertit les scopes
    let scopes: Vec<ApiScope> = claims.scope
        .iter()
        .filter_map(|s| ApiScope::from_str(s))
        .collect();

    // Ajoute les informations d'authentification à la requête
    let auth_info = AuthInfo {
        claims: claims.clone(),
        user_id: claims.sub.clone(),
        scopes,
    };

    req.extensions_mut().insert(auth_info);

    Ok(next.run(req).await)
}

/// Middleware de rate limiting
pub async fn rate_limit_middleware(
    State(state): State<MiddlewareState>,
    req: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Obtient l'IP du client
    let client_ip = req.headers()
        .get("x-forwarded-for")
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.split(',').next())
        .and_then(|s| s.trim().parse::<IpAddr>().ok())
        .unwrap_or_else(|| "127.0.0.1".parse().unwrap());

    // Vérifie la limite globale par IP
    if let Err(_) = state.rate_limiters.ip_limiter.check_key(&client_ip) {
        warn!("Rate limit exceeded for IP: {}", client_ip);
        return Err(ApiError::RateLimit);
    }

    // Si authentifié, vérifie la limite par utilisateur
    if let Some(auth_info) = req.extensions().get::<AuthInfo>() {
        let user_id = &auth_info.user_id;
        let user_limit = if auth_info.claims.rate_limit.requests_per_hour > 10000 {
            state.config.rate_limit.premium_per_user
        } else {
            state.config.rate_limit.authenticated_per_user
        };

        let mut user_limiters = state.rate_limiters.user_limiters.write().await;
        let user_limiter = user_limiters.entry(user_id.clone()).or_insert_with(|| {
            let quota = Quota::per_minute(user_limit);
            RateLimiter::direct(quota)
        });

        if let Err(_) = user_limiter.check_key(user_id) {
            warn!("Rate limit exceeded for user: {}", user_id);
            return Err(ApiError::RateLimit);
        }
    }

    Ok(next.run(req).await)
}

/// Middleware de validation des permissions
pub fn require_scope(required_scope: ApiScope) -> impl Fn(Request, Next) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response, ApiError>> + Send>> + Clone {
    move |req: Request, next: Next| {
        let scope = required_scope.clone();
        Box::pin(async move {
            let auth_info = req.extensions()
                .get::<AuthInfo>()
                .ok_or_else(|| ApiError::authentication("Authentication required"))?;

            // Vérifie les permissions
            if !auth_info.scopes.contains(&scope) && !auth_info.scopes.contains(&ApiScope::AdminAll) {
                return Err(ApiError::authorization(format!("Required scope: {}", scope.as_str())));
            }

            Ok(next.run(req).await)
        })
    }
}

/// Middleware pour ajouter un Request ID
pub async fn request_id_middleware(
    mut req: Request,
    next: Next,
) -> Response {
    let request_id = req.headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or_else(|| {
            let id = uuid::Uuid::new_v4().to_string();
            req.headers_mut().insert(
                "x-request-id",
                HeaderValue::from_str(&id).unwrap(),
            );
            req.headers().get("x-request-id").unwrap().to_str().unwrap()
        });

    let mut response = next.run(req).await;
    
    // Ajoute le Request ID à la réponse
    response.headers_mut().insert(
        "x-request-id",
        HeaderValue::from_str(request_id).unwrap(),
    );

    response
}

/// Middleware de logging des requêtes
pub async fn logging_middleware(
    req: Request,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let query = req.uri().query().unwrap_or("").to_string();
    let user_agent = req.headers()
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");
    let request_id = req.headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    let start_time = std::time::Instant::now();

    info!(
        request_id = request_id,
        method = %method,
        path = path,
        query = query,
        user_agent = user_agent,
        "Request started"
    );

    let response = next.run(req).await;
    let duration = start_time.elapsed();

    let status = response.status();
    let level = if status.is_server_error() {
        tracing::Level::ERROR
    } else if status.is_client_error() {
        tracing::Level::WARN
    } else {
        tracing::Level::INFO
    };

    tracing::event!(
        level,
        request_id = request_id,
        method = %method,
        path = path,
        status = %status,
        duration_ms = duration.as_millis(),
        "Request completed"
    );

    response
}

/// Middleware de gestion d'erreurs
pub async fn error_handler_middleware(
    req: Request,
    next: Next,
) -> Response {
    let request_id = req.headers()
        .get("x-request-id")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("unknown");

    match next.run(req).await.into_response() {
        response if response.status().is_success() => response,
        response => {
            let status = response.status();
            if status.is_server_error() {
                error!(
                    request_id = request_id,
                    status = %status,
                    "Internal server error occurred"
                );
            }
            response
        }
    }
}

/// Builder pour les middlewares CORS
pub fn cors_middleware(config: &CorsConfig) -> CorsLayer {
    let mut cors = CorsLayer::new();

    // Origins
    if config.allowed_origins.contains(&"*".to_string()) {
        cors = cors.allow_origin(Any);
    } else {
        for origin in &config.allowed_origins {
            if let Ok(origin) = origin.parse::<HeaderValue>() {
                cors = cors.allow_origin(origin);
            }
        }
    }

    // Methods
    let methods: Vec<Method> = config.allowed_methods
        .iter()
        .filter_map(|m| m.parse().ok())
        .collect();
    cors = cors.allow_methods(methods);

    // Headers
    let headers: Vec<HeaderValue> = config.allowed_headers
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    cors = cors.allow_headers(headers);

    // Expose headers
    let expose_headers: Vec<HeaderValue> = config.expose_headers
        .iter()
        .filter_map(|h| h.parse().ok())
        .collect();
    cors = cors.expose_headers(expose_headers);

    // Max age
    if let Some(max_age) = config.max_age {
        cors = cors.max_age(Duration::from_secs(max_age));
    }

    // Credentials
    if config.allow_credentials {
        cors = cors.allow_credentials(true);
    }

    cors
}

/// Builder pour le middleware de compression
pub fn compression_middleware(config: &CompressionConfig) -> Option<CompressionLayer> {
    if config.enabled {
        Some(CompressionLayer::new())
    } else {
        None
    }
}

/// Builder pour le middleware de tracing
pub fn tracing_middleware(config: &LoggingConfig) -> Option<TraceLayer> {
    if config.enabled {
        Some(TraceLayer::new_for_http())
    } else {
        None
    }
}

/// Macro pour créer un middleware de permission
#[macro_export]
macro_rules! require_scope {
    ($scope:expr) => {
        axum::middleware::from_fn(crate::api::middleware::require_scope($scope))
    };
}

// Re-export removed to avoid duplicate definition

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::auth::{AuthConfig, AuthService};

    #[test]
    fn test_middleware_config_default() {
        let config = MiddlewareConfig::default();
        assert!(config.cors.allow_credentials);
        assert_eq!(config.rate_limit.global_per_ip, 60);
        assert!(config.compression.enabled);
        assert!(config.logging.enabled);
    }

    #[test]
    fn test_cors_config() {
        let config = CorsConfig::default();
        assert!(config.allowed_origins.contains(&"*".to_string()));
        assert!(config.allowed_methods.contains(&"GET".to_string()));
        assert!(config.allowed_headers.contains(&"authorization".to_string()));
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.global_per_ip, 60);
        assert_eq!(config.authenticated_per_user, 300);
        assert_eq!(config.premium_per_user, 1000);
        assert_eq!(config.window_seconds, 60);
    }

    #[tokio::test]
    async fn test_rate_limiters_creation() {
        let config = RateLimitConfig::default();
        let rate_limiters = RateLimiters::new(&config);
        
        let test_ip: IpAddr = "127.0.0.1".parse().unwrap();
        
        // Premier appel devrait passer
        assert!(rate_limiters.ip_limiter.check_key(&test_ip).is_ok());
    }

    #[test]
    fn test_auth_info() {
        use crate::api::auth::{JwtClaims, RateLimit};
        use std::collections::HashMap;
        
        let claims = JwtClaims {
            sub: "test_user".to_string(),
            iss: "test".to_string(),
            aud: "test".to_string(),
            exp: 0,
            iat: 0,
            nbf: 0,
            jti: "test".to_string(),
            scope: vec!["archives:read".to_string()],
            node_id: None,
            rate_limit: RateLimit::default(),
            user_metadata: HashMap::new(),
        };

        let auth_info = AuthInfo {
            claims: claims.clone(),
            user_id: claims.sub.clone(),
            scopes: vec![ApiScope::ArchivesRead],
        };

        assert_eq!(auth_info.user_id, "test_user");
        assert!(auth_info.scopes.contains(&ApiScope::ArchivesRead));
    }

    #[test]
    fn test_compression_config() {
        let config = CompressionConfig::default();
        assert!(config.enabled);
        assert!(config.algorithms.contains(&"gzip".to_string()));
        assert_eq!(config.min_size, 1024);
    }

    #[test]
    fn test_logging_config() {
        let config = LoggingConfig::default();
        assert!(config.enabled);
        assert!(config.log_requests);
        assert!(!config.log_responses);
        assert!(config.log_errors);
        assert_eq!(config.max_body_size, 4096);
    }
}