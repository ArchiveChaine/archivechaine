//! ArchiveChain Core Library
//!
//! This is the core library for ArchiveChain, a decentralized web archiving platform
//! that preserves digital content on a distributed blockchain network.
//!
//! # Features
//!
//! - **Blockchain Core**: Proof-of-Archive consensus and distributed storage
//! - **Economic System**: Native ARC token with advanced tokenomics
//! - **Cryptographic Security**: EdDSA signatures, Merkle trees, and content hashing
//! - **Multiple APIs**: REST, GraphQL, WebSocket, gRPC, and P2P protocols
//! - **Authentication**: JWT-based auth with role-based access control
//! - **Real-time Updates**: WebSocket subscriptions and P2P gossip
//! - **High Performance**: Async/await throughout with optimized data structures
//! - **Distributed Nodes**: Full Archive, Light Storage, Relay, and Gateway nodes
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use archivechain_core::{Blockchain, BlockchainConfig, api::ApiServer, nodes::NodeManager};
//! use std::sync::Arc;
//! 
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize blockchain
//!     let blockchain = Arc::new(Blockchain::new(BlockchainConfig::default())?);
//!     
//!     // Initialize node manager
//!     let node_manager = NodeManager::new(nodes::NodeConfig::default()).await?;
//!     
//!     // Start API server with all endpoints
//!     let server = ApiServer::new_with_blockchain(blockchain).await?;
//!     server.start().await?;
//!     
//!     Ok(())
//! }
//! ```
//!
//! # Architecture
//!
//! ArchiveChain is organized into several key modules:
//!
//! - [`blockchain`] - Core blockchain functionality and consensus
//! - [`crypto`] - Cryptographic primitives and utilities  
//! - [`state`] - State management and Merkle tree structures
//! - [`transaction`] - Transaction types and validation
//! - [`block`] - Block structures and archive metadata
//! - [`token`] - Native ARC token and economic system
//! - [`api`] - Complete API layer with multiple protocols
//! - [`nodes`] - Distributed node management and orchestration
//! - [`storage`] - Distributed storage system (re-exported from consensus module)
//! - [`consensus`] - Proof-of-Archive consensus implementation (re-exported from consensus module)
//!
//! # Node Types
//!
//! ArchiveChain supports multiple specialized node types:
//!
//! ## Full Archive Nodes
//! - Store complete archives (>10TB capacity)
//! - High redundancy (5-15 replicas)
//! - Full consensus participation
//! - Primary network backbone
//!
//! ## Light Storage Nodes
//! - Specialized storage (1-10TB capacity)
//! - Content-type or geographic specialization
//! - Selective consensus participation
//! - Efficient resource utilization
//!
//! ## Relay Nodes
//! - Network communication facilitation
//! - High bandwidth capacity (1GB/s+)
//! - Message routing and discovery
//! - Minimal storage requirements
//!
//! ## Gateway Nodes
//! - Public API interfaces
//! - Load balancing and caching
//! - Rate limiting and security
//! - Multiple protocol support
//!
//! # Economic System
//!
//! ArchiveChain features a comprehensive economic model with the native ARC token:
//!
//! ## Token Distribution
//! - **40%** (40B ARC): Archival rewards distributed over 10 years
//! - **25%** (25B ARC): Team allocation with 4-year vesting
//! - **20%** (20B ARC): Community treasury for governance
//! - **15%** (15B ARC): Public and private sales
//!
//! ## Reward System
//! - **Archival**: 100-500 ARC for content archiving (quality + rarity based)
//! - **Storage**: 10-50 ARC/month for continuous storage
//! - **Bandwidth**: 1-5 ARC/GB for content delivery
//! - **Discovery**: 25-100 ARC for content discovery
//!
//! ## Deflationary Mechanisms
//! - **10%** of transaction fees burned automatically
//! - Quality staking with slashing for poor performance
//! - Long-term bonus multipliers (up to 2x for 2+ years)
//!
//! ## Governance & Staking
//! - Minimum 1M ARC for governance participation
//! - Minimum 10M ARC for validator status
//! - Delegation and voting power based on stake duration
//!
//! # Examples
//!
//! ## Working with Nodes
//!
//! ```rust,no_run
//! use archivechain_core::nodes::{NodeManager, NodeConfig, NodeType};
//! 
//! async fn setup_nodes() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize node manager
//!     let node_manager = NodeManager::new(NodeConfig::default()).await?;
//!     
//!     // Create a full archive node
//!     let archive_node_id = node_manager.create_node(
//!         NodeType::FullArchive {
//!             storage_capacity: 20_000_000_000_000, // 20TB
//!             replication_factor: 10,
//!         },
//!         None
//!     ).await?;
//!     
//!     // Start the node
//!     node_manager.start_node(&archive_node_id).await?;
//!     
//!     // Monitor health
//!     let health_results = node_manager.health_check_all_nodes().await?;
//!     println!("Node health: {:?}", health_results);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Working with the Economic System
//!
//! ```rust,no_run
//! use archivechain_core::token::{EconomicModel, EconomicConfig};
//! 
//! async fn setup_economics() -> Result<(), Box<dyn std::error::Error>> {
//!     // Initialize economic model
//!     let mut economics = EconomicModel::new(EconomicConfig::default());
//!     
//!     // Update all metrics
//!     economics.update_all_metrics()?;
//!     
//!     // Generate economic report
//!     let report = economics.generate_economic_report();
//!     println!("Economic Health Index: {:.2}", report.summary.economic_health_index);
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Creating and Managing Archives
//!
//! ```rust,no_run
//! use archivechain_core::api::{ApiServer, types::*};
//! 
//! async fn create_archive() -> Result<(), Box<dyn std::error::Error>> {
//!     let server = ApiServer::new().await?;
//!     
//!     // Archive via REST API
//!     let archive_request = CreateArchiveRequest {
//!         url: "https://example.com".to_string(),
//!         metadata: ArchiveMetadataDto {
//!             title: Some("Example Page".to_string()),
//!             description: Some("A test page".to_string()),
//!             // ...
//!         },
//!         options: None,
//!     };
//!     
//!     // Process through blockchain...
//!     Ok(())
//! }
//! ```
//!
//! ## Using Different APIs
//!
//! ```rust,no_run
//! use archivechain_core::api::{
//!     rest::ArchiveController,
//!     graphql::ArchiveServiceImpl,
//!     websocket::EventManager,
//!     grpc::ArchiveServiceClient,
//!     p2p::P2PManager,
//! };
//! 
//! // Each API can be used independently or together
//! ```

#![deny(missing_docs)]
#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(clippy::module_name_repetitions)]

// Core blockchain modules
pub mod blockchain;
pub mod crypto;
pub mod state;
pub mod transaction;
pub mod block;

// Storage and consensus  
pub mod storage;
pub mod consensus;

// Economic system
pub mod token;

// Node management and orchestration
pub mod nodes;

// API layer - comprehensive multi-protocol support
pub mod api;

// Error handling
pub mod error;

// Re-exports for convenience
pub use blockchain::{Blockchain, BlockchainConfig, BlockchainStats};
pub use error::{ArchiveChainError, Result, CoreError};

// Node system re-exports
pub use nodes::{
    NodeManager, NodeConfig, NodeType, Node,
    FullArchiveNode, LightStorageNode, RelayNode, GatewayNode,
    NodeRegistry, HealthMonitor, NodeHealth, HealthStatus,
    NodeManagerStats
};

// Economic system re-exports
pub use token::{
    EconomicModel, EconomicMetrics,
    ARCToken, TokenConfig, GlobalTokenMetrics,
    RewardSystem, StakingSystem, Treasury, DeflationaryMechanisms,
    TokenDistribution, TokenOperationResult, TokenOperationError,
    TOTAL_SUPPLY, ARCHIVAL_REWARDS_ALLOCATION, TEAM_ALLOCATION,
    COMMUNITY_RESERVE, PUBLIC_SALE
};

// Economic config and report from specific modules
pub use token::economics::{EconomicConfig, EconomicReport};

// Storage and consensus re-exports
pub use consensus::{
    NodeId, ProofOfArchive, ConsensusConfig, ConsensusScore
};

// Common type aliases
pub use crypto::{Hash, PublicKey};
pub use state::StateRoot;
pub use transaction::Transaction;
pub use block::{Block, ArchiveMetadata};

/// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Git commit hash (if available)
pub const GIT_HASH: Option<&str> = option_env!("GIT_HASH");

/// Build timestamp
pub const BUILD_TIMESTAMP: &str = option_env!("BUILD_TIMESTAMP").unwrap_or("unknown");

/// Version info structure
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VersionInfo {
    /// Semantic version
    pub version: String,
    /// Git commit hash
    pub git_hash: Option<String>,
    /// Build timestamp
    pub build_timestamp: String,
    /// Rust compiler version used
    pub rustc_version: String,
    /// Target triple
    pub target: String,
}

impl VersionInfo {
    /// Get current version information
    pub fn current() -> Self {
        Self {
            version: VERSION.to_string(),
            git_hash: GIT_HASH.map(String::from),
            build_timestamp: BUILD_TIMESTAMP.to_string(),
            rustc_version: option_env!("RUSTC_VERSION").unwrap_or("unknown").to_string(),
            target: option_env!("TARGET").unwrap_or("unknown").to_string(),
        }
    }
}

impl std::fmt::Display for VersionInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ArchiveChain v{}", self.version)?;
        if let Some(ref hash) = self.git_hash {
            write!(f, " ({})", &hash[..8])?;
        }
        Ok(())
    }
}

/// Feature flags for conditional compilation
pub mod features {
    /// Whether WebSocket support is enabled
    pub const WEBSOCKET: bool = cfg!(feature = "websocket");
    
    /// Whether gRPC support is enabled  
    pub const GRPC: bool = cfg!(feature = "grpc");
    
    /// Whether P2P networking is enabled
    pub const P2P: bool = cfg!(feature = "p2p");
    
    /// Whether GraphQL support is enabled
    pub const GRAPHQL: bool = cfg!(feature = "graphql");
    
    /// Whether TLS support is enabled
    pub const TLS: bool = cfg!(feature = "tls");
    
    /// Whether metrics collection is enabled
    pub const METRICS: bool = cfg!(feature = "metrics");
    
    /// Whether advanced economic features are enabled
    pub const ADVANCED_ECONOMICS: bool = cfg!(feature = "advanced-economics");
    
    /// Whether economic simulations are enabled
    pub const ECONOMIC_SIMULATION: bool = cfg!(feature = "economic-simulation");
    
    /// Whether distributed nodes are enabled
    pub const DISTRIBUTED_NODES: bool = cfg!(feature = "distributed-nodes");
}

/// Prelude module for common imports
pub mod prelude {
    //! Common types and traits for convenient importing
    //!
    //! ```rust
    //! use archivechain_core::prelude::*;
    //! ```

    // Core types
    pub use crate::{
        Blockchain, BlockchainConfig, BlockchainStats,
        Hash, StateRoot, Transaction, Block,
        ArchiveChainError, Result, VersionInfo,
    };

    // Node system types
    pub use crate::{
        NodeManager, NodeConfig, NodeType, Node,
        FullArchiveNode, LightStorageNode, RelayNode, GatewayNode,
        NodeRegistry, HealthMonitor, NodeHealth, HealthStatus,
        NodeManagerStats, NodeId
    };

    // Storage and consensus types
    pub use crate::{
        ProofOfArchive, ConsensusConfig, ConsensusScore
    };

    // Economic system types
    pub use crate::{
        EconomicModel, EconomicConfig, EconomicMetrics, EconomicReport,
        ARCToken, TokenConfig, GlobalTokenMetrics,
        RewardSystem, StakingSystem, Treasury, DeflationaryMechanisms,
        TokenDistribution, TokenOperationResult, TokenOperationError,
        TOTAL_SUPPLY, ARCHIVAL_REWARDS_ALLOCATION, TEAM_ALLOCATION, 
        COMMUNITY_RESERVE, PUBLIC_SALE
    };

    // API types and traits
    pub use crate::api::{
        ApiConfig, ApiError, ApiResult,
        types::*,
        server::{ApiServer, ServerState},
        auth::{AuthService, AuthConfig, ApiScope},
    };

    // Common traits
    pub use crate::crypto::{Hashable, Signable};
    pub use crate::state::MerkleProof;
    pub use crate::transaction::{Validatable, TransactionType};

    // Async trait for convenience
    pub use async_trait::async_trait;

    // Common external types
    pub use serde::{Serialize, Deserialize};
    pub use uuid::Uuid;
    pub use chrono::{DateTime, Utc};
}

/// Utility functions and helpers
pub mod utils {
    //! Utility functions for common operations

    use crate::Hash;
    
    /// Generate a random hash for testing purposes
    pub fn random_hash() -> Hash {
        use rand::RngCore;
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        Hash::from_bytes(&bytes).unwrap()
    }
    
    /// Generate a random ID string
    pub fn random_id() -> String {
        uuid::Uuid::new_v4().simple().to_string()
    }
    
    /// Format bytes as human-readable size
    pub fn format_bytes(bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB", "PB"];
        
        if bytes == 0 {
            return "0 B".to_string();
        }
        
        let base = 1024u64;
        let exp = (bytes as f64).log(base as f64).floor() as usize;
        let exp = exp.min(UNITS.len() - 1);
        
        let size = bytes as f64 / base.pow(exp as u32) as f64;
        
        if exp == 0 {
            format!("{} {}", bytes, UNITS[exp])
        } else {
            format!("{:.1} {}", size, UNITS[exp])
        }
    }
    
    /// Convert duration to human-readable string
    pub fn format_duration(duration: std::time::Duration) -> String {
        let secs = duration.as_secs();
        
        if secs < 60 {
            format!("{}s", secs)
        } else if secs < 3600 {
            format!("{}m {}s", secs / 60, secs % 60)
        } else if secs < 86400 {
            format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
        } else {
            format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
        }
    }

    /// Format token amount with proper decimals
    pub fn format_token_amount(amount: u64) -> String {
        let decimals = 18; // ARC token has 18 decimals
        let divisor = 10u64.pow(decimals);
        let whole = amount / divisor;
        let fractional = amount % divisor;
        
        if fractional == 0 {
            format!("{} ARC", whole)
        } else {
            let fractional_formatted = format!("{:018}", fractional);
            let fractional_str = fractional_formatted.trim_end_matches('0');
            format!("{}.{} ARC", whole, fractional_str)
        }
    }

    /// Parse token amount from string
    pub fn parse_token_amount(amount_str: &str) -> Result<u64, crate::ArchiveChainError> {
        let amount_str = amount_str.trim().trim_end_matches(" ARC");
        
        if let Some(dot_pos) = amount_str.find('.') {
            let whole_part: u64 = amount_str[..dot_pos].parse()
                .map_err(|_| crate::ArchiveChainError::InvalidInput("Invalid token amount".into()))?;
            let fractional_part = &amount_str[dot_pos + 1..];
            
            if fractional_part.len() > 18 {
                return Err(crate::ArchiveChainError::InvalidInput("Too many decimal places".into()));
            }
            
            let fractional_padded = format!("{:0<18}", fractional_part);
            let fractional: u64 = fractional_padded.parse()
                .map_err(|_| crate::ArchiveChainError::InvalidInput("Invalid fractional part".into()))?;
            
            Ok(whole_part * 10u64.pow(18) + fractional)
        } else {
            let whole: u64 = amount_str.parse()
                .map_err(|_| crate::ArchiveChainError::InvalidInput("Invalid token amount".into()))?;
            Ok(whole * 10u64.pow(18))
        }
    }

    /// Calculate percentage change between two values
    pub fn calculate_percentage_change(old_value: f64, new_value: f64) -> f64 {
        if old_value == 0.0 {
            if new_value == 0.0 { 0.0 } else { 100.0 }
        } else {
            ((new_value - old_value) / old_value) * 100.0
        }
    }

    /// Format percentage with appropriate precision
    pub fn format_percentage(percentage: f64) -> String {
        format!("{:.2}%", percentage)
    }

    /// Format node type as human-readable string
    pub fn format_node_type(node_type: &crate::NodeType) -> String {
        match node_type {
            crate::NodeType::FullArchive { storage_capacity, replication_factor } => {
                format!("Full Archive ({}, x{})", format_bytes(*storage_capacity), replication_factor)
            },
            crate::NodeType::LightStorage { storage_capacity, specialization } => {
                format!("Light Storage ({}, {:?})", format_bytes(*storage_capacity), specialization)
            },
            crate::NodeType::Relay { bandwidth_capacity, max_connections } => {
                format!("Relay ({}/s, {} conn)", format_bytes(*bandwidth_capacity), max_connections)
            },
            crate::NodeType::Gateway { exposed_apis, rate_limit } => {
                format!("Gateway ({} APIs, {} req/s)", exposed_apis.len(), rate_limit)
            },
        }
    }
}

/// Constants used throughout the library
pub mod constants {
    //! Constants and configuration values

    /// Maximum size for archive metadata in bytes
    pub const MAX_METADATA_SIZE: usize = 1024 * 1024; // 1MB
    
    /// Maximum number of tags per archive
    pub const MAX_TAGS_PER_ARCHIVE: usize = 50;
    
    /// Maximum length of a tag
    pub const MAX_TAG_LENGTH: usize = 100;
    
    /// Default block size in bytes
    pub const DEFAULT_BLOCK_SIZE: usize = 1024 * 1024; // 1MB
    
    /// Maximum number of transactions per block
    pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 1000;
    
    /// Default network timeout in seconds
    pub const DEFAULT_NETWORK_TIMEOUT: u64 = 30;
    
    /// Default JWT expiration time in hours
    pub const DEFAULT_JWT_EXPIRATION: i64 = 24;
    
    /// Minimum required peer connections
    pub const MIN_PEER_CONNECTIONS: usize = 3;
    
    /// Maximum peer connections
    pub const MAX_PEER_CONNECTIONS: usize = 50;
    
    /// Archive URL validation regex pattern
    pub const URL_PATTERN: &str = r"^https?://[^\s/$.?#].[^\s]*$";
    
    /// Supported archive content types
    pub const SUPPORTED_CONTENT_TYPES: &[&str] = &[
        "text/html",
        "text/plain", 
        "application/pdf",
        "application/json",
        "text/css",
        "application/javascript",
        "image/png",
        "image/jpeg",
        "image/gif",
        "image/webp",
    ];

    /// Node constants
    pub mod nodes {
        /// Minimum storage capacity for Full Archive Nodes (10TB)
        pub const MIN_FULL_ARCHIVE_CAPACITY: u64 = 10_000_000_000_000;
        
        /// Minimum storage capacity for Light Storage Nodes (1TB)
        pub const MIN_LIGHT_STORAGE_CAPACITY: u64 = 1_000_000_000_000;
        
        /// Maximum storage capacity for Light Storage Nodes (10TB)
        pub const MAX_LIGHT_STORAGE_CAPACITY: u64 = 10_000_000_000_000;
        
        /// Minimum bandwidth for Relay Nodes (100MB/s)
        pub const MIN_RELAY_BANDWIDTH: u64 = 100_000_000;
        
        /// Default health check interval (30 seconds)
        pub const DEFAULT_HEALTH_CHECK_INTERVAL: u64 = 30;
        
        /// Default node timeout (5 minutes)
        pub const DEFAULT_NODE_TIMEOUT: u64 = 300;
        
        /// Maximum nodes per cluster
        pub const MAX_NODES_PER_CLUSTER: u32 = 10000;
    }

    /// Economic constants
    pub mod economic {
        /// ARC token decimals
        pub const ARC_DECIMALS: u8 = 18;
        
        /// Minimum transaction fee in wei (0.001 ARC)
        pub const MIN_TRANSACTION_FEE: u64 = 1_000_000_000_000_000; // 10^15 wei
        
        /// Maximum transaction fee in wei (1 ARC)
        pub const MAX_TRANSACTION_FEE: u64 = 1_000_000_000_000_000_000; // 10^18 wei
        
        /// Default gas price in wei
        pub const DEFAULT_GAS_PRICE: u64 = 1_000_000_000; // 1 Gwei
        
        /// Block reward for miners/validators (100 ARC)
        pub const BLOCK_REWARD: u64 = 100_000_000_000_000_000_000; // 100 * 10^18 wei
        
        /// Maximum supply cap (100 billion ARC)
        pub const MAX_SUPPLY: u64 = crate::TOTAL_SUPPLY;
        
        /// Inflation rate per year (percentage)
        pub const ANNUAL_INFLATION_RATE: f64 = 3.0;
        
        /// Staking reward rate per year (percentage)
        pub const STAKING_REWARD_RATE: f64 = 8.0;
        
        /// Minimum staking period in days
        pub const MIN_STAKING_PERIOD: u32 = 7;
        
        /// Maximum staking period in days
        pub const MAX_STAKING_PERIOD: u32 = 1460; // 4 years
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_info() {
        let version = VersionInfo::current();
        assert!(!version.version.is_empty());
        assert!(!version.build_timestamp.is_empty());
        assert!(!version.rustc_version.is_empty());
        assert!(!version.target.is_empty());
        
        let display = format!("{}", version);
        assert!(display.starts_with("ArchiveChain v"));
    }

    #[test]
    fn test_constants() {
        assert!(constants::MAX_METADATA_SIZE > 0);
        assert!(constants::MAX_TAGS_PER_ARCHIVE > 0);
        assert!(constants::MAX_TAG_LENGTH > 0);
        assert!(constants::SUPPORTED_CONTENT_TYPES.len() > 0);
        
        // Test node constants
        assert!(constants::nodes::MIN_FULL_ARCHIVE_CAPACITY > 0);
        assert!(constants::nodes::MIN_LIGHT_STORAGE_CAPACITY > 0);
        assert!(constants::nodes::MAX_LIGHT_STORAGE_CAPACITY > constants::nodes::MIN_LIGHT_STORAGE_CAPACITY);
    }

    #[test]
    fn test_economic_constants() {
        assert_eq!(constants::economic::ARC_DECIMALS, 18);
        assert!(constants::economic::MIN_TRANSACTION_FEE > 0);
        assert!(constants::economic::MAX_TRANSACTION_FEE > constants::economic::MIN_TRANSACTION_FEE);
        assert_eq!(constants::economic::MAX_SUPPLY, TOTAL_SUPPLY);
    }

    #[test]
    fn test_utils_format_bytes() {
        assert_eq!(utils::format_bytes(0), "0 B");
        assert_eq!(utils::format_bytes(1024), "1.0 KB");
        assert_eq!(utils::format_bytes(1024 * 1024), "1.0 MB");
        assert_eq!(utils::format_bytes(1536), "1.5 KB");
    }

    #[test]
    fn test_utils_format_duration() {
        use std::time::Duration;
        
        assert_eq!(utils::format_duration(Duration::from_secs(30)), "30s");
        assert_eq!(utils::format_duration(Duration::from_secs(90)), "1m 30s");
        assert_eq!(utils::format_duration(Duration::from_secs(3700)), "1h 1m");
    }

    #[test]
    fn test_utils_token_formatting() {
        // Test format_token_amount
        assert_eq!(utils::format_token_amount(1_000_000_000_000_000_000), "1 ARC");
        assert_eq!(utils::format_token_amount(1_500_000_000_000_000_000), "1.5 ARC");
        assert_eq!(utils::format_token_amount(500_000_000_000_000_000), "0.5 ARC");
        
        // Test parse_token_amount
        assert_eq!(utils::parse_token_amount("1 ARC").unwrap(), 1_000_000_000_000_000_000);
        assert_eq!(utils::parse_token_amount("1.5 ARC").unwrap(), 1_500_000_000_000_000_000);
        assert_eq!(utils::parse_token_amount("0.5").unwrap(), 500_000_000_000_000_000);
    }

    #[test]
    fn test_utils_percentage() {
        assert_eq!(utils::calculate_percentage_change(100.0, 110.0), 10.0);
        assert_eq!(utils::calculate_percentage_change(100.0, 90.0), -10.0);
        assert_eq!(utils::calculate_percentage_change(0.0, 100.0), 100.0);
        
        assert_eq!(utils::format_percentage(10.5), "10.50%");
        assert_eq!(utils::format_percentage(-5.25), "-5.25%");
    }

    #[test]
    fn test_utils_random_functions() {
        let hash1 = utils::random_hash();
        let hash2 = utils::random_hash();
        assert_ne!(hash1, hash2);
        
        let id1 = utils::random_id();
        let id2 = utils::random_id();
        assert_ne!(id1, id2);
        assert_eq!(id1.len(), 32); // UUID simple format
    }

    #[test]
    fn test_feature_flags() {
        // These should compile regardless of features
        let _websocket = features::WEBSOCKET;
        let _grpc = features::GRPC;
        let _p2p = features::P2P;
        let _graphql = features::GRAPHQL;
        let _tls = features::TLS;
        let _metrics = features::METRICS;
        let _advanced_economics = features::ADVANCED_ECONOMICS;
        let _economic_simulation = features::ECONOMIC_SIMULATION;
        let _distributed_nodes = features::DISTRIBUTED_NODES;
    }

    #[tokio::test]
    async fn test_prelude_imports() {
        use crate::prelude::*;
        
        // Test that common types are available
        let _config = BlockchainConfig::default();
        let _api_config = ApiConfig::default();
        let _auth_config = AuthConfig::default();
        
        // Test that economic types are available
        let _economic_config = EconomicConfig::default();
        let _token_config = TokenConfig::default();
        
        // Test that node types are available
        let _node_config = NodeConfig::default();
        
        // Test that traits are available
        let _now: DateTime<Utc> = Utc::now();
        let _uuid = Uuid::new_v4();
    }

    #[test]
    fn test_url_pattern_regex() {
        let regex = regex::Regex::new(constants::URL_PATTERN).unwrap();
        
        assert!(regex.is_match("https://example.com"));
        assert!(regex.is_match("http://test.org/path"));
        assert!(regex.is_match("https://sub.domain.com/path?query=1"));
        
        assert!(!regex.is_match("ftp://example.com"));
        assert!(!regex.is_match("not-a-url"));
        assert!(!regex.is_match(""));
    }

    #[test]
    fn test_supported_content_types() {
        assert!(constants::SUPPORTED_CONTENT_TYPES.contains(&"text/html"));
        assert!(constants::SUPPORTED_CONTENT_TYPES.contains(&"application/pdf"));
        assert!(constants::SUPPORTED_CONTENT_TYPES.contains(&"application/json"));
        
        // Verify all are valid MIME types
        for content_type in constants::SUPPORTED_CONTENT_TYPES {
            assert!(content_type.contains('/'));
            assert!(!content_type.is_empty());
        }
    }

    #[test]
    fn test_token_constants() {
        // Test that token supply constants are consistent
        assert_eq!(ARCHIVAL_REWARDS_ALLOCATION + TEAM_ALLOCATION + COMMUNITY_RESERVE + PUBLIC_SALE, TOTAL_SUPPLY);
        
        // Test individual allocations
        assert_eq!(ARCHIVAL_REWARDS_ALLOCATION, 40_000_000_000); // 40B
        assert_eq!(TEAM_ALLOCATION, 25_000_000_000);            // 25B
        assert_eq!(COMMUNITY_RESERVE, 20_000_000_000);          // 20B
        assert_eq!(PUBLIC_SALE, 15_000_000_000);               // 15B
        assert_eq!(TOTAL_SUPPLY, 100_000_000_000);             // 100B
    }
}

/// Integration tests module
#[cfg(test)]
pub mod integration_tests {
    //! Integration tests that require multiple components
    
    use super::*;
    use crate::prelude::*;

    #[tokio::test]
    async fn test_full_system_integration() {
        // Test that we can create a blockchain and basic API server
        let blockchain_config = BlockchainConfig::default();
        let blockchain_result = Blockchain::new(blockchain_config);
        assert!(blockchain_result.is_ok());
        
        let blockchain = blockchain_result.unwrap();
        let stats = blockchain.get_stats().unwrap();
        assert_eq!(stats.height, 0);
        assert_eq!(stats.total_transactions, 0);
    }

    #[tokio::test]
    async fn test_economic_system_integration() {
        // Test that economic system can be initialized
        let economic_config = EconomicConfig::default();
        let mut economic_model = EconomicModel::new(economic_config);
        
        // Test metrics update
        let result = economic_model.update_all_metrics();
        assert!(result.is_ok());
        
        // Test report generation
        let report = economic_model.generate_economic_report();
        assert_eq!(report.summary.total_supply, TOTAL_SUPPLY);
        assert!(report.summary.economic_health_index >= 0.0);
        assert!(report.summary.economic_health_index <= 1.0);
    }

    #[tokio::test]
    async fn test_token_system_integration() {
        // Test token creation and basic operations
        let mut token = ARCToken::new();
        assert_eq!(token.total_supply, TOTAL_SUPPLY);
        assert_eq!(token.circulating_supply, 0);
        
        // Test token validation
        let validation_result = token.validate_integrity();
        assert!(validation_result.is_ok());
    }

    #[tokio::test]
    async fn test_node_system_integration() {
        // Test node manager creation
        let node_config = NodeConfig::default();
        let node_manager_result = NodeManager::new(node_config).await;
        assert!(node_manager_result.is_ok());
        
        let node_manager = node_manager_result.unwrap();
        
        // Test node creation
        let node_type = NodeType::FullArchive {
            storage_capacity: 20_000_000_000_000,
            replication_factor: 10,
        };
        
        let node_id_result = node_manager.create_node(node_type, None).await;
        assert!(node_id_result.is_ok());
        
        let node_id = node_id_result.unwrap();
        assert!(!node_id.hash().is_zero());
        
        // Test cluster stats
        let stats = node_manager.get_cluster_stats().await;
        assert!(stats.total_nodes > 0);
    }

    #[tokio::test]
    async fn test_api_configuration() {
        // Test that API configuration works
        let mut api_config = ApiConfig::default();
        api_config.rest.port = 8080;
        api_config.websocket.listen_port = 8081;
        api_config.grpc.port = 9090;
        
        assert_eq!(api_config.rest.port, 8080);
        assert_eq!(api_config.websocket.listen_port, 8081);
        assert_eq!(api_config.grpc.port, 9090);
    }

    #[tokio::test]
    async fn test_auth_service_creation() {
        let auth_config = AuthConfig::default();
        let auth_service = AuthService::new(auth_config);
        assert!(auth_service.is_ok());
    }

    #[tokio::test]
    async fn test_staking_system_integration() {
        // Test staking system initialization
        let staking_config = crate::token::staking::StakingConfig::default();
        let staking_system = StakingSystem::new(staking_config);
        
        assert_eq!(staking_system.governance_stakes.len(), 0);
        assert_eq!(staking_system.validator_stakes.len(), 0);
        assert_eq!(staking_system.metrics.total_governance_staked, 0);
    }

    #[tokio::test]
    async fn test_treasury_system_integration() {
        // Test treasury initialization
        let treasury_config = crate::token::treasury::TreasuryConfig::default();
        let treasury = Treasury::new(treasury_config);
        
        assert_eq!(treasury.available_funds, COMMUNITY_RESERVE);
        assert_eq!(treasury.allocated_funds, 0);
        assert_eq!(treasury.disbursed_funds, 0);
    }

    #[tokio::test]
    async fn test_reward_system_integration() {
        // Test reward system initialization
        let reward_config = crate::token::rewards::RewardConfig::default();
        let reward_system = RewardSystem::new(ARCHIVAL_REWARDS_ALLOCATION, reward_config);
        
        let stats = reward_system.get_system_statistics();
        assert_eq!(stats.total_allocated, ARCHIVAL_REWARDS_ALLOCATION);
        assert_eq!(stats.total_distributed, 0);
    }

    #[tokio::test]
    async fn test_health_monitoring_integration() {
        // Test health monitor initialization
        let health_config = crate::nodes::health_monitor::HealthMonitorConfig::default();
        let health_monitor_result = HealthMonitor::new(health_config).await;
        assert!(health_monitor_result.is_ok());
        
        let health_monitor = health_monitor_result.unwrap();
        let stats = health_monitor.get_monitoring_stats().await;
        assert_eq!(stats.total_health_checks, 0);
        assert_eq!(stats.successful_checks, 0);
    }
}