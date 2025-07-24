//! Système de staking et gouvernance pour ArchiveChain
//!
//! Implémente :
//! - Staking pour la gouvernance (minimum 1M ARC)
//! - Staking pour la validation (minimum 10M ARC)
//! - Système de vote et propositions
//! - Délégation de pouvoir de vote
//! - Récompenses de staking

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey, Signature};
use super::{TokenOperationResult, TokenOperationError, ARCToken};

/// Système de staking principal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingSystem {
    /// Stakes de gouvernance
    pub governance_stakes: HashMap<PublicKey, GovernanceStake>,
    /// Stakes de validation
    pub validator_stakes: HashMap<PublicKey, ValidatorStake>,
    /// Propositions de gouvernance
    pub proposals: HashMap<Hash, GovernanceProposal>,
    /// Délégations de vote
    pub delegations: HashMap<PublicKey, VoteDelegation>,
    /// Configuration du staking
    pub config: StakingConfig,
    /// Métriques du système
    pub metrics: StakingMetrics,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Stake pour la gouvernance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceStake {
    /// Adresse du stakeur
    pub staker: PublicKey,
    /// Montant staké
    pub amount: u64,
    /// Date de début du stake
    pub start_date: DateTime<Utc>,
    /// Durée de lock (en jours)
    pub lock_duration_days: u32,
    /// Date de fin de lock
    pub lock_end_date: DateTime<Utc>,
    /// Multiplicateur de pouvoir de vote
    pub voting_power_multiplier: f64,
    /// Votes récents
    pub recent_votes: Vec<VoteRecord>,
    /// Récompenses accumulées
    pub accumulated_rewards: u64,
    /// Dernière réclamation de récompenses
    pub last_reward_claim: Option<DateTime<Utc>>,
    /// Statut du stake
    pub status: StakeStatus,
}

/// Stake pour la validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorStake {
    /// Adresse du validateur
    pub validator: PublicKey,
    /// Montant staké
    pub amount: u64,
    /// Date de début
    pub start_date: DateTime<Utc>,
    /// Commission du validateur (0.0 à 1.0)
    pub commission_rate: f64,
    /// Délégations reçues
    pub delegated_amount: u64,
    /// Délégateurs
    pub delegators: HashMap<PublicKey, DelegatorInfo>,
    /// Performance du validateur
    pub performance_metrics: ValidatorPerformance,
    /// Récompenses totales générées
    pub total_rewards_generated: u64,
    /// Récompenses distribuées aux délégateurs
    pub rewards_distributed_to_delegators: u64,
    /// Pénalités subies
    pub penalties: Vec<ValidatorPenalty>,
    /// Statut du validateur
    pub status: ValidatorStatus,
}

/// Information sur un délégateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegatorInfo {
    /// Adresse du délégateur
    pub delegator: PublicKey,
    /// Montant délégué
    pub delegated_amount: u64,
    /// Date de délégation
    pub delegation_date: DateTime<Utc>,
    /// Récompenses accumulées
    pub accumulated_rewards: u64,
    /// Dernière réclamation
    pub last_reward_claim: Option<DateTime<Utc>>,
}

/// Proposition de gouvernance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceProposal {
    /// ID unique de la proposition
    pub proposal_id: Hash,
    /// Proposeur
    pub proposer: PublicKey,
    /// Titre de la proposition
    pub title: String,
    /// Description détaillée
    pub description: String,
    /// Type de proposition
    pub proposal_type: ProposalType,
    /// Stake requis pour proposer
    pub proposer_stake: u64,
    /// Date de création
    pub created_at: DateTime<Utc>,
    /// Date de début de vote
    pub voting_start: DateTime<Utc>,
    /// Date de fin de vote
    pub voting_end: DateTime<Utc>,
    /// Quorum requis
    pub required_quorum: u64,
    /// Seuil d'approbation (0.0 à 1.0)
    pub approval_threshold: f64,
    /// Votes pour
    pub votes_for: u64,
    /// Votes contre
    pub votes_against: u64,
    /// Votes d'abstention
    pub votes_abstain: u64,
    /// Détails des votes
    pub vote_details: HashMap<PublicKey, Vote>,
    /// Statut de la proposition
    pub status: ProposalStatus,
    /// Résultat de l'exécution (si applicable)
    pub execution_result: Option<ExecutionResult>,
}

/// Délégation de vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteDelegation {
    /// Délégateur
    pub delegator: PublicKey,
    /// Délégué (receveur du pouvoir de vote)
    pub delegate: PublicKey,
    /// Montant de pouvoir de vote délégué
    pub voting_power_delegated: u64,
    /// Date de délégation
    pub delegation_date: DateTime<Utc>,
    /// Date d'expiration (optionnelle)
    pub expiration_date: Option<DateTime<Utc>>,
    /// Statut de la délégation
    pub status: DelegationStatus,
}

/// Enregistrement de vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoteRecord {
    /// ID de la proposition
    pub proposal_id: Hash,
    /// Position du vote
    pub vote_position: VotePosition,
    /// Pouvoir de vote utilisé
    pub voting_power_used: u64,
    /// Date du vote
    pub vote_date: DateTime<Utc>,
}

/// Vote sur une proposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Votant
    pub voter: PublicKey,
    /// Position du vote
    pub position: VotePosition,
    /// Pouvoir de vote
    pub voting_power: u64,
    /// Justification (optionnelle)
    pub justification: Option<String>,
    /// Date du vote
    pub vote_date: DateTime<Utc>,
    /// Signature du vote
    pub signature: Signature,
}

/// Performance d'un validateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorPerformance {
    /// Blocs validés avec succès
    pub blocks_validated: u64,
    /// Blocs manqués
    pub blocks_missed: u64,
    /// Temps de réponse moyen (ms)
    pub average_response_time_ms: u64,
    /// Taux de disponibilité
    pub uptime_percentage: f64,
    /// Score de qualité global
    pub quality_score: f64,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Pénalité de validateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatorPenalty {
    /// Type de pénalité
    pub penalty_type: PenaltyType,
    /// Montant de la pénalité
    pub penalty_amount: u64,
    /// Raison de la pénalité
    pub reason: String,
    /// Date de la pénalité
    pub penalty_date: DateTime<Utc>,
    /// Hash de la transaction de pénalité
    pub transaction_hash: Hash,
}

/// Configuration du système de staking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingConfig {
    /// Stake minimum pour la gouvernance
    pub min_governance_stake: u64,
    /// Stake minimum pour la validation
    pub min_validator_stake: u64,
    /// Durée minimum de lock pour gouvernance (jours)
    pub min_governance_lock_days: u32,
    /// Durée minimum de lock pour validation (jours)
    pub min_validator_lock_days: u32,
    /// Taux de récompense annuel de base (%)
    pub base_annual_reward_rate: f64,
    /// Multiplicateur maximum pour durée de lock
    pub max_lock_duration_multiplier: f64,
    /// Durée de vote des propositions (jours)
    pub proposal_voting_duration_days: u32,
    /// Quorum minimum pour les propositions (%)
    pub minimum_quorum_percentage: f64,
    /// Seuil d'approbation par défaut (%)
    pub default_approval_threshold: f64,
    /// Commission maximum des validateurs (%)
    pub max_validator_commission: f64,
}

/// Métriques du système de staking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakingMetrics {
    /// Total staké pour gouvernance
    pub total_governance_staked: u64,
    /// Total staké pour validation
    pub total_validator_staked: u64,
    /// Nombre de stakeurs de gouvernance
    pub governance_stakers_count: usize,
    /// Nombre de validateurs actifs
    pub active_validators_count: usize,
    /// Propositions actives
    pub active_proposals_count: usize,
    /// Taux de participation aux votes
    pub voting_participation_rate: f64,
    /// Récompenses totales distribuées
    pub total_rewards_distributed: u64,
    /// Pénalités totales appliquées
    pub total_penalties_applied: u64,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Information sur un stake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StakeInfo {
    /// Type de stake
    pub stake_type: StakeType,
    /// Montant staké
    pub amount: u64,
    /// Pouvoir de vote
    pub voting_power: u64,
    /// Récompenses accumulées
    pub accumulated_rewards: u64,
    /// Statut
    pub status: StakeStatus,
}

/// Types de stakes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakeType {
    /// Stake de gouvernance
    Governance,
    /// Stake de validation
    Validator,
    /// Délégation à un validateur
    Delegation,
}

/// Statuts de stake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakeStatus {
    /// Actif
    Active,
    /// En cours de déstaking
    Unstaking,
    /// Locked (en période de lock)
    Locked,
    /// Slashé
    Slashed,
    /// Retiré
    Withdrawn,
}

/// Statuts de validateur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidatorStatus {
    /// Actif
    Active,
    /// Inactif temporairement
    Inactive,
    /// En probation
    Probation,
    /// Slashé
    Slashed,
    /// Retiré
    Retired,
}

/// Types de propositions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalType {
    /// Modification de paramètres
    ParameterChange,
    /// Allocation de fonds du trésor
    TreasuryAllocation,
    /// Mise à jour du protocole
    ProtocolUpgrade,
    /// Ajout/suppression de validateur
    ValidatorManagement,
    /// Proposition générale
    General,
}

/// Statuts de proposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// En cours de vote
    Voting,
    /// Approuvée
    Approved,
    /// Rejetée
    Rejected,
    /// Expirée
    Expired,
    /// Exécutée
    Executed,
    /// Échec d'exécution
    ExecutionFailed,
}

/// Position de vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotePosition {
    /// Pour
    For,
    /// Contre
    Against,
    /// Abstention
    Abstain,
}

/// Statut de délégation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DelegationStatus {
    /// Active
    Active,
    /// Expirée
    Expired,
    /// Révoquée
    Revoked,
}

/// Types de pénalités
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PenaltyType {
    /// Manquer des blocs
    MissedBlocks,
    /// Comportement malveillant
    MaliciousBehavior,
    /// Performance insuffisante
    PoorPerformance,
    /// Violation des règles
    RuleViolation,
}

/// Résultat d'exécution de proposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Succès de l'exécution
    pub success: bool,
    /// Message de résultat
    pub message: String,
    /// Hash de transaction d'exécution
    pub execution_tx_hash: Option<Hash>,
    /// Date d'exécution
    pub execution_date: DateTime<Utc>,
}

impl Default for StakingConfig {
    fn default() -> Self {
        Self {
            min_governance_stake: 1_000_000,     // 1M ARC
            min_validator_stake: 10_000_000,     // 10M ARC
            min_governance_lock_days: 30,        // 30 jours minimum
            min_validator_lock_days: 90,         // 90 jours minimum
            base_annual_reward_rate: 8.0,        // 8% annuel
            max_lock_duration_multiplier: 2.0,   // 2x max pour lock long
            proposal_voting_duration_days: 7,    // 7 jours de vote
            minimum_quorum_percentage: 15.0,     // 15% de quorum
            default_approval_threshold: 60.0,    // 60% d'approbation
            max_validator_commission: 20.0,      // 20% commission max
        }
    }
}

impl StakingSystem {
    /// Crée un nouveau système de staking
    pub fn new(config: StakingConfig) -> Self {
        Self {
            governance_stakes: HashMap::new(),
            validator_stakes: HashMap::new(),
            proposals: HashMap::new(),
            delegations: HashMap::new(),
            config,
            metrics: StakingMetrics::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Crée un stake de gouvernance
    pub fn create_governance_stake(&mut self, staker: PublicKey, amount: u64, lock_duration_days: u32, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<()> {
        if amount < self.config.min_governance_stake {
            return Err(TokenOperationError::InsufficientStake {
                required: self.config.min_governance_stake,
                provided: amount,
            });
        }

        if lock_duration_days < self.config.min_governance_lock_days {
            return Err(TokenOperationError::Internal {
                message: format!("Durée de lock minimum : {} jours", self.config.min_governance_lock_days),
            });
        }

        // Vérifier qu'il n'y a pas déjà un stake actif
        if self.governance_stakes.contains_key(&staker) {
            return Err(TokenOperationError::Internal {
                message: "Stake de gouvernance déjà actif".to_string(),
            });
        }

        // Verrouiller les tokens
        token.lock_tokens(&staker, amount, "governance_stake", tx_hash)?;

        // Calculer le multiplicateur de pouvoir de vote basé sur la durée de lock
        let lock_multiplier = 1.0 + (lock_duration_days as f64 / 365.0) * (self.config.max_lock_duration_multiplier - 1.0);
        let lock_multiplier = lock_multiplier.min(self.config.max_lock_duration_multiplier);

        let stake = GovernanceStake {
            staker: staker.clone(),
            amount,
            start_date: Utc::now(),
            lock_duration_days,
            lock_end_date: Utc::now() + Duration::days(lock_duration_days as i64),
            voting_power_multiplier: lock_multiplier,
            recent_votes: Vec::new(),
            accumulated_rewards: 0,
            last_reward_claim: None,
            status: StakeStatus::Locked,
        };

        self.governance_stakes.insert(staker, stake);
        self.metrics.total_governance_staked += amount;
        self.metrics.governance_stakers_count += 1;
        self.update_metrics();

        Ok(())
    }

    /// Crée un stake de validateur
    pub fn create_validator_stake(&mut self, validator: PublicKey, amount: u64, commission_rate: f64, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<()> {
        if amount < self.config.min_validator_stake {
            return Err(TokenOperationError::InsufficientStake {
                required: self.config.min_validator_stake,
                provided: amount,
            });
        }

        if commission_rate > self.config.max_validator_commission / 100.0 {
            return Err(TokenOperationError::Internal {
                message: format!("Commission maximum : {}%", self.config.max_validator_commission),
            });
        }

        if self.validator_stakes.contains_key(&validator) {
            return Err(TokenOperationError::Internal {
                message: "Validateur déjà enregistré".to_string(),
            });
        }

        // Verrouiller les tokens
        token.lock_tokens(&validator, amount, "validator_stake", tx_hash)?;

        let stake = ValidatorStake {
            validator: validator.clone(),
            amount,
            start_date: Utc::now(),
            commission_rate,
            delegated_amount: 0,
            delegators: HashMap::new(),
            performance_metrics: ValidatorPerformance::new(),
            total_rewards_generated: 0,
            rewards_distributed_to_delegators: 0,
            penalties: Vec::new(),
            status: ValidatorStatus::Active,
        };

        self.validator_stakes.insert(validator, stake);
        self.metrics.total_validator_staked += amount;
        self.metrics.active_validators_count += 1;
        self.update_metrics();

        Ok(())
    }

    /// Crée une proposition de gouvernance
    pub fn create_proposal(&mut self, proposer: PublicKey, title: String, description: String, proposal_type: ProposalType, required_quorum: Option<u64>, approval_threshold: Option<f64>) -> TokenOperationResult<Hash> {
        // Vérifier que le proposeur a un stake suffisant
        let stake = self.governance_stakes.get(&proposer)
            .ok_or_else(|| TokenOperationError::InsufficientStake {
                required: self.config.min_governance_stake,
                provided: 0,
            })?;

        if stake.amount < self.config.min_governance_stake {
            return Err(TokenOperationError::InsufficientStake {
                required: self.config.min_governance_stake,
                provided: stake.amount,
            });
        }

        // Générer un ID unique pour la proposition
        let proposal_id = Hash::from_bytes([
            &proposer.as_bytes()[..16],
            &title.as_bytes()[..std::cmp::min(title.len(), 16)],
            &Utc::now().timestamp().to_le_bytes(),
        ].concat().try_into().unwrap());

        let now = Utc::now();
        let voting_start = now + Duration::hours(24); // 24h de délai avant le vote
        let voting_end = voting_start + Duration::days(self.config.proposal_voting_duration_days as i64);

        // Calculer le quorum requis
        let total_voting_power = self.calculate_total_voting_power();
        let required_quorum = required_quorum.unwrap_or(
            (total_voting_power as f64 * self.config.minimum_quorum_percentage / 100.0) as u64
        );

        let proposal = GovernanceProposal {
            proposal_id,
            proposer: proposer.clone(),
            title,
            description,
            proposal_type,
            proposer_stake: stake.amount,
            created_at: now,
            voting_start,
            voting_end,
            required_quorum,
            approval_threshold: approval_threshold.unwrap_or(self.config.default_approval_threshold / 100.0),
            votes_for: 0,
            votes_against: 0,
            votes_abstain: 0,
            vote_details: HashMap::new(),
            status: ProposalStatus::Voting,
            execution_result: None,
        };

        self.proposals.insert(proposal_id, proposal);
        self.metrics.active_proposals_count += 1;
        self.update_metrics();

        Ok(proposal_id)
    }

    /// Vote sur une proposition
    pub fn vote_on_proposal(&mut self, voter: PublicKey, proposal_id: Hash, position: VotePosition, justification: Option<String>, signature: Signature) -> TokenOperationResult<()> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| TokenOperationError::ProposalNotFound { proposal_id })?;

        // Vérifier que le vote est encore ouvert
        let now = Utc::now();
        if now < proposal.voting_start || now > proposal.voting_end {
            return Err(TokenOperationError::Internal {
                message: "Période de vote fermée".to_string(),
            });
        }

        // Vérifier que le voteur n'a pas déjà voté
        if proposal.vote_details.contains_key(&voter) {
            return Err(TokenOperationError::Internal {
                message: "Vote déjà enregistré".to_string(),
            });
        }

        // Calculer le pouvoir de vote
        let voting_power = self.calculate_voting_power(&voter)?;
        if voting_power == 0 {
            return Err(TokenOperationError::InsufficientStake {
                required: self.config.min_governance_stake,
                provided: 0,
            });
        }

        // Enregistrer le vote
        let vote = Vote {
            voter: voter.clone(),
            position: position.clone(),
            voting_power,
            justification,
            vote_date: now,
            signature,
        };

        // Mettre à jour les compteurs
        match position {
            VotePosition::For => proposal.votes_for += voting_power,
            VotePosition::Against => proposal.votes_against += voting_power,
            VotePosition::Abstain => proposal.votes_abstain += voting_power,
        }

        proposal.vote_details.insert(voter.clone(), vote);

        // Mettre à jour l'historique de vote du stakeur
        if let Some(stake) = self.governance_stakes.get_mut(&voter) {
            stake.recent_votes.push(VoteRecord {
                proposal_id,
                vote_position: position,
                voting_power_used: voting_power,
                vote_date: now,
            });

            // Garder seulement les 10 derniers votes
            if stake.recent_votes.len() > 10 {
                stake.recent_votes.drain(0..stake.recent_votes.len() - 10);
            }
        }

        self.update_metrics();
        Ok(())
    }

    /// Finalise une proposition après la fin du vote
    pub fn finalize_proposal(&mut self, proposal_id: Hash) -> TokenOperationResult<ProposalStatus> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| TokenOperationError::ProposalNotFound { proposal_id })?;

        let now = Utc::now();
        if now <= proposal.voting_end {
            return Err(TokenOperationError::Internal {
                message: "Période de vote encore ouverte".to_string(),
            });
        }

        if proposal.status != ProposalStatus::Voting {
            return Err(TokenOperationError::Internal {
                message: "Proposition déjà finalisée".to_string(),
            });
        }

        let total_votes = proposal.votes_for + proposal.votes_against + proposal.votes_abstain;

        // Vérifier le quorum
        if total_votes < proposal.required_quorum {
            proposal.status = ProposalStatus::Rejected;
            self.metrics.active_proposals_count -= 1;
            self.update_metrics();
            return Ok(ProposalStatus::Rejected);
        }

        // Vérifier le seuil d'approbation
        let approval_rate = if proposal.votes_for + proposal.votes_against > 0 {
            proposal.votes_for as f64 / (proposal.votes_for + proposal.votes_against) as f64
        } else {
            0.0
        };

        if approval_rate >= proposal.approval_threshold {
            proposal.status = ProposalStatus::Approved;
        } else {
            proposal.status = ProposalStatus::Rejected;
        }

        self.metrics.active_proposals_count -= 1;
        self.update_metrics();
        Ok(proposal.status.clone())
    }

    /// Délègue à un validateur
    pub fn delegate_to_validator(&mut self, delegator: PublicKey, validator: PublicKey, amount: u64, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<()> {
        // Vérifier que le validateur existe et est actif
        let validator_stake = self.validator_stakes.get_mut(&validator)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Validateur non trouvé".to_string(),
            })?;

        if validator_stake.status != ValidatorStatus::Active {
            return Err(TokenOperationError::Internal {
                message: "Validateur inactif".to_string(),
            });
        }

        // Verrouiller les tokens du délégateur
        token.lock_tokens(&delegator, amount, "delegation", tx_hash)?;

        // Ajouter la délégation
        let delegator_info = DelegatorInfo {
            delegator: delegator.clone(),
            delegated_amount: amount,
            delegation_date: Utc::now(),
            accumulated_rewards: 0,
            last_reward_claim: None,
        };

        validator_stake.delegators.insert(delegator, delegator_info);
        validator_stake.delegated_amount += amount;
        self.metrics.total_validator_staked += amount;
        self.update_metrics();

        Ok(())
    }

    /// Calcule le pouvoir de vote d'une adresse
    pub fn calculate_voting_power(&self, address: &PublicKey) -> TokenOperationResult<u64> {
        let mut total_power = 0;

        // Pouvoir de vote du stake de gouvernance
        if let Some(stake) = self.governance_stakes.get(address) {
            if stake.status == StakeStatus::Active || stake.status == StakeStatus::Locked {
                total_power += (stake.amount as f64 * stake.voting_power_multiplier) as u64;
            }
        }

        // Pouvoir de vote délégué
        for delegation in self.delegations.values() {
            if delegation.delegate == *address && delegation.status == DelegationStatus::Active {
                total_power += delegation.voting_power_delegated;
            }
        }

        Ok(total_power)
    }

    /// Calcule le pouvoir de vote total du système
    fn calculate_total_voting_power(&self) -> u64 {
        self.governance_stakes.values()
            .filter(|stake| stake.status == StakeStatus::Active || stake.status == StakeStatus::Locked)
            .map(|stake| (stake.amount as f64 * stake.voting_power_multiplier) as u64)
            .sum()
    }

    /// Distribue les récompenses de staking
    pub fn distribute_staking_rewards(&mut self, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let mut total_distributed = 0;

        // Récompenses de gouvernance
        for stake in self.governance_stakes.values_mut() {
            if stake.status == StakeStatus::Active || stake.status == StakeStatus::Locked {
                let reward = self.calculate_governance_reward(stake)?;
                if reward > 0 {
                    token.mint(&stake.staker, reward, tx_hash)?;
                    stake.accumulated_rewards += reward;
                    stake.last_reward_claim = Some(Utc::now());
                    total_distributed += reward;
                }
            }
        }

        // Récompenses de validation
        for stake in self.validator_stakes.values_mut() {
            if stake.status == ValidatorStatus::Active {
                let (validator_reward, delegator_rewards) = self.calculate_validator_rewards(stake)?;
                
                // Récompense du validateur
                if validator_reward > 0 {
                    token.mint(&stake.validator, validator_reward, tx_hash)?;
                    stake.total_rewards_generated += validator_reward;
                    total_distributed += validator_reward;
                }

                // Récompenses des délégateurs
                for (delegator, reward) in delegator_rewards {
                    if reward > 0 {
                        token.mint(&delegator, reward, tx_hash)?;
                        if let Some(delegator_info) = stake.delegators.get_mut(&delegator) {
                            delegator_info.accumulated_rewards += reward;
                            delegator_info.last_reward_claim = Some(Utc::now());
                        }
                        stake.rewards_distributed_to_delegators += reward;
                        total_distributed += reward;
                    }
                }
            }
        }

        self.metrics.total_rewards_distributed += total_distributed;
        self.update_metrics();
        Ok(total_distributed)
    }

    /// Calcule les récompenses de gouvernance pour un stake
    fn calculate_governance_reward(&self, stake: &GovernanceStake) -> TokenOperationResult<u64> {
        let now = Utc::now();
        let last_claim = stake.last_reward_claim.unwrap_or(stake.start_date);
        let days_since_claim = (now - last_claim).num_days();

        if days_since_claim < 30 {
            return Ok(0); // Récompenses mensuelles
        }

        // Calcul basé sur le taux annuel et le multiplicateur de lock
        let annual_rate = self.config.base_annual_reward_rate / 100.0;
        let monthly_rate = annual_rate / 12.0;
        let base_reward = (stake.amount as f64 * monthly_rate) as u64;
        let final_reward = (base_reward as f64 * stake.voting_power_multiplier) as u64;

        Ok(final_reward)
    }

    /// Calcule les récompenses de validation
    fn calculate_validator_rewards(&self, stake: &ValidatorStake) -> TokenOperationResult<(u64, Vec<(PublicKey, u64)>)> {
        let total_stake = stake.amount + stake.delegated_amount;
        let annual_rate = self.config.base_annual_reward_rate / 100.0;
        let monthly_rate = annual_rate / 12.0;
        
        // Bonus de performance
        let performance_multiplier = stake.performance_metrics.quality_score;
        
        let total_monthly_reward = (total_stake as f64 * monthly_rate * performance_multiplier) as u64;
        
        // Commission du validateur
        let validator_commission = (total_monthly_reward as f64 * stake.commission_rate) as u64;
        let remaining_for_delegators = total_monthly_reward - validator_commission;
        
        // Récompense propre du validateur (sur son propre stake)
        let validator_own_reward = if total_stake > 0 {
            (remaining_for_delegators * stake.amount / total_stake) + validator_commission
        } else {
            total_monthly_reward
        };
        
        // Répartition pour les délégateurs
        let mut delegator_rewards = Vec::new();
        for (delegator, info) in &stake.delegators {
            let delegator_reward = if total_stake > 0 {
                remaining_for_delegators * info.delegated_amount / total_stake
            } else {
                0
            };
            delegator_rewards.push((delegator.clone(), delegator_reward));
        }

        Ok((validator_own_reward, delegator_rewards))
    }

    /// Met à jour les métriques du système
    fn update_metrics(&mut self) {
        self.metrics.governance_stakers_count = self.governance_stakes.len();
        self.metrics.active_validators_count = self.validator_stakes.values()
            .filter(|v| v.status == ValidatorStatus::Active)
            .count();
        self.metrics.active_proposals_count = self.proposals.values()
            .filter(|p| p.status == ProposalStatus::Voting)
            .count();
        
        // Calculer le taux de participation aux votes
        let total_proposals = self.proposals.len();
        if total_proposals > 0 {
            let total_participation: usize = self.proposals.values()
                .map(|p| p.vote_details.len())
                .sum();
            self.metrics.voting_participation_rate = total_participation as f64 / total_proposals as f64;
        }

        self.metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();
    }

    /// Obtient les informations de stake pour une adresse
    pub fn get_stake_info(&self, address: &PublicKey) -> Option<StakeInfo> {
        if let Some(gov_stake) = self.governance_stakes.get(address) {
            Some(StakeInfo {
                stake_type: StakeType::Governance,
                amount: gov_stake.amount,
                voting_power: (gov_stake.amount as f64 * gov_stake.voting_power_multiplier) as u64,
                accumulated_rewards: gov_stake.accumulated_rewards,
                status: gov_stake.status.clone(),
            })
        } else if let Some(val_stake) = self.validator_stakes.get(address) {
            Some(StakeInfo {
                stake_type: StakeType::Validator,
                amount: val_stake.amount,
                voting_power: val_stake.amount, // Les validateurs ont un pouvoir de vote égal à leur stake
                accumulated_rewards: val_stake.total_rewards_generated,
                status: StakeStatus::Active, // Conversion simplifiée
            })
        } else {
            None
        }
    }
}

impl ValidatorPerformance {
    fn new() -> Self {
        Self {
            blocks_validated: 0,
            blocks_missed: 0,
            average_response_time_ms: 0,
            uptime_percentage: 100.0,
            quality_score: 1.0,
            last_updated: Utc::now(),
        }
    }
}

impl StakingMetrics {
    fn new() -> Self {
        Self {
            total_governance_staked: 0,
            total_validator_staked: 0,
            governance_stakers_count: 0,
            active_validators_count: 0,
            active_proposals_count: 0,
            voting_participation_rate: 0.0,
            total_rewards_distributed: 0,
            total_penalties_applied: 0,
            last_updated: Utc::now(),
        }
    }
}

impl Default for StakingSystem {
    fn default() -> Self {
        Self::new(StakingConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_staking_system_creation() {
        let config = StakingConfig::default();
        let system = StakingSystem::new(config);
        
        assert_eq!(system.governance_stakes.len(), 0);
        assert_eq!(system.validator_stakes.len(), 0);
        assert_eq!(system.metrics.total_governance_staked, 0);
    }

    #[test]
    fn test_governance_stake_creation() {
        let mut system = StakingSystem::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let staker = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Mint tokens to staker
        token.mint(&staker, 2_000_000, tx_hash).unwrap();

        let result = system.create_governance_stake(
            staker.clone(),
            1_500_000,
            90, // 90 days lock
            &mut token,
            tx_hash,
        );

        assert!(result.is_ok());
        assert!(system.governance_stakes.contains_key(&staker));
        assert_eq!(system.metrics.total_governance_staked, 1_500_000);
        assert_eq!(system.metrics.governance_stakers_count, 1);
    }

    #[test]
    fn test_validator_stake_creation() {
        let mut system = StakingSystem::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let validator = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Mint tokens to validator
        token.mint(&validator, 15_000_000, tx_hash).unwrap();

        let result = system.create_validator_stake(
            validator.clone(),
            12_000_000,
            0.05, // 5% commission
            &mut token,
            tx_hash,
        );

        assert!(result.is_ok());
        assert!(system.validator_stakes.contains_key(&validator));
        assert_eq!(system.metrics.total_validator_staked, 12_000_000);
        assert_eq!(system.metrics.active_validators_count, 1);
    }

    #[test]
    fn test_proposal_creation() {
        let mut system = StakingSystem::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let proposer = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Setup governance stake
        token.mint(&proposer, 2_000_000, tx_hash).unwrap();
        system.create_governance_stake(proposer.clone(), 1_500_000, 90, &mut token, tx_hash).unwrap();

        let proposal_id = system.create_proposal(
            proposer,
            "Test Proposal".to_string(),
            "This is a test proposal".to_string(),
            ProposalType::General,
            None,
            None,
        ).unwrap();

        assert!(system.proposals.contains_key(&proposal_id));
        assert_eq!(system.metrics.active_proposals_count, 1);
    }

    #[test]
    fn test_voting_power_calculation() {
        let mut system = StakingSystem::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let staker = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Setup governance stake with lock
        token.mint(&staker, 2_000_000, tx_hash).unwrap();
        system.create_governance_stake(staker.clone(), 1_500_000, 365, &mut token, tx_hash).unwrap(); // 1 year lock

        let voting_power = system.calculate_voting_power(&staker).unwrap();
        
        // Should be more than base amount due to lock multiplier
        assert!(voting_power > 1_500_000);
        assert!(voting_power <= 3_000_000); // Max 2x multiplier
    }

    #[test]
    fn test_delegation() {
        let mut system = StakingSystem::default();
        let mut token = ARCToken::new();
        let validator_keypair = generate_keypair().unwrap();
        let delegator_keypair = generate_keypair().unwrap();
        let validator = validator_keypair.public_key().clone();
        let delegator = delegator_keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Setup validator
        token.mint(&validator, 15_000_000, tx_hash).unwrap();
        system.create_validator_stake(validator.clone(), 12_000_000, 0.05, &mut token, tx_hash).unwrap();

        // Setup delegation
        token.mint(&delegator, 5_000_000, tx_hash).unwrap();
        let result = system.delegate_to_validator(delegator.clone(), validator.clone(), 3_000_000, &mut token, tx_hash);

        assert!(result.is_ok());
        
        if let Some(validator_stake) = system.validator_stakes.get(&validator) {
            assert_eq!(validator_stake.delegated_amount, 3_000_000);
            assert!(validator_stake.delegators.contains_key(&delegator));
        }
    }
}