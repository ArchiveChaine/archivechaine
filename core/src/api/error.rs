//! Gestion des erreurs pour l'API ArchiveChain

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

/// Type de résultat pour les opérations API
pub type ApiResult<T> = Result<T, ApiError>;

/// Erreurs spécifiques à l'API
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    /// Erreurs d'authentification
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Erreurs d'autorisation
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Erreurs de validation
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Ressource non trouvée
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Conflit de ressource
    #[error("Resource conflict: {0}")]
    Conflict(String),

    /// Limite de taux dépassée
    #[error("Rate limit exceeded")]
    RateLimit,

    /// Erreurs de sérialisation
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Erreurs internes du serveur
    #[error("Internal server error: {0}")]
    Internal(String),

    /// Service temporairement indisponible
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    /// Erreurs de blockchain
    #[error("Blockchain error: {0}")]
    Blockchain(#[from] crate::error::CoreError),

    /// Erreurs JWT
    #[error("JWT error: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    /// Erreurs de sérialisation JSON
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Erreurs HTTP
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::Error),

    /// Erreurs WebSocket
    #[error("WebSocket error: {0}")]
    WebSocket(String),

    /// Erreurs gRPC
    #[error("gRPC error: {0}")]
    Grpc(#[from] tonic::Status),

    /// Erreurs P2P
    #[error("P2P error: {0}")]
    P2P(String),
}

impl ApiError {
    /// Retourne le code de statut HTTP approprié
    pub fn status_code(&self) -> StatusCode {
        match self {
            ApiError::Authentication(_) => StatusCode::UNAUTHORIZED,
            ApiError::Authorization(_) => StatusCode::FORBIDDEN,
            ApiError::Validation(_) => StatusCode::BAD_REQUEST,
            ApiError::NotFound(_) => StatusCode::NOT_FOUND,
            ApiError::Conflict(_) => StatusCode::CONFLICT,
            ApiError::RateLimit => StatusCode::TOO_MANY_REQUESTS,
            ApiError::Serialization(_) => StatusCode::UNPROCESSABLE_ENTITY,
            ApiError::ServiceUnavailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            ApiError::Internal(_) 
            | ApiError::Blockchain(_) 
            | ApiError::Jwt(_) 
            | ApiError::Json(_) 
            | ApiError::Http(_) 
            | ApiError::WebSocket(_) 
            | ApiError::Grpc(_) 
            | ApiError::P2P(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Retourne le code d'erreur pour l'API
    pub fn error_code(&self) -> &'static str {
        match self {
            ApiError::Authentication(_) => "AUTHENTICATION_FAILED",
            ApiError::Authorization(_) => "AUTHORIZATION_FAILED",
            ApiError::Validation(_) => "VALIDATION_FAILED",
            ApiError::NotFound(_) => "RESOURCE_NOT_FOUND",
            ApiError::Conflict(_) => "RESOURCE_CONFLICT",
            ApiError::RateLimit => "RATE_LIMIT_EXCEEDED",
            ApiError::Serialization(_) => "SERIALIZATION_ERROR",
            ApiError::Internal(_) => "INTERNAL_SERVER_ERROR",
            ApiError::ServiceUnavailable(_) => "SERVICE_UNAVAILABLE",
            ApiError::Blockchain(_) => "BLOCKCHAIN_ERROR",
            ApiError::Jwt(_) => "JWT_ERROR",
            ApiError::Json(_) => "JSON_ERROR",
            ApiError::Http(_) => "HTTP_ERROR",
            ApiError::WebSocket(_) => "WEBSOCKET_ERROR",
            ApiError::Grpc(_) => "GRPC_ERROR",
            ApiError::P2P(_) => "P2P_ERROR",
        }
    }

    /// Indique si l'erreur doit être loggée comme erreur interne
    pub fn is_internal_error(&self) -> bool {
        matches!(
            self,
            ApiError::Internal(_) 
            | ApiError::Blockchain(_) 
            | ApiError::Http(_) 
            | ApiError::WebSocket(_) 
            | ApiError::Grpc(_) 
            | ApiError::P2P(_)
        )
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let error_code = self.error_code();
        let message = self.to_string();

        // Log les erreurs internes
        if self.is_internal_error() {
            tracing::error!("Internal API error: {} - {}", error_code, message);
        } else {
            tracing::warn!("API error: {} - {}", error_code, message);
        }

        let body = json!({
            "error": {
                "code": error_code,
                "message": message,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            }
        });

        (status, Json(body)).into_response()
    }
}

/// Erreur de validation avec détails
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

/// Réponse d'erreur détaillée pour la validation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ValidationErrorResponse {
    pub code: String,
    pub message: String,
    pub errors: Vec<ValidationError>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl ValidationErrorResponse {
    pub fn new(errors: Vec<ValidationError>) -> Self {
        Self {
            code: "VALIDATION_FAILED".to_string(),
            message: "Request validation failed".to_string(),
            errors,
            timestamp: chrono::Utc::now(),
        }
    }
}

impl IntoResponse for ValidationErrorResponse {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

/// Helper pour créer des erreurs communes
impl ApiError {
    pub fn authentication<S: Into<String>>(msg: S) -> Self {
        Self::Authentication(msg.into())
    }

    pub fn authorization<S: Into<String>>(msg: S) -> Self {
        Self::Authorization(msg.into())
    }

    pub fn validation<S: Into<String>>(msg: S) -> Self {
        Self::Validation(msg.into())
    }

    pub fn not_found<S: Into<String>>(resource: S) -> Self {
        Self::NotFound(resource.into())
    }

    pub fn conflict<S: Into<String>>(msg: S) -> Self {
        Self::Conflict(msg.into())
    }

    pub fn internal<S: Into<String>>(msg: S) -> Self {
        Self::Internal(msg.into())
    }

    pub fn service_unavailable<S: Into<String>>(msg: S) -> Self {
        Self::ServiceUnavailable(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_error_status_codes() {
        assert_eq!(ApiError::authentication("test").status_code(), StatusCode::UNAUTHORIZED);
        assert_eq!(ApiError::authorization("test").status_code(), StatusCode::FORBIDDEN);
        assert_eq!(ApiError::validation("test").status_code(), StatusCode::BAD_REQUEST);
        assert_eq!(ApiError::not_found("test").status_code(), StatusCode::NOT_FOUND);
        assert_eq!(ApiError::conflict("test").status_code(), StatusCode::CONFLICT);
        assert_eq!(ApiError::RateLimit.status_code(), StatusCode::TOO_MANY_REQUESTS);
    }

    #[test]
    fn test_error_codes() {
        assert_eq!(ApiError::authentication("test").error_code(), "AUTHENTICATION_FAILED");
        assert_eq!(ApiError::validation("test").error_code(), "VALIDATION_FAILED");
        assert_eq!(ApiError::not_found("test").error_code(), "RESOURCE_NOT_FOUND");
    }

    #[test]
    fn test_internal_error_detection() {
        assert!(ApiError::internal("test").is_internal_error());
        assert!(ApiError::Blockchain(crate::error::CoreError::InvalidHash).is_internal_error());
        assert!(!ApiError::validation("test").is_internal_error());
        assert!(!ApiError::not_found("test").is_internal_error());
    }

    #[test]
    fn test_validation_error() {
        let error = ValidationError::new("email", "Invalid email format");
        assert_eq!(error.field, "email");
        assert_eq!(error.message, "Invalid email format");
    }

    #[test]
    fn test_validation_error_response() {
        let errors = vec![
            ValidationError::new("email", "Invalid email format"),
            ValidationError::new("password", "Password too short"),
        ];
        let response = ValidationErrorResponse::new(errors);
        assert_eq!(response.code, "VALIDATION_FAILED");
        assert_eq!(response.errors.len(), 2);
    }
}