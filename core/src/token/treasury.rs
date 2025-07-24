//! Système de treasury et gestion des fonds communautaires
//!
//! Gère les 20% de tokens alloués à la réserve communautaire :
//! - Propositions de financement communautaire
//! - Système de vote pour l'allocation des fonds
//! - Gestion des budgets et des dépenses
//! - Suivi des projets financés
//! - Mécanismes de transparence et d'audit

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey, Signature};
use super::{TokenOperationResult, TokenOperationError, ARCToken, COMMUNITY_RESERVE};

/// Système de treasury principal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Treasury {
    /// Fonds disponibles
    pub available_funds: u64,
    /// Fonds alloués mais non encore déboursés
    pub allocated_funds: u64,
    /// Fonds déjà déboursés
    pub disbursed_funds: u64,
    /// Propositions de financement
    pub proposals: HashMap<Hash, TreasuryProposal>,
    /// Budgets approuvés
    pub approved_budgets: HashMap<Hash, Budget>,
    /// Projets en cours
    pub active_projects: HashMap<Hash, Project>,
    /// Comités de gouvernance
    pub governance_committees: HashMap<Hash, GovernanceCommittee>,
    /// Configuration du treasury
    pub config: TreasuryConfig,
    /// Métriques et statistiques
    pub metrics: TreasuryMetrics,
    /// Historique des transactions
    pub transaction_history: Vec<TreasuryTransaction>,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Proposition de financement du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryProposal {
    /// ID unique de la proposition
    pub proposal_id: Hash,
    /// Proposeur
    pub proposer: PublicKey,
    /// Titre de la proposition
    pub title: String,
    /// Description détaillée
    pub description: String,
    /// Catégorie de la proposition
    pub category: ProposalCategory,
    /// Montant demandé
    pub requested_amount: u64,
    /// Budget détaillé
    pub budget_breakdown: Vec<BudgetItem>,
    /// Bénéficiaire des fonds
    pub beneficiary: PublicKey,
    /// Jalons du projet
    pub milestones: Vec<Milestone>,
    /// Critères de succès
    pub success_criteria: Vec<String>,
    /// Date de soumission
    pub submitted_at: DateTime<Utc>,
    /// Période de vote
    pub voting_period: VotingPeriod,
    /// Votes reçus
    pub votes: HashMap<PublicKey, TreasuryVote>,
    /// Statut de la proposition
    pub status: ProposalStatus,
    /// Comité assigné pour évaluation
    pub assigned_committee: Option<Hash>,
    /// Rapport d'évaluation
    pub evaluation_report: Option<EvaluationReport>,
    /// Résultat du vote
    pub voting_result: Option<VotingResult>,
}

/// Budget approuvé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    /// ID du budget
    pub budget_id: Hash,
    /// ID de la proposition associée
    pub proposal_id: Hash,
    /// Montant total approuvé
    pub total_amount: u64,
    /// Montant déjà déboursé
    pub disbursed_amount: u64,
    /// Montant restant
    pub remaining_amount: u64,
    /// Planning de débours
    pub disbursement_schedule: Vec<DisbursementMilestone>,
    /// Date d'approbation
    pub approved_at: DateTime<Utc>,
    /// Date limite d'utilisation
    pub expiry_date: DateTime<Utc>,
    /// Statut du budget
    pub status: BudgetStatus,
}

/// Projet financé par le treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// ID du projet
    pub project_id: Hash,
    /// ID du budget associé
    pub budget_id: Hash,
    /// Gestionnaire du projet
    pub project_manager: PublicKey,
    /// Équipe du projet
    pub team_members: Vec<PublicKey>,
    /// Progression actuelle
    pub current_progress: f64, // 0.0 à 1.0
    /// Jalons complétés
    pub completed_milestones: Vec<Hash>,
    /// Prochains jalons
    pub upcoming_milestones: Vec<Hash>,
    /// Rapports de progression
    pub progress_reports: Vec<ProgressReport>,
    /// Dépenses effectuées
    pub expenses: Vec<Expense>,
    /// Date de début
    pub start_date: DateTime<Utc>,
    /// Date prévue de fin
    pub expected_end_date: DateTime<Utc>,
    /// Statut du projet
    pub status: ProjectStatus,
}

/// Comité de gouvernance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceCommittee {
    /// ID du comité
    pub committee_id: Hash,
    /// Nom du comité
    pub name: String,
    /// Description du rôle
    pub description: String,
    /// Domaines d'expertise
    pub expertise_areas: Vec<String>,
    /// Membres du comité
    pub members: Vec<CommitteeMember>,
    /// Propositions assignées
    pub assigned_proposals: Vec<Hash>,
    /// Rapports produits
    pub evaluation_reports: Vec<Hash>,
    /// Date de création
    pub created_at: DateTime<Utc>,
    /// Statut du comité
    pub status: CommitteeStatus,
}

/// Membre d'un comité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitteeMember {
    /// Adresse du membre
    pub member: PublicKey,
    /// Rôle dans le comité
    pub role: CommitteeRole,
    /// Domaines d'expertise
    pub expertise: Vec<String>,
    /// Date de nomination
    pub appointed_at: DateTime<Utc>,
    /// Mandat (durée en mois)
    pub term_months: u32,
    /// Date de fin de mandat
    pub term_end_date: DateTime<Utc>,
    /// Statut du membre
    pub status: MemberStatus,
}

/// Transaction du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryTransaction {
    /// ID de la transaction
    pub transaction_id: Hash,
    /// Type de transaction
    pub transaction_type: TransactionType,
    /// Montant
    pub amount: u64,
    /// Émetteur
    pub from: Option<PublicKey>,
    /// Destinataire
    pub to: Option<PublicKey>,
    /// Référence (proposition, projet, etc.)
    pub reference: Option<Hash>,
    /// Description
    pub description: String,
    /// Date de la transaction
    pub timestamp: DateTime<Utc>,
    /// Hash de la transaction blockchain
    pub blockchain_tx_hash: Hash,
}

/// Configuration du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryConfig {
    /// Montant minimum pour une proposition
    pub min_proposal_amount: u64,
    /// Montant maximum pour une proposition
    pub max_proposal_amount: u64,
    /// Durée de vote par défaut (jours)
    pub default_voting_duration_days: u32,
    /// Quorum minimum pour les votes (%)
    pub minimum_quorum_percentage: f64,
    /// Seuil d'approbation (%)
    pub approval_threshold_percentage: f64,
    /// Stake minimum requis pour proposer
    pub min_proposer_stake: u64,
    /// Nombre maximum de propositions actives
    pub max_active_proposals: usize,
    /// Durée maximum d'un projet (mois)
    pub max_project_duration_months: u32,
    /// Pourcentage maximum du treasury par proposition
    pub max_treasury_percentage_per_proposal: f64,
}

/// Métriques du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryMetrics {
    /// Nombre total de propositions
    pub total_proposals: usize,
    /// Propositions approuvées
    pub approved_proposals: usize,
    /// Propositions rejetées
    pub rejected_proposals: usize,
    /// Projets actifs
    pub active_projects: usize,
    /// Projets complétés
    pub completed_projects: usize,
    /// Taux de succès des projets
    pub project_success_rate: f64,
    /// Utilisation des fonds (%)
    pub fund_utilization_rate: f64,
    /// ROI moyen des projets
    pub average_project_roi: f64,
    /// Délai moyen d'approbation (jours)
    pub average_approval_time_days: f64,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Vote sur une proposition de treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryVote {
    /// Votant
    pub voter: PublicKey,
    /// Position du vote
    pub position: VotePosition,
    /// Pouvoir de vote
    pub voting_power: u64,
    /// Justification
    pub justification: Option<String>,
    /// Date du vote
    pub vote_date: DateTime<Utc>,
    /// Signature
    pub signature: Signature,
}

/// Rapport d'évaluation d'une proposition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationReport {
    /// ID du rapport
    pub report_id: Hash,
    /// Comité évaluateur
    pub evaluating_committee: Hash,
    /// Score global (0.0 à 1.0)
    pub overall_score: f64,
    /// Évaluation technique
    pub technical_assessment: AssessmentSection,
    /// Évaluation financière
    pub financial_assessment: AssessmentSection,
    /// Évaluation de l'impact
    pub impact_assessment: AssessmentSection,
    /// Recommandation
    pub recommendation: Recommendation,
    /// Commentaires détaillés
    pub detailed_comments: String,
    /// Conditions d'approbation
    pub approval_conditions: Vec<String>,
    /// Date du rapport
    pub report_date: DateTime<Utc>,
    /// Membres ayant signé
    pub signatories: Vec<PublicKey>,
}

/// Section d'évaluation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssessmentSection {
    /// Score de la section (0.0 à 1.0)
    pub score: f64,
    /// Commentaires
    pub comments: String,
    /// Critères évalués
    pub criteria_scores: HashMap<String, f64>,
}

/// Poste budgétaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetItem {
    /// Nom du poste
    pub item_name: String,
    /// Description
    pub description: String,
    /// Montant
    pub amount: u64,
    /// Catégorie
    pub category: BudgetCategory,
    /// Justification
    pub justification: String,
}

/// Jalon d'un projet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    /// ID du jalon
    pub milestone_id: Hash,
    /// Nom du jalon
    pub name: String,
    /// Description
    pub description: String,
    /// Montant à débourser à ce jalon
    pub payment_amount: u64,
    /// Critères de validation
    pub completion_criteria: Vec<String>,
    /// Date prévue
    pub target_date: DateTime<Utc>,
    /// Date de completion (si complété)
    pub completed_date: Option<DateTime<Utc>>,
    /// Statut du jalon
    pub status: MilestoneStatus,
}

/// Jalon de débours
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisbursementMilestone {
    /// Référence au jalon du projet
    pub milestone_id: Hash,
    /// Montant à débourser
    pub amount: u64,
    /// Date prévue de débours
    pub scheduled_date: DateTime<Utc>,
    /// Date effective de débours
    pub actual_disbursement_date: Option<DateTime<Utc>>,
    /// Conditions de débours
    pub conditions: Vec<String>,
    /// Statut du débours
    pub status: DisbursementStatus,
}

/// Rapport de progression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgressReport {
    /// ID du rapport
    pub report_id: Hash,
    /// Période couverte
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    /// Progression générale (0.0 à 1.0)
    pub overall_progress: f64,
    /// Réalisations de la période
    pub achievements: Vec<String>,
    /// Défis rencontrés
    pub challenges: Vec<String>,
    /// Prochaines étapes
    pub next_steps: Vec<String>,
    /// Modifications du planning
    pub schedule_changes: Vec<ScheduleChange>,
    /// Dépenses de la période
    pub period_expenses: u64,
    /// Date de soumission
    pub submitted_at: DateTime<Utc>,
    /// Validé par le comité
    pub committee_approved: bool,
}

/// Dépense d'un projet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Expense {
    /// ID de la dépense
    pub expense_id: Hash,
    /// Poste budgétaire associé
    pub budget_item: String,
    /// Montant
    pub amount: u64,
    /// Description
    pub description: String,
    /// Bénéficiaire
    pub recipient: PublicKey,
    /// Date de la dépense
    pub expense_date: DateTime<Utc>,
    /// Justificatifs
    pub supporting_documents: Vec<String>,
    /// Statut de validation
    pub approval_status: ApprovalStatus,
}

/// Modification de planning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleChange {
    /// Élément modifié
    pub item: String,
    /// Date originale
    pub original_date: DateTime<Utc>,
    /// Nouvelle date
    pub new_date: DateTime<Utc>,
    /// Raison du changement
    pub reason: String,
}

/// Résultat de vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingResult {
    /// Votes pour
    pub votes_for: u64,
    /// Votes contre
    pub votes_against: u64,
    /// Abstentions
    pub votes_abstain: u64,
    /// Quorum atteint
    pub quorum_reached: bool,
    /// Seuil d'approbation atteint
    pub approval_threshold_met: bool,
    /// Résultat final
    pub result: bool, // true = approuvé, false = rejeté
    /// Date de finalisation
    pub finalized_at: DateTime<Utc>,
}

/// Période de vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VotingPeriod {
    /// Date de début
    pub start_date: DateTime<Utc>,
    /// Date de fin
    pub end_date: DateTime<Utc>,
    /// Type de vote
    pub voting_type: VotingType,
}

/// Types d'énumérations

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalCategory {
    Development,
    Research,
    Marketing,
    Infrastructure,
    Community,
    Education,
    Partnership,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    Draft,
    Submitted,
    UnderReview,
    Voting,
    Approved,
    Rejected,
    Expired,
    Withdrawn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetStatus {
    Active,
    Partially_Disbursed,
    Fully_Disbursed,
    Expired,
    Frozen,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProjectStatus {
    Planning,
    Active,
    OnHold,
    Completed,
    Cancelled,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitteeStatus {
    Active,
    Inactive,
    Disbanded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CommitteeRole {
    Chairman,
    ViceChairman,
    Member,
    Secretary,
    TechnicalLead,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MemberStatus {
    Active,
    Inactive,
    Resigned,
    Terminated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Allocation,
    Disbursement,
    Refund,
    Penalty,
    Interest,
    Fee,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotePosition {
    For,
    Against,
    Abstain,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Recommendation {
    StronglyApprove,
    Approve,
    ConditionalApproval,
    Reject,
    StronglyReject,
    RequiresRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BudgetCategory {
    Personnel,
    Equipment,
    Software,
    Services,
    Travel,
    Marketing,
    Operations,
    Contingency,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MilestoneStatus {
    Planned,
    InProgress,
    Completed,
    Delayed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DisbursementStatus {
    Scheduled,
    Ready,
    Processed,
    Delayed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ApprovalStatus {
    Pending,
    Approved,
    Rejected,
    RequiresDocumentation,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VotingType {
    Simple,
    Weighted,
    Quadratic,
}

impl Default for TreasuryConfig {
    fn default() -> Self {
        Self {
            min_proposal_amount: 10_000,                    // 10K ARC minimum
            max_proposal_amount: COMMUNITY_RESERVE / 100,   // 1% du treasury maximum
            default_voting_duration_days: 14,               // 2 semaines de vote
            minimum_quorum_percentage: 10.0,                // 10% de quorum
            approval_threshold_percentage: 60.0,            // 60% pour approuver
            min_proposer_stake: 100_000,                    // 100K ARC pour proposer
            max_active_proposals: 20,                       // Max 20 propositions actives
            max_project_duration_months: 24,                // Max 2 ans par projet
            max_treasury_percentage_per_proposal: 5.0,      // Max 5% du treasury
        }
    }
}

impl Treasury {
    /// Crée un nouveau treasury
    pub fn new(config: TreasuryConfig) -> Self {
        Self {
            available_funds: COMMUNITY_RESERVE,
            allocated_funds: 0,
            disbursed_funds: 0,
            proposals: HashMap::new(),
            approved_budgets: HashMap::new(),
            active_projects: HashMap::new(),
            governance_committees: HashMap::new(),
            config,
            metrics: TreasuryMetrics::new(),
            transaction_history: Vec::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Soumet une nouvelle proposition
    pub fn submit_proposal(&mut self, proposer: PublicKey, title: String, description: String, category: ProposalCategory, requested_amount: u64, budget_breakdown: Vec<BudgetItem>, beneficiary: PublicKey, milestones: Vec<Milestone>) -> TokenOperationResult<Hash> {
        // Validations
        if requested_amount < self.config.min_proposal_amount {
            return Err(TokenOperationError::InvalidAmount { amount: requested_amount });
        }

        if requested_amount > self.config.max_proposal_amount {
            return Err(TokenOperationError::InvalidAmount { amount: requested_amount });
        }

        let treasury_percentage = (requested_amount as f64 / self.available_funds as f64) * 100.0;
        if treasury_percentage > self.config.max_treasury_percentage_per_proposal {
            return Err(TokenOperationError::Internal {
                message: format!("Demande trop importante : {:.1}% du treasury", treasury_percentage),
            });
        }

        if self.proposals.values().filter(|p| matches!(p.status, ProposalStatus::Voting | ProposalStatus::UnderReview)).count() >= self.config.max_active_proposals {
            return Err(TokenOperationError::Internal {
                message: "Trop de propositions actives".to_string(),
            });
        }

        // Générer ID de proposition
        let proposal_id = Hash::from_bytes([
            &proposer.as_bytes()[..16],
            &title.as_bytes()[..std::cmp::min(title.len(), 16)],
            &Utc::now().timestamp().to_le_bytes(),
        ].concat().try_into().unwrap());

        let now = Utc::now();
        let voting_start = now + Duration::days(3); // 3 jours de review
        let voting_end = voting_start + Duration::days(self.config.default_voting_duration_days as i64);

        let proposal = TreasuryProposal {
            proposal_id,
            proposer,
            title,
            description,
            category,
            requested_amount,
            budget_breakdown,
            beneficiary,
            milestones,
            submitted_at: now,
            voting_period: VotingPeriod {
                start_date: voting_start,
                end_date: voting_end,
                voting_type: VotingType::Weighted,
            },
            votes: HashMap::new(),
            status: ProposalStatus::Submitted,
            assigned_committee: None,
            evaluation_report: None,
            voting_result: None,
        };

        self.proposals.insert(proposal_id, proposal);
        self.metrics.total_proposals += 1;
        self.update_metrics();

        Ok(proposal_id)
    }

    /// Vote sur une proposition
    pub fn vote_on_proposal(&mut self, voter: PublicKey, proposal_id: Hash, position: VotePosition, voting_power: u64, justification: Option<String>, signature: Signature) -> TokenOperationResult<()> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| TokenOperationError::ProposalNotFound { proposal_id })?;

        let now = Utc::now();
        if now < proposal.voting_period.start_date || now > proposal.voting_period.end_date {
            return Err(TokenOperationError::Internal {
                message: "Période de vote fermée".to_string(),
            });
        }

        if proposal.status != ProposalStatus::Voting {
            return Err(TokenOperationError::Internal {
                message: "Proposition non ouverte au vote".to_string(),
            });
        }

        if proposal.votes.contains_key(&voter) {
            return Err(TokenOperationError::Internal {
                message: "Vote déjà enregistré".to_string(),
            });
        }

        let vote = TreasuryVote {
            voter: voter.clone(),
            position,
            voting_power,
            justification,
            vote_date: now,
            signature,
        };

        proposal.votes.insert(voter, vote);
        self.update_metrics();

        Ok(())
    }

    /// Finalise une proposition après le vote
    pub fn finalize_proposal(&mut self, proposal_id: Hash) -> TokenOperationResult<bool> {
        let proposal = self.proposals.get_mut(&proposal_id)
            .ok_or_else(|| TokenOperationError::ProposalNotFound { proposal_id })?;

        let now = Utc::now();
        if now <= proposal.voting_period.end_date {
            return Err(TokenOperationError::Internal {
                message: "Période de vote encore ouverte".to_string(),
            });
        }

        if proposal.status != ProposalStatus::Voting {
            return Err(TokenOperationError::Internal {
                message: "Proposition non en cours de vote".to_string(),
            });
        }

        // Calculer les résultats
        let mut votes_for = 0;
        let mut votes_against = 0;
        let mut votes_abstain = 0;

        for vote in proposal.votes.values() {
            match vote.position {
                VotePosition::For => votes_for += vote.voting_power,
                VotePosition::Against => votes_against += vote.voting_power,
                VotePosition::Abstain => votes_abstain += vote.voting_power,
            }
        }

        let total_votes = votes_for + votes_against + votes_abstain;
        let total_eligible_votes = self.calculate_total_eligible_voting_power();
        let quorum_percentage = (total_votes as f64 / total_eligible_votes as f64) * 100.0;
        let quorum_reached = quorum_percentage >= self.config.minimum_quorum_percentage;

        let approval_rate = if votes_for + votes_against > 0 {
            (votes_for as f64 / (votes_for + votes_against) as f64) * 100.0
        } else {
            0.0
        };
        let approval_threshold_met = approval_rate >= self.config.approval_threshold_percentage;

        let approved = quorum_reached && approval_threshold_met;

        let voting_result = VotingResult {
            votes_for,
            votes_against,
            votes_abstain,
            quorum_reached,
            approval_threshold_met,
            result: approved,
            finalized_at: now,
        };

        proposal.voting_result = Some(voting_result);

        if approved {
            proposal.status = ProposalStatus::Approved;
            self.approve_proposal(proposal_id)?;
            self.metrics.approved_proposals += 1;
        } else {
            proposal.status = ProposalStatus::Rejected;
            self.metrics.rejected_proposals += 1;
        }

        self.update_metrics();
        Ok(approved)
    }

    /// Approuve une proposition et crée le budget associé
    fn approve_proposal(&mut self, proposal_id: Hash) -> TokenOperationResult<()> {
        let proposal = self.proposals.get(&proposal_id)
            .ok_or_else(|| TokenOperationError::ProposalNotFound { proposal_id })?;

        // Vérifier la disponibilité des fonds
        if self.available_funds < proposal.requested_amount {
            return Err(TokenOperationError::InsufficientRewardPool);
        }

        // Créer le budget
        let budget_id = Hash::from_bytes([
            &proposal_id.as_bytes()[..16],
            b"budget",
            &Utc::now().timestamp().to_le_bytes()[..10],
        ].concat().try_into().unwrap());

        let mut disbursement_schedule = Vec::new();
        for milestone in &proposal.milestones {
            disbursement_schedule.push(DisbursementMilestone {
                milestone_id: milestone.milestone_id,
                amount: milestone.payment_amount,
                scheduled_date: milestone.target_date,
                actual_disbursement_date: None,
                conditions: milestone.completion_criteria.clone(),
                status: DisbursementStatus::Scheduled,
            });
        }

        let budget = Budget {
            budget_id,
            proposal_id,
            total_amount: proposal.requested_amount,
            disbursed_amount: 0,
            remaining_amount: proposal.requested_amount,
            disbursement_schedule,
            approved_at: Utc::now(),
            expiry_date: Utc::now() + Duration::days((self.config.max_project_duration_months * 30) as i64),
            status: BudgetStatus::Active,
        };

        // Allouer les fonds
        self.available_funds -= proposal.requested_amount;
        self.allocated_funds += proposal.requested_amount;

        self.approved_budgets.insert(budget_id, budget);

        // Créer le projet
        self.create_project_from_proposal(proposal)?;

        // Enregistrer la transaction
        self.record_transaction(TransactionType::Allocation, proposal.requested_amount, None, Some(proposal.beneficiary.clone()), Some(proposal_id), format!("Allocation pour: {}", proposal.title), Hash::zero());

        Ok(())
    }

    /// Crée un projet à partir d'une proposition approuvée
    fn create_project_from_proposal(&mut self, proposal: &TreasuryProposal) -> TokenOperationResult<()> {
        let project_id = Hash::from_bytes([
            &proposal.proposal_id.as_bytes()[..16],
            b"project",
            &Utc::now().timestamp().to_le_bytes()[..10],
        ].concat().try_into().unwrap());

        let budget_id = self.approved_budgets.iter()
            .find(|(_, budget)| budget.proposal_id == proposal.proposal_id)
            .map(|(id, _)| *id)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Budget associé non trouvé".to_string(),
            })?;

        let project = Project {
            project_id,
            budget_id,
            project_manager: proposal.beneficiary.clone(),
            team_members: vec![proposal.beneficiary.clone()],
            current_progress: 0.0,
            completed_milestones: Vec::new(),
            upcoming_milestones: proposal.milestones.iter().map(|m| m.milestone_id).collect(),
            progress_reports: Vec::new(),
            expenses: Vec::new(),
            start_date: Utc::now(),
            expected_end_date: proposal.milestones.iter()
                .map(|m| m.target_date)
                .max()
                .unwrap_or(Utc::now() + Duration::days(365)),
            status: ProjectStatus::Planning,
        };

        self.active_projects.insert(project_id, project);
        self.metrics.active_projects += 1;
        self.update_metrics();

        Ok(())
    }

    /// Débourse des fonds pour un jalon complété
    pub fn disburse_milestone_payment(&mut self, project_id: Hash, milestone_id: Hash, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let project = self.active_projects.get_mut(&project_id)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Projet non trouvé".to_string(),
            })?;

        let budget = self.approved_budgets.get_mut(&project.budget_id)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Budget non trouvé".to_string(),
            })?;

        // Trouver le jalon dans le planning de débours
        let disbursement = budget.disbursement_schedule.iter_mut()
            .find(|d| d.milestone_id == milestone_id)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Jalon de débours non trouvé".to_string(),
            })?;

        if disbursement.status != DisbursementStatus::Ready {
            return Err(TokenOperationError::Internal {
                message: "Jalon non prêt pour débours".to_string(),
            });
        }

        if budget.remaining_amount < disbursement.amount {
            return Err(TokenOperationError::InsufficientRewardPool);
        }

        // Effectuer le disbursement
        token.mint(&project.project_manager, disbursement.amount, tx_hash)?;

        // Mettre à jour les montants
        budget.disbursed_amount += disbursement.amount;
        budget.remaining_amount -= disbursement.amount;
        self.allocated_funds -= disbursement.amount;
        self.disbursed_funds += disbursement.amount;

        // Mettre à jour le statut
        disbursement.status = DisbursementStatus::Processed;
        disbursement.actual_disbursement_date = Some(Utc::now());

        // Marquer le jalon comme complété dans le projet
        project.completed_milestones.push(milestone_id);
        project.upcoming_milestones.retain(|&m| m != milestone_id);

        // Mettre à jour la progression
        let total_milestones = project.completed_milestones.len() + project.upcoming_milestones.len();
        if total_milestones > 0 {
            project.current_progress = project.completed_milestones.len() as f64 / total_milestones as f64;
        }

        // Enregistrer la transaction
        self.record_transaction(
            TransactionType::Disbursement,
            disbursement.amount,
            None,
            Some(project.project_manager.clone()),
            Some(project_id),
            format!("Débours jalon: {}", milestone_id),
            tx_hash,
        );

        self.update_metrics();
        Ok(disbursement.amount)
    }

    /// Enregistre une transaction
    fn record_transaction(&mut self, transaction_type: TransactionType, amount: u64, from: Option<PublicKey>, to: Option<PublicKey>, reference: Option<Hash>, description: String, blockchain_tx_hash: Hash) {
        let transaction_id = Hash::from_bytes([
            &Utc::now().timestamp().to_le_bytes(),
            &amount.to_le_bytes(),
            &blockchain_tx_hash.as_bytes()[..16],
        ].concat().try_into().unwrap());

        let transaction = TreasuryTransaction {
            transaction_id,
            transaction_type,
            amount,
            from,
            to,
            reference,
            description,
            timestamp: Utc::now(),
            blockchain_tx_hash,
        };

        self.transaction_history.push(transaction);
    }

    /// Calcule le pouvoir de vote total éligible
    fn calculate_total_eligible_voting_power(&self) -> u64 {
        // Cette méthode devrait être intégrée avec le système de staking
        // Pour l'instant, retourne une valeur placeholder
        100_000_000 // 100M tokens de pouvoir de vote total
    }

    /// Met à jour les métriques
    fn update_metrics(&mut self) {
        self.metrics.active_projects = self.active_projects.values()
            .filter(|p| matches!(p.status, ProjectStatus::Active | ProjectStatus::Planning))
            .count();

        self.metrics.completed_projects = self.active_projects.values()
            .filter(|p| p.status == ProjectStatus::Completed)
            .count();

        let total_projects = self.metrics.active_projects + self.metrics.completed_projects;
        if total_projects > 0 {
            self.metrics.project_success_rate = (self.metrics.completed_projects as f64 / total_projects as f64) * 100.0;
        }

        let total_treasury = COMMUNITY_RESERVE;
        self.metrics.fund_utilization_rate = ((total_treasury - self.available_funds) as f64 / total_treasury as f64) * 100.0;

        self.metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();
    }

    /// Obtient les statistiques du treasury
    pub fn get_treasury_statistics(&self) -> TreasuryStatistics {
        TreasuryStatistics {
            available_funds: self.available_funds,
            allocated_funds: self.allocated_funds,
            disbursed_funds: self.disbursed_funds,
            total_proposals: self.metrics.total_proposals,
            approved_proposals: self.metrics.approved_proposals,
            rejected_proposals: self.metrics.rejected_proposals,
            active_projects: self.metrics.active_projects,
            completed_projects: self.metrics.completed_projects,
            fund_utilization_rate: self.metrics.fund_utilization_rate,
            project_success_rate: self.metrics.project_success_rate,
        }
    }
}

impl TreasuryMetrics {
    fn new() -> Self {
        Self {
            total_proposals: 0,
            approved_proposals: 0,
            rejected_proposals: 0,
            active_projects: 0,
            completed_projects: 0,
            project_success_rate: 0.0,
            fund_utilization_rate: 0.0,
            average_project_roi: 0.0,
            average_approval_time_days: 0.0,
            last_updated: Utc::now(),
        }
    }
}

/// Statistiques simplifiées du treasury
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreasuryStatistics {
    pub available_funds: u64,
    pub allocated_funds: u64,
    pub disbursed_funds: u64,
    pub total_proposals: usize,
    pub approved_proposals: usize,
    pub rejected_proposals: usize,
    pub active_projects: usize,
    pub completed_projects: usize,
    pub fund_utilization_rate: f64,
    pub project_success_rate: f64,
}

impl Default for Treasury {
    fn default() -> Self {
        Self::new(TreasuryConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_treasury_creation() {
        let treasury = Treasury::default();
        assert_eq!(treasury.available_funds, COMMUNITY_RESERVE);
        assert_eq!(treasury.allocated_funds, 0);
        assert_eq!(treasury.disbursed_funds, 0);
    }

    #[test]
    fn test_proposal_submission() {
        let mut treasury = Treasury::default();
        let keypair = generate_keypair().unwrap();
        let proposer = keypair.public_key().clone();
        
        let budget_items = vec![
            BudgetItem {
                item_name: "Development".to_string(),
                description: "Core development work".to_string(),
                amount: 80_000,
                category: BudgetCategory::Personnel,
                justification: "Required for project delivery".to_string(),
            },
            BudgetItem {
                item_name: "Equipment".to_string(),
                description: "Hardware for testing".to_string(),
                amount: 20_000,
                category: BudgetCategory::Equipment,
                justification: "Testing infrastructure".to_string(),
            },
        ];

        let milestones = vec![
            Milestone {
                milestone_id: Hash::zero(),
                name: "Phase 1".to_string(),
                description: "Initial development".to_string(),
                payment_amount: 50_000,
                completion_criteria: vec!["Deliverable 1 completed".to_string()],
                target_date: Utc::now() + Duration::days(90),
                completed_date: None,
                status: MilestoneStatus::Planned,
            },
        ];

        let proposal_id = treasury.submit_proposal(
            proposer.clone(),
            "Test Project".to_string(),
            "A test project for the treasury".to_string(),
            ProposalCategory::Development,
            100_000,
            budget_items,
            proposer,
            milestones,
        ).unwrap();

        assert!(treasury.proposals.contains_key(&proposal_id));
        assert_eq!(treasury.metrics.total_proposals, 1);
    }

    #[test]
    fn test_proposal_voting() {
        let mut treasury = Treasury::default();
        let proposer_keypair = generate_keypair().unwrap();
        let voter_keypair = generate_keypair().unwrap();
        let proposer = proposer_keypair.public_key().clone();
        let voter = voter_keypair.public_key().clone();

        // Submit proposal
        let proposal_id = treasury.submit_proposal(
            proposer.clone(),
            "Test Project".to_string(),
            "A test project".to_string(),
            ProposalCategory::Development,
            100_000,
            vec![],
            proposer,
            vec![],
        ).unwrap();

        // Set proposal to voting status manually for test
        if let Some(proposal) = treasury.proposals.get_mut(&proposal_id) {
            proposal.status = ProposalStatus::Voting;
            proposal.voting_period.start_date = Utc::now() - Duration::hours(1);
        }

        // Vote on proposal
        let result = treasury.vote_on_proposal(
            voter,
            proposal_id,
            VotePosition::For,
            1_000_000, // 1M voting power
            Some("Support this project".to_string()),
            crate::crypto::Signature::zero(),
        );

        assert!(result.is_ok());
        assert_eq!(treasury.proposals[&proposal_id].votes.len(), 1);
    }

    #[test]
    fn test_invalid_proposal_amount() {
        let mut treasury = Treasury::default();
        let keypair = generate_keypair().unwrap();
        let proposer = keypair.public_key().clone();

        // Try to submit proposal with amount too small
        let result = treasury.submit_proposal(
            proposer.clone(),
            "Too Small".to_string(),
            "Too small amount".to_string(),
            ProposalCategory::Development,
            5_000, // Less than minimum
            vec![],
            proposer.clone(),
            vec![],
        );

        assert!(result.is_err());

        // Try to submit proposal with amount too large
        let result = treasury.submit_proposal(
            proposer.clone(),
            "Too Large".to_string(),
            "Too large amount".to_string(),
            ProposalCategory::Development,
            COMMUNITY_RESERVE, // More than maximum percentage
            vec![],
            proposer,
            vec![],
        );

        assert!(result.is_err());
    }
}