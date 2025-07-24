//! API GraphQL pour ArchiveChain
//!
//! Implémente une API GraphQL complète avec queries, mutations et subscriptions
//! selon les spécifications API. Utilise async-graphql pour une performance optimale.

pub mod schema;
pub mod resolvers;
pub mod types;
pub mod subscriptions;

use axum::{
    extract::State,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use async_graphql::{
    http::{playground_source, GraphQLPlaygroundConfig},
    Schema, EmptySubscription, ErrorExtensions,
};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse, GraphQLSubscription};
use serde::{Deserialize, Serialize};

use crate::api::{ApiResult, server::ServerState};
use schema::{QueryRoot, MutationRoot, SubscriptionRoot};

// Re-exports
pub use schema::*;
pub use resolvers::*;
pub use types::*;

/// Configuration GraphQL
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQLConfig {
    /// Active le playground GraphQL
    pub enable_playground: bool,
    /// Active l'introspection
    pub enable_introspection: bool,
    /// Profondeur maximum des requêtes
    pub max_depth: u32,
    /// Complexité maximum des requêtes
    pub max_complexity: u32,
    /// Timeout des requêtes (en secondes)
    pub query_timeout: u64,
    /// Active les subscriptions WebSocket
    pub enable_subscriptions: bool,
}

impl Default for GraphQLConfig {
    fn default() -> Self {
        Self {
            enable_playground: true,
            enable_introspection: true,
            max_depth: 15,
            max_complexity: 1000,
            query_timeout: 30,
            enable_subscriptions: true,
        }
    }
}

/// Type de schéma GraphQL
pub type ArchiveChainSchema = Schema<QueryRoot, MutationRoot, SubscriptionRoot>;

/// Crée le schéma GraphQL
pub fn create_schema() -> ArchiveChainSchema {
    Schema::build(QueryRoot, MutationRoot, SubscriptionRoot::default())
        .limit_depth(15)
        .limit_complexity(1000)
        .enable_federation()
        .finish()
}

/// Crée les routes GraphQL
pub async fn create_routes() -> ApiResult<Router<ServerState>> {
    let schema = create_schema();

    let router = Router::new()
        // Endpoint principal GraphQL
        .route("/", axum::routing::post(graphql_handler))
        // Playground GraphQL (en mode développement)
        .route("/playground", get(graphql_playground))
        // Introspection schema
        .route("/schema", get(graphql_schema))
        // WebSocket pour les subscriptions
        .route("/ws", get(graphql_subscription_handler))
        .with_state(schema);

    Ok(router)
}

/// Handler principal GraphQL
async fn graphql_handler(
    State(schema): State<ArchiveChainSchema>,
    State(server_state): State<ServerState>,
    req: GraphQLRequest,
) -> GraphQLResponse {
    let request = req.into_inner().data(server_state);
    schema.execute(request).await.into()
}

/// Handler pour le playground GraphQL
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(
        GraphQLPlaygroundConfig::new("/api/v1/graphql")
            .subscription_endpoint("/api/v1/graphql/ws")
    ))
}

/// Handler pour récupérer le schéma
async fn graphql_schema(
    State(schema): State<ArchiveChainSchema>,
) -> impl IntoResponse {
    schema.sdl()
}

/// Handler pour les subscriptions WebSocket
async fn graphql_subscription_handler(
    State(schema): State<ArchiveChainSchema>,
    State(server_state): State<ServerState>,
    ws: axum::extract::WebSocketUpgrade,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        GraphQLSubscription::new(socket, schema, async_graphql::http::ALL_WEBSOCKET_PROTOCOLS)
            .serve()
    })
}

/// Extensions GraphQL pour le contexte
pub struct GraphQLContext {
    pub server_state: ServerState,
    pub auth_info: Option<crate::api::middleware::AuthInfo>,
}

impl GraphQLContext {
    pub fn new(server_state: ServerState, auth_info: Option<crate::api::middleware::AuthInfo>) -> Self {
        Self {
            server_state,
            auth_info,
        }
    }

    /// Vérifie l'authentification
    pub fn require_auth(&self) -> async_graphql::Result<&crate::api::middleware::AuthInfo> {
        self.auth_info.as_ref().ok_or_else(|| {
            async_graphql::Error::new("Authentication required")
                .extend_with(|_, e| e.set("code", "UNAUTHENTICATED"))
        })
    }

    /// Vérifie les permissions
    pub fn require_scope(&self, scope: crate::api::auth::ApiScope) -> async_graphql::Result<()> {
        let auth = self.require_auth()?;
        
        if !auth.scopes.contains(&scope) && !auth.scopes.contains(&crate::api::auth::ApiScope::AdminAll) {
            return Err(async_graphql::Error::new(format!("Required scope: {}", scope.as_str()))
                .extend_with(|_, e| e.set("code", "FORBIDDEN")));
        }
        
        Ok(())
    }
}

/// Macro pour créer facilement des erreurs GraphQL
#[macro_export]
macro_rules! graphql_error {
    ($msg:expr) => {
        async_graphql::Error::new($msg)
    };
    ($msg:expr, $code:expr) => {
        async_graphql::Error::new($msg).extend_with(|_, e| e.set("code", $code))
    };
}

/// Macro pour créer des erreurs de validation
#[macro_export]
macro_rules! validation_error {
    ($msg:expr) => {
        async_graphql::Error::new($msg).extend_with(|_, e| e.set("code", "VALIDATION_ERROR"))
    };
}

/// Macro pour créer des erreurs de ressource non trouvée
#[macro_export]
macro_rules! not_found_error {
    ($resource:expr) => {
        async_graphql::Error::new(format!("{} not found", $resource))
            .extend_with(|_, e| e.set("code", "NOT_FOUND"))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphql_config_default() {
        let config = GraphQLConfig::default();
        assert!(config.enable_playground);
        assert!(config.enable_introspection);
        assert_eq!(config.max_depth, 15);
        assert_eq!(config.max_complexity, 1000);
        assert_eq!(config.query_timeout, 30);
        assert!(config.enable_subscriptions);
    }

    #[test]
    fn test_schema_creation() {
        let schema = create_schema();
        // Vérifie que le schéma a été créé avec les bons types
        assert!(!schema.sdl().is_empty());
    }

    #[tokio::test]
    async fn test_routes_creation() {
        let result = create_routes().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_graphql_context() {
        use crate::api::{auth::AuthConfig, server::ServerState};
        use crate::{Blockchain, BlockchainConfig};
        use std::sync::Arc;

        let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap());
        let auth_service = Arc::new(crate::api::auth::AuthService::new(AuthConfig::default()).unwrap());
        let user_manager = Arc::new(tokio::sync::RwLock::new(crate::api::auth::UserManager::new()));
        let config = crate::api::ApiConfig::default();

        let server_state = ServerState::new(blockchain, auth_service, user_manager, config);
        let context = GraphQLContext::new(server_state, None);

        // Test que l'authentification est requise
        assert!(context.require_auth().is_err());
    }
}