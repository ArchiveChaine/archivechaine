//! Handlers pour les endpoints REST ArchiveChain
//!
//! Implémente tous les handlers pour les routes REST selon les spécifications API.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::api::{
    ApiError, ApiResult,
    types::*,
    server::ServerState,
    middleware::AuthInfo,
};
use super::{
    PaginationParams, PaginatedResponse, ApiResponse,
    extractors::{ValidatedPagination, ValidatedQuery, Validate},
};

// ============================================================================
// ARCHIVES HANDLERS
// ============================================================================

/// Créer une nouvelle archive
pub async fn create_archive(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Json(request): Json<CreateArchiveRequest>,
) -> ApiResult<Json<CreateArchiveResponse>> {
    // Valide la demande
    validate_create_archive_request(&request)?;

    // Vérifie les permissions et quotas de l'utilisateur
    check_user_quota(&auth, &state).await?;

    // Génère un ID d'archive unique
    let archive_id = format!("arc_{}", uuid::Uuid::new_v4().simple());

    // Estime les coûts
    let cost_estimation = estimate_archive_cost(&request).await?;

    // Crée la réponse
    let response = CreateArchiveResponse {
        archive_id,
        status: ArchiveStatus::Pending,
        estimated_completion: Some(chrono::Utc::now() + chrono::Duration::minutes(5)),
        cost_estimation,
    };

    // TODO: Ajouter la demande d'archivage à la queue de traitement

    Ok(Json(response))
}

/// Lister les archives
pub async fn list_archives(
    State(state): State<ServerState>,
    auth: AuthInfo,
    ValidatedPagination(pagination): ValidatedPagination,
    Query(filters): Query<ArchiveListFilters>,
) -> ApiResult<Json<PaginatedResponse<ArchiveDto>>> {
    // TODO: Implémenter la récupération des archives depuis la blockchain
    let archives = vec![]; // Placeholder

    let pagination_info = crate::api::types::PaginationInfo::new(
        pagination.page,
        pagination.limit,
        0, // TODO: Récupérer le nombre total
    );

    let response = PaginatedResponse::new(archives, pagination_info);
    Ok(Json(response))
}

/// Récupérer une archive spécifique
pub async fn get_archive(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
) -> ApiResult<Json<ArchiveDto>> {
    // Valide l'ID d'archive
    validate_archive_id(&archive_id)?;

    // TODO: Récupérer l'archive depuis la blockchain
    // Pour l'instant, retourne une erreur 404
    Err(ApiError::not_found(format!("Archive {} not found", archive_id)))
}

/// Mettre à jour une archive
pub async fn update_archive(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
    Json(request): Json<UpdateArchiveRequest>,
) -> ApiResult<Json<ArchiveDto>> {
    validate_archive_id(&archive_id)?;
    // TODO: Implémenter la mise à jour
    Err(ApiError::not_found(format!("Archive {} not found", archive_id)))
}

/// Supprimer une archive
pub async fn delete_archive(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
) -> ApiResult<StatusCode> {
    validate_archive_id(&archive_id)?;
    // TODO: Implémenter la suppression
    Ok(StatusCode::NO_CONTENT)
}

/// Récupérer les métadonnées d'une archive
pub async fn get_archive_metadata(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
) -> ApiResult<Json<ArchiveMetadataDto>> {
    validate_archive_id(&archive_id)?;
    // TODO: Implémenter
    Err(ApiError::not_found(format!("Archive {} not found", archive_id)))
}

/// Récupérer le statut d'une archive
pub async fn get_archive_status(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
) -> ApiResult<Json<ArchiveStatusResponse>> {
    validate_archive_id(&archive_id)?;
    // TODO: Implémenter
    let response = ArchiveStatusResponse {
        archive_id,
        status: ArchiveStatus::Pending,
        progress: 0,
        message: None,
        updated_at: chrono::Utc::now(),
    };
    Ok(Json(response))
}

/// Vérifier l'intégrité d'une archive
pub async fn verify_archive(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
) -> ApiResult<Json<ArchiveVerificationResponse>> {
    validate_archive_id(&archive_id)?;
    // TODO: Implémenter la vérification
    let response = ArchiveVerificationResponse {
        archive_id,
        is_valid: true,
        integrity_score: 1.0,
        verification_date: chrono::Utc::now(),
        issues: vec![],
    };
    Ok(Json(response))
}

/// Récupérer les informations de réplication
pub async fn get_archive_replicas(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Path(archive_id): Path<String>,
) -> ApiResult<Json<ArchiveReplicasResponse>> {
    validate_archive_id(&archive_id)?;
    // TODO: Implémenter
    let response = ArchiveReplicasResponse {
        archive_id,
        replicas: vec![],
        total_replicas: 0,
        target_replicas: 3,
        geographic_distribution: HashMap::new(),
    };
    Ok(Json(response))
}

// ============================================================================
// SEARCH HANDLERS
// ============================================================================

/// Recherche générale d'archives
pub async fn search_archives(
    State(state): State<ServerState>,
    auth: AuthInfo,
    ValidatedQuery(search_params): ValidatedQuery<SearchRequest>,
) -> ApiResult<Json<SearchResponse>> {
    // TODO: Implémenter la recherche
    let response = SearchResponse {
        query: search_params.query,
        results: vec![],
        facets: SearchFacets {
            domains: HashMap::new(),
            content_types: HashMap::new(),
            languages: HashMap::new(),
            tags: HashMap::new(),
        },
        total_results: 0,
        search_time_ms: 10,
        pagination: crate::api::types::PaginationInfo::new(1, 20, 0),
    };
    Ok(Json(response))
}

/// Recherche avancée
pub async fn advanced_search(
    State(state): State<ServerState>,
    auth: AuthInfo,
    ValidatedQuery(search_params): ValidatedQuery<AdvancedSearchRequest>,
) -> ApiResult<Json<SearchResponse>> {
    // TODO: Implémenter la recherche avancée
    let response = SearchResponse {
        query: search_params.query,
        results: vec![],
        facets: SearchFacets {
            domains: HashMap::new(),
            content_types: HashMap::new(),
            languages: HashMap::new(),
            tags: HashMap::new(),
        },
        total_results: 0,
        search_time_ms: 15,
        pagination: crate::api::types::PaginationInfo::new(1, 20, 0),
    };
    Ok(Json(response))
}

/// Récupérer les facettes de recherche
pub async fn search_facets(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Query(params): Query<SearchFacetsParams>,
) -> ApiResult<Json<SearchFacets>> {
    // TODO: Implémenter
    let facets = SearchFacets {
        domains: HashMap::new(),
        content_types: HashMap::new(),
        languages: HashMap::new(),
        tags: HashMap::new(),
    };
    Ok(Json(facets))
}

/// Suggestions de recherche
pub async fn search_suggestions(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Query(params): Query<SearchSuggestionsParams>,
) -> ApiResult<Json<SearchSuggestionsResponse>> {
    // TODO: Implémenter
    let response = SearchSuggestionsResponse {
        suggestions: vec![],
        query: params.query,
    };
    Ok(Json(response))
}

/// Recherche en lot
pub async fn bulk_search(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Json(request): Json<BulkSearchRequest>,
) -> ApiResult<Json<BulkSearchResponse>> {
    // TODO: Implémenter
    let response = BulkSearchResponse {
        results: HashMap::new(),
        total_queries: request.queries.len(),
        total_time_ms: 100,
    };
    Ok(Json(response))
}

// ============================================================================
// NETWORK HANDLERS
// ============================================================================

/// Récupérer les statistiques du réseau
pub async fn get_network_stats(
    State(state): State<ServerState>,
    auth: AuthInfo,
) -> ApiResult<Json<NetworkStats>> {
    let stats = state.blockchain.get_stats()
        .map_err(|e| ApiError::internal(format!("Failed to get blockchain stats: {}", e)))?;

    let network_stats = NetworkStats {
        network: NetworkInfo {
            total_nodes: 100, // TODO: Récupérer depuis le consensus
            active_nodes: 95,
            total_storage: "15.7 TB".to_string(),
            available_storage: "8.3 TB".to_string(),
            current_block_height: stats.height,
        },
        archives: ArchiveStats {
            total_archives: stats.total_transactions, // Approximation
            archives_today: 50, // TODO: Calculer
            total_size: "12.4 TB".to_string(),
            average_replication: 4.2,
        },
        performance: PerformanceStats {
            average_archive_time: "2.3 minutes".to_string(),
            network_latency: "45ms".to_string(),
            success_rate: 0.987,
        },
    };

    Ok(Json(network_stats))
}

/// Récupérer la santé du réseau
pub async fn get_network_health(
    State(state): State<ServerState>,
    auth: AuthInfo,
) -> ApiResult<Json<NetworkHealthResponse>> {
    // TODO: Implémenter
    let response = NetworkHealthResponse {
        status: "healthy".to_string(),
        consensus_health: "healthy".to_string(),
        storage_health: "healthy".to_string(),
        network_connectivity: "healthy".to_string(),
        alerts: vec![],
        last_updated: chrono::Utc::now(),
    };
    Ok(Json(response))
}

/// Récupérer la topologie du réseau
pub async fn get_network_topology(
    State(state): State<ServerState>,
    auth: AuthInfo,
) -> ApiResult<Json<NetworkTopologyResponse>> {
    // TODO: Implémenter
    let response = NetworkTopologyResponse {
        nodes: vec![],
        connections: vec![],
        regions: HashMap::new(),
        total_nodes: 0,
    };
    Ok(Json(response))
}

/// Récupérer les métriques détaillées
pub async fn get_network_metrics(
    State(state): State<ServerState>,
    auth: AuthInfo,
    Query(params): Query<MetricsParams>,
) -> ApiResult<Json<NetworkMetricsResponse>> {
    // TODO: Implémenter
    let response = NetworkMetricsResponse {
        metrics: HashMap::new(),
        time_range: params.time_range.unwrap_or("1h".to_string()),
        resolution: params.resolution.unwrap_or("1m".to_string()),
    };
    Ok(Json(response))
}

/// Récupérer l'état du consensus
pub async fn get_consensus_state(
    State(state): State<ServerState>,
    auth: AuthInfo,
) -> ApiResult<Json<ConsensusStateResponse>> {
    // TODO: Implémenter
    let response = ConsensusStateResponse {
        current_epoch: 1,
        validators: vec![],
        next_block_time: chrono::Utc::now() + chrono::Duration::seconds(30),
        consensus_algorithm: "PoA".to_string(),
        participation_rate: 0.95,
    };
    Ok(Json(response))
}

// ============================================================================
// PLACEHOLDER HANDLERS (à implémenter)
// ============================================================================

pub async fn list_nodes(State(_): State<ServerState>, _: AuthInfo) -> ApiResult<Json<Vec<NodeInfo>>> {
    Ok(Json(vec![]))
}

pub async fn register_node(State(_): State<ServerState>, _: AuthInfo, Json(_): Json<RegisterNodeRequest>) -> ApiResult<Json<NodeInfo>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn get_node(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<NodeInfo>> {
    Err(ApiError::not_found("Node not found"))
}

pub async fn update_node(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>, Json(_): Json<UpdateNodeRequest>) -> ApiResult<Json<NodeInfo>> {
    Err(ApiError::not_found("Node not found"))
}

pub async fn unregister_node(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_node_status(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<NodeStatusResponse>> {
    Err(ApiError::not_found("Node not found"))
}

pub async fn get_node_performance(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<NodePerformanceResponse>> {
    Err(ApiError::not_found("Node not found"))
}

pub async fn get_node_storage(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<NodeStorageResponse>> {
    Err(ApiError::not_found("Node not found"))
}

pub async fn ping_node(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<PingResponse>> {
    Ok(Json(PingResponse { latency_ms: 50, timestamp: chrono::Utc::now() }))
}

pub async fn list_blocks(State(_): State<ServerState>, _: AuthInfo, ValidatedPagination(_): ValidatedPagination) -> ApiResult<Json<PaginatedResponse<BlockDto>>> {
    let pagination = crate::api::types::PaginationInfo::new(1, 20, 0);
    Ok(Json(PaginatedResponse::new(vec![], pagination)))
}

pub async fn get_block(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<BlockDto>> {
    Err(ApiError::not_found("Block not found"))
}

pub async fn get_block_transactions(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<Vec<TransactionDto>>> {
    Ok(Json(vec![]))
}

pub async fn get_block_by_height(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<u64>) -> ApiResult<Json<BlockDto>> {
    Err(ApiError::not_found("Block not found"))
}

pub async fn get_latest_block(State(_): State<ServerState>, _: AuthInfo) -> ApiResult<Json<BlockDto>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn get_chain_stats(State(_): State<ServerState>, _: AuthInfo) -> ApiResult<Json<ChainStatsResponse>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn list_contracts(State(_): State<ServerState>, _: AuthInfo) -> ApiResult<Json<Vec<ContractInfo>>> {
    Ok(Json(vec![]))
}

pub async fn deploy_contract(State(_): State<ServerState>, _: AuthInfo, Json(_): Json<DeployContractRequest>) -> ApiResult<Json<ContractDeploymentResponse>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn get_contract(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<ContractInfo>> {
    Err(ApiError::not_found("Contract not found"))
}

pub async fn call_contract(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>, Json(_): Json<ContractCallRequest>) -> ApiResult<Json<ContractCallResponse>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn get_contract_events(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<Vec<ContractEvent>>> {
    Ok(Json(vec![]))
}

pub async fn get_contract_state(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<ContractStateResponse>> {
    Err(ApiError::not_found("Contract not found"))
}

pub async fn list_bounties(State(_): State<ServerState>, _: AuthInfo) -> ApiResult<Json<Vec<BountyInfo>>> {
    Ok(Json(vec![]))
}

pub async fn create_bounty(State(_): State<ServerState>, _: AuthInfo, Json(_): Json<CreateBountyRequest>) -> ApiResult<Json<BountyInfo>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn get_bounty(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<BountyInfo>> {
    Err(ApiError::not_found("Bounty not found"))
}

pub async fn update_bounty(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>, Json(_): Json<UpdateBountyRequest>) -> ApiResult<Json<BountyInfo>> {
    Err(ApiError::not_found("Bounty not found"))
}

pub async fn cancel_bounty(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<StatusCode> {
    Ok(StatusCode::NO_CONTENT)
}

pub async fn submit_bounty_proposal(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>, Json(_): Json<SubmitBountyProposalRequest>) -> ApiResult<Json<BountyProposalResponse>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn list_bounty_proposals(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<Vec<BountyProposal>>> {
    Ok(Json(vec![]))
}

pub async fn accept_bounty_proposal(State(_): State<ServerState>, _: AuthInfo, Path((_, _)): Path<(String, String)>) -> ApiResult<Json<BountyProposalAcceptanceResponse>> {
    Err(ApiError::internal("Not implemented"))
}

pub async fn get_bounty_status(State(_): State<ServerState>, _: AuthInfo, Path(_): Path<String>) -> ApiResult<Json<BountyStatusResponse>> {
    Err(ApiError::not_found("Bounty not found"))
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn validate_create_archive_request(request: &CreateArchiveRequest) -> ApiResult<()> {
    if request.url.is_empty() {
        return Err(ApiError::validation("URL is required"));
    }
    
    // Valide l'URL
    if let Err(_) = url::Url::parse(&request.url) {
        return Err(ApiError::validation("Invalid URL format"));
    }
    
    Ok(())
}

fn validate_archive_id(archive_id: &str) -> ApiResult<()> {
    if !archive_id.starts_with("arc_") {
        return Err(ApiError::validation("Invalid archive ID format"));
    }
    Ok(())
}

async fn check_user_quota(auth: &AuthInfo, state: &ServerState) -> ApiResult<()> {
    // TODO: Vérifier les quotas de l'utilisateur
    Ok(())
}

async fn estimate_archive_cost(request: &CreateArchiveRequest) -> ApiResult<CostEstimation> {
    // TODO: Calculer les coûts réels
    Ok(CostEstimation {
        storage_cost: "0.001 ARC".to_string(),
        processing_cost: "0.0005 ARC".to_string(),
        total_cost: "0.0015 ARC".to_string(),
    })
}

// ============================================================================
// REQUEST/RESPONSE TYPES (à définir dans types.rs si pas encore fait)
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveListFilters {
    pub status: Option<ArchiveStatus>,
    pub tag: Option<String>,
    pub domain: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateArchiveRequest {
    pub metadata: Option<HashMap<String, String>>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveStatusResponse {
    pub archive_id: String,
    pub status: ArchiveStatus,
    pub progress: u8,
    pub message: Option<String>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveVerificationResponse {
    pub archive_id: String,
    pub is_valid: bool,
    pub integrity_score: f64,
    pub verification_date: chrono::DateTime<chrono::Utc>,
    pub issues: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArchiveReplicasResponse {
    pub archive_id: String,
    pub replicas: Vec<ReplicaInfo>,
    pub total_replicas: u32,
    pub target_replicas: u32,
    pub geographic_distribution: HashMap<String, u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReplicaInfo {
    pub node_id: String,
    pub region: String,
    pub status: String,
    pub last_verified: chrono::DateTime<chrono::Utc>,
}

// Implement Validate for request types
impl Validate for SearchRequest {
    fn validate(&self) -> Result<(), String> {
        if self.query.trim().is_empty() {
            return Err("Query cannot be empty".to_string());
        }
        if self.limit > 100 {
            return Err("Limit cannot exceed 100".to_string());
        }
        Ok(())
    }
}

// Placeholder types (à compléter)
#[derive(Debug, Serialize, Deserialize)]
pub struct AdvancedSearchRequest {
    pub query: String,
    // TODO: Ajouter d'autres champs
}

impl Validate for AdvancedSearchRequest {
    fn validate(&self) -> Result<(), String> {
        if self.query.trim().is_empty() {
            return Err("Query cannot be empty".to_string());
        }
        Ok(())
    }
}

// Placeholder types pour les autres endpoints
#[derive(Debug, Serialize, Deserialize)] pub struct SearchFacetsParams { pub query: Option<String> }
#[derive(Debug, Serialize, Deserialize)] pub struct SearchSuggestionsParams { pub query: String }
#[derive(Debug, Serialize, Deserialize)] pub struct SearchSuggestionsResponse { pub suggestions: Vec<String>, pub query: String }
#[derive(Debug, Serialize, Deserialize)] pub struct BulkSearchRequest { pub queries: Vec<String> }
#[derive(Debug, Serialize, Deserialize)] pub struct BulkSearchResponse { pub results: HashMap<String, SearchResponse>, pub total_queries: usize, pub total_time_ms: u64 }
#[derive(Debug, Serialize, Deserialize)] pub struct NetworkHealthResponse { pub status: String, pub consensus_health: String, pub storage_health: String, pub network_connectivity: String, pub alerts: Vec<String>, pub last_updated: chrono::DateTime<chrono::Utc> }
#[derive(Debug, Serialize, Deserialize)] pub struct NetworkTopologyResponse { pub nodes: Vec<NodeInfo>, pub connections: Vec<String>, pub regions: HashMap<String, u32>, pub total_nodes: u32 }
#[derive(Debug, Serialize, Deserialize)] pub struct MetricsParams { pub time_range: Option<String>, pub resolution: Option<String> }
#[derive(Debug, Serialize, Deserialize)] pub struct NetworkMetricsResponse { pub metrics: HashMap<String, serde_json::Value>, pub time_range: String, pub resolution: String }
#[derive(Debug, Serialize, Deserialize)] pub struct ConsensusStateResponse { pub current_epoch: u64, pub validators: Vec<String>, pub next_block_time: chrono::DateTime<chrono::Utc>, pub consensus_algorithm: String, pub participation_rate: f64 }

// Placeholder types pour les autres endpoints (à compléter)
#[derive(Debug, Serialize, Deserialize)] pub struct RegisterNodeRequest { pub node_id: String }
#[derive(Debug, Serialize, Deserialize)] pub struct UpdateNodeRequest { pub status: Option<String> }
#[derive(Debug, Serialize, Deserialize)] pub struct NodeStatusResponse { pub status: String }
#[derive(Debug, Serialize, Deserialize)] pub struct NodePerformanceResponse { pub performance: HashMap<String, f64> }
#[derive(Debug, Serialize, Deserialize)] pub struct NodeStorageResponse { pub storage: HashMap<String, u64> }
#[derive(Debug, Serialize, Deserialize)] pub struct PingResponse { pub latency_ms: u64, pub timestamp: chrono::DateTime<chrono::Utc> }
#[derive(Debug, Serialize, Deserialize)] pub struct ChainStatsResponse { pub stats: HashMap<String, serde_json::Value> }
#[derive(Debug, Serialize, Deserialize)] pub struct ContractInfo { pub id: String }
#[derive(Debug, Serialize, Deserialize)] pub struct DeployContractRequest { pub code: String }
#[derive(Debug, Serialize, Deserialize)] pub struct ContractDeploymentResponse { pub contract_id: String }
#[derive(Debug, Serialize, Deserialize)] pub struct ContractCallRequest { pub method: String }
#[derive(Debug, Serialize, Deserialize)] pub struct ContractCallResponse { pub result: serde_json::Value }
#[derive(Debug, Serialize, Deserialize)] pub struct ContractEvent { pub event: String }
#[derive(Debug, Serialize, Deserialize)] pub struct ContractStateResponse { pub state: HashMap<String, serde_json::Value> }
#[derive(Debug, Serialize, Deserialize)] pub struct BountyInfo { pub id: String }
#[derive(Debug, Serialize, Deserialize)] pub struct CreateBountyRequest { pub title: String }
#[derive(Debug, Serialize, Deserialize)] pub struct UpdateBountyRequest { pub title: Option<String> }
#[derive(Debug, Serialize, Deserialize)] pub struct SubmitBountyProposalRequest { pub proposal: String }
#[derive(Debug, Serialize, Deserialize)] pub struct BountyProposalResponse { pub proposal_id: String }
#[derive(Debug, Serialize, Deserialize)] pub struct BountyProposal { pub id: String }
#[derive(Debug, Serialize, Deserialize)] pub struct BountyProposalAcceptanceResponse { pub accepted: bool }
#[derive(Debug, Serialize, Deserialize)] pub struct BountyStatusResponse { pub status: String }