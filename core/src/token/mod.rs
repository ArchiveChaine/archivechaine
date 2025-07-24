//! Module économique du token ARC pour ArchiveChain
//!
//! Ce module implémente le système économique complet d'ArchiveChain avec :
//! - Token ARC principal avec fonctionnalités ERC-20-like
//! - Distribution initiale selon les spécifications (40% récompenses, 25% équipe, 20% réserve, 15% vente)
//! - Mécanismes déflationnistes (burning des frais, quality staking, bonus long terme)
//! - Système de récompenses pour archivage, stockage, bande passante et découverte
//! - Staking et gouvernance
//! - Treasury communautaire

pub mod arc_token;
pub mod distribution;
pub mod economics;
pub mod rewards;
pub mod staking;
pub mod treasury;
pub mod deflation;

// Re-exports principaux
pub use arc_token::{ARCToken, TokenError, TokenResult};
pub use distribution::{TokenDistribution, VestingSchedule, DistributionError};
pub use economics::{EconomicModel, EconomicMetrics, RewardCalculation};
pub use rewards::{RewardSystem, RewardPool, RewardType, RewardDistribution};
pub use staking::{StakingSystem, StakeInfo, GovernanceStake, ValidatorStake};
pub use treasury::{Treasury, TreasuryProposal, ProposalStatus};
pub use deflation::{DeflationaryMechanisms, BurnRecord, LongtermBonusRecord};

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, PublicKey};
use crate::error::Result;

/// Supply total de tokens ARC (100 milliards)
pub const TOTAL_SUPPLY: u64 = 100_000_000_000;

/// Distribution initiale selon les spécifications
pub const ARCHIVAL_REWARDS_ALLOCATION: u64 = 40_000_000_000; // 40% - 40 milliards
pub const TEAM_ALLOCATION: u64 = 25_000_000_000;            // 25% - 25 milliards  
pub const COMMUNITY_RESERVE: u64 = 20_000_000_000;          // 20% - 20 milliards
pub const PUBLIC_SALE: u64 = 15_000_000_000;               // 15% - 15 milliards

/// Adresse système pour les opérations de mint/burn
pub const SYSTEM_ADDRESS: &str = "0x0000000000000000000000000000000000000001";

/// Configuration économique principale
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenConfig {
    /// Taux de burning des frais de transaction (10% par défaut)
    pub burn_rate: f64,
    /// Multiplicateur de bonus pour stockage long terme (>1 an)
    pub longterm_multiplier: f64,
    /// Période de vesting pour l'équipe (4 ans)
    pub team_vesting_years: u32,
    /// Période de distribution des récompenses d'archivage (10 ans)
    pub archival_rewards_years: u32,
    /// Staking minimum requis pour la gouvernance
    pub min_governance_stake: u64,
    /// Staking minimum requis pour validation
    pub min_validator_stake: u64,
}

impl Default for TokenConfig {
    fn default() -> Self {
        Self {
            burn_rate: 0.10,           // 10% des frais brûlés
            longterm_multiplier: 2.0,  // 2x pour stockage >1 an
            team_vesting_years: 4,     // 4 ans de vesting équipe
            archival_rewards_years: 10, // 10 ans pour les récompenses
            min_governance_stake: 1_000_000, // 1M ARC minimum pour gouvernance
            min_validator_stake: 10_000_000, // 10M ARC minimum pour validation
        }
    }
}

/// Résultat unifié pour toutes les opérations token
pub type TokenOperationResult<T> = std::result::Result<T, TokenOperationError>;

/// Erreurs unifiées pour les opérations token
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum TokenOperationError {
    #[error("Solde insuffisant : requis {required}, disponible {available}")]
    InsufficientBalance { required: u64, available: u64 },
    
    #[error("Opération non autorisée pour l'adresse {address}")]
    Unauthorized { address: String },
    
    #[error("Montant invalide : {amount}")]
    InvalidAmount { amount: u64 },
    
    #[error("Adresse invalide : {address}")]
    InvalidAddress { address: String },
    
    #[error("Période de vesting non atteinte")]
    VestingPeriodNotReached,
    
    #[error("Pool de récompenses insuffisant")]
    InsufficientRewardPool,
    
    #[error("Stake minimum non atteint : requis {required}, fourni {provided}")]
    InsufficientStake { required: u64, provided: u64 },
    
    #[error("Proposition de governance non trouvée : {proposal_id}")]
    ProposalNotFound { proposal_id: Hash },
    
    #[error("Erreur interne : {message}")]
    Internal { message: String },
}

/// Événement émis lors des opérations token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEvent {
    /// Hash de la transaction associée
    pub transaction_hash: Hash,
    /// Type d'événement
    pub event_type: TokenEventType,
    /// Timestamp de l'événement
    pub timestamp: DateTime<Utc>,
    /// Données additionnelles selon le type
    pub data: HashMap<String, serde_json::Value>,
}

/// Types d'événements token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TokenEventType {
    /// Transfert de tokens
    Transfer {
        from: PublicKey,
        to: PublicKey,
        amount: u64,
    },
    /// Tokens brûlés
    Burn {
        from: PublicKey,
        amount: u64,
    },
    /// Récompense distribuée
    RewardDistributed {
        to: PublicKey,
        amount: u64,
        reward_type: String,
    },
    /// Tokens stakés
    Staked {
        staker: PublicKey,
        amount: u64,
        stake_type: String,
    },
    /// Unstaking de tokens
    Unstaked {
        staker: PublicKey,
        amount: u64,
        stake_type: String,
    },
    /// Proposition de gouvernance créée
    ProposalCreated {
        proposer: PublicKey,
        proposal_id: Hash,
        stake_amount: u64,
    },
    /// Vote sur une proposition
    ProposalVoted {
        voter: PublicKey,
        proposal_id: Hash,
        voting_power: u64,
        support: bool,
    },
}

/// Métriques globales du système token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalTokenMetrics {
    /// Supply totale
    pub total_supply: u64,
    /// Supply en circulation
    pub circulating_supply: u64,
    /// Tokens brûlés (total cumulé)
    pub total_burned: u64,
    /// Tokens stakés (toutes catégories)
    pub total_staked: u64,
    /// Récompenses distribuées (total cumulé)
    pub total_rewards_distributed: u64,
    /// Nombre d'adresses détentrices
    pub holder_count: usize,
    /// Valeur totale verrouillée (TVL)
    pub total_value_locked: u64,
    /// Timestamp de dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

impl GlobalTokenMetrics {
    /// Crée des métriques initiales
    pub fn new() -> Self {
        Self {
            total_supply: TOTAL_SUPPLY,
            circulating_supply: 0, // Démarrera à 0, augmentera avec les distributions
            total_burned: 0,
            total_staked: 0,
            total_rewards_distributed: 0,
            holder_count: 0,
            total_value_locked: 0,
            last_updated: Utc::now(),
        }
    }
    
    /// Met à jour les métriques
    pub fn update(&mut self, 
                  circulating: u64, 
                  burned: u64, 
                  staked: u64, 
                  rewards: u64, 
                  holders: usize) {
        self.circulating_supply = circulating;
        self.total_burned = burned;
        self.total_staked = staked;
        self.total_rewards_distributed = rewards;
        self.holder_count = holders;
        self.total_value_locked = staked; // TVL = tokens stakés
        self.last_updated = Utc::now();
    }
}

/// Adresse système pour les opérations spéciales
pub fn system_address() -> PublicKey {
    // Crée une adresse système déterministe
    PublicKey::from_bytes(&[0u8; 32]).expect("System address should be valid")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_total_supply_allocation() {
        let total = ARCHIVAL_REWARDS_ALLOCATION + TEAM_ALLOCATION + 
                   COMMUNITY_RESERVE + PUBLIC_SALE;
        assert_eq!(total, TOTAL_SUPPLY);
    }

    #[test]
    fn test_token_config_defaults() {
        let config = TokenConfig::default();
        assert_eq!(config.burn_rate, 0.10);
        assert_eq!(config.longterm_multiplier, 2.0);
        assert_eq!(config.team_vesting_years, 4);
        assert_eq!(config.archival_rewards_years, 10);
    }

    #[test]
    fn test_global_metrics_initialization() {
        let metrics = GlobalTokenMetrics::new();
        assert_eq!(metrics.total_supply, TOTAL_SUPPLY);
        assert_eq!(metrics.circulating_supply, 0);
        assert_eq!(metrics.total_burned, 0);
    }

    #[test]
    fn test_system_address() {
        let addr = system_address();
        assert!(addr.is_valid());
    }
}