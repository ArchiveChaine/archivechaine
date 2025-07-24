//! Mécanismes déflationnistes pour le token ARC
//!
//! Implémente les mécanismes suivants :
//! - Burning automatique de 10% des frais de transaction
//! - Quality staking avec tokens bloqués proportionnels à la qualité promise
//! - Bonus long terme avec multiplicateurs croissants (1.2x/6mois, 1.5x/1an, 2x/2ans+)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey};
use super::{TokenOperationResult, TokenOperationError, ARCToken};

/// Gestionnaire des mécanismes déflationnistes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeflationaryMechanisms {
    /// Taux de burning des frais (10% par défaut)
    pub burn_rate: f64,
    /// Pool de tokens stakés pour la qualité
    pub quality_staking_pool: QualityStakingPool,
    /// Système de bonus long terme
    pub longterm_bonus_system: LongtermBonusSystem,
    /// Historique des burns
    pub burn_history: Vec<BurnRecord>,
    /// Métriques de déflation
    pub deflation_metrics: DeflationMetrics,
    /// Configuration
    pub config: DeflationConfig,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Pool de staking pour garantir la qualité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStakingPool {
    /// Stakes actifs par adresse
    pub active_stakes: HashMap<PublicKey, QualityStake>,
    /// Montant total staké
    pub total_staked: u64,
    /// Exigences de staking par niveau de qualité
    pub quality_requirements: HashMap<QualityLevel, u64>,
    /// Historique des slashing
    pub slashing_history: Vec<SlashingRecord>,
}

/// Système de bonus long terme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongtermBonusSystem {
    /// Multiplicateurs par durée
    pub multipliers: LongtermMultipliers,
    /// Positions long terme actives
    pub longterm_positions: HashMap<PublicKey, Vec<LongtermPosition>>,
    /// Historique des bonus distribués
    pub bonus_history: Vec<LongtermBonusRecord>,
}

/// Stake de qualité pour un archiveur
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityStake {
    /// Adresse de l'archiveur
    pub staker: PublicKey,
    /// Montant staké
    pub amount: u64,
    /// Niveau de qualité promis
    pub promised_quality: QualityLevel,
    /// Score de qualité actuel
    pub current_quality_score: f64,
    /// Date de début du stake
    pub start_date: DateTime<Utc>,
    /// Dernière évaluation de qualité
    pub last_quality_check: DateTime<Utc>,
    /// Nombre de violations de qualité
    pub quality_violations: u32,
    /// Montant slashé total
    pub slashed_amount: u64,
    /// Statut du stake
    pub status: StakeStatus,
}

/// Position long terme pour bonus
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongtermPosition {
    /// ID unique de la position
    pub position_id: Hash,
    /// Détenteur
    pub holder: PublicKey,
    /// Montant en position
    pub amount: u64,
    /// Date de début
    pub start_date: DateTime<Utc>,
    /// Durée minimale d'engagement (en mois)
    pub commitment_months: u32,
    /// Date de fin d'engagement
    pub commitment_end_date: DateTime<Utc>,
    /// Multiplicateur actuel
    pub current_multiplier: f64,
    /// Bonus accumulés non réclamés
    pub unclaimed_bonus: u64,
    /// Dernière réclamation de bonus
    pub last_bonus_claim: Option<DateTime<Utc>>,
    /// Statut de la position
    pub status: PositionStatus,
}

/// Enregistrement de burn
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurnRecord {
    /// Hash de la transaction source
    pub transaction_hash: Hash,
    /// Montant des frais originaux
    pub original_fee: u64,
    /// Montant brûlé
    pub burned_amount: u64,
    /// Montant conservé
    pub retained_amount: u64,
    /// Date du burn
    pub burn_date: DateTime<Utc>,
    /// Raison du burn
    pub burn_reason: BurnReason,
}

/// Enregistrement de slashing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashingRecord {
    /// Adresse slashée
    pub slashed_address: PublicKey,
    /// Montant slashé
    pub slashed_amount: u64,
    /// Raison du slashing
    pub reason: SlashingReason,
    /// Score de qualité au moment du slashing
    pub quality_score_at_slash: f64,
    /// Date du slashing
    pub slash_date: DateTime<Utc>,
    /// Hash de la transaction de slashing
    pub transaction_hash: Hash,
}

/// Enregistrement de bonus long terme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongtermBonusRecord {
    /// Bénéficiaire
    pub beneficiary: PublicKey,
    /// ID de position
    pub position_id: Hash,
    /// Montant du bonus
    pub bonus_amount: u64,
    /// Multiplicateur appliqué
    pub multiplier_applied: f64,
    /// Période couverte (en jours)
    pub period_days: u32,
    /// Date de distribution
    pub distribution_date: DateTime<Utc>,
    /// Hash de transaction
    pub transaction_hash: Hash,
}

/// Multiplicateurs pour bonus long terme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LongtermMultipliers {
    /// Multiplicateur pour 6 mois (1.2x)
    pub six_months: f64,
    /// Multiplicateur pour 1 an (1.5x)
    pub one_year: f64,
    /// Multiplicateur pour 2 ans et plus (2.0x)
    pub two_years_plus: f64,
    /// Multiplicateur maximum possible
    pub max_multiplier: f64,
}

/// Métriques de déflation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeflationMetrics {
    /// Total des tokens brûlés
    pub total_burned: u64,
    /// Tokens brûlés cette période
    pub burned_this_period: u64,
    /// Total des tokens stakés pour qualité
    pub total_quality_staked: u64,
    /// Total des tokens en position long terme
    pub total_longterm_locked: u64,
    /// Taux de déflation annuel estimé
    pub estimated_annual_deflation_rate: f64,
    /// Bonus long terme distribués
    pub total_longterm_bonus_distributed: u64,
    /// Montant total slashé
    pub total_slashed: u64,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Configuration des mécanismes déflationnistes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeflationConfig {
    /// Taux de burn des frais (0.0 à 1.0)
    pub fee_burn_rate: f64,
    /// Période de slashing pour qualité (en jours)
    pub quality_evaluation_period_days: u32,
    /// Seuil de qualité minimum pour éviter le slashing
    pub minimum_quality_threshold: f64,
    /// Taux de slashing pour violation de qualité (0.0 à 1.0)
    pub quality_slashing_rate: f64,
    /// Période minimum pour bonus long terme (en jours)
    pub minimum_longterm_period_days: u32,
    /// Fréquence de distribution des bonus (en jours)
    pub bonus_distribution_frequency_days: u32,
}

/// Niveaux de qualité pour staking
#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum QualityLevel {
    /// Qualité basique
    Basic,
    /// Qualité standard
    Standard,
    /// Qualité premium
    Premium,
    /// Qualité exceptionnelle
    Exceptional,
}

/// Raisons de burning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BurnReason {
    /// Burn automatique des frais de transaction
    TransactionFees,
    /// Slashing pour mauvaise qualité
    QualitySlashing,
    /// Burn manuel pour déflation
    ManualDeflation,
}

/// Raisons de slashing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SlashingReason {
    /// Qualité insuffisante
    PoorQuality,
    /// Archive corrompue
    CorruptedArchive,
    /// Indisponibilité du service
    ServiceUnavailable,
    /// Violation des termes de service
    TermsViolation,
}

/// Statut d'un stake
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StakeStatus {
    /// Actif
    Active,
    /// En cours de retrait
    Unstaking,
    /// Slashé
    Slashed,
    /// Retiré
    Withdrawn,
}

/// Statut d'une position long terme
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionStatus {
    /// Active
    Active,
    /// En cours de retrait anticipé
    EarlyWithdrawal,
    /// Terminée
    Completed,
    /// Liquidée
    Liquidated,
}

impl Default for LongtermMultipliers {
    fn default() -> Self {
        Self {
            six_months: 1.2,      // +20% pour 6 mois
            one_year: 1.5,        // +50% pour 1 an
            two_years_plus: 2.0,  // +100% pour 2 ans+
            max_multiplier: 2.0,
        }
    }
}

impl Default for DeflationConfig {
    fn default() -> Self {
        Self {
            fee_burn_rate: 0.10,                    // 10% des frais brûlés
            quality_evaluation_period_days: 30,     // Évaluation mensuelle
            minimum_quality_threshold: 0.8,        // 80% de qualité minimum
            quality_slashing_rate: 0.15,           // 15% de slashing
            minimum_longterm_period_days: 180,     // 6 mois minimum
            bonus_distribution_frequency_days: 30, // Distribution mensuelle
        }
    }
}

impl DeflationaryMechanisms {
    /// Crée un nouveau système de mécanismes déflationnistes
    pub fn new(config: DeflationConfig) -> Self {
        let mut quality_requirements = HashMap::new();
        quality_requirements.insert(QualityLevel::Basic, 10_000);        // 10K ARC
        quality_requirements.insert(QualityLevel::Standard, 50_000);     // 50K ARC
        quality_requirements.insert(QualityLevel::Premium, 200_000);     // 200K ARC
        quality_requirements.insert(QualityLevel::Exceptional, 1_000_000); // 1M ARC

        Self {
            burn_rate: config.fee_burn_rate,
            quality_staking_pool: QualityStakingPool {
                active_stakes: HashMap::new(),
                total_staked: 0,
                quality_requirements,
                slashing_history: Vec::new(),
            },
            longterm_bonus_system: LongtermBonusSystem {
                multipliers: LongtermMultipliers::default(),
                longterm_positions: HashMap::new(),
                bonus_history: Vec::new(),
            },
            burn_history: Vec::new(),
            deflation_metrics: DeflationMetrics {
                total_burned: 0,
                burned_this_period: 0,
                total_quality_staked: 0,
                total_longterm_locked: 0,
                estimated_annual_deflation_rate: 0.0,
                total_longterm_bonus_distributed: 0,
                total_slashed: 0,
                last_updated: Utc::now(),
            },
            config,
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Brûle automatiquement une portion des frais de transaction
    pub fn burn_transaction_fees(&mut self, fee_amount: u64, tx_hash: Hash, token: &mut ARCToken) -> TokenOperationResult<u64> {
        let burn_amount = (fee_amount as f64 * self.burn_rate) as u64;
        let retained_amount = fee_amount - burn_amount;

        if burn_amount > 0 {
            // Brûler les tokens depuis le système
            token.burn(&super::system_address(), burn_amount, tx_hash)?;

            // Enregistrer le burn
            self.burn_history.push(BurnRecord {
                transaction_hash: tx_hash,
                original_fee: fee_amount,
                burned_amount: burn_amount,
                retained_amount,
                burn_date: Utc::now(),
                burn_reason: BurnReason::TransactionFees,
            });

            // Mettre à jour les métriques
            self.deflation_metrics.total_burned += burn_amount;
            self.deflation_metrics.burned_this_period += burn_amount;
            self.deflation_metrics.last_updated = Utc::now();
        }

        self.last_updated = Utc::now();
        Ok(burn_amount)
    }

    /// Crée un stake de qualité
    pub fn create_quality_stake(&mut self, staker: PublicKey, amount: u64, quality_level: QualityLevel, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<()> {
        // Vérifier les exigences minimales
        let required_amount = self.quality_staking_pool.quality_requirements.get(&quality_level)
            .copied().unwrap_or(0);

        if amount < required_amount {
            return Err(TokenOperationError::InsufficientStake {
                required: required_amount,
                provided: amount,
            });
        }

        // Vérifier que l'adresse n'a pas déjà un stake actif
        if self.quality_staking_pool.active_stakes.contains_key(&staker) {
            return Err(TokenOperationError::Internal {
                message: "Stake de qualité déjà actif pour cette adresse".to_string(),
            });
        }

        // Verrouiller les tokens
        token.lock_tokens(&staker, amount, "quality_stake", tx_hash)?;

        // Créer le stake
        let stake = QualityStake {
            staker: staker.clone(),
            amount,
            promised_quality: quality_level,
            current_quality_score: 1.0, // Démarre à 100%
            start_date: Utc::now(),
            last_quality_check: Utc::now(),
            quality_violations: 0,
            slashed_amount: 0,
            status: StakeStatus::Active,
        };

        self.quality_staking_pool.active_stakes.insert(staker, stake);
        self.quality_staking_pool.total_staked += amount;
        self.deflation_metrics.total_quality_staked += amount;
        self.deflation_metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();

        Ok(())
    }

    /// Évalue la qualité et applique le slashing si nécessaire
    pub fn evaluate_quality_and_slash(&mut self, staker: &PublicKey, quality_score: f64, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let stake = self.quality_staking_pool.active_stakes.get_mut(staker)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Stake de qualité non trouvé".to_string(),
            })?;

        stake.current_quality_score = quality_score;
        stake.last_quality_check = Utc::now();

        let mut slashed_amount = 0;

        // Vérifier si le slashing est nécessaire
        if quality_score < self.config.minimum_quality_threshold {
            stake.quality_violations += 1;
            
            // Calculer le montant à slasher
            let slash_amount = (stake.amount as f64 * self.config.quality_slashing_rate) as u64;
            
            if slash_amount > 0 && stake.amount >= slash_amount {
                // Effectuer le slashing (burn des tokens)
                token.burn(&super::system_address(), slash_amount, tx_hash)?;

                // Mettre à jour le stake
                stake.amount -= slash_amount;
                stake.slashed_amount += slash_amount;

                // Si le stake devient trop petit, le marquer comme slashé
                if stake.amount < self.quality_staking_pool.quality_requirements
                    .get(&stake.promised_quality).copied().unwrap_or(0) / 2 {
                    stake.status = StakeStatus::Slashed;
                }

                // Enregistrer le slashing
                self.quality_staking_pool.slashing_history.push(SlashingRecord {
                    slashed_address: staker.clone(),
                    slashed_amount: slash_amount,
                    reason: SlashingReason::PoorQuality,
                    quality_score_at_slash: quality_score,
                    slash_date: Utc::now(),
                    transaction_hash: tx_hash,
                });

                // Enregistrer le burn
                self.burn_history.push(BurnRecord {
                    transaction_hash: tx_hash,
                    original_fee: slash_amount,
                    burned_amount: slash_amount,
                    retained_amount: 0,
                    burn_date: Utc::now(),
                    burn_reason: BurnReason::QualitySlashing,
                });

                // Mettre à jour les métriques
                self.quality_staking_pool.total_staked -= slash_amount;
                self.deflation_metrics.total_quality_staked -= slash_amount;
                self.deflation_metrics.total_burned += slash_amount;
                self.deflation_metrics.total_slashed += slash_amount;

                slashed_amount = slash_amount;
            }
        }

        self.deflation_metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();
        Ok(slashed_amount)
    }

    /// Crée une position long terme
    pub fn create_longterm_position(&mut self, holder: PublicKey, amount: u64, commitment_months: u32, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<Hash> {
        if commitment_months < (self.config.minimum_longterm_period_days / 30) {
            return Err(TokenOperationError::Internal {
                message: format!("Engagement minimum de {} mois requis", 
                               self.config.minimum_longterm_period_days / 30),
            });
        }

        // Verrouiller les tokens
        token.lock_tokens(&holder, amount, "longterm_position", tx_hash)?;

        // Générer un ID unique pour la position
        let position_id = Hash::from_bytes([
            &holder.as_bytes()[..16],
            &amount.to_le_bytes(),
            &Utc::now().timestamp().to_le_bytes(),
        ].concat().try_into().unwrap());

        let position = LongtermPosition {
            position_id,
            holder: holder.clone(),
            amount,
            start_date: Utc::now(),
            commitment_months,
            commitment_end_date: Utc::now() + Duration::days((commitment_months * 30) as i64),
            current_multiplier: self.calculate_longterm_multiplier(commitment_months),
            unclaimed_bonus: 0,
            last_bonus_claim: None,
            status: PositionStatus::Active,
        };

        // Ajouter à la liste des positions
        self.longterm_bonus_system.longterm_positions
            .entry(holder)
            .or_insert_with(Vec::new)
            .push(position);

        self.deflation_metrics.total_longterm_locked += amount;
        self.deflation_metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();

        Ok(position_id)
    }

    /// Calcule et distribue les bonus long terme
    pub fn distribute_longterm_bonuses(&mut self, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let mut total_distributed = 0;

        for (holder, positions) in &mut self.longterm_bonus_system.longterm_positions {
            for position in positions {
                if position.status != PositionStatus::Active {
                    continue;
                }

                let bonus = self.calculate_position_bonus(position)?;
                if bonus > 0 {
                    // Mint les bonus vers le détenteur
                    token.mint(holder, bonus, tx_hash)?;

                    // Enregistrer le bonus
                    self.longterm_bonus_system.bonus_history.push(LongtermBonusRecord {
                        beneficiary: holder.clone(),
                        position_id: position.position_id,
                        bonus_amount: bonus,
                        multiplier_applied: position.current_multiplier,
                        period_days: self.config.bonus_distribution_frequency_days,
                        distribution_date: Utc::now(),
                        transaction_hash: tx_hash,
                    });

                    position.unclaimed_bonus = 0;
                    position.last_bonus_claim = Some(Utc::now());
                    total_distributed += bonus;
                }
            }
        }

        self.deflation_metrics.total_longterm_bonus_distributed += total_distributed;
        self.deflation_metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();

        Ok(total_distributed)
    }

    /// Calcule le multiplicateur selon la durée d'engagement
    fn calculate_longterm_multiplier(&self, commitment_months: u32) -> f64 {
        if commitment_months >= 24 {
            self.longterm_bonus_system.multipliers.two_years_plus
        } else if commitment_months >= 12 {
            self.longterm_bonus_system.multipliers.one_year
        } else if commitment_months >= 6 {
            self.longterm_bonus_system.multipliers.six_months
        } else {
            1.0
        }
    }

    /// Calcule le bonus pour une position
    fn calculate_position_bonus(&self, position: &LongtermPosition) -> TokenOperationResult<u64> {
        let now = Utc::now();
        
        // Vérifier si la distribution de bonus est due
        let last_claim = position.last_bonus_claim.unwrap_or(position.start_date);
        let days_since_last_claim = (now - last_claim).num_days();
        
        if days_since_last_claim < self.config.bonus_distribution_frequency_days as i64 {
            return Ok(0);
        }

        // Calculer le bonus basé sur le montant et le multiplicateur
        // Base: 0.1% par mois, ajusté par le multiplicateur
        let monthly_rate = 0.001; // 0.1% par mois
        let periods = days_since_last_claim as f64 / 30.0; // Périodes mensuelles
        let base_bonus = (position.amount as f64 * monthly_rate * periods) as u64;
        let bonus_with_multiplier = (base_bonus as f64 * position.current_multiplier) as u64;

        Ok(bonus_with_multiplier)
    }

    /// Retire un stake de qualité
    pub fn withdraw_quality_stake(&mut self, staker: &PublicKey, token: &mut ARCToken, tx_hash: Hash) -> TokenOperationResult<u64> {
        let stake = self.quality_staking_pool.active_stakes.remove(staker)
            .ok_or_else(|| TokenOperationError::Internal {
                message: "Stake de qualité non trouvé".to_string(),
            })?;

        if stake.status == StakeStatus::Slashed {
            return Err(TokenOperationError::Internal {
                message: "Impossible de retirer un stake slashé".to_string(),
            });
        }

        // Déverrouiller les tokens restants
        token.unlock_tokens(staker, stake.amount, "quality_stake", tx_hash)?;

        self.quality_staking_pool.total_staked -= stake.amount;
        self.deflation_metrics.total_quality_staked -= stake.amount;
        self.deflation_metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();

        Ok(stake.amount)
    }

    /// Obtient les métriques de déflation actuelles
    pub fn get_deflation_metrics(&self) -> &DeflationMetrics {
        &self.deflation_metrics
    }

    /// Met à jour le taux de déflation estimé
    pub fn update_deflation_rate_estimate(&mut self, current_supply: u64) {
        if current_supply > 0 {
            // Estimer le taux annuel basé sur les burns récents
            let annual_burn_estimate = self.deflation_metrics.burned_this_period * 12; // Approximation mensuelle
            self.deflation_metrics.estimated_annual_deflation_rate = 
                (annual_burn_estimate as f64 / current_supply as f64) * 100.0;
        }
        
        self.deflation_metrics.last_updated = Utc::now();
        self.last_updated = Utc::now();
    }
}

impl Default for DeflationaryMechanisms {
    fn default() -> Self {
        Self::new(DeflationConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_deflationary_mechanisms_creation() {
        let mechanisms = DeflationaryMechanisms::default();
        assert_eq!(mechanisms.burn_rate, 0.10);
        assert_eq!(mechanisms.deflation_metrics.total_burned, 0);
    }

    #[test]
    fn test_burn_transaction_fees() {
        let mut mechanisms = DeflationaryMechanisms::default();
        let mut token = ARCToken::new();
        let tx_hash = Hash::zero();

        // Mint des tokens au système pour pouvoir les brûler
        token.mint(&super::super::system_address(), 1000, tx_hash).unwrap();

        let burned = mechanisms.burn_transaction_fees(100, tx_hash, &mut token).unwrap();
        assert_eq!(burned, 10); // 10% de 100
        assert_eq!(mechanisms.deflation_metrics.total_burned, 10);
    }

    #[test]
    fn test_quality_stake_creation() {
        let mut mechanisms = DeflationaryMechanisms::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let staker = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Mint des tokens au staker
        token.mint(&staker, 50_000, tx_hash).unwrap();

        let result = mechanisms.create_quality_stake(
            staker.clone(),
            50_000,
            QualityLevel::Standard,
            &mut token,
            tx_hash,
        );

        assert!(result.is_ok());
        assert!(mechanisms.quality_staking_pool.active_stakes.contains_key(&staker));
        assert_eq!(mechanisms.quality_staking_pool.total_staked, 50_000);
    }

    #[test]
    fn test_longterm_position_creation() {
        let mut mechanisms = DeflationaryMechanisms::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let holder = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Mint des tokens au holder
        token.mint(&holder, 100_000, tx_hash).unwrap();

        let position_id = mechanisms.create_longterm_position(
            holder.clone(),
            100_000,
            12, // 12 mois
            &mut token,
            tx_hash,
        ).unwrap();

        assert!(!position_id.is_zero());
        assert!(mechanisms.longterm_bonus_system.longterm_positions.contains_key(&holder));
        assert_eq!(mechanisms.deflation_metrics.total_longterm_locked, 100_000);
    }

    #[test]
    fn test_quality_slashing() {
        let mut mechanisms = DeflationaryMechanisms::default();
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let staker = keypair.public_key().clone();
        let tx_hash = Hash::zero();

        // Setup stake
        token.mint(&staker, 50_000, tx_hash).unwrap();
        mechanisms.create_quality_stake(
            staker.clone(),
            50_000,
            QualityLevel::Standard,
            &mut token,
            tx_hash,
        ).unwrap();

        // Mint des tokens système pour le slashing
        token.mint(&super::super::system_address(), 10_000, tx_hash).unwrap();

        // Simuler une mauvaise qualité (0.5 < 0.8)
        let slashed = mechanisms.evaluate_quality_and_slash(&staker, 0.5, &mut token, tx_hash).unwrap();
        
        assert!(slashed > 0);
        assert_eq!(mechanisms.deflation_metrics.total_slashed, slashed);
    }

    #[test]
    fn test_longterm_multiplier_calculation() {
        let mechanisms = DeflationaryMechanisms::default();
        
        assert_eq!(mechanisms.calculate_longterm_multiplier(3), 1.0);   // < 6 mois
        assert_eq!(mechanisms.calculate_longterm_multiplier(6), 1.2);   // 6 mois
        assert_eq!(mechanisms.calculate_longterm_multiplier(12), 1.5);  // 1 an
        assert_eq!(mechanisms.calculate_longterm_multiplier(24), 2.0);  // 2 ans+
    }
}