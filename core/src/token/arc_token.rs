//! Token ARC principal pour ArchiveChain
//!
//! Implémentation du token natif avec fonctionnalités ERC-20-like adaptées
//! aux besoins spécifiques d'ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use crate::crypto::{Hash, PublicKey, Signature};
use crate::error::Result;
use super::{TokenOperationError, TokenOperationResult, TokenEvent, TokenEventType, TOTAL_SUPPLY};

/// Token ARC principal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ARCToken {
    /// Supply totale de tokens
    pub total_supply: u64,
    /// Supply en circulation (non verrouillée)
    pub circulating_supply: u64,
    /// Tokens brûlés de manière permanente
    pub burned_tokens: u64,
    /// Tokens verrouillés (staking, vesting, etc.)
    pub locked_tokens: u64,
    /// Soldes par adresse
    pub balances: HashMap<PublicKey, u64>,
    /// Allocations (allowances) pour transferts délégués
    pub allowances: HashMap<(PublicKey, PublicKey), u64>,
    /// Métadonnées du token
    pub metadata: TokenMetadata,
    /// Historique des événements
    pub events: Vec<TokenEvent>,
    /// Timestamp de création
    pub created_at: DateTime<Utc>,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

/// Métadonnées du token ARC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenMetadata {
    /// Nom du token
    pub name: String,
    /// Symbole du token
    pub symbol: String,
    /// Nombre de décimales
    pub decimals: u8,
    /// Description du token
    pub description: String,
    /// Version du contrat
    pub version: String,
    /// Site web officiel
    pub website: Option<String>,
    /// Logo/icône
    pub logo: Option<String>,
}

/// Résultat spécifique aux opérations du token ARC
pub type TokenResult<T> = TokenOperationResult<T>;

/// Erreurs spécifiques au token ARC
pub type TokenError = TokenOperationError;

impl Default for TokenMetadata {
    fn default() -> Self {
        Self {
            name: "ArchiveChain Token".to_string(),
            symbol: "ARC".to_string(),
            decimals: 18,
            description: "Token natif de la blockchain ArchiveChain pour l'archivage décentralisé".to_string(),
            version: "1.0.0".to_string(),
            website: Some("https://archivechain.org".to_string()),
            logo: None,
        }
    }
}

impl ARCToken {
    /// Crée un nouveau token ARC avec la supply initiale
    pub fn new() -> Self {
        Self {
            total_supply: TOTAL_SUPPLY,
            circulating_supply: 0,
            burned_tokens: 0,
            locked_tokens: 0,
            balances: HashMap::new(),
            allowances: HashMap::new(),
            metadata: TokenMetadata::default(),
            events: Vec::new(),
            created_at: Utc::now(),
            last_updated: Utc::now(),
        }
    }

    /// Obtient le solde d'une adresse
    pub fn balance_of(&self, address: &PublicKey) -> u64 {
        self.balances.get(address).copied().unwrap_or(0)
    }

    /// Obtient l'allocation entre deux adresses
    pub fn allowance(&self, owner: &PublicKey, spender: &PublicKey) -> u64 {
        self.allowances.get(&(owner.clone(), spender.clone())).copied().unwrap_or(0)
    }

    /// Mint des tokens vers une adresse (opération système uniquement)
    pub fn mint(&mut self, to: &PublicKey, amount: u64, tx_hash: Hash) -> TokenResult<()> {
        if amount == 0 {
            return Err(TokenError::InvalidAmount { amount });
        }

        // Vérification de la supply maximale
        if self.circulating_supply + self.locked_tokens + amount > self.total_supply {
            return Err(TokenError::Internal {
                message: "Dépassement de la supply maximale".to_string(),
            });
        }

        // Ajouter au solde
        let current_balance = self.balance_of(to);
        self.balances.insert(to.clone(), current_balance + amount);
        
        // Mettre à jour la supply en circulation
        self.circulating_supply += amount;
        self.last_updated = Utc::now();

        // Émettre un événement
        self.emit_event(TokenEvent {
            transaction_hash: tx_hash,
            event_type: TokenEventType::Transfer {
                from: super::system_address(),
                to: to.clone(),
                amount,
            },
            timestamp: Utc::now(),
            data: HashMap::new(),
        });

        Ok(())
    }

    /// Transfère des tokens entre deux adresses
    pub fn transfer(&mut self, from: &PublicKey, to: &PublicKey, amount: u64, tx_hash: Hash) -> TokenResult<()> {
        self.internal_transfer(from, to, amount, tx_hash)
    }

    /// Approuve une allocation pour un spender
    pub fn approve(&mut self, owner: &PublicKey, spender: &PublicKey, amount: u64, tx_hash: Hash) -> TokenResult<()> {
        self.allowances.insert((owner.clone(), spender.clone()), amount);
        self.last_updated = Utc::now();

        // Émettre un événement d'approbation
        let mut data = HashMap::new();
        data.insert("owner".to_string(), serde_json::to_value(owner).unwrap());
        data.insert("spender".to_string(), serde_json::to_value(spender).unwrap());
        data.insert("amount".to_string(), serde_json::to_value(amount).unwrap());

        self.emit_event(TokenEvent {
            transaction_hash: tx_hash,
            event_type: TokenEventType::Transfer {
                from: owner.clone(),
                to: spender.clone(),
                amount: 0, // Pas de transfert réel, juste approbation
            },
            timestamp: Utc::now(),
            data,
        });

        Ok(())
    }

    /// Transfère depuis une allocation (transferFrom)
    pub fn transfer_from(&mut self, spender: &PublicKey, from: &PublicKey, to: &PublicKey, amount: u64, tx_hash: Hash) -> TokenResult<()> {
        let current_allowance = self.allowance(from, spender);
        
        if current_allowance < amount {
            return Err(TokenError::InsufficientBalance {
                required: amount,
                available: current_allowance,
            });
        }

        // Effectuer le transfert
        self.internal_transfer(from, to, amount, tx_hash)?;

        // Réduire l'allocation
        self.allowances.insert((from.clone(), spender.clone()), current_allowance - amount);

        Ok(())
    }

    /// Brûle des tokens de manière permanente
    pub fn burn(&mut self, from: &PublicKey, amount: u64, tx_hash: Hash) -> TokenResult<()> {
        let current_balance = self.balance_of(from);
        
        if current_balance < amount {
            return Err(TokenError::InsufficientBalance {
                required: amount,
                available: current_balance,
            });
        }

        // Retirer du solde
        self.balances.insert(from.clone(), current_balance - amount);
        
        // Ajouter aux tokens brûlés
        self.burned_tokens += amount;
        self.circulating_supply -= amount;
        self.last_updated = Utc::now();

        // Émettre un événement de burn
        self.emit_event(TokenEvent {
            transaction_hash: tx_hash,
            event_type: TokenEventType::Burn {
                from: from.clone(),
                amount,
            },
            timestamp: Utc::now(),
            data: HashMap::new(),
        });

        Ok(())
    }

    /// Verrouille des tokens (pour staking, vesting, etc.)
    pub fn lock_tokens(&mut self, from: &PublicKey, amount: u64, lock_type: &str, tx_hash: Hash) -> TokenResult<()> {
        let current_balance = self.balance_of(from);
        
        if current_balance < amount {
            return Err(TokenError::InsufficientBalance {
                required: amount,
                available: current_balance,
            });
        }

        // Retirer de la circulation et ajouter aux tokens verrouillés
        self.balances.insert(from.clone(), current_balance - amount);
        self.locked_tokens += amount;
        self.circulating_supply -= amount;
        self.last_updated = Utc::now();

        // Émettre un événement de verrouillage
        let mut data = HashMap::new();
        data.insert("lock_type".to_string(), serde_json::Value::String(lock_type.to_string()));

        self.emit_event(TokenEvent {
            transaction_hash: tx_hash,
            event_type: TokenEventType::Staked {
                staker: from.clone(),
                amount,
                stake_type: lock_type.to_string(),
            },
            timestamp: Utc::now(),
            data,
        });

        Ok(())
    }

    /// Déverrouille des tokens
    pub fn unlock_tokens(&mut self, to: &PublicKey, amount: u64, lock_type: &str, tx_hash: Hash) -> TokenResult<()> {
        if self.locked_tokens < amount {
            return Err(TokenError::Internal {
                message: "Pas assez de tokens verrouillés".to_string(),
            });
        }

        // Ajouter au solde et retirer des tokens verrouillés
        let current_balance = self.balance_of(to);
        self.balances.insert(to.clone(), current_balance + amount);
        self.locked_tokens -= amount;
        self.circulating_supply += amount;
        self.last_updated = Utc::now();

        // Émettre un événement de déverrouillage
        let mut data = HashMap::new();
        data.insert("lock_type".to_string(), serde_json::Value::String(lock_type.to_string()));

        self.emit_event(TokenEvent {
            transaction_hash: tx_hash,
            event_type: TokenEventType::Unstaked {
                staker: to.clone(),
                amount,
                stake_type: lock_type.to_string(),
            },
            timestamp: Utc::now(),
            data,
        });

        Ok(())
    }

    /// Obtient les statistiques globales du token
    pub fn get_statistics(&self) -> TokenStatistics {
        TokenStatistics {
            total_supply: self.total_supply,
            circulating_supply: self.circulating_supply,
            burned_tokens: self.burned_tokens,
            locked_tokens: self.locked_tokens,
            holder_count: self.balances.len(),
            total_transactions: self.events.len(),
            last_updated: self.last_updated,
        }
    }

    /// Transfère des tokens (implémentation interne)
    fn internal_transfer(&mut self, from: &PublicKey, to: &PublicKey, amount: u64, tx_hash: Hash) -> TokenResult<()> {
        if amount == 0 {
            return Err(TokenError::InvalidAmount { amount });
        }

        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(TokenError::InsufficientBalance {
                required: amount,
                available: from_balance,
            });
        }

        // Effectuer le transfert
        self.balances.insert(from.clone(), from_balance - amount);
        let to_balance = self.balance_of(to);
        self.balances.insert(to.clone(), to_balance + amount);
        
        self.last_updated = Utc::now();

        // Émettre un événement de transfert
        self.emit_event(TokenEvent {
            transaction_hash: tx_hash,
            event_type: TokenEventType::Transfer {
                from: from.clone(),
                to: to.clone(),
                amount,
            },
            timestamp: Utc::now(),
            data: HashMap::new(),
        });

        Ok(())
    }

    /// Émet un événement
    fn emit_event(&mut self, event: TokenEvent) {
        self.events.push(event);
    }

    /// Obtient les événements pour une adresse
    pub fn get_events_for_address(&self, address: &PublicKey) -> Vec<&TokenEvent> {
        self.events.iter().filter(|event| {
            match &event.event_type {
                TokenEventType::Transfer { from, to, .. } => from == address || to == address,
                TokenEventType::Burn { from, .. } => from == address,
                TokenEventType::RewardDistributed { to, .. } => to == address,
                TokenEventType::Staked { staker, .. } => staker == address,
                TokenEventType::Unstaked { staker, .. } => staker == address,
                TokenEventType::ProposalCreated { proposer, .. } => proposer == address,
                TokenEventType::ProposalVoted { voter, .. } => voter == address,
            }
        }).collect()
    }

    /// Valide l'intégrité du token
    pub fn validate_integrity(&self) -> TokenResult<()> {
        // Vérifier que la somme des balances + tokens brûlés + tokens verrouillés <= supply totale
        let total_balances: u64 = self.balances.values().sum();
        let total_accounted = total_balances + self.burned_tokens + self.locked_tokens;
        
        if total_accounted > self.total_supply {
            return Err(TokenError::Internal {
                message: format!("Intégrité compromise: {} > {}", total_accounted, self.total_supply),
            });
        }

        // Vérifier que la supply en circulation correspond
        if self.circulating_supply != total_balances {
            return Err(TokenError::Internal {
                message: format!("Supply en circulation incohérente: {} != {}", 
                               self.circulating_supply, total_balances),
            });
        }

        Ok(())
    }
}

/// Statistiques du token ARC
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenStatistics {
    /// Supply totale
    pub total_supply: u64,
    /// Supply en circulation
    pub circulating_supply: u64,
    /// Tokens brûlés
    pub burned_tokens: u64,
    /// Tokens verrouillés
    pub locked_tokens: u64,
    /// Nombre de détenteurs
    pub holder_count: usize,
    /// Nombre total de transactions
    pub total_transactions: usize,
    /// Dernière mise à jour
    pub last_updated: DateTime<Utc>,
}

impl Default for ARCToken {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::generate_keypair;

    #[test]
    fn test_arc_token_creation() {
        let token = ARCToken::new();
        assert_eq!(token.total_supply, TOTAL_SUPPLY);
        assert_eq!(token.circulating_supply, 0);
        assert_eq!(token.burned_tokens, 0);
        assert_eq!(token.locked_tokens, 0);
    }

    #[test]
    fn test_mint_tokens() {
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let address = keypair.public_key();
        let tx_hash = Hash::zero();

        let result = token.mint(address, 1000, tx_hash);
        assert!(result.is_ok());
        assert_eq!(token.balance_of(address), 1000);
        assert_eq!(token.circulating_supply, 1000);
    }

    #[test]
    fn test_transfer_tokens() {
        let mut token = ARCToken::new();
        let keypair1 = generate_keypair().unwrap();
        let keypair2 = generate_keypair().unwrap();
        let addr1 = keypair1.public_key();
        let addr2 = keypair2.public_key();
        let tx_hash = Hash::zero();

        // Mint tokens to addr1
        token.mint(addr1, 1000, tx_hash).unwrap();

        // Transfer from addr1 to addr2
        let result = token.transfer(addr1, addr2, 300, tx_hash);
        assert!(result.is_ok());
        assert_eq!(token.balance_of(addr1), 700);
        assert_eq!(token.balance_of(addr2), 300);
    }

    #[test]
    fn test_burn_tokens() {
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let address = keypair.public_key();
        let tx_hash = Hash::zero();

        // Mint tokens
        token.mint(address, 1000, tx_hash).unwrap();

        // Burn tokens
        let result = token.burn(address, 200, tx_hash);
        assert!(result.is_ok());
        assert_eq!(token.balance_of(address), 800);
        assert_eq!(token.burned_tokens, 200);
        assert_eq!(token.circulating_supply, 800);
    }

    #[test]
    fn test_lock_unlock_tokens() {
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let address = keypair.public_key();
        let tx_hash = Hash::zero();

        // Mint tokens
        token.mint(address, 1000, tx_hash).unwrap();

        // Lock tokens
        let result = token.lock_tokens(address, 300, "staking", tx_hash);
        assert!(result.is_ok());
        assert_eq!(token.balance_of(address), 700);
        assert_eq!(token.locked_tokens, 300);
        assert_eq!(token.circulating_supply, 700);

        // Unlock tokens
        let result = token.unlock_tokens(address, 100, "staking", tx_hash);
        assert!(result.is_ok());
        assert_eq!(token.balance_of(address), 800);
        assert_eq!(token.locked_tokens, 200);
        assert_eq!(token.circulating_supply, 800);
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let mut token = ARCToken::new();
        let keypair1 = generate_keypair().unwrap();
        let keypair2 = generate_keypair().unwrap();
        let keypair3 = generate_keypair().unwrap();
        let owner = keypair1.public_key();
        let spender = keypair2.public_key();
        let recipient = keypair3.public_key();
        let tx_hash = Hash::zero();

        // Mint tokens to owner
        token.mint(owner, 1000, tx_hash).unwrap();

        // Approve spender
        token.approve(owner, spender, 300, tx_hash).unwrap();
        assert_eq!(token.allowance(owner, spender), 300);

        // Transfer from owner to recipient via spender
        let result = token.transfer_from(spender, owner, recipient, 200, tx_hash);
        assert!(result.is_ok());
        assert_eq!(token.balance_of(owner), 800);
        assert_eq!(token.balance_of(recipient), 200);
        assert_eq!(token.allowance(owner, spender), 100);
    }

    #[test]
    fn test_insufficient_balance() {
        let mut token = ARCToken::new();
        let keypair1 = generate_keypair().unwrap();
        let keypair2 = generate_keypair().unwrap();
        let addr1 = keypair1.public_key();
        let addr2 = keypair2.public_key();
        let tx_hash = Hash::zero();

        // Try to transfer without sufficient balance
        let result = token.transfer(addr1, addr2, 100, tx_hash);
        assert!(result.is_err());
        
        if let Err(TokenError::InsufficientBalance { required, available }) = result {
            assert_eq!(required, 100);
            assert_eq!(available, 0);
        } else {
            panic!("Expected InsufficientBalance error");
        }
    }

    #[test]
    fn test_integrity_validation() {
        let mut token = ARCToken::new();
        let keypair = generate_keypair().unwrap();
        let address = keypair.public_key();
        let tx_hash = Hash::zero();

        // Mint some tokens
        token.mint(address, 1000, tx_hash).unwrap();
        
        // Burn some tokens
        token.burn(address, 200, tx_hash).unwrap();
        
        // Lock some tokens
        token.lock_tokens(address, 300, "staking", tx_hash).unwrap();

        // Should pass integrity check
        assert!(token.validate_integrity().is_ok());
    }
}