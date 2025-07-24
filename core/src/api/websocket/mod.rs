//! API WebSocket pour ArchiveChain
//!
//! Implémente une API WebSocket complète pour la communication temps réel,
//! les notifications d'événements et le streaming de données.

pub mod handler;
pub mod messages;
pub mod connection;
pub mod events;

use axum::{
    extract::{ws::WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::api::{ApiResult, server::ServerState};
use handler::{WebSocketHandler, ConnectionManager};
use messages::*;

// Re-exports
pub use handler::*;
pub use messages::*;
pub use connection::*;
pub use events::*;

/// Configuration WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    /// Nombre maximum de connexions par utilisateur
    pub max_connections_per_user: usize,
    /// Nombre maximum de connexions totales
    pub max_total_connections: usize,
    /// Timeout pour les messages de ping/pong (en secondes)
    pub ping_timeout: u64,
    /// Intervalle de ping (en secondes)
    pub ping_interval: u64,
    /// Taille maximum des messages (en bytes)
    pub max_message_size: usize,
    /// Buffer size pour les messages sortants
    pub send_buffer_size: usize,
    /// Active la compression des messages
    pub enable_compression: bool,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        Self {
            max_connections_per_user: 10,
            max_total_connections: 10000,
            ping_timeout: 60,
            ping_interval: 30,
            max_message_size: 1024 * 1024, // 1MB
            send_buffer_size: 1000,
            enable_compression: true,
        }
    }
}

/// État partagé WebSocket
#[derive(Clone)]
pub struct WebSocketState {
    pub connection_manager: Arc<RwLock<ConnectionManager>>,
    pub config: WebSocketConfig,
    pub server_state: ServerState,
}

impl WebSocketState {
    pub fn new(config: WebSocketConfig, server_state: ServerState) -> Self {
        Self {
            connection_manager: Arc::new(RwLock::new(ConnectionManager::new(config.clone()))),
            config,
            server_state,
        }
    }
}

/// Crée les routes WebSocket
pub async fn create_routes() -> ApiResult<Router<ServerState>> {
    let router = Router::new()
        // Endpoint principal WebSocket
        .route("/", get(websocket_handler))
        // Endpoint pour les statistiques de connexions
        .route("/stats", get(connection_stats))
        // Endpoint pour les connexions actives (admin seulement)
        .route("/connections", get(active_connections));

    Ok(router)
}

/// Handler principal WebSocket
async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(server_state): State<ServerState>,
) -> Response {
    let config = server_state.config.websocket.clone();
    let ws_state = WebSocketState::new(config, server_state);
    
    ws.on_upgrade(move |socket| async move {
        let handler = WebSocketHandler::new(socket, ws_state);
        handler.handle_connection().await;
    })
}

/// Handler pour les statistiques de connexions
async fn connection_stats(
    State(server_state): State<ServerState>,
) -> axum::response::Json<ConnectionStats> {
    let config = server_state.config.websocket.clone();
    let ws_state = WebSocketState::new(config, server_state);
    
    let manager = ws_state.connection_manager.read().await;
    let stats = manager.get_stats().await;
    
    axum::response::Json(stats)
}

/// Handler pour lister les connexions actives (admin seulement)
async fn active_connections(
    State(server_state): State<ServerState>,
) -> axum::response::Json<Vec<ConnectionInfo>> {
    let config = server_state.config.websocket.clone();
    let ws_state = WebSocketState::new(config, server_state);
    
    let manager = ws_state.connection_manager.read().await;
    let connections = manager.get_all_connections().await;
    
    axum::response::Json(connections)
}

/// Statistiques des connexions WebSocket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionStats {
    pub total_connections: usize,
    pub authenticated_connections: usize,
    pub anonymous_connections: usize,
    pub connections_by_user: HashMap<String, usize>,
    pub subscriptions_by_topic: HashMap<String, usize>,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub uptime_seconds: u64,
}

/// Informations sur une connexion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionInfo {
    pub connection_id: String,
    pub user_id: Option<String>,
    pub connected_at: chrono::DateTime<chrono::Utc>,
    pub last_activity: chrono::DateTime<chrono::Utc>,
    pub subscriptions: Vec<String>,
    pub messages_sent: u64,
    pub messages_received: u64,
    pub remote_addr: Option<String>,
    pub user_agent: Option<String>,
}

/// Erreurs WebSocket spécifiques
#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("Connection limit exceeded")]
    ConnectionLimitExceeded,
    
    #[error("Authentication required")]
    AuthenticationRequired,
    
    #[error("Invalid message format: {0}")]
    InvalidMessageFormat(String),
    
    #[error("Message too large: {0} bytes")]
    MessageTooLarge(usize),
    
    #[error("Connection closed")]
    ConnectionClosed,
    
    #[error("Subscription failed: {0}")]
    SubscriptionFailed(String),
    
    #[error("Topic not found: {0}")]
    TopicNotFound(String),
    
    #[error("Permission denied for topic: {0}")]
    PermissionDenied(String),
    
    #[error("Rate limit exceeded")]
    RateLimitExceeded,
    
    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<WebSocketError> for crate::api::ApiError {
    fn from(err: WebSocketError) -> Self {
        match err {
            WebSocketError::ConnectionLimitExceeded => {
                crate::api::ApiError::RateLimit
            }
            WebSocketError::AuthenticationRequired => {
                crate::api::ApiError::authentication("Authentication required for WebSocket")
            }
            WebSocketError::PermissionDenied(topic) => {
                crate::api::ApiError::authorization(format!("Permission denied for topic: {}", topic))
            }
            WebSocketError::RateLimitExceeded => {
                crate::api::ApiError::RateLimit
            }
            _ => crate::api::ApiError::internal(err.to_string()),
        }
    }
}

/// Résultat WebSocket
pub type WebSocketResult<T> = Result<T, WebSocketError>;

/// Helper pour créer des réponses d'erreur WebSocket
pub fn create_error_message(code: &str, message: &str) -> WsMessage {
    WsMessage::Error {
        code: code.to_string(),
        message: message.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

/// Helper pour créer des réponses de succès
pub fn create_success_message(message: &str) -> WsMessage {
    WsMessage::Success {
        message: message.to_string(),
        timestamp: chrono::Utc::now(),
    }
}

/// Macro pour créer facilement des messages WebSocket
#[macro_export]
macro_rules! ws_message {
    (error, $code:expr, $msg:expr) => {
        crate::api::websocket::create_error_message($code, $msg)
    };
    (success, $msg:expr) => {
        crate::api::websocket::create_success_message($msg)
    };
}

pub use ws_message;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_config_default() {
        let config = WebSocketConfig::default();
        assert_eq!(config.max_connections_per_user, 10);
        assert_eq!(config.max_total_connections, 10000);
        assert_eq!(config.ping_timeout, 60);
        assert_eq!(config.ping_interval, 30);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_connection_stats_creation() {
        let stats = ConnectionStats {
            total_connections: 100,
            authenticated_connections: 80,
            anonymous_connections: 20,
            connections_by_user: HashMap::new(),
            subscriptions_by_topic: HashMap::new(),
            total_messages_sent: 1000,
            total_messages_received: 950,
            uptime_seconds: 3600,
        };
        
        assert_eq!(stats.total_connections, 100);
        assert_eq!(stats.authenticated_connections + stats.anonymous_connections, 100);
    }

    #[test]
    fn test_connection_info_creation() {
        let info = ConnectionInfo {
            connection_id: "conn_123".to_string(),
            user_id: Some("user_456".to_string()),
            connected_at: chrono::Utc::now(),
            last_activity: chrono::Utc::now(),
            subscriptions: vec!["archive_updates".to_string()],
            messages_sent: 10,
            messages_received: 8,
            remote_addr: Some("127.0.0.1:12345".to_string()),
            user_agent: Some("ArchiveChain Client 1.0".to_string()),
        };
        
        assert_eq!(info.connection_id, "conn_123");
        assert_eq!(info.user_id, Some("user_456".to_string()));
        assert_eq!(info.subscriptions.len(), 1);
    }

    #[test]
    fn test_websocket_error_conversion() {
        let error = WebSocketError::AuthenticationRequired;
        let api_error: crate::api::ApiError = error.into();
        
        match api_error {
            crate::api::ApiError::Authentication(_) => (),
            _ => panic!("Expected Authentication error"),
        }
    }

    #[test]
    fn test_error_message_creation() {
        let error_msg = create_error_message("INVALID_FORMAT", "Message format is invalid");
        
        match error_msg {
            WsMessage::Error { code, message, .. } => {
                assert_eq!(code, "INVALID_FORMAT");
                assert_eq!(message, "Message format is invalid");
            }
            _ => panic!("Expected Error message"),
        }
    }

    #[test]
    fn test_success_message_creation() {
        let success_msg = create_success_message("Operation completed successfully");
        
        match success_msg {
            WsMessage::Success { message, .. } => {
                assert_eq!(message, "Operation completed successfully");
            }
            _ => panic!("Expected Success message"),
        }
    }

    #[tokio::test]
    async fn test_routes_creation() {
        let result = create_routes().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_ws_message_macro() {
        let error_msg = ws_message!(error, "TEST_ERROR", "Test error message");
        match error_msg {
            WsMessage::Error { code, .. } => assert_eq!(code, "TEST_ERROR"),
            _ => panic!("Expected Error message"),
        }

        let success_msg = ws_message!(success, "Test success");
        match success_msg {
            WsMessage::Success { message, .. } => assert_eq!(message, "Test success"),
            _ => panic!("Expected Success message"),
        }
    }
}