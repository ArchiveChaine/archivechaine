//! Implémentations des services gRPC pour ArchiveChain
//!
//! Contient tous les services gRPC selon les spécifications API.

use std::collections::HashMap;
use tonic::{Request, Response, Status, async_trait};
use futures_util::Stream;
use std::pin::Pin;

use crate::api::server::ServerState;
use super::{GrpcError, GrpcResult, proto::*};

/// Service d'archivage gRPC
#[derive(Debug, Clone)]
pub struct ArchiveServiceImpl {
    state: ServerState,
}

impl ArchiveServiceImpl {
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }

    /// Convertit vers un service tonic
    pub fn into_service(self) -> ArchiveServiceServer {
        ArchiveServiceServer { inner: self }
    }
}

/// Wrapper pour le service d'archivage
pub struct ArchiveServiceServer {
    inner: ArchiveServiceImpl,
}

#[async_trait]
impl ArchiveService for ArchiveServiceServer {
    async fn submit_archive(
        &self,
        request: Request<SubmitArchiveRequest>,
    ) -> Result<Response<SubmitArchiveResponse>, Status> {
        let req = request.into_inner();
        
        // Valide la requête
        if req.url.is_empty() {
            return Err(GrpcError::InvalidRequest("URL is required".to_string()).into());
        }

        // TODO: Valide l'URL
        if let Err(_) = url::Url::parse(&req.url) {
            return Err(GrpcError::InvalidRequest("Invalid URL format".to_string()).into());
        }

        // Génère un ID d'archive
        let archive_id = format!("arc_{}", uuid::Uuid::new_v4().simple());

        // TODO: Ajouter l'archive à la queue de traitement
        tracing::info!("Submitting archive for URL: {}", req.url);

        let response = SubmitArchiveResponse {
            archive_id,
            status: "pending".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn get_archive(
        &self,
        request: Request<GetArchiveRequest>,
    ) -> Result<Response<GetArchiveResponse>, Status> {
        let req = request.into_inner();
        
        if req.archive_id.is_empty() {
            return Err(GrpcError::InvalidRequest("Archive ID is required".to_string()).into());
        }

        // TODO: Récupérer l'archive depuis la blockchain
        tracing::info!("Getting archive: {}", req.archive_id);

        // Pour l'instant, retourne une archive fictive
        if req.archive_id.starts_with("arc_") {
            let archive = Archive {
                id: req.archive_id.clone(),
                url: "https://example.com".to_string(),
                status: "completed".to_string(),
                size: 1024,
                created_at: chrono::Utc::now().timestamp(),
            };

            let response = GetArchiveResponse {
                archive: Some(archive),
            };

            Ok(Response::new(response))
        } else {
            Err(GrpcError::NotFound("Archive not found".to_string()).into())
        }
    }

    async fn search_archives(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        let req = request.into_inner();
        
        if req.query.trim().is_empty() {
            return Err(GrpcError::InvalidRequest("Search query is required".to_string()).into());
        }

        // TODO: Implémenter la recherche réelle
        tracing::info!("Searching archives for: {}", req.query);

        let response = SearchResponse {
            archives: vec![], // Placeholder
            total_count: 0,
            has_more: false,
        };

        Ok(Response::new(response))
    }

    type StreamArchiveUpdatesStream = Pin<Box<dyn Stream<Item = Result<ArchiveUpdate, Status>> + Send>>;

    async fn stream_archive_updates(
        &self,
        request: Request<StreamArchiveUpdatesRequest>,
    ) -> Result<Response<Self::StreamArchiveUpdatesStream>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Starting archive updates stream for archive: {:?}", req.archive_id);

        // TODO: Implémenter le streaming réel depuis le système de pubsub
        let stream = futures_util::stream::empty();
        
        Ok(Response::new(Box::pin(stream)))
    }
}

/// Service réseau gRPC
#[derive(Debug, Clone)]
pub struct NetworkServiceImpl {
    state: ServerState,
}

impl NetworkServiceImpl {
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }

    pub fn into_service(self) -> NetworkServiceServer {
        NetworkServiceServer { inner: self }
    }
}

pub struct NetworkServiceServer {
    inner: NetworkServiceImpl,
}

#[async_trait]
impl NetworkService for NetworkServiceServer {
    async fn get_network_stats(
        &self,
        _request: Request<GetNetworkStatsRequest>,
    ) -> Result<Response<NetworkStats>, Status> {
        // Récupère les statistiques depuis la blockchain
        let blockchain_stats = self.inner.state.blockchain.stats();

        let stats = NetworkStats {
            total_nodes: 100, // TODO: Récupérer le vrai nombre
            active_nodes: 95,
            current_block_height: blockchain_stats.height,
            total_archives: blockchain_stats.total_transactions, // Approximation
        };

        Ok(Response::new(stats))
    }

    async fn get_node_info(
        &self,
        request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<NodeInfo>, Status> {
        let req = request.into_inner();
        
        if req.node_id.is_empty() {
            return Err(GrpcError::InvalidRequest("Node ID is required".to_string()).into());
        }

        // TODO: Récupérer les informations du nœud depuis le réseau P2P
        tracing::info!("Getting node info for: {}", req.node_id);

        let node_info = NodeInfo {
            node_id: req.node_id,
            status: "active".to_string(),
            region: "us-east".to_string(),
            last_seen: chrono::Utc::now().timestamp(),
        };

        Ok(Response::new(node_info))
    }

    async fn list_peers(
        &self,
        _request: Request<ListPeersRequest>,
    ) -> Result<Response<ListPeersResponse>, Status> {
        // TODO: Récupérer la liste des pairs depuis le réseau P2P
        tracing::info!("Listing network peers");

        let response = ListPeersResponse {
            peers: vec![], // Placeholder
            total_count: 0,
        };

        Ok(Response::new(response))
    }
}

/// Service de synchronisation gRPC
#[derive(Debug, Clone)]
pub struct SyncServiceImpl {
    state: ServerState,
}

impl SyncServiceImpl {
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }

    pub fn into_service(self) -> SyncServiceServer {
        SyncServiceServer { inner: self }
    }
}

pub struct SyncServiceServer {
    inner: SyncServiceImpl,
}

#[async_trait]
impl SyncService for SyncServiceServer {
    async fn get_block(
        &self,
        request: Request<GetBlockRequest>,
    ) -> Result<Response<GetBlockResponse>, Status> {
        let req = request.into_inner();
        
        if req.block_hash.is_empty() {
            return Err(GrpcError::InvalidRequest("Block hash is required".to_string()).into());
        }

        // TODO: Récupérer le bloc depuis la blockchain
        tracing::info!("Getting block: {}", req.block_hash);

        // Pour l'instant, retourne None si le bloc n'existe pas
        let response = GetBlockResponse {
            block: None, // TODO: Implémenter la récupération réelle
        };

        Ok(Response::new(response))
    }

    async fn get_block_range(
        &self,
        request: Request<GetBlockRangeRequest>,
    ) -> Result<Response<GetBlockRangeResponse>, Status> {
        let req = request.into_inner();
        
        if req.start_height > req.end_height {
            return Err(GrpcError::InvalidRequest("Start height must be <= end height".to_string()).into());
        }

        let range_size = req.end_height - req.start_height;
        if range_size > 1000 {
            return Err(GrpcError::InvalidRequest("Range too large (max 1000 blocks)".to_string()).into());
        }

        // TODO: Récupérer les blocs depuis la blockchain
        tracing::info!("Getting block range: {} - {}", req.start_height, req.end_height);

        let response = GetBlockRangeResponse {
            blocks: vec![], // Placeholder
        };

        Ok(Response::new(response))
    }

    type SyncBlocksStream = Pin<Box<dyn Stream<Item = Result<Block, Status>> + Send>>;

    async fn sync_blocks(
        &self,
        request: Request<SyncRequest>,
    ) -> Result<Response<Self::SyncBlocksStream>, Status> {
        let req = request.into_inner();
        
        tracing::info!("Starting block sync from height: {}", req.start_height);

        // TODO: Implémenter le streaming des blocs
        let stream = futures_util::stream::empty();
        
        Ok(Response::new(Box::pin(stream)))
    }
}

// Traits de service (normalement générés par tonic-build)
#[async_trait]
pub trait ArchiveService {
    async fn submit_archive(
        &self,
        request: Request<SubmitArchiveRequest>,
    ) -> Result<Response<SubmitArchiveResponse>, Status>;

    async fn get_archive(
        &self,
        request: Request<GetArchiveRequest>,
    ) -> Result<Response<GetArchiveResponse>, Status>;

    async fn search_archives(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status>;

    type StreamArchiveUpdatesStream: Stream<Item = Result<ArchiveUpdate, Status>> + Send + 'static;

    async fn stream_archive_updates(
        &self,
        request: Request<StreamArchiveUpdatesRequest>,
    ) -> Result<Response<Self::StreamArchiveUpdatesStream>, Status>;
}

#[async_trait]
pub trait NetworkService {
    async fn get_network_stats(
        &self,
        request: Request<GetNetworkStatsRequest>,
    ) -> Result<Response<NetworkStats>, Status>;

    async fn get_node_info(
        &self,
        request: Request<GetNodeInfoRequest>,
    ) -> Result<Response<NodeInfo>, Status>;

    async fn list_peers(
        &self,
        request: Request<ListPeersRequest>,
    ) -> Result<Response<ListPeersResponse>, Status>;
}

#[async_trait]
pub trait SyncService {
    async fn get_block(
        &self,
        request: Request<GetBlockRequest>,
    ) -> Result<Response<GetBlockResponse>, Status>;

    async fn get_block_range(
        &self,
        request: Request<GetBlockRangeRequest>,
    ) -> Result<Response<GetBlockRangeResponse>, Status>;

    type SyncBlocksStream: Stream<Item = Result<Block, Status>> + Send + 'static;

    async fn sync_blocks(
        &self,
        request: Request<SyncRequest>,
    ) -> Result<Response<Self::SyncBlocksStream>, Status>;
}

// Types de requête/réponse supplémentaires (complétant proto::*)
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetArchiveRequest {
    pub archive_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetArchiveResponse {
    pub archive: Option<Archive>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub archives: Vec<Archive>,
    pub total_count: u64,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamArchiveUpdatesRequest {
    pub archive_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveUpdate {
    pub archive_id: String,
    pub status: String,
    pub progress: f32,
    pub message: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNetworkStatsRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetNodeInfoRequest {
    pub node_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub status: String,
    pub region: String,
    pub last_seen: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeersRequest {}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListPeersResponse {
    pub peers: Vec<NodeInfo>,
    pub total_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlockRangeRequest {
    pub start_height: u64,
    pub end_height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GetBlockRangeResponse {
    pub blocks: Vec<Block>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{ApiConfig, server::ServerState};

    fn create_test_state() -> ServerState {
        let blockchain = std::sync::Arc::new(
            crate::Blockchain::new(crate::BlockchainConfig::default()).unwrap()
        );
        let auth_service = std::sync::Arc::new(
            crate::api::auth::AuthService::new(crate::api::auth::AuthConfig::default()).unwrap()
        );
        let user_manager = std::sync::Arc::new(tokio::sync::RwLock::new(
            crate::api::auth::UserManager::new()
        ));
        let config = ApiConfig::default();

        ServerState::new(blockchain, auth_service, user_manager, config)
    }

    #[tokio::test]
    async fn test_archive_service_submit_archive() {
        let state = create_test_state();
        let service = ArchiveServiceServer {
            inner: ArchiveServiceImpl::new(state),
        };

        let request = Request::new(SubmitArchiveRequest {
            url: "https://example.com".to_string(),
            metadata: HashMap::new(),
        });

        let response = service.submit_archive(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.archive_id.starts_with("arc_"));
        assert_eq!(response.status, "pending");
    }

    #[tokio::test]
    async fn test_archive_service_submit_archive_invalid_url() {
        let state = create_test_state();
        let service = ArchiveServiceServer {
            inner: ArchiveServiceImpl::new(state),
        };

        let request = Request::new(SubmitArchiveRequest {
            url: "".to_string(),
            metadata: HashMap::new(),
        });

        let response = service.submit_archive(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_archive_service_get_archive() {
        let state = create_test_state();
        let service = ArchiveServiceServer {
            inner: ArchiveServiceImpl::new(state),
        };

        let request = Request::new(GetArchiveRequest {
            archive_id: "arc_123456".to_string(),
        });

        let response = service.get_archive(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.archive.is_some());
        assert_eq!(response.archive.unwrap().id, "arc_123456");
    }

    #[tokio::test]
    async fn test_archive_service_get_archive_not_found() {
        let state = create_test_state();
        let service = ArchiveServiceServer {
            inner: ArchiveServiceImpl::new(state),
        };

        let request = Request::new(GetArchiveRequest {
            archive_id: "invalid_id".to_string(),
        });

        let response = service.get_archive(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::NotFound);
    }

    #[tokio::test]
    async fn test_network_service_get_stats() {
        let state = create_test_state();
        let service = NetworkServiceServer {
            inner: NetworkServiceImpl::new(state),
        };

        let request = Request::new(GetNetworkStatsRequest {});

        let response = service.get_network_stats(request).await;
        assert!(response.is_ok());

        let stats = response.unwrap().into_inner();
        assert_eq!(stats.total_nodes, 100);
        assert_eq!(stats.active_nodes, 95);
    }

    #[tokio::test]
    async fn test_sync_service_get_block() {
        let state = create_test_state();
        let service = SyncServiceServer {
            inner: SyncServiceImpl::new(state),
        };

        let request = Request::new(GetBlockRequest {
            block_hash: "0x123456".to_string(),
        });

        let response = service.get_block(request).await;
        assert!(response.is_ok());

        let response = response.unwrap().into_inner();
        assert!(response.block.is_none()); // Pas encore implémenté
    }

    #[tokio::test]
    async fn test_sync_service_get_block_range_invalid() {
        let state = create_test_state();
        let service = SyncServiceServer {
            inner: SyncServiceImpl::new(state),
        };

        let request = Request::new(GetBlockRangeRequest {
            start_height: 100,
            end_height: 50, // end < start
        });

        let response = service.get_block_range(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
    }

    #[tokio::test]
    async fn test_sync_service_get_block_range_too_large() {
        let state = create_test_state();
        let service = SyncServiceServer {
            inner: SyncServiceImpl::new(state),
        };

        let request = Request::new(GetBlockRangeRequest {
            start_height: 1,
            end_height: 2000, // Range > 1000
        });

        let response = service.get_block_range(request).await;
        assert!(response.is_err());
        assert_eq!(response.unwrap_err().code(), tonic::Code::InvalidArgument);
    }
}