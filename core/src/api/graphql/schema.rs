//! Schéma GraphQL pour ArchiveChain
//!
//! Définit le schéma GraphQL complet avec tous les types, queries, mutations et subscriptions.

use async_graphql::{Object, Schema, Subscription, Union, Enum, InputObject, SimpleObject};
use futures_util::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::pin::Pin;

use crate::api::{
    server::ServerState,
    middleware::AuthInfo,
    auth::ApiScope,
    types::*,
};
use super::{GraphQLContext, resolvers::*};

/// Root Query pour l'API GraphQL
pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Récupère une archive par son ID
    async fn archive(&self, ctx: &async_graphql::Context<'_>, id: String) -> async_graphql::Result<Option<Archive>> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesRead)?;
        
        ArchiveResolver::get_archive(id).await
    }

    /// Liste les archives avec filtres et pagination
    async fn archives(
        &self,
        ctx: &async_graphql::Context<'_>,
        filter: Option<ArchiveFilter>,
        sort: Option<ArchiveSort>,
        first: Option<i32>,
        after: Option<String>,
    ) -> async_graphql::Result<ArchiveConnection> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesRead)?;
        
        ArchiveResolver::list_archives(filter, sort, first, after).await
    }

    /// Recherche d'archives
    async fn search_archives(
        &self,
        ctx: &async_graphql::Context<'_>,
        query: String,
        filters: Option<SearchFilters>,
        first: Option<i32>,
        after: Option<String>,
    ) -> async_graphql::Result<SearchConnection> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::SearchRead)?;
        
        SearchResolver::search_archives(query, filters, first, after).await
    }

    /// Statistiques du réseau
    async fn network_stats(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<NetworkStats> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::NetworkRead)?;
        
        NetworkResolver::get_network_stats().await
    }

    /// Liste des nœuds
    async fn nodes(&self, ctx: &async_graphql::Context<'_>, status: Option<NodeStatus>) -> async_graphql::Result<Vec<Node>> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::NetworkRead)?;
        
        NodeResolver::list_nodes(status).await
    }

    /// Informations de l'utilisateur connecté
    async fn me(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<User> {
        let context = ctx.data::<GraphQLContext>()?;
        let auth = context.require_auth()?;
        
        UserResolver::get_current_user(&auth.user_id).await
    }

    /// Statistiques d'usage de l'utilisateur
    async fn my_usage(&self, ctx: &async_graphql::Context<'_>) -> async_graphql::Result<UsageStats> {
        let context = ctx.data::<GraphQLContext>()?;
        let auth = context.require_auth()?;
        
        UserResolver::get_usage_stats(&auth.user_id).await
    }

    /// Récupère un bloc par son hash
    async fn block(&self, ctx: &async_graphql::Context<'_>, hash: String) -> async_graphql::Result<Option<Block>> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::NetworkRead)?;
        
        BlockResolver::get_block(hash).await
    }

    /// Liste des blocs récents
    async fn blocks(
        &self,
        ctx: &async_graphql::Context<'_>,
        first: Option<i32>,
        after: Option<String>,
    ) -> async_graphql::Result<BlockConnection> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::NetworkRead)?;
        
        BlockResolver::list_blocks(first, after).await
    }
}

/// Root Mutation pour l'API GraphQL
pub struct MutationRoot;

#[Object]
impl MutationRoot {
    /// Crée une nouvelle archive
    async fn create_archive(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: CreateArchiveInput,
    ) -> async_graphql::Result<CreateArchivePayload> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesWrite)?;
        
        ArchiveResolver::create_archive(input).await
    }

    /// Met à jour une archive
    async fn update_archive(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: String,
        input: UpdateArchiveInput,
    ) -> async_graphql::Result<UpdateArchivePayload> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesWrite)?;
        
        ArchiveResolver::update_archive(id, input).await
    }

    /// Supprime une archive
    async fn delete_archive(
        &self,
        ctx: &async_graphql::Context<'_>,
        id: String,
    ) -> async_graphql::Result<DeleteArchivePayload> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesDelete)?;
        
        ArchiveResolver::delete_archive(id).await
    }

    /// Met à jour le profil utilisateur
    async fn update_profile(
        &self,
        ctx: &async_graphql::Context<'_>,
        input: UpdateProfileInput,
    ) -> async_graphql::Result<UpdateProfilePayload> {
        let context = ctx.data::<GraphQLContext>()?;
        let auth = context.require_auth()?;
        
        UserResolver::update_profile(&auth.user_id, input).await
    }
}

/// Root Subscription pour l'API GraphQL
#[derive(Default)]
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    /// Souscription aux mises à jour d'une archive
    async fn archive_status_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
        archive_id: String,
    ) -> async_graphql::Result<impl Stream<Item = Archive>> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesRead)?;
        
        SubscriptionResolver::archive_updates(archive_id).await
    }

    /// Souscription aux nouvelles archives créées
    async fn new_archive_created(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<impl Stream<Item = Archive>> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::ArchivesRead)?;
        
        SubscriptionResolver::new_archives().await
    }

    /// Souscription aux mises à jour des statistiques réseau
    async fn network_stats_updated(
        &self,
        ctx: &async_graphql::Context<'_>,
    ) -> async_graphql::Result<impl Stream<Item = NetworkStats>> {
        let context = ctx.data::<GraphQLContext>()?;
        context.require_scope(ApiScope::NetworkRead)?;
        
        SubscriptionResolver::network_stats().await
    }
}

// ============================================================================
// TYPES GRAPHQL
// ============================================================================

/// Archive GraphQL
#[derive(SimpleObject)]
pub struct Archive {
    pub id: String,
    pub url: String,
    pub status: ArchiveStatus,
    pub metadata: ArchiveMetadata,
    pub storage_info: StorageInfo,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub size: i64,
    pub cost: TokenAmount,
}

/// Statut d'archive
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ArchiveStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Expired,
}

/// Métadonnées d'archive
#[derive(SimpleObject)]
pub struct ArchiveMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub content_type: String,
    pub language: Option<String>,
    pub author: Option<String>,
    pub published_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Informations de stockage
#[derive(SimpleObject)]
pub struct StorageInfo {
    pub replicas: i32,
    pub locations: Vec<String>,
    pub integrity_score: f64,
    pub last_verified: chrono::DateTime<chrono::Utc>,
}

/// Montant de token
#[derive(SimpleObject)]
pub struct TokenAmount {
    pub amount: String,
    pub currency: String,
}

/// Connexion paginée pour les archives
#[derive(SimpleObject)]
pub struct ArchiveConnection {
    pub edges: Vec<ArchiveEdge>,
    pub page_info: PageInfo,
}

/// Edge pour une archive
#[derive(SimpleObject)]
pub struct ArchiveEdge {
    pub node: Archive,
    pub cursor: String,
}

/// Informations de pagination
#[derive(SimpleObject)]
pub struct PageInfo {
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub start_cursor: Option<String>,
    pub end_cursor: Option<String>,
}

/// Filtres pour les archives
#[derive(InputObject)]
pub struct ArchiveFilter {
    pub status: Option<ArchiveStatus>,
    pub tags: Option<Vec<String>>,
    pub content_type: Option<String>,
    pub created_after: Option<chrono::DateTime<chrono::Utc>>,
    pub created_before: Option<chrono::DateTime<chrono::Utc>>,
}

/// Tri pour les archives
#[derive(InputObject)]
pub struct ArchiveSort {
    pub field: ArchiveSortField,
    pub direction: SortDirection,
}

/// Champs de tri pour les archives
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum ArchiveSortField {
    CreatedAt,
    Size,
    Status,
    Url,
}

/// Direction de tri
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// Entrée pour créer une archive
#[derive(InputObject)]
pub struct CreateArchiveInput {
    pub url: String,
    pub metadata: Option<HashMap<String, String>>,
    pub options: Option<ArchiveOptions>,
}

/// Options d'archivage
#[derive(InputObject)]
pub struct ArchiveOptions {
    pub include_assets: Option<bool>,
    pub max_depth: Option<i32>,
    pub preserve_javascript: Option<bool>,
    pub allowed_domains: Option<Vec<String>>,
}

/// Payload de création d'archive
#[derive(SimpleObject)]
pub struct CreateArchivePayload {
    pub archive: Archive,
    pub errors: Vec<String>,
}

/// Entrée pour mettre à jour une archive
#[derive(InputObject)]
pub struct UpdateArchiveInput {
    pub metadata: Option<HashMap<String, String>>,
    pub tags: Option<Vec<String>>,
}

/// Payload de mise à jour d'archive
#[derive(SimpleObject)]
pub struct UpdateArchivePayload {
    pub archive: Archive,
    pub errors: Vec<String>,
}

/// Payload de suppression d'archive
#[derive(SimpleObject)]
pub struct DeleteArchivePayload {
    pub success: bool,
    pub errors: Vec<String>,
}

/// Connexion de recherche
#[derive(SimpleObject)]
pub struct SearchConnection {
    pub edges: Vec<SearchEdge>,
    pub page_info: PageInfo,
    pub facets: SearchFacets,
    pub total_count: i32,
}

/// Edge de recherche
#[derive(SimpleObject)]
pub struct SearchEdge {
    pub node: SearchResult,
    pub cursor: String,
}

/// Résultat de recherche
#[derive(SimpleObject)]
pub struct SearchResult {
    pub archive: Archive,
    pub relevance_score: f64,
    pub snippet: Option<String>,
}

/// Facettes de recherche
#[derive(SimpleObject)]
pub struct SearchFacets {
    pub domains: Vec<FacetValue>,
    pub content_types: Vec<FacetValue>,
    pub languages: Vec<FacetValue>,
    pub tags: Vec<FacetValue>,
}

/// Valeur de facette
#[derive(SimpleObject)]
pub struct FacetValue {
    pub value: String,
    pub count: i32,
}

/// Filtres de recherche
#[derive(InputObject)]
pub struct SearchFilters {
    pub content_type: Option<String>,
    pub domain: Option<String>,
    pub date_range: Option<DateRangeInput>,
    pub tags: Option<Vec<String>>,
    pub size_range: Option<SizeRangeInput>,
    pub language: Option<String>,
}

/// Plage de dates
#[derive(InputObject)]
pub struct DateRangeInput {
    pub start: chrono::DateTime<chrono::Utc>,
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Plage de tailles
#[derive(InputObject)]
pub struct SizeRangeInput {
    pub min: i64,
    pub max: i64,
}

/// Statistiques du réseau
#[derive(SimpleObject)]
pub struct NetworkStats {
    pub total_nodes: i32,
    pub active_nodes: i32,
    pub total_storage: String,
    pub available_storage: String,
    pub current_block_height: i64,
    pub total_archives: i64,
    pub archives_today: i32,
    pub average_archive_time: String,
    pub success_rate: f64,
}

/// Nœud du réseau
#[derive(SimpleObject)]
pub struct Node {
    pub id: String,
    pub status: NodeStatus,
    pub region: String,
    pub capacity: StorageCapacity,
    pub performance: NodePerformance,
    pub last_seen: chrono::DateTime<chrono::Utc>,
}

/// Statut de nœud
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum NodeStatus {
    Active,
    Inactive,
    Syncing,
    Maintenance,
}

/// Capacité de stockage
#[derive(SimpleObject)]
pub struct StorageCapacity {
    pub total: i64,
    pub used: i64,
    pub available: i64,
}

/// Performance de nœud
#[derive(SimpleObject)]
pub struct NodePerformance {
    pub bandwidth: i64,
    pub latency: i32,
    pub reliability_score: f64,
}

/// Utilisateur
#[derive(SimpleObject)]
pub struct User {
    pub id: String,
    pub public_key: Option<String>,
    pub scopes: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_login: Option<chrono::DateTime<chrono::Utc>>,
    pub is_active: bool,
    pub metadata: HashMap<String, String>,
}

/// Statistiques d'usage
#[derive(SimpleObject)]
pub struct UsageStats {
    pub archives_created: i32,
    pub storage_used: i64,
    pub requests_this_month: i32,
    pub quota_remaining: i32,
}

/// Entrée pour mettre à jour le profil
#[derive(InputObject)]
pub struct UpdateProfileInput {
    pub metadata: Option<HashMap<String, String>>,
}

/// Payload de mise à jour du profil
#[derive(SimpleObject)]
pub struct UpdateProfilePayload {
    pub user: User,
    pub errors: Vec<String>,
}

/// Bloc de la blockchain
#[derive(SimpleObject)]
pub struct Block {
    pub height: i64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub transactions: Vec<Transaction>,
    pub archive_count: i32,
    pub validator: String,
}

/// Transaction
#[derive(SimpleObject)]
pub struct Transaction {
    pub hash: String,
    pub transaction_type: TransactionType,
    pub sender: String,
    pub recipient: Option<String>,
    pub amount: i64,
    pub fee: i64,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Type de transaction
#[derive(Enum, Copy, Clone, Eq, PartialEq)]
pub enum TransactionType {
    Archive,
    Transfer,
    ContractCall,
    ContractDeploy,
    Stake,
    Unstake,
    Vote,
}

/// Connexion de blocs
#[derive(SimpleObject)]
pub struct BlockConnection {
    pub edges: Vec<BlockEdge>,
    pub page_info: PageInfo,
}

/// Edge de bloc
#[derive(SimpleObject)]
pub struct BlockEdge {
    pub node: Block,
    pub cursor: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_archive_status_enum() {
        // Test que les enums se compilent et peuvent être utilisés
        let status = ArchiveStatus::Pending;
        assert_eq!(status, ArchiveStatus::Pending);
    }

    #[test]
    fn test_node_status_enum() {
        let status = NodeStatus::Active;
        assert_eq!(status, NodeStatus::Active);
    }

    #[test]
    fn test_sort_direction_enum() {
        let direction = SortDirection::Asc;
        assert_eq!(direction, SortDirection::Asc);
    }

    #[test]
    fn test_transaction_type_enum() {
        let tx_type = TransactionType::Archive;
        assert_eq!(tx_type, TransactionType::Archive);
    }
}