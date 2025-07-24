//! Smart contract pour les Preservation Pools d'ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey};
use crate::contracts::{
    ContractError, ContractResult, ContractContext, SmartContract, 
    ContractMetadata, ContractVersion, AbiValue
};

/// Statut d'un pool de préservation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PoolStatus {
    /// Pool actif, accepte les contributions et distribue les récompenses
    Active,
    /// Pool suspendu temporairement
    Suspended,
    /// Pool terminé, plus de distributions
    Ended,
    /// Pool en cours de liquidation
    Liquidating,
}

/// Type de contribution à un pool
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContributionType {
    /// Contribution ponctuelle
    OneTime,
    /// Contribution récurrente mensuelle
    Monthly,
    /// Contribution récurrente annuelle
    Yearly,
}

/// Participant à un pool de préservation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolParticipant {
    /// Adresse du participant
    pub address: PublicKey,
    /// Montant total contribué
    pub total_contributed: u64,
    /// Date de première contribution
    pub joined_at: DateTime<Utc>,
    /// Date de dernière contribution
    pub last_contribution: DateTime<Utc>,
    /// Type de contribution
    pub contribution_type: ContributionType,
    /// Montant de contribution récurrente (si applicable)
    pub recurring_amount: Option<u64>,
    /// Durée d'engagement minimum (en mois)
    pub commitment_months: u32,
    /// Récompenses accumulées non réclamées
    pub unclaimed_rewards: u64,
    /// Total des récompenses reçues
    pub total_rewards_received: u64,
    /// Pénalités appliquées
    pub penalties_applied: u64,
    /// Statut du participant
    pub status: ParticipantStatus,
}

/// Statut d'un participant
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParticipantStatus {
    /// Participant actif
    Active,
    /// Participant ayant quitté le pool
    Withdrawn,
    /// Participant suspendu (non-respect des engagements)
    Suspended,
    /// Participant en période de grâce
    GracePeriod,
}

/// Règles de distribution des récompenses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionRules {
    /// Fréquence de distribution (en jours)
    pub distribution_frequency_days: u32,
    /// Pourcentage distribué à chaque cycle (0.0 à 1.0)
    pub distribution_percentage: f64,
    /// Bonus pour les contributions de longue durée
    pub longevity_bonus_multiplier: f64,
    /// Pénalité pour retrait précoce (0.0 à 1.0)
    pub early_withdrawal_penalty: f64,
    /// Période de grâce pour les contributions manquées (jours)
    pub grace_period_days: u32,
    /// Récompense minimum par distribution
    pub minimum_reward: u64,
}

impl Default for DistributionRules {
    fn default() -> Self {
        Self {
            distribution_frequency_days: 30, // Mensuel
            distribution_percentage: 0.05,   // 5% par mois
            longevity_bonus_multiplier: 1.1, // 10% bonus par année
            early_withdrawal_penalty: 0.15,  // 15% de pénalité
            grace_period_days: 7,            // 7 jours de grâce
            minimum_reward: 1,               // 1 ARC minimum
        }
    }
}

/// Historique de distribution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionRecord {
    /// ID unique de la distribution
    pub distribution_id: Hash,
    /// Timestamp de la distribution
    pub timestamp: DateTime<Utc>,
    /// Montant total distribué
    pub total_amount: u64,
    /// Nombre de participants récompensés
    pub participants_count: u32,
    /// Montant moyen par participant
    pub average_reward: u64,
    /// Hash du snapshot des participants
    pub participants_snapshot: Hash,
}

/// Pool de préservation principal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreservationPool {
    /// ID unique du pool
    pub pool_id: u64,
    /// Nom du pool
    pub name: String,
    /// Description du pool
    pub description: String,
    /// Contenu ciblé pour la préservation
    pub target_content: Vec<Hash>,
    /// Fonds totaux du pool
    pub total_funds: u64,
    /// Fonds déjà distribués
    pub total_distributed: u64,
    /// Récompense mensuelle configurée
    pub monthly_reward: u64,
    /// Participants du pool
    pub participants: HashMap<PublicKey, PoolParticipant>,
    /// Durée minimale de stockage requise (en mois)
    pub minimum_storage_time: u32,
    /// Règles de distribution
    pub distribution_rules: DistributionRules,
    /// Statut du pool
    pub status: PoolStatus,
    /// Date de création
    pub created_at: DateTime<Utc>,
    /// Dernière distribution
    pub last_distribution: Option<DateTime<Utc>>,
    /// Historique des distributions
    pub distribution_history: Vec<DistributionRecord>,
    /// Gestionnaire du pool
    pub manager: PublicKey,
    /// Date de fin prévue (optionnelle)
    pub end_date: Option<DateTime<Utc>>,
}

/// État du smart contract Preservation Pool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreservationPoolState {
    /// Compteur pour les IDs de pool
    pub next_pool_id: u64,
    /// Pools indexés par ID
    pub pools: HashMap<u64, PreservationPool>,
    /// Index des pools par gestionnaire
    pub pools_by_manager: HashMap<PublicKey, Vec<u64>>,
    /// Index des pools par statut
    pub pools_by_status: HashMap<PoolStatus, Vec<u64>>,
    /// Participants globaux (pour éviter les doublons)
    pub global_participants: HashMap<PublicKey, Vec<u64>>,
    /// Statistiques globales
    pub stats: PoolStats,
}

/// Statistiques des pools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    pub total_pools_created: u64,
    pub total_funds_pooled: u64,
    pub total_rewards_distributed: u64,
    pub total_participants: u64,
    pub average_pool_size: u64,
    pub active_pools_count: u64,
}

impl Default for PreservationPoolState {
    fn default() -> Self {
        Self {
            next_pool_id: 1,
            pools: HashMap::new(),
            pools_by_manager: HashMap::new(),
            pools_by_status: HashMap::new(),
            global_participants: HashMap::new(),
            stats: PoolStats {
                total_pools_created: 0,
                total_funds_pooled: 0,
                total_rewards_distributed: 0,
                total_participants: 0,
                average_pool_size: 0,
                active_pools_count: 0,
            },
        }
    }
}

/// Données d'appel pour les fonctions du contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreservationPoolCall {
    /// Crée un nouveau pool
    CreatePool {
        name: String,
        description: String,
        target_content: Vec<Hash>,
        monthly_reward: u64,
        minimum_storage_time: u32,
        distribution_rules: DistributionRules,
        end_date: Option<DateTime<Utc>>,
    },
    /// Rejoint un pool avec une contribution
    JoinPool {
        pool_id: u64,
        contribution_amount: u64,
        contribution_type: ContributionType,
        commitment_months: u32,
    },
    /// Ajoute une contribution à un pool existant
    Contribute {
        pool_id: u64,
        amount: u64,
    },
    /// Quitte un pool (avec pénalités éventuelles)
    LeavePool {
        pool_id: u64,
    },
    /// Distribue les récompenses d'un pool
    DistributeRewards {
        pool_id: u64,
    },
    /// Réclame les récompenses accumulées
    ClaimRewards {
        pool_id: u64,
    },
    /// Obtient les détails d'un pool
    GetPool {
        pool_id: u64,
    },
    /// Liste les pools par statut
    ListPoolsByStatus {
        status: PoolStatus,
        limit: u32,
        offset: u32,
    },
    /// Obtient les pools d'un participant
    GetParticipantPools {
        participant: PublicKey,
    },
}

/// Données de retour du contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PreservationPoolReturn {
    /// Pool créé
    PoolCreated { pool_id: u64 },
    /// Participation confirmée
    JoinedPool { participant_id: Hash },
    /// Contribution ajoutée
    ContributionAdded { new_total: u64 },
    /// Sortie du pool confirmée
    LeftPool { refund_amount: u64 },
    /// Récompenses distribuées
    RewardsDistributed { total_amount: u64, participants_count: u32 },
    /// Récompenses réclamées
    RewardsClaimed { amount: u64 },
    /// Détails du pool
    PoolDetails(PreservationPool),
    /// Liste de pools
    PoolList(Vec<PreservationPool>),
    /// Pools du participant
    ParticipantPools(Vec<u64>),
    /// Erreur
    Error(String),
}

/// Implémentation du smart contract Preservation Pool
pub struct PreservationPoolContract {
    state: PreservationPoolState,
}

impl Default for PreservationPoolContract {
    fn default() -> Self {
        Self {
            state: PreservationPoolState::default(),
        }
    }
}

impl PreservationPoolContract {
    /// Crée un nouveau pool de préservation
    pub fn create_pool(
        &mut self,
        manager: PublicKey,
        name: String,
        description: String,
        target_content: Vec<Hash>,
        monthly_reward: u64,
        minimum_storage_time: u32,
        distribution_rules: DistributionRules,
        end_date: Option<DateTime<Utc>>,
        context: &mut ContractContext,
    ) -> ContractResult<u64> {
        // Validations
        if name.is_empty() {
            return Err(ContractError::InvalidParameters {
                message: "Pool name cannot be empty".to_string(),
            });
        }

        if target_content.is_empty() {
            return Err(ContractError::InvalidParameters {
                message: "Target content cannot be empty".to_string(),
            });
        }

        if let Some(end) = end_date {
            if end <= Utc::now() {
                return Err(ContractError::InvalidParameters {
                    message: "End date must be in the future".to_string(),
                });
            }
        }

        let pool_id = self.state.next_pool_id;
        self.state.next_pool_id += 1;

        let pool = PreservationPool {
            pool_id,
            name: name.clone(),
            description,
            target_content,
            total_funds: 0,
            total_distributed: 0,
            monthly_reward,
            participants: HashMap::new(),
            minimum_storage_time,
            distribution_rules,
            status: PoolStatus::Active,
            created_at: Utc::now(),
            last_distribution: None,
            distribution_history: Vec::new(),
            manager: manager.clone(),
            end_date,
        };

        // Enregistre le pool
        self.state.pools.insert(pool_id, pool);
        
        // Met à jour les index
        self.state.pools_by_manager
            .entry(manager.clone())
            .or_insert_with(Vec::new)
            .push(pool_id);
        
        self.state.pools_by_status
            .entry(PoolStatus::Active)
            .or_insert_with(Vec::new)
            .push(pool_id);

        // Met à jour les statistiques
        self.state.stats.total_pools_created += 1;
        self.state.stats.active_pools_count += 1;

        // Émet un event
        context.emit_event(
            "PoolCreated".to_string(),
            bincode::serialize(&pool_id).unwrap_or_default(),
            vec![context.compute_hash(&manager.as_bytes())?],
        );

        context.emit_log(format!(
            "Preservation pool '{}' created with ID {} by {:?}",
            name, pool_id, manager
        ));

        Ok(pool_id)
    }

    /// Rejoint un pool avec une contribution
    pub fn join_pool(
        &mut self,
        participant: PublicKey,
        pool_id: u64,
        contribution_amount: u64,
        contribution_type: ContributionType,
        commitment_months: u32,
        context: &mut ContractContext,
    ) -> ContractResult<Hash> {
        // Vérifie que le pool existe et est actif
        let pool = self.state.pools.get_mut(&pool_id)
            .ok_or(ContractError::InvalidParameters {
                message: format!("Pool {} not found", pool_id),
            })?;

        if pool.status != PoolStatus::Active {
            return Err(ContractError::InvalidState {
                message: "Pool is not active".to_string(),
            });
        }

        // Vérifie que le participant n'est pas déjà dans le pool
        if pool.participants.contains_key(&participant) {
            return Err(ContractError::InvalidState {
                message: "Already participating in this pool".to_string(),
            });
        }

        // Vérifie que le participant a suffisamment de fonds
        let participant_balance = context.get_balance(&participant)?;
        if participant_balance < contribution_amount {
            return Err(ContractError::InsufficientFunds {
                required: contribution_amount,
                available: participant_balance,
            });
        }

        // Transfert des fonds vers le pool
        context.transfer_tokens(context.get_contract_address().into(), contribution_amount)?;

        // Crée le participant
        let pool_participant = PoolParticipant {
            address: participant.clone(),
            total_contributed: contribution_amount,
            joined_at: Utc::now(),
            last_contribution: Utc::now(),
            contribution_type,
            recurring_amount: match contribution_type {
                ContributionType::OneTime => None,
                _ => Some(contribution_amount / commitment_months as u64),
            },
            commitment_months,
            unclaimed_rewards: 0,
            total_rewards_received: 0,
            penalties_applied: 0,
            status: ParticipantStatus::Active,
        };

        // Ajoute le participant au pool
        pool.participants.insert(participant.clone(), pool_participant);
        pool.total_funds += contribution_amount;

        // Met à jour les index globaux
        self.state.global_participants
            .entry(participant.clone())
            .or_insert_with(Vec::new)
            .push(pool_id);

        // Met à jour les statistiques
        self.state.stats.total_funds_pooled += contribution_amount;
        if self.state.global_participants.get(&participant).unwrap().len() == 1 {
            self.state.stats.total_participants += 1;
        }
        self.update_average_pool_size();

        // Génère un ID de participant
        let participant_id = context.compute_hash(&bincode::serialize(&(
            pool_id,
            &participant,
            Utc::now().timestamp()
        )).unwrap_or_default())?;

        // Émet un event
        context.emit_event(
            "ParticipantJoined".to_string(),
            bincode::serialize(&participant_id).unwrap_or_default(),
            vec![
                context.compute_hash(&participant.as_bytes())?,
                context.compute_hash(&pool_id.to_le_bytes())?,
            ],
        );

        context.emit_log(format!(
            "Participant {:?} joined pool {} with contribution {} ARC",
            participant, pool_id, contribution_amount
        ));

        Ok(participant_id)
    }

    /// Distribue les récompenses d'un pool
    pub fn distribute_rewards(
        &mut self,
        pool_id: u64,
        context: &mut ContractContext,
    ) -> ContractResult<(u64, u32)> {
        let pool = self.state.pools.get_mut(&pool_id)
            .ok_or(ContractError::InvalidParameters {
                message: format!("Pool {} not found", pool_id),
            })?;

        if pool.status != PoolStatus::Active {
            return Err(ContractError::InvalidState {
                message: "Pool is not active".to_string(),
            });
        }

        // Vérifie si c'est le moment de distribuer
        let should_distribute = match pool.last_distribution {
            None => true,
            Some(last) => {
                let days_since = (Utc::now() - last).num_days();
                days_since >= pool.distribution_rules.distribution_frequency_days as i64
            }
        };

        if !should_distribute {
            return Err(ContractError::InvalidState {
                message: "Distribution not due yet".to_string(),
            });
        }

        // Calcule le montant total à distribuer
        let available_funds = pool.total_funds - pool.total_distributed;
        let distribution_amount = ((available_funds as f64) * pool.distribution_rules.distribution_percentage) as u64;
        
        if distribution_amount < pool.distribution_rules.minimum_reward {
            return Err(ContractError::InsufficientFunds {
                required: pool.distribution_rules.minimum_reward,
                available: distribution_amount,
            });
        }

        let active_participants: Vec<_> = pool.participants
            .iter_mut()
            .filter(|(_, p)| p.status == ParticipantStatus::Active)
            .collect();

        if active_participants.is_empty() {
            return Err(ContractError::InvalidState {
                message: "No active participants to reward".to_string(),
            });
        }

        let mut total_distributed = 0u64;
        let mut participants_count = 0u32;

        // Calcule et distribue les récompenses
        for (address, participant) in active_participants {
            let reward = self.calculate_participant_reward(
                participant,
                distribution_amount,
                pool.participants.len(),
                &pool.distribution_rules,
            );

            if reward > 0 {
                participant.unclaimed_rewards += reward;
                total_distributed += reward;
                participants_count += 1;

                context.emit_log(format!(
                    "Rewarded {} ARC to participant {:?} in pool {}",
                    reward, address, pool_id
                ));
            }
        }

        // Met à jour le pool
        pool.total_distributed += total_distributed;
        pool.last_distribution = Some(Utc::now());

        // Enregistre la distribution
        let distribution_record = DistributionRecord {
            distribution_id: context.compute_hash(&bincode::serialize(&(
                pool_id,
                Utc::now().timestamp(),
                total_distributed
            )).unwrap_or_default())?,
            timestamp: Utc::now(),
            total_amount: total_distributed,
            participants_count,
            average_reward: if participants_count > 0 { total_distributed / participants_count as u64 } else { 0 },
            participants_snapshot: Hash::zero(), // Simplifié pour l'exemple
        };

        pool.distribution_history.push(distribution_record);

        // Met à jour les statistiques globales
        self.state.stats.total_rewards_distributed += total_distributed;

        // Émet un event
        context.emit_event(
            "RewardsDistributed".to_string(),
            bincode::serialize(&total_distributed).unwrap_or_default(),
            vec![context.compute_hash(&pool_id.to_le_bytes())?],
        );

        context.emit_log(format!(
            "Distributed {} ARC to {} participants in pool {}",
            total_distributed, participants_count, pool_id
        ));

        Ok((total_distributed, participants_count))
    }

    /// Calcule la récompense d'un participant
    fn calculate_participant_reward(
        &self,
        participant: &PoolParticipant,
        total_distribution: u64,
        total_participants: usize,
        rules: &DistributionRules,
    ) -> u64 {
        // Récompense de base (distribution équitable)
        let base_reward = total_distribution / total_participants as u64;

        // Bonus de longévité
        let months_active = (Utc::now() - participant.joined_at).num_weeks() / 4;
        let longevity_multiplier = 1.0 + (months_active as f64 / 12.0) * (rules.longevity_bonus_multiplier - 1.0);

        // Bonus de contribution (plus on contribue, plus on reçoit)
        let contribution_multiplier = if participant.total_contributed > 0 {
            1.0 + (participant.total_contributed as f64).log10() / 10.0
        } else {
            1.0
        };

        let final_reward = (base_reward as f64 * longevity_multiplier * contribution_multiplier) as u64;
        final_reward.max(rules.minimum_reward)
    }

    /// Permet à un participant de réclamer ses récompenses
    pub fn claim_rewards(
        &mut self,
        claimer: PublicKey,
        pool_id: u64,
        context: &mut ContractContext,
    ) -> ContractResult<u64> {
        let pool = self.state.pools.get_mut(&pool_id)
            .ok_or(ContractError::InvalidParameters {
                message: format!("Pool {} not found", pool_id),
            })?;

        let participant = pool.participants.get_mut(&claimer)
            .ok_or(ContractError::InvalidParameters {
                message: "Not a participant in this pool".to_string(),
            })?;

        let reward_amount = participant.unclaimed_rewards;
        if reward_amount == 0 {
            return Err(ContractError::InvalidState {
                message: "No rewards to claim".to_string(),
            });
        }

        // Transfert des récompenses
        context.transfer_tokens(claimer.clone(), reward_amount)?;

        // Met à jour le participant
        participant.unclaimed_rewards = 0;
        participant.total_rewards_received += reward_amount;

        // Émet un event
        context.emit_event(
            "RewardsClaimed".to_string(),
            bincode::serialize(&reward_amount).unwrap_or_default(),
            vec![
                context.compute_hash(&claimer.as_bytes())?,
                context.compute_hash(&pool_id.to_le_bytes())?,
            ],
        );

        context.emit_log(format!(
            "Participant {:?} claimed {} ARC from pool {}",
            claimer, reward_amount, pool_id
        ));

        Ok(reward_amount)
    }

    /// Met à jour la taille moyenne des pools
    fn update_average_pool_size(&mut self) {
        if self.state.stats.total_pools_created > 0 {
            self.state.stats.average_pool_size = 
                self.state.stats.total_funds_pooled / self.state.stats.total_pools_created;
        }
    }

    /// Obtient les détails d'un pool
    pub fn get_pool(&self, pool_id: u64) -> ContractResult<PreservationPool> {
        self.state.pools.get(&pool_id)
            .cloned()
            .ok_or(ContractError::InvalidParameters {
                message: format!("Pool {} not found", pool_id),
            })
    }

    /// Liste les pools d'un participant
    pub fn get_participant_pools(&self, participant: &PublicKey) -> ContractResult<Vec<u64>> {
        Ok(self.state.global_participants
            .get(participant)
            .cloned()
            .unwrap_or_default())
    }
}

impl SmartContract for PreservationPoolContract {
    type State = PreservationPoolState;
    type CallData = PreservationPoolCall;
    type ReturnData = PreservationPoolReturn;

    fn initialize(&mut self, _context: &mut ContractContext) -> ContractResult<()> {
        self.state = PreservationPoolState::default();
        Ok(())
    }

    fn call(
        &mut self,
        _function: &str,
        call_data: Self::CallData,
        context: &mut ContractContext,
    ) -> ContractResult<Self::ReturnData> {
        match call_data {
            PreservationPoolCall::CreatePool {
                name,
                description,
                target_content,
                monthly_reward,
                minimum_storage_time,
                distribution_rules,
                end_date,
            } => {
                let manager = context.get_caller().clone();
                let pool_id = self.create_pool(
                    manager,
                    name,
                    description,
                    target_content,
                    monthly_reward,
                    minimum_storage_time,
                    distribution_rules,
                    end_date,
                    context,
                )?;
                Ok(PreservationPoolReturn::PoolCreated { pool_id })
            }
            
            PreservationPoolCall::JoinPool {
                pool_id,
                contribution_amount,
                contribution_type,
                commitment_months,
            } => {
                let participant = context.get_caller().clone();
                let participant_id = self.join_pool(
                    participant,
                    pool_id,
                    contribution_amount,
                    contribution_type,
                    commitment_months,
                    context,
                )?;
                Ok(PreservationPoolReturn::JoinedPool { participant_id })
            }
            
            PreservationPoolCall::DistributeRewards { pool_id } => {
                let (total_amount, participants_count) = self.distribute_rewards(pool_id, context)?;
                Ok(PreservationPoolReturn::RewardsDistributed { total_amount, participants_count })
            }
            
            PreservationPoolCall::ClaimRewards { pool_id } => {
                let claimer = context.get_caller().clone();
                let amount = self.claim_rewards(claimer, pool_id, context)?;
                Ok(PreservationPoolReturn::RewardsClaimed { amount })
            }
            
            PreservationPoolCall::GetPool { pool_id } => {
                let pool = self.get_pool(pool_id)?;
                Ok(PreservationPoolReturn::PoolDetails(pool))
            }
            
            PreservationPoolCall::GetParticipantPools { participant } => {
                let pools = self.get_participant_pools(&participant)?;
                Ok(PreservationPoolReturn::ParticipantPools(pools))
            }
            
            _ => Err(ContractError::InvalidParameters {
                message: "Function not implemented".to_string(),
            }),
        }
    }

    fn get_state(&self) -> &Self::State {
        &self.state
    }

    fn set_state(&mut self, state: Self::State) {
        self.state = state;
    }

    fn metadata(&self) -> ContractMetadata {
        ContractMetadata {
            name: "PreservationPoolContract".to_string(),
            version: ContractVersion::new(1, 0, 0),
            description: "Smart contract for community preservation pools".to_string(),
            author: "ArchiveChain Team".to_string(),
            license: "MIT".to_string(),
            abi_hash: Hash::zero(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contracts::context::MockContextProvider;
    use crate::contracts::ExecutionEnvironment;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_pool_creation() {
        let mut contract = PreservationPoolContract::default();
        let keypair = generate_keypair().unwrap();
        
        let env = ExecutionEnvironment {
            block_hash: Hash::zero(),
            block_number: 1,
            block_timestamp: Utc::now(),
            transaction_hash: Hash::zero(),
            transaction_sender: keypair.public_key().clone(),
            contract_address: Hash::zero(),
            caller_address: keypair.public_key().clone(),
            value_sent: 0,
            gas_limit: 1000000,
            gas_price: 1,
        };

        let provider = MockContextProvider::new();
        let mut context = ContractContext::new(env, Box::new(provider));

        let pool_id = contract.create_pool(
            keypair.public_key().clone(),
            "Test Pool".to_string(),
            "A test preservation pool".to_string(),
            vec![Hash::zero()],
            1000,
            12,
            DistributionRules::default(),
            None,
            &mut context,
        ).unwrap();

        assert_eq!(pool_id, 1);
        assert_eq!(contract.state.pools.len(), 1);
        
        let pool = contract.get_pool(pool_id).unwrap();
        assert_eq!(pool.name, "Test Pool");
        assert_eq!(pool.status, PoolStatus::Active);
    }

    #[test]
    fn test_contribution_types() {
        assert_eq!(ContributionType::OneTime, ContributionType::OneTime);
        assert_ne!(ContributionType::OneTime, ContributionType::Monthly);
    }

    #[test]
    fn test_distribution_rules() {
        let rules = DistributionRules::default();
        assert_eq!(rules.distribution_frequency_days, 30);
        assert_eq!(rules.distribution_percentage, 0.05);
        assert_eq!(rules.early_withdrawal_penalty, 0.15);
    }
}