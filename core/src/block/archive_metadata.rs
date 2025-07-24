//! Métadonnées d'archives pour ArchiveChain
//! 
//! Contient les structures pour décrire les archives web stockées dans la blockchain

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::crypto::{Hash, HashAlgorithm, compute_hash};
use crate::error::{BlockError, Result};

/// Types de compression supportés
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// Pas de compression
    None,
    /// Compression Gzip
    Gzip,
    /// Compression Brotli (optimisée pour le web)
    Brotli,
    /// Compression LZ4 (rapide)
    Lz4,
    /// Compression Zstandard (haute performance)
    Zstd,
}

impl CompressionType {
    /// Retourne l'extension de fichier associée
    pub fn extension(&self) -> &'static str {
        match self {
            CompressionType::None => "",
            CompressionType::Gzip => ".gz",
            CompressionType::Brotli => ".br",
            CompressionType::Lz4 => ".lz4",
            CompressionType::Zstd => ".zst",
        }
    }

    /// Retourne le ratio de compression typique
    pub fn typical_ratio(&self) -> f32 {
        match self {
            CompressionType::None => 1.0,
            CompressionType::Gzip => 0.3,
            CompressionType::Brotli => 0.25,
            CompressionType::Lz4 => 0.4,
            CompressionType::Zstd => 0.28,
        }
    }
}

/// Métadonnées détaillées d'une archive
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveMetadata {
    /// Titre de la page archivée
    pub title: Option<String>,
    
    /// Description ou extrait du contenu
    pub description: Option<String>,
    
    /// Mots-clés pour la recherche
    pub keywords: Vec<String>,
    
    /// Type MIME du contenu principal
    pub content_type: String,
    
    /// Langue détectée du contenu
    pub language: Option<String>,
    
    /// Auteur ou créateur du contenu
    pub author: Option<String>,
    
    /// Date de publication originale
    pub published_at: Option<DateTime<Utc>>,
    
    /// Métadonnées personnalisées
    pub custom_metadata: HashMap<String, String>,
    
    /// Nombre de liens externes
    pub external_links_count: u32,
    
    /// Nombre de ressources (images, CSS, JS)
    pub resource_count: u32,
    
    /// Score de qualité du contenu (0-100)
    pub quality_score: u8,
    
    /// Indicateurs de contenu
    pub content_flags: ContentFlags,
}

/// Indicateurs sur le type et la qualité du contenu
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentFlags {
    /// Contient du JavaScript exécutable
    pub has_javascript: bool,
    
    /// Contient des formulaires
    pub has_forms: bool,
    
    /// Contient du contenu multimédia
    pub has_media: bool,
    
    /// Contient des publicités détectées
    pub has_ads: bool,
    
    /// Contenu potentiellement sensible
    pub is_sensitive: bool,
    
    /// Archive complète (tous les assets)
    pub is_complete: bool,
}

impl Default for ContentFlags {
    fn default() -> Self {
        Self {
            has_javascript: false,
            has_forms: false,
            has_media: false,
            has_ads: false,
            is_sensitive: false,
            is_complete: true,
        }
    }
}

/// Structure d'un bloc d'archive selon les spécifications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveBlock {
    /// Identifiant unique de l'archive
    pub archive_id: Hash,
    
    /// URL originale archivée
    pub original_url: String,
    
    /// Timestamp de capture
    pub capture_timestamp: DateTime<Utc>,
    
    /// Type de contenu principal
    pub content_type: String,
    
    /// Type de compression utilisé
    pub compression: CompressionType,
    
    /// Taille des données compressées
    pub size_compressed: u64,
    
    /// Taille des données originales
    pub size_original: u64,
    
    /// Checksum des données
    pub checksum: Hash,
    
    /// Métadonnées détaillées
    pub metadata: ArchiveMetadata,
    
    /// Hash de verification pour l'intégrité
    pub verification_hash: Hash,
}

impl ArchiveBlock {
    /// Crée un nouveau bloc d'archive
    pub fn new(
        original_url: String,
        content_type: String,
        compression: CompressionType,
        size_compressed: u64,
        size_original: u64,
        checksum: Hash,
        metadata: ArchiveMetadata,
    ) -> Self {
        let capture_timestamp = Utc::now();
        let archive_id = Self::generate_archive_id(&original_url, capture_timestamp);
        
        let mut archive = Self {
            archive_id,
            original_url,
            capture_timestamp,
            content_type,
            compression,
            size_compressed,
            size_original,
            checksum,
            metadata,
            verification_hash: Hash::zero(),
        };
        
        // Calcule le hash de vérification
        archive.verification_hash = archive.calculate_verification_hash();
        archive
    }

    /// Génère un identifiant unique pour l'archive
    fn generate_archive_id(url: &str, timestamp: DateTime<Utc>) -> Hash {
        let data = format!("{}:{}", url, timestamp.timestamp());
        compute_hash(data.as_bytes(), HashAlgorithm::Blake3)
    }

    /// Calcule le hash de vérification de l'archive
    pub fn calculate_verification_hash(&self) -> Hash {
        let mut data = Vec::new();
        
        // Sérialise les champs principaux
        data.extend_from_slice(self.archive_id.as_bytes());
        data.extend_from_slice(self.original_url.as_bytes());
        data.extend_from_slice(&self.capture_timestamp.timestamp().to_le_bytes());
        data.extend_from_slice(self.content_type.as_bytes());
        data.extend_from_slice(&self.size_compressed.to_le_bytes());
        data.extend_from_slice(&self.size_original.to_le_bytes());
        data.extend_from_slice(self.checksum.as_bytes());
        
        compute_hash(&data, HashAlgorithm::Blake3)
    }

    /// Vérifie l'intégrité de l'archive
    pub fn verify_integrity(&self) -> bool {
        let calculated_hash = self.calculate_verification_hash();
        calculated_hash == self.verification_hash
    }

    /// Vérifie si l'archive est valide
    pub fn is_valid(&self) -> Result<bool> {
        // Vérifie l'intégrité
        if !self.verify_integrity() {
            return Ok(false);
        }

        // Vérifie l'URL
        if self.original_url.is_empty() {
            return Ok(false);
        }

        // Vérifie les tailles
        if self.size_compressed == 0 || self.size_original == 0 {
            return Ok(false);
        }

        // Vérifie que la compression est cohérente
        if self.size_compressed > self.size_original && self.compression != CompressionType::None {
            return Ok(false);
        }

        // Vérifie le timestamp (ne doit pas être dans le futur)
        if self.capture_timestamp > Utc::now() {
            return Ok(false);
        }

        Ok(true)
    }

    /// Calcule le ratio de compression
    pub fn compression_ratio(&self) -> f32 {
        if self.size_original == 0 {
            return 1.0;
        }
        self.size_compressed as f32 / self.size_original as f32
    }

    /// Obtient la taille économisée par la compression
    pub fn bytes_saved(&self) -> u64 {
        self.size_original.saturating_sub(self.size_compressed)
    }

    /// Vérifie si l'archive correspond à un pattern d'URL
    pub fn matches_url_pattern(&self, pattern: &str) -> bool {
        // Implémentation simple - peut être étendue avec regex
        self.original_url.contains(pattern)
    }

    /// Obtient l'âge de l'archive en secondes
    pub fn age_seconds(&self) -> i64 {
        (Utc::now() - self.capture_timestamp).num_seconds()
    }

    /// Vérifie si l'archive est récente (moins de X heures)
    pub fn is_recent(&self, hours: i64) -> bool {
        self.age_seconds() < (hours * 3600)
    }
}

/// Builder pour créer des archives de manière fluide
#[derive(Debug)]
pub struct ArchiveBlockBuilder {
    original_url: String,
    content_type: String,
    compression: CompressionType,
    size_compressed: u64,
    size_original: u64,
    checksum: Hash,
    metadata: Option<ArchiveMetadata>,
}

impl ArchiveBlockBuilder {
    /// Crée un nouveau builder
    pub fn new(
        original_url: String,
        content_type: String,
        compression: CompressionType,
        size_compressed: u64,
        size_original: u64,
        checksum: Hash,
    ) -> Self {
        Self {
            original_url,
            content_type,
            compression,
            size_compressed,
            size_original,
            checksum,
            metadata: None,
        }
    }

    /// Définit les métadonnées
    pub fn metadata(mut self, metadata: ArchiveMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Construit l'archive
    pub fn build(self) -> ArchiveBlock {
        let metadata = self.metadata.unwrap_or_else(|| ArchiveMetadata {
            title: None,
            description: None,
            keywords: Vec::new(),
            content_type: self.content_type.clone(),
            language: None,
            author: None,
            published_at: None,
            custom_metadata: HashMap::new(),
            external_links_count: 0,
            resource_count: 0,
            quality_score: 50,
            content_flags: ContentFlags::default(),
        });

        ArchiveBlock::new(
            self.original_url,
            self.content_type,
            self.compression,
            self.size_compressed,
            self.size_original,
            self.checksum,
            metadata,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    fn create_test_metadata() -> ArchiveMetadata {
        ArchiveMetadata {
            title: Some("Test Page".to_string()),
            description: Some("A test page for ArchiveChain".to_string()),
            keywords: vec!["test".to_string(), "archive".to_string()],
            content_type: "text/html".to_string(),
            language: Some("en".to_string()),
            author: Some("Test Author".to_string()),
            published_at: Some(Utc::now()),
            custom_metadata: HashMap::new(),
            external_links_count: 5,
            resource_count: 10,
            quality_score: 85,
            content_flags: ContentFlags::default(),
        }
    }

    fn create_test_archive() -> ArchiveBlock {
        ArchiveBlockBuilder::new(
            "https://example.com/test".to_string(),
            "text/html".to_string(),
            CompressionType::Gzip,
            1024,
            4096,
            Hash::zero(),
        )
        .metadata(create_test_metadata())
        .build()
    }

    #[test]
    fn test_compression_type() {
        assert_eq!(CompressionType::Gzip.extension(), ".gz");
        assert!(CompressionType::Brotli.typical_ratio() < 1.0);
    }

    #[test]
    fn test_archive_creation() {
        let archive = create_test_archive();
        assert_eq!(archive.original_url, "https://example.com/test");
        assert_eq!(archive.content_type, "text/html");
        assert_eq!(archive.compression, CompressionType::Gzip);
    }

    #[test]
    fn test_archive_integrity() {
        let archive = create_test_archive();
        assert!(archive.verify_integrity());
    }

    #[test]
    fn test_archive_validity() {
        let archive = create_test_archive();
        assert!(archive.is_valid().unwrap());
    }

    #[test]
    fn test_compression_ratio() {
        let archive = create_test_archive();
        assert_eq!(archive.compression_ratio(), 0.25); // 1024/4096
        assert_eq!(archive.bytes_saved(), 3072); // 4096-1024
    }

    #[test]
    fn test_url_pattern_matching() {
        let archive = create_test_archive();
        assert!(archive.matches_url_pattern("example.com"));
        assert!(archive.matches_url_pattern("test"));
        assert!(!archive.matches_url_pattern("nonexistent"));
    }

    #[test]
    fn test_archive_age() {
        let archive = create_test_archive();
        assert!(archive.is_recent(24)); // Should be recent (created now)
        assert!(archive.age_seconds() >= 0);
    }

    #[test]
    fn test_archive_builder() {
        let metadata = create_test_metadata();
        let archive = ArchiveBlockBuilder::new(
            "https://test.com".to_string(),
            "application/json".to_string(),
            CompressionType::Zstd,
            500,
            2000,
            Hash::zero(),
        )
        .metadata(metadata)
        .build();

        assert_eq!(archive.compression, CompressionType::Zstd);
        assert_eq!(archive.metadata.title, Some("Test Page".to_string()));
    }

    #[test]
    fn test_verification_hash_consistency() {
        let archive = create_test_archive();
        let hash1 = archive.calculate_verification_hash();
        let hash2 = archive.calculate_verification_hash();
        assert_eq!(hash1, hash2);
    }
}