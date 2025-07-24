//! Système de récompenses pour ArchiveChain
//! 
//! Calcule et distribue les récompenses selon le tableau d'incitations PoA

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::crypto::Hash;
use crate::error::Result;
use super::{NodeId, ConsensusScore};

/// Calculateur de récompenses pour le consensus PoA
#[derive(Debug)]
pub struct RewardCalculator {
    /// Table d'incitations configurée
    incentive_table: IncentiveTable,
    /// Historique des récompenses distribuées
    reward_history: HashMap<NodeId, Vec<RewardEntry>>,
    /// Pool de récompenses disponible
    reward_pool: RewardPool,
    /// Statistiques de distribution
    distribution_stats: DistributionStatistics,
}

/// Table d'incitations selon le modèle économique ArchiveChain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncentiveTable {
    /// Récompenses pour l'archivage initial (100-500 ARC tokens)
    pub initial_archiving: RewardRange,
    /// Récompenses pour le stockage continu (10-50 ARC/mois)
    pub continuous_storage: RewardRange,
    /// Récompenses pour la bande passante (1-5 ARC/GB servi)
    pub bandwidth_service: RewardRange,
    /// Récompenses pour la découverte de contenu (25-100 ARC)
    pub content_discovery: RewardRange,
    /// Récompenses pour la validation de blocs
    pub block_validation: RewardRange,
    /// Récompenses pour la participation au consensus
    pub consensus_participation: RewardRange,
    /// Multiplicateurs pour les bonus de longévité
    pub longevity_multipliers: LongevityMultipliers,
}

/// Plage de récompenses (min, max)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRange {
    /// Récompense minimum
    pub min: u64,
    /// Récompense maximum
    pub max: u64,
    /// Unité de mesure (tokens, tokens/mois, tokens/GB, etc.)
    pub unit: String,
}

/// Multiplicateurs de bonus pour la longévité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongevityMultipliers {
    /// Bonus pour 30 jours continus
    pub thirty_days: f64,
    /// Bonus pour 90 jours continus
    pub ninety_days: f64,
    /// Bonus pour 365 jours continus
    pub one_year: f64,
    /// Bonus maximum possible
    pub max_multiplier: f64,
}

/// Pool de récompenses disponible
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPool {
    /// Tokens disponibles pour distribution
    pub available_tokens: u64,
    /// Tokens distribués cette période
    pub distributed_this_period: u64,
    /// Limite de distribution par période
    pub period_limit: u64,
    /// Timestamp de reset de la période
    pub period_reset: chrono::DateTime<chrono::Utc>,
    /// Type de période (quotidien, hebdomadaire, mensuel)
    pub period_type: PeriodType,
}

/// Types de périodes pour les récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PeriodType {
    Daily,
    Weekly,
    Monthly,
}

/// Entrée d'historique de récompense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardEntry {
    /// Type de récompense
    pub reward_type: RewardType,
    /// Montant accordé
    pub amount: u64,
    /// Raison/contexte de la récompense
    pub reason: String,
    /// Hash de la transaction ou action récompensée
    pub reference_hash: Option<Hash>,
    /// Timestamp d'attribution
    pub awarded_at: chrono::DateTime<chrono::Utc>,
    /// Multiplicateurs appliqués
    pub multipliers_applied: Vec<MultiplierInfo>,
}

/// Types de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewardType {
    /// Archivage initial d'un contenu
    InitialArchiving,
    /// Stockage continu
    ContinuousStorage,
    /// Service de bande passante
    BandwidthService,
    /// Découverte de contenu
    ContentDiscovery,
    /// Validation de bloc
    BlockValidation,
    /// Participation au consensus
    ConsensusParticipation,
    /// Bonus de longévité
    LongevityBonus,
}

/// Information sur un multiplicateur appliqué
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiplierInfo {
    /// Type de multiplicateur
    pub multiplier_type: MultiplierType,
    /// Valeur du multiplicateur
    pub value: f64,
    /// Raison du multiplicateur
    pub reason: String,
}

/// Types de multiplicateurs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiplierType {
    /// Bonus de performance
    Performance,
    /// Bonus de longévité
    Longevity,
    /// Bonus de participation
    Participation,
    /// Pénalité
    Penalty,
}

/// Distribution de récompenses pour un epoch/période
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistribution {
    /// Période concernée
    pub period: RewardPeriod,
    /// Récompenses par nœud
    pub node_rewards: HashMap<NodeId, NodeRewardSummary>,
    /// Total distribué
    pub total_distributed: u64,
    /// Pool restant après distribution
    pub remaining_pool: u64,
    /// Timestamp de création
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Période de récompense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPeriod {
    /// Numéro d'epoch ou de période
    pub period_id: u64,
    /// Date de début
    pub start_date: chrono::DateTime<chrono::Utc>,
    /// Date de fin
    pub end_date: chrono::DateTime<chrono::Utc>,
    /// Type de période
    pub period_type: PeriodType,
}

/// Résumé des récompenses pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeRewardSummary {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Score de consensus utilisé
    pub consensus_score: ConsensusScore,
    /// Récompenses détaillées par type
    pub rewards_by_type: HashMap<RewardType, u64>,
    /// Total des récompenses
    pub total_rewards: u64,
    /// Multiplicateurs appliqués
    pub applied_multipliers: Vec<MultiplierInfo>,
    /// Pénalités appliquées
    pub penalties: u64,
    /// Récompense finale
    pub final_reward: u64,
}

/// Statistiques de distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionStatistics {
    /// Total distribué depuis le début
    pub total_lifetime_distributed: u64,
    /// Nombre de bénéficiaires uniques
    pub unique_recipients: usize,
    /// Distribution moyenne par nœud
    pub average_reward_per_node: u64,
    /// Dernière distribution
    pub last_distribution: Option<chrono::DateTime<chrono::Utc>>,
    /// Période la plus généreuse
    pub highest_distribution_period: Option<u64>,
    /// Montant de la plus grosse distribution
    pub highest_distribution_amount: u64,
}

impl RewardCalculator {
    /// Crée un nouveau calculateur de récompenses
    pub fn new(incentive_table: IncentiveTable, initial_pool: u64) -> Self {
        Self {
            incentive_table,
            reward_history: HashMap::new(),
            reward_pool: RewardPool {
                available_tokens: initial_pool,
                distributed_this_period: 0,
                period_limit: initial_pool / 100, // 1% par période par défaut
                period_reset: chrono::Utc::now() + chrono::Duration::days(1),
                period_type: PeriodType::Daily,
            },
            distribution_stats: DistributionStatistics {
                total_lifetime_distributed: 0,
                unique_recipients: 0,
                average_reward_per_node: 0,
                last_distribution: None,
                highest_distribution_period: None,
                highest_distribution_amount: 0,
            },
        }
    }

    /// Calcule les récompenses pour l'archivage initial
    pub fn calculate_initial_archiving_reward(
        &self,
        archive_size: u64,
        quality_score: f64,
        consensus_score: &ConsensusScore,
    ) -> u64 {
        let base_reward = self.calculate_base_reward(
            &self.incentive_table.initial_archiving,
            quality_score,
        );
        
        // Bonus basé sur la taille de l'archive
        let size_multiplier = if archive_size > 1024 * 1024 { // > 1MB
            1.2
        } else if archive_size > 1024 * 100 { // > 100KB
            1.1
        } else {
            1.0
        };
        
        let consensus_multiplier = 0.5 + consensus_score.combined_score * 0.5;
        
        (base_reward as f64 * size_multiplier * consensus_multiplier) as u64
    }

    /// Calcule les récompenses pour le stockage continu
    pub fn calculate_continuous_storage_reward(
        &self,
        stored_archives: u32,
        storage_duration_days: u64,
        consensus_score: &ConsensusScore,
    ) -> u64 {
        let base_reward = self.calculate_base_reward(
            &self.incentive_table.continuous_storage,
            consensus_score.storage_score,
        );
        
        // Bonus pour la durée de stockage
        let duration_multiplier = (storage_duration_days as f64 / 30.0).min(2.0); // Max 2x pour 60+ jours
        
        // Bonus pour le nombre d'archives
        let volume_multiplier = (stored_archives as f64 / 10.0).min(1.5); // Max 1.5x pour 15+ archives
        
        (base_reward as f64 * duration_multiplier * volume_multiplier) as u64
    }

    /// Calcule les récompenses pour le service de bande passante
    pub fn calculate_bandwidth_reward(
        &self,
        bytes_served: u64,
        service_quality: f64,
        consensus_score: &ConsensusScore,
    ) -> u64 {
        let gb_served = bytes_served as f64 / (1024.0 * 1024.0 * 1024.0);
        
        let base_rate = self.interpolate_reward_range(
            &self.incentive_table.bandwidth_service,
            service_quality,
        );
        
        let consensus_multiplier = 0.7 + consensus_score.bandwidth_score * 0.3;
        
        (gb_served * base_rate as f64 * consensus_multiplier) as u64
    }

    /// Calcule les récompenses pour la découverte de contenu
    pub fn calculate_content_discovery_reward(
        &self,
        discovery_value: f64,
        consensus_score: &ConsensusScore,
    ) -> u64 {
        let base_reward = self.calculate_base_reward(
            &self.incentive_table.content_discovery,
            discovery_value,
        );
        
        let consensus_multiplier = 0.8 + consensus_score.combined_score * 0.2;
        
        (base_reward as f64 * consensus_multiplier) as u64
    }

    /// Calcule les récompenses pour la validation de blocs
    pub fn calculate_block_validation_reward(
        &self,
        validation_success: bool,
        block_complexity: f64,
        consensus_score: &ConsensusScore,
    ) -> u64 {
        if !validation_success {
            return 0;
        }
        
        let base_reward = self.calculate_base_reward(
            &self.incentive_table.block_validation,
            block_complexity,
        );
        
        let consensus_multiplier = 0.6 + consensus_score.combined_score * 0.4;
        
        (base_reward as f64 * consensus_multiplier) as u64
    }

    /// Calcule les bonus de longévité
    pub fn calculate_longevity_bonus(
        &self,
        base_rewards: u64,
        participation_days: u64,
    ) -> u64 {
        let multiplier = if participation_days >= 365 {
            self.incentive_table.longevity_multipliers.one_year
        } else if participation_days >= 90 {
            self.incentive_table.longevity_multipliers.ninety_days
        } else if participation_days >= 30 {
            self.incentive_table.longevity_multipliers.thirty_days
        } else {
            1.0
        };
        
        let bonus = (base_rewards as f64 * (multiplier - 1.0)) as u64;
        bonus.min((base_rewards as f64 * (self.incentive_table.longevity_multipliers.max_multiplier - 1.0)) as u64)
    }

    /// Distribue les récompenses pour une période
    pub fn distribute_rewards_for_period(
        &mut self,
        period: RewardPeriod,
        node_contributions: HashMap<NodeId, NodeContribution>,
    ) -> Result<RewardDistribution> {
        // Vérifie si le pool a suffisamment de tokens
        if self.reward_pool.available_tokens == 0 {
            return Err(crate::error::CoreError::Internal {
                message: "Pool de récompenses vide".to_string()
            });
        }

        // Reset le pool si nécessaire
        self.check_and_reset_period();

        let mut distribution = RewardDistribution {
            period: period.clone(),
            node_rewards: HashMap::new(),
            total_distributed: 0,
            remaining_pool: self.reward_pool.available_tokens,
            created_at: chrono::Utc::now(),
        };

        // Calcule les récompenses pour chaque nœud
        for (node_id, contribution) in node_contributions {
            let node_summary = self.calculate_node_rewards(&node_id, &contribution)?;
            let total_reward = node_summary.final_reward;

            // Vérifie les limites du pool
            if distribution.total_distributed + total_reward > self.reward_pool.period_limit {
                break; // Pool épuisé pour cette période
            }

            distribution.node_rewards.insert(node_id.clone(), node_summary);
            distribution.total_distributed += total_reward;

            // Enregistre dans l'historique
            self.record_reward_in_history(node_id, total_reward, RewardType::ConsensusParticipation);
        }

        // Met à jour le pool
        self.reward_pool.distributed_this_period += distribution.total_distributed;
        self.reward_pool.available_tokens -= distribution.total_distributed;
        distribution.remaining_pool = self.reward_pool.available_tokens;

        // Met à jour les statistiques
        self.update_distribution_statistics(&distribution);

        Ok(distribution)
    }

    /// Ajoute des tokens au pool de récompenses
    pub fn add_to_reward_pool(&mut self, amount: u64) {
        self.reward_pool.available_tokens += amount;
    }

    /// Obtient l'historique des récompenses d'un nœud
    pub fn get_node_reward_history(&self, node_id: &NodeId) -> Vec<&RewardEntry> {
        self.reward_history.get(node_id)
            .map(|entries| entries.iter().collect())
            .unwrap_or_default()
    }

    /// Obtient les statistiques de distribution
    pub fn get_distribution_statistics(&self) -> &DistributionStatistics {
        &self.distribution_stats
    }

    /// Obtient l'état actuel du pool de récompenses
    pub fn get_reward_pool_status(&self) -> &RewardPool {
        &self.reward_pool
    }

    // Méthodes privées

    fn calculate_base_reward(&self, range: &RewardRange, quality_factor: f64) -> u64 {
        self.interpolate_reward_range(range, quality_factor)
    }

    fn interpolate_reward_range(&self, range: &RewardRange, factor: f64) -> u64 {
        let factor = factor.clamp(0.0, 1.0);
        let diff = range.max - range.min;
        range.min + ((diff as f64) * factor) as u64
    }

    fn calculate_node_rewards(
        &self,
        node_id: &NodeId,
        contribution: &NodeContribution,
    ) -> Result<NodeRewardSummary> {
        let mut rewards_by_type = HashMap::new();
        let mut total_rewards = 0u64;
        let mut applied_multipliers = Vec::new();

        // Calcule chaque type de récompense
        if contribution.blocks_validated > 0 {
            let reward = self.calculate_block_validation_reward(
                true,
                contribution.validation_quality,
                &contribution.consensus_score,
            );
            rewards_by_type.insert(RewardType::BlockValidation, reward);
            total_rewards += reward;
        }

        if contribution.bytes_served > 0 {
            let reward = self.calculate_bandwidth_reward(
                contribution.bytes_served,
                contribution.service_quality,
                &contribution.consensus_score,
            );
            rewards_by_type.insert(RewardType::BandwidthService, reward);
            total_rewards += reward;
        }

        if contribution.archives_stored > 0 {
            let reward = self.calculate_continuous_storage_reward(
                contribution.archives_stored,
                contribution.storage_duration_days,
                &contribution.consensus_score,
            );
            rewards_by_type.insert(RewardType::ContinuousStorage, reward);
            total_rewards += reward;
        }

        // Applique les bonus de longévité
        let longevity_bonus = self.calculate_longevity_bonus(
            total_rewards,
            contribution.participation_days,
        );
        
        if longevity_bonus > 0 {
            rewards_by_type.insert(RewardType::LongevityBonus, longevity_bonus);
            applied_multipliers.push(MultiplierInfo {
                multiplier_type: MultiplierType::Longevity,
                value: (longevity_bonus as f64 / total_rewards as f64) + 1.0,
                reason: format!("Bonus de {} jours de participation", contribution.participation_days),
            });
            total_rewards += longevity_bonus;
        }

        // Applique les pénalités
        let penalties = contribution.penalties * 100; // 100 tokens par pénalité
        let final_reward = total_rewards.saturating_sub(penalties);

        Ok(NodeRewardSummary {
            node_id: node_id.clone(),
            consensus_score: contribution.consensus_score.clone(),
            rewards_by_type,
            total_rewards,
            applied_multipliers,
            penalties,
            final_reward,
        })
    }

    fn check_and_reset_period(&mut self) {
        if chrono::Utc::now() > self.reward_pool.period_reset {
            self.reward_pool.distributed_this_period = 0;
            
            // Calcule la prochaine date de reset
            let next_reset = match self.reward_pool.period_type {
                PeriodType::Daily => chrono::Utc::now() + chrono::Duration::days(1),
                PeriodType::Weekly => chrono::Utc::now() + chrono::Duration::weeks(1),
                PeriodType::Monthly => chrono::Utc::now() + chrono::Duration::days(30),
            };
            
            self.reward_pool.period_reset = next_reset;
        }
    }

    fn record_reward_in_history(&mut self, node_id: NodeId, amount: u64, reward_type: RewardType) {
        let entry = RewardEntry {
            reward_type,
            amount,
            reason: "Participation au consensus".to_string(),
            reference_hash: None,
            awarded_at: chrono::Utc::now(),
            multipliers_applied: Vec::new(),
        };

        self.reward_history.entry(node_id).or_insert_with(Vec::new).push(entry);
    }

    fn update_distribution_statistics(&mut self, distribution: &RewardDistribution) {
        self.distribution_stats.total_lifetime_distributed += distribution.total_distributed;
        self.distribution_stats.unique_recipients = self.reward_history.len();
        
        if self.distribution_stats.unique_recipients > 0 {
            self.distribution_stats.average_reward_per_node = 
                self.distribution_stats.total_lifetime_distributed / self.distribution_stats.unique_recipients as u64;
        }

        self.distribution_stats.last_distribution = Some(distribution.created_at);

        if distribution.total_distributed > self.distribution_stats.highest_distribution_amount {
            self.distribution_stats.highest_distribution_amount = distribution.total_distributed;
            self.distribution_stats.highest_distribution_period = Some(distribution.period.period_id);
        }
    }
}

/// Contribution d'un nœud pour une période
#[derive(Debug, Clone)]
pub struct NodeContribution {
    /// Score de consensus actuel
    pub consensus_score: ConsensusScore,
    /// Nombre de blocs validés
    pub blocks_validated: u32,
    /// Qualité de validation (0.0 - 1.0)
    pub validation_quality: f64,
    /// Bytes servis en bande passante
    pub bytes_served: u64,
    /// Qualité de service (0.0 - 1.0)
    pub service_quality: f64,
    /// Nombre d'archives stockées
    pub archives_stored: u32,
    /// Durée de stockage en jours
    pub storage_duration_days: u64,
    /// Jours de participation total
    pub participation_days: u64,
    /// Pénalités appliquées
    pub penalties: u32,
}

impl Default for IncentiveTable {
    fn default() -> Self {
        Self {
            initial_archiving: RewardRange {
                min: 100,
                max: 500,
                unit: "ARC tokens".to_string(),
            },
            continuous_storage: RewardRange {
                min: 10,
                max: 50,
                unit: "ARC tokens/mois".to_string(),
            },
            bandwidth_service: RewardRange {
                min: 1,
                max: 5,
                unit: "ARC tokens/GB".to_string(),
            },
            content_discovery: RewardRange {
                min: 25,
                max: 100,
                unit: "ARC tokens".to_string(),
            },
            block_validation: RewardRange {
                min: 10,
                max: 50,
                unit: "ARC tokens".to_string(),
            },
            consensus_participation: RewardRange {
                min: 5,
                max: 25,
                unit: "ARC tokens".to_string(),
            },
            longevity_multipliers: LongevityMultipliers {
                thirty_days: 1.1,   // +10%
                ninety_days: 1.25,  // +25%
                one_year: 1.5,      // +50%
                max_multiplier: 2.0, // +100% maximum
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    #[test]
    fn test_reward_calculator_creation() {
        let incentive_table = IncentiveTable::default();
        let calculator = RewardCalculator::new(incentive_table, 1000000);
        
        assert_eq!(calculator.reward_pool.available_tokens, 1000000);
        assert_eq!(calculator.distribution_stats.total_lifetime_distributed, 0);
    }

    #[test]
    fn test_initial_archiving_reward() {
        let incentive_table = IncentiveTable::default();
        let calculator = RewardCalculator::new(incentive_table, 1000000);
        
        let consensus_score = super::super::ConsensusScore {
            storage_score: 0.8,
            bandwidth_score: 0.7,
            longevity_score: 0.6,
            combined_score: 0.7,
            node_id: NodeId::from(Hash::zero()),
            calculated_at: chrono::Utc::now(),
        };
        
        let reward = calculator.calculate_initial_archiving_reward(
            1024 * 1024, // 1MB
            0.9,          // High quality
            &consensus_score,
        );
        
        assert!(reward > 0);
        assert!(reward >= 100); // Minimum de la table
        assert!(reward <= 600); // Maximum + bonus
    }

    #[test]
    fn test_bandwidth_reward() {
        let incentive_table = IncentiveTable::default();
        let calculator = RewardCalculator::new(incentive_table, 1000000);
        
        let consensus_score = super::super::ConsensusScore {
            storage_score: 0.8,
            bandwidth_score: 0.9,
            longevity_score: 0.6,
            combined_score: 0.77,
            node_id: NodeId::from(Hash::zero()),
            calculated_at: chrono::Utc::now(),
        };
        
        let bytes_served = 1024 * 1024 * 1024; // 1GB
        let reward = calculator.calculate_bandwidth_reward(
            bytes_served,
            0.8, // Good service quality
            &consensus_score,
        );
        
        assert!(reward > 0);
        // Devrait être entre 1-5 ARC par GB avec multiplicateurs
    }

    #[test]
    fn test_longevity_bonus() {
        let incentive_table = IncentiveTable::default();
        let calculator = RewardCalculator::new(incentive_table, 1000000);
        
        let base_rewards = 1000;
        
        // Test bonus 30 jours
        let bonus_30 = calculator.calculate_longevity_bonus(base_rewards, 30);
        assert_eq!(bonus_30, 100); // 10% de 1000
        
        // Test bonus 1 an
        let bonus_365 = calculator.calculate_longevity_bonus(base_rewards, 365);
        assert_eq!(bonus_365, 500); // 50% de 1000
    }

    #[test]
    fn test_reward_pool_management() {
        let incentive_table = IncentiveTable::default();
        let mut calculator = RewardCalculator::new(incentive_table, 1000);
        
        let initial_amount = calculator.reward_pool.available_tokens;
        
        // Ajoute des tokens
        calculator.add_to_reward_pool(500);
        assert_eq!(calculator.reward_pool.available_tokens, initial_amount + 500);
        
        // Vérifie les limites de période
        assert!(calculator.reward_pool.period_limit > 0);
    }
}