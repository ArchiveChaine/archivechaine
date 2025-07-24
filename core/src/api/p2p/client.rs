//! Client P2P pour ArchiveChain
//!
//! Implémente le client P2P avec gestion des connexions, envoi/réception de messages
//! et maintien de l'état du réseau.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock, oneshot};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{Duration, timeout};

use super::{P2PConfig, P2PError, P2PResult, messages::*};

/// Client P2P principal
#[derive(Debug)]
pub struct P2PClient {
    /// Configuration
    config: P2PConfig,
    /// Connexions actives
    connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
    /// Canal pour les messages entrants
    message_tx: mpsc::UnboundedSender<IncomingMessage>,
    /// Récepteur pour les messages entrants
    message_rx: Arc<RwLock<Option<mpsc::UnboundedReceiver<IncomingMessage>>>>,
    /// Canal pour arrêter le client
    shutdown_tx: Arc<RwLock<Option<oneshot::Sender<()>>>>,
    /// ID de ce nœud
    node_id: String,
}

/// Connexion vers un pair
#[derive(Debug, Clone)]
pub struct PeerConnection {
    /// ID du pair
    pub peer_id: String,
    /// Adresse du pair
    pub addr: SocketAddr,
    /// Canal pour envoyer des messages à ce pair
    pub sender: mpsc::UnboundedSender<P2PMessage>,
    /// Statut de la connexion
    pub status: ConnectionStatus,
    /// Dernière activité
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Latence moyenne
    pub latency_ms: u64,
}

/// Statut de connexion
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStatus {
    Connecting,
    Handshaking,
    Connected,
    Disconnecting,
    Disconnected,
    Error(String),
}

/// Message entrant avec métadonnées
#[derive(Debug)]
pub struct IncomingMessage {
    pub peer_id: String,
    pub message: P2PMessage,
    pub received_at: chrono::DateTime<chrono::Utc>,
}

impl P2PClient {
    /// Crée un nouveau client P2P
    pub async fn new(config: P2PConfig) -> P2PResult<Self> {
        let (message_tx, message_rx) = mpsc::unbounded_channel();
        let node_id = Self::generate_node_id();

        Ok(Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            message_tx,
            message_rx: Arc::new(RwLock::new(Some(message_rx))),
            shutdown_tx: Arc::new(RwLock::new(None)),
            node_id,
        })
    }

    /// Génère un ID de nœud unique
    fn generate_node_id() -> String {
        format!("node_{}", uuid::Uuid::new_v4().simple())
    }

    /// Démarre le client P2P
    pub async fn start(&self) -> P2PResult<()> {
        tracing::info!("Starting P2P client on port {}", self.config.listen_port);

        let listen_addr = format!("{}:{}", self.config.listen_addr, self.config.listen_port);
        let listener = TcpListener::bind(&listen_addr).await
            .map_err(|e| P2PError::NetworkError(format!("Failed to bind to {}: {}", listen_addr, e)))?;

        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();
        {
            let mut shutdown_guard = self.shutdown_tx.write().await;
            *shutdown_guard = Some(shutdown_tx);
        }

        // Tâche d'écoute des connexions entrantes
        let connections = self.connections.clone();
        let message_tx = self.message_tx.clone();
        let config = self.config.clone();
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    result = listener.accept() => {
                        match result {
                            Ok((stream, addr)) => {
                                tracing::debug!("Incoming connection from {}", addr);
                                
                                if let Err(e) = Self::handle_incoming_connection(
                                    stream,
                                    addr,
                                    connections.clone(),
                                    message_tx.clone(),
                                    config.clone(),
                                    node_id.clone(),
                                ).await {
                                    tracing::error!("Failed to handle incoming connection: {}", e);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to accept connection: {}", e);
                            }
                        }
                    }
                    _ = &mut shutdown_rx => {
                        tracing::info!("P2P client listener shutting down");
                        break;
                    }
                }
            }
        });

        // Tâche de maintenance des connexions
        self.start_maintenance_task().await;

        tracing::info!("P2P client started successfully");
        Ok(())
    }

    /// Arrête le client P2P
    pub async fn stop(&self) -> P2PResult<()> {
        tracing::info!("Stopping P2P client");

        // Envoie le signal d'arrêt
        if let Some(shutdown_tx) = self.shutdown_tx.write().await.take() {
            let _ = shutdown_tx.send(());
        }

        // Ferme toutes les connexions
        let mut connections = self.connections.write().await;
        for (peer_id, connection) in connections.drain() {
            let disconnect_msg = MessageBuilder::disconnect("Client shutting down".to_string());
            let _ = connection.sender.send(disconnect_msg);
        }

        tracing::info!("P2P client stopped");
        Ok(())
    }

    /// Connecte à un pair
    pub async fn connect_to_peer(&self, addr: SocketAddr) -> P2PResult<String> {
        tracing::debug!("Connecting to peer at {}", addr);

        let stream = timeout(
            Duration::from_secs(self.config.connection_timeout),
            TcpStream::connect(addr)
        ).await
        .map_err(|_| P2PError::Timeout)?
        .map_err(|e| P2PError::ConnectionFailed(format!("Failed to connect to {}: {}", addr, e)))?;

        let peer_id = format!("peer_{}", uuid::Uuid::new_v4().simple());
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        // Crée la connexion
        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            addr,
            sender: message_sender,
            status: ConnectionStatus::Connecting,
            last_activity: chrono::Utc::now(),
            latency_ms: 0,
        };

        // Ajoute à la liste des connexions
        {
            let mut connections = self.connections.write().await;
            connections.insert(peer_id.clone(), connection);
        }

        // Lance la tâche de gestion de cette connexion
        let connections = self.connections.clone();
        let message_tx = self.message_tx.clone();
        let config = self.config.clone();
        let node_id = self.node_id.clone();

        tokio::spawn(async move {
            if let Err(e) = Self::handle_outgoing_connection(
                stream,
                peer_id.clone(),
                addr,
                message_receiver,
                connections,
                message_tx,
                config,
                node_id,
            ).await {
                tracing::error!("Connection to {} failed: {}", addr, e);
            }
        });

        Ok(peer_id)
    }

    /// Envoie un message à un pair
    pub async fn send_message(&self, peer_id: &str, message: P2PMessage) -> P2PResult<()> {
        let connections = self.connections.read().await;
        
        if let Some(connection) = connections.get(peer_id) {
            connection.sender.send(message)
                .map_err(|_| P2PError::PeerNotFound(peer_id.to_string()))?;
            Ok(())
        } else {
            Err(P2PError::PeerNotFound(peer_id.to_string()))
        }
    }

    /// Récupère le récepteur de messages
    pub async fn take_message_receiver(&self) -> Option<mpsc::UnboundedReceiver<IncomingMessage>> {
        let mut rx_guard = self.message_rx.write().await;
        rx_guard.take()
    }

    /// Gère une connexion entrante
    async fn handle_incoming_connection(
        stream: TcpStream,
        addr: SocketAddr,
        connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
        message_tx: mpsc::UnboundedSender<IncomingMessage>,
        config: P2PConfig,
        node_id: String,
    ) -> P2PResult<()> {
        let peer_id = format!("peer_{}", uuid::Uuid::new_v4().simple());
        let (message_sender, message_receiver) = mpsc::unbounded_channel();

        // Crée la connexion
        let connection = PeerConnection {
            peer_id: peer_id.clone(),
            addr,
            sender: message_sender,
            status: ConnectionStatus::Handshaking,
            last_activity: chrono::Utc::now(),
            latency_ms: 0,
        };

        // Ajoute à la liste des connexions
        {
            let mut connections_guard = connections.write().await;
            connections_guard.insert(peer_id.clone(), connection);
        }

        // Gère la connexion
        Self::handle_connection(
            stream,
            peer_id,
            addr,
            message_receiver,
            connections,
            message_tx,
            config,
            node_id,
            true, // incoming
        ).await
    }

    /// Gère une connexion sortante
    async fn handle_outgoing_connection(
        stream: TcpStream,
        peer_id: String,
        addr: SocketAddr,
        message_receiver: mpsc::UnboundedReceiver<P2PMessage>,
        connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
        message_tx: mpsc::UnboundedSender<IncomingMessage>,
        config: P2PConfig,
        node_id: String,
    ) -> P2PResult<()> {
        Self::handle_connection(
            stream,
            peer_id,
            addr,
            message_receiver,
            connections,
            message_tx,
            config,
            node_id,
            false, // outgoing
        ).await
    }

    /// Gère une connexion (commune aux entrantes et sortantes)
    async fn handle_connection(
        mut stream: TcpStream,
        peer_id: String,
        addr: SocketAddr,
        mut message_receiver: mpsc::UnboundedReceiver<P2PMessage>,
        connections: Arc<RwLock<HashMap<String, PeerConnection>>>,
        message_tx: mpsc::UnboundedSender<IncomingMessage>,
        config: P2PConfig,
        node_id: String,
        is_incoming: bool,
    ) -> P2PResult<()> {
        tracing::debug!("Handling {} connection with {}", 
            if is_incoming { "incoming" } else { "outgoing" }, addr);

        // Effectue le handshake
        if !is_incoming {
            // Pour les connexions sortantes, envoie le handshake en premier
            let handshake = MessageBuilder::handshake(
                node_id.clone(),
                "1.0".to_string(),
                "archivechain-0.1.0".to_string(),
                0, // TODO: Récupérer la vraie hauteur de bloc
                "0x0".to_string(), // TODO: Récupérer le vrai hash
                vec!["sync".to_string(), "gossip".to_string()],
            );

            Self::send_message_to_stream(&mut stream, &handshake).await?;
        }

        // Divise la stream en read/write
        let (mut read_half, mut write_half) = stream.into_split();

        // Tâche de lecture
        let connections_read = connections.clone();
        let message_tx_read = message_tx.clone();
        let peer_id_read = peer_id.clone();
        let read_task = tokio::spawn(async move {
            let mut buffer = vec![0u8; config.max_message_size];
            
            loop {
                match read_half.read(&mut buffer).await {
                    Ok(0) => {
                        // Connexion fermée
                        tracing::debug!("Connection closed by peer {}", peer_id_read);
                        break;
                    }
                    Ok(n) => {
                        // Message reçu
                        match Self::parse_message(&buffer[..n]) {
                            Ok(message) => {
                                let incoming = IncomingMessage {
                                    peer_id: peer_id_read.clone(),
                                    message,
                                    received_at: chrono::Utc::now(),
                                };
                                
                                if let Err(_) = message_tx_read.send(incoming) {
                                    tracing::error!("Failed to send incoming message to handler");
                                    break;
                                }

                                // Met à jour l'activité
                                if let Some(connection) = connections_read.write().await.get_mut(&peer_id_read) {
                                    connection.last_activity = chrono::Utc::now();
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to parse message from {}: {}", peer_id_read, e);
                            }
                        }
                    }
                    Err(e) => {
                        tracing::error!("Read error from {}: {}", peer_id_read, e);
                        break;
                    }
                }
            }
        });

        // Tâche d'écriture
        let peer_id_write = peer_id.clone();
        let write_task = tokio::spawn(async move {
            while let Some(message) = message_receiver.recv().await {
                if let Err(e) = Self::send_message_to_stream(&mut write_half, &message).await {
                    tracing::error!("Failed to send message to {}: {}", peer_id_write, e);
                    break;
                }
            }
        });

        // Attend qu'une des tâches se termine
        tokio::select! {
            _ = read_task => {},
            _ = write_task => {},
        }

        // Nettoie la connexion
        {
            let mut connections_guard = connections.write().await;
            connections_guard.remove(&peer_id);
        }

        tracing::debug!("Connection with {} ended", addr);
        Ok(())
    }

    /// Envoie un message via une stream
    async fn send_message_to_stream<W>(writer: &mut W, message: &P2PMessage) -> P2PResult<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        let serialized = serde_json::to_vec(message)
            .map_err(|e| P2PError::InvalidMessage)?;

        // Envoie la taille du message d'abord (4 bytes little-endian)
        let size = serialized.len() as u32;
        writer.write_all(&size.to_le_bytes()).await
            .map_err(|e| P2PError::NetworkError(e.to_string()))?;

        // Envoie le message
        writer.write_all(&serialized).await
            .map_err(|e| P2PError::NetworkError(e.to_string()))?;

        writer.flush().await
            .map_err(|e| P2PError::NetworkError(e.to_string()))?;

        Ok(())
    }

    /// Parse un message depuis des bytes
    fn parse_message(data: &[u8]) -> P2PResult<P2PMessage> {
        if data.len() < 4 {
            return Err(P2PError::InvalidMessage);
        }

        // Lit la taille du message
        let size = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        
        if data.len() < 4 + size {
            return Err(P2PError::InvalidMessage);
        }

        // Parse le message JSON
        let message_data = &data[4..4 + size];
        serde_json::from_slice(message_data)
            .map_err(|_| P2PError::InvalidMessage)
    }

    /// Démarre la tâche de maintenance
    async fn start_maintenance_task(&self) {
        let connections = self.connections.clone();
        let config = self.config.clone();

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(config.ping_interval));

            loop {
                interval.tick().await;

                let mut connections_guard = connections.write().await;
                let cutoff = chrono::Utc::now() - chrono::Duration::seconds(config.ping_interval as i64 * 2);

                // Supprime les connexions inactives
                connections_guard.retain(|peer_id, connection| {
                    if connection.last_activity < cutoff {
                        tracing::debug!("Removing inactive connection: {}", peer_id);
                        false
                    } else {
                        // Envoie un ping
                        let ping = MessageBuilder::ping(rand::random());
                        let _ = connection.sender.send(ping);
                        true
                    }
                });
            }
        });
    }

    /// Récupère les connexions actives
    pub async fn get_connections(&self) -> HashMap<String, PeerConnection> {
        let connections = self.connections.read().await;
        connections.clone()
    }

    /// Récupère l'ID de ce nœud
    pub fn node_id(&self) -> &str {
        &self.node_id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[tokio::test]
    async fn test_p2p_client_creation() {
        let config = P2PConfig::default();
        let client = P2PClient::new(config).await;
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert!(!client.node_id().is_empty());
        assert!(client.node_id().starts_with("node_"));
    }

    #[test]
    fn test_connection_status() {
        assert_eq!(ConnectionStatus::Connected, ConnectionStatus::Connected);
        assert_ne!(ConnectionStatus::Connected, ConnectionStatus::Disconnected);
        
        let error_status = ConnectionStatus::Error("test error".to_string());
        match error_status {
            ConnectionStatus::Error(msg) => assert_eq!(msg, "test error"),
            _ => panic!("Expected Error status"),
        }
    }

    #[test]
    fn test_peer_connection_creation() {
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);
        let (tx, _) = mpsc::unbounded_channel();
        
        let connection = PeerConnection {
            peer_id: "peer_123".to_string(),
            addr,
            sender: tx,
            status: ConnectionStatus::Connected,
            last_activity: chrono::Utc::now(),
            latency_ms: 50,
        };
        
        assert_eq!(connection.peer_id, "peer_123");
        assert_eq!(connection.status, ConnectionStatus::Connected);
        assert_eq!(connection.latency_ms, 50);
    }

    #[test]
    fn test_incoming_message() {
        let message = MessageBuilder::ping(12345);
        let incoming = IncomingMessage {
            peer_id: "peer_123".to_string(),
            message,
            received_at: chrono::Utc::now(),
        };
        
        assert_eq!(incoming.peer_id, "peer_123");
        match incoming.message {
            P2PMessage::Ping { nonce, .. } => assert_eq!(nonce, 12345),
            _ => panic!("Expected Ping message"),
        }
    }

    #[test]
    fn test_message_parsing() {
        let ping = MessageBuilder::ping(12345);
        let serialized = serde_json::to_vec(&ping).unwrap();
        
        // Crée le format avec taille
        let size = serialized.len() as u32;
        let mut data = size.to_le_bytes().to_vec();
        data.extend_from_slice(&serialized);
        
        let parsed = P2PClient::parse_message(&data).unwrap();
        match parsed {
            P2PMessage::Ping { nonce, .. } => assert_eq!(nonce, 12345),
            _ => panic!("Expected Ping message"),
        }
    }

    #[test]
    fn test_message_parsing_invalid() {
        // Données trop courtes
        let result = P2PClient::parse_message(&[1, 2]);
        assert!(result.is_err());
        
        // Taille invalide
        let result = P2PClient::parse_message(&[255, 255, 255, 255, 1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_node_id_generation() {
        let id1 = P2PClient::generate_node_id();
        let id2 = P2PClient::generate_node_id();
        
        assert!(id1.starts_with("node_"));
        assert!(id2.starts_with("node_"));
        assert_ne!(id1, id2);
    }
}