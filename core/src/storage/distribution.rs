//! Système de distribution géographique pour ArchiveChain
//! 
//! Implémente la distribution géographique des données avec :
//! - Mapping des nœuds par région géographique
//! - Algorithme de placement optimal
//! - Optimisation de la latence d'accès
//! - Stratégies de disaster recovery
//! - Minimum 3 régions pour contenu critique

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use crate::crypto::Hash;
use crate::consensus::NodeId;
use crate::error::Result;
use super::{StorageNodeInfo, ContentMetadata, replication::ContentImportance};

/// Information sur une région géographique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Region {
    /// Identifiant de la région (ex: "eu-west-1")
    pub id: String,
    /// Nom complet de la région
    pub name: String,
    /// Continent
    pub continent: String,
    /// Pays
    pub country: String,
    /// Coordonnées géographiques
    pub coordinates: Coordinates,
    /// Latence moyenne entre régions
    pub inter_region_latencies: HashMap<String, Duration>,
}

/// Coordonnées géographiques
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Coordinates {
    /// Latitude
    pub latitude: f64,
    /// Longitude
    pub longitude: f64,
}

impl Coordinates {
    /// Calcule la distance en kilomètres vers une autre coordonnée
    pub fn distance_to(&self, other: &Coordinates) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2) +
                lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        6371.0 * c // Rayon de la Terre en km
    }
}

/// Informations détaillées sur une région
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    /// Région de base
    pub region: Region,
    /// Nœuds disponibles dans cette région
    pub available_nodes: Vec<NodeId>,
    /// Capacité totale de la région
    pub total_capacity: u64,
    /// Capacité utilisée
    pub used_capacity: u64,
    /// Latence moyenne d'accès
    pub average_latency: Duration,
    /// Score de fiabilité de la région
    pub reliability_score: f64,
    /// Statut de la région
    pub status: RegionStatus,
}

/// Statut d'une région
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RegionStatus {
    /// Région active et disponible
    Active,
    /// Région en maintenance
    Maintenance,
    /// Région surchargée
    Overloaded,
    /// Région partiellement indisponible
    Degraded,
    /// Région hors ligne
    Offline,
}

impl RegionInfo {
    /// Calcule le pourcentage d'utilisation de la capacité
    pub fn capacity_usage_percent(&self) -> f64 {
        if self.total_capacity == 0 {
            return 0.0;
        }
        (self.used_capacity as f64 / self.total_capacity as f64) * 100.0
    }

    /// Vérifie si la région peut accepter du nouveau contenu
    pub fn can_accept_content(&self) -> bool {
        matches!(self.status, RegionStatus::Active) 
            && self.capacity_usage_percent() < 85.0
            && !self.available_nodes.is_empty()
    }
}

/// Configuration de distribution géographique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Nombre minimum de régions par contenu
    pub min_regions_per_content: u32,
    /// Optimisation de latence activée
    pub latency_optimization: bool,
    /// Disaster recovery activé
    pub disaster_recovery: bool,
    /// Seuil de latence maximum acceptable (ms)
    pub max_acceptable_latency: u32,
    /// Facteur de pondération pour la distance géographique
    pub distance_weight: f64,
    /// Facteur de pondération pour la latence réseau
    pub latency_weight: f64,
    /// Facteur de pondération pour la capacité
    pub capacity_weight: f64,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            min_regions_per_content: 2,
            latency_optimization: true,
            disaster_recovery: true,
            max_acceptable_latency: 500, // 500ms
            distance_weight: 0.3,
            latency_weight: 0.4,
            capacity_weight: 0.3,
        }
    }
}

/// Stratégie de placement géographique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlacementStrategy {
    /// Placement optimal pour la latence globale
    GlobalLatencyOptimized,
    /// Placement pour la résilience (spread maximal)
    MaximumResilience,
    /// Placement régional concentré
    RegionalConcentrated,
    /// Placement équilibré
    Balanced,
}

impl PlacementStrategy {
    /// Retourne les poids pour l'optimisation
    pub fn get_weights(&self) -> (f64, f64, f64) { // (distance, latence, capacité)
        match self {
            PlacementStrategy::GlobalLatencyOptimized => (0.2, 0.6, 0.2),
            PlacementStrategy::MaximumResilience => (0.5, 0.2, 0.3),
            PlacementStrategy::RegionalConcentrated => (0.1, 0.3, 0.6),
            PlacementStrategy::Balanced => (0.3, 0.4, 0.3),
        }
    }
}

/// Gestionnaire de distribution géographique
#[derive(Debug)]
pub struct DistributionManager {
    /// Configuration
    config: DistributionConfig,
    /// Informations sur les régions
    regions: HashMap<String, RegionInfo>,
    /// Mapping nœud -> région
    node_to_region: HashMap<NodeId, String>,
    /// Optimiseur de latence
    latency_optimizer: LatencyOptimizer,
    /// Stratégie de placement par défaut
    default_strategy: PlacementStrategy,
}

impl DistributionManager {
    /// Crée un nouveau gestionnaire de distribution
    pub fn new(config: DistributionConfig) -> Self {
        Self {
            config,
            regions: HashMap::new(),
            node_to_region: HashMap::new(),
            latency_optimizer: LatencyOptimizer::new(),
            default_strategy: PlacementStrategy::Balanced,
        }
    }

    /// Ajoute une région
    pub fn add_region(&mut self, region_info: RegionInfo) {
        let region_id = region_info.region.id.clone();
        
        // Met à jour le mapping nœud -> région
        for node_id in &region_info.available_nodes {
            self.node_to_region.insert(node_id.clone(), region_id.clone());
        }
        
        self.regions.insert(region_id, region_info);
    }

    /// Met à jour les informations d'un nœud
    pub fn update_node_info(&mut self, node_id: NodeId, node_info: &StorageNodeInfo) {
        if let Some(region_id) = self.node_to_region.get(&node_id) {
            if let Some(region_info) = self.regions.get_mut(region_id) {
                // Met à jour les statistiques de la région
                self.update_region_stats(region_info, node_info);
            }
        }
    }

    /// Met à jour les statistiques d'une région
    fn update_region_stats(&self, region_info: &mut RegionInfo, node_info: &StorageNodeInfo) {
        // Implémentation simplifiée - dans la réalité, on agrégerait toutes les métriques des nœuds
        region_info.average_latency = Duration::from_millis(node_info.average_latency as u64);
        region_info.reliability_score = node_info.reliability_score;
    }

    /// Sélectionne les régions optimales pour un contenu
    pub fn select_optimal_regions(
        &self,
        metadata: &ContentMetadata,
        strategy: Option<PlacementStrategy>,
    ) -> Result<Vec<String>> {
        let strategy = strategy.unwrap_or_else(|| self.default_strategy.clone());
        let min_regions = self.calculate_min_regions_required(metadata);
        
        let mut region_scores: Vec<_> = self.regions.iter()
            .filter(|(_, region)| region.can_accept_content())
            .map(|(region_id, region_info)| {
                let score = self.calculate_region_score(region_info, metadata, &strategy);
                (region_id.clone(), score)
            })
            .collect();

        // Trie par score décroissant
        region_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Sélectionne les meilleures régions
        let selected_regions: Vec<String> = region_scores
            .into_iter()
            .take(min_regions.max(self.config.min_regions_per_content) as usize)
            .map(|(region_id, _)| region_id)
            .collect();

        // Vérifie la contrainte de distribution minimale
        if selected_regions.len() < min_regions as usize {
            return Err(crate::error::CoreError::Internal {
                message: format!(
                    "Impossible de satisfaire la contrainte de {} régions minimales",
                    min_regions
                ),
            });
        }

        Ok(selected_regions)
    }

    /// Calcule le nombre minimum de régions requises
    fn calculate_min_regions_required(&self, metadata: &ContentMetadata) -> u32 {
        let importance_requirement = metadata.importance.min_regions_required();
        let config_requirement = self.config.min_regions_per_content;
        importance_requirement.max(config_requirement)
    }

    /// Calcule le score d'une région pour un contenu
    fn calculate_region_score(
        &self,
        region: &RegionInfo,
        metadata: &ContentMetadata,
        strategy: &PlacementStrategy,
    ) -> f64 {
        let (distance_weight, latency_weight, capacity_weight) = strategy.get_weights();

        // Score de capacité (plus de capacité disponible = meilleur score)
        let capacity_score = 1.0 - (region.capacity_usage_percent() / 100.0);

        // Score de latence (latence plus faible = meilleur score)
        let latency_ms = region.average_latency.as_millis() as f64;
        let latency_score = 1.0 - (latency_ms / self.config.max_acceptable_latency as f64).min(1.0);

        // Score de distance (pour les régions préférées)
        let distance_score = if metadata.preferred_regions.contains(&region.region.id) {
            1.0
        } else {
            0.5
        };

        // Score de fiabilité
        let reliability_score = region.reliability_score;

        // Score composite
        let base_score = distance_score * distance_weight 
            + latency_score * latency_weight 
            + capacity_score * capacity_weight;

        // Pondération par la fiabilité
        base_score * reliability_score
    }

    /// Sélectionne les nœuds dans les régions choisies
    pub fn select_nodes_in_regions(
        &self,
        regions: &[String],
        nodes_per_region: u32,
        available_nodes: &HashMap<NodeId, StorageNodeInfo>,
    ) -> Result<HashMap<String, Vec<NodeId>>> {
        let mut result = HashMap::new();

        for region_id in regions {
            if let Some(region_info) = self.regions.get(region_id) {
                let region_nodes: Vec<NodeId> = region_info.available_nodes
                    .iter()
                    .filter(|node_id| {
                        available_nodes.get(node_id)
                            .map(|node| node.is_available_for_storage())
                            .unwrap_or(false)
                    })
                    .take(nodes_per_region as usize)
                    .cloned()
                    .collect();

                result.insert(region_id.clone(), region_nodes);
            }
        }

        Ok(result)
    }

    /// Obtient les statistiques de distribution
    pub fn get_distribution_stats(&self) -> DistributionStats {
        let total_regions = self.regions.len();
        let active_regions = self.regions.values()
            .filter(|r| r.status == RegionStatus::Active)
            .count();

        let total_capacity: u64 = self.regions.values()
            .map(|r| r.total_capacity)
            .sum();

        let used_capacity: u64 = self.regions.values()
            .map(|r| r.used_capacity)
            .sum();

        let average_latency = if !self.regions.is_empty() {
            let total_latency: u64 = self.regions.values()
                .map(|r| r.average_latency.as_millis() as u64)
                .sum();
            Duration::from_millis(total_latency / self.regions.len() as u64)
        } else {
            Duration::ZERO
        };

        DistributionStats {
            total_regions: total_regions as u32,
            active_regions: active_regions as u32,
            total_capacity,
            used_capacity,
            average_inter_region_latency: average_latency,
            regional_distribution: self.get_regional_distribution(),
        }
    }

    /// Obtient la distribution régionale
    fn get_regional_distribution(&self) -> HashMap<String, RegionDistributionInfo> {
        self.regions.iter()
            .map(|(region_id, region_info)| {
                let info = RegionDistributionInfo {
                    node_count: region_info.available_nodes.len() as u32,
                    capacity_usage_percent: region_info.capacity_usage_percent(),
                    status: region_info.status.clone(),
                };
                (region_id.clone(), info)
            })
            .collect()
    }

    /// Optimise la distribution existante
    pub async fn optimize_distribution(&mut self) -> Result<OptimizationResult> {
        let mut improvements = 0;
        let mut redistributions = Vec::new();

        // Identifie les régions surchargées
        let overloaded_regions: Vec<_> = self.regions.iter()
            .filter(|(_, region)| region.capacity_usage_percent() > 90.0)
            .map(|(id, _)| id.clone())
            .collect();

        // Identifie les régions sous-utilisées
        let underloaded_regions: Vec<_> = self.regions.iter()
            .filter(|(_, region)| {
                region.capacity_usage_percent() < 50.0 && region.status == RegionStatus::Active
            })
            .map(|(id, _)| id.clone())
            .collect();

        // Planifie les redistributions
        for overloaded in &overloaded_regions {
            if let Some(target) = underloaded_regions.first() {
                redistributions.push(RedistributionPlan {
                    source_region: overloaded.clone(),
                    target_region: target.clone(),
                    estimated_data_size: 0, // À calculer selon les besoins
                });
                improvements += 1;
            }
        }

        Ok(OptimizationResult {
            improvements_identified: improvements,
            redistribution_plans: redistributions,
        })
    }

    /// Obtient les régions disponibles
    pub fn get_available_regions(&self) -> Vec<&RegionInfo> {
        self.regions.values()
            .filter(|region| region.can_accept_content())
            .collect()
    }
}

/// Optimiseur de latence
#[derive(Debug)]
pub struct LatencyOptimizer {
    /// Cache des latences mesurées
    latency_cache: HashMap<(String, String), Duration>,
}

impl LatencyOptimizer {
    /// Crée un nouveau optimiseur de latence
    pub fn new() -> Self {
        Self {
            latency_cache: HashMap::new(),
        }
    }

    /// Met à jour les données de latence
    pub fn update_latency(&mut self, from_region: String, to_region: String, latency: Duration) {
        self.latency_cache.insert((from_region.clone(), to_region.clone()), latency);
        self.latency_cache.insert((to_region, from_region), latency); // Bidirectionnel
    }

    /// Obtient la latence entre deux régions
    pub fn get_latency(&self, from_region: &str, to_region: &str) -> Option<Duration> {
        self.latency_cache.get(&(from_region.to_string(), to_region.to_string())).copied()
    }

    /// Trouve la région la plus proche d'un utilisateur
    pub fn find_closest_region(&self, user_region: &str, available_regions: &[String]) -> Option<String> {
        available_regions.iter()
            .filter_map(|region| {
                self.get_latency(user_region, region)
                    .map(|latency| (region.clone(), latency))
            })
            .min_by_key(|(_, latency)| *latency)
            .map(|(region, _)| region)
    }
}

/// Statistiques de distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionStats {
    /// Nombre total de régions
    pub total_regions: u32,
    /// Nombre de régions actives
    pub active_regions: u32,
    /// Capacité totale
    pub total_capacity: u64,
    /// Capacité utilisée
    pub used_capacity: u64,
    /// Latence moyenne inter-régions
    pub average_inter_region_latency: Duration,
    /// Distribution par région
    pub regional_distribution: HashMap<String, RegionDistributionInfo>,
}

/// Informations de distribution régionale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionDistributionInfo {
    /// Nombre de nœuds
    pub node_count: u32,
    /// Pourcentage d'utilisation de la capacité
    pub capacity_usage_percent: f64,
    /// Statut de la région
    pub status: RegionStatus,
}

/// Résultat d'optimisation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Nombre d'améliorations identifiées
    pub improvements_identified: u32,
    /// Plans de redistribution
    pub redistribution_plans: Vec<RedistributionPlan>,
}

/// Plan de redistribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedistributionPlan {
    /// Région source (surchargée)
    pub source_region: String,
    /// Région cible (sous-utilisée)
    pub target_region: String,
    /// Taille estimée des données à déplacer
    pub estimated_data_size: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    fn create_test_region() -> RegionInfo {
        RegionInfo {
            region: Region {
                id: "eu-west-1".to_string(),
                name: "Europe West 1".to_string(),
                continent: "Europe".to_string(),
                country: "Ireland".to_string(),
                coordinates: Coordinates {
                    latitude: 53.3498,
                    longitude: -6.2603,
                },
                inter_region_latencies: HashMap::new(),
            },
            available_nodes: vec![NodeId::from(Hash::zero())],
            total_capacity: 1_000_000_000,
            used_capacity: 400_000_000,
            average_latency: Duration::from_millis(50),
            reliability_score: 0.95,
            status: RegionStatus::Active,
        }
    }

    fn create_test_metadata() -> ContentMetadata {
        super::super::ContentMetadata {
            content_hash: Hash::zero(),
            size: 1024 * 1024,
            content_type: "text/html".to_string(),
            importance: ContentImportance::High,
            popularity: 1500,
            created_at: chrono::Utc::now(),
            preferred_regions: vec!["eu-west-1".to_string()],
            redundancy_level: 3,
            tags: vec!["web".to_string()],
        }
    }

    #[test]
    fn test_coordinates_distance() {
        let dublin = Coordinates { latitude: 53.3498, longitude: -6.2603 };
        let paris = Coordinates { latitude: 48.8566, longitude: 2.3522 };
        
        let distance = dublin.distance_to(&paris);
        assert!(distance > 700.0 && distance < 800.0); // ~780 km
    }

    #[test]
    fn test_region_info() {
        let region = create_test_region();
        assert_eq!(region.capacity_usage_percent(), 40.0);
        assert!(region.can_accept_content());
    }

    #[test]
    fn test_placement_strategy_weights() {
        let strategy = PlacementStrategy::GlobalLatencyOptimized;
        let (distance, latency, capacity) = strategy.get_weights();
        assert!((distance + latency + capacity - 1.0).abs() < 0.01);
        assert!(latency > distance); // Latence prioritaire
    }

    #[test]
    fn test_distribution_manager() {
        let config = DistributionConfig::default();
        let mut manager = DistributionManager::new(config);
        let region = create_test_region();
        
        manager.add_region(region);
        assert_eq!(manager.regions.len(), 1);
        
        let metadata = create_test_metadata();
        let regions = manager.select_optimal_regions(&metadata, None).unwrap();
        assert!(!regions.is_empty());
    }

    #[test]
    fn test_latency_optimizer() {
        let mut optimizer = LatencyOptimizer::new();
        
        optimizer.update_latency(
            "eu-west-1".to_string(),
            "us-east-1".to_string(),
            Duration::from_millis(150),
        );
        
        let latency = optimizer.get_latency("eu-west-1", "us-east-1");
        assert_eq!(latency, Some(Duration::from_millis(150)));
        
        // Test bidirectionnel
        let reverse_latency = optimizer.get_latency("us-east-1", "eu-west-1");
        assert_eq!(reverse_latency, Some(Duration::from_millis(150)));
    }

    #[test]
    fn test_min_regions_calculation() {
        let config = DistributionConfig::default();
        let manager = DistributionManager::new(config);
        
        let critical_metadata = super::super::ContentMetadata {
            importance: ContentImportance::Critical,
            ..create_test_metadata()
        };
        
        let min_regions = manager.calculate_min_regions_required(&critical_metadata);
        assert_eq!(min_regions, 3); // Critical content requires 3 regions
    }
}