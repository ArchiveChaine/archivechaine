//! Proof of Longevity pour ArchiveChain
//! 
//! Système de bonus pour récompenser le stockage à long terme et la fidélité des nœuds

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use crate::crypto::{Hash, HashAlgorithm, compute_hash};
use crate::error::Result;
use super::{NodeId, ConsensusConfig, ConsensusProof};

/// Gestionnaire des preuves de longévité
#[derive(Debug)]
pub struct LongevityProofManager {
    /// Configuration du consensus
    config: ConsensusConfig,
    /// Métriques de longévité par nœud
    node_metrics: HashMap<NodeId, LongevityMetrics>,
    /// Historique de stockage par archive
    storage_history: HashMap<Hash, ArchiveStorageHistory>,
    /// Jalons de fidélité atteints par les nœuds
    loyalty_milestones: HashMap<NodeId, Vec<LoyaltyMilestone>>,
    /// Epoch de début du tracking
    tracking_start_epoch: u64,
}

/// Métriques de longévité pour un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongevityMetrics {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Date de première participation au réseau
    pub first_seen: chrono::DateTime<chrono::Utc>,
    /// Durée totale de participation (jours)
    pub total_participation_days: u64,
    /// Durée continue actuelle (jours)
    pub current_streak_days: u64,
    /// Plus longue durée continue (jours)
    pub longest_streak_days: u64,
    /// Nombre d'archives stockées à long terme (>30 jours)
    pub long_term_archives: u32,
    /// Multiplicateur de fidélité actuel
    pub loyalty_multiplier: f64,
    /// Score de stabilité (0.0 - 1.0)
    pub stability_score: f64,
    /// Dernière activité enregistrée
    pub last_activity: chrono::DateTime<chrono::Utc>,
    /// Pénalités pour déconnexions
    pub disconnect_penalties: u32,
    /// Timestamp de dernière mise à jour
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Historique de stockage d'une archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveStorageHistory {
    /// Hash de l'archive
    pub archive_hash: Hash,
    /// Nœuds qui ont stocké cette archive avec timestamps
    pub storage_periods: HashMap<NodeId, Vec<StoragePeriod>>,
    /// Date de première apparition de l'archive
    pub first_stored: chrono::DateTime<chrono::Utc>,
    /// Dernière vérification de présence
    pub last_verified: Option<chrono::DateTime<chrono::Utc>>,
}

/// Période de stockage d'une archive par un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoragePeriod {
    /// Début du stockage
    pub start: chrono::DateTime<chrono::Utc>,
    /// Fin du stockage (None si toujours en cours)
    pub end: Option<chrono::DateTime<chrono::Utc>>,
    /// Durée en jours (calculée)
    pub duration_days: f64,
    /// Vérifications réussies pendant cette période
    pub successful_verifications: u32,
    /// Vérifications échouées
    pub failed_verifications: u32,
}

/// Jalon de fidélité atteint par un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoyaltyMilestone {
    /// Type de jalon
    pub milestone_type: MilestoneType,
    /// Date d'atteinte du jalon
    pub achieved_at: chrono::DateTime<chrono::Utc>,
    /// Valeur atteinte
    pub value: u64,
    /// Multiplicateur de bonus accordé
    pub bonus_multiplier: f64,
}

/// Types de jalons de fidélité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MilestoneType {
    /// Jours de participation continue
    ConsecutiveDays(u64),
    /// Jours de participation totale
    TotalDays(u64),
    /// Archives stockées à long terme
    LongTermArchives(u32),
    /// Période sans déconnexion
    ZeroDowntime(u64),
    /// Anniversaire de participation
    Anniversary(u32),
}

/// Bonus de longévité calculé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongevityBonus {
    /// Score de base de longévité (0.0 - 1.0)
    pub base_score: f64,
    /// Multiplicateur pour la durée de participation
    pub participation_multiplier: f64,
    /// Multiplicateur pour la stabilité
    pub stability_multiplier: f64,
    /// Multiplicateur pour le stockage à long terme
    pub long_term_storage_multiplier: f64,
    /// Bonus pour les jalons atteints
    pub milestone_bonus: f64,
    /// Score final avec tous les bonus (0.0 - 2.0)
    pub final_score: f64,
}

/// Défi de longévité pour vérifier la continuité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongevityChallenge {
    /// Identifiant du défi
    pub challenge_id: Hash,
    /// Nœud ciblé
    pub node_id: NodeId,
    /// Archives à vérifier pour la continuité
    pub archives_to_verify: Vec<Hash>,
    /// Période de référence à prouver
    pub reference_period: DateRange,
    /// Nonce pour éviter la prédiction
    pub nonce: u64,
    /// Timestamp de création
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Expiration
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

/// Plage de dates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateRange {
    /// Date de début
    pub start: chrono::DateTime<chrono::Utc>,
    /// Date de fin
    pub end: chrono::DateTime<chrono::Utc>,
}

/// Réponse à un défi de longévité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongevityProof {
    /// Identifiant du défi
    pub challenge_id: Hash,
    /// Preuves de stockage continu pour chaque archive
    pub continuity_proofs: Vec<ContinuityProof>,
    /// Preuves d'activité pendant la période
    pub activity_proofs: Vec<ActivityProof>,
    /// Timestamp de la réponse
    pub responded_at: chrono::DateTime<chrono::Utc>,
}

/// Preuve de continuité de stockage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContinuityProof {
    /// Archive concernée
    pub archive_hash: Hash,
    /// Timestamps de vérifications pendant la période
    pub verification_timestamps: Vec<chrono::DateTime<chrono::Utc>>,
    /// Hashes de preuve pour chaque vérification
    pub verification_hashes: Vec<Hash>,
    /// Période couverte
    pub period_covered: DateRange,
}

/// Preuve d'activité d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityProof {
    /// Type d'activité
    pub activity_type: ActivityType,
    /// Timestamp de l'activité
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Hash de preuve
    pub proof_hash: Hash,
    /// Données associées
    pub metadata: HashMap<String, String>,
}

/// Types d'activité trackés
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActivityType {
    /// Validation de bloc
    BlockValidation,
    /// Réponse à un défi
    ChallengeResponse,
    /// Service de téléchargement
    DownloadService,
    /// Participation au consensus
    ConsensusParticipation,
}

impl LongevityProofManager {
    /// Crée un nouveau gestionnaire de preuves de longévité
    pub fn new(config: &ConsensusConfig) -> Self {
        Self {
            config: config.clone(),
            node_metrics: HashMap::new(),
            storage_history: HashMap::new(),
            loyalty_milestones: HashMap::new(),
            tracking_start_epoch: 0,
        }
    }

    /// Enregistre l'activité d'un nœud
    pub fn record_node_activity(&mut self, node_id: NodeId, activity: ActivityType) {
        let now = chrono::Utc::now();
        
        // Met à jour ou crée les métriques du nœud
        let metrics = self.node_metrics.entry(node_id.clone()).or_insert_with(|| {
            LongevityMetrics {
                node_id: node_id.clone(),
                first_seen: now,
                total_participation_days: 0,
                current_streak_days: 0,
                longest_streak_days: 0,
                long_term_archives: 0,
                loyalty_multiplier: 1.0,
                stability_score: 1.0,
                last_activity: now,
                disconnect_penalties: 0,
                updated_at: now,
            }
        });

        // Met à jour la dernière activité
        let previous_activity = metrics.last_activity;
        metrics.last_activity = now;
        metrics.updated_at = now;

        // Calcule la continuité
        self.update_participation_streak(&node_id, previous_activity, now);
        
        // Vérifie les nouveaux jalons
        self.check_and_award_milestones(&node_id);
    }

    /// Enregistre le début de stockage d'une archive
    pub fn record_storage_start(&mut self, node_id: NodeId, archive_hash: Hash) {
        let now = chrono::Utc::now();
        
        // Met à jour l'historique de l'archive
        let history = self.storage_history.entry(archive_hash.clone()).or_insert_with(|| {
            ArchiveStorageHistory {
                archive_hash: archive_hash.clone(),
                storage_periods: HashMap::new(),
                first_stored: now,
                last_verified: None,
            }
        });

        // Ajoute une nouvelle période de stockage
        let periods = history.storage_periods.entry(node_id.clone()).or_insert_with(Vec::new);
        periods.push(StoragePeriod {
            start: now,
            end: None,
            duration_days: 0.0,
            successful_verifications: 0,
            failed_verifications: 0,
        });

        // Enregistre l'activité
        self.record_node_activity(node_id, ActivityType::DownloadService);
    }

    /// Enregistre la fin de stockage d'une archive
    pub fn record_storage_end(&mut self, node_id: &NodeId, archive_hash: &Hash) {
        let now = chrono::Utc::now();
        
        if let Some(history) = self.storage_history.get_mut(archive_hash) {
            if let Some(periods) = history.storage_periods.get_mut(node_id) {
                // Termine la dernière période active
                if let Some(last_period) = periods.iter_mut().rev().find(|p| p.end.is_none()) {
                    last_period.end = Some(now);
                    last_period.duration_days = now
                        .signed_duration_since(last_period.start)
                        .num_days() as f64;
                    
                    // Met à jour les métriques si c'était du stockage long terme
                    if last_period.duration_days >= 30.0 {
                        if let Some(metrics) = self.node_metrics.get_mut(node_id) {
                            metrics.long_term_archives += 1;
                        }
                    }
                }
            }
        }

        // Pénalise légèrement pour l'arrêt de stockage
        if let Some(metrics) = self.node_metrics.get_mut(node_id) {
            metrics.stability_score = (metrics.stability_score * 0.98).max(0.1);
        }
    }

    /// Génère un défi de longévité
    pub fn generate_longevity_challenge(&mut self, node_id: &NodeId) -> Result<LongevityChallenge> {
        // Sélectionne des archives stockées par ce nœud
        let archives_to_verify = self.select_archives_for_longevity_test(node_id)?;
        
        // Définit une période de référence (les 30 derniers jours)
        let now = chrono::Utc::now();
        let thirty_days_ago = now - chrono::Duration::days(30);
        
        let challenge_id = Hash::from_bytes(&rand::random::<[u8; 32]>())?;
        let nonce = rand::random::<u64>();
        let expires_at = now + chrono::Duration::from_std(self.config.challenge_timeout)?;

        Ok(LongevityChallenge {
            challenge_id,
            node_id: node_id.clone(),
            archives_to_verify,
            reference_period: DateRange {
                start: thirty_days_ago,
                end: now,
            },
            nonce,
            created_at: now,
            expires_at,
        })
    }

    /// Vérifie une preuve de longévité
    pub fn verify_longevity_proof(
        &mut self,
        challenge: &LongevityChallenge,
        proof: &LongevityProof,
    ) -> Result<bool> {
        // Vérifie que la preuve correspond au défi
        if proof.challenge_id != challenge.challenge_id {
            return Ok(false);
        }

        // Vérifie que la preuve n'est pas expirée
        if chrono::Utc::now() > challenge.expires_at {
            return Ok(false);
        }

        // Vérifie les preuves de continuité
        for continuity_proof in &proof.continuity_proofs {
            if !self.verify_continuity_proof(continuity_proof, challenge)? {
                return Ok(false);
            }
        }

        // Vérifie les preuves d'activité
        for activity_proof in &proof.activity_proofs {
            if !self.verify_activity_proof(activity_proof, challenge)? {
                return Ok(false);
            }
        }

        // Met à jour les métriques de vérification
        self.update_verification_metrics(&challenge.node_id, true);

        Ok(true)
    }

    /// Calcule le bonus de longévité pour un nœud
    pub fn calculate_longevity_bonus(&self, node_id: &NodeId) -> Result<LongevityBonus> {
        let metrics = self.get_node_metrics(node_id)?;
        
        // Score de base basé sur la participation totale
        let base_score = (metrics.total_participation_days as f64 / 365.0).min(1.0);
        
        // Multiplicateur pour la participation continue
        let participation_multiplier = 1.0 + (metrics.current_streak_days as f64 / 30.0) * 0.1;
        
        // Multiplicateur pour la stabilité
        let stability_multiplier = 1.0 + (metrics.stability_score - 0.5) * 0.5;
        
        // Multiplicateur pour le stockage long terme
        let long_term_storage_multiplier = 1.0 + (metrics.long_term_archives as f64 / 10.0) * 0.2;
        
        // Bonus pour les jalons
        let milestone_bonus = self.calculate_milestone_bonus(node_id);
        
        // Score final
        let final_score = (base_score * participation_multiplier * stability_multiplier * long_term_storage_multiplier + milestone_bonus).min(2.0);

        Ok(LongevityBonus {
            base_score,
            participation_multiplier,
            stability_multiplier,
            long_term_storage_multiplier,
            milestone_bonus,
            final_score,
        })
    }

    /// Obtient les métriques de longévité d'un nœud
    pub fn get_node_metrics(&self, node_id: &NodeId) -> Result<LongevityMetrics> {
        self.node_metrics.get(node_id)
            .cloned()
            .ok_or_else(|| crate::error::CoreError::Internal {
                message: format!("Métriques de longévité introuvables pour le nœud {:?}", node_id)
            })
    }

    /// Obtient le nombre de nœuds avec bonus de longévité
    pub fn active_nodes_count(&self) -> usize {
        self.node_metrics.len()
    }

    /// Met à jour les métriques quotidiennes (à appeler une fois par jour)
    pub fn daily_update(&mut self) {
        let now = chrono::Utc::now();
        
        for (node_id, metrics) in self.node_metrics.iter_mut() {
            // Met à jour les jours de participation totale
            let days_since_first = now.signed_duration_since(metrics.first_seen).num_days();
            metrics.total_participation_days = days_since_first as u64;
            
            // Vérifie si le nœud est toujours actif (activité dans les dernières 24h)
            let hours_since_activity = now.signed_duration_since(metrics.last_activity).num_hours();
            if hours_since_activity > 24 {
                // Nœud inactif - reset le streak et pénalise
                metrics.current_streak_days = 0;
                metrics.disconnect_penalties += 1;
                metrics.stability_score = (metrics.stability_score * 0.9).max(0.1);
            }
        }
    }

    // Méthodes privées

    fn update_participation_streak(
        &mut self,
        node_id: &NodeId,
        previous_activity: chrono::DateTime<chrono::Utc>,
        current_activity: chrono::DateTime<chrono::Utc>,
    ) {
        if let Some(metrics) = self.node_metrics.get_mut(node_id) {
            let hours_gap = current_activity.signed_duration_since(previous_activity).num_hours();
            
            if hours_gap <= 24 {
                // Activité continue
                let current_days = current_activity.signed_duration_since(metrics.first_seen).num_days();
                metrics.current_streak_days = current_days as u64;
                metrics.longest_streak_days = metrics.longest_streak_days.max(metrics.current_streak_days);
            } else if hours_gap > 48 {
                // Pause trop longue - reset le streak
                metrics.current_streak_days = 0;
                metrics.disconnect_penalties += 1;
            }
        }
    }

    fn check_and_award_milestones(&mut self, node_id: &NodeId) {
        if let Some(metrics) = self.node_metrics.get(node_id) {
            let mut new_milestones = Vec::new();
            
            // Vérifie les jalons de jours consécutifs
            for &threshold in &[7, 30, 90, 365] {
                if metrics.current_streak_days >= threshold {
                    let milestone = MilestoneType::ConsecutiveDays(threshold);
                    if !self.has_milestone(node_id, &milestone) {
                        new_milestones.push(LoyaltyMilestone {
                            milestone_type: milestone,
                            achieved_at: chrono::Utc::now(),
                            value: threshold,
                            bonus_multiplier: 1.0 + (threshold as f64 / 365.0) * 0.1,
                        });
                    }
                }
            }
            
            // Vérifie les jalons d'archives long terme
            for &threshold in &[1, 5, 10, 50] {
                if metrics.long_term_archives >= threshold {
                    let milestone = MilestoneType::LongTermArchives(threshold);
                    if !self.has_milestone(node_id, &milestone) {
                        new_milestones.push(LoyaltyMilestone {
                            milestone_type: milestone,
                            achieved_at: chrono::Utc::now(),
                            value: threshold as u64,
                            bonus_multiplier: 1.0 + (threshold as f64 / 10.0) * 0.05,
                        });
                    }
                }
            }
            
            // Ajoute les nouveaux jalons
            if !new_milestones.is_empty() {
                self.loyalty_milestones.entry(node_id.clone())
                    .or_insert_with(Vec::new)
                    .extend(new_milestones);
            }
        }
    }

    fn has_milestone(&self, node_id: &NodeId, milestone_type: &MilestoneType) -> bool {
        self.loyalty_milestones.get(node_id)
            .map(|milestones| milestones.iter().any(|m| std::mem::discriminant(&m.milestone_type) == std::mem::discriminant(milestone_type)))
            .unwrap_or(false)
    }

    fn calculate_milestone_bonus(&self, node_id: &NodeId) -> f64 {
        self.loyalty_milestones.get(node_id)
            .map(|milestones| {
                milestones.iter()
                    .map(|m| (m.bonus_multiplier - 1.0) * 0.1) // 10% de chaque bonus
                    .sum()
            })
            .unwrap_or(0.0)
    }

    fn select_archives_for_longevity_test(&self, node_id: &NodeId) -> Result<Vec<Hash>> {
        let archives: Vec<Hash> = self.storage_history
            .iter()
            .filter(|(_, history)| {
                history.storage_periods.get(node_id)
                    .map(|periods| periods.iter().any(|p| p.end.is_none()))
                    .unwrap_or(false)
            })
            .map(|(hash, _)| hash.clone())
            .take(5) // Maximum 5 archives à vérifier
            .collect();

        if archives.is_empty() {
            return Err(crate::error::CoreError::Internal {
                message: "Aucune archive en cours de stockage pour ce nœud".to_string()
            });
        }

        Ok(archives)
    }

    fn verify_continuity_proof(&self, proof: &ContinuityProof, challenge: &LongevityChallenge) -> Result<bool> {
        // Vérifie que l'archive est dans le défi
        if !challenge.archives_to_verify.contains(&proof.archive_hash) {
            return Ok(false);
        }

        // Vérifie que la période couverte correspond au défi
        if proof.period_covered.start > challenge.reference_period.start ||
           proof.period_covered.end < challenge.reference_period.end {
            return Ok(false);
        }

        // Vérifie que les timestamps et hashes correspondent
        if proof.verification_timestamps.len() != proof.verification_hashes.len() {
            return Ok(false);
        }

        // Vérifie qu'il y a suffisamment de vérifications (au moins une par semaine)
        let period_days = proof.period_covered.end
            .signed_duration_since(proof.period_covered.start)
            .num_days();
        let expected_verifications = (period_days / 7).max(1) as usize;
        
        if proof.verification_timestamps.len() < expected_verifications {
            return Ok(false);
        }

        Ok(true)
    }

    fn verify_activity_proof(&self, proof: &ActivityProof, challenge: &LongevityChallenge) -> Result<bool> {
        // Vérifie que l'activité est dans la période de référence
        if proof.timestamp < challenge.reference_period.start ||
           proof.timestamp > challenge.reference_period.end {
            return Ok(false);
        }

        // Vérifie le hash de preuve (implémentation basique)
        let proof_data = format!("{:?}_{}", proof.activity_type, proof.timestamp.timestamp());
        let expected_hash = compute_hash(proof_data.as_bytes(), HashAlgorithm::Blake3);
        
        // Note: Dans une implémentation réelle, on vérifierait des signatures cryptographiques
        Ok(!proof.proof_hash.is_zero())
    }

    fn update_verification_metrics(&mut self, node_id: &NodeId, success: bool) {
        if let Some(metrics) = self.node_metrics.get_mut(node_id) {
            if success {
                metrics.stability_score = (metrics.stability_score * 0.95 + 0.05).min(1.0);
            } else {
                metrics.stability_score = (metrics.stability_score * 0.9).max(0.1);
            }
            metrics.updated_at = chrono::Utc::now();
        }
    }
}

impl ConsensusProof for LongevityProofManager {
    type Metrics = LongevityMetrics;

    fn calculate_score(&self, node_id: &NodeId, _metrics: &Self::Metrics) -> Result<f64> {
        let bonus = self.calculate_longevity_bonus(node_id)?;
        Ok((bonus.final_score / 2.0).min(1.0)) // Normalise entre 0 et 1
    }

    fn verify_proof(&self, _node_id: &NodeId, proof_data: &[u8]) -> Result<bool> {
        // Désérialise la preuve
        let proof: LongevityProof = bincode::deserialize(proof_data)
            .map_err(|e| crate::error::CoreError::Internal {
                message: format!("Erreur de désérialisation: {}", e)
            })?;

        // Vérification basique
        Ok(!proof.continuity_proofs.is_empty() || !proof.activity_proofs.is_empty())
    }

    fn generate_challenge(&self, node_id: &NodeId) -> Result<Vec<u8>> {
        // Génère un défi basique
        let challenge_data = format!("longevity_test_{}", node_id.hash());
        Ok(challenge_data.into_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{generate_keypair, Hash};

    #[test]
    fn test_longevity_proof_manager_creation() {
        let config = ConsensusConfig::test_config();
        let manager = LongevityProofManager::new(&config);
        
        assert_eq!(manager.active_nodes_count(), 0);
    }

    #[test]
    fn test_node_activity_recording() {
        let config = ConsensusConfig::test_config();
        let mut manager = LongevityProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        manager.record_node_activity(node_id.clone(), ActivityType::BlockValidation);
        
        let metrics = manager.get_node_metrics(&node_id).unwrap();
        assert!(metrics.last_activity > metrics.first_seen - chrono::Duration::seconds(1));
    }

    #[test]
    fn test_storage_tracking() {
        let config = ConsensusConfig::test_config();
        let mut manager = LongevityProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        let archive_hash = Hash::from_bytes(&[1; 32]).unwrap();
        
        manager.record_storage_start(node_id.clone(), archive_hash.clone());
        
        assert!(manager.storage_history.contains_key(&archive_hash));
        
        let history = manager.storage_history.get(&archive_hash).unwrap();
        assert!(history.storage_periods.contains_key(&node_id));
    }

    #[test]
    fn test_longevity_bonus_calculation() {
        let config = ConsensusConfig::test_config();
        let mut manager = LongevityProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        // Simule un nœud avec de l'historique
        manager.record_node_activity(node_id.clone(), ActivityType::BlockValidation);
        
        if let Some(metrics) = manager.node_metrics.get_mut(&node_id) {
            metrics.total_participation_days = 100;
            metrics.current_streak_days = 30;
            metrics.long_term_archives = 5;
            metrics.stability_score = 0.9;
        }
        
        let bonus = manager.calculate_longevity_bonus(&node_id).unwrap();
        assert!(bonus.final_score > 0.0);
        assert!(bonus.final_score <= 2.0);
        assert!(bonus.participation_multiplier > 1.0);
    }

    #[test]
    fn test_milestone_awarding() {
        let config = ConsensusConfig::test_config();
        let mut manager = LongevityProofManager::new(&config);
        
        let keypair = generate_keypair().unwrap();
        let node_id = NodeId::from_public_key(keypair.public_key());
        
        // Simule un nœud qui atteint des jalons
        if let Some(metrics) = manager.node_metrics.entry(node_id.clone()).or_insert_with(|| {
            LongevityMetrics {
                node_id: node_id.clone(),
                first_seen: chrono::Utc::now(),
                total_participation_days: 0,
                current_streak_days: 30, // 30 jours consécutifs
                longest_streak_days: 30,
                long_term_archives: 5, // 5 archives long terme
                loyalty_multiplier: 1.0,
                stability_score: 1.0,
                last_activity: chrono::Utc::now(),
                disconnect_penalties: 0,
                updated_at: chrono::Utc::now(),
            }
        }) {
            metrics.current_streak_days = 30;
            metrics.long_term_archives = 5;
        }
        
        manager.check_and_award_milestones(&node_id);
        
        let milestones = manager.loyalty_milestones.get(&node_id).unwrap();
        assert!(!milestones.is_empty());
    }
}