//! Système de distribution initiale des tokens ARC
//!
//! Gère la distribution selon les spécifications :
//! - 40% (40B ARC) : Récompenses d'archivage (distribution sur 10 ans)
//! - 25% (25B ARC) : Équipe (vesting sur 4 ans avec cliff de 1 an)
//! - 20% (20B ARC) : Réserve communautaire (gouvernance)
//! - 15% (15B ARC) : Vente publique/privée

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey};
use super::{
    TokenOperationResult, TokenOperationError, ARCHIVAL_REWARDS_ALLOCATION, 
    TEAM_ALLOCATION, COMMUNITY_RESERVE, PUBLIC_SALE, ARCToken
};

/// Gestionnaire de distribution des tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDistribution {
    /// Pool de récompenses d'archivage (40% - 10 ans)
    pub archival_rewards: RewardPool,
    /// Allocation équipe (25% - 4 ans vesting)
    pub team_allocation: TeamAllocation,
    /// Réserve communautaire (20% - gouvernance)
    pub community_reserve: CommunityReserve,
    /// Vente publique (15% - distribution immédiate)
    pub public_sale: PublicSale,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Pool de récompenses d'archivage (40B ARC sur 10 ans)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardPool {
    /// Allocation totale
    pub total_allocation: u64,
    /// Montant distribué à ce jour
    pub distributed_amount: u64,
    /// Montant disponible
    pub available_amount: u64,
    /// Distribution annuelle prévue
    pub yearly_distribution: u64,
    /// Date de début de distribution
    pub start_date: DateTime<Utc>,
    /// Date de fin de distribution (10 ans)
    pub end_date: DateTime<Utc>,
    /// Historique des distributions
    pub distribution_history: Vec<RewardDistributionRecord>,
}

/// Allocation de l'équipe avec vesting (25B ARC sur 4 ans)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAllocation {
    /// Allocation totale
    pub total_allocation: u64,
    /// Montant déjà distribué
    pub distributed_amount: u64,
    /// Schedule de vesting par bénéficiaire
    pub vesting_schedules: HashMap<PublicKey, VestingSchedule>,
    /// Date de début du vesting
    pub start_date: DateTime<Utc>,
    /// Période de cliff (1 an)
    pub cliff_duration_months: u32,
    /// Durée totale de vesting (4 ans)
    pub total_vesting_months: u32,
}

/// Réserve communautaire (20B ARC pour gouvernance)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommunityReserve {
    /// Allocation totale
    pub total_allocation: u64,
    /// Montant utilisé/alloué
    pub allocated_amount: u64,
    /// Montant disponible
    pub available_amount: u64,
    /// Propositions financées
    pub funded_proposals: Vec<FundedProposal>,
    /// Règles de gouvernance pour l'utilisation
    pub governance_rules: GovernanceRules,
}

/// Vente publique/privée (15B ARC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublicSale {
    /// Allocation totale
    pub total_allocation: u64,
    /// Montant vendu
    pub sold_amount: u64,
    /// Montant restant
    pub remaining_amount: u64,
    /// Prix de vente (en unité externe, ex: ETH)
    pub sale_price: f64,
    /// Participants à la vente
    pub participants: HashMap<PublicKey, SaleParticipation>,
    /// Statut de la vente
    pub sale_status: SaleStatus,
}

/// Schedule de vesting pour un bénéficiaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    /// Bénéficiaire
    pub beneficiary: PublicKey,
    /// Allocation totale
    pub total_allocation: u64,
    /// Montant déjà réclamé
    pub claimed_amount: u64,
    /// Date de début
    pub start_date: DateTime<Utc>,
    /// Date de cliff
    pub cliff_date: DateTime<Utc>,
    /// Date de fin
    pub end_date: DateTime<Utc>,
    /// Montant disponible au cliff
    pub cliff_amount: u64,
    /// Libération mensuelle après cliff
    pub monthly_release: u64,
    /// Dernier claim
    pub last_claim_date: Option<DateTime<Utc>>,
}

/// Proposition financée par la réserve communautaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FundedProposal {
    /// ID de la proposition
    pub proposal_id: Hash,
    /// Bénéficiaire
    pub beneficiary: PublicKey,
    /// Montant alloué
    pub amount: u64,
    /// Date d'approbation
    pub approved_at: DateTime<Utc>,
    /// Date de distribution
    pub distributed_at: Option<DateTime<Utc>>,
    /// Description/justification
    pub description: String,
    /// Statut
    pub status: ProposalStatus,
}

/// Participation à la vente publique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaleParticipation {
    /// Participant
    pub participant: PublicKey,
    /// Montant acheté (en ARC)
    pub amount_purchased: u64,
    /// Prix payé (unité externe)
    pub price_paid: f64,
    /// Date d'achat
    pub purchase_date: DateTime<Utc>,
    /// Tokens déjà distribués
    pub tokens_distributed: bool,
}

/// Enregistrement de distribution de récompense
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardDistributionRecord {
    /// Date de distribution
    pub date: DateTime<Utc>,
    /// Montant distribué
    pub amount: u64,
    /// Type de récompense
    pub reward_type: String,
    /// Bénéficiaires
    pub recipients: Vec<(PublicKey, u64)>,
}

/// Règles de gouvernance pour la réserve communautaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceRules {
    /// Quorum minimum pour les votes (en pourcentage)
    pub minimum_quorum: f64,
    /// Majorité requise pour l'approbation (en pourcentage)
    pub approval_threshold: f64,
    /// Durée de vote (en jours)
    pub voting_duration_days: u32,
    /// Montant maximum par proposition
    pub max_proposal_amount: u64,
}

/// Statut de la vente publique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SaleStatus {
    /// Pas encore commencée
    NotStarted,
    /// En cours
    Active,
    /// Terminée avec succès
    Completed,
    /// Annulée
    Cancelled,
}

/// Statut d'une proposition communautaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProposalStatus {
    /// En attente de distribution
    Approved,
    /// Distribuée
    Distributed,
    /// Rejetée
    Rejected,
}

/// Erreurs spécifiques à la distribution
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum DistributionError {
    #[error("Période de cliff non atteinte pour {beneficiary}")]
    CliffNotReached { beneficiary: String },
    
    #[error("Aucun token disponible pour le claim")]
    NoTokensAvailable,
    
    #[error("Allocation épuisée")]
    AllocationExhausted,
    
    #[error("Vente non active")]
    SaleNotActive,
    
    #[error("Proposition non trouvée : {proposal_id}")]
    ProposalNotFound { proposal_id: String },
    
    #[error("Montant de vente insuffisant")]
    InsufficientSaleAmount,
}

impl TokenDistribution {
    /// Crée une nouvelle distribution avec les allocations par défaut
    pub fn new() -> Self {
        let now = Utc::now();
        
        Self {
            archival_rewards: RewardPool {
                total_allocation: ARCHIVAL_REWARDS_ALLOCATION,
                distributed_amount: 0,
                available_amount: ARCHIVAL_REWARDS_ALLOCATION,
                yearly_distribution: ARCHIVAL_REWARDS_ALLOCATION / 10, // Distribution sur 10 ans
                start_date: now,
                end_date: now + Duration::days(365 * 10), // 10 ans
                distribution_history: Vec::new(),
            },
            team_allocation: TeamAllocation {
                total_allocation: TEAM_ALLOCATION,
                distributed_amount: 0,
                vesting_schedules: HashMap::new(),
                start_date: now,
                cliff_duration_months: 12, // 1 an de cliff
                total_vesting_months: 48,  // 4 ans total
            },
            community_reserve: CommunityReserve {
                total_allocation: COMMUNITY_RESERVE,
                allocated_amount: 0,
                available_amount: COMMUNITY_RESERVE,
                funded_proposals: Vec::new(),
                governance_rules: GovernanceRules {
                    minimum_quorum: 0.15,        // 15% de quorum
                    approval_threshold: 0.60,    // 60% d'approbation
                    voting_duration_days: 7,     // 7 jours de vote
                    max_proposal_amount: COMMUNITY_RESERVE / 100, // Max 1% par proposition
                },
            },
            public_sale: PublicSale {
                total_allocation: PUBLIC_SALE,
                sold_amount: 0,
                remaining_amount: PUBLIC_SALE,
                sale_price: 0.001, // Prix initial en ETH par exemple
                participants: HashMap::new(),
                sale_status: SaleStatus::NotStarted,
            },
            created_at: now,
            last_updated: now,
        }
    }

    /// Ajoute un schedule de vesting pour un membre de l'équipe
    pub fn add_team_vesting(&mut self, beneficiary: PublicKey, allocation: u64) -> TokenOperationResult<()> {
        if self.team_allocation.distributed_amount + allocation > self.team_allocation.total_allocation {
            return Err(TokenOperationError::Internal {
                message: "Allocation équipe dépassée".to_string(),
            });
        }

        let start_date = self.team_allocation.start_date;
        let cliff_date = start_date + Duration::days(365); // 1 an de cliff
        let end_date = start_date + Duration::days(365 * 4); // 4 ans total
        
        // 25% disponible au cliff, 75% distribué mensuellement sur 3 ans
        let cliff_amount = allocation / 4;
        let remaining_amount = allocation - cliff_amount;
        let monthly_release = remaining_amount / 36; // 36 mois après cliff

        let schedule = VestingSchedule {
            beneficiary: beneficiary.clone(),
            total_allocation: allocation,
            claimed_amount: 0,
            start_date,
            cliff_date,
            end_date,
            cliff_amount,
            monthly_release,
            last_claim_date: None,
        };

        self.team_allocation.vesting_schedules.insert(beneficiary, schedule);
        self.team_allocation.distributed_amount += allocation;
        self.last_updated = Utc::now();

        Ok(())
    }

    /// Calcule le montant disponible pour un bénéficiaire de vesting
    pub fn calculate_vested_amount(&self, beneficiary: &PublicKey) -> TokenOperationResult<u64> {
        let schedule = self.team_allocation.vesting_schedules.get(beneficiary)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Schedule de vesting non trouvé".to_string(),
            })?;

        let now = Utc::now();
        
        // Vérifier si le cliff est atteint
        if now < schedule.cliff_date {
            return Ok(0);
        }

        let mut vested_amount = schedule.cliff_amount;

        // Calculer les tokens vested mensuellement depuis le cliff
        if now > schedule.cliff_date {
            let months_since_cliff = (now - schedule.cliff_date).num_days() / 30;
            let months_since_cliff = months_since_cliff.min(36) as u64; // Max 36 mois
            vested_amount += months_since_cliff * schedule.monthly_release;
        }

        // Ne peut pas dépasser l'allocation totale
        vested_amount = vested_amount.min(schedule.total_allocation);
        
        // Soustraire ce qui a déjà été réclamé
        let available = vested_amount.saturating_sub(schedule.claimed_amount);
        
        Ok(available)
    }

    /// Effectue un claim de vesting pour un bénéficiaire
    pub fn claim_vested_tokens(&mut self, beneficiary: &PublicKey, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let available_amount = self.calculate_vested_amount(beneficiary)?;
        
        if available_amount == 0 {
            return Err(TokenOperationError::Internal {
                message: "Aucun token disponible pour le claim".to_string(),
            });
        }

        // Mint les tokens vers le bénéficiaire
        token.mint(beneficiary, available_amount, tx_hash)?;

        // Mettre à jour le schedule
        if let Some(schedule) = self.team_allocation.vesting_schedules.get_mut(beneficiary) {
            schedule.claimed_amount += available_amount;
            schedule.last_claim_date = Some(Utc::now());
        }

        self.last_updated = Utc::now();
        Ok(available_amount)
    }

    /// Distribue des récompenses d'archivage
    pub fn distribute_archival_rewards(&mut self, recipients: Vec<(PublicKey, u64)>, reward_type: String, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let total_amount: u64 = recipients.iter().map(|(_, amount)| amount).sum();
        
        if self.archival_rewards.available_amount < total_amount {
            return Err(TokenOperationError::InsufficientRewardPool);
        }

        // Distribuer aux bénéficiaires
        for (recipient, amount) in &recipients {
            token.mint(recipient, *amount, tx_hash.clone())?;
        }

        // Mettre à jour le pool
        self.archival_rewards.distributed_amount += total_amount;
        self.archival_rewards.available_amount -= total_amount;

        // Enregistrer dans l'historique
        self.archival_rewards.distribution_history.push(RewardDistributionRecord {
            date: Utc::now(),
            amount: total_amount,
            reward_type,
            recipients,
        });

        self.last_updated = Utc::now();
        Ok(total_amount)
    }

    /// Finance une proposition communautaire
    pub fn fund_community_proposal(&mut self, proposal_id: Hash, beneficiary: PublicKey, amount: u64, description: String, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<()> {
        if self.community_reserve.available_amount < amount {
            return Err(TokenOperationError::InsufficientRewardPool);
        }

        if amount > self.community_reserve.governance_rules.max_proposal_amount {
            return Err(TokenOperationError::InvalidAmount { amount });
        }

        // Mint les tokens vers le bénéficiaire
        token.mint(&beneficiary, amount, tx_hash)?;

        // Enregistrer la proposition financée
        let funded_proposal = FundedProposal {
            proposal_id,
            beneficiary,
            amount,
            approved_at: Utc::now(),
            distributed_at: Some(Utc::now()),
            description,
            status: ProposalStatus::Distributed,
        };

        self.community_reserve.funded_proposals.push(funded_proposal);
        self.community_reserve.allocated_amount += amount;
        self.community_reserve.available_amount -= amount;
        self.last_updated = Utc::now();

        Ok(())
    }

    /// Traite un achat lors de la vente publique
    pub fn process_public_sale(&mut self, participant: PublicKey, amount: u64, price_paid: f64, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<()> {
        if !matches!(self.public_sale.sale_status, SaleStatus::Active) {
            return Err(TokenOperationError::Internal {
                message: "Vente publique non active".to_string(),
            });
        }

        if self.public_sale.remaining_amount < amount {
            return Err(TokenOperationError::Internal {
                message: "Montant insuffisant dans la vente publique".to_string(),
            });
        }

        // Mint les tokens vers le participant
        token.mint(&participant, amount, tx_hash)?;

        // Enregistrer la participation
        let participation = SaleParticipation {
            participant: participant.clone(),
            amount_purchased: amount,
            price_paid,
            purchase_date: Utc::now(),
            tokens_distributed: true,
        };

        self.public_sale.participants.insert(participant, participation);
        self.public_sale.sold_amount += amount;
        self.public_sale.remaining_amount -= amount;
        self.last_updated = Utc::now();

        Ok(())
    }

    /// Active la vente publique
    pub fn activate_public_sale(&mut self) -> TokenOperationResult<()> {
        self.public_sale.sale_status = SaleStatus::Active;
        self.last_updated = Utc::now();
        Ok(())
    }

    /// Termine la vente publique
    pub fn complete_public_sale(&mut self) -> TokenOperationResult<()> {
        self.public_sale.sale_status = SaleStatus::Completed;
        self.last_updated = Utc::now();
        Ok(())
    }

    /// Obtient les statistiques de distribution
    pub fn get_distribution_stats(&self) -> DistributionStatistics {
        DistributionStatistics {
            archival_rewards_distributed: self.archival_rewards.distributed_amount,
            archival_rewards_remaining: self.archival_rewards.available_amount,
            team_tokens_distributed: self.team_allocation.distributed_amount,
            team_schedules_count: self.team_allocation.vesting_schedules.len(),
            community_funds_allocated: self.community_reserve.allocated_amount,
            community_funds_available: self.community_reserve.available_amount,
            public_sale_progress: if self.public_sale.total_allocation > 0 {
                (self.public_sale.sold_amount as f64 / self.public_sale.total_allocation as f64) * 100.0
            } else { 0.0 },
            total_distributed: self.archival_rewards.distributed_amount + 
                             self.team_allocation.distributed_amount + 
                             self.community_reserve.allocated_amount + 
                             self.public_sale.sold_amount,
        }
    }
}

/// Statistiques de distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionStatistics {
    /// Récompenses d'archivage distribuées
    pub archival_rewards_distributed: u64,
    /// Récompenses d'archivage restantes
    pub archival_rewards_remaining: u64,
    /// Tokens équipe distribués
    pub team_tokens_distributed: u64,
    /// Nombre de schedules de vesting
    pub team_schedules_count: usize,
    /// Fonds communautaires alloués
    pub community_funds_allocated: u64,
    /// Fonds communautaires disponibles
    pub community_funds_available: u64,
    /// Progression de la vente publique (%)
    pub public_sale_progress: f64,
    /// Total distribué toutes catégories
    pub total_distributed: u64,
}

impl Default for TokenDistribution {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_distribution_creation() {
        let distribution = TokenDistribution::new();
        assert_eq!(distribution.archival_rewards.total_allocation, ARCHIVAL_REWARDS_ALLOCATION);
        assert_eq!(distribution.team_allocation.total_allocation, TEAM_ALLOCATION);
        assert_eq!(distribution.community_reserve.total_allocation, COMMUNITY_RESERVE);
        assert_eq!(distribution.public_sale.total_allocation, PUBLIC_SALE);
    }

    #[test]
    fn test_team_vesting_schedule() {
        let mut distribution = TokenDistribution::new();
        let keypair = generate_keypair().unwrap();
        let beneficiary = keypair.public_key().clone();
        
        let result = distribution.add_team_vesting(beneficiary.clone(), 1_000_000);
        assert!(result.is_ok());
        
        assert!(distribution.team_allocation.vesting_schedules.contains_key(&beneficiary));
        assert_eq!(distribution.team_allocation.distributed_amount, 1_000_000);
    }

    #[test]
    fn test_vested_amount_calculation() {
        let mut distribution = TokenDistribution::new();
        let keypair = generate_keypair().unwrap();
        let beneficiary = keypair.public_key().clone();
        
        // Configurer le vesting avec une date dans le passé pour les tests
        distribution.team_allocation.start_date = Utc::now() - Duration::days(400); // Plus d'un an
        distribution.add_team_vesting(beneficiary.clone(), 1_000_000).unwrap();
        
        let vested = distribution.calculate_vested_amount(&beneficiary).unwrap();
        assert!(vested > 0); // Devrait avoir des tokens vested après le cliff
    }

    #[test]
    fn test_community_proposal_funding() {
        let mut distribution = TokenDistribution::new();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let beneficiary = keypair.public_key().clone();
        let proposal_id = Hash::zero();
        let tx_hash = Hash::zero();
        
        let result = distribution.fund_community_proposal(
            proposal_id,
            beneficiary.clone(),
            1_000_000,
            "Test proposal".to_string(),
            &mut token,
            tx_hash,
        );
        
        assert!(result.is_ok());
        assert_eq!(token.balance_of(&beneficiary), 1_000_000);
        assert_eq!(distribution.community_reserve.allocated_amount, 1_000_000);
    }

    #[test]
    fn test_public_sale_processing() {
        let mut distribution = TokenDistribution::new();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let participant = keypair.public_key().clone();
        let tx_hash = Hash::zero();
        
        // Activer la vente
        distribution.activate_public_sale().unwrap();
        
        let result = distribution.process_public_sale(
            participant.clone(),
            1_000_000,
            1.0,
            &mut token,
            tx_hash,
        );
        
        assert!(result.is_ok());
        assert_eq!(token.balance_of(&participant), 1_000_000);
        assert_eq!(distribution.public_sale.sold_amount, 1_000_000);
    }
}