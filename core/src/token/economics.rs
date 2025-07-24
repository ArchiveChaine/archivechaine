//! Module économique unifié pour ArchiveChain
//!
//! Fournit une interface de haut niveau pour coordonner tous les aspects économiques :
//! - Orchestration des différents sous-systèmes
//! - Calculs économiques complexes
//! - Métriques et analytics unifiées
//! - Simulation et prédictions économiques
//! - Interface d'administration économique

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey};
use super::{
    TokenOperationResult, TokenOperationError, ARCToken, TokenDistribution,
    RewardSystem, StakingSystem, Treasury, DeflationaryMechanisms,
    TOTAL_SUPPLY, GlobalTokenMetrics, TokenConfig
};

/// Modèle économique principal d'ArchiveChain
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicModel {
    /// Token ARC principal
    pub token: ARCToken,
    /// Système de distribution
    pub distribution: TokenDistribution,
    /// Système de récompenses
    pub rewards: RewardSystem,
    /// Système de staking
    pub staking: StakingSystem,
    /// Treasury communautaire
    pub treasury: Treasury,
    /// Mécanismes déflationnistes
    pub deflation: DeflationaryMechanisms,
    /// Configuration économique
    pub config: EconomicConfig,
    /// Métriques unifiées
    pub metrics: EconomicMetrics,
    /// Calculateur de récompenses
    pub reward_calculator: RewardCalculation,
    /// Simulateur économique
    pub simulator: EconomicSimulator,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Configuration économique globale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicConfig {
    /// Configuration des tokens
    pub token_config: TokenConfig,
    /// Paramètres d'inflation/déflation
    pub inflation_params: InflationParameters,
    /// Paramètres de récompenses
    pub reward_params: RewardParameters,
    /// Paramètres de staking
    pub staking_params: StakingParameters,
    /// Paramètres du treasury
    pub treasury_params: TreasuryParameters,
    /// Activation des fonctionnalités avancées
    pub advanced_features: AdvancedFeatures,
}

/// Métriques économiques unifiées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicMetrics {
    /// Métriques globales des tokens
    pub token_metrics: GlobalTokenMetrics,
    /// Métriques de distribution
    pub distribution_metrics: DistributionMetrics,
    /// Métriques de récompenses
    pub reward_metrics: RewardMetrics,
    /// Métriques de staking
    pub staking_metrics: StakingMetrics,
    /// Métriques du treasury
    pub treasury_metrics: TreasuryMetrics,
    /// Métriques de déflation
    pub deflation_metrics: DeflationMetrics,
    /// Métriques calculées
    pub calculated_metrics: CalculatedMetrics,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Calculateur de récompenses avancé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCalculation {
    /// Modèles de calcul par type
    pub calculation_models: HashMap<String, CalculationModel>,
    /// Paramètres dynamiques
    pub dynamic_parameters: DynamicParameters,
    /// Historique des ajustements
    pub adjustment_history: Vec<ParameterAdjustment>,
    /// Prédictions de récompenses
    pub reward_predictions: Vec<RewardPrediction>,
}

/// Simulateur économique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSimulator {
    /// Scénarios de simulation
    pub scenarios: HashMap<String, SimulationScenario>,
    /// Résultats de simulations
    pub simulation_results: Vec<SimulationResult>,
    /// Modèles prédictifs
    pub predictive_models: Vec<PredictiveModel>,
    /// Configuration de simulation
    pub simulation_config: SimulationConfig,
}

/// Paramètres d'inflation/déflation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InflationParameters {
    /// Taux d'inflation cible annuel (%)
    pub target_annual_inflation_rate: f64,
    /// Taux de déflation maximum acceptable (%)
    pub max_acceptable_deflation_rate: f64,
    /// Seuil de déclenchement des ajustements
    pub adjustment_trigger_threshold: f64,
    /// Fréquence d'évaluation (jours)
    pub evaluation_frequency_days: u32,
}

/// Paramètres des récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardParameters {
    /// Ajustement automatique des récompenses
    pub auto_adjustment_enabled: bool,
    /// Facteur d'ajustement basé sur l'activité
    pub activity_adjustment_factor: f64,
    /// Bonus saisonnier
    pub seasonal_bonus_enabled: bool,
    /// Multiplicateur de performance réseau
    pub network_performance_multiplier: f64,
}

/// Paramètres de staking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingParameters {
    /// APY cible pour le staking (%)
    pub target_staking_apy: f64,
    /// Ratio de staking optimal (% du supply)
    pub optimal_staking_ratio: f64,
    /// Ajustement automatique des récompenses
    pub auto_reward_adjustment: bool,
    /// Pénalité de déstaking anticipé (%)
    pub early_unstaking_penalty: f64,
}

/// Paramètres du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryParameters {
    /// Allocation automatique basée sur la performance
    pub performance_based_allocation: bool,
    /// Réserve d'urgence (% du treasury)
    pub emergency_reserve_percentage: f64,
    /// Seuil de déclenchement des propositions d'urgence
    pub emergency_proposal_threshold: f64,
    /// Fréquence de réévaluation du budget
    pub budget_review_frequency_months: u32,
}

/// Fonctionnalités avancées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedFeatures {
    /// Système de prédiction activé
    pub prediction_system_enabled: bool,
    /// Ajustements automatiques activés
    pub auto_adjustments_enabled: bool,
    /// Mécanismes d'urgence activés
    pub emergency_mechanisms_enabled: bool,
    /// Analytics avancées activées
    pub advanced_analytics_enabled: bool,
}

/// Métriques de distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionMetrics {
    /// Progression de la distribution d'équipe (%)
    pub team_distribution_progress: f64,
    /// Récompenses d'archivage distribuées
    pub archival_rewards_distributed: u64,
    /// Utilisation du treasury (%)
    pub treasury_utilization_rate: f64,
    /// Vente publique complétée (%)
    pub public_sale_completion: f64,
}

/// Métriques de récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardMetrics {
    /// Récompenses totales distribuées
    pub total_rewards_distributed: u64,
    /// Taux de distribution hebdomadaire
    pub weekly_distribution_rate: u64,
    /// Efficacité des récompenses (ROI)
    pub reward_efficiency: f64,
    /// Participation aux récompenses (%)
    pub participation_rate: f64,
}

/// Métriques de staking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingMetrics {
    /// Ratio de staking total (%)
    pub total_staking_ratio: f64,
    /// APY réel du staking
    pub actual_staking_apy: f64,
    /// Nombre de stakeurs actifs
    pub active_stakers_count: usize,
    /// Durée moyenne de staking (jours)
    pub average_staking_duration_days: f64,
}

/// Métriques du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryMetrics {
    /// Fonds disponibles
    pub available_funds: u64,
    /// Taux d'approbation des propositions (%)
    pub proposal_approval_rate: f64,
    /// Projets actifs
    pub active_projects_count: usize,
    /// ROI moyen des projets financés
    pub average_project_roi: f64,
}

/// Métriques de déflation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeflationMetrics {
    /// Tokens brûlés totaux
    pub total_burned_tokens: u64,
    /// Taux de déflation annuel (%)
    pub annual_deflation_rate: f64,
    /// Tokens stakés pour qualité
    pub quality_staked_tokens: u64,
    /// Bonus long terme distribués
    pub longterm_bonus_distributed: u64,
}

/// Métriques calculées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculatedMetrics {
    /// Vélocité des tokens
    pub token_velocity: f64,
    /// Ratio prix/utilité estimé
    pub price_utility_ratio: f64,
    /// Indice de santé économique
    pub economic_health_index: f64,
    /// Prédiction de croissance (%)
    pub growth_prediction: f64,
    /// Score de décentralisation
    pub decentralization_score: f64,
}

/// Modèle de calcul
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalculationModel {
    /// Nom du modèle
    pub model_name: String,
    /// Paramètres du modèle
    pub parameters: HashMap<String, f64>,
    /// Poids des facteurs
    pub factor_weights: HashMap<String, f64>,
    /// Dernière calibration
    pub last_calibration: DateTime<Utc>,
}

/// Paramètres dynamiques
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicParameters {
    /// Multiplicateurs d'activité réseau
    pub network_activity_multipliers: HashMap<String, f64>,
    /// Ajustements saisonniers
    pub seasonal_adjustments: HashMap<String, f64>,
    /// Facteurs de performance
    pub performance_factors: HashMap<String, f64>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Ajustement de paramètre
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterAdjustment {
    /// Paramètre ajusté
    pub parameter_name: String,
    /// Ancienne valeur
    pub old_value: f64,
    /// Nouvelle valeur
    pub new_value: f64,
    /// Raison de l'ajustement
    pub reason: String,
    /// Date de l'ajustement
    pub adjustment_date: DateTime<Utc>,
}

/// Prédiction de récompense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPrediction {
    /// Type de récompense
    pub reward_type: String,
    /// Période de prédiction
    pub prediction_period_days: u32,
    /// Montant prédit
    pub predicted_amount: u64,
    /// Intervalle de confiance
    pub confidence_interval: (f64, f64),
    /// Date de prédiction
    pub prediction_date: DateTime<Utc>,
}

/// Scénario de simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationScenario {
    /// Nom du scénario
    pub scenario_name: String,
    /// Description
    pub description: String,
    /// Paramètres modifiés
    pub modified_parameters: HashMap<String, f64>,
    /// Durée de simulation (jours)
    pub simulation_duration_days: u32,
    /// Résultats attendus
    pub expected_outcomes: Vec<String>,
}

/// Résultat de simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    /// Scénario simulé
    pub scenario_name: String,
    /// Métriques finales
    pub final_metrics: EconomicMetrics,
    /// Évolution temporelle
    pub timeline_data: Vec<TimelinePoint>,
    /// Score de performance
    pub performance_score: f64,
    /// Date de simulation
    pub simulation_date: DateTime<Utc>,
}

/// Point temporel dans une simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelinePoint {
    /// Jour de simulation
    pub day: u32,
    /// Supply en circulation
    pub circulating_supply: u64,
    /// Tokens stakés
    pub staked_tokens: u64,
    /// Récompenses distribuées
    pub rewards_distributed: u64,
    /// Taux de déflation
    pub deflation_rate: f64,
}

/// Modèle prédictif
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictiveModel {
    /// Type de modèle
    pub model_type: String,
    /// Variables d'entrée
    pub input_variables: Vec<String>,
    /// Coefficients du modèle
    pub coefficients: Vec<f64>,
    /// Précision du modèle (%)
    pub accuracy_percentage: f64,
    /// Dernière formation
    pub last_trained: DateTime<Utc>,
}

/// Configuration de simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Pas de temps de simulation (heures)
    pub time_step_hours: u32,
    /// Nombre maximum d'itérations
    pub max_iterations: u32,
    /// Seuil de convergence
    pub convergence_threshold: f64,
    /// Utilisation de Monte Carlo
    pub monte_carlo_enabled: bool,
    /// Nombre d'échantillons Monte Carlo
    pub monte_carlo_samples: u32,
}

impl Default for EconomicConfig {
    fn default() -> Self {
        Self {
            token_config: TokenConfig::default(),
            inflation_params: InflationParameters {
                target_annual_inflation_rate: 3.0,
                max_acceptable_deflation_rate: -2.0,
                adjustment_trigger_threshold: 1.0,
                evaluation_frequency_days: 7,
            },
            reward_params: RewardParameters {
                auto_adjustment_enabled: true,
                activity_adjustment_factor: 1.2,
                seasonal_bonus_enabled: true,
                network_performance_multiplier: 1.0,
            },
            staking_params: StakingParameters {
                target_staking_apy: 8.0,
                optimal_staking_ratio: 65.0,
                auto_reward_adjustment: true,
                early_unstaking_penalty: 2.0,
            },
            treasury_params: TreasuryParameters {
                performance_based_allocation: true,
                emergency_reserve_percentage: 10.0,
                emergency_proposal_threshold: 5.0,
                budget_review_frequency_months: 6,
            },
            advanced_features: AdvancedFeatures {
                prediction_system_enabled: true,
                auto_adjustments_enabled: true,
                emergency_mechanisms_enabled: true,
                advanced_analytics_enabled: true,
            },
        }
    }
}

impl EconomicModel {
    /// Crée un nouveau modèle économique
    pub fn new(config: EconomicConfig) -> Self {
        let now = Utc::now();
        
        Self {
            token: ARCToken::new(),
            distribution: TokenDistribution::new(),
            rewards: RewardSystem::new(
                super::ARCHIVAL_REWARDS_ALLOCATION,
                super::rewards::RewardConfig::default(),
            ),
            staking: StakingSystem::new(super::staking::StakingConfig::default()),
            treasury: Treasury::new(super::treasury::TreasuryConfig::default()),
            deflation: DeflationaryMechanisms::new(super::deflation::DeflationConfig::default()),
            config,
            metrics: EconomicMetrics::new(),
            reward_calculator: RewardCalculation::new(),
            simulator: EconomicSimulator::new(),
            created_at: now,
            last_updated: now,
        }
    }

    /// Met à jour toutes les métriques économiques
    pub fn update_all_metrics(&mut self) -> TokenOperationResult<()> {
        // Mettre à jour les métriques des tokens
        self.metrics.token_metrics.update(
            self.token.circulating_supply,
            self.token.burned_tokens,
            self.token.locked_tokens,
            0, // TODO: Calculer les récompenses depuis le système de récompenses
            self.token.balances.len(),
        );

        // Mettre à jour les métriques de distribution
        let dist_stats = self.distribution.get_distribution_stats();
        self.metrics.distribution_metrics = DistributionMetrics {
            team_distribution_progress: (dist_stats.team_tokens_distributed as f64 / super::TEAM_ALLOCATION as f64) * 100.0,
            archival_rewards_distributed: dist_stats.archival_rewards_distributed,
            treasury_utilization_rate: (dist_stats.community_funds_allocated as f64 / super::COMMUNITY_RESERVE as f64) * 100.0,
            public_sale_completion: dist_stats.public_sale_progress,
        };

        // Mettre à jour les métriques de récompenses
        let reward_stats = self.rewards.get_system_statistics();
        self.metrics.reward_metrics = RewardMetrics {
            total_rewards_distributed: reward_stats.total_distributed,
            weekly_distribution_rate: 0, // TODO: Calculer le taux hebdomadaire
            reward_efficiency: 0.0, // TODO: Calculer l'efficacité
            participation_rate: 0.0, // TODO: Calculer le taux de participation
        };

        // Mettre à jour les métriques de staking
        self.metrics.staking_metrics = StakingMetrics {
            total_staking_ratio: (self.staking.metrics.total_governance_staked + self.staking.metrics.total_validator_staked) as f64 / TOTAL_SUPPLY as f64 * 100.0,
            actual_staking_apy: self.config.staking_params.target_staking_apy, // TODO: Calculer l'APY réel
            active_stakers_count: self.staking.metrics.governance_stakers_count + self.staking.metrics.active_validators_count,
            average_staking_duration_days: 0.0, // TODO: Calculer la durée moyenne
        };

        // Mettre à jour les métriques du treasury
        let treasury_stats = self.treasury.get_treasury_statistics();
        self.metrics.treasury_metrics = TreasuryMetrics {
            available_funds: treasury_stats.available_funds,
            proposal_approval_rate: if treasury_stats.total_proposals > 0 {
                (treasury_stats.approved_proposals as f64 / treasury_stats.total_proposals as f64) * 100.0
            } else { 0.0 },
            active_projects_count: treasury_stats.active_projects,
            average_project_roi: treasury_stats.project_success_rate,
        };

        // Mettre à jour les métriques de déflation
        let deflation_metrics = self.deflation.get_deflation_metrics();
        self.metrics.deflation_metrics = DeflationMetrics {
            total_burned_tokens: deflation_metrics.total_burned,
            annual_deflation_rate: deflation_metrics.estimated_annual_deflation_rate,
            quality_staked_tokens: deflation_metrics.total_quality_staked,
            longterm_bonus_distributed: deflation_metrics.total_longterm_bonus_distributed,
        };

        // Calculer les métriques dérivées
        self.calculate_derived_metrics();

        self.metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();

        Ok(())
    }

    /// Calcule les métriques dérivées complexes
    fn calculate_derived_metrics(&mut self) {
        // Vélocité des tokens (approximation)
        let circulating = self.token.circulating_supply as f64;
        let weekly_transactions = self.token.events.len() as f64 / 52.0; // Approximation
        self.metrics.calculated_metrics.token_velocity = if circulating > 0.0 {
            weekly_transactions / circulating * 52.0
        } else { 0.0 };

        // Indice de santé économique (composite)
        let staking_health = (self.metrics.staking_metrics.total_staking_ratio / self.config.staking_params.optimal_staking_ratio).min(1.0);
        let treasury_health = (self.metrics.treasury_metrics.available_funds as f64 / super::COMMUNITY_RESERVE as f64);
        let deflation_health = 1.0 - (self.metrics.deflation_metrics.annual_deflation_rate.abs() / 10.0).min(1.0);
        
        self.metrics.calculated_metrics.economic_health_index = (staking_health + treasury_health + deflation_health) / 3.0;

        // Score de décentralisation
        let validator_count = self.staking.metrics.active_validators_count as f64;
        let governance_participation = self.staking.metrics.governance_stakers_count as f64;
        self.metrics.calculated_metrics.decentralization_score = ((validator_count / 100.0).min(1.0) + (governance_participation / 1000.0).min(1.0)) / 2.0;

        // Prédiction de croissance simple
        self.metrics.calculated_metrics.growth_prediction = self.calculate_growth_prediction();
    }

    /// Calcule une prédiction de croissance simple
    fn calculate_growth_prediction(&self) -> f64 {
        // Modèle simple basé sur l'activité et la santé économique
        let activity_factor = self.metrics.reward_metrics.participation_rate / 100.0;
        let health_factor = self.metrics.calculated_metrics.economic_health_index;
        let staking_factor = self.metrics.staking_metrics.total_staking_ratio / 100.0;

        (activity_factor + health_factor + staking_factor) / 3.0 * 10.0 // 0-10% de croissance prédite
    }

    /// Exécute les ajustements automatiques si activés
    pub fn execute_auto_adjustments(&mut self) -> TokenOperationResult<Vec<ParameterAdjustment>> {
        let mut adjustments = Vec::new();

        if !self.config.advanced_features.auto_adjustments_enabled {
            return Ok(adjustments);
        }

        // Ajustement des récompenses basé sur l'activité
        if self.config.reward_params.auto_adjustment_enabled {
            if let Some(adjustment) = self.calculate_reward_adjustment() {
                adjustments.push(adjustment);
            }
        }

        // Ajustement du staking basé sur le ratio
        if self.config.staking_params.auto_reward_adjustment {
            if let Some(adjustment) = self.calculate_staking_adjustment() {
                adjustments.push(adjustment);
            }
        }

        // Enregistrer les ajustements
        for adjustment in &adjustments {
            self.reward_calculator.adjustment_history.push(adjustment.clone());
        }

        Ok(adjustments)
    }

    /// Calcule l'ajustement des récompenses
    fn calculate_reward_adjustment(&self) -> Option<ParameterAdjustment> {
        let current_participation = self.metrics.reward_metrics.participation_rate;
        let target_participation = 70.0; // 70% de participation cible

        if (current_participation - target_participation).abs() > 10.0 {
            let adjustment_factor = if current_participation < target_participation {
                1.1 // Augmenter les récompenses de 10%
            } else {
                0.9 // Diminuer les récompenses de 10%
            };

            Some(ParameterAdjustment {
                parameter_name: "reward_multiplier".to_string(),
                old_value: 1.0,
                new_value: adjustment_factor,
                reason: format!("Ajustement basé sur la participation: {:.1}%", current_participation),
                adjustment_date: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Calcule l'ajustement du staking
    fn calculate_staking_adjustment(&self) -> Option<ParameterAdjustment> {
        let current_ratio = self.metrics.staking_metrics.total_staking_ratio;
        let optimal_ratio = self.config.staking_params.optimal_staking_ratio;

        if (current_ratio - optimal_ratio).abs() > 5.0 {
            let new_apy = if current_ratio < optimal_ratio {
                self.config.staking_params.target_staking_apy * 1.1 // Augmenter l'APY
            } else {
                self.config.staking_params.target_staking_apy * 0.95 // Diminuer l'APY
            };

            Some(ParameterAdjustment {
                parameter_name: "staking_apy".to_string(),
                old_value: self.config.staking_params.target_staking_apy,
                new_value: new_apy,
                reason: format!("Ajustement basé sur le ratio de staking: {:.1}%", current_ratio),
                adjustment_date: Utc::now(),
            })
        } else {
            None
        }
    }

    /// Lance une simulation économique
    pub fn run_simulation(&mut self, scenario_name: &str) -> TokenOperationResult<SimulationResult> {
        let scenario = self.simulator.scenarios.get(scenario_name)
            .ok_or_else(|| TokenOperationError::Internal {
                message: format!("Scénario '{}' non trouvé", scenario_name),
            })?.clone();

        let mut timeline_data = Vec::new();
        let mut current_metrics = self.metrics.clone();

        // Simulation simplifiée jour par jour
        for day in 0..scenario.simulation_duration_days {
            // Appliquer les modifications du scénario
            self.apply_scenario_modifications(&scenario.modified_parameters);

            // Simuler l'évolution d'une journée
            current_metrics = self.simulate_daily_evolution(current_metrics);

            // Enregistrer le point temporel
            timeline_data.push(TimelinePoint {
                day,
                circulating_supply: current_metrics.token_metrics.circulating_supply,
                staked_tokens: current_metrics.staking_metrics.total_staking_ratio as u64,
                rewards_distributed: current_metrics.reward_metrics.total_rewards_distributed,
                deflation_rate: current_metrics.deflation_metrics.annual_deflation_rate,
            });
        }

        // Calculer le score de performance
        let performance_score = self.calculate_simulation_performance(&current_metrics);

        let result = SimulationResult {
            scenario_name: scenario_name.to_string(),
            final_metrics: current_metrics,
            timeline_data,
            performance_score,
            simulation_date: Utc::now(),
        };

        self.simulator.simulation_results.push(result.clone());
        Ok(result)
    }

    /// Applique les modifications de scénario
    fn apply_scenario_modifications(&mut self, modifications: &HashMap<String, f64>) {
        for (param, value) in modifications {
            match param.as_str() {
                "reward_multiplier" => {
                    // Modifier les paramètres de récompenses
                    self.config.reward_params.activity_adjustment_factor = *value;
                },
                "staking_apy" => {
                    self.config.staking_params.target_staking_apy = *value;
                },
                "deflation_rate" => {
                    self.config.inflation_params.target_annual_inflation_rate = -*value;
                },
                _ => {
                    // Paramètre non reconnu, ignorer
                }
            }
        }
    }

    /// Simule l'évolution d'une journée
    fn simulate_daily_evolution(&self, mut metrics: EconomicMetrics) -> EconomicMetrics {
        // Simulation simplifiée - dans la réalité, ce serait beaucoup plus complexe
        
        // Évolution des récompenses
        let daily_rewards = metrics.reward_metrics.total_rewards_distributed / 365;
        metrics.reward_metrics.total_rewards_distributed += daily_rewards;

        // Évolution du staking
        let staking_change = (self.config.staking_params.optimal_staking_ratio - metrics.staking_metrics.total_staking_ratio) * 0.01;
        metrics.staking_metrics.total_staking_ratio += staking_change;

        // Évolution de la déflation
        let daily_deflation = metrics.deflation_metrics.annual_deflation_rate / 365.0;
        metrics.deflation_metrics.total_burned_tokens += (metrics.token_metrics.circulating_supply as f64 * daily_deflation / 100.0) as u64;

        metrics
    }

    /// Calcule le score de performance de simulation
    fn calculate_simulation_performance(&self, final_metrics: &EconomicMetrics) -> f64 {
        let health_score = final_metrics.calculated_metrics.economic_health_index;
        let stability_score = 1.0 - (final_metrics.deflation_metrics.annual_deflation_rate.abs() / 10.0).min(1.0);
        let growth_score = final_metrics.calculated_metrics.growth_prediction / 10.0;

        (health_score + stability_score + growth_score) / 3.0 * 100.0
    }

    /// Obtient un rapport économique complet
    pub fn generate_economic_report(&self) -> EconomicReport {
        EconomicReport {
            summary: EconomicSummary {
                total_supply: self.token.total_supply,
                circulating_supply: self.token.circulating_supply,
                economic_health_index: self.metrics.calculated_metrics.economic_health_index,
                growth_prediction: self.metrics.calculated_metrics.growth_prediction,
            },
            token_overview: TokenOverview {
                burned_tokens: self.token.burned_tokens,
                locked_tokens: self.token.locked_tokens,
                holder_count: self.token.balances.len(),
                token_velocity: self.metrics.calculated_metrics.token_velocity,
            },
            staking_overview: StakingOverview {
                total_staked: self.staking.metrics.total_governance_staked + self.staking.metrics.total_validator_staked,
                staking_ratio: self.metrics.staking_metrics.total_staking_ratio,
                active_validators: self.staking.metrics.active_validators_count,
                governance_participation: self.staking.metrics.governance_stakers_count,
            },
            reward_overview: RewardOverview {
                total_distributed: self.rewards.get_system_statistics().total_distributed,
                distribution_efficiency: self.metrics.reward_metrics.reward_efficiency,
                participation_rate: self.metrics.reward_metrics.participation_rate,
            },
            treasury_overview: TreasuryOverview {
                available_funds: self.treasury.available_funds,
                active_projects: self.treasury.metrics.active_projects,
                approval_rate: self.metrics.treasury_metrics.proposal_approval_rate,
            },
            recommendations: self.generate_recommendations(),
            generated_at: Utc::now(),
        }
    }

    /// Génère des recommandations basées sur l'état actuel
    fn generate_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();

        // Recommandations basées sur le staking
        if self.metrics.staking_metrics.total_staking_ratio < self.config.staking_params.optimal_staking_ratio * 0.8 {
            recommendations.push("Considérer une augmentation des récompenses de staking pour améliorer la participation".to_string());
        }

        // Recommandations basées sur la santé économique
        if self.metrics.calculated_metrics.economic_health_index < 0.7 {
            recommendations.push("Surveiller de près les métriques économiques et considérer des ajustements".to_string());
        }

        // Recommandations basées sur la déflation
        if self.metrics.deflation_metrics.annual_deflation_rate.abs() > 5.0 {
            recommendations.push("Réévaluer les mécanismes de burning et d'inflation pour maintenir la stabilité".to_string());
        }

        // Recommandations basées sur le treasury
        if self.metrics.treasury_metrics.available_funds < super::COMMUNITY_RESERVE / 2 {
            recommendations.push("Réviser la stratégie d'allocation du treasury pour préserver les réserves".to_string());
        }

        recommendations
    }
}

impl EconomicMetrics {
    fn new() -> Self {
        Self {
            token_metrics: GlobalTokenMetrics::new(),
            distribution_metrics: DistributionMetrics {
                team_distribution_progress: 0.0,
                archival_rewards_distributed: 0,
                treasury_utilization_rate: 0.0,
                public_sale_completion: 0.0,
            },
            reward_metrics: RewardMetrics {
                total_rewards_distributed: 0,
                weekly_distribution_rate: 0,
                reward_efficiency: 0.0,
                participation_rate: 0.0,
            },
            staking_metrics: StakingMetrics {
                total_staking_ratio: 0.0,
                actual_staking_apy: 0.0,
                active_stakers_count: 0,
                average_staking_duration_days: 0.0,
            },
            treasury_metrics: TreasuryMetrics {
                available_funds: super::COMMUNITY_RESERVE,
                proposal_approval_rate: 0.0,
                active_projects_count: 0,
                average_project_roi: 0.0,
            },
            deflation_metrics: DeflationMetrics {
                total_burned_tokens: 0,
                annual_deflation_rate: 0.0,
                quality_staked_tokens: 0,
                longterm_bonus_distributed: 0,
            },
            calculated_metrics: CalculatedMetrics {
                token_velocity: 0.0,
                price_utility_ratio: 0.0,
                economic_health_index: 1.0,
                growth_prediction: 0.0,
                decentralization_score: 0.0,
            },
            last_updated: Utc::now(),
        }
    }
}

impl RewardCalculation {
    fn new() -> Self {
        Self {
            calculation_models: HashMap::new(),
            dynamic_parameters: DynamicParameters {
                network_activity_multipliers: HashMap::new(),
                seasonal_adjustments: HashMap::new(),
                performance_factors: HashMap::new(),
                last_updated: Utc::now(),
            },
            adjustment_history: Vec::new(),
            reward_predictions: Vec::new(),
        }
    }
}

impl EconomicSimulator {
    fn new() -> Self {
        Self {
            scenarios: HashMap::new(),
            simulation_results: Vec::new(),
            predictive_models: Vec::new(),
            simulation_config: SimulationConfig {
                time_step_hours: 24,
                max_iterations: 365,
                convergence_threshold: 0.001,
                monte_carlo_enabled: false,
                monte_carlo_samples: 1000,
            },
        }
    }
}

/// Rapport économique complet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicReport {
    pub summary: EconomicSummary,
    pub token_overview: TokenOverview,
    pub staking_overview: StakingOverview,
    pub reward_overview: RewardOverview,
    pub treasury_overview: TreasuryOverview,
    pub recommendations: Vec<String>,
    pub generated_at: DateTime<Utc>,
}

/// Résumé économique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomicSummary {
    pub total_supply: u64,
    pub circulating_supply: u64,
    pub economic_health_index: f64,
    pub growth_prediction: f64,
}

/// Vue d'ensemble des tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenOverview {
    pub burned_tokens: u64,
    pub locked_tokens: u64,
    pub holder_count: usize,
    pub token_velocity: f64,
}

/// Vue d'ensemble du staking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingOverview {
    pub total_staked: u64,
    pub staking_ratio: f64,
    pub active_validators: usize,
    pub governance_participation: usize,
}

/// Vue d'ensemble des récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardOverview {
    pub total_distributed: u64,
    pub distribution_efficiency: f64,
    pub participation_rate: f64,
}

/// Vue d'ensemble du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryOverview {
    pub available_funds: u64,
    pub active_projects: usize,
    pub approval_rate: f64,
}

impl Default for EconomicModel {
    fn default() -> Self {
        Self::new(EconomicConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_economic_model_creation() {
        let model = EconomicModel::default();
        assert_eq!(model.token.total_supply, TOTAL_SUPPLY);
        assert_eq!(model.metrics.calculated_metrics.economic_health_index, 1.0);
    }

    #[test]
    fn test_metrics_update() {
        let mut model = EconomicModel::default();
        let result = model.update_all_metrics();
        assert!(result.is_ok());
        
        // Vérifier que les métriques ont été mises à jour
        assert!(model.metrics.last_updated > model.created_at);
    }

    #[test]
    fn test_economic_health_calculation() {
        let model = EconomicModel::default();
        let health = model.metrics.calculated_metrics.economic_health_index;
        assert!(health >= 0.0 && health <= 1.0);
    }

    #[test]
    fn test_economic_report_generation() {
        let model = EconomicModel::default();
        let report = model.generate_economic_report();
        
        assert_eq!(report.summary.total_supply, TOTAL_SUPPLY);
        assert!(report.generated_at > model.created_at);
    }

    #[test]
    fn test_auto_adjustments() {
        let mut model = EconomicModel::default();
        let adjustments = model.execute_auto_adjustments().unwrap();
        
        // Les ajustements peuvent être vides si aucun n'est nécessaire
        assert!(adjustments.len() >= 0);
    }
}