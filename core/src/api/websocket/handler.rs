//! Handler principal WebSocket pour ArchiveChain
//!
//! Gère le cycle de vie complet des connexions WebSocket incluant
//! l'authentification, les souscriptions et la communication bidirectionnelle.

use axum::extract::ws::{WebSocket, Message};
use futures_util::{SinkExt, StreamExt, stream::{SplitSink, SplitStream}};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio::time::{Duration, Instant, interval};

use crate::api::{
    auth::AuthService,
    middleware::AuthInfo,
};
use super::{
    connection::ConnectionManager,
    messages::*,
    WebSocketState, WebSocketError, WebSocketResult,
};

/// Handler principal pour les connexions WebSocket
pub struct WebSocketHandler {
    /// Socket WebSocket
    socket: WebSocket,
    /// État partagé WebSocket
    state: WebSocketState,
    /// ID unique de cette connexion
    connection_id: String,
    /// Canal pour recevoir les messages à envoyer
    message_receiver: mpsc::UnboundedReceiver<WsMessage>,
    /// Sender pour envoyer des messages à cette connexion
    message_sender: mpsc::UnboundedSender<WsMessage>,
}

impl WebSocketHandler {
    /// Crée un nouveau handler WebSocket
    pub fn new(socket: WebSocket, state: WebSocketState) -> Self {
        let connection_id = uuid::Uuid::new_v4().to_string();
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        Self {
            socket,
            state,
            connection_id,
            message_receiver,
            message_sender,
        }
    }

    /// Gère la connexion WebSocket
    pub async fn handle_connection(mut self) {
        tracing::info!("New WebSocket connection: {}", self.connection_id);

        // Ajoute la connexion au gestionnaire
        {
            let mut manager = self.state.connection_manager.write().await;
            if let Err(e) = manager.add_connection(
                self.connection_id.clone(),
                self.message_sender.clone(),
                None, // TODO: Récupérer l'adresse IP réelle
                None, // TODO: Récupérer le User-Agent
            ).await {
                tracing::error!("Failed to add connection: {}", e);
                return;
            }
        }

        // Divise le socket en sink et stream
        let (mut socket_sender, mut socket_receiver) = self.socket.split();

        // Tâche pour envoyer des messages
        let connection_id_send = self.connection_id.clone();
        let state_send = self.state.clone();
        let send_task = tokio::spawn(async move {
            let mut message_receiver = self.message_receiver;
            
            while let Some(message) = message_receiver.recv().await {
                let serialized = match serde_json::to_string(&message) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::error!("Failed to serialize message: {}", e);
                        continue;
                    }
                };

                if let Err(e) = socket_sender.send(Message::Text(serialized)).await {
                    tracing::error!("Failed to send WebSocket message: {}", e);
                    break;
                }
            }
        });

        // Tâche pour recevoir des messages
        let connection_id_recv = self.connection_id.clone();
        let state_recv = self.state.clone();
        let message_sender_recv = self.message_sender.clone();
        let recv_task = tokio::spawn(async move {
            while let Some(message) = socket_receiver.next().await {
                match message {
                    Ok(Message::Text(text)) => {
                        let message_len = text.len() as u64;
                        
                        // Met à jour l'activité
                        {
                            let mut manager = state_recv.connection_manager.write().await;
                            manager.update_activity(&connection_id_recv, message_len).await;
                        }

                        // Traite le message
                        if let Err(e) = Self::handle_text_message(
                            text,
                            &connection_id_recv,
                            &state_recv,
                            &message_sender_recv,
                        ).await {
                            tracing::error!("Error handling message: {}", e);
                            let error_msg = MessageBuilder::error(
                                "MESSAGE_ERROR".to_string(),
                                e.to_string(),
                            );
                            let _ = message_sender_recv.send(error_msg);
                        }
                    }
                    Ok(Message::Close(_)) => {
                        tracing::info!("WebSocket connection closed: {}", connection_id_recv);
                        break;
                    }
                    Ok(Message::Ping(data)) => {
                        // Répond automatiquement au ping
                        if let Err(e) = socket_sender.send(Message::Pong(data)).await {
                            tracing::error!("Failed to send pong: {}", e);
                            break;
                        }
                    }
                    Ok(Message::Pong(_)) => {
                        // Ping reçu, met à jour l'activité
                        let mut manager = state_recv.connection_manager.write().await;
                        manager.update_activity(&connection_id_recv, 0).await;
                    }
                    Ok(Message::Binary(_)) => {
                        // Messages binaires non supportés pour l'instant
                        let error_msg = MessageBuilder::error(
                            "UNSUPPORTED_MESSAGE_TYPE".to_string(),
                            "Binary messages are not supported".to_string(),
                        );
                        let _ = message_sender_recv.send(error_msg);
                    }
                    Err(e) => {
                        tracing::error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }
        });

        // Tâche de ping périodique
        let ping_task = Self::start_ping_task(
            self.connection_id.clone(),
            self.message_sender.clone(),
            self.state.config.ping_interval,
        );

        // Attend qu'une des tâches se termine
        tokio::select! {
            _ = send_task => tracing::debug!("Send task ended"),
            _ = recv_task => tracing::debug!("Receive task ended"),
            _ = ping_task => tracing::debug!("Ping task ended"),
        }

        // Nettoie la connexion
        {
            let mut manager = self.state.connection_manager.write().await;
            manager.remove_connection(&self.connection_id).await;
        }

        tracing::info!("WebSocket connection ended: {}", self.connection_id);
    }

    /// Gère un message texte reçu
    async fn handle_text_message(
        text: String,
        connection_id: &str,
        state: &WebSocketState,
        message_sender: &mpsc::UnboundedSender<WsMessage>,
    ) -> WebSocketResult<()> {
        // Vérifie la taille du message
        if text.len() > state.config.max_message_size {
            return Err(WebSocketError::MessageTooLarge(text.len()));
        }

        // Parse le message JSON
        let message: WsMessage = serde_json::from_str(&text)
            .map_err(|e| WebSocketError::InvalidMessageFormat(e.to_string()))?;

        // Valide le message
        MessageValidator::validate(&message)
            .map_err(|e| WebSocketError::InvalidMessageFormat(e))?;

        // Traite selon le type de message
        match message {
            WsMessage::Auth { token } => {
                Self::handle_auth(token, connection_id, state, message_sender).await
            }
            WsMessage::Subscribe { topics, filters } => {
                Self::handle_subscribe(topics, filters, connection_id, state, message_sender).await
            }
            WsMessage::Unsubscribe { topics } => {
                Self::handle_unsubscribe(topics, connection_id, state, message_sender).await
            }
            WsMessage::Ping { .. } => {
                let pong = MessageBuilder::pong();
                message_sender.send(pong)
                    .map_err(|_| WebSocketError::ConnectionClosed)?;
                Ok(())
            }
            WsMessage::ConnectionStatus => {
                Self::handle_connection_status(connection_id, state, message_sender).await
            }
            _ => {
                // Types de messages non supportés pour les clients
                Err(WebSocketError::InvalidMessageFormat(
                    "Message type not supported for client requests".to_string()
                ))
            }
        }
    }

    /// Gère l'authentification
    async fn handle_auth(
        token: String,
        connection_id: &str,
        state: &WebSocketState,
        message_sender: &mpsc::UnboundedSender<WsMessage>,
    ) -> WebSocketResult<()> {
        // Valide le token JWT
        let claims = match state.server_state.auth_service.validate_token(&token) {
            Ok(claims) => claims,
            Err(_) => {
                let response = MessageBuilder::auth_failure(
                    "Invalid or expired token".to_string()
                );
                message_sender.send(response)
                    .map_err(|_| WebSocketError::ConnectionClosed)?;
                return Ok(());
            }
        };

        // Crée les informations d'authentification
        let scopes = claims.scope.iter()
            .filter_map(|s| crate::api::auth::ApiScope::from_str(s))
            .collect();

        let auth_info = AuthInfo {
            claims: claims.clone(),
            user_id: claims.sub.clone(),
            scopes,
        };

        // Authentifie la connexion
        {
            let mut manager = state.connection_manager.write().await;
            if let Err(e) = manager.authenticate_connection(connection_id, auth_info).await {
                let response = MessageBuilder::auth_failure(e.to_string());
                message_sender.send(response)
                    .map_err(|_| WebSocketError::ConnectionClosed)?;
                return Ok(());
            }
        }

        // Envoie la confirmation
        let response = MessageBuilder::auth_success(
            claims.sub,
            claims.scope,
        );
        message_sender.send(response)
            .map_err(|_| WebSocketError::ConnectionClosed)?;

        Ok(())
    }

    /// Gère les souscriptions
    async fn handle_subscribe(
        topics: Vec<String>,
        _filters: Option<std::collections::HashMap<String, serde_json::Value>>,
        connection_id: &str,
        state: &WebSocketState,
        message_sender: &mpsc::UnboundedSender<WsMessage>,
    ) -> WebSocketResult<()> {
        let mut successful_topics = Vec::new();
        let mut manager = state.connection_manager.write().await;

        for topic in topics {
            match manager.subscribe_to_topic(connection_id, &topic).await {
                Ok(()) => successful_topics.push(topic),
                Err(e) => {
                    let error_msg = MessageBuilder::subscription_error(topic, e.to_string());
                    let _ = message_sender.send(error_msg);
                }
            }
        }

        if !successful_topics.is_empty() {
            let subscription_id = uuid::Uuid::new_v4().to_string();
            let confirmation = MessageBuilder::subscription_confirmed(
                successful_topics,
                subscription_id,
            );
            message_sender.send(confirmation)
                .map_err(|_| WebSocketError::ConnectionClosed)?;
        }

        Ok(())
    }

    /// Gère les désouscriptions
    async fn handle_unsubscribe(
        topics: Vec<String>,
        connection_id: &str,
        state: &WebSocketState,
        message_sender: &mpsc::UnboundedSender<WsMessage>,
    ) -> WebSocketResult<()> {
        let mut manager = state.connection_manager.write().await;

        for topic in topics {
            if let Err(e) = manager.unsubscribe_from_topic(connection_id, &topic).await {
                tracing::warn!("Failed to unsubscribe from {}: {}", topic, e);
            }
        }

        let success_msg = MessageBuilder::success("Unsubscribed successfully".to_string());
        message_sender.send(success_msg)
            .map_err(|_| WebSocketError::ConnectionClosed)?;

        Ok(())
    }

    /// Gère la demande de statut de connexion
    async fn handle_connection_status(
        connection_id: &str,
        state: &WebSocketState,
        message_sender: &mpsc::UnboundedSender<WsMessage>,
    ) -> WebSocketResult<()> {
        let manager = state.connection_manager.read().await;
        
        if let Some(connection) = manager.get_connection(connection_id) {
            let stats = connection.stats.read().await.clone();
            
            let response = WsMessage::ConnectionStatusResponse {
                connection_id: connection.id.clone(),
                authenticated: connection.auth_info.is_some(),
                subscriptions: connection.subscriptions.iter().cloned().collect(),
                connected_since: chrono::DateTime::from_timestamp(
                    connection.connected_at.elapsed().as_secs() as i64, 0
                ).unwrap_or_else(chrono::Utc::now),
                stats,
            };

            message_sender.send(response)
                .map_err(|_| WebSocketError::ConnectionClosed)?;
        }

        Ok(())
    }

    /// Démarre la tâche de ping périodique
    async fn start_ping_task(
        connection_id: String,
        message_sender: mpsc::UnboundedSender<WsMessage>,
        ping_interval: u64,
    ) {
        let mut interval = interval(Duration::from_secs(ping_interval));
        
        loop {
            interval.tick().await;
            
            let ping_msg = MessageBuilder::ping();
            if message_sender.send(ping_msg).is_err() {
                tracing::debug!("Ping task ending for connection {}", connection_id);
                break;
            }
        }
    }
}

// Re-export pour compatibilité
// Re-export removed to avoid duplicate definition

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ApiConfig, server::ServerState};
    use crate::{Blockchain, BlockchainConfig};
    use std::sync::Arc;

    fn create_test_state() -> WebSocketState {
        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let auth_service = Arc::new(
            crate::api::auth::AuthService::new(crate::api::auth::AuthConfig::default()).unwrap()
        );
        let user_manager = Arc::new(tokio::sync::RwLock::new(
            crate::api::auth::UserManager::new()
        ));
        let config = ApiConfig::default();

        let server_state = ServerState::new(blockchain, auth_service, user_manager, config);
        let ws_config = super::WebSocketConfig::default();
        
        WebSocketState::new(ws_config, server_state)
    }

    #[tokio::test]
    async fn test_handle_auth_invalid_token() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let result = WebSocketHandler::handle_auth(
            "invalid_token".to_string(),
            "conn_1",
            &state,
            &tx,
        ).await;

        assert!(result.is_ok());
        
        // Vérifie qu'un message d'erreur a été envoyé
        let message = rx.try_recv().unwrap();
        match message {
            WsMessage::AuthResponse { success, .. } => assert!(!success),
            _ => panic!("Expected AuthResponse"),
        }
    }

    #[tokio::test]
    async fn test_handle_subscribe_without_auth() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Ajoute la connexion sans authentification
        {
            let mut manager = state.connection_manager.write().await;
            let (dummy_tx, _) = mpsc::unbounded_channel();
            manager.add_connection("conn_1".to_string(), dummy_tx, None, None).await.unwrap();
        }

        let result = WebSocketHandler::handle_subscribe(
            vec!["archive_updates".to_string()],
            None,
            "conn_1",
            &state,
            &tx,
        ).await;

        assert!(result.is_ok());
        
        // Vérifie qu'un message d'erreur a été envoyé
        let message = rx.try_recv().unwrap();
        match message {
            WsMessage::SubscriptionError { topic, error } => {
                assert_eq!(topic, "archive_updates");
                assert!(error.contains("Authentication required"));
            }
            _ => panic!("Expected SubscriptionError"),
        }
    }

    #[tokio::test]
    async fn test_handle_unsubscribe() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        let result = WebSocketHandler::handle_unsubscribe(
            vec!["archive_updates".to_string()],
            "conn_1",
            &state,
            &tx,
        ).await;

        assert!(result.is_ok());
        
        // Vérifie qu'un message de succès a été envoyé
        let message = rx.try_recv().unwrap();
        match message {
            WsMessage::Success { .. } => (),
            _ => panic!("Expected Success message"),
        }
    }

    #[tokio::test]
    async fn test_handle_connection_status() {
        let state = create_test_state();
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Ajoute une connexion
        {
            let mut manager = state.connection_manager.write().await;
            let (dummy_tx, _) = mpsc::unbounded_channel();
            manager.add_connection("conn_1".to_string(), dummy_tx, None, None).await.unwrap();
        }

        let result = WebSocketHandler::handle_connection_status("conn_1", &state, &tx).await;
        assert!(result.is_ok());
        
        // Vérifie qu'une réponse de statut a été envoyée
        let message = rx.try_recv().unwrap();
        match message {
            WsMessage::ConnectionStatusResponse { connection_id, authenticated, .. } => {
                assert_eq!(connection_id, "conn_1");
                assert!(!authenticated);
            }
            _ => panic!("Expected ConnectionStatusResponse"),
        }
    }

    #[tokio::test]
    async fn test_message_validation() {
        let valid_auth = WsMessage::Auth {
            token: "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9".to_string(),
        };
        assert!(MessageValidator::validate(&valid_auth).is_ok());

        let invalid_auth = WsMessage::Auth {
            token: "".to_string(),
        };
        assert!(MessageValidator::validate(&invalid_auth).is_err());

        let valid_subscribe = WsMessage::Subscribe {
            topics: vec!["archive_updates".to_string()],
            filters: None,
        };
        assert!(MessageValidator::validate(&valid_subscribe).is_ok());

        let invalid_subscribe = WsMessage::Subscribe {
            topics: vec![],
            filters: None,
        };
        assert!(MessageValidator::validate(&invalid_subscribe).is_err());
    }
}