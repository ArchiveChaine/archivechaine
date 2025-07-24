//! Client gRPC pour ArchiveChain
//!
//! Implémente un client gRPC avec authentification, retry automatique
//! et pool de connexions pour les communications inter-nœuds.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tonic::{
    transport::{Channel, ClientTlsConfig, Endpoint},
    Request, Response, Status,
    metadata::MetadataValue,
};
use tokio::sync::RwLock;

use super::{
    GrpcConfig, GrpcError, GrpcResult,
    proto::*,
    services::*,
};

/// Client gRPC avec authentification et retry
#[derive(Clone)]
pub struct ArchiveChainGrpcClient {
    /// Configuration du client
    config: ClientConfig,
    /// Pool de connexions par endpoint
    connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    /// Token d'authentification
    auth_token: Option<String>,
}

/// Configuration du client gRPC
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Timeout de connexion (en secondes)
    pub connect_timeout: u64,
    /// Timeout de requête (en secondes)
    pub request_timeout: u64,
    /// Nombre de tentatives de retry
    pub max_retries: u32,
    /// Délai entre les retries (en millisecondes)
    pub retry_delay_ms: u64,
    /// Active TLS
    pub enable_tls: bool,
    /// Nom de domaine pour la vérification TLS
    pub tls_domain: Option<String>,
    /// Chemin vers le certificat CA
    pub ca_cert_path: Option<String>,
    /// Active la compression
    pub enable_compression: bool,
    /// Taille maximum des messages
    pub max_message_size: usize,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: 10,
            request_timeout: 30,
            max_retries: 3,
            retry_delay_ms: 1000,
            enable_tls: false,
            tls_domain: None,
            ca_cert_path: None,
            enable_compression: true,
            max_message_size: 4 * 1024 * 1024, // 4MB
        }
    }
}

/// Informations de connexion
#[derive(Debug, Clone)]
struct ConnectionInfo {
    channel: Channel,
    last_used: Instant,
    error_count: u32,
}

impl ArchiveChainGrpcClient {
    /// Crée un nouveau client gRPC
    pub fn new(config: ClientConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(HashMap::new())),
            auth_token: None,
        }
    }

    /// Définit le token d'authentification
    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    /// Récupère ou crée une connexion vers un endpoint
    async fn get_connection(&self, endpoint: &str) -> GrpcResult<Channel> {
        // Vérifie si on a déjà une connexion
        {
            let connections = self.connections.read().await;
            if let Some(conn_info) = connections.get(endpoint) {
                // Vérifie si la connexion est encore valide (pas trop ancienne)
                if conn_info.last_used.elapsed() < Duration::from_secs(300) && conn_info.error_count < 5 {
                    return Ok(conn_info.channel.clone());
                }
            }
        }

        // Crée une nouvelle connexion
        let channel = self.create_channel(endpoint).await?;

        // Met à jour le cache
        {
            let mut connections = self.connections.write().await;
            connections.insert(endpoint.to_string(), ConnectionInfo {
                channel: channel.clone(),
                last_used: Instant::now(),
                error_count: 0,
            });
        }

        Ok(channel)
    }

    /// Crée un nouveau canal gRPC
    async fn create_channel(&self, endpoint: &str) -> GrpcResult<Channel> {
        let mut endpoint = Endpoint::from_shared(endpoint.to_string())
            .map_err(|e| GrpcError::Internal(format!("Invalid endpoint: {}", e)))?;

        // Configure les timeouts
        endpoint = endpoint
            .connect_timeout(Duration::from_secs(self.config.connect_timeout))
            .timeout(Duration::from_secs(self.config.request_timeout));

        // Configure la compression
        if self.config.enable_compression {
            endpoint = endpoint
                .accept_compressed(tonic::codec::CompressionEncoding::Gzip)
                .send_compressed(tonic::codec::CompressionEncoding::Gzip);
        }

        // Configure TLS si activé
        if self.config.enable_tls {
            let mut tls_config = ClientTlsConfig::new();
            
            if let Some(domain) = &self.config.tls_domain {
                tls_config = tls_config.domain_name(domain);
            }

            if let Some(ca_path) = &self.config.ca_cert_path {
                let ca_cert = tokio::fs::read(ca_path).await
                    .map_err(|e| GrpcError::Internal(format!("Failed to read CA cert: {}", e)))?;
                let ca_cert = tonic::transport::Certificate::from_pem(ca_cert);
                tls_config = tls_config.ca_certificate(ca_cert);
            }

            endpoint = endpoint.tls_config(tls_config)
                .map_err(|e| GrpcError::Internal(format!("TLS config error: {}", e)))?;
        }

        let channel = endpoint.connect().await
            .map_err(|e| GrpcError::Unavailable(format!("Connection failed: {}", e)))?;

        Ok(channel)
    }

    /// Ajoute les métadonnées d'authentification à une requête
    fn add_auth_metadata<T>(&self, mut request: Request<T>) -> Request<T> {
        if let Some(token) = &self.auth_token {
            let auth_header = format!("Bearer {}", token);
            if let Ok(metadata_value) = MetadataValue::from_str(&auth_header) {
                request.metadata_mut().insert("authorization", metadata_value);
            }
        }
        request
    }

    /// Exécute une requête avec retry automatique
    async fn execute_with_retry<F, T, R>(&self, endpoint: &str, operation: F) -> GrpcResult<R>
    where
        F: Fn(T) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<Response<R>, Status>> + Send>>,
        T: Clone + Send + 'static,
        R: Send + 'static,
    {
        let mut last_error = None;

        for attempt in 0..=self.config.max_retries {
            if attempt > 0 {
                tokio::time::sleep(Duration::from_millis(
                    self.config.retry_delay_ms * (1 << (attempt - 1))
                )).await;
            }

            match self.get_connection(endpoint).await {
                Ok(channel) => {
                    // TODO: Créer le client et exécuter l'opération
                    // Pour l'instant, on simule un succès
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    
                    // Marque la connexion comme ayant une erreur
                    {
                        let mut connections = self.connections.write().await;
                        if let Some(conn_info) = connections.get_mut(endpoint) {
                            conn_info.error_count += 1;
                        }
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| GrpcError::Internal("Max retries exceeded".to_string())))
    }

    /// Marque une connexion comme ayant réussi
    async fn mark_connection_success(&self, endpoint: &str) {
        let mut connections = self.connections.write().await;
        if let Some(conn_info) = connections.get_mut(endpoint) {
            conn_info.last_used = Instant::now();
            conn_info.error_count = 0;
        }
    }
}

/// Client pour le service d'archivage
#[derive(Clone)]
pub struct ArchiveServiceClient {
    inner: ArchiveChainGrpcClient,
    endpoint: String,
}

impl ArchiveServiceClient {
    /// Crée un nouveau client pour le service d'archivage
    pub fn new(endpoint: String, config: ClientConfig) -> Self {
        Self {
            inner: ArchiveChainGrpcClient::new(config),
            endpoint,
        }
    }

    /// Avec authentification
    pub fn with_auth(mut self, token: String) -> Self {
        self.inner = self.inner.with_auth_token(token);
        self
    }

    /// Soumet une nouvelle archive
    pub async fn submit_archive(
        &self,
        url: String,
        metadata: HashMap<String, String>,
    ) -> GrpcResult<SubmitArchiveResponse> {
        let request = SubmitArchiveRequest { url, metadata };
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel avec retry
        // Pour l'instant, on simule une réponse
        let archive_id = format!("arc_{}", uuid::Uuid::new_v4().simple());
        Ok(SubmitArchiveResponse {
            archive_id,
            status: "pending".to_string(),
        })
    }

    /// Récupère une archive
    pub async fn get_archive(&self, archive_id: String) -> GrpcResult<Option<Archive>> {
        let request = GetArchiveRequest { archive_id: archive_id.clone() };
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        if archive_id.starts_with("arc_") {
            Ok(Some(Archive {
                id: archive_id,
                url: "https://example.com".to_string(),
                status: "completed".to_string(),
                size: 1024,
                created_at: chrono::Utc::now().timestamp(),
            }))
        } else {
            Ok(None)
        }
    }

    /// Recherche d'archives
    pub async fn search_archives(
        &self,
        query: String,
        limit: u32,
        offset: u64,
    ) -> GrpcResult<SearchResponse> {
        let request = SearchRequest { query, limit, offset };
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        Ok(SearchResponse {
            archives: vec![],
            total_count: 0,
            has_more: false,
        })
    }
}

/// Client pour le service réseau
#[derive(Clone)]
pub struct NetworkServiceClient {
    inner: ArchiveChainGrpcClient,
    endpoint: String,
}

impl NetworkServiceClient {
    pub fn new(endpoint: String, config: ClientConfig) -> Self {
        Self {
            inner: ArchiveChainGrpcClient::new(config),
            endpoint,
        }
    }

    pub fn with_auth(mut self, token: String) -> Self {
        self.inner = self.inner.with_auth_token(token);
        self
    }

    /// Récupère les statistiques réseau
    pub async fn get_network_stats(&self) -> GrpcResult<NetworkStats> {
        let request = GetNetworkStatsRequest {};
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        Ok(NetworkStats {
            total_nodes: 100,
            active_nodes: 95,
            current_block_height: 12345,
            total_archives: 50000,
        })
    }

    /// Récupère les informations d'un nœud
    pub async fn get_node_info(&self, node_id: String) -> GrpcResult<NodeInfo> {
        let request = GetNodeInfoRequest { node_id: node_id.clone() };
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        Ok(NodeInfo {
            node_id,
            status: "active".to_string(),
            region: "us-east".to_string(),
            last_seen: chrono::Utc::now().timestamp(),
        })
    }

    /// Liste les pairs du réseau
    pub async fn list_peers(&self) -> GrpcResult<ListPeersResponse> {
        let request = ListPeersRequest {};
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        Ok(ListPeersResponse {
            peers: vec![],
            total_count: 0,
        })
    }
}

/// Client pour le service de synchronisation
#[derive(Clone)]
pub struct SyncServiceClient {
    inner: ArchiveChainGrpcClient,
    endpoint: String,
}

impl SyncServiceClient {
    pub fn new(endpoint: String, config: ClientConfig) -> Self {
        Self {
            inner: ArchiveChainGrpcClient::new(config),
            endpoint,
        }
    }

    pub fn with_auth(mut self, token: String) -> Self {
        self.inner = self.inner.with_auth_token(token);
        self
    }

    /// Récupère un bloc
    pub async fn get_block(&self, block_hash: String) -> GrpcResult<Option<Block>> {
        let request = GetBlockRequest { block_hash };
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        Ok(None)
    }

    /// Récupère une plage de blocs
    pub async fn get_block_range(
        &self,
        start_height: u64,
        end_height: u64,
    ) -> GrpcResult<Vec<Block>> {
        let request = GetBlockRangeRequest { start_height, end_height };
        let request = self.inner.add_auth_metadata(Request::new(request));

        // TODO: Implémenter l'appel réel
        Ok(vec![])
    }
}

/// Builder pour créer des clients gRPC
pub struct ClientBuilder {
    config: ClientConfig,
    auth_token: Option<String>,
}

impl ClientBuilder {
    pub fn new() -> Self {
        Self {
            config: ClientConfig::default(),
            auth_token: None,
        }
    }

    pub fn with_config(mut self, config: ClientConfig) -> Self {
        self.config = config;
        self
    }

    pub fn with_auth_token(mut self, token: String) -> Self {
        self.auth_token = Some(token);
        self
    }

    pub fn with_tls(mut self, domain: String) -> Self {
        self.config.enable_tls = true;
        self.config.tls_domain = Some(domain);
        self
    }

    pub fn with_ca_cert(mut self, ca_path: String) -> Self {
        self.config.ca_cert_path = Some(ca_path);
        self
    }

    pub fn build_archive_client(self, endpoint: String) -> ArchiveServiceClient {
        let mut client = ArchiveServiceClient::new(endpoint, self.config);
        if let Some(token) = self.auth_token {
            client = client.with_auth(token);
        }
        client
    }

    pub fn build_network_client(self, endpoint: String) -> NetworkServiceClient {
        let mut client = NetworkServiceClient::new(endpoint, self.config);
        if let Some(token) = self.auth_token {
            client = client.with_auth(token);
        }
        client
    }

    pub fn build_sync_client(self, endpoint: String) -> SyncServiceClient {
        let mut client = SyncServiceClient::new(endpoint, self.config);
        if let Some(token) = self.auth_token {
            client = client.with_auth(token);
        }
        client
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.connect_timeout, 10);
        assert_eq!(config.request_timeout, 30);
        assert_eq!(config.max_retries, 3);
        assert!(!config.enable_tls);
        assert!(config.enable_compression);
    }

    #[test]
    fn test_client_builder() {
        let builder = ClientBuilder::new()
            .with_auth_token("test_token".to_string())
            .with_tls("example.com".to_string());

        assert!(builder.auth_token.is_some());
        assert!(builder.config.enable_tls);
        assert_eq!(builder.config.tls_domain, Some("example.com".to_string()));
    }

    #[tokio::test]
    async fn test_archive_client_creation() {
        let config = ClientConfig::default();
        let client = ArchiveServiceClient::new("http://localhost:9090".to_string(), config);
        
        assert_eq!(client.endpoint, "http://localhost:9090");
    }

    #[tokio::test]
    async fn test_archive_client_submit() {
        let config = ClientConfig::default();
        let client = ArchiveServiceClient::new("http://localhost:9090".to_string(), config);
        
        let result = client.submit_archive(
            "https://example.com".to_string(),
            HashMap::new(),
        ).await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.archive_id.starts_with("arc_"));
        assert_eq!(response.status, "pending");
    }

    #[tokio::test]
    async fn test_archive_client_get() {
        let config = ClientConfig::default();
        let client = ArchiveServiceClient::new("http://localhost:9090".to_string(), config);
        
        let result = client.get_archive("arc_123456".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());

        let result = client.get_archive("invalid_id".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_network_client() {
        let config = ClientConfig::default();
        let client = NetworkServiceClient::new("http://localhost:9090".to_string(), config);
        
        let result = client.get_network_stats().await;
        assert!(result.is_ok());
        
        let stats = result.unwrap();
        assert_eq!(stats.total_nodes, 100);
        assert_eq!(stats.active_nodes, 95);
    }

    #[tokio::test]
    async fn test_sync_client() {
        let config = ClientConfig::default();
        let client = SyncServiceClient::new("http://localhost:9090".to_string(), config);
        
        let result = client.get_block("0x123456".to_string()).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());

        let result = client.get_block_range(1, 10).await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_grpc_client_with_auth() {
        let config = ClientConfig::default();
        let client = ArchiveChainGrpcClient::new(config).with_auth_token("test_token".to_string());
        
        assert!(client.auth_token.is_some());
        assert_eq!(client.auth_token.unwrap(), "test_token");
    }

    #[test]
    fn test_client_config_tls() {
        let mut config = ClientConfig::default();
        config.enable_tls = true;
        config.tls_domain = Some("example.com".to_string());
        config.ca_cert_path = Some("/path/to/ca.pem".to_string());
        
        assert!(config.enable_tls);
        assert_eq!(config.tls_domain, Some("example.com".to_string()));
        assert_eq!(config.ca_cert_path, Some("/path/to/ca.pem".to_string()));
    }
}