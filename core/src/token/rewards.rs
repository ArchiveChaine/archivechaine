//! Système de récompenses économiques pour ArchiveChain
//!
//! Implémente les récompenses selon les spécifications :
//! - Archivage initial : 100-500 ARC (base + qualité + rareté)
//! - Stockage continu : 10-50 ARC/mois (capacité + performance)
//! - Bande passante : 1-5 ARC/GB (performance + qualité de service)
//! - Découverte : 25-100 ARC (importance + impact)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey};
use super::{TokenOperationResult, TokenOperationError, ARCToken};

/// Système de récompenses principal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSystem {
    /// Pool de récompenses d'archivage
    pub archival_pool: RewardPool,
    /// Pool de récompenses de stockage
    pub storage_pool: RewardPool,
    /// Pool de récompenses de bande passante
    pub bandwidth_pool: RewardPool,
    /// Pool de récompenses de découverte
    pub discovery_pool: RewardPool,
    /// Modèle économique pour calculs
    pub economic_model: EconomicModel,
    /// Historique des distributions
    pub distribution_history: Vec<RewardDistribution>,
    /// Métriques de performance
    pub performance_metrics: PerformanceMetrics,
    /// Configuration
    pub config: RewardConfig,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Pool de récompenses pour un type spécifique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPool {
    /// Type de récompense
    pub reward_type: RewardType,
    /// Montant total alloué
    pub total_allocation: u64,
    /// Montant distribué
    pub distributed_amount: u64,
    /// Montant disponible
    pub available_amount: u64,
    /// Limite de distribution par période
    pub period_limit: u64,
    /// Montant distribué cette période
    pub distributed_this_period: u64,
    /// Date de reset de période
    pub period_reset_date: DateTime<Utc>,
    /// Historique des distributions
    pub distribution_records: Vec<PoolDistributionRecord>,
}

/// Modèle économique pour calculs de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicModel {
    /// Récompense de base pour archivage (100 ARC)
    pub base_archive_reward: u64,
    /// Multiplicateur maximum de qualité (5x)
    pub max_quality_multiplier: f64,
    /// Bonus pour contenu rare (100 ARC)
    pub rarity_bonus: u64,
    /// Taux de base pour stockage (10 ARC/TB/mois)
    pub base_storage_rate_per_tb: u64,
    /// Multiplicateur de performance stockage (max 5x)
    pub max_storage_performance_multiplier: f64,
    /// Taux de base bande passante (1 ARC/GB)
    pub base_bandwidth_rate_per_gb: u64,
    /// Multiplicateur de performance bande passante (max 5x)
    pub max_bandwidth_performance_multiplier: f64,
    /// Récompense de base découverte (25 ARC)
    pub base_discovery_reward: u64,
    /// Multiplicateur d'importance découverte (max 4x)
    pub max_discovery_importance_multiplier: f64,
}

/// Distribution de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistribution {
    /// ID unique de la distribution
    pub distribution_id: Hash,
    /// Type de récompense
    pub reward_type: RewardType,
    /// Bénéficiaires et montants
    pub recipients: HashMap<PublicKey, RewardAllocation>,
    /// Montant total distribué
    pub total_amount: u64,
    /// Critères utilisés
    pub criteria: RewardCriteria,
    /// Date de distribution
    pub distribution_date: DateTime<Utc>,
    /// Hash de transaction
    pub transaction_hash: Hash,
}

/// Allocation de récompense pour un bénéficiaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardAllocation {
    /// Bénéficiaire
    pub recipient: PublicKey,
    /// Montant de base
    pub base_amount: u64,
    /// Multiplicateurs appliqués
    pub multipliers: Vec<RewardMultiplier>,
    /// Bonus appliqués
    pub bonuses: Vec<RewardBonus>,
    /// Montant final
    pub final_amount: u64,
    /// Détails du calcul
    pub calculation_details: String,
}

/// Multiplicateur de récompense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardMultiplier {
    /// Type de multiplicateur
    pub multiplier_type: MultiplierType,
    /// Valeur du multiplicateur
    pub value: f64,
    /// Justification
    pub reason: String,
}

/// Bonus de récompense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardBonus {
    /// Type de bonus
    pub bonus_type: BonusType,
    /// Montant du bonus
    pub amount: u64,
    /// Justification
    pub reason: String,
}

/// Critères pour une distribution de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCriteria {
    /// Période couverte
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Critères spécifiques selon le type
    pub specific_criteria: serde_json::Value,
    /// Seuils minimums
    pub minimum_thresholds: HashMap<String, f64>,
}

/// Enregistrement de distribution pour un pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolDistributionRecord {
    /// Date de distribution
    pub date: DateTime<Utc>,
    /// Montant distribué
    pub amount: u64,
    /// Nombre de bénéficiaires
    pub recipient_count: usize,
    /// Montant moyen par bénéficiaire
    pub average_amount: u64,
    /// Critères utilisés
    pub criteria_hash: Hash,
}

/// Métriques de performance du système de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Total des récompenses distribuées
    pub total_distributed: u64,
    /// Distributions ce mois
    pub distributions_this_month: u32,
    /// Nombre de bénéficiaires uniques
    pub unique_recipients: usize,
    /// Récompense moyenne par bénéficiaire
    pub average_reward_per_recipient: u64,
    /// Temps moyen de traitement des récompenses
    pub average_processing_time_ms: u64,
    /// Taux de succès des distributions
    pub distribution_success_rate: f64,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Configuration du système de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardConfig {
    /// Fréquence des distributions (en heures)
    pub distribution_frequency_hours: u32,
    /// Pourcentage maximum du pool par distribution
    pub max_pool_percentage_per_distribution: f64,
    /// Nombre minimum de bénéficiaires par distribution
    pub min_recipients_per_distribution: usize,
    /// Délai d'attente pour les réclamations (en jours)
    pub claim_timeout_days: u32,
    /// Activation du système adaptatif
    pub adaptive_rewards_enabled: bool,
    /// Seuils de qualité minimums
    pub quality_thresholds: QualityThresholds,
}

/// Seuils de qualité pour différents types de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityThresholds {
    /// Seuil minimum pour archivage
    pub minimum_archive_quality: f64,
    /// Seuil minimum pour stockage
    pub minimum_storage_reliability: f64,
    /// Seuil minimum pour bande passante
    pub minimum_bandwidth_performance: f64,
    /// Seuil minimum pour découverte
    pub minimum_discovery_relevance: f64,
}

/// Types de récompenses
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum RewardType {
    /// Archivage initial d'un contenu
    InitialArchiving,
    /// Stockage continu
    ContinuousStorage,
    /// Service de bande passante
    BandwidthService,
    /// Découverte de contenu
    ContentDiscovery,
}

/// Types de multiplicateurs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MultiplierType {
    /// Multiplicateur de qualité
    Quality,
    /// Multiplicateur de performance
    Performance,
    /// Multiplicateur de rareté
    Rarity,
    /// Multiplicateur d'importance
    Importance,
    /// Multiplicateur de longévité
    Longevity,
}

/// Types de bonus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BonusType {
    /// Bonus de rareté du contenu
    RarityBonus,
    /// Bonus de première découverte
    FirstDiscoveryBonus,
    /// Bonus de performance exceptionnelle
    ExceptionalPerformanceBonus,
    /// Bonus de longue durée
    LongDurationBonus,
}

impl Default for EconomicModel {
    fn default() -> Self {
        Self {
            base_archive_reward: 100,                    // 100 ARC de base
            max_quality_multiplier: 5.0,                 // Jusqu'à 5x pour qualité
            rarity_bonus: 100,                           // 100 ARC pour contenu rare
            base_storage_rate_per_tb: 10,                // 10 ARC/TB/mois
            max_storage_performance_multiplier: 5.0,     // Jusqu'à 5x pour performance
            base_bandwidth_rate_per_gb: 1,               // 1 ARC/GB
            max_bandwidth_performance_multiplier: 5.0,   // Jusqu'à 5x pour performance
            base_discovery_reward: 25,                   // 25 ARC de base
            max_discovery_importance_multiplier: 4.0,    // Jusqu'à 4x pour importance
        }
    }
}

impl Default for RewardConfig {
    fn default() -> Self {
        Self {
            distribution_frequency_hours: 24,            // Distributions quotidiennes
            max_pool_percentage_per_distribution: 0.05,  // Max 5% du pool par distribution
            min_recipients_per_distribution: 1,          // Au moins 1 bénéficiaire
            claim_timeout_days: 30,                      // 30 jours pour réclamer
            adaptive_rewards_enabled: true,              // Système adaptatif activé
            quality_thresholds: QualityThresholds {
                minimum_archive_quality: 0.8,            // 80% minimum
                minimum_storage_reliability: 0.95,       // 95% minimum
                minimum_bandwidth_performance: 0.9,      // 90% minimum
                minimum_discovery_relevance: 0.7,        // 70% minimum
            },
        }
    }
}

impl RewardSystem {
    /// Crée un nouveau système de récompenses
    pub fn new(total_reward_allocation: u64, config: RewardConfig) -> Self {
        let economic_model = EconomicModel::default();
        
        // Répartition des allocations (ajustable selon les besoins)
        let archival_allocation = total_reward_allocation * 40 / 100;  // 40%
        let storage_allocation = total_reward_allocation * 30 / 100;   // 30%
        let bandwidth_allocation = total_reward_allocation * 20 / 100; // 20%
        let discovery_allocation = total_reward_allocation * 10 / 100; // 10%

        let now = Utc::now();
        let period_duration = Duration::hours(config.distribution_frequency_hours as i64);

        Self {
            archival_pool: RewardPool::new(RewardType::InitialArchiving, archival_allocation, period_duration),
            storage_pool: RewardPool::new(RewardType::ContinuousStorage, storage_allocation, period_duration),
            bandwidth_pool: RewardPool::new(RewardType::BandwidthService, bandwidth_allocation, period_duration),
            discovery_pool: RewardPool::new(RewardType::ContentDiscovery, discovery_allocation, period_duration),
            economic_model,
            distribution_history: Vec::new(),
            performance_metrics: PerformanceMetrics::new(),
            config,
            created_at: now,
            last_updated: now,
        }
    }

    /// Calcule et distribue les récompenses d'archivage initial
    pub fn distribute_archival_rewards(&mut self, contributions: Vec<ArchivalContribution>, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<RewardDistribution> {
        let mut recipients = HashMap::new();
        let mut total_amount = 0;

        for contribution in contributions {
            if contribution.quality_score < self.config.quality_thresholds.minimum_archive_quality {
                continue; // Skip contributions below quality threshold
            }

            let allocation = self.calculate_archival_reward(&contribution)?;
            recipients.insert(contribution.contributor.clone(), allocation.clone());
            total_amount += allocation.final_amount;

            // Mint tokens to contributor
            token.mint(&contribution.contributor, allocation.final_amount, tx_hash)?;
        }

        // Update pool
        self.archival_pool.distributed_amount += total_amount;
        self.archival_pool.available_amount = self.archival_pool.available_amount.saturating_sub(total_amount);
        self.archival_pool.distributed_this_period += total_amount;

        // Create distribution record
        let distribution = RewardDistribution {
            distribution_id: Hash::from_bytes([
                &tx_hash.as_bytes()[..16],
                &Utc::now().timestamp().to_le_bytes(),
            ].concat().try_into().unwrap()),
            reward_type: RewardType::InitialArchiving,
            recipients,
            total_amount,
            criteria: RewardCriteria {
                period_start: Utc::now() - Duration::hours(24),
                period_end: Utc::now(),
                specific_criteria: serde_json::json!({
                    "minimum_quality": self.config.quality_thresholds.minimum_archive_quality
                }),
                minimum_thresholds: HashMap::new(),
            },
            distribution_date: Utc::now(),
            transaction_hash: tx_hash,
        };

        self.distribution_history.push(distribution.clone());
        self.update_performance_metrics();
        self.last_updated = Utc::now();

        Ok(distribution)
    }

    /// Calcule les récompenses de stockage continu
    pub fn distribute_storage_rewards(&mut self, contributions: Vec<StorageContribution>, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<RewardDistribution> {
        let mut recipients = HashMap::new();
        let mut total_amount = 0;

        for contribution in contributions {
            if contribution.reliability_score < self.config.quality_thresholds.minimum_storage_reliability {
                continue;
            }

            let allocation = self.calculate_storage_reward(&contribution)?;
            recipients.insert(contribution.provider.clone(), allocation.clone());
            total_amount += allocation.final_amount;

            token.mint(&contribution.provider, allocation.final_amount, tx_hash)?;
        }

        self.storage_pool.distributed_amount += total_amount;
        self.storage_pool.available_amount = self.storage_pool.available_amount.saturating_sub(total_amount);
        self.storage_pool.distributed_this_period += total_amount;

        let distribution = RewardDistribution {
            distribution_id: Hash::from_bytes([
                &tx_hash.as_bytes()[..16],
                &Utc::now().timestamp().to_le_bytes(),
                &[1u8], // Different from archival
            ].concat().try_into().unwrap()),
            reward_type: RewardType::ContinuousStorage,
            recipients,
            total_amount,
            criteria: RewardCriteria {
                period_start: Utc::now() - Duration::days(30), // Monthly storage rewards
                period_end: Utc::now(),
                specific_criteria: serde_json::json!({
                    "minimum_reliability": self.config.quality_thresholds.minimum_storage_reliability
                }),
                minimum_thresholds: HashMap::new(),
            },
            distribution_date: Utc::now(),
            transaction_hash: tx_hash,
        };

        self.distribution_history.push(distribution.clone());
        self.update_performance_metrics();
        self.last_updated = Utc::now();

        Ok(distribution)
    }

    /// Calcule les récompenses de bande passante
    pub fn distribute_bandwidth_rewards(&mut self, contributions: Vec<BandwidthContribution>, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<RewardDistribution> {
        let mut recipients = HashMap::new();
        let mut total_amount = 0;

        for contribution in contributions {
            if contribution.performance_score < self.config.quality_thresholds.minimum_bandwidth_performance {
                continue;
            }

            let allocation = self.calculate_bandwidth_reward(&contribution)?;
            recipients.insert(contribution.provider.clone(), allocation.clone());
            total_amount += allocation.final_amount;

            token.mint(&contribution.provider, allocation.final_amount, tx_hash)?;
        }

        self.bandwidth_pool.distributed_amount += total_amount;
        self.bandwidth_pool.available_amount = self.bandwidth_pool.available_amount.saturating_sub(total_amount);
        self.bandwidth_pool.distributed_this_period += total_amount;

        let distribution = RewardDistribution {
            distribution_id: Hash::from_bytes([
                &tx_hash.as_bytes()[..16],
                &Utc::now().timestamp().to_le_bytes(),
                &[2u8], // Different from others
            ].concat().try_into().unwrap()),
            reward_type: RewardType::BandwidthService,
            recipients,
            total_amount,
            criteria: RewardCriteria {
                period_start: Utc::now() - Duration::hours(24),
                period_end: Utc::now(),
                specific_criteria: serde_json::json!({
                    "minimum_performance": self.config.quality_thresholds.minimum_bandwidth_performance
                }),
                minimum_thresholds: HashMap::new(),
            },
            distribution_date: Utc::now(),
            transaction_hash: tx_hash,
        };

        self.distribution_history.push(distribution.clone());
        self.update_performance_metrics();
        self.last_updated = Utc::now();

        Ok(distribution)
    }

    /// Calcule les récompenses de découverte
    pub fn distribute_discovery_rewards(&mut self, contributions: Vec<DiscoveryContribution>, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<RewardDistribution> {
        let mut recipients = HashMap::new();
        let mut total_amount = 0;

        for contribution in contributions {
            if contribution.relevance_score < self.config.quality_thresholds.minimum_discovery_relevance {
                continue;
            }

            let allocation = self.calculate_discovery_reward(&contribution)?;
            recipients.insert(contribution.discoverer.clone(), allocation.clone());
            total_amount += allocation.final_amount;

            token.mint(&contribution.discoverer, allocation.final_amount, tx_hash)?;
        }

        self.discovery_pool.distributed_amount += total_amount;
        self.discovery_pool.available_amount = self.discovery_pool.available_amount.saturating_sub(total_amount);
        self.discovery_pool.distributed_this_period += total_amount;

        let distribution = RewardDistribution {
            distribution_id: Hash::from_bytes([
                &tx_hash.as_bytes()[..16],
                &Utc::now().timestamp().to_le_bytes(),
                &[3u8], // Different from others
            ].concat().try_into().unwrap()),
            reward_type: RewardType::ContentDiscovery,
            recipients,
            total_amount,
            criteria: RewardCriteria {
                period_start: Utc::now() - Duration::hours(24),
                period_end: Utc::now(),
                specific_criteria: serde_json::json!({
                    "minimum_relevance": self.config.quality_thresholds.minimum_discovery_relevance
                }),
                minimum_thresholds: HashMap::new(),
            },
            distribution_date: Utc::now(),
            transaction_hash: tx_hash,
        };

        self.distribution_history.push(distribution.clone());
        self.update_performance_metrics();
        self.last_updated = Utc::now();

        Ok(distribution)
    }

    /// Calcule la récompense d'archivage pour une contribution
    fn calculate_archival_reward(&self, contribution: &ArchivalContribution) -> TokenOperationResult<RewardAllocation> {
        let mut base_amount = self.economic_model.base_archive_reward;
        let mut multipliers = Vec::new();
        let mut bonuses = Vec::new();

        // Multiplicateur de qualité (1.0 à 5.0)
        let quality_multiplier = 1.0 + (contribution.quality_score - 0.5) * (self.economic_model.max_quality_multiplier - 1.0) / 0.5;
        let quality_multiplier = quality_multiplier.clamp(1.0, self.economic_model.max_quality_multiplier);
        
        multipliers.push(RewardMultiplier {
            multiplier_type: MultiplierType::Quality,
            value: quality_multiplier,
            reason: format!("Qualité: {:.1}%", contribution.quality_score * 100.0),
        });

        // Bonus de rareté
        if contribution.is_rare_content {
            bonuses.push(RewardBonus {
                bonus_type: BonusType::RarityBonus,
                amount: self.economic_model.rarity_bonus,
                reason: "Contenu rare identifié".to_string(),
            });
        }

        // Calcul final
        let multiplied_amount = (base_amount as f64 * quality_multiplier) as u64;
        let bonus_amount: u64 = bonuses.iter().map(|b| b.amount).sum();
        let final_amount = multiplied_amount + bonus_amount;

        Ok(RewardAllocation {
            recipient: contribution.contributor.clone(),
            base_amount,
            multipliers,
            bonuses,
            final_amount,
            calculation_details: format!(
                "Base: {} ARC × {:.2} (qualité) + {} ARC (bonus) = {} ARC",
                base_amount, quality_multiplier, bonus_amount, final_amount
            ),
        })
    }

    /// Calcule la récompense de stockage pour une contribution
    fn calculate_storage_reward(&self, contribution: &StorageContribution) -> TokenOperationResult<RewardAllocation> {
        let tb_stored = contribution.storage_capacity_bytes as f64 / (1024.0 * 1024.0 * 1024.0 * 1024.0); // Convert to TB
        let base_amount = (tb_stored * self.economic_model.base_storage_rate_per_tb as f64) as u64;
        
        let mut multipliers = Vec::new();
        let mut bonuses = Vec::new();

        // Multiplicateur de performance
        let performance_multiplier = 1.0 + (contribution.reliability_score - 0.8) * (self.economic_model.max_storage_performance_multiplier - 1.0) / 0.2;
        let performance_multiplier = performance_multiplier.clamp(1.0, self.economic_model.max_storage_performance_multiplier);

        multipliers.push(RewardMultiplier {
            multiplier_type: MultiplierType::Performance,
            value: performance_multiplier,
            reason: format!("Fiabilité: {:.1}%", contribution.reliability_score * 100.0),
        });

        // Bonus de longue durée (plus de 6 mois)
        if contribution.storage_duration_days > 180 {
            let duration_bonus = (contribution.storage_duration_days - 180) * base_amount / 365; // Bonus progressif
            bonuses.push(RewardBonus {
                bonus_type: BonusType::LongDurationBonus,
                amount: duration_bonus,
                reason: format!("Stockage longue durée: {} jours", contribution.storage_duration_days),
            });
        }

        let multiplied_amount = (base_amount as f64 * performance_multiplier) as u64;
        let bonus_amount: u64 = bonuses.iter().map(|b| b.amount).sum();
        let final_amount = multiplied_amount + bonus_amount;

        Ok(RewardAllocation {
            recipient: contribution.provider.clone(),
            base_amount,
            multipliers,
            bonuses,
            final_amount,
            calculation_details: format!(
                "{:.2} TB × {} ARC/TB × {:.2} (performance) + {} ARC (bonus) = {} ARC",
                tb_stored, self.economic_model.base_storage_rate_per_tb, performance_multiplier, bonus_amount, final_amount
            ),
        })
    }

    /// Calcule la récompense de bande passante pour une contribution
    fn calculate_bandwidth_reward(&self, contribution: &BandwidthContribution) -> TokenOperationResult<RewardAllocation> {
        let gb_served = contribution.bytes_served as f64 / (1024.0 * 1024.0 * 1024.0); // Convert to GB
        let base_amount = (gb_served * self.economic_model.base_bandwidth_rate_per_gb as f64) as u64;
        
        let mut multipliers = Vec::new();
        let mut bonuses = Vec::new();

        // Multiplicateur de performance
        let performance_multiplier = 1.0 + (contribution.performance_score - 0.8) * (self.economic_model.max_bandwidth_performance_multiplier - 1.0) / 0.2;
        let performance_multiplier = performance_multiplier.clamp(1.0, self.economic_model.max_bandwidth_performance_multiplier);

        multipliers.push(RewardMultiplier {
            multiplier_type: MultiplierType::Performance,
            value: performance_multiplier,
            reason: format!("Performance: {:.1}%", contribution.performance_score * 100.0),
        });

        // Bonus pour performance exceptionnelle (>95%)
        if contribution.performance_score > 0.95 {
            let exceptional_bonus = base_amount / 10; // 10% bonus
            bonuses.push(RewardBonus {
                bonus_type: BonusType::ExceptionalPerformanceBonus,
                amount: exceptional_bonus,
                reason: "Performance exceptionnelle (>95%)".to_string(),
            });
        }

        let multiplied_amount = (base_amount as f64 * performance_multiplier) as u64;
        let bonus_amount: u64 = bonuses.iter().map(|b| b.amount).sum();
        let final_amount = multiplied_amount + bonus_amount;

        Ok(RewardAllocation {
            recipient: contribution.provider.clone(),
            base_amount,
            multipliers,
            bonuses,
            final_amount,
            calculation_details: format!(
                "{:.2} GB × {} ARC/GB × {:.2} (performance) + {} ARC (bonus) = {} ARC",
                gb_served, self.economic_model.base_bandwidth_rate_per_gb, performance_multiplier, bonus_amount, final_amount
            ),
        })
    }

    /// Calcule la récompense de découverte pour une contribution
    fn calculate_discovery_reward(&self, contribution: &DiscoveryContribution) -> TokenOperationResult<RewardAllocation> {
        let base_amount = self.economic_model.base_discovery_reward;
        let mut multipliers = Vec::new();
        let mut bonuses = Vec::new();

        // Multiplicateur d'importance
        let importance_multiplier = 1.0 + (contribution.importance_factor - 0.5) * (self.economic_model.max_discovery_importance_multiplier - 1.0) / 0.5;
        let importance_multiplier = importance_multiplier.clamp(1.0, self.economic_model.max_discovery_importance_multiplier);

        multipliers.push(RewardMultiplier {
            multiplier_type: MultiplierType::Importance,
            value: importance_multiplier,
            reason: format!("Importance: {:.1}", contribution.importance_factor),
        });

        // Bonus de première découverte
        if contribution.is_first_discovery {
            bonuses.push(RewardBonus {
                bonus_type: BonusType::FirstDiscoveryBonus,
                amount: base_amount / 2, // 50% bonus
                reason: "Première découverte".to_string(),
            });
        }

        let multiplied_amount = (base_amount as f64 * importance_multiplier) as u64;
        let bonus_amount: u64 = bonuses.iter().map(|b| b.amount).sum();
        let final_amount = multiplied_amount + bonus_amount;

        Ok(RewardAllocation {
            recipient: contribution.discoverer.clone(),
            base_amount,
            multipliers,
            bonuses,
            final_amount,
            calculation_details: format!(
                "Base: {} ARC × {:.2} (importance) + {} ARC (bonus) = {} ARC",
                base_amount, importance_multiplier, bonus_amount, final_amount
            ),
        })
    }

    /// Met à jour les métriques de performance
    fn update_performance_metrics(&mut self) {
        let total_distributed = self.archival_pool.distributed_amount + 
                              self.storage_pool.distributed_amount + 
                              self.bandwidth_pool.distributed_amount + 
                              self.discovery_pool.distributed_amount;

        let unique_recipients: std::collections::HashSet<PublicKey> = self.distribution_history
            .iter()
            .flat_map(|d| d.recipients.keys())
            .cloned()
            .collect();

        self.performance_metrics.total_distributed = total_distributed;
        self.performance_metrics.unique_recipients = unique_recipients.len();
        self.performance_metrics.average_reward_per_recipient = if unique_recipients.len() > 0 {
            total_distributed / unique_recipients.len() as u64
        } else {
            0
        };
        self.performance_metrics.last_updated = Utc::now();
    }

    /// Obtient les statistiques du système
    pub fn get_system_statistics(&self) -> RewardSystemStatistics {
        RewardSystemStatistics {
            total_allocated: self.archival_pool.total_allocation + 
                           self.storage_pool.total_allocation + 
                           self.bandwidth_pool.total_allocation + 
                           self.discovery_pool.total_allocation,
            total_distributed: self.performance_metrics.total_distributed,
            pools_status: vec![
                PoolStatus { reward_type: RewardType::InitialArchiving, available: self.archival_pool.available_amount, distributed: self.archival_pool.distributed_amount },
                PoolStatus { reward_type: RewardType::ContinuousStorage, available: self.storage_pool.available_amount, distributed: self.storage_pool.distributed_amount },
                PoolStatus { reward_type: RewardType::BandwidthService, available: self.bandwidth_pool.available_amount, distributed: self.bandwidth_pool.distributed_amount },
                PoolStatus { reward_type: RewardType::ContentDiscovery, available: self.discovery_pool.available_amount, distributed: self.discovery_pool.distributed_amount },
            ],
            performance_metrics: self.performance_metrics.clone(),
        }
    }
}

impl RewardPool {
    fn new(reward_type: RewardType, total_allocation: u64, period_duration: Duration) -> Self {
        let period_limit = total_allocation / 100; // 1% par période par défaut
        
        Self {
            reward_type,
            total_allocation,
            distributed_amount: 0,
            available_amount: total_allocation,
            period_limit,
            distributed_this_period: 0,
            period_reset_date: Utc::now() + period_duration,
            distribution_records: Vec::new(),
        }
    }
}

impl PerformanceMetrics {
    fn new() -> Self {
        Self {
            total_distributed: 0,
            distributions_this_month: 0,
            unique_recipients: 0,
            average_reward_per_recipient: 0,
            average_processing_time_ms: 0,
            distribution_success_rate: 1.0,
            last_updated: Utc::now(),
        }
    }
}

/// Contribution d'archivage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchivalContribution {
    pub contributor: PublicKey,
    pub content_hash: Hash,
    pub content_size_bytes: u64,
    pub quality_score: f64,
    pub is_rare_content: bool,
    pub archive_date: DateTime<Utc>,
}

/// Contribution de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageContribution {
    pub provider: PublicKey,
    pub storage_capacity_bytes: u64,
    pub reliability_score: f64,
    pub storage_duration_days: u64,
    pub uptime_percentage: f64,
}

/// Contribution de bande passante
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BandwidthContribution {
    pub provider: PublicKey,
    pub bytes_served: u64,
    pub performance_score: f64,
    pub average_response_time_ms: u64,
    pub error_rate: f64,
}

/// Contribution de découverte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryContribution {
    pub discoverer: PublicKey,
    pub discovered_content_hash: Hash,
    pub relevance_score: f64,
    pub importance_factor: f64,
    pub is_first_discovery: bool,
    pub discovery_date: DateTime<Utc>,
}

/// Statistiques du système de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSystemStatistics {
    pub total_allocated: u64,
    pub total_distributed: u64,
    pub pools_status: Vec<PoolStatus>,
    pub performance_metrics: PerformanceMetrics,
}

/// Statut d'un pool de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStatus {
    pub reward_type: RewardType,
    pub available: u64,
    pub distributed: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_reward_system_creation() {
        let config = RewardConfig::default();
        let system = RewardSystem::new(1_000_000, config);
        
        assert_eq!(system.archival_pool.total_allocation, 400_000);
        assert_eq!(system.storage_pool.total_allocation, 300_000);
        assert_eq!(system.bandwidth_pool.total_allocation, 200_000);
        assert_eq!(system.discovery_pool.total_allocation, 100_000);
    }

    #[test]
    fn test_archival_reward_calculation() {
        let config = RewardConfig::default();
        let system = RewardSystem::new(1_000_000, config);
        let keypair = generate_keypair().unwrap();
        
        let contribution = ArchivalContribution {
            contributor: keypair.public_key().clone(),
            content_hash: Hash::zero(),
            content_size_bytes: 1024 * 1024, // 1MB
            quality_score: 0.9, // High quality
            is_rare_content: true,
            archive_date: Utc::now(),
        };

        let allocation = system.calculate_archival_reward(&contribution).unwrap();
        
        // Should have base reward + quality multiplier + rarity bonus
        assert!(allocation.final_amount > system.economic_model.base_archive_reward);
        assert!(allocation.final_amount <= 500); // Max selon specs (100-500 ARC)
        assert_eq!(allocation.bonuses.len(), 1); // Rarity bonus
        assert_eq!(allocation.multipliers.len(), 1); // Quality multiplier
    }

    #[test]
    fn test_storage_reward_calculation() {
        let config = RewardConfig::default();
        let system = RewardSystem::new(1_000_000, config);
        let keypair = generate_keypair().unwrap();
        
        let contribution = StorageContribution {
            provider: keypair.public_key().clone(),
            storage_capacity_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
            reliability_score: 0.98,
            storage_duration_days: 200, // Long duration
            uptime_percentage: 99.9,
        };

        let allocation = system.calculate_storage_reward(&contribution).unwrap();
        
        // Should have base rate per TB + performance multiplier + duration bonus
        assert!(allocation.final_amount >= 10); // Base 10 ARC/TB
        assert_eq!(allocation.bonuses.len(), 1); // Duration bonus
        assert_eq!(allocation.multipliers.len(), 1); // Performance multiplier
    }

    #[test]
    fn test_bandwidth_reward_calculation() {
        let config = RewardConfig::default();
        let system = RewardSystem::new(1_000_000, config);
        let keypair = generate_keypair().unwrap();
        
        let contribution = BandwidthContribution {
            provider: keypair.public_key().clone(),
            bytes_served: 5 * 1024 * 1024 * 1024, // 5GB
            performance_score: 0.96, // Exceptional performance
            average_response_time_ms: 50,
            error_rate: 0.01,
        };

        let allocation = system.calculate_bandwidth_reward(&contribution).unwrap();
        
        // Should have base rate per GB + performance multiplier + exceptional bonus
        assert!(allocation.final_amount >= 5); // Base 5 ARC for 5GB
        assert_eq!(allocation.bonuses.len(), 1); // Exceptional performance bonus
        assert_eq!(allocation.multipliers.len(), 1); // Performance multiplier
    }

    #[test]
    fn test_discovery_reward_calculation() {
        let config = RewardConfig::default();
        let system = RewardSystem::new(1_000_000, config);
        let keypair = generate_keypair().unwrap();
        
        let contribution = DiscoveryContribution {
            discoverer: keypair.public_key().clone(),
            discovered_content_hash: Hash::zero(),
            relevance_score: 0.8,
            importance_factor: 0.9, // High importance
            is_first_discovery: true,
            discovery_date: Utc::now(),
        };

        let allocation = system.calculate_discovery_reward(&contribution).unwrap();
        
        // Should have base reward + importance multiplier + first discovery bonus
        assert!(allocation.final_amount >= 25); // Base 25 ARC
        assert!(allocation.final_amount <= 100); // Max selon specs (25-100 ARC)
        assert_eq!(allocation.bonuses.len(), 1); // First discovery bonus
        assert_eq!(allocation.multipliers.len(), 1); // Importance multiplier
    }
}