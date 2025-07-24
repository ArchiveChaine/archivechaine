//! Système de réplication intelligente pour ArchiveChain
//! 
//! Implémente la réplication adaptative basée sur :
//! - La popularité du contenu (>1000 accès/jour = 2x plus de répliques)
//! - L'importance du contenu (gouvernemental/académique prioritaire)
//! - La distribution géographique (minimum 3 régions pour contenu critique)
//! - L'adaptation automatique (réévaluation hebdomadaire)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use crate::crypto::Hash;
use crate::consensus::NodeId;
use crate::error::Result;
use super::{StorageNodeInfo, NodeType, StorageType, ContentMetadata};

/// Importance du contenu pour la réplication
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContentImportance {
    /// Contenu critique (gouvernemental, académique, légal)
    Critical,
    /// Contenu important (médias, références)
    High,
    /// Contenu standard
    Medium,
    /// Contenu à faible priorité
    Low,
}

impl ContentImportance {
    /// Retourne le facteur multiplicateur pour les répliques
    pub fn replication_multiplier(&self) -> f64 {
        match self {
            ContentImportance::Critical => 2.0,
            ContentImportance::High => 1.5,
            ContentImportance::Medium => 1.0,
            ContentImportance::Low => 0.7,
        }
    }

    /// Retourne le nombre minimum de régions requises
    pub fn min_regions_required(&self) -> u32 {
        match self {
            ContentImportance::Critical => 3,
            ContentImportance::High => 2,
            ContentImportance::Medium => 2,
            ContentImportance::Low => 1,
        }
    }
}

/// Configuration de la réplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationConfig {
    /// Nombre minimum de répliques (défaut: 3)
    pub min_replicas: u32,
    /// Nombre maximum de répliques (défaut: 15)
    pub max_replicas: u32,
    /// Seuil de popularité pour augmenter la réplication
    pub popularity_threshold: u64,
    /// Facteur multiplicateur pour contenu populaire
    pub popularity_multiplier: f64,
    /// Distribution géographique activée
    pub geographic_distribution: bool,
    /// Intervalle de réévaluation des stratégies
    pub reevaluation_interval: Duration,
    /// Seuil de capacité pour éviter les nœuds surchargés
    pub node_capacity_threshold: f64,
}

impl Default for ReplicationConfig {
    fn default() -> Self {
        Self {
            min_replicas: 3,
            max_replicas: 15,
            popularity_threshold: 1000, // 1000 accès/jour
            popularity_multiplier: 2.0,
            geographic_distribution: true,
            reevaluation_interval: Duration::from_secs(7 * 24 * 3600), // 1 semaine
            node_capacity_threshold: 0.85, // 85%
        }
    }
}

/// Stratégie de réplication pour un contenu spécifique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationStrategy {
    /// Nombre minimum de répliques
    pub min_replicas: u32,
    /// Nombre maximum de répliques
    pub max_replicas: u32,
    /// Seuil de popularité
    pub popularity_threshold: f64,
    /// Distribution géographique requise
    pub geographic_distribution: bool,
    /// Importance du contenu
    pub content_importance: ContentImportance,
    /// Préférences de placement
    pub placement_preferences: PlacementPreferences,
    /// Timestamp de dernière évaluation
    pub last_evaluated: SystemTime,
}

impl ReplicationStrategy {
    /// Crée une stratégie de réplication fixe avec un nombre spécifique de répliques
    pub fn fixed(replica_count: u32) -> Self {
        Self {
            min_replicas: replica_count,
            max_replicas: replica_count,
            popularity_threshold: 1000.0,
            geographic_distribution: false,
            content_importance: ContentImportance::Medium,
            placement_preferences: PlacementPreferences::default(),
            last_evaluated: SystemTime::now(),
        }
    }

    /// Crée une nouvelle stratégie basée sur les métadonnées
    pub fn from_metadata(metadata: &ContentMetadata, config: &ReplicationConfig) -> Self {
        let base_replicas = config.min_replicas;
        let importance_factor = metadata.importance.replication_multiplier();
        let popularity_factor = if metadata.popularity > config.popularity_threshold {
            config.popularity_multiplier
        } else {
            1.0
        };

        let calculated_replicas = (base_replicas as f64 * importance_factor * popularity_factor) as u32;
        let min_replicas = calculated_replicas.max(config.min_replicas);
        let max_replicas = calculated_replicas.min(config.max_replicas);

        Self {
            min_replicas,
            max_replicas,
            popularity_threshold: config.popularity_threshold as f64,
            geographic_distribution: config.geographic_distribution,
            content_importance: metadata.importance.clone(),
            placement_preferences: PlacementPreferences::from_metadata(metadata),
            last_evaluated: SystemTime::now(),
        }
    }

    /// Calcule le nombre optimal de répliques
    pub fn calculate_optimal_replicas(&self, current_popularity: u64) -> u32 {
        let base = self.min_replicas as f64;
        let importance_multiplier = self.content_importance.replication_multiplier();
        let popularity_multiplier = if current_popularity as f64 > self.popularity_threshold {
            2.0
        } else {
            1.0
        };

        let optimal = (base * importance_multiplier * popularity_multiplier) as u32;
        optimal.max(self.min_replicas).min(self.max_replicas)
    }

    /// Vérifie si la stratégie nécessite une réévaluation
    pub fn needs_reevaluation(&self, interval: Duration) -> bool {
        self.last_evaluated.elapsed().unwrap_or(Duration::ZERO) > interval
    }
}

/// Préférences de placement pour la réplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementPreferences {
    /// Régions préférées
    pub preferred_regions: Vec<String>,
    /// Types de nœuds préférés
    pub preferred_node_types: Vec<NodeType>,
    /// Types de stockage préférés
    pub preferred_storage_types: Vec<StorageType>,
    /// Éviter certains nœuds
    pub excluded_nodes: Vec<NodeId>,
    /// Affinité géographique
    pub geographic_affinity: bool,
}

impl PlacementPreferences {
    /// Crée des préférences à partir des métadonnées
    pub fn from_metadata(metadata: &ContentMetadata) -> Self {
        let preferred_storage_types = match metadata.importance {
            ContentImportance::Critical => vec![StorageType::Hot, StorageType::Warm],
            ContentImportance::High => vec![StorageType::Hot, StorageType::Warm],
            ContentImportance::Medium => vec![StorageType::Warm, StorageType::Cold],
            ContentImportance::Low => vec![StorageType::Cold],
        };

        let preferred_node_types = if metadata.popularity > 1000 {
            vec![NodeType::HotStorage, NodeType::FullArchive]
        } else {
            vec![NodeType::FullArchive, NodeType::LightStorage]
        };

        Self {
            preferred_regions: metadata.preferred_regions.clone(),
            preferred_node_types,
            preferred_storage_types,
            excluded_nodes: Vec::new(),
            geographic_affinity: true,
        }
    }
}

impl Default for PlacementPreferences {
    fn default() -> Self {
        Self {
            preferred_regions: Vec::new(),
            preferred_node_types: vec![NodeType::FullArchive, NodeType::LightStorage],
            preferred_storage_types: vec![StorageType::Warm],
            excluded_nodes: Vec::new(),
            geographic_affinity: false,
        }
    }
}

/// Gestionnaire de réplication adaptative
#[derive(Debug)]
pub struct ReplicationManager {
    /// Configuration de réplication
    config: ReplicationConfig,
    /// Stratégies actives par contenu
    strategies: HashMap<Hash, ReplicationStrategy>,
    /// Métriques de réplication
    metrics: ReplicationMetrics,
    /// Cache des nœuds disponibles
    available_nodes: HashMap<NodeId, StorageNodeInfo>,
}

impl ReplicationManager {
    /// Crée un nouveau gestionnaire de réplication
    pub fn new(config: ReplicationConfig) -> Self {
        Self {
            config,
            strategies: HashMap::new(),
            metrics: ReplicationMetrics::new(),
            available_nodes: HashMap::new(),
        }
    }

    /// Met à jour la liste des nœuds disponibles
    pub fn update_available_nodes(&mut self, nodes: HashMap<NodeId, StorageNodeInfo>) {
        self.available_nodes = nodes;
    }

    /// Crée une stratégie de réplication pour un nouveau contenu
    pub fn create_strategy(
        &mut self,
        content_hash: Hash,
        metadata: &ContentMetadata,
    ) -> Result<ReplicationStrategy> {
        let strategy = ReplicationStrategy::from_metadata(metadata, &self.config);
        self.strategies.insert(content_hash, strategy.clone());
        
        self.metrics.strategies_created += 1;
        Ok(strategy)
    }

    /// Sélectionne les nœuds optimaux pour la réplication
    pub fn select_nodes_for_replication(
        &self,
        content_hash: &Hash,
        target_replicas: u32,
    ) -> Result<Vec<NodeId>> {
        let strategy = self.strategies.get(content_hash)
            .ok_or_else(|| crate::error::CoreError::Internal {
                message: "Stratégie de réplication non trouvée".to_string(),
            })?;

        let mut candidates: Vec<_> = self.available_nodes.values()
            .filter(|node| self.is_node_suitable(node, strategy))
            .collect();

        // Trie par score de performance
        candidates.sort_by(|a, b| {
            b.performance_score().partial_cmp(&a.performance_score()).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Sélectionne les meilleurs nœuds en respectant la distribution géographique
        let mut selected = Vec::new();
        let mut used_regions = std::collections::HashSet::new();

        // Phase 1: sélection avec distribution géographique
        if strategy.geographic_distribution {
            for node in &candidates {
                if selected.len() >= target_replicas as usize {
                    break;
                }
                
                if !used_regions.contains(&node.region) || used_regions.len() < strategy.content_importance.min_regions_required() as usize {
                    selected.push(node.node_id.clone());
                    used_regions.insert(node.region.clone());
                }
            }
        }

        // Phase 2: compléter si nécessaire sans contrainte géographique
        for node in &candidates {
            if selected.len() >= target_replicas as usize {
                break;
            }
            
            if !selected.contains(&node.node_id) {
                selected.push(node.node_id.clone());
            }
        }

        Ok(selected)
    }

    /// Vérifie si un nœud est adapté pour stocker un contenu
    fn is_node_suitable(&self, node: &StorageNodeInfo, strategy: &ReplicationStrategy) -> bool {
        // Vérifie la disponibilité
        if !node.is_available_for_storage() {
            return false;
        }

        // Vérifie les préférences de type de nœud
        if !strategy.placement_preferences.preferred_node_types.is_empty() 
            && !strategy.placement_preferences.preferred_node_types.contains(&node.node_type) {
            return false;
        }

        // Vérifie les types de stockage supportés
        let storage_compatible = strategy.placement_preferences.preferred_storage_types.iter()
            .any(|st| node.supported_storage_types.contains(st));
        
        if !strategy.placement_preferences.preferred_storage_types.is_empty() && !storage_compatible {
            return false;
        }

        // Vérifie les exclusions
        if strategy.placement_preferences.excluded_nodes.contains(&node.node_id) {
            return false;
        }

        // Vérifie la capacité
        if node.capacity_usage_percent() > self.config.node_capacity_threshold * 100.0 {
            return false;
        }

        true
    }

    /// Réévalue les stratégies de réplication existantes
    pub async fn reevaluate_strategies(&mut self, popularity_data: &HashMap<Hash, u64>) -> Result<Vec<Hash>> {
        let mut updated_content = Vec::new();

        for (content_hash, strategy) in &mut self.strategies {
            if strategy.needs_reevaluation(self.config.reevaluation_interval) {
                if let Some(&current_popularity) = popularity_data.get(content_hash) {
                    let old_replicas = strategy.calculate_optimal_replicas(0);
                    let new_replicas = strategy.calculate_optimal_replicas(current_popularity);
                    
                    if old_replicas != new_replicas {
                        strategy.last_evaluated = SystemTime::now();
                        updated_content.push(*content_hash);
                        self.metrics.strategies_updated += 1;
                    }
                }
            }
        }

        Ok(updated_content)
    }

    /// Obtient les métriques de réplication
    pub fn get_metrics(&self) -> &ReplicationMetrics {
        &self.metrics
    }

    /// Obtient une stratégie de réplication
    pub fn get_strategy(&self, content_hash: &Hash) -> Option<&ReplicationStrategy> {
        self.strategies.get(content_hash)
    }

    /// Met à jour une stratégie de réplication
    pub fn update_strategy(&mut self, content_hash: Hash, strategy: ReplicationStrategy) {
        self.strategies.insert(content_hash, strategy);
        self.metrics.strategies_updated += 1;
    }
}

/// Métriques de réplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationMetrics {
    /// Nombre de stratégies créées
    pub strategies_created: u64,
    /// Nombre de stratégies mises à jour
    pub strategies_updated: u64,
    /// Nombre total de répliques créées
    pub total_replicas_created: u64,
    /// Nombre de répliques supprimées
    pub replicas_removed: u64,
    /// Distribution par région
    pub regional_distribution: HashMap<String, u64>,
    /// Latence moyenne de réplication
    pub average_replication_latency: Duration,
    /// Taux de succès de réplication
    pub replication_success_rate: f64,
}

impl ReplicationMetrics {
    /// Crée de nouvelles métriques
    pub fn new() -> Self {
        Self {
            strategies_created: 0,
            strategies_updated: 0,
            total_replicas_created: 0,
            replicas_removed: 0,
            regional_distribution: HashMap::new(),
            average_replication_latency: Duration::ZERO,
            replication_success_rate: 1.0,
        }
    }

    /// Met à jour les métriques après une réplication réussie
    pub fn record_successful_replication(&mut self, region: String, latency: Duration) {
        self.total_replicas_created += 1;
        *self.regional_distribution.entry(region).or_insert(0) += 1;
        
        // Calcul de la moyenne mobile de latence
        let current_avg_ms = self.average_replication_latency.as_millis() as f64;
        let new_latency_ms = latency.as_millis() as f64;
        let updated_avg = (current_avg_ms * 0.9) + (new_latency_ms * 0.1);
        self.average_replication_latency = Duration::from_millis(updated_avg as u64);
    }
}

/// Réplication adaptative automatique
pub struct AdaptiveReplication {
    manager: ReplicationManager,
    monitoring_interval: Duration,
    last_evaluation: SystemTime,
}

impl AdaptiveReplication {
    /// Crée un nouveau système de réplication adaptative
    pub fn new(config: ReplicationConfig, monitoring_interval: Duration) -> Self {
        Self {
            manager: ReplicationManager::new(config),
            monitoring_interval,
            last_evaluation: SystemTime::now(),
        }
    }

    /// Vérifie et adapte les stratégies de réplication
    pub async fn adapt_strategies(&mut self, popularity_data: HashMap<Hash, u64>) -> Result<()> {
        if self.last_evaluation.elapsed().unwrap_or(Duration::ZERO) < self.monitoring_interval {
            return Ok(());
        }

        let updated_content = self.manager.reevaluate_strategies(&popularity_data).await?;
        
        // Ici, on déclencherait les actions de réplication/suppression
        // selon les nouvelles stratégies
        
        self.last_evaluation = SystemTime::now();
        Ok(())
    }

    /// Accède au gestionnaire de réplication
    pub fn manager(&self) -> &ReplicationManager {
        &self.manager
    }

    /// Accès mutable au gestionnaire de réplication
    pub fn manager_mut(&mut self) -> &mut ReplicationManager {
        &mut self.manager
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    fn create_test_metadata() -> ContentMetadata {
        super::super::ContentMetadata {
            content_hash: Hash::zero(),
            size: 1024 * 1024, // 1MB
            content_type: "text/html".to_string(),
            importance: ContentImportance::High,
            popularity: 1500, // Au-dessus du seuil
            created_at: chrono::Utc::now(),
            preferred_regions: vec!["eu-west-1".to_string(), "us-east-1".to_string()],
            redundancy_level: 5,
            tags: vec!["web".to_string(), "important".to_string()],
        }
    }

    #[test]
    fn test_content_importance() {
        assert_eq!(ContentImportance::Critical.replication_multiplier(), 2.0);
        assert_eq!(ContentImportance::Critical.min_regions_required(), 3);
        assert_eq!(ContentImportance::Low.replication_multiplier(), 0.7);
    }

    #[test]
    fn test_replication_strategy_creation() {
        let config = ReplicationConfig::default();
        let metadata = create_test_metadata();
        let strategy = ReplicationStrategy::from_metadata(&metadata, &config);

        assert!(strategy.min_replicas >= config.min_replicas);
        assert!(strategy.max_replicas <= config.max_replicas);
        assert_eq!(strategy.content_importance, ContentImportance::High);
    }

    #[test]
    fn test_optimal_replicas_calculation() {
        let config = ReplicationConfig::default();
        let metadata = create_test_metadata();
        let strategy = ReplicationStrategy::from_metadata(&metadata, &config);

        let replicas_low_popularity = strategy.calculate_optimal_replicas(500);
        let replicas_high_popularity = strategy.calculate_optimal_replicas(1500);

        assert!(replicas_high_popularity >= replicas_low_popularity);
    }

    #[test]
    fn test_placement_preferences() {
        let metadata = create_test_metadata();
        let preferences = PlacementPreferences::from_metadata(&metadata);

        assert_eq!(preferences.preferred_regions, metadata.preferred_regions);
        assert!(preferences.geographic_affinity);
        assert!(preferences.preferred_storage_types.contains(&StorageType::Hot));
    }

    #[test]
    fn test_replication_manager() {
        let config = ReplicationConfig::default();
        let mut manager = ReplicationManager::new(config);
        let metadata = create_test_metadata();
        let content_hash = Hash::zero();

        let strategy = manager.create_strategy(content_hash, &metadata).unwrap();
        assert!(manager.get_strategy(&content_hash).is_some());
        
        let retrieved_strategy = manager.get_strategy(&content_hash).unwrap();
        assert_eq!(retrieved_strategy.content_importance, strategy.content_importance);
    }

    #[test]
    fn test_replication_metrics() {
        let mut metrics = ReplicationMetrics::new();
        assert_eq!(metrics.strategies_created, 0);

        metrics.record_successful_replication("eu-west-1".to_string(), Duration::from_millis(100));
        assert_eq!(metrics.total_replicas_created, 1);
        assert_eq!(metrics.regional_distribution.get("eu-west-1"), Some(&1));
    }
}