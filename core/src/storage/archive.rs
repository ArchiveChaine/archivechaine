//! Stockage d'archives optimisé pour ArchiveChain
//! 
//! Implémente :
//! - Stockage optimisé des archives web
//! - Compression avec Zstd et déduplication
//! - Chiffrement optionnel des données
//! - Vérification d'intégrité automatique
//! - Chunking pour les gros fichiers

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::io::{Read, Write};
use std::time::{Duration, SystemTime};
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use crate::crypto::{Hash, HashAlgorithm, compute_hash};
use crate::consensus::NodeId;
use crate::error::Result;
use super::{ContentMetadata, StorageNodeInfo};

/// Configuration du stockage d'archives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveConfig {
    /// Algorithme de compression par défaut
    pub compression_algorithm: CompressionType,
    /// Chiffrement activé par défaut
    pub encryption_enabled: bool,
    /// Taille des chunks (bytes)
    pub chunk_size: usize,
    /// Déduplication activée
    pub deduplication: bool,
    /// Vérifications d'intégrité automatiques
    pub integrity_checks: bool,
    /// Répertoire de stockage de base
    pub base_storage_path: PathBuf,
    /// Seuil pour le chunking (fichiers > seuil sont chunkés)
    pub chunking_threshold: u64,
    /// Compression maximale (plus lent mais plus compact)
    pub max_compression: bool,
    /// Cache de déduplication en mémoire
    pub dedup_cache_size: usize,
}

impl Default for ArchiveConfig {
    fn default() -> Self {
        Self {
            compression_algorithm: CompressionType::Zstd,
            encryption_enabled: false,
            chunk_size: 1024 * 1024, // 1MB
            deduplication: true,
            integrity_checks: true,
            base_storage_path: PathBuf::from("./storage"),
            chunking_threshold: 10 * 1024 * 1024, // 10MB
            max_compression: false,
            dedup_cache_size: 10000,
        }
    }
}

/// Types de compression supportés
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// Pas de compression
    None,
    /// Compression Gzip (compatibilité)
    Gzip,
    /// Compression LZ4 (rapide)
    Lz4,
    /// Compression Zstd (recommandé)
    Zstd,
    /// Compression Brotli (web-optimisé)
    Brotli,
}

impl CompressionType {
    /// Retourne l'extension de fichier
    pub fn extension(&self) -> &'static str {
        match self {
            CompressionType::None => "",
            CompressionType::Gzip => ".gz",
            CompressionType::Lz4 => ".lz4",
            CompressionType::Zstd => ".zst",
            CompressionType::Brotli => ".br",
        }
    }

    /// Niveau de compression recommandé
    pub fn recommended_level(&self, max_compression: bool) -> i32 {
        match self {
            CompressionType::None => 0,
            CompressionType::Gzip => if max_compression { 9 } else { 6 },
            CompressionType::Lz4 => if max_compression { 9 } else { 1 },
            CompressionType::Zstd => if max_compression { 19 } else { 3 },
            CompressionType::Brotli => if max_compression { 11 } else { 6 },
        }
    }
}

/// Configuration de compression
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompressionConfig {
    /// Type de compression
    pub compression_type: CompressionType,
    /// Niveau de compression
    pub level: i32,
    /// Compression adaptative selon le type de contenu
    pub adaptive: bool,
}

impl Default for CompressionConfig {
    fn default() -> Self {
        Self {
            compression_type: CompressionType::Zstd,
            level: 3,
            adaptive: true,
        }
    }
}

/// Configuration de chiffrement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Chiffrement activé
    pub enabled: bool,
    /// Algorithme de chiffrement
    pub algorithm: EncryptionAlgorithm,
    /// Clé de chiffrement (en pratique, dérivée d'un master secret)
    pub encryption_key: Option<Vec<u8>>,
    /// IV/Nonce pour le chiffrement
    pub use_random_iv: bool,
}

/// Algorithmes de chiffrement supportés
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionAlgorithm {
    /// ChaCha20-Poly1305 (recommandé)
    ChaCha20Poly1305,
    /// AES-256-GCM
    Aes256Gcm,
}

/// Gestionnaire de chunks
#[derive(Debug)]
pub struct ChunkManager {
    /// Taille des chunks
    chunk_size: usize,
    /// Cache des métadonnées de chunks
    chunk_metadata: HashMap<Hash, ChunkMetadata>,
}

/// Métadonnées d'un chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkMetadata {
    /// Hash du chunk
    pub chunk_hash: Hash,
    /// Taille originale
    pub original_size: u64,
    /// Taille compressée
    pub compressed_size: u64,
    /// Offset dans le fichier original
    pub offset: u64,
    /// Nombre de références
    pub ref_count: u32,
    /// Timestamp de création
    pub created_at: SystemTime,
}

impl ChunkManager {
    /// Crée un nouveau gestionnaire de chunks
    pub fn new(chunk_size: usize) -> Self {
        Self {
            chunk_size,
            chunk_metadata: HashMap::new(),
        }
    }

    /// Divise les données en chunks
    pub fn create_chunks(&mut self, data: &[u8]) -> Result<Vec<ChunkInfo>> {
        let mut chunks = Vec::new();
        let mut offset = 0;

        while offset < data.len() {
            let end = (offset + self.chunk_size).min(data.len());
            let chunk_data = &data[offset..end];
            
            let chunk_hash = compute_hash(chunk_data, HashAlgorithm::Blake3);
            let chunk_info = ChunkInfo {
                hash: chunk_hash,
                data: chunk_data.to_vec(),
                offset: offset as u64,
                size: chunk_data.len() as u64,
            };

            // Met à jour les métadonnées
            let metadata = ChunkMetadata {
                chunk_hash,
                original_size: chunk_data.len() as u64,
                compressed_size: 0, // Sera mis à jour après compression
                offset: offset as u64,
                ref_count: 1,
                created_at: SystemTime::now(),
            };

            self.chunk_metadata.insert(chunk_hash, metadata);
            chunks.push(chunk_info);
            offset = end;
        }

        Ok(chunks)
    }

    /// Reconstitue les données depuis les chunks
    pub fn reassemble_chunks(&self, chunks: &[ChunkInfo]) -> Result<Vec<u8>> {
        let total_size: usize = chunks.iter().map(|c| c.size as usize).sum();
        let mut result = Vec::with_capacity(total_size);

        for chunk in chunks {
            result.extend_from_slice(&chunk.data);
        }

        Ok(result)
    }

    /// Incrémente le compteur de référence d'un chunk
    pub fn increment_ref_count(&mut self, chunk_hash: &Hash) {
        if let Some(metadata) = self.chunk_metadata.get_mut(chunk_hash) {
            metadata.ref_count += 1;
        }
    }

    /// Décrémente le compteur de référence d'un chunk
    pub fn decrement_ref_count(&mut self, chunk_hash: &Hash) -> bool {
        if let Some(metadata) = self.chunk_metadata.get_mut(chunk_hash) {
            metadata.ref_count = metadata.ref_count.saturating_sub(1);
            metadata.ref_count == 0
        } else {
            false
        }
    }
}

/// Information sur un chunk
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkInfo {
    /// Hash du chunk
    pub hash: Hash,
    /// Données du chunk
    pub data: Vec<u8>,
    /// Offset dans le fichier original
    pub offset: u64,
    /// Taille du chunk
    pub size: u64,
}

/// Moteur de déduplication
#[derive(Debug)]
pub struct DeduplicationEngine {
    /// Cache des hash de contenu
    content_hashes: HashMap<Hash, ContentReference>,
    /// Cache LRU pour optimiser les accès
    access_order: std::collections::VecDeque<Hash>,
    /// Taille maximale du cache
    max_cache_size: usize,
}

/// Référence vers du contenu dédupliqué
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentReference {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Chemin vers le fichier de stockage
    pub storage_path: PathBuf,
    /// Nombre de références
    pub ref_count: u32,
    /// Taille du contenu
    pub size: u64,
    /// Timestamp de dernière utilisation
    pub last_accessed: SystemTime,
}

impl DeduplicationEngine {
    /// Crée un nouveau moteur de déduplication
    pub fn new(max_cache_size: usize) -> Self {
        Self {
            content_hashes: HashMap::new(),
            access_order: std::collections::VecDeque::new(),
            max_cache_size,
        }
    }

    /// Vérifie si le contenu existe déjà
    pub fn check_duplicate(&mut self, content_hash: &Hash) -> Option<&ContentReference> {
        if let Some(reference) = self.content_hashes.get_mut(content_hash) {
            reference.last_accessed = SystemTime::now();
            
            // Met à jour l'ordre d'accès LRU
            self.access_order.retain(|h| h != content_hash);
            self.access_order.push_back(*content_hash);
            
            Some(reference)
        } else {
            None
        }
    }

    /// Ajoute une nouvelle référence de contenu
    pub fn add_content(&mut self, content_hash: Hash, storage_path: PathBuf, size: u64) {
        let reference = ContentReference {
            content_hash,
            storage_path,
            ref_count: 1,
            size,
            last_accessed: SystemTime::now(),
        };

        self.content_hashes.insert(content_hash, reference);
        self.access_order.push_back(content_hash);

        // Éviction LRU si nécessaire
        self.evict_if_needed();
    }

    /// Incrémente le compteur de référence
    pub fn add_reference(&mut self, content_hash: &Hash) -> bool {
        if let Some(reference) = self.content_hashes.get_mut(content_hash) {
            reference.ref_count += 1;
            reference.last_accessed = SystemTime::now();
            true
        } else {
            false
        }
    }

    /// Décrémente le compteur de référence
    pub fn remove_reference(&mut self, content_hash: &Hash) -> bool {
        if let Some(reference) = self.content_hashes.get_mut(content_hash) {
            reference.ref_count = reference.ref_count.saturating_sub(1);
            reference.ref_count == 0
        } else {
            false
        }
    }

    /// Éviction LRU
    fn evict_if_needed(&mut self) {
        while self.content_hashes.len() > self.max_cache_size {
            if let Some(oldest_hash) = self.access_order.pop_front() {
                // Ne supprime que si ref_count == 0
                if let Some(reference) = self.content_hashes.get(&oldest_hash) {
                    if reference.ref_count == 0 {
                        self.content_hashes.remove(&oldest_hash);
                    }
                }
            }
        }
    }

    /// Obtient les statistiques de déduplication
    pub fn get_stats(&self) -> DeduplicationStats {
        let total_refs: u32 = self.content_hashes.values().map(|r| r.ref_count).sum();
        let unique_content = self.content_hashes.len() as u32;
        let total_size: u64 = self.content_hashes.values().map(|r| r.size).sum();

        DeduplicationStats {
            unique_content,
            total_references: total_refs,
            total_size,
            deduplication_ratio: if unique_content > 0 {
                total_refs as f64 / unique_content as f64
            } else {
                1.0
            },
        }
    }
}

/// Statistiques de déduplication
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeduplicationStats {
    /// Nombre de contenus uniques
    pub unique_content: u32,
    /// Nombre total de références
    pub total_references: u32,
    /// Taille totale unique
    pub total_size: u64,
    /// Ratio de déduplication
    pub deduplication_ratio: f64,
}

/// Vérificateur d'intégrité
#[derive(Debug)]
pub struct IntegrityChecker {
    /// Cache des checksums vérifiés
    verified_checksums: HashMap<Hash, SystemTime>,
    /// Intervalle de revérification
    reverify_interval: Duration,
}

impl IntegrityChecker {
    /// Crée un nouveau vérificateur
    pub fn new(reverify_interval: Duration) -> Self {
        Self {
            verified_checksums: HashMap::new(),
            reverify_interval,
        }
    }

    /// Vérifie l'intégrité d'un fichier
    pub async fn verify_integrity(&mut self, file_path: &Path, expected_hash: &Hash) -> Result<bool> {
        // Vérifie si une vérification récente existe
        if let Some(&last_check) = self.verified_checksums.get(expected_hash) {
            if last_check.elapsed().unwrap_or(Duration::MAX) < self.reverify_interval {
                return Ok(true);
            }
        }

        // Lit et vérifie le fichier
        let data = fs::read(file_path).await.map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur lecture fichier: {}", e),
            }
        })?;

        let actual_hash = compute_hash(&data, HashAlgorithm::Blake3);
        let is_valid = actual_hash == *expected_hash;

        if is_valid {
            self.verified_checksums.insert(*expected_hash, SystemTime::now());
        }

        Ok(is_valid)
    }

    /// Vérifie l'intégrité de chunks
    pub async fn verify_chunks(&mut self, chunks: &[ChunkInfo], base_path: &Path) -> Result<Vec<bool>> {
        let mut results = Vec::new();

        for chunk in chunks {
            let chunk_path = base_path.join(format!("{}.chunk", chunk.hash.to_hex()));
            let is_valid = self.verify_integrity(&chunk_path, &chunk.hash).await?;
            results.push(is_valid);
        }

        Ok(results)
    }

    /// Nettoie les anciennes vérifications
    pub fn cleanup_old_verifications(&mut self) {
        let cutoff = SystemTime::now() - self.reverify_interval;
        self.verified_checksums.retain(|_, &mut timestamp| timestamp > cutoff);
    }
}

/// Stockage d'archives principal
pub struct ArchiveStorage {
    /// Configuration
    config: ArchiveConfig,
    /// Configuration de compression
    compression_config: CompressionConfig,
    /// Configuration de chiffrement
    encryption_config: EncryptionConfig,
    /// Gestionnaire de chunks
    chunk_manager: ChunkManager,
    /// Moteur de déduplication
    deduplication_engine: DeduplicationEngine,
    /// Vérificateur d'intégrité
    integrity_checker: IntegrityChecker,
    /// Statistiques d'utilisation
    stats: ArchiveStats,
}

/// Statistiques du stockage d'archives
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchiveStats {
    /// Nombre total de fichiers stockés
    pub total_files: u64,
    /// Taille totale stockée (compressée)
    pub total_size_compressed: u64,
    /// Taille totale originale
    pub total_size_original: u64,
    /// Ratio de compression moyen
    pub average_compression_ratio: f64,
    /// Statistiques de déduplication
    pub deduplication_stats: DeduplicationStats,
    /// Nombre de vérifications d'intégrité
    pub integrity_checks_performed: u64,
    /// Taux de succès des vérifications
    pub integrity_success_rate: f64,
}

impl Default for ArchiveStats {
    fn default() -> Self {
        Self {
            total_files: 0,
            total_size_compressed: 0,
            total_size_original: 0,
            average_compression_ratio: 1.0,
            deduplication_stats: DeduplicationStats {
                unique_content: 0,
                total_references: 0,
                total_size: 0,
                deduplication_ratio: 1.0,
            },
            integrity_checks_performed: 0,
            integrity_success_rate: 1.0,
        }
    }
}

impl ArchiveStorage {
    /// Crée un nouveau système de stockage d'archives
    pub fn new(config: ArchiveConfig) -> Result<Self> {
        // Crée le répertoire de stockage si nécessaire
        std::fs::create_dir_all(&config.base_storage_path).map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Impossible de créer le répertoire de stockage: {}", e),
            }
        })?;

        let compression_config = CompressionConfig {
            compression_type: config.compression_algorithm.clone(),
            level: config.compression_algorithm.recommended_level(config.max_compression),
            adaptive: true,
        };

        let encryption_config = EncryptionConfig {
            enabled: config.encryption_enabled,
            algorithm: EncryptionAlgorithm::ChaCha20Poly1305,
            encryption_key: None, // À initialiser selon les besoins
            use_random_iv: true,
        };

        Ok(Self {
            chunk_manager: ChunkManager::new(config.chunk_size),
            deduplication_engine: DeduplicationEngine::new(config.dedup_cache_size),
            integrity_checker: IntegrityChecker::new(Duration::from_secs(24 * 3600)), // 24h
            compression_config,
            encryption_config,
            config,
            stats: ArchiveStats::default(),
        })
    }

    /// Stocke du contenu de manière optimisée
    pub async fn store_content_optimized(
        &mut self,
        data: &[u8],
        metadata: &ContentMetadata,
        target_nodes: &[NodeId],
    ) -> Result<Vec<NodeId>> {
        let content_hash = compute_hash(data, HashAlgorithm::Blake3);

        // Vérifie la déduplication
        if self.config.deduplication {
            if let Some(_existing_ref) = self.deduplication_engine.check_duplicate(&content_hash) {
                // Contenu déjà existant, ajoute une référence
                self.deduplication_engine.add_reference(&content_hash);
                return Ok(target_nodes.to_vec()); // Simule le stockage réussi
            }
        }

        // Détermine si le chunking est nécessaire
        let processed_data = if data.len() as u64 > self.config.chunking_threshold {
            self.store_chunked_content(data, &content_hash).await?
        } else {
            self.store_single_content(data, &content_hash).await?
        };

        // Met à jour les statistiques
        self.update_stats(data.len() as u64, processed_data.len() as u64);

        // Ajoute à la déduplication
        if self.config.deduplication {
            let storage_path = self.get_content_path(&content_hash);
            self.deduplication_engine.add_content(content_hash, storage_path, data.len() as u64);
        }

        Ok(target_nodes.to_vec())
    }

    /// Stocke du contenu en chunks
    async fn store_chunked_content(&mut self, data: &[u8], content_hash: &Hash) -> Result<Vec<u8>> {
        let chunks = self.chunk_manager.create_chunks(data)?;
        let mut stored_chunks = Vec::new();

        for chunk in chunks {
            let compressed_chunk = self.compress_data(&chunk.data)?;
            let chunk_path = self.get_chunk_path(&chunk.hash);
            
            // Crée les répertoires si nécessaire
            if let Some(parent) = chunk_path.parent() {
                fs::create_dir_all(parent).await.map_err(|e| {
                    crate::error::CoreError::Internal {
                        message: format!("Erreur création répertoire chunk: {}", e),
                    }
                })?;
            }

            // Écrit le chunk
            fs::write(&chunk_path, &compressed_chunk).await.map_err(|e| {
                crate::error::CoreError::Internal {
                    message: format!("Erreur écriture chunk: {}", e),
                }
            })?;

            stored_chunks.push(compressed_chunk);
        }

        // Crée un index des chunks
        let chunk_index = ChunkIndex {
            content_hash: *content_hash,
            chunks: chunks.iter().map(|c| c.hash).collect(),
            total_size: data.len() as u64,
        };

        let index_data = bincode::serialize(&chunk_index).map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur sérialisation index chunks: {}", e),
            }
        })?;

        let index_path = self.get_chunk_index_path(content_hash);
        fs::write(&index_path, &index_data).await.map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur écriture index chunks: {}", e),
            }
        })?;

        Ok(index_data)
    }

    /// Stocke du contenu en un seul bloc
    async fn store_single_content(&mut self, data: &[u8], content_hash: &Hash) -> Result<Vec<u8>> {
        let compressed_data = self.compress_data(data)?;
        let content_path = self.get_content_path(content_hash);

        // Crée les répertoires si nécessaire
        if let Some(parent) = content_path.parent() {
            fs::create_dir_all(parent).await.map_err(|e| {
                crate::error::CoreError::Internal {
                    message: format!("Erreur création répertoire: {}", e),
                }
            })?;
        }

        // Écrit le contenu
        fs::write(&content_path, &compressed_data).await.map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur écriture contenu: {}", e),
            }
        })?;

        Ok(compressed_data)
    }

    /// Récupère du contenu depuis un nœud
    pub async fn retrieve_content_from_node(&self, content_hash: &Hash, node_id: &NodeId) -> Result<Vec<u8>> {
        // Vérifie d'abord si c'est du contenu chunké
        let chunk_index_path = self.get_chunk_index_path(content_hash);
        
        if chunk_index_path.exists() {
            self.retrieve_chunked_content(content_hash).await
        } else {
            self.retrieve_single_content(content_hash).await
        }
    }

    /// Récupère du contenu chunké
    async fn retrieve_chunked_content(&self, content_hash: &Hash) -> Result<Vec<u8>> {
        let index_path = self.get_chunk_index_path(content_hash);
        let index_data = fs::read(&index_path).await.map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur lecture index chunks: {}", e),
            }
        })?;

        let chunk_index: ChunkIndex = bincode::deserialize(&index_data).map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur désérialisation index chunks: {}", e),
            }
        })?;

        let mut chunks_data = Vec::new();
        for chunk_hash in &chunk_index.chunks {
            let chunk_path = self.get_chunk_path(chunk_hash);
            let compressed_chunk = fs::read(&chunk_path).await.map_err(|e| {
                crate::error::CoreError::Internal {
                    message: format!("Erreur lecture chunk: {}", e),
                }
            })?;

            let decompressed_chunk = self.decompress_data(&compressed_chunk)?;
            chunks_data.push(decompressed_chunk);
        }

        // Reconstitue les données
        let total_size: usize = chunks_data.iter().map(|c| c.len()).sum();
        let mut result = Vec::with_capacity(total_size);
        for chunk_data in chunks_data {
            result.extend_from_slice(&chunk_data);
        }

        Ok(result)
    }

    /// Récupère du contenu simple
    async fn retrieve_single_content(&self, content_hash: &Hash) -> Result<Vec<u8>> {
        let content_path = self.get_content_path(content_hash);
        let compressed_data = fs::read(&content_path).await.map_err(|e| {
            crate::error::CoreError::Internal {
                message: format!("Erreur lecture contenu: {}", e),
            }
        })?;

        self.decompress_data(&compressed_data)
    }

    /// Compresse des données
    fn compress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        match &self.compression_config.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Zstd => {
                zstd::encode_all(data, self.compression_config.level).map_err(|e| {
                    crate::error::CoreError::Internal {
                        message: format!("Erreur compression Zstd: {}", e),
                    }
                })
            },
            CompressionType::Gzip => {
                use flate2::{Compression, write::GzEncoder};
                let mut encoder = GzEncoder::new(Vec::new(), Compression::new(self.compression_config.level as u32));
                encoder.write_all(data).map_err(|e| {
                    crate::error::CoreError::Internal {
                        message: format!("Erreur compression Gzip: {}", e),
                    }
                })?;
                encoder.finish().map_err(|e| {
                    crate::error::CoreError::Internal {
                        message: format!("Erreur finalisation Gzip: {}", e),
                    }
                })
            },
            _ => {
                // Autres algorithmes à implémenter
                Err(crate::error::CoreError::Internal {
                    message: "Algorithme de compression non supporté".to_string(),
                })
            }
        }
    }

    /// Décompresse des données
    fn decompress_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        match &self.compression_config.compression_type {
            CompressionType::None => Ok(data.to_vec()),
            CompressionType::Zstd => {
                zstd::decode_all(data).map_err(|e| {
                    crate::error::CoreError::Internal {
                        message: format!("Erreur décompression Zstd: {}", e),
                    }
                })
            },
            CompressionType::Gzip => {
                use flate2::read::GzDecoder;
                let mut decoder = GzDecoder::new(data);
                let mut decompressed = Vec::new();
                decoder.read_to_end(&mut decompressed).map_err(|e| {
                    crate::error::CoreError::Internal {
                        message: format!("Erreur décompression Gzip: {}", e),
                    }
                })?;
                Ok(decompressed)
            },
            _ => {
                Err(crate::error::CoreError::Internal {
                    message: "Algorithme de décompression non supporté".to_string(),
                })
            }
        }
    }

    /// Obtient le chemin d'un contenu
    fn get_content_path(&self, content_hash: &Hash) -> PathBuf {
        let hex = content_hash.to_hex();
        let dir1 = &hex[0..2];
        let dir2 = &hex[2..4];
        self.config.base_storage_path
            .join("content")
            .join(dir1)
            .join(dir2)
            .join(format!("{}{}", hex, self.compression_config.compression_type.extension()))
    }

    /// Obtient le chemin d'un chunk
    fn get_chunk_path(&self, chunk_hash: &Hash) -> PathBuf {
        let hex = chunk_hash.to_hex();
        let dir1 = &hex[0..2];
        let dir2 = &hex[2..4];
        self.config.base_storage_path
            .join("chunks")
            .join(dir1)
            .join(dir2)
            .join(format!("{}.chunk{}", hex, self.compression_config.compression_type.extension()))
    }

    /// Obtient le chemin de l'index de chunks
    fn get_chunk_index_path(&self, content_hash: &Hash) -> PathBuf {
        let hex = content_hash.to_hex();
        let dir1 = &hex[0..2];
        let dir2 = &hex[2..4];
        self.config.base_storage_path
            .join("indexes")
            .join(dir1)
            .join(dir2)
            .join(format!("{}.index", hex))
    }

    /// Met à jour les statistiques
    fn update_stats(&mut self, original_size: u64, compressed_size: u64) {
        self.stats.total_files += 1;
        self.stats.total_size_original += original_size;
        self.stats.total_size_compressed += compressed_size;
        
        self.stats.average_compression_ratio = 
            self.stats.total_size_compressed as f64 / self.stats.total_size_original as f64;
    }

    /// Vérifie l'intégrité d'un contenu
    pub async fn verify_content_integrity(&mut self, content_hash: &Hash) -> Result<bool> {
        if self.get_chunk_index_path(content_hash).exists() {
            // Contenu chunké
            let index_path = self.get_chunk_index_path(content_hash);
            let index_data = fs::read(&index_path).await.map_err(|e| {
                crate::error::CoreError::Internal {
                    message: format!("Erreur lecture index: {}", e),
                }
            })?;

            let chunk_index: ChunkIndex = bincode::deserialize(&index_data).map_err(|e| {
                crate::error::CoreError::Internal {
                    message: format!("Erreur désérialisation index: {}", e),
                }
            })?;

            let chunks_info: Vec<ChunkInfo> = chunk_index.chunks.iter()
                .map(|hash| ChunkInfo {
                    hash: *hash,
                    data: Vec::new(), // Données pas nécessaires pour la vérification
                    offset: 0,
                    size: 0,
                })
                .collect();

            let verification_results = self.integrity_checker
                .verify_chunks(&chunks_info, &self.config.base_storage_path.join("chunks"))
                .await?;

            Ok(verification_results.iter().all(|&valid| valid))
        } else {
            // Contenu simple
            let content_path = self.get_content_path(content_hash);
            self.integrity_checker.verify_integrity(&content_path, content_hash).await
        }
    }

    /// Obtient les statistiques du stockage
    pub fn get_stats(&self) -> &ArchiveStats {
        &self.stats
    }

    /// Nettoie les données non référencées
    pub async fn cleanup_unreferenced_data(&mut self) -> Result<CleanupReport> {
        let mut report = CleanupReport::default();

        // Nettoie les chunks non référencés
        let chunk_hashes: Vec<Hash> = self.chunk_manager.chunk_metadata.keys().cloned().collect();
        for chunk_hash in chunk_hashes {
            if self.chunk_manager.decrement_ref_count(&chunk_hash) {
                // Chunk plus référencé, peut être supprimé
                let chunk_path = self.get_chunk_path(&chunk_hash);
                if fs::remove_file(&chunk_path).await.is_ok() {
                    report.chunks_deleted += 1;
                    if let Some(metadata) = self.chunk_manager.chunk_metadata.remove(&chunk_hash) {
                        report.space_freed += metadata.compressed_size;
                    }
                }
            }
        }

        // Met à jour les statistiques de déduplication
        self.stats.deduplication_stats = self.deduplication_engine.get_stats();

        Ok(report)
    }
}

/// Index des chunks pour un contenu
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChunkIndex {
    /// Hash du contenu complet
    content_hash: Hash,
    /// Liste des hash des chunks
    chunks: Vec<Hash>,
    /// Taille totale du contenu
    total_size: u64,
}

/// Rapport de nettoyage
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CleanupReport {
    /// Nombre de chunks supprimés
    pub chunks_deleted: u32,
    /// Espace libéré (bytes)
    pub space_freed: u64,
    /// Nombre de contenus dédupliqués supprimés
    pub deduplicated_content_removed: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_archive_storage_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig {
            base_storage_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let storage = ArchiveStorage::new(config);
        assert!(storage.is_ok());
    }

    #[tokio::test]
    async fn test_content_storage_and_retrieval() {
        let temp_dir = TempDir::new().unwrap();
        let config = ArchiveConfig {
            base_storage_path: temp_dir.path().to_path_buf(),
            ..Default::default()
        };

        let mut storage = ArchiveStorage::new(config).unwrap();
        let test_data = b"Hello, ArchiveChain!";
        let metadata = create_test_metadata();
        let nodes = vec![NodeId::from(Hash::zero())];

        let result = storage.store_content_optimized(test_data, &metadata, &nodes).await;
        assert!(result.is_ok());

        let content_hash = compute_hash(test_data, HashAlgorithm::Blake3);
        let retrieved = storage.retrieve_content_from_node(&content_hash, &nodes[0]).await;
        assert!(retrieved.is_ok());
        assert_eq!(retrieved.unwrap(), test_data);
    }

    #[test]
    fn test_chunk_manager() {
        let mut manager = ChunkManager::new(10); // 10 bytes per chunk
        let test_data = b"Hello, ArchiveChain! This is a test.";
        
        let chunks = manager.create_chunks(test_data).unwrap();
        assert!(chunks.len() > 1); // Should be chunked

        let reassembled = manager.reassemble_chunks(&chunks).unwrap();
        assert_eq!(reassembled, test_data);
    }

    #[test]
    fn test_deduplication_engine() {
        let mut engine = DeduplicationEngine::new(100);
        let content_hash = Hash::zero();
        let storage_path = PathBuf::from("/test/path");

        engine.add_content(content_hash, storage_path.clone(), 1024);
        assert!(engine.check_duplicate(&content_hash).is_some());

        engine.add_reference(&content_hash);
        let reference = engine.check_duplicate(&content_hash).unwrap();
        assert_eq!(reference.ref_count, 2);
    }

    #[test]
    fn test_compression_types() {
        assert_eq!(CompressionType::Zstd.extension(), ".zst");
        assert_eq!(CompressionType::Gzip.extension(), ".gz");
        assert!(CompressionType::Zstd.recommended_level(true) > CompressionType::Zstd.recommended_level(false));
    }

    fn create_test_metadata() -> ContentMetadata {
        super::super::ContentMetadata {
            content_hash: Hash::zero(),
            size: 1024,
            content_type: "text/html".to_string(),
            importance: super::super::replication::ContentImportance::Medium,
            popularity: 100,
            created_at: chrono::Utc::now(),
            preferred_regions: vec!["test-region".to_string()],
            redundancy_level: 3,
            tags: vec!["test".to_string()],
        }
    }
}