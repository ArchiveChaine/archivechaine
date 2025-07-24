//! Gestion des connexions WebSocket pour ArchiveChain
//!
//! Gère le cycle de vie des connexions WebSocket, l'authentification,
//! les souscriptions et le rate limiting.

use axum::extract::ws::{WebSocket, Message};
use futures_util::{SinkExt, StreamExt};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock, mpsc};
use tokio::time::{Duration, Instant};

use crate::api::{
    auth::{AuthService, JwtClaims, ApiScope},
    middleware::AuthInfo,
};
use super::{
    messages::*,
    WebSocketConfig, WebSocketError, WebSocketResult,
    ConnectionStats, ConnectionInfo,
};

/// Gestionnaire des connexions WebSocket
#[derive(Debug)]
pub struct ConnectionManager {
    /// Configuration WebSocket
    config: WebSocketConfig,
    /// Connexions actives par ID
    connections: HashMap<String, Arc<Connection>>,
    /// Connexions par utilisateur
    connections_by_user: HashMap<String, HashSet<String>>,
    /// Topics et leurs abonnés
    topic_subscribers: HashMap<String, HashSet<String>>,
    /// Canaux de diffusion par topic
    broadcast_channels: HashMap<String, broadcast::Sender<WsMessage>>,
    /// Statistiques globales
    stats: GlobalStats,
    /// Heure de démarrage
    start_time: Instant,
}

/// Connexion WebSocket individuelle
#[derive(Debug)]
pub struct Connection {
    /// ID unique de la connexion
    pub id: String,
    /// Informations d'authentification (si connecté)
    pub auth_info: Option<AuthInfo>,
    /// Topics auxquels cette connexion est abonnée
    pub subscriptions: HashSet<String>,
    /// Heure de connexion
    pub connected_at: Instant,
    /// Dernière activité
    pub last_activity: RwLock<Instant>,
    /// Statistiques de cette connexion
    pub stats: RwLock<ConnectionStatsData>,
    /// Canal pour envoyer des messages à cette connexion
    pub sender: mpsc::UnboundedSender<WsMessage>,
    /// Adresse IP distante
    pub remote_addr: Option<String>,
    /// User-Agent
    pub user_agent: Option<String>,
}

/// Statistiques globales du serveur WebSocket
#[derive(Debug, Default)]
pub struct GlobalStats {
    pub total_connections: u64,
    pub current_connections: usize,
    pub authenticated_connections: usize,
    pub total_messages_sent: u64,
    pub total_messages_received: u64,
    pub total_bytes_sent: u64,
    pub total_bytes_received: u64,
}

impl ConnectionManager {
    /// Crée un nouveau gestionnaire de connexions
    pub fn new(config: WebSocketConfig) -> Self {
        let mut broadcast_channels = HashMap::new();
        
        // Crée les canaux de diffusion pour chaque topic
        for topic in SubscriptionTopic::all_topics() {
            let (tx, _) = broadcast::channel(1000);
            broadcast_channels.insert(topic.as_str().to_string(), tx);
        }

        Self {
            config,
            connections: HashMap::new(),
            connections_by_user: HashMap::new(),
            topic_subscribers: HashMap::new(),
            broadcast_channels,
            stats: GlobalStats::default(),
            start_time: Instant::now(),
        }
    }

    /// Ajoute une nouvelle connexion
    pub async fn add_connection(
        &mut self,
        connection_id: String,
        sender: mpsc::UnboundedSender<WsMessage>,
        remote_addr: Option<String>,
        user_agent: Option<String>,
    ) -> WebSocketResult<()> {
        // Vérifie les limites de connexions
        if self.connections.len() >= self.config.max_total_connections {
            return Err(WebSocketError::ConnectionLimitExceeded);
        }

        let connection = Arc::new(Connection {
            id: connection_id.clone(),
            auth_info: None,
            subscriptions: HashSet::new(),
            connected_at: Instant::now(),
            last_activity: RwLock::new(Instant::now()),
            stats: RwLock::new(ConnectionStatsData {
                messages_sent: 0,
                messages_received: 0,
                bytes_sent: 0,
                bytes_received: 0,
                last_activity: chrono::Utc::now(),
            }),
            sender,
            remote_addr,
            user_agent,
        });

        self.connections.insert(connection_id, connection);
        self.stats.total_connections += 1;
        self.stats.current_connections += 1;

        Ok(())
    }

    /// Supprime une connexion
    pub async fn remove_connection(&mut self, connection_id: &str) {
        if let Some(connection) = self.connections.remove(connection_id) {
            // Supprime de la liste des connexions par utilisateur
            if let Some(auth_info) = &connection.auth_info {
                if let Some(user_connections) = self.connections_by_user.get_mut(&auth_info.user_id) {
                    user_connections.remove(connection_id);
                    if user_connections.is_empty() {
                        self.connections_by_user.remove(&auth_info.user_id);
                    }
                }
                self.stats.authenticated_connections -= 1;
            }

            // Supprime des souscriptions
            for topic in &connection.subscriptions {
                if let Some(subscribers) = self.topic_subscribers.get_mut(topic) {
                    subscribers.remove(connection_id);
                    if subscribers.is_empty() {
                        self.topic_subscribers.remove(topic);
                    }
                }
            }

            self.stats.current_connections -= 1;
        }
    }

    /// Authentifie une connexion
    pub async fn authenticate_connection(
        &mut self,
        connection_id: &str,
        auth_info: AuthInfo,
    ) -> WebSocketResult<()> {
        let connection = self.connections.get_mut(connection_id)
            .ok_or(WebSocketError::ConnectionClosed)?;

        // Vérifie les limites par utilisateur
        if let Some(user_connections) = self.connections_by_user.get(&auth_info.user_id) {
            if user_connections.len() >= self.config.max_connections_per_user {
                return Err(WebSocketError::ConnectionLimitExceeded);
            }
        }

        // Met à jour la connexion
        if let Some(connection) = Arc::get_mut(connection) {
            connection.auth_info = Some(auth_info.clone());
        }

        // Ajoute à la liste des connexions par utilisateur
        self.connections_by_user
            .entry(auth_info.user_id)
            .or_insert_with(HashSet::new)
            .insert(connection_id.to_string());

        self.stats.authenticated_connections += 1;

        Ok(())
    }

    /// Souscrit une connexion à un topic
    pub async fn subscribe_to_topic(
        &mut self,
        connection_id: &str,
        topic: &str,
    ) -> WebSocketResult<()> {
        let connection = self.connections.get_mut(connection_id)
            .ok_or(WebSocketError::ConnectionClosed)?;

        // Vérifie les permissions
        if let Some(topic_enum) = SubscriptionTopic::from_str(topic) {
            if topic_enum.requires_auth() {
                let auth_info = connection.auth_info.as_ref()
                    .ok_or(WebSocketError::AuthenticationRequired)?;

                if let Some(required_scope) = topic_enum.required_scope() {
                    let required_scope_enum = match required_scope {
                        "archives:read" => ApiScope::ArchivesRead,
                        "network:read" => ApiScope::NetworkRead,
                        "node:manage" => ApiScope::NodeManage,
                        "admin:all" => ApiScope::AdminAll,
                        _ => return Err(WebSocketError::PermissionDenied(topic.to_string())),
                    };

                    if !auth_info.scopes.contains(&required_scope_enum) && 
                       !auth_info.scopes.contains(&ApiScope::AdminAll) {
                        return Err(WebSocketError::PermissionDenied(topic.to_string()));
                    }
                }
            }
        } else {
            return Err(WebSocketError::TopicNotFound(topic.to_string()));
        }

        // Ajoute la souscription
        if let Some(connection) = Arc::get_mut(connection) {
            connection.subscriptions.insert(topic.to_string());
        }

        self.topic_subscribers
            .entry(topic.to_string())
            .or_insert_with(HashSet::new)
            .insert(connection_id.to_string());

        Ok(())
    }

    /// Désabonne une connexion d'un topic
    pub async fn unsubscribe_from_topic(
        &mut self,
        connection_id: &str,
        topic: &str,
    ) -> WebSocketResult<()> {
        let connection = self.connections.get_mut(connection_id)
            .ok_or(WebSocketError::ConnectionClosed)?;

        // Supprime la souscription
        if let Some(connection) = Arc::get_mut(connection) {
            connection.subscriptions.remove(topic);
        }

        if let Some(subscribers) = self.topic_subscribers.get_mut(topic) {
            subscribers.remove(connection_id);
            if subscribers.is_empty() {
                self.topic_subscribers.remove(topic);
            }
        }

        Ok(())
    }

    /// Diffuse un message à tous les abonnés d'un topic
    pub async fn broadcast_to_topic(&mut self, topic: &str, message: WsMessage) -> WebSocketResult<usize> {
        let subscribers = self.topic_subscribers.get(topic)
            .map(|s| s.clone())
            .unwrap_or_default();

        let mut sent_count = 0;

        for connection_id in subscribers {
            if let Some(connection) = self.connections.get(&connection_id) {
                if let Err(_) = connection.sender.send(message.clone()) {
                    // Connexion fermée, on la supprimera au prochain nettoyage
                    tracing::warn!("Failed to send message to connection {}", connection_id);
                } else {
                    sent_count += 1;
                    
                    // Met à jour les statistiques
                    if let Ok(mut stats) = connection.stats.write().await {
                        stats.messages_sent += 1;
                        stats.last_activity = chrono::Utc::now();
                    }
                }
            }
        }

        self.stats.total_messages_sent += sent_count as u64;

        Ok(sent_count)
    }

    /// Envoie un message à une connexion spécifique
    pub async fn send_to_connection(
        &mut self,
        connection_id: &str,
        message: WsMessage,
    ) -> WebSocketResult<()> {
        let connection = self.connections.get(connection_id)
            .ok_or(WebSocketError::ConnectionClosed)?;

        connection.sender.send(message)
            .map_err(|_| WebSocketError::ConnectionClosed)?;

        // Met à jour les statistiques
        if let Ok(mut stats) = connection.stats.write().await {
            stats.messages_sent += 1;
            stats.last_activity = chrono::Utc::now();
        }

        self.stats.total_messages_sent += 1;

        Ok(())
    }

    /// Met à jour l'activité d'une connexion
    pub async fn update_activity(&mut self, connection_id: &str, bytes_received: u64) {
        if let Some(connection) = self.connections.get(connection_id) {
            if let Ok(mut last_activity) = connection.last_activity.write().await {
                *last_activity = Instant::now();
            }

            if let Ok(mut stats) = connection.stats.write().await {
                stats.messages_received += 1;
                stats.bytes_received += bytes_received;
                stats.last_activity = chrono::Utc::now();
            }
        }

        self.stats.total_messages_received += 1;
        self.stats.total_bytes_received += bytes_received;
    }

    /// Nettoie les connexions inactives
    pub async fn cleanup_inactive_connections(&mut self, timeout: Duration) {
        let cutoff = Instant::now() - timeout;
        let mut to_remove = Vec::new();

        for (connection_id, connection) in &self.connections {
            if let Ok(last_activity) = connection.last_activity.read().await {
                if *last_activity < cutoff {
                    to_remove.push(connection_id.clone());
                }
            }
        }

        for connection_id in to_remove {
            self.remove_connection(&connection_id).await;
        }
    }

    /// Récupère les statistiques
    pub async fn get_stats(&self) -> ConnectionStats {
        let mut connections_by_user = HashMap::new();
        let mut subscriptions_by_topic = HashMap::new();

        for connection in self.connections.values() {
            if let Some(auth_info) = &connection.auth_info {
                *connections_by_user.entry(auth_info.user_id.clone()).or_insert(0) += 1;
            }
        }

        for (topic, subscribers) in &self.topic_subscribers {
            subscriptions_by_topic.insert(topic.clone(), subscribers.len());
        }

        ConnectionStats {
            total_connections: self.stats.current_connections,
            authenticated_connections: self.stats.authenticated_connections,
            anonymous_connections: self.stats.current_connections - self.stats.authenticated_connections,
            connections_by_user,
            subscriptions_by_topic,
            total_messages_sent: self.stats.total_messages_sent,
            total_messages_received: self.stats.total_messages_received,
            uptime_seconds: self.start_time.elapsed().as_secs(),
        }
    }

    /// Récupère toutes les connexions actives
    pub async fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        let mut connections = Vec::new();

        for connection in self.connections.values() {
            let stats = connection.stats.read().await.clone();
            
            connections.push(ConnectionInfo {
                connection_id: connection.id.clone(),
                user_id: connection.auth_info.as_ref().map(|a| a.user_id.clone()),
                connected_at: chrono::DateTime::from_timestamp(
                    connection.connected_at.elapsed().as_secs() as i64, 0
                ).unwrap_or_else(chrono::Utc::now),
                last_activity: stats.last_activity,
                subscriptions: connection.subscriptions.iter().cloned().collect(),
                messages_sent: stats.messages_sent,
                messages_received: stats.messages_received,
                remote_addr: connection.remote_addr.clone(),
                user_agent: connection.user_agent.clone(),
            });
        }

        connections
    }

    /// Récupère une connexion par ID
    pub fn get_connection(&self, connection_id: &str) -> Option<&Arc<Connection>> {
        self.connections.get(connection_id)
    }

    /// Récupère le nombre de connexions d'un utilisateur
    pub fn get_user_connection_count(&self, user_id: &str) -> usize {
        self.connections_by_user
            .get(user_id)
            .map(|connections| connections.len())
            .unwrap_or(0)
    }

    /// Récupère le nombre d'abonnés à un topic
    pub fn get_topic_subscriber_count(&self, topic: &str) -> usize {
        self.topic_subscribers
            .get(topic)
            .map(|subscribers| subscribers.len())
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::auth::ApiScope;

    fn create_test_auth_info() -> AuthInfo {
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

        AuthInfo {
            claims,
            user_id: "test_user".to_string(),
            scopes: vec![ApiScope::ArchivesRead],
        }
    }

    #[tokio::test]
    async fn test_connection_manager_creation() {
        let config = WebSocketConfig::default();
        let manager = ConnectionManager::new(config);
        
        assert_eq!(manager.connections.len(), 0);
        assert_eq!(manager.stats.current_connections, 0);
    }

    #[tokio::test]
    async fn test_add_remove_connection() {
        let config = WebSocketConfig::default();
        let mut manager = ConnectionManager::new(config);
        let (tx, _) = mpsc::unbounded_channel();

        // Ajoute une connexion
        let result = manager.add_connection(
            "conn_1".to_string(),
            tx,
            Some("127.0.0.1:12345".to_string()),
            Some("Test Client".to_string()),
        ).await;

        assert!(result.is_ok());
        assert_eq!(manager.connections.len(), 1);
        assert_eq!(manager.stats.current_connections, 1);

        // Supprime la connexion
        manager.remove_connection("conn_1").await;
        assert_eq!(manager.connections.len(), 0);
        assert_eq!(manager.stats.current_connections, 0);
    }

    #[tokio::test]
    async fn test_authentication() {
        let config = WebSocketConfig::default();
        let mut manager = ConnectionManager::new(config);
        let (tx, _) = mpsc::unbounded_channel();

        manager.add_connection(
            "conn_1".to_string(),
            tx,
            None,
            None,
        ).await.unwrap();

        let auth_info = create_test_auth_info();
        let result = manager.authenticate_connection("conn_1", auth_info).await;

        assert!(result.is_ok());
        assert_eq!(manager.stats.authenticated_connections, 1);
        assert_eq!(manager.connections_by_user.len(), 1);
    }

    #[tokio::test]
    async fn test_subscription() {
        let config = WebSocketConfig::default();
        let mut manager = ConnectionManager::new(config);
        let (tx, _) = mpsc::unbounded_channel();

        manager.add_connection("conn_1".to_string(), tx, None, None).await.unwrap();
        
        let auth_info = create_test_auth_info();
        manager.authenticate_connection("conn_1", auth_info).await.unwrap();

        // Souscription à un topic autorisé
        let result = manager.subscribe_to_topic("conn_1", "archive_updates").await;
        assert!(result.is_ok());
        assert_eq!(manager.topic_subscribers.get("archive_updates").unwrap().len(), 1);

        // Souscription à un topic non autorisé
        let result = manager.subscribe_to_topic("conn_1", "admin:all").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_unsubscription() {
        let config = WebSocketConfig::default();
        let mut manager = ConnectionManager::new(config);
        let (tx, _) = mpsc::unbounded_channel();

        manager.add_connection("conn_1".to_string(), tx, None, None).await.unwrap();
        
        let auth_info = create_test_auth_info();
        manager.authenticate_connection("conn_1", auth_info).await.unwrap();
        manager.subscribe_to_topic("conn_1", "archive_updates").await.unwrap();

        // Désabonnement
        let result = manager.unsubscribe_from_topic("conn_1", "archive_updates").await;
        assert!(result.is_ok());
        assert_eq!(manager.topic_subscribers.get("archive_updates").map(|s| s.len()).unwrap_or(0), 0);
    }

    #[tokio::test]
    async fn test_broadcast() {
        let config = WebSocketConfig::default();
        let mut manager = ConnectionManager::new(config);
        let (tx, mut rx) = mpsc::unbounded_channel();

        manager.add_connection("conn_1".to_string(), tx, None, None).await.unwrap();
        
        let auth_info = create_test_auth_info();
        manager.authenticate_connection("conn_1", auth_info).await.unwrap();
        manager.subscribe_to_topic("conn_1", "archive_updates").await.unwrap();

        // Diffuse un message
        let message = MessageBuilder::ping();
        let sent_count = manager.broadcast_to_topic("archive_updates", message).await.unwrap();
        
        assert_eq!(sent_count, 1);
        assert!(rx.try_recv().is_ok());
    }

    #[tokio::test]
    async fn test_connection_limits() {
        let mut config = WebSocketConfig::default();
        config.max_total_connections = 1;
        
        let mut manager = ConnectionManager::new(config);
        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();

        // Première connexion OK
        assert!(manager.add_connection("conn_1".to_string(), tx1, None, None).await.is_ok());
        
        // Deuxième connexion refusée
        assert!(manager.add_connection("conn_2".to_string(), tx2, None, None).await.is_err());
    }

    #[tokio::test]
    async fn test_user_connection_limits() {
        let mut config = WebSocketConfig::default();
        config.max_connections_per_user = 1;
        
        let mut manager = ConnectionManager::new(config);
        let (tx1, _) = mpsc::unbounded_channel();
        let (tx2, _) = mpsc::unbounded_channel();

        manager.add_connection("conn_1".to_string(), tx1, None, None).await.unwrap();
        manager.add_connection("conn_2".to_string(), tx2, None, None).await.unwrap();

        let auth_info = create_test_auth_info();
        
        // Première authentification OK
        assert!(manager.authenticate_connection("conn_1", auth_info.clone()).await.is_ok());
        
        // Deuxième authentification pour le même utilisateur refusée
        assert!(manager.authenticate_connection("conn_2", auth_info).await.is_err());
    }

    #[tokio::test]
    async fn test_stats() {
        let config = WebSocketConfig::default();
        let mut manager = ConnectionManager::new(config);
        let (tx, _) = mpsc::unbounded_channel();

        manager.add_connection("conn_1".to_string(), tx, None, None).await.unwrap();
        
        let stats = manager.get_stats().await;
        assert_eq!(stats.total_connections, 1);
        assert_eq!(stats.anonymous_connections, 1);
        assert_eq!(stats.authenticated_connections, 0);
    }
}