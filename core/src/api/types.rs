//! Types de données et DTOs pour l'API ArchiveChain
//!
//! Ce module contient toutes les structures de données utilisées par les différentes APIs,
//! avec leurs conversions vers/depuis les types core d'ArchiveChain.

use crate::{Hash, Block, Transaction, ArchiveMetadata};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Statut d'une archive
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ArchiveStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
}

impl Default for ArchiveStatus {
    fn default() -> Self {
        Self::Pending
    }
}

/// Options de création d'archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveOptions {
    #[serde(default)]
    pub include_assets: bool,
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
    #[serde(default)]
    pub preserve_javascript: bool,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

fn default_max_depth() -> u32 {
    3
}

impl Default for ArchiveOptions {
    fn default() -> Self {
        Self {
            include_assets: true,
            max_depth: default_max_depth(),
            preserve_javascript: false,
            allowed_domains: Vec::new(),
            timeout_seconds: Some(300), // 5 minutes
        }
    }
}

/// Demande de création d'archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArchiveRequest {
    pub url: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    #[serde(default)]
    pub options: ArchiveOptions,
}

/// Réponse de création d'archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArchiveResponse {
    pub archive_id: String,
    pub status: ArchiveStatus,
    pub estimated_completion: Option<chrono::DateTime<chrono::Utc>>,
    pub cost_estimation: CostEstimation,
}

/// Estimation des coûts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimation {
    pub storage_cost: String,
    pub processing_cost: String,
    pub total_cost: String,
}

/// Informations de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    pub replicas: u32,
    pub locations: Vec<String>,
    pub integrity_score: f64,
    pub last_verified: chrono::DateTime<chrono::Utc>,
}

/// URLs d'accès à l'archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessUrls {
    pub view: String,
    pub download: String,
    pub raw: String,
}

/// Archive complète (DTO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveDto {
    pub archive_id: String,
    pub url: String,
    pub status: ArchiveStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub size: u64,
    pub metadata: ArchiveMetadataDto,
    pub storage_info: StorageInfo,
    pub access_urls: AccessUrls,
}

/// Métadonnées d'archive (DTO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadataDto {
    pub title: Option<String>,
    pub description: Option<String>,
    pub mime_type: String,
    pub language: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
    pub tags: Vec<String>,
}

/// Demande de recherche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    #[serde(default)]
    pub filters: SearchFilters,
    #[serde(default = "default_search_limit")]
    pub limit: u32,
    pub offset: Option<u64>,
}

fn default_search_limit() -> u32 {
    20
}

/// Filtres de recherche
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchFilters {
    pub content_type: Option<String>,
    pub domain: Option<String>,
    pub date_range: Option<DateRange>,
    pub tags: Vec<String>,
    pub size_range: Option<SizeRange>,
    pub language: Option<String>,
}

/// Plage de dates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Plage de tailles
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeRange {
    pub min: u64,
    pub max: u64,
}

/// Résultat de recherche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub archive_id: String,
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
    pub relevance_score: f64,
    pub archived_at: chrono::DateTime<chrono::Utc>,
    pub size: u64,
    pub content_type: String,
}

/// Réponse de recherche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub results: Vec<SearchResult>,
    pub facets: SearchFacets,
    pub total_results: u64,
    pub search_time_ms: u64,
    pub pagination: PaginationInfo,
}

/// Facettes de recherche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchFacets {
    pub domains: HashMap<String, u64>,
    pub content_types: HashMap<String, u64>,
    pub languages: HashMap<String, u64>,
    pub tags: HashMap<String, u64>,
}

/// Informations de pagination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaginationInfo {
    pub page: u32,
    pub limit: u32,
    pub total: u64,
    pub has_next: bool,
    pub has_prev: bool,
}

/// Statistiques du réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub network: NetworkInfo,
    pub archives: ArchiveStats,
    pub performance: PerformanceStats,
}

/// Informations du réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInfo {
    pub total_nodes: u64,
    pub active_nodes: u64,
    pub total_storage: String,
    pub available_storage: String,
    pub current_block_height: u64,
}

/// Statistiques des archives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveStats {
    pub total_archives: u64,
    pub archives_today: u64,
    pub total_size: String,
    pub average_replication: f64,
}

/// Statistiques de performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceStats {
    pub average_archive_time: String,
    pub network_latency: String,
    pub success_rate: f64,
}

/// Informations de nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub node_id: String,
    pub status: NodeStatus,
    pub region: String,
    pub capacity: StorageCapacity,
    pub performance: NodePerformance,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Statut de nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeStatus {
    Active,
    Inactive,
    Syncing,
    Maintenance,
}

/// Capacité de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageCapacity {
    pub total: u64,
    pub used: u64,
    pub available: u64,
}

/// Performance de nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodePerformance {
    pub bandwidth: u64,
    pub latency: u32,
    pub reliability_score: f64,
}

/// Bloc (DTO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockDto {
    pub height: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub transactions: Vec<TransactionDto>,
    pub archive_count: u32,
    pub validator: String,
}

/// Transaction (DTO)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionDto {
    pub hash: String,
    pub transaction_type: TransactionType,
    pub sender: String,
    pub recipient: Option<String>,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: Option<serde_json::Value>,
}

/// Type de transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionType {
    Archive,
    Transfer,
    ContractCall,
    ContractDeploy,
    Stake,
    Unstake,
    Vote,
}

/// Conversions vers/depuis les types core

impl From<&ArchiveMetadata> for ArchiveMetadataDto {
    fn from(metadata: &ArchiveMetadata) -> Self {
        Self {
            title: metadata.title.clone(),
            description: metadata.description.clone(),
            mime_type: metadata.content_type.clone(),
            language: metadata.language.clone(),
            author: metadata.author.clone(),
            published_at: metadata.published_at,
            tags: metadata.tags.clone(),
        }
    }
}

impl From<&Block> for BlockDto {
    fn from(block: &Block) -> Self {
        Self {
            height: block.header().height,
            hash: block.hash().to_string(),
            previous_hash: block.header().previous_hash.to_string(),
            timestamp: block.header().timestamp,
            transactions: block.body().transactions.iter().map(TransactionDto::from).collect(),
            archive_count: block.body().archive_metadata.len() as u32,
            validator: block.header().validator.to_string(),
        }
    }
}

impl From<&Transaction> for TransactionDto {
    fn from(tx: &Transaction) -> Self {
        Self {
            hash: tx.hash().to_string(),
            transaction_type: TransactionType::from(&tx.transaction_type()),
            sender: tx.from().to_string(),
            recipient: tx.to().map(|addr| addr.to_string()),
            amount: tx.amount(),
            fee: tx.fee(),
            timestamp: tx.timestamp(),
            data: tx.data().map(|data| serde_json::to_value(data).unwrap_or(serde_json::Value::Null)),
        }
    }
}

impl From<&crate::transaction::TransactionType> for TransactionType {
    fn from(tx_type: &crate::transaction::TransactionType) -> Self {
        match tx_type {
            crate::transaction::TransactionType::Archive => Self::Archive,
            crate::transaction::TransactionType::Transfer => Self::Transfer,
            crate::transaction::TransactionType::ContractCall => Self::ContractCall,
            crate::transaction::TransactionType::ContractDeploy => Self::ContractDeploy,
            crate::transaction::TransactionType::Stake => Self::Stake,
            crate::transaction::TransactionType::Unstake => Self::Unstake,
            crate::transaction::TransactionType::Vote => Self::Vote,
        }
    }
}

/// Helper pour créer des URLs d'accès
impl AccessUrls {
    pub fn new(base_url: &str, archive_id: &str) -> Self {
        Self {
            view: format!("{}/view/{}", base_url, archive_id),
            download: format!("{}/download/{}", base_url, archive_id),
            raw: format!("{}/raw/{}", base_url, archive_id),
        }
    }
}

/// Helper pour la pagination
impl PaginationInfo {
    pub fn new(page: u32, limit: u32, total: u64) -> Self {
        let has_next = (page * limit) < total as u32;
        let has_prev = page > 1;
        
        Self {
            page,
            limit,
            total,
            has_next,
            has_prev,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_options_default() {
        let options = ArchiveOptions::default();
        assert!(options.include_assets);
        assert_eq!(options.max_depth, 3);
        assert!(!options.preserve_javascript);
        assert_eq!(options.timeout_seconds, Some(300));
    }

    #[test]
    fn test_archive_status_serialization() {
        let status = ArchiveStatus::Completed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, "\"completed\"");
        
        let deserialized: ArchiveStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, ArchiveStatus::Completed);
    }

    #[test]
    fn test_search_request_default() {
        let request = SearchRequest {
            query: "test".to_string(),
            filters: SearchFilters::default(),
            limit: default_search_limit(),
            offset: None,
        };
        
        assert_eq!(request.limit, 20);
        assert!(request.filters.tags.is_empty());
    }

    #[test]
    fn test_access_urls_creation() {
        let urls = AccessUrls::new("https://gateway.archivechain.org", "arc_123");
        assert_eq!(urls.view, "https://gateway.archivechain.org/view/arc_123");
        assert_eq!(urls.download, "https://gateway.archivechain.org/download/arc_123");
        assert_eq!(urls.raw, "https://gateway.archivechain.org/raw/arc_123");
    }

    #[test]
    fn test_pagination_info() {
        let pagination = PaginationInfo::new(2, 10, 100);
        assert_eq!(pagination.page, 2);
        assert_eq!(pagination.limit, 10);
        assert_eq!(pagination.total, 100);
        assert!(pagination.has_next);
        assert!(pagination.has_prev);
        
        let first_page = PaginationInfo::new(1, 10, 100);
        assert!(!first_page.has_prev);
        assert!(first_page.has_next);
        
        let last_page = PaginationInfo::new(10, 10, 100);
        assert!(last_page.has_prev);
        assert!(!last_page.has_next);
    }

    #[test]
    fn test_cost_estimation() {
        let cost = CostEstimation {
            storage_cost: "0.001 ARC".to_string(),
            processing_cost: "0.0005 ARC".to_string(),
            total_cost: "0.0015 ARC".to_string(),
        };
        
        let json = serde_json::to_string(&cost).unwrap();
        let deserialized: CostEstimation = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.storage_cost, cost.storage_cost);
    }
}