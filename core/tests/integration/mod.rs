//! Tests d'intégration pour les APIs ArchiveChain
//!
//! Teste l'intégration entre toutes les APIs (REST, GraphQL, WebSocket, gRPC, P2P).

pub mod api_integration;
pub mod rest_tests;
pub mod graphql_tests;
pub mod websocket_tests;
pub mod grpc_tests;
pub mod p2p_tests;
pub mod end_to_end;

use std::sync::Arc;
use tokio::sync::RwLock;

use archivechain_core::api::{
    ApiConfig, 
    auth::{AuthConfig, AuthService, UserManager},
    server::{ApiServer, ServerState},
};
use archivechain_core::{Blockchain, BlockchainConfig};

/// Configuration de test
pub struct TestConfig {
    pub rest_port: u16,
    pub graphql_port: u16,
    pub websocket_port: u16,
    pub grpc_port: u16,
    pub p2p_port: u16,
}

impl Default for TestConfig {
    fn default() -> Self {
        Self {
            rest_port: 8080,
            graphql_port: 8081,
            websocket_port: 8082,
            grpc_port: 9090,
            p2p_port: 8000,
        }
    }
}

/// Setup de test commun
pub struct TestSetup {
    pub blockchain: Arc<Blockchain>,
    pub auth_service: Arc<AuthService>,
    pub user_manager: Arc<RwLock<UserManager>>,
    pub server_state: ServerState,
    pub api_server: Option<ApiServer>,
    pub config: TestConfig,
}

impl TestSetup {
    /// Crée un nouveau setup de test
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::new_with_config(TestConfig::default()).await
    }

    /// Crée un setup avec une configuration personnalisée
    pub async fn new_with_config(config: TestConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Crée la blockchain
        let blockchain_config = BlockchainConfig::default();
        let blockchain = Arc::new(Blockchain::new(blockchain_config)?);

        // Crée le service d'authentification
        let auth_config = AuthConfig::default();
        let auth_service = Arc::new(AuthService::new(auth_config)?);

        // Crée le gestionnaire d'utilisateurs
        let user_manager = Arc::new(RwLock::new(UserManager::new()));

        // Configuration API
        let mut api_config = ApiConfig::default();
        api_config.rest.port = config.rest_port;
        api_config.websocket.listen_port = config.websocket_port;
        api_config.grpc.port = config.grpc_port;
        api_config.p2p.listen_port = config.p2p_port;

        // Crée l'état du serveur
        let server_state = ServerState::new(
            blockchain.clone(),
            auth_service.clone(),
            user_manager.clone(),
            api_config,
        );

        Ok(Self {
            blockchain,
            auth_service,
            user_manager,
            server_state,
            api_server: None,
            config,
        })
    }

    /// Démarre le serveur API
    pub async fn start_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let server = ApiServer::new(self.server_state.clone());
        server.start().await?;
        self.api_server = Some(server);
        
        // Attend un peu pour que le serveur démarre
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        
        Ok(())
    }

    /// Arrête le serveur API
    pub async fn stop_server(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(server) = self.api_server.take() {
            server.stop().await?;
        }
        Ok(())
    }

    /// Crée un token JWT pour les tests
    pub async fn create_test_token(&self) -> Result<String, Box<dyn std::error::Error>> {
        use archivechain_core::api::auth::{JwtClaims, ApiScope, RateLimit};
        use std::collections::HashMap;

        let claims = JwtClaims {
            sub: "test_user".to_string(),
            iss: "archivechain_test".to_string(),
            aud: "test".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: chrono::Utc::now().timestamp() as usize,
            nbf: chrono::Utc::now().timestamp() as usize,
            jti: uuid::Uuid::new_v4().to_string(),
            scope: vec![
                "archives:read".to_string(),
                "archives:write".to_string(),
                "network:read".to_string(),
            ],
            node_id: None,
            rate_limit: RateLimit::default(),
            user_metadata: HashMap::new(),
        };

        let token = self.auth_service.create_token(&claims)?;
        Ok(token)
    }

    /// Récupère l'URL de base pour REST
    pub fn rest_base_url(&self) -> String {
        format!("http://localhost:{}/api/v1", self.config.rest_port)
    }

    /// Récupère l'URL pour GraphQL
    pub fn graphql_url(&self) -> String {
        format!("http://localhost:{}/api/v1/graphql", self.config.rest_port)
    }

    /// Récupère l'URL pour WebSocket
    pub fn websocket_url(&self) -> String {
        format!("ws://localhost:{}/api/v1/ws", self.config.websocket_port)
    }

    /// Récupère l'adresse pour gRPC
    pub fn grpc_address(&self) -> String {
        format!("http://localhost:{}", self.config.grpc_port)
    }

    /// Récupère l'adresse pour P2P
    pub fn p2p_address(&self) -> String {
        format!("localhost:{}", self.config.p2p_port)
    }
}

impl Drop for TestSetup {
    fn drop(&mut self) {
        // Nettoyage automatique - dans un vrai test on utiliserait tokio::test
        // mais ici on fait au mieux
    }
}

/// Helpers pour les tests
pub mod test_helpers {
    use super::*;
    use reqwest::Client;
    use serde_json::Value;

    /// Client HTTP configuré pour les tests
    pub fn create_http_client() -> Client {
        Client::builder()
            .timeout(tokio::time::Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client")
    }

    /// Effectue une requête GET avec authentification
    pub async fn authenticated_get(
        client: &Client,
        url: &str,
        token: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
    }

    /// Effectue une requête POST avec authentification
    pub async fn authenticated_post(
        client: &Client,
        url: &str,
        token: &str,
        body: Value,
    ) -> Result<reqwest::Response, reqwest::Error> {
        client
            .post(url)
            .header("Authorization", format!("Bearer {}", token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
    }

    /// Attend qu'un service soit disponible
    pub async fn wait_for_service(url: &str, max_attempts: u32) -> bool {
        let client = create_http_client();
        
        for attempt in 1..=max_attempts {
            if let Ok(response) = client.get(url).send().await {
                if response.status().is_success() || response.status().as_u16() == 401 {
                    return true;
                }
            }
            
            if attempt < max_attempts {
                tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
            }
        }
        
        false
    }

    /// Génère des données de test aléatoires
    pub fn generate_test_data() -> TestData {
        TestData {
            archive_url: format!("https://example-{}.com", uuid::Uuid::new_v4().simple()),
            archive_title: format!("Test Archive {}", uuid::Uuid::new_v4().simple()),
            search_query: format!("test query {}", rand::random::<u32>()),
        }
    }

    /// Données de test
    pub struct TestData {
        pub archive_url: String,
        pub archive_title: String,
        pub search_query: String,
    }
}

/// Assertions personnalisées pour les tests
#[macro_export]
macro_rules! assert_api_success {
    ($response:expr) => {
        assert!(
            $response.status().is_success(),
            "API call failed with status: {}",
            $response.status()
        );
    };
}

#[macro_export]
macro_rules! assert_api_error {
    ($response:expr, $expected_status:expr) => {
        assert_eq!(
            $response.status(),
            $expected_status,
            "Expected status {}, got {}",
            $expected_status,
            $response.status()
        );
    };
}

/// Trait pour les tests d'API
#[async_trait::async_trait]
pub trait ApiTest {
    /// Nom du test
    fn test_name(&self) -> &'static str;
    
    /// Prépare le test
    async fn setup(&mut self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Exécute le test
    async fn run(&self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>>;
    
    /// Nettoie après le test
    async fn cleanup(&mut self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        // Par défaut, pas de nettoyage
        Ok(())
    }
}

/// Runner de tests d'intégration
pub struct IntegrationTestRunner {
    pub tests: Vec<Box<dyn ApiTest + Send + Sync>>,
}

impl IntegrationTestRunner {
    pub fn new() -> Self {
        Self {
            tests: Vec::new(),
        }
    }

    pub fn add_test<T>(&mut self, test: T) 
    where
        T: ApiTest + Send + Sync + 'static,
    {
        self.tests.push(Box::new(test));
    }

    pub async fn run_all(&mut self) -> Result<TestResults, Box<dyn std::error::Error>> {
        let mut results = TestResults::new();
        let test_setup = TestSetup::new().await?;

        for test in &self.tests {
            let test_name = test.test_name();
            println!("Running test: {}", test_name);
            
            let start_time = std::time::Instant::now();
            let result = test.run(&test_setup).await;
            let duration = start_time.elapsed();

            match result {
                Ok(()) => {
                    println!("✅ {} - passed ({:?})", test_name, duration);
                    results.add_success(test_name, duration);
                }
                Err(e) => {
                    println!("❌ {} - failed: {}", test_name, e);
                    results.add_failure(test_name, duration, e.to_string());
                }
            }
        }

        Ok(results)
    }
}

/// Résultats des tests
pub struct TestResults {
    pub successes: Vec<TestResult>,
    pub failures: Vec<TestResult>,
}

impl TestResults {
    pub fn new() -> Self {
        Self {
            successes: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn add_success(&mut self, name: &str, duration: std::time::Duration) {
        self.successes.push(TestResult {
            name: name.to_string(),
            duration,
            error: None,
        });
    }

    pub fn add_failure(&mut self, name: &str, duration: std::time::Duration, error: String) {
        self.failures.push(TestResult {
            name: name.to_string(),
            duration,
            error: Some(error),
        });
    }

    pub fn total_tests(&self) -> usize {
        self.successes.len() + self.failures.len()
    }

    pub fn success_rate(&self) -> f64 {
        if self.total_tests() == 0 {
            0.0
        } else {
            self.successes.len() as f64 / self.total_tests() as f64
        }
    }

    pub fn print_summary(&self) {
        println!("\n📊 Test Summary:");
        println!("Total tests: {}", self.total_tests());
        println!("Successes: {}", self.successes.len());
        println!("Failures: {}", self.failures.len());
        println!("Success rate: {:.1}%", self.success_rate() * 100.0);

        if !self.failures.is_empty() {
            println!("\n❌ Failed tests:");
            for failure in &self.failures {
                println!("  - {}: {}", failure.name, failure.error.as_ref().unwrap());
            }
        }
    }
}

pub struct TestResult {
    pub name: String,
    pub duration: std::time::Duration,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_setup_creation() {
        let setup = TestSetup::new().await;
        assert!(setup.is_ok());
        
        let setup = setup.unwrap();
        assert_eq!(setup.config.rest_port, 8080);
        assert!(!setup.rest_base_url().is_empty());
    }

    #[tokio::test]
    async fn test_token_creation() {
        let setup = TestSetup::new().await.unwrap();
        let token = setup.create_test_token().await;
        assert!(token.is_ok());
        
        let token = token.unwrap();
        assert!(!token.is_empty());
        assert!(token.starts_with("eyJ")); // JWT format
    }

    #[test]
    fn test_config_urls() {
        let config = TestConfig {
            rest_port: 9000,
            graphql_port: 9001,
            websocket_port: 9002,
            grpc_port: 9003,
            p2p_port: 9004,
        };

        let setup = TestSetup {
            blockchain: Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap()),
            auth_service: Arc::new(AuthService::new(AuthConfig::default()).unwrap()),
            user_manager: Arc::new(RwLock::new(UserManager::new())),
            server_state: ServerState::new(
                Arc::new(Blockchain::new(BlockchainConfig::default()).unwrap()),
                Arc::new(AuthService::new(AuthConfig::default()).unwrap()),
                Arc::new(RwLock::new(UserManager::new())),
                ApiConfig::default(),
            ),
            api_server: None,
            config,
        };

        assert_eq!(setup.rest_base_url(), "http://localhost:9000/api/v1");
        assert_eq!(setup.graphql_url(), "http://localhost:9000/api/v1/graphql");
        assert_eq!(setup.websocket_url(), "ws://localhost:9002/api/v1/ws");
        assert_eq!(setup.grpc_address(), "http://localhost:9003");
        assert_eq!(setup.p2p_address(), "localhost:9004");
    }

    #[test]
    fn test_results() {
        let mut results = TestResults::new();
        assert_eq!(results.total_tests(), 0);
        assert_eq!(results.success_rate(), 0.0);

        results.add_success("test1", std::time::Duration::from_millis(100));
        results.add_failure("test2", std::time::Duration::from_millis(200), "error".to_string());

        assert_eq!(results.total_tests(), 2);
        assert_eq!(results.success_rate(), 0.5);
        assert_eq!(results.successes.len(), 1);
        assert_eq!(results.failures.len(), 1);
    }
}