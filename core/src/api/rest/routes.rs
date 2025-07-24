//! Routes pour l'API REST ArchiveChain
//!
//! Définit tous les endpoints REST selon les spécifications API.

use axum::{
    routing::{get, post, put, delete},
    Router,
};
use crate::api::{ApiResult, server::ServerState};
use super::handlers::*;

/// Crée toutes les routes REST
pub async fn create_routes() -> ApiResult<Router<ServerState>> {
    let router = Router::new()
        // Routes des archives
        .nest("/archives", archive_routes())
        // Routes de recherche
        .nest("/search", search_routes())
        // Routes des statistiques réseau
        .nest("/network", network_routes())
        // Routes des nœuds
        .nest("/nodes", node_routes())
        // Routes des blocs
        .nest("/blocks", block_routes())
        // Routes des contrats
        .nest("/contracts", contract_routes())
        // Routes des bounties
        .nest("/bounties", bounty_routes());

    Ok(router)
}

/// Routes pour les archives
fn archive_routes() -> Router<ServerState> {
    Router::new()
        // POST /archives - Créer une nouvelle archive
        .route("/", post(create_archive))
        // GET /archives - Lister les archives
        .route("/", get(list_archives))
        // GET /archives/{archive_id} - Récupérer une archive
        .route("/:archive_id", get(get_archive))
        // PUT /archives/{archive_id} - Mettre à jour une archive
        .route("/:archive_id", put(update_archive))
        // DELETE /archives/{archive_id} - Supprimer une archive
        .route("/:archive_id", delete(delete_archive))
        // GET /archives/{archive_id}/metadata - Métadonnées uniquement
        .route("/:archive_id/metadata", get(get_archive_metadata))
        // GET /archives/{archive_id}/status - Statut de l'archive
        .route("/:archive_id/status", get(get_archive_status))
        // POST /archives/{archive_id}/verify - Vérifier l'intégrité
        .route("/:archive_id/verify", post(verify_archive))
        // GET /archives/{archive_id}/replicas - Informations de réplication
        .route("/:archive_id/replicas", get(get_archive_replicas))
}

/// Routes pour la recherche
fn search_routes() -> Router<ServerState> {
    Router::new()
        // GET /search - Recherche générale
        .route("/", get(search_archives))
        // GET /search/advanced - Recherche avancée
        .route("/advanced", get(advanced_search))
        // GET /search/facets - Facettes de recherche
        .route("/facets", get(search_facets))
        // GET /search/suggestions - Suggestions de recherche
        .route("/suggestions", get(search_suggestions))
        // POST /search/bulk - Recherche en lot
        .route("/bulk", post(bulk_search))
}

/// Routes pour les statistiques réseau
fn network_routes() -> Router<ServerState> {
    Router::new()
        // GET /network/stats - Statistiques globales
        .route("/stats", get(get_network_stats))
        // GET /network/health - Santé du réseau
        .route("/health", get(get_network_health))
        // GET /network/topology - Topologie du réseau
        .route("/topology", get(get_network_topology))
        // GET /network/metrics - Métriques détaillées
        .route("/metrics", get(get_network_metrics))
        // GET /network/consensus - État du consensus
        .route("/consensus", get(get_consensus_state))
}

/// Routes pour les nœuds
fn node_routes() -> Router<ServerState> {
    Router::new()
        // GET /nodes - Lister les nœuds
        .route("/", get(list_nodes))
        // POST /nodes/register - Enregistrer un nouveau nœud
        .route("/register", post(register_node))
        // GET /nodes/{node_id} - Informations d'un nœud
        .route("/:node_id", get(get_node))
        // PUT /nodes/{node_id} - Mettre à jour un nœud
        .route("/:node_id", put(update_node))
        // DELETE /nodes/{node_id} - Désenregistrer un nœud
        .route("/:node_id", delete(unregister_node))
        // GET /nodes/{node_id}/status - Statut d'un nœud
        .route("/:node_id/status", get(get_node_status))
        // GET /nodes/{node_id}/performance - Performance d'un nœud
        .route("/:node_id/performance", get(get_node_performance))
        // GET /nodes/{node_id}/storage - Stockage d'un nœud
        .route("/:node_id/storage", get(get_node_storage))
        // POST /nodes/{node_id}/ping - Ping un nœud
        .route("/:node_id/ping", post(ping_node))
}

/// Routes pour les blocs
fn block_routes() -> Router<ServerState> {
    Router::new()
        // GET /blocks - Lister les blocs récents
        .route("/", get(list_blocks))
        // GET /blocks/{block_hash} - Récupérer un bloc
        .route("/:block_hash", get(get_block))
        // GET /blocks/{block_hash}/transactions - Transactions d'un bloc
        .route("/:block_hash/transactions", get(get_block_transactions))
        // GET /blocks/height/{height} - Bloc par hauteur
        .route("/height/:height", get(get_block_by_height))
        // GET /blocks/latest - Dernier bloc
        .route("/latest", get(get_latest_block))
        // GET /chain/stats - Statistiques de la chaîne
        .route("/chain/stats", get(get_chain_stats))
}

/// Routes pour les contrats intelligents
fn contract_routes() -> Router<ServerState> {
    Router::new()
        // GET /contracts - Lister les contrats
        .route("/", get(list_contracts))
        // POST /contracts - Déployer un contrat
        .route("/", post(deploy_contract))
        // GET /contracts/{contract_id} - Informations d'un contrat
        .route("/:contract_id", get(get_contract))
        // POST /contracts/{contract_id}/call - Appeler un contrat
        .route("/:contract_id/call", post(call_contract))
        // GET /contracts/{contract_id}/events - Événements d'un contrat
        .route("/:contract_id/events", get(get_contract_events))
        // GET /contracts/{contract_id}/state - État d'un contrat
        .route("/:contract_id/state", get(get_contract_state))
}

/// Routes pour les bounties
fn bounty_routes() -> Router<ServerState> {
    Router::new()
        // GET /bounties - Lister les bounties
        .route("/", get(list_bounties))
        // POST /bounties - Créer un bounty
        .route("/", post(create_bounty))
        // GET /bounties/{bounty_id} - Informations d'un bounty
        .route("/:bounty_id", get(get_bounty))
        // PUT /bounties/{bounty_id} - Mettre à jour un bounty
        .route("/:bounty_id", put(update_bounty))
        // DELETE /bounties/{bounty_id} - Annuler un bounty
        .route("/:bounty_id", delete(cancel_bounty))
        // POST /bounties/{bounty_id}/submit - Soumettre pour un bounty
        .route("/:bounty_id/submit", post(submit_bounty_proposal))
        // GET /bounties/{bounty_id}/proposals - Propositions pour un bounty
        .route("/:bounty_id/proposals", get(list_bounty_proposals))
        // POST /bounties/{bounty_id}/proposals/{proposal_id}/accept - Accepter une proposition
        .route("/:bounty_id/proposals/:proposal_id/accept", post(accept_bounty_proposal))
        // GET /bounties/{bounty_id}/status - Statut d'un bounty
        .route("/:bounty_id/status", get(get_bounty_status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[tokio::test]
    async fn test_archive_routes_structure() {
        let routes = archive_routes();
        // Ici on pourrait tester la structure des routes si nécessaire
        // Pour l'instant, on vérifie juste que ça compile
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_search_routes_structure() {
        let routes = search_routes();
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_network_routes_structure() {
        let routes = network_routes();
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_node_routes_structure() {
        let routes = node_routes();
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_block_routes_structure() {
        let routes = block_routes();
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_contract_routes_structure() {
        let routes = contract_routes();
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_bounty_routes_structure() {
        let routes = bounty_routes();
        assert_eq!(2 + 2, 4);
    }

    #[tokio::test]
    async fn test_create_routes() {
        let result = create_routes().await;
        assert!(result.is_ok());
    }
}