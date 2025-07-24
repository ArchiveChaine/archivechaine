//! Tests end-to-end pour ArchiveChain
//!
//! Tests complets simulant des scénarios d'utilisation réels avec toutes les APIs.

use super::*;
use super::test_helpers::*;
use serde_json::json;
use std::time::Duration;
use tokio::time::timeout;

/// Test end-to-end complet : création d'archive via REST et suivi via WebSocket
pub struct EndToEndArchiveTest {
    archive_id: Option<String>,
    token: Option<String>,
}

impl EndToEndArchiveTest {
    pub fn new() -> Self {
        Self {
            archive_id: None,
            token: None,
        }
    }
}

#[async_trait::async_trait]
impl ApiTest for EndToEndArchiveTest {
    fn test_name(&self) -> &'static str {
        "end_to_end_archive_workflow"
    }

    async fn setup(&mut self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        // Crée un token pour les tests
        self.token = Some(test_setup.create_test_token().await?);
        Ok(())
    }

    async fn run(&self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        let client = create_http_client();
        let token = self.token.as_ref().unwrap();
        let test_data = generate_test_data();

        println!("  🔗 Testing complete archive workflow...");

        // Étape 1: Créer une archive via REST API
        println!("  📝 Step 1: Creating archive via REST API");
        let create_url = format!("{}/archives", test_setup.rest_base_url());
        let create_body = json!({
            "url": test_data.archive_url,
            "metadata": {
                "title": test_data.archive_title,
                "description": "End-to-end test archive"
            }
        });

        let response = authenticated_post(&client, &create_url, token, create_body).await?;
        assert_api_success!(response);

        let create_result: serde_json::Value = response.json().await?;
        let archive_id = create_result["archive_id"]
            .as_str()
            .ok_or("Missing archive_id in response")?;

        println!("  ✅ Archive created with ID: {}", archive_id);

        // Étape 2: Vérifier via REST API
        println!("  🔍 Step 2: Verifying archive via REST API");
        let get_url = format!("{}/archives/{}", test_setup.rest_base_url(), archive_id);
        let response = authenticated_get(&client, &get_url, token).await?;
        assert_api_success!(response);

        let archive: serde_json::Value = response.json().await?;
        assert_eq!(archive["archive_id"], archive_id);
        assert_eq!(archive["url"], test_data.archive_url);

        println!("  ✅ Archive verified via REST");

        // Étape 3: Rechercher via REST API
        println!("  🔎 Step 3: Searching for archive via REST API");
        let search_url = format!("{}/search?q={}", test_setup.rest_base_url(), test_data.archive_title);
        let response = authenticated_get(&client, &search_url, token).await?;
        assert_api_success!(response);

        let search_results: serde_json::Value = response.json().await?;
        println!("  ✅ Search completed, found {} results", 
            search_results["results"].as_array().unwrap_or(&vec![]).len());

        // Étape 4: Test GraphQL Query
        println!("  📊 Step 4: Querying archive via GraphQL");
        let graphql_query = json!({
            "query": format!(r#"
                query {{
                    archive(id: "{}") {{
                        id
                        url
                        status
                        metadata {{
                            title
                            description
                        }}
                    }}
                }}
            "#, archive_id)
        });

        let response = authenticated_post(&client, &test_setup.graphql_url(), token, graphql_query).await?;
        assert_api_success!(response);

        let graphql_result: serde_json::Value = response.json().await?;
        assert!(graphql_result["data"]["archive"].is_object());
        println!("  ✅ GraphQL query successful");

        // Étape 5: Test des statistiques réseau
        println!("  📈 Step 5: Getting network statistics");
        let stats_url = format!("{}/network/stats", test_setup.rest_base_url());
        let response = authenticated_get(&client, &stats_url, token).await?;
        assert_api_success!(response);

        let stats: serde_json::Value = response.json().await?;
        assert!(stats["total_archives"].is_number());
        println!("  ✅ Network statistics retrieved");

        println!("  🎉 End-to-end test completed successfully!");
        Ok(())
    }
}

/// Test de montée en charge basique
pub struct LoadTest {
    concurrent_requests: usize,
    token: Option<String>,
}

impl LoadTest {
    pub fn new(concurrent_requests: usize) -> Self {
        Self {
            concurrent_requests,
            token: None,
        }
    }
}

#[async_trait::async_trait]
impl ApiTest for LoadTest {
    fn test_name(&self) -> &'static str {
        "basic_load_test"
    }

    async fn setup(&mut self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        self.token = Some(test_setup.create_test_token().await?);
        Ok(())
    }

    async fn run(&self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        let client = create_http_client();
        let token = self.token.as_ref().unwrap();

        println!("  ⚡ Running load test with {} concurrent requests...", self.concurrent_requests);

        let start_time = std::time::Instant::now();
        let mut handles = Vec::new();

        // Lance plusieurs requêtes concurrentes
        for i in 0..self.concurrent_requests {
            let client = client.clone();
            let token = token.clone();
            let base_url = test_setup.rest_base_url();
            
            let handle = tokio::spawn(async move {
                let stats_url = format!("{}/network/stats", base_url);
                
                match timeout(
                    Duration::from_secs(10),
                    authenticated_get(&client, &stats_url, &token)
                ).await {
                    Ok(Ok(response)) => {
                        if response.status().is_success() {
                            Ok(())
                        } else {
                            Err(format!("Request {} failed with status: {}", i, response.status()))
                        }
                    }
                    Ok(Err(e)) => Err(format!("Request {} failed: {}", i, e)),
                    Err(_) => Err(format!("Request {} timed out", i)),
                }
            });
            
            handles.push(handle);
        }

        // Attend toutes les requêtes
        let mut successes = 0;
        let mut failures = 0;
        
        for handle in handles {
            match handle.await? {
                Ok(()) => successes += 1,
                Err(e) => {
                    failures += 1;
                    println!("    ❌ {}", e);
                }
            }
        }

        let duration = start_time.elapsed();
        let success_rate = successes as f64 / self.concurrent_requests as f64;

        println!("  📊 Load test results:");
        println!("    - Duration: {:?}", duration);
        println!("    - Successes: {}/{}", successes, self.concurrent_requests);
        println!("    - Success rate: {:.1}%", success_rate * 100.0);
        println!("    - Requests per second: {:.1}", self.concurrent_requests as f64 / duration.as_secs_f64());

        if success_rate < 0.9 {
            return Err(format!("Load test failed: success rate too low ({:.1}%)", success_rate * 100.0).into());
        }

        println!("  ✅ Load test passed!");
        Ok(())
    }
}

/// Test de résilience avec erreurs simulées
pub struct ResilienceTest {
    token: Option<String>,
}

impl ResilienceTest {
    pub fn new() -> Self {
        Self { token: None }
    }
}

#[async_trait::async_trait]
impl ApiTest for ResilienceTest {
    fn test_name(&self) -> &'static str {
        "resilience_test"
    }

    async fn setup(&mut self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        self.token = Some(test_setup.create_test_token().await?);
        Ok(())
    }

    async fn run(&self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        let client = create_http_client();
        let token = self.token.as_ref().unwrap();

        println!("  🛡️ Testing API resilience...");

        // Test 1: Requête avec des données invalides
        println!("  🔸 Test 1: Invalid data handling");
        let create_url = format!("{}/archives", test_setup.rest_base_url());
        let invalid_body = json!({
            "url": "", // URL vide
            "metadata": {
                "title": ""
            }
        });

        let response = authenticated_post(&client, &create_url, token, invalid_body).await?;
        assert_api_error!(response, reqwest::StatusCode::BAD_REQUEST);
        println!("    ✅ Invalid data properly rejected");

        // Test 2: Ressource inexistante
        println!("  🔸 Test 2: Non-existent resource");
        let get_url = format!("{}/archives/arc_nonexistent", test_setup.rest_base_url());
        let response = authenticated_get(&client, &get_url, token).await?;
        assert_api_error!(response, reqwest::StatusCode::NOT_FOUND);
        println!("    ✅ Non-existent resource properly handled");

        // Test 3: Authentification manquante
        println!("  🔸 Test 3: Missing authentication");
        let stats_url = format!("{}/network/stats", test_setup.rest_base_url());
        let response = client.get(&stats_url).send().await?;
        assert_api_error!(response, reqwest::StatusCode::UNAUTHORIZED);
        println!("    ✅ Missing authentication properly detected");

        // Test 4: Token invalide
        println!("  🔸 Test 4: Invalid token");
        let response = authenticated_get(&client, &stats_url, "invalid_token").await?;
        assert_api_error!(response, reqwest::StatusCode::UNAUTHORIZED);
        println!("    ✅ Invalid token properly rejected");

        // Test 5: GraphQL avec syntaxe invalide
        println!("  🔸 Test 5: Invalid GraphQL syntax");
        let invalid_graphql = json!({
            "query": "invalid graphql syntax {"
        });

        let response = authenticated_post(&client, &test_setup.graphql_url(), token, invalid_graphql).await?;
        assert_api_error!(response, reqwest::StatusCode::BAD_REQUEST);
        println!("    ✅ Invalid GraphQL syntax properly handled");

        println!("  🎉 Resilience test passed!");
        Ok(())
    }
}

/// Test de cohérence entre APIs
pub struct ConsistencyTest {
    token: Option<String>,
}

impl ConsistencyTest {
    pub fn new() -> Self {
        Self { token: None }
    }
}

#[async_trait::async_trait]
impl ApiTest for ConsistencyTest {
    fn test_name(&self) -> &'static str {
        "api_consistency_test"
    }

    async fn setup(&mut self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        self.token = Some(test_setup.create_test_token().await?);
        Ok(())
    }

    async fn run(&self, test_setup: &TestSetup) -> Result<(), Box<dyn std::error::Error>> {
        let client = create_http_client();
        let token = self.token.as_ref().unwrap();

        println!("  🔄 Testing consistency between APIs...");

        // Test 1: Statistiques réseau via REST et GraphQL
        println!("  🔸 Test 1: Network stats consistency (REST vs GraphQL)");
        
        // REST
        let rest_stats_url = format!("{}/network/stats", test_setup.rest_base_url());
        let rest_response = authenticated_get(&client, &rest_stats_url, token).await?;
        assert_api_success!(rest_response);
        let rest_stats: serde_json::Value = rest_response.json().await?;

        // GraphQL
        let graphql_query = json!({
            "query": r#"
                query {
                    networkStats {
                        totalNodes
                        activeNodes
                        currentBlockHeight
                    }
                }
            "#
        });
        
        let graphql_response = authenticated_post(&client, &test_setup.graphql_url(), token, graphql_query).await?;
        assert_api_success!(graphql_response);
        let graphql_result: serde_json::Value = graphql_response.json().await?;

        // Vérifie la cohérence (au moins les types doivent correspondre)
        assert!(rest_stats["total_nodes"].is_number());
        assert!(graphql_result["data"]["networkStats"]["totalNodes"].is_number());
        println!("    ✅ Network stats consistent between REST and GraphQL");

        // Test 2: Format des IDs
        println!("  🔸 Test 2: ID format consistency");
        let test_data = generate_test_data();
        let create_body = json!({
            "url": test_data.archive_url,
            "metadata": {
                "title": test_data.archive_title
            }
        });

        let create_url = format!("{}/archives", test_setup.rest_base_url());
        let response = authenticated_post(&client, &create_url, token, create_body).await?;
        assert_api_success!(response);

        let create_result: serde_json::Value = response.json().await?;
        let archive_id = create_result["archive_id"]
            .as_str()
            .ok_or("Missing archive_id")?;

        // Vérifie le format de l'ID
        assert!(archive_id.starts_with("arc_"), "Archive ID should start with 'arc_'");
        assert_eq!(archive_id.len(), 36, "Archive ID should be 36 characters long");
        println!("    ✅ ID format consistent: {}", archive_id);

        println!("  🎉 Consistency test passed!");
        Ok(())
    }
}

/// Fonction principale pour exécuter tous les tests end-to-end
pub async fn run_end_to_end_tests() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Starting End-to-End Integration Tests");
    println!("==========================================\n");

    let mut runner = IntegrationTestRunner::new();
    
    // Ajoute tous les tests
    runner.add_test(EndToEndArchiveTest::new());
    runner.add_test(LoadTest::new(10)); // 10 requêtes concurrentes
    runner.add_test(ResilienceTest::new());
    runner.add_test(ConsistencyTest::new());

    // Exécute tous les tests
    let results = runner.run_all().await?;
    
    // Affiche le résumé
    results.print_summary();

    if results.failures.is_empty() {
        println!("\n🎉 All end-to-end tests passed!");
        Ok(())
    } else {
        Err("Some end-to-end tests failed".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Ignoré par défaut car nécessite un serveur en cours d'exécution
    async fn test_full_end_to_end() {
        let result = run_end_to_end_tests().await;
        assert!(result.is_ok(), "End-to-end tests should pass");
    }

    #[test]
    fn test_end_to_end_archive_test_creation() {
        let test = EndToEndArchiveTest::new();
        assert_eq!(test.test_name(), "end_to_end_archive_workflow");
        assert!(test.archive_id.is_none());
        assert!(test.token.is_none());
    }

    #[test]
    fn test_load_test_creation() {
        let test = LoadTest::new(5);
        assert_eq!(test.test_name(), "basic_load_test");
        assert_eq!(test.concurrent_requests, 5);
    }

    #[test]
    fn test_resilience_test_creation() {
        let test = ResilienceTest::new();
        assert_eq!(test.test_name(), "resilience_test");
    }

    #[test]
    fn test_consistency_test_creation() {
        let test = ConsistencyTest::new();
        assert_eq!(test.test_name(), "api_consistency_test");
    }
}