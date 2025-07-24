//! Système de sélection des leaders pour ArchiveChain
//! 
//! Algorithme équitable pour sélectionner les validateurs basé sur les scores PoA

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap};
use crate::crypto::{Hash, HashAlgorithm, compute_hash, compute_combined_hash};
use crate::error::Result;
use super::{NodeId, ConsensusScore, ConsensusConfig};

/// Sélecteur de leaders pour le consensus
#[derive(Debug)]
pub struct LeaderSelector {
    /// Configuration du consensus
    config: ConsensusConfig,
    /// Informations sur les validateurs éligibles
    validator_pool: HashMap<NodeId, ValidatorInfo>,
    /// Historique des sélections pour éviter la centralisation
    selection_history: BTreeMap<u64, Vec<NodeId>>, // epoch -> validateurs sélectionnés
    /// Seed aléatoire pour la sélection déterministe
    random_seed: Hash,
    /// Epoch actuel
    current_epoch: u64,
}

/// Informations sur un validateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorInfo {
    /// Identifiant du nœud
    pub node_id: NodeId,
    /// Score de consensus actuel
    pub consensus_score: ConsensusScore,
    /// Stake du validateur (pour la sélection pondérée)
    pub stake_amount: u64,
    /// Nombre de blocs validés récemment
    pub recent_validations: u32,
    /// Taux de participation (0.0 - 1.0)
    pub participation_rate: f64,
    /// Dernière fois sélectionné comme leader
    pub last_selected_epoch: Option<u64>,
    /// Pénalités accumulées
    pub penalties: u32,
    /// Statut d'éligibilité
    pub eligibility_status: EligibilityStatus,
    /// Timestamp d'enregistrement
    pub registered_at: chrono::DateTime<chrono::Utc>,
    /// Dernière mise à jour
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Statut d'éligibilité d'un validateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EligibilityStatus {
    /// Éligible pour être sélectionné
    Eligible,
    /// Temporairement suspendu
    Suspended(SuspensionReason),
    /// Banni définitivement
    Banned(String),
    /// En probation (scores réduits)
    Probation(u32), // nombre d'epochs restantes
}

/// Raisons de suspension
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuspensionReason {
    /// Score de consensus trop faible
    LowScore,
    /// Échec de validation de blocs
    ValidationFailure,
    /// Inactivité prolongée
    Inactivity,
    /// Comportement malveillant détecté
    MaliciousBehavior,
}

/// Résultat d'une élection de leader
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaderElectionResult {
    /// Epoch pour lequel la sélection est faite
    pub epoch: u64,
    /// Leader principal sélectionné
    pub primary_leader: NodeId,
    /// Leaders de secours
    pub backup_leaders: Vec<NodeId>,
    /// Tous les validateurs pour cet epoch
    pub validators: Vec<NodeId>,
    /// Seed utilisé pour cette sélection
    pub selection_seed: Hash,
    /// Timestamp de la sélection
    pub selected_at: chrono::DateTime<chrono::Utc>,
    /// Métriques de diversité
    pub diversity_metrics: DiversityMetrics,
}

/// Métriques de diversité de la sélection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiversityMetrics {
    /// Distribution des scores (écart-type)
    pub score_distribution: f64,
    /// Rotation par rapport à l'epoch précédent (0.0 - 1.0)
    pub rotation_rate: f64,
    /// Représentation géographique (si disponible)
    pub geographic_distribution: Option<HashMap<String, u32>>,
    /// Équité de la sélection (coefficient de Gini)
    pub fairness_coefficient: f64,
}

/// Algorithme de sélection utilisé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SelectionAlgorithm {
    /// Sélection pondérée par score
    WeightedByScore,
    /// Sélection aléatoire pondérée
    WeightedRandom,
    /// Rotation équitable avec scores
    FairRotation,
    /// Algorithme hybride (défaut)
    Hybrid,
}

impl LeaderSelector {
    /// Crée un nouveau sélecteur de leaders
    pub fn new(config: ConsensusConfig, initial_seed: Hash) -> Self {
        Self {
            config,
            validator_pool: HashMap::new(),
            selection_history: BTreeMap::new(),
            random_seed: initial_seed,
            current_epoch: 0,
        }
    }

    /// Enregistre un nouveau validateur
    pub fn register_validator(&mut self, node_id: NodeId, initial_score: ConsensusScore) -> Result<()> {
        // Vérifie l'éligibilité basique
        if !initial_score.is_eligible_validator(&self.config) {
            return Err(crate::error::CoreError::Validation {
                message: "Score insuffisant pour être validateur".to_string()
            });
        }

        let validator_info = ValidatorInfo {
            node_id: node_id.clone(),
            consensus_score: initial_score,
            stake_amount: 0, // À définir selon le modèle économique
            recent_validations: 0,
            participation_rate: 1.0,
            last_selected_epoch: None,
            penalties: 0,
            eligibility_status: EligibilityStatus::Eligible,
            registered_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        };

        self.validator_pool.insert(node_id, validator_info);
        Ok(())
    }

    /// Met à jour le score d'un validateur
    pub fn update_validator_score(&mut self, node_id: &NodeId, new_score: ConsensusScore) -> Result<()> {
        if let Some(validator) = self.validator_pool.get_mut(node_id) {
            validator.consensus_score = new_score;
            validator.updated_at = chrono::Utc::now();

            // Met à jour le statut d'éligibilité
            self.update_eligibility_status(node_id)?;
        } else {
            return Err(crate::error::CoreError::Internal {
                message: format!("Validateur {:?} non trouvé", node_id)
            });
        }
        Ok(())
    }

    /// Sélectionne les leaders pour l'epoch suivant
    pub fn select_leaders_for_epoch(&mut self, target_epoch: u64) -> Result<LeaderElectionResult> {
        self.current_epoch = target_epoch;
        
        // Nettoie les validateurs non éligibles
        self.cleanup_ineligible_validators();
        
        // Obtient les validateurs éligibles
        let eligible_validators = self.get_eligible_validators();
        
        if eligible_validators.is_empty() {
            return Err(crate::error::CoreError::Internal {
                message: "Aucun validateur éligible disponible".to_string()
            });
        }

        // Génère un seed pour cette epoch
        let selection_seed = self.generate_epoch_seed(target_epoch)?;
        
        // Sélectionne en utilisant l'algorithme hybride
        let selection_result = self.hybrid_selection_algorithm(&eligible_validators, &selection_seed)?;
        
        // Enregistre la sélection dans l'historique
        self.selection_history.insert(target_epoch, selection_result.validators.clone());
        
        // Limite l'historique
        self.trim_selection_history();
        
        // Met à jour les informations des validateurs sélectionnés
        self.update_selected_validators(&selection_result)?;

        Ok(selection_result)
    }

    /// Rapporte la performance d'un validateur
    pub fn report_validator_performance(
        &mut self,
        node_id: &NodeId,
        blocks_validated: u32,
        participation: bool,
    ) -> Result<()> {
        if let Some(validator) = self.validator_pool.get_mut(node_id) {
            validator.recent_validations += blocks_validated;
            
            // Met à jour le taux de participation
            let new_participation = if participation { 1.0 } else { 0.0 };
            validator.participation_rate = validator.participation_rate * 0.9 + new_participation * 0.1;
            
            validator.updated_at = chrono::Utc::now();
            
            // Met à jour l'éligibilité
            self.update_eligibility_status(node_id)?;
        }
        Ok(())
    }

    /// Applique une pénalité à un validateur
    pub fn penalize_validator(&mut self, node_id: &NodeId, reason: SuspensionReason) -> Result<()> {
        if let Some(validator) = self.validator_pool.get_mut(node_id) {
            validator.penalties += 1;
            
            // Applique la suspension selon la gravité
            validator.eligibility_status = match reason {
                SuspensionReason::LowScore => {
                    if validator.penalties >= 3 {
                        EligibilityStatus::Suspended(reason)
                    } else {
                        EligibilityStatus::Probation(5) // 5 epochs
                    }
                },
                SuspensionReason::ValidationFailure => {
                    EligibilityStatus::Suspended(reason)
                },
                SuspensionReason::MaliciousBehavior => {
                    EligibilityStatus::Banned("Comportement malveillant détecté".to_string())
                },
                _ => EligibilityStatus::Suspended(reason),
            };
            
            validator.updated_at = chrono::Utc::now();
        }
        Ok(())
    }

    /// Obtient les informations d'un validateur
    pub fn get_validator_info(&self, node_id: &NodeId) -> Option<&ValidatorInfo> {
        self.validator_pool.get(node_id)
    }

    /// Obtient la liste de tous les validateurs éligibles
    pub fn get_eligible_validators(&self) -> Vec<&ValidatorInfo> {
        self.validator_pool
            .values()
            .filter(|v| matches!(v.eligibility_status, EligibilityStatus::Eligible))
            .collect()
    }

    /// Obtient les statistiques de sélection
    pub fn get_selection_statistics(&self) -> SelectionStatistics {
        let total_validators = self.validator_pool.len();
        let eligible_validators = self.get_eligible_validators().len();
        let suspended_validators = self.validator_pool.values()
            .filter(|v| matches!(v.eligibility_status, EligibilityStatus::Suspended(_)))
            .count();

        SelectionStatistics {
            total_validators,
            eligible_validators,
            suspended_validators,
            current_epoch: self.current_epoch,
            recent_selections: self.selection_history.len(),
        }
    }

    // Méthodes privées

    fn update_eligibility_status(&mut self, node_id: &NodeId) -> Result<()> {
        if let Some(validator) = self.validator_pool.get_mut(node_id) {
            match &validator.eligibility_status {
                EligibilityStatus::Probation(epochs_left) => {
                    if *epochs_left <= 1 {
                        validator.eligibility_status = EligibilityStatus::Eligible;
                    } else {
                        validator.eligibility_status = EligibilityStatus::Probation(epochs_left - 1);
                    }
                },
                EligibilityStatus::Suspended(reason) => {
                    // Vérifie si la suspension peut être levée
                    if validator.consensus_score.is_eligible_validator(&self.config) && 
                       validator.participation_rate > 0.8 {
                        validator.eligibility_status = EligibilityStatus::Probation(3);
                    }
                },
                _ => {
                    // Vérifie si le validateur doit être suspendu
                    if !validator.consensus_score.is_eligible_validator(&self.config) {
                        validator.eligibility_status = EligibilityStatus::Suspended(SuspensionReason::LowScore);
                    } else if validator.participation_rate < 0.5 {
                        validator.eligibility_status = EligibilityStatus::Suspended(SuspensionReason::Inactivity);
                    }
                }
            }
        }
        Ok(())
    }

    fn cleanup_ineligible_validators(&mut self) {
        let current_time = chrono::Utc::now();
        
        self.validator_pool.retain(|_, validator| {
            // Supprime les validateurs inactifs depuis plus de 30 jours
            let inactive_duration = current_time.signed_duration_since(validator.updated_at);
            if inactive_duration.num_days() > 30 {
                matches!(validator.eligibility_status, EligibilityStatus::Banned(_))
            } else {
                true
            }
        });
    }

    fn generate_epoch_seed(&self, epoch: u64) -> Result<Hash> {
        let mut seed_data = Vec::new();
        seed_data.extend_from_slice(self.random_seed.as_bytes());
        seed_data.extend_from_slice(&epoch.to_le_bytes());
        
        // Ajoute de l'entropie des validateurs précédents
        if let Some(previous_validators) = self.selection_history.get(&(epoch - 1)) {
            for validator_id in previous_validators {
                seed_data.extend_from_slice(validator_id.hash().as_bytes());
            }
        }
        
        Ok(compute_hash(&seed_data, HashAlgorithm::Blake3))
    }

    fn hybrid_selection_algorithm(
        &self,
        eligible_validators: &[&ValidatorInfo],
        seed: &Hash,
    ) -> Result<LeaderElectionResult> {
        let target_count = self.config.validators_per_round.min(eligible_validators.len());
        
        // Calcule les poids de sélection
        let mut weighted_validators: Vec<(f64, &ValidatorInfo)> = eligible_validators
            .iter()
            .map(|v| {
                let weight = self.calculate_selection_weight(v);
                (weight, *v)
            })
            .collect();

        // Trie par poids décroissant
        weighted_validators.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());

        // Sélection hybride : 70% basé sur le score, 30% aléatoire pondéré
        let mut selected = Vec::new();
        let top_performers_count = (target_count as f64 * 0.7).ceil() as usize;
        
        // Sélectionne les top performers
        for (_, validator) in weighted_validators.iter().take(top_performers_count) {
            selected.push(validator.node_id.clone());
        }

        // Sélection aléatoire pondérée pour le reste
        let remaining_count = target_count - selected.len();
        if remaining_count > 0 {
            let remaining_validators: Vec<_> = weighted_validators
                .iter()
                .skip(top_performers_count)
                .collect();
            
            let random_selected = self.weighted_random_selection(
                &remaining_validators,
                remaining_count,
                seed,
            )?;
            
            selected.extend(random_selected);
        }

        // Sélectionne le leader principal (celui avec le plus haut score)
        let primary_leader = selected[0].clone();
        let backup_leaders = selected[1..].to_vec();

        // Calcule les métriques de diversité
        let diversity_metrics = self.calculate_diversity_metrics(&selected);

        Ok(LeaderElectionResult {
            epoch: self.current_epoch,
            primary_leader,
            backup_leaders,
            validators: selected,
            selection_seed: seed.clone(),
            selected_at: chrono::Utc::now(),
            diversity_metrics,
        })
    }

    fn calculate_selection_weight(&self, validator: &ValidatorInfo) -> f64 {
        let base_weight = validator.consensus_score.combined_score;
        
        // Facteur de rotation (encourage la diversité)
        let rotation_factor = if let Some(last_epoch) = validator.last_selected_epoch {
            let epochs_since = self.current_epoch.saturating_sub(last_epoch);
            1.0 + (epochs_since as f64 / 10.0).min(0.5) // Bonus jusqu'à 50%
        } else {
            1.2 // Bonus pour les nouveaux validateurs
        };

        // Facteur de participation
        let participation_factor = 0.5 + validator.participation_rate * 0.5;

        // Pénalité pour les fautes
        let penalty_factor = 1.0 - (validator.penalties as f64 * 0.1).min(0.5);

        base_weight * rotation_factor * participation_factor * penalty_factor
    }

    fn weighted_random_selection(
        &self,
        candidates: &[(f64, &ValidatorInfo)],
        count: usize,
        seed: &Hash,
    ) -> Result<Vec<NodeId>> {
        let mut selected = Vec::new();
        let mut available = candidates.to_vec();

        for i in 0..count.min(available.len()) {
            // Utilise le seed pour générer un nombre pseudo-aléatoire déterministe
            let random_data = compute_combined_hash(
                &[seed.as_bytes(), &i.to_le_bytes()],
                HashAlgorithm::Blake3,
            );
            let random_value = u64::from_le_bytes(
                random_data.as_bytes()[0..8].try_into().unwrap()
            ) as f64 / u64::MAX as f64;

            // Sélection pondérée
            let total_weight: f64 = available.iter().map(|(w, _)| *w).sum();
            let mut cumulative_weight = 0.0;
            let target_weight = random_value * total_weight;

            for (j, (weight, validator)) in available.iter().enumerate() {
                cumulative_weight += weight;
                if cumulative_weight >= target_weight {
                    selected.push(validator.node_id.clone());
                    available.remove(j);
                    break;
                }
            }
        }

        Ok(selected)
    }

    fn calculate_diversity_metrics(&self, selected: &[NodeId]) -> DiversityMetrics {
        // Calcule la distribution des scores
        let scores: Vec<f64> = selected
            .iter()
            .filter_map(|id| self.validator_pool.get(id))
            .map(|v| v.consensus_score.combined_score)
            .collect();

        let mean_score = scores.iter().sum::<f64>() / scores.len() as f64;
        let variance = scores.iter()
            .map(|score| (score - mean_score).powi(2))
            .sum::<f64>() / scores.len() as f64;
        let score_distribution = variance.sqrt();

        // Calcule le taux de rotation
        let rotation_rate = if let Some(previous) = self.selection_history.get(&(self.current_epoch - 1)) {
            let new_validators = selected.iter()
                .filter(|id| !previous.contains(id))
                .count();
            new_validators as f64 / selected.len() as f64
        } else {
            1.0
        };

        // Coefficient de Gini simplifié (mesure d'équité)
        let mut sorted_scores = scores;
        sorted_scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let n = sorted_scores.len() as f64;
        let sum_scores: f64 = sorted_scores.iter().sum();
        
        let gini = if sum_scores > 0.0 {
            let mut gini_sum = 0.0;
            for (i, score) in sorted_scores.iter().enumerate() {
                gini_sum += (2.0 * (i as f64 + 1.0) - n - 1.0) * score;
            }
            gini_sum / (n * sum_scores)
        } else {
            0.0
        };

        DiversityMetrics {
            score_distribution,
            rotation_rate,
            geographic_distribution: None, // À implémenter avec des données géographiques
            fairness_coefficient: gini.abs(),
        }
    }

    fn update_selected_validators(&mut self, result: &LeaderElectionResult) -> Result<()> {
        for validator_id in &result.validators {
            if let Some(validator) = self.validator_pool.get_mut(validator_id) {
                validator.last_selected_epoch = Some(result.epoch);
                validator.updated_at = chrono::Utc::now();
            }
        }
        Ok(())
    }

    fn trim_selection_history(&mut self) {
        // Garde seulement les 100 dernières epochs
        if self.selection_history.len() > 100 {
            let cutoff_epoch = self.current_epoch.saturating_sub(100);
            self.selection_history.retain(|&epoch, _| epoch >= cutoff_epoch);
        }
    }
}

/// Statistiques de sélection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelectionStatistics {
    /// Nombre total de validateurs
    pub total_validators: usize,
    /// Nombre de validateurs éligibles
    pub eligible_validators: usize,
    /// Nombre de validateurs suspendus
    pub suspended_validators: usize,
    /// Epoch actuel
    pub current_epoch: u64,
    /// Nombre de sélections récentes dans l'historique
    pub recent_selections: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    fn create_test_validator_info(node_id: NodeId, score: f64) -> ValidatorInfo {
        let consensus_score = super::super::ConsensusScore {
            storage_score: score,
            bandwidth_score: score,
            longevity_score: score,
            combined_score: score,
            node_id: node_id.clone(),
            calculated_at: chrono::Utc::now(),
        };

        ValidatorInfo {
            node_id,
            consensus_score,
            stake_amount: 1000,
            recent_validations: 10,
            participation_rate: 0.9,
            last_selected_epoch: None,
            penalties: 0,
            eligibility_status: EligibilityStatus::Eligible,
            registered_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn test_leader_selector_creation() {
        let config = ConsensusConfig::test_config();
        let seed = Hash::from_bytes(&[1; 32]).unwrap();
        let selector = LeaderSelector::new(config, seed);
        
        assert_eq!(selector.validator_pool.len(), 0);
        assert_eq!(selector.current_epoch, 0);
    }

    #[test]
    fn test_validator_registration() {
        let config = ConsensusConfig::test_config();
        let seed = Hash::from_bytes(&[1; 32]).unwrap();
        let mut selector = LeaderSelector::new(config, seed);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        let consensus_score = super::super::ConsensusScore {
            storage_score: 0.8,
            bandwidth_score: 0.7,
            longevity_score: 0.6,
            combined_score: 0.7,
            node_id: node_id.clone(),
            calculated_at: chrono::Utc::now(),
        };
        
        let result = selector.register_validator(node_id.clone(), consensus_score);
        assert!(result.is_ok());
        assert_eq!(selector.validator_pool.len(), 1);
        assert!(selector.get_validator_info(&node_id).is_some());
    }

    #[test]
    fn test_leader_selection() {
        let config = ConsensusConfig::test_config();
        let seed = Hash::from_bytes(&[1; 32]).unwrap();
        let mut selector = LeaderSelector::new(config, seed);
        
        // Ajoute plusieurs validateurs
        for i in 0..5 {
            let keypair = generate_keypair().unwrap();
            let node_id = NodeId::from_public_key(keypair.public_key());
            let score = 0.5 + (i as f64 * 0.1);
            
            let consensus_score = super::super::ConsensusScore {
                storage_score: score,
                bandwidth_score: score,
                longevity_score: score,
                combined_score: score,
                node_id: node_id.clone(),
                calculated_at: chrono::Utc::now(),
            };
            
            selector.register_validator(node_id, consensus_score).unwrap();
        }
        
        let result = selector.select_leaders_for_epoch(1).unwrap();
        
        assert_eq!(result.epoch, 1);
        assert!(!result.validators.is_empty());
        assert!(result.validators.len() <= selector.config.validators_per_round);
        assert!(result.diversity_metrics.rotation_rate >= 0.0);
    }

    #[test]
    fn test_validator_performance_reporting() {
        let config = ConsensusConfig::test_config();
        let seed = Hash::from_bytes(&[1; 32]).unwrap();
        let mut selector = LeaderSelector::new(config, seed);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        let consensus_score = super::super::ConsensusScore {
            storage_score: 0.8,
            bandwidth_score: 0.7,
            longevity_score: 0.6,
            combined_score: 0.7,
            node_id: node_id.clone(),
            calculated_at: chrono::Utc::now(),
        };
        
        selector.register_validator(node_id.clone(), consensus_score).unwrap();
        
        let initial_validations = selector.get_validator_info(&node_id).unwrap().recent_validations;
        
        selector.report_validator_performance(&node_id, 5, true).unwrap();
        
        let updated_validations = selector.get_validator_info(&node_id).unwrap().recent_validations;
        assert_eq!(updated_validations, initial_validations + 5);
    }

    #[test]
    fn test_validator_penalization() {
        let config = ConsensusConfig::test_config();
        let seed = Hash::from_bytes(&[1; 32]).unwrap();
        let mut selector = LeaderSelector::new(config, seed);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        let consensus_score = super::super::ConsensusScore {
            storage_score: 0.8,
            bandwidth_score: 0.7,
            longevity_score: 0.6,
            combined_score: 0.7,
            node_id: node_id.clone(),
            calculated_at: chrono::Utc::now(),
        };
        
        selector.register_validator(node_id.clone(), consensus_score).unwrap();
        
        // Vérifie que le validateur est initialement éligible
        assert!(matches!(
            selector.get_validator_info(&node_id).unwrap().eligibility_status,
            EligibilityStatus::Eligible
        ));
        
        // Applique une pénalité
        selector.penalize_validator(&node_id, SuspensionReason::ValidationFailure).unwrap();
        
        // Vérifie que le validateur est maintenant suspendu
        assert!(matches!(
            selector.get_validator_info(&node_id).unwrap().eligibility_status,
            EligibilityStatus::Suspended(_)
        ));
    }
}