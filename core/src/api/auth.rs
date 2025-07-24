//! Système d'authentification JWT pour ArchiveChain API
//!
//! Gère l'authentification basée sur des tokens JWT avec des scopes et permissions,
//! le rate limiting par utilisateur, et la validation des tokens.

use crate::api::{ApiError, ApiResult};
use crate::{PublicKey, Hash};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Configuration de l'authentification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Clé secrète pour signer les JWT
    pub jwt_secret: String,
    /// Durée de validité des tokens (en secondes)
    pub token_expiry: u64,
    /// Durée de validité des refresh tokens (en secondes)
    pub refresh_token_expiry: u64,
    /// Algorithme de signature
    pub algorithm: String,
    /// Issuer des tokens
    pub issuer: String,
    /// Audience des tokens
    pub audience: String,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "default-secret-change-in-production".to_string(),
            token_expiry: 3600, // 1 heure
            refresh_token_expiry: 86400 * 7, // 7 jours
            algorithm: "EdDSA".to_string(),
            issuer: "archivechain.org".to_string(),
            audience: "api.archivechain.org".to_string(),
        }
    }
}

/// Claims JWT pour ArchiveChain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    /// Subject (user ID)
    pub sub: String,
    /// Issuer
    pub iss: String,
    /// Audience
    pub aud: String,
    /// Expiration time
    pub exp: u64,
    /// Issued at
    pub iat: u64,
    /// Not before
    pub nbf: u64,
    /// JWT ID
    pub jti: String,
    /// Scopes autorisés
    pub scope: Vec<String>,
    /// Node ID associé (optionnel)
    pub node_id: Option<String>,
    /// Limites de taux
    pub rate_limit: RateLimit,
    /// Métadonnées utilisateur
    pub user_metadata: HashMap<String, serde_json::Value>,
}

/// Limites de taux par utilisateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimit {
    pub requests_per_hour: u32,
    pub storage_limit_gb: u32,
    pub concurrent_requests: u32,
}

impl Default for RateLimit {
    fn default() -> Self {
        Self {
            requests_per_hour: 1000,
            storage_limit_gb: 100,
            concurrent_requests: 10,
        }
    }
}

/// Scopes d'autorisation disponibles
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApiScope {
    ArchivesRead,
    ArchivesWrite,
    ArchivesDelete,
    SearchRead,
    NetworkRead,
    NodeManage,
    AdminAll,
}

impl ApiScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ArchivesRead => "archives:read",
            Self::ArchivesWrite => "archives:write",
            Self::ArchivesDelete => "archives:delete",
            Self::SearchRead => "search:read",
            Self::NetworkRead => "network:read",
            Self::NodeManage => "node:manage",
            Self::AdminAll => "admin:all",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "archives:read" => Some(Self::ArchivesRead),
            "archives:write" => Some(Self::ArchivesWrite),
            "archives:delete" => Some(Self::ArchivesDelete),
            "search:read" => Some(Self::SearchRead),
            "network:read" => Some(Self::NetworkRead),
            "node:manage" => Some(Self::NodeManage),
            "admin:all" => Some(Self::AdminAll),
            _ => None,
        }
    }

    pub fn all_scopes() -> Vec<Self> {
        vec![
            Self::ArchivesRead,
            Self::ArchivesWrite,
            Self::ArchivesDelete,
            Self::SearchRead,
            Self::NetworkRead,
            Self::NodeManage,
            Self::AdminAll,
        ]
    }
}

/// Informations sur un token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenInfo {
    pub token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    pub token_type: String,
    pub scope: Vec<String>,
}

/// Erreurs d'authentification
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    
    #[error("Token expired")]
    TokenExpired,
    
    #[error("Insufficient permissions: required {required}, got {actual:?}")]
    InsufficientPermissions { required: String, actual: Vec<String> },
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("User not found: {0}")]
    UserNotFound(String),
    
    #[error("Invalid credentials")]
    InvalidCredentials,
    
    #[error("Token generation failed: {0}")]
    TokenGenerationFailed(String),
}

impl From<AuthError> for ApiError {
    fn from(err: AuthError) -> Self {
        match err {
            AuthError::InvalidToken(msg) | AuthError::TokenExpired => {
                ApiError::Authentication(err.to_string())
            }
            AuthError::InsufficientPermissions { .. } => {
                ApiError::Authorization(err.to_string())
            }
            AuthError::RateLimitExceeded => ApiError::RateLimit,
            AuthError::UserNotFound(_) | AuthError::InvalidCredentials => {
                ApiError::Authentication(err.to_string())
            }
            AuthError::TokenGenerationFailed(msg) => ApiError::Internal(msg),
        }
    }
}

/// Service d'authentification
#[derive(Debug, Clone)]
pub struct AuthService {
    config: AuthConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    validation: Validation,
}

impl AuthService {
    /// Crée un nouveau service d'authentification
    pub fn new(config: AuthConfig) -> ApiResult<Self> {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
        
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_issuer(&[&config.issuer]);
        validation.set_audience(&[&config.audience]);
        
        Ok(Self {
            config,
            encoding_key,
            decoding_key,
            validation,
        })
    }

    /// Génère un token JWT pour un utilisateur
    pub fn generate_token(
        &self,
        user_id: &str,
        scopes: Vec<ApiScope>,
        node_id: Option<String>,
        rate_limit: Option<RateLimit>,
    ) -> ApiResult<TokenInfo> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = JwtClaims {
            sub: user_id.to_string(),
            iss: self.config.issuer.clone(),
            aud: self.config.audience.clone(),
            exp: now + self.config.token_expiry,
            iat: now,
            nbf: now,
            jti: uuid::Uuid::new_v4().to_string(),
            scope: scopes.iter().map(|s| s.as_str().to_string()).collect(),
            node_id,
            rate_limit: rate_limit.unwrap_or_default(),
            user_metadata: HashMap::new(),
        };

        let header = Header::new(Algorithm::HS256);
        let token = encode(&header, &claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGenerationFailed(e.to_string()))?;

        // Génère aussi un refresh token
        let refresh_claims = JwtClaims {
            exp: now + self.config.refresh_token_expiry,
            ..claims
        };

        let refresh_token = encode(&header, &refresh_claims, &self.encoding_key)
            .map_err(|e| AuthError::TokenGenerationFailed(e.to_string()))?;

        Ok(TokenInfo {
            token,
            refresh_token,
            expires_in: self.config.token_expiry,
            token_type: "Bearer".to_string(),
            scope: scopes.iter().map(|s| s.as_str().to_string()).collect(),
        })
    }

    /// Valide un token JWT
    pub fn validate_token(&self, token: &str) -> ApiResult<JwtClaims> {
        let token_data = decode::<JwtClaims>(token, &self.decoding_key, &self.validation)
            .map_err(|e| {
                match e.kind() {
                    jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                    _ => AuthError::InvalidToken(e.to_string()),
                }
            })?;

        Ok(token_data.claims)
    }

    /// Vérifie qu'un utilisateur a les permissions requises
    pub fn check_permission(&self, claims: &JwtClaims, required_scope: &ApiScope) -> ApiResult<()> {
        // Admin a tous les droits
        if claims.scope.contains(&ApiScope::AdminAll.as_str().to_string()) {
            return Ok(());
        }

        // Vérifie le scope spécifique
        if claims.scope.contains(&required_scope.as_str().to_string()) {
            return Ok(());
        }

        Err(AuthError::InsufficientPermissions {
            required: required_scope.as_str().to_string(),
            actual: claims.scope.clone(),
        }.into())
    }

    /// Extrait et valide un token depuis un header Authorization
    pub fn extract_token_from_header(&self, auth_header: &str) -> ApiResult<JwtClaims> {
        if !auth_header.starts_with("Bearer ") {
            return Err(AuthError::InvalidToken("Invalid authorization header format".to_string()).into());
        }

        let token = &auth_header[7..]; // Retire "Bearer "
        self.validate_token(token)
    }

    /// Rafraîchit un token
    pub fn refresh_token(&self, refresh_token: &str) -> ApiResult<TokenInfo> {
        let claims = self.validate_token(refresh_token)?;
        
        // Génère un nouveau token avec les mêmes permissions
        let scopes: Vec<ApiScope> = claims.scope
            .iter()
            .filter_map(|s| ApiScope::from_str(s))
            .collect();

        self.generate_token(
            &claims.sub,
            scopes,
            claims.node_id,
            Some(claims.rate_limit),
        )
    }
}

/// Gestionnaire d'utilisateurs et permissions
#[derive(Debug)]
pub struct UserManager {
    users: HashMap<String, UserAccount>,
    api_keys: HashMap<String, String>, // api_key -> user_id
}

/// Compte utilisateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserAccount {
    pub user_id: String,
    pub public_key: Option<PublicKey>,
    pub scopes: HashSet<ApiScope>,
    pub rate_limit: RateLimit,
    pub node_id: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            users: HashMap::new(),
            api_keys: HashMap::new(),
        }
    }

    /// Crée un nouvel utilisateur
    pub fn create_user(
        &mut self,
        user_id: String,
        public_key: Option<PublicKey>,
        scopes: HashSet<ApiScope>,
        rate_limit: Option<RateLimit>,
    ) -> ApiResult<String> {
        if self.users.contains_key(&user_id) {
            return Err(ApiError::conflict("User already exists"));
        }

        let account = UserAccount {
            user_id: user_id.clone(),
            public_key,
            scopes,
            rate_limit: rate_limit.unwrap_or_default(),
            node_id: None,
            created_at: chrono::Utc::now(),
            last_login: None,
            is_active: true,
            metadata: HashMap::new(),
        };

        self.users.insert(user_id.clone(), account);

        // Génère une API key
        let api_key = format!("arc_{}", uuid::Uuid::new_v4().simple());
        self.api_keys.insert(api_key.clone(), user_id);

        Ok(api_key)
    }

    /// Récupère un utilisateur par ID
    pub fn get_user(&self, user_id: &str) -> Option<&UserAccount> {
        self.users.get(user_id)
    }

    /// Récupère un utilisateur par API key
    pub fn get_user_by_api_key(&self, api_key: &str) -> Option<&UserAccount> {
        self.api_keys.get(api_key)
            .and_then(|user_id| self.users.get(user_id))
    }

    /// Met à jour la dernière connexion
    pub fn update_last_login(&mut self, user_id: &str) {
        if let Some(user) = self.users.get_mut(user_id) {
            user.last_login = Some(chrono::Utc::now());
        }
    }

    /// Désactive un utilisateur
    pub fn deactivate_user(&mut self, user_id: &str) -> ApiResult<()> {
        match self.users.get_mut(user_id) {
            Some(user) => {
                user.is_active = false;
                Ok(())
            }
            None => Err(AuthError::UserNotFound(user_id.to_string()).into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_config_default() {
        let config = AuthConfig::default();
        assert_eq!(config.token_expiry, 3600);
        assert_eq!(config.refresh_token_expiry, 86400 * 7);
        assert_eq!(config.issuer, "archivechain.org");
    }

    #[test]
    fn test_api_scope_conversion() {
        let scope = ApiScope::ArchivesRead;
        assert_eq!(scope.as_str(), "archives:read");
        assert_eq!(ApiScope::from_str("archives:read"), Some(ApiScope::ArchivesRead));
        assert_eq!(ApiScope::from_str("invalid"), None);
    }

    #[test]
    fn test_rate_limit_default() {
        let rate_limit = RateLimit::default();
        assert_eq!(rate_limit.requests_per_hour, 1000);
        assert_eq!(rate_limit.storage_limit_gb, 100);
        assert_eq!(rate_limit.concurrent_requests, 10);
    }

    #[test]
    fn test_user_manager() {
        let mut manager = UserManager::new();
        
        let scopes = vec![ApiScope::ArchivesRead, ApiScope::SearchRead].into_iter().collect();
        let api_key = manager.create_user(
            "test_user".to_string(),
            None,
            scopes,
            None,
        ).unwrap();

        assert!(api_key.starts_with("arc_"));
        
        let user = manager.get_user("test_user").unwrap();
        assert_eq!(user.user_id, "test_user");
        assert!(user.is_active);
        assert!(user.scopes.contains(&ApiScope::ArchivesRead));

        let user_by_key = manager.get_user_by_api_key(&api_key).unwrap();
        assert_eq!(user_by_key.user_id, "test_user");
    }

    #[tokio::test]
    async fn test_jwt_generation_and_validation() {
        let config = AuthConfig::default();
        let auth_service = AuthService::new(config).unwrap();

        let scopes = vec![ApiScope::ArchivesRead, ApiScope::SearchRead];
        let token_info = auth_service.generate_token(
            "test_user",
            scopes.clone(),
            None,
            None,
        ).unwrap();

        assert!(!token_info.token.is_empty());
        assert!(!token_info.refresh_token.is_empty());
        assert_eq!(token_info.token_type, "Bearer");

        // Valide le token
        let claims = auth_service.validate_token(&token_info.token).unwrap();
        assert_eq!(claims.sub, "test_user");
        assert_eq!(claims.scope.len(), 2);

        // Teste les permissions
        assert!(auth_service.check_permission(&claims, &ApiScope::ArchivesRead).is_ok());
        assert!(auth_service.check_permission(&claims, &ApiScope::ArchivesWrite).is_err());
    }

    #[test]
    fn test_auth_header_extraction() {
        let config = AuthConfig::default();
        let auth_service = AuthService::new(config).unwrap();

        // Test avec header invalide
        let result = auth_service.extract_token_from_header("Invalid header");
        assert!(result.is_err());

        // Test avec format Bearer invalide
        let result = auth_service.extract_token_from_header("Basic token123");
        assert!(result.is_err());
    }
}