//! Smart contract pour les Archive Bounties d'ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc, Duration};
use crate::crypto::{Hash, PublicKey};
use crate::contracts::{
    ContractError, ContractResult, ContractContext, SmartContract, 
    ContractMetadata, ContractVersion, AbiValue
};

/// Niveau de qualité requis pour un archivage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityLevel {
    /// Archivage basique (compression minimale, vérification simple)
    Basic,
    /// Archivage standard (compression moyenne, vérifications standard)
    Standard,
    /// Archivage haute qualité (compression optimale, vérifications étendues)
    High,
    /// Archivage premium (redondance, vérifications cryptographiques)
    Premium,
}

impl QualityLevel {
    /// Obtient le multiplicateur de récompense pour ce niveau
    pub fn reward_multiplier(&self) -> f64 {
        match self {
            QualityLevel::Basic => 1.0,
            QualityLevel::Standard => 1.5,
            QualityLevel::High => 2.0,
            QualityLevel::Premium => 3.0,
        }
    }

    /// Obtient le coût de gas additionnel pour ce niveau
    pub fn gas_cost(&self) -> u64 {
        match self {
            QualityLevel::Basic => 0,
            QualityLevel::Standard => 1000,
            QualityLevel::High => 2500,
            QualityLevel::Premium => 5000,
        }
    }
}

/// Statut d'un bounty d'archivage
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BountyStatus {
    /// Bounty actif, en attente de soumissions
    Active,
    /// Bounty complété avec succès
    Completed,
    /// Bounty expiré sans soumission valide
    Expired,
    /// Bounty annulé par le créateur
    Cancelled,
    /// Bounty en cours de validation
    Validating,
}

/// Métadonnées d'archive requises
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// URL ou identifiant du contenu à archiver
    pub content_url: String,
    /// Taille estimée du contenu (en bytes)
    pub estimated_size: u64,
    /// Type MIME du contenu
    pub content_type: String,
    /// Hash du contenu original (si disponible)
    pub original_hash: Option<Hash>,
    /// Métadonnées additionnelles
    pub additional_metadata: HashMap<String, String>,
}

/// Soumission d'archivage pour un bounty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveSubmission {
    /// ID unique de la soumission
    pub submission_id: Hash,
    /// Adresse du soumetteur
    pub submitter: PublicKey,
    /// Hash de l'archive soumise
    pub archive_hash: Hash,
    /// Métadonnées de l'archive
    pub metadata: ArchiveMetadata,
    /// Preuve de stockage
    pub storage_proof: Vec<u8>,
    /// Timestamp de soumission
    pub submitted_at: DateTime<Utc>,
    /// Statut de validation
    pub validation_status: ValidationStatus,
    /// Score de qualité (0.0 à 1.0)
    pub quality_score: f64,
}

/// Statut de validation d'une soumission
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationStatus {
    /// En attente de validation
    Pending,
    /// Validé avec succès
    Validated,
    /// Rejeté (avec raison)
    Rejected(String),
    /// En cours de validation
    InProgress,
}

/// Structure principale d'un Archive Bounty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveBounty {
    /// ID unique du bounty
    pub bounty_id: u64,
    /// URL ou description du contenu à archiver
    pub target_url: String,
    /// Récompense en tokens ARC
    pub reward: u64,
    /// Date limite pour les soumissions
    pub deadline: DateTime<Utc>,
    /// Adresse du créateur du bounty
    pub creator: PublicKey,
    /// Statut du bounty
    pub status: BountyStatus,
    /// Niveau de qualité requis
    pub required_quality: QualityLevel,
    /// Métadonnées du contenu cible
    pub target_metadata: ArchiveMetadata,
    /// Soumissions reçues
    pub submissions: Vec<ArchiveSubmission>,
    /// Gagnant sélectionné (si complété)
    pub winner: Option<PublicKey>,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Nombre maximum de soumissions acceptées
    pub max_submissions: u32,
    /// Critères de validation personnalisés
    pub validation_criteria: ValidationCriteria,
}

/// Critères de validation pour un bounty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationCriteria {
    /// Taille minimale acceptable (bytes)
    pub min_size: Option<u64>,
    /// Taille maximale acceptable (bytes)
    pub max_size: Option<u64>,
    /// Types de fichiers acceptés
    pub allowed_content_types: Vec<String>,
    /// Niveau de compression minimum requis
    pub min_compression_ratio: Option<f64>,
    /// Durée de stockage minimale requise (heures)
    pub min_storage_duration: u64,
    /// Validation automatique activée
    pub auto_validation: bool,
}

impl Default for ValidationCriteria {
    fn default() -> Self {
        Self {
            min_size: None,
            max_size: Some(1_000_000_000), // 1GB par défaut
            allowed_content_types: vec!["*/*".to_string()], // Tous types par défaut
            min_compression_ratio: None,
            min_storage_duration: 24 * 30, // 30 jours par défaut
            auto_validation: true,
        }
    }
}

/// État du smart contract Archive Bounty
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveBountyState {
    /// Compteur pour les IDs de bounty
    pub next_bounty_id: u64,
    /// Bounties actifs indexés par ID
    pub bounties: HashMap<u64, ArchiveBounty>,
    /// Index des bounties par créateur
    pub bounties_by_creator: HashMap<PublicKey, Vec<u64>>,
    /// Index des bounties par statut
    pub bounties_by_status: HashMap<BountyStatus, Vec<u64>>,
    /// Pool total de récompenses
    pub total_reward_pool: u64,
    /// Statistiques globales
    pub stats: BountyStats,
}

/// Statistiques des bounties
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BountyStats {
    pub total_bounties_created: u64,
    pub total_bounties_completed: u64,
    pub total_rewards_distributed: u64,
    pub average_completion_time: u64, // en heures
    pub total_archived_content_size: u64, // en bytes
}

impl Default for ArchiveBountyState {
    fn default() -> Self {
        Self {
            next_bounty_id: 1,
            bounties: HashMap::new(),
            bounties_by_creator: HashMap::new(),
            bounties_by_status: HashMap::new(),
            total_reward_pool: 0,
            stats: BountyStats {
                total_bounties_created: 0,
                total_bounties_completed: 0,
                total_rewards_distributed: 0,
                average_completion_time: 0,
                total_archived_content_size: 0,
            },
        }
    }
}

/// Données d'appel pour les fonctions du contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveBountyCall {
    /// Crée un nouveau bounty
    CreateBounty {
        target_url: String,
        reward: u64,
        deadline_hours: u64,
        quality_level: QualityLevel,
        metadata: ArchiveMetadata,
        criteria: ValidationCriteria,
    },
    /// Soumet une archive pour un bounty
    SubmitArchive {
        bounty_id: u64,
        archive_hash: Hash,
        metadata: ArchiveMetadata,
        storage_proof: Vec<u8>,
    },
    /// Valide une soumission
    ValidateSubmission {
        bounty_id: u64,
        submission_id: Hash,
        approved: bool,
        quality_score: f64,
        notes: String,
    },
    /// Annule un bounty
    CancelBounty {
        bounty_id: u64,
    },
    /// Récupère les détails d'un bounty
    GetBounty {
        bounty_id: u64,
    },
    /// Liste les bounties par statut
    ListBountiesByStatus {
        status: BountyStatus,
        limit: u32,
        offset: u32,
    },
}

/// Données de retour du contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArchiveBountyReturn {
    /// ID du bounty créé
    BountyCreated { bounty_id: u64 },
    /// Confirmation de soumission
    SubmissionReceived { submission_id: Hash },
    /// Confirmation de validation
    ValidationCompleted { bounty_completed: bool },
    /// Confirmation d'annulation
    BountyCancelled,
    /// Détails d'un bounty
    BountyDetails(ArchiveBounty),
    /// Liste de bounties
    BountyList(Vec<ArchiveBounty>),
    /// Erreur
    Error(String),
}

/// Implémentation du smart contract Archive Bounty
pub struct ArchiveBountyContract {
    state: ArchiveBountyState,
}

impl Default for ArchiveBountyContract {
    fn default() -> Self {
        Self {
            state: ArchiveBountyState::default(),
        }
    }
}

impl ArchiveBountyContract {
    /// Crée un nouveau bounty d'archivage
    pub fn create_bounty(
        &mut self,
        creator: PublicKey,
        target_url: String,
        reward: u64,
        deadline_hours: u64,
        quality_level: QualityLevel,
        metadata: ArchiveMetadata,
        criteria: ValidationCriteria,
        context: &mut ContractContext,
    ) -> ContractResult<u64> {
        // Vérifie que le créateur a suffisamment de fonds
        let creator_balance = context.get_balance(&creator)?;
        if creator_balance < reward {
            return Err(ContractError::InsufficientFunds {
                required: reward,
                available: creator_balance,
            });
        }

        // Calcule la deadline
        let deadline = Utc::now() + Duration::hours(deadline_hours as i64);
        if deadline <= Utc::now() {
            return Err(ContractError::InvalidParameters {
                message: "Deadline must be in the future".to_string(),
            });
        }

        let bounty_id = self.state.next_bounty_id;
        self.state.next_bounty_id += 1;

        let bounty = ArchiveBounty {
            bounty_id,
            target_url,
            reward,
            deadline,
            creator: creator.clone(),
            status: BountyStatus::Active,
            required_quality: quality_level,
            target_metadata: metadata,
            submissions: Vec::new(),
            winner: None,
            created_at: Utc::now(),
            max_submissions: criteria.auto_validation.then(|| 10).unwrap_or(100),
            validation_criteria: criteria,
        };

        // Enregistre le bounty
        self.state.bounties.insert(bounty_id, bounty);
        
        // Met à jour les index
        self.state.bounties_by_creator
            .entry(creator)
            .or_insert_with(Vec::new)
            .push(bounty_id);
        
        self.state.bounties_by_status
            .entry(BountyStatus::Active)
            .or_insert_with(Vec::new)
            .push(bounty_id);

        // Met à jour les statistiques
        self.state.stats.total_bounties_created += 1;
        self.state.total_reward_pool += reward;

        // Émet un event
        context.emit_event(
            "BountyCreated".to_string(),
            bincode::serialize(&bounty_id).unwrap_or_default(),
            vec![context.compute_hash(&creator.as_bytes())?],
        );

        // Log
        context.emit_log(format!(
            "Archive bounty {} created by {:?} with reward {} ARC",
            bounty_id, creator, reward
        ));

        Ok(bounty_id)
    }

    /// Soumet une archive pour un bounty
    pub fn submit_archive(
        &mut self,
        submitter: PublicKey,
        bounty_id: u64,
        archive_hash: Hash,
        metadata: ArchiveMetadata,
        storage_proof: Vec<u8>,
        context: &mut ContractContext,
    ) -> ContractResult<Hash> {
        let bounty = self.state.bounties.get_mut(&bounty_id)
            .ok_or(ContractError::InvalidParameters {
                message: format!("Bounty {} not found", bounty_id),
            })?;

        // Vérifie que le bounty est actif
        if bounty.status != BountyStatus::Active {
            return Err(ContractError::InvalidState {
                message: "Bounty is not active".to_string(),
            });
        }

        // Vérifie la deadline
        if Utc::now() > bounty.deadline {
            bounty.status = BountyStatus::Expired;
            return Err(ContractError::DeadlineExpired);
        }

        // Vérifie les critères de validation
        self.validate_submission_criteria(&metadata, &bounty.validation_criteria)?;

        // Génère un ID de soumission
        let submission_id = context.compute_hash(&bincode::serialize(&(
            bounty_id,
            &submitter,
            &archive_hash,
            Utc::now().timestamp()
        )).unwrap_or_default())?;

        let submission = ArchiveSubmission {
            submission_id,
            submitter: submitter.clone(),
            archive_hash,
            metadata,
            storage_proof,
            submitted_at: Utc::now(),
            validation_status: if bounty.validation_criteria.auto_validation {
                ValidationStatus::InProgress
            } else {
                ValidationStatus::Pending
            },
            quality_score: 0.0,
        };

        bounty.submissions.push(submission);

        // Si validation automatique, valide immédiatement
        if bounty.validation_criteria.auto_validation {
            self.auto_validate_submission(bounty_id, submission_id, context)?;
        }

        // Émet un event
        context.emit_event(
            "ArchiveSubmitted".to_string(),
            bincode::serialize(&submission_id).unwrap_or_default(),
            vec![
                context.compute_hash(&submitter.as_bytes())?,
                context.compute_hash(&bounty_id.to_le_bytes())?,
            ],
        );

        context.emit_log(format!(
            "Archive submitted for bounty {} by {:?}",
            bounty_id, submitter
        ));

        Ok(submission_id)
    }

    /// Validation automatique d'une soumission
    fn auto_validate_submission(
        &mut self,
        bounty_id: u64,
        submission_id: Hash,
        context: &mut ContractContext,
    ) -> ContractResult<()> {
        let bounty = self.state.bounties.get_mut(&bounty_id)
            .ok_or(ContractError::InvalidParameters {
                message: format!("Bounty {} not found", bounty_id),
            })?;

        let submission = bounty.submissions.iter_mut()
            .find(|s| s.submission_id == submission_id)
            .ok_or(ContractError::InvalidParameters {
                message: "Submission not found".to_string(),
            })?;

        // Calcule un score de qualité basique
        let quality_score = self.calculate_quality_score(&submission.metadata, &bounty.required_quality);
        
        submission.quality_score = quality_score;

        // Valide selon le niveau de qualité requis
        if quality_score >= bounty.required_quality.reward_multiplier() * 0.5 {
            submission.validation_status = ValidationStatus::Validated;
            
            // Si c'est la première soumission valide, marque le bounty comme complété
            if bounty.status == BountyStatus::Active {
                bounty.status = BountyStatus::Completed;
                bounty.winner = Some(submission.submitter.clone());
                
                // Distribue la récompense
                context.transfer_tokens(submission.submitter.clone(), bounty.reward)?;
                
                // Met à jour les statistiques
                self.state.stats.total_bounties_completed += 1;
                self.state.stats.total_rewards_distributed += bounty.reward;
                self.state.stats.total_archived_content_size += submission.metadata.estimated_size;
                
                context.emit_event(
                    "BountyCompleted".to_string(),
                    bincode::serialize(&bounty_id).unwrap_or_default(),
                    vec![context.compute_hash(&submission.submitter.as_bytes())?],
                );
            }
        } else {
            submission.validation_status = ValidationStatus::Rejected(
                "Quality score too low".to_string()
            );
        }

        Ok(())
    }

    /// Calcule un score de qualité pour une soumission
    fn calculate_quality_score(&self, metadata: &ArchiveMetadata, required_quality: &QualityLevel) -> f64 {
        let mut score = 0.5; // Score de base

        // Bonus pour la taille (assume qu'une taille proche de l'estimée est bonne)
        if metadata.estimated_size > 0 {
            score += 0.2;
        }

        // Bonus pour le type de contenu approprié
        if !metadata.content_type.is_empty() && metadata.content_type != "application/octet-stream" {
            score += 0.15;
        }

        // Bonus pour les métadonnées additionnelles
        if !metadata.additional_metadata.is_empty() {
            score += 0.1;
        }

        // Bonus pour le hash original (vérification d'intégrité)
        if metadata.original_hash.is_some() {
            score += 0.05;
        }

        // Applique le multiplicateur de qualité
        score * required_quality.reward_multiplier().min(1.0)
    }

    /// Valide les critères de soumission
    fn validate_submission_criteria(
        &self,
        metadata: &ArchiveMetadata,
        criteria: &ValidationCriteria,
    ) -> ContractResult<()> {
        // Vérifie la taille
        if let Some(min_size) = criteria.min_size {
            if metadata.estimated_size < min_size {
                return Err(ContractError::InvalidParameters {
                    message: format!("Content too small: {} < {}", metadata.estimated_size, min_size),
                });
            }
        }

        if let Some(max_size) = criteria.max_size {
            if metadata.estimated_size > max_size {
                return Err(ContractError::InvalidParameters {
                    message: format!("Content too large: {} > {}", metadata.estimated_size, max_size),
                });
            }
        }

        // Vérifie le type de contenu
        if !criteria.allowed_content_types.contains(&"*/*".to_string()) {
            if !criteria.allowed_content_types.contains(&metadata.content_type) {
                return Err(ContractError::InvalidParameters {
                    message: format!("Content type not allowed: {}", metadata.content_type),
                });
            }
        }

        Ok(())
    }

    /// Obtient les détails d'un bounty
    pub fn get_bounty(&self, bounty_id: u64) -> ContractResult<ArchiveBounty> {
        self.state.bounties.get(&bounty_id)
            .cloned()
            .ok_or(ContractError::InvalidParameters {
                message: format!("Bounty {} not found", bounty_id),
            })
    }

    /// Liste les bounties par statut
    pub fn list_bounties_by_status(
        &self,
        status: BountyStatus,
        limit: u32,
        offset: u32,
    ) -> ContractResult<Vec<ArchiveBounty>> {
        let bounty_ids = self.state.bounties_by_status
            .get(&status)
            .map(|ids| ids.as_slice())
            .unwrap_or(&[]);

        let start = offset as usize;
        let end = (start + limit as usize).min(bounty_ids.len());

        let bounties = bounty_ids[start..end]
            .iter()
            .filter_map(|&id| self.state.bounties.get(&id))
            .cloned()
            .collect();

        Ok(bounties)
    }
}

impl SmartContract for ArchiveBountyContract {
    type State = ArchiveBountyState;
    type CallData = ArchiveBountyCall;
    type ReturnData = ArchiveBountyReturn;

    fn initialize(&mut self, _context: &mut ContractContext) -> ContractResult<()> {
        self.state = ArchiveBountyState::default();
        Ok(())
    }

    fn call(
        &mut self,
        function: &str,
        call_data: Self::CallData,
        context: &mut ContractContext,
    ) -> ContractResult<Self::ReturnData> {
        match call_data {
            ArchiveBountyCall::CreateBounty {
                target_url,
                reward,
                deadline_hours,
                quality_level,
                metadata,
                criteria,
            } => {
                let caller = context.get_caller().clone();
                let bounty_id = self.create_bounty(
                    caller,
                    target_url,
                    reward,
                    deadline_hours,
                    quality_level,
                    metadata,
                    criteria,
                    context,
                )?;
                Ok(ArchiveBountyReturn::BountyCreated { bounty_id })
            }
            
            ArchiveBountyCall::SubmitArchive {
                bounty_id,
                archive_hash,
                metadata,
                storage_proof,
            } => {
                let caller = context.get_caller().clone();
                let submission_id = self.submit_archive(
                    caller,
                    bounty_id,
                    archive_hash,
                    metadata,
                    storage_proof,
                    context,
                )?;
                Ok(ArchiveBountyReturn::SubmissionReceived { submission_id })
            }
            
            ArchiveBountyCall::GetBounty { bounty_id } => {
                let bounty = self.get_bounty(bounty_id)?;
                Ok(ArchiveBountyReturn::BountyDetails(bounty))
            }
            
            ArchiveBountyCall::ListBountiesByStatus { status, limit, offset } => {
                let bounties = self.list_bounties_by_status(status, limit, offset)?;
                Ok(ArchiveBountyReturn::BountyList(bounties))
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
            name: "ArchiveBountyContract".to_string(),
            version: ContractVersion::new(1, 0, 0),
            description: "Smart contract for archive bounties and rewards".to_string(),
            author: "ArchiveChain Team".to_string(),
            license: "MIT".to_string(),
            abi_hash: Hash::zero(), // Sera calculé lors du déploiement
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
    fn test_bounty_creation() {
        let mut contract = ArchiveBountyContract::default();
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

        let mut provider = MockContextProvider::new();
        provider.set_balance(keypair.public_key().clone(), 10000);
        
        let mut context = ContractContext::new(env, Box::new(provider));

        let metadata = ArchiveMetadata {
            content_url: "https://example.com/content".to_string(),
            estimated_size: 1024,
            content_type: "text/html".to_string(),
            original_hash: None,
            additional_metadata: HashMap::new(),
        };

        let bounty_id = contract.create_bounty(
            keypair.public_key().clone(),
            "https://example.com".to_string(),
            1000,
            24,
            QualityLevel::Standard,
            metadata,
            ValidationCriteria::default(),
            &mut context,
        ).unwrap();

        assert_eq!(bounty_id, 1);
        assert_eq!(contract.state.bounties.len(), 1);
        
        let bounty = contract.get_bounty(bounty_id).unwrap();
        assert_eq!(bounty.reward, 1000);
        assert_eq!(bounty.status, BountyStatus::Active);
    }

    #[test]
    fn test_quality_level_multipliers() {
        assert_eq!(QualityLevel::Basic.reward_multiplier(), 1.0);
        assert_eq!(QualityLevel::Standard.reward_multiplier(), 1.5);
        assert_eq!(QualityLevel::High.reward_multiplier(), 2.0);
        assert_eq!(QualityLevel::Premium.reward_multiplier(), 3.0);
    }

    #[test]
    fn test_validation_criteria() {
        let contract = ArchiveBountyContract::default();
        
        let metadata = ArchiveMetadata {
            content_url: "test".to_string(),
            estimated_size: 500,
            content_type: "text/plain".to_string(),
            original_hash: None,
            additional_metadata: HashMap::new(),
        };

        let criteria = ValidationCriteria {
            min_size: Some(100),
            max_size: Some(1000),
            allowed_content_types: vec!["text/plain".to_string()],
            min_compression_ratio: None,
            min_storage_duration: 24,
            auto_validation: true,
        };

        assert!(contract.validate_submission_criteria(&metadata, &criteria).is_ok());

        let invalid_metadata = ArchiveMetadata {
            content_url: "test".to_string(),
            estimated_size: 50, // Trop petit
            content_type: "text/plain".to_string(),
            original_hash: None,
            additional_metadata: HashMap::new(),
        };

        assert!(contract.validate_submission_criteria(&invalid_metadata, &criteria).is_err());
    }
}