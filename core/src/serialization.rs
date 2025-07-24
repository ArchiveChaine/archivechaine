//! Module de sérialisation pour ArchiveChain
//! 
//! Fournit des fonctions de sérialisation/désérialisation avec bincode et CBOR

use serde::{Serialize, Deserialize};
use crate::error::{SerializationError, Result};

/// Formats de sérialisation supportés
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationFormat {
    /// Bincode - Format binaire compact et rapide
    Bincode,
    /// CBOR - Format binaire standardisé pour l'interopérabilité
    Cbor,
    /// JSON - Format texte pour le debug et APIs
    Json,
}

/// Trait pour les objets sérialisables
pub trait Serializable: Serialize + for<'de> Deserialize<'de> {
    /// Sérialise l'objet dans le format spécifié
    fn serialize(&self, format: SerializationFormat) -> Result<Vec<u8>> {
        serialize_with_format(self, format)
    }

    /// Désérialise un objet depuis les bytes
    fn deserialize(data: &[u8], format: SerializationFormat) -> Result<Self>
    where
        Self: Sized,
    {
        deserialize_with_format(data, format)
    }

    /// Calcule la taille sérialisée de l'objet
    fn serialized_size(&self, format: SerializationFormat) -> Result<usize> {
        match format {
            SerializationFormat::Bincode => {
                Ok(bincode::serialized_size(self)? as usize)
            }
            SerializationFormat::Cbor => {
                let data = self.serialize(format)?;
                Ok(data.len())
            }
            SerializationFormat::Json => {
                let data = self.serialize(format)?;
                Ok(data.len())
            }
        }
    }
}

/// Sérialise un objet avec le format spécifié
pub fn serialize_with_format<T: Serialize>(
    obj: &T,
    format: SerializationFormat,
) -> Result<Vec<u8>> {
    match format {
        SerializationFormat::Bincode => {
            Ok(bincode::serialize(obj)?)
        }
        SerializationFormat::Cbor => {
            let mut buffer = Vec::new();
            cbor4ii::serde::to_writer(obj, &mut buffer)?;
            Ok(buffer)
        }
        SerializationFormat::Json => {
            let json_str = serde_json::to_string(obj)?;
            Ok(json_str.into_bytes())
        }
    }
}

/// Désérialise un objet depuis les bytes avec le format spécifié
pub fn deserialize_with_format<T: for<'de> Deserialize<'de>>(
    data: &[u8],
    format: SerializationFormat,
) -> Result<T> {
    match format {
        SerializationFormat::Bincode => {
            Ok(bincode::deserialize(data)?)
        }
        SerializationFormat::Cbor => {
            Ok(cbor4ii::serde::from_slice(data)?)
        }
        SerializationFormat::Json => {
            let json_str = std::str::from_utf8(data)
                .map_err(|_| SerializationError::UnsupportedFormat {
                    format: "Invalid UTF-8 for JSON".to_string(),
                })?;
            Ok(serde_json::from_str(json_str)?)
        }
    }
}

/// Compresse des données avec différents algorithmes
pub fn compress_data(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Gzip => {
            use std::io::Write;
            let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            encoder.write_all(data)
                .map_err(|e| SerializationError::Cbor(e.to_string()))?;
            encoder.finish()
                .map_err(|e| SerializationError::Cbor(e.to_string()))
        }
        CompressionAlgorithm::Zstd => {
            zstd::bulk::compress(data, 3)
                .map_err(|e| SerializationError::Cbor(e.to_string()))
        }
    }
}

/// Décompresse des données
pub fn decompress_data(data: &[u8], algorithm: CompressionAlgorithm) -> Result<Vec<u8>> {
    match algorithm {
        CompressionAlgorithm::None => Ok(data.to_vec()),
        CompressionAlgorithm::Gzip => {
            use std::io::Read;
            let mut decoder = flate2::read::GzDecoder::new(data);
            let mut result = Vec::new();
            decoder.read_to_end(&mut result)
                .map_err(|e| SerializationError::Cbor(e.to_string()))?;
            Ok(result)
        }
        CompressionAlgorithm::Zstd => {
            zstd::bulk::decompress(data, 1024 * 1024 * 100) // Max 100MB
                .map_err(|e| SerializationError::Cbor(e.to_string()))
        }
    }
}

/// Algorithmes de compression supportés
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionAlgorithm {
    /// Pas de compression
    None,
    /// Compression Gzip
    Gzip,
    /// Compression Zstandard
    Zstd,
}

/// Wrapper pour données sérialisées avec métadonnées
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedData {
    /// Format de sérialisation utilisé
    pub format: SerializationFormat,
    /// Algorithme de compression utilisé
    pub compression: CompressionAlgorithm,
    /// Données sérialisées (potentiellement compressées)
    pub data: Vec<u8>,
    /// Taille originale avant compression
    pub original_size: usize,
    /// Checksum des données
    pub checksum: crate::crypto::Hash,
    /// Version du format
    pub version: u32,
}

impl SerializedData {
    /// Crée des données sérialisées à partir d'un objet
    pub fn from_object<T: Serialize>(
        obj: &T,
        format: SerializationFormat,
        compression: CompressionAlgorithm,
    ) -> Result<Self> {
        // Sérialise l'objet
        let serialized = serialize_with_format(obj, format)?;
        let original_size = serialized.len();
        
        // Compresse si nécessaire
        let compressed = compress_data(&serialized, compression)?;
        
        // Calcule le checksum des données compressées
        let checksum = crate::crypto::compute_hash(&compressed, crate::crypto::HashAlgorithm::Blake3);
        
        Ok(Self {
            format,
            compression,
            data: compressed,
            original_size,
            checksum,
            version: 1,
        })
    }

    /// Désérialise vers un objet
    pub fn to_object<T: for<'de> Deserialize<'de>>(&self) -> Result<T> {
        // Vérifie le checksum
        let calculated_checksum = crate::crypto::compute_hash(&self.data, crate::crypto::HashAlgorithm::Blake3);
        if calculated_checksum != self.checksum {
            return Err(SerializationError::Cbor("Checksum mismatch".to_string()).into());
        }

        // Décompresse si nécessaire
        let decompressed = decompress_data(&self.data, self.compression)?;
        
        // Vérifie la taille
        if decompressed.len() != self.original_size {
            return Err(SerializationError::Cbor("Size mismatch after decompression".to_string()).into());
        }

        // Désérialise
        deserialize_with_format(&decompressed, self.format)
    }

    /// Obtient le ratio de compression
    pub fn compression_ratio(&self) -> f64 {
        if self.original_size == 0 {
            1.0
        } else {
            self.data.len() as f64 / self.original_size as f64
        }
    }

    /// Obtient les bytes économisés
    pub fn bytes_saved(&self) -> usize {
        self.original_size.saturating_sub(self.data.len())
    }
}

/// Utilitaires pour la sérialisation de blockchain
pub mod blockchain_serialization {
    use super::*;
    use crate::block::Block;
    use crate::transaction::Transaction;

    /// Sérialise un bloc pour le stockage
    pub fn serialize_block(block: &Block) -> Result<SerializedData> {
        SerializedData::from_object(
            block,
            SerializationFormat::Bincode,
            CompressionAlgorithm::Zstd,
        )
    }

    /// Désérialise un bloc depuis le stockage
    pub fn deserialize_block(data: &SerializedData) -> Result<Block> {
        data.to_object()
    }

    /// Sérialise une transaction pour le réseau
    pub fn serialize_transaction(tx: &Transaction) -> Result<Vec<u8>> {
        serialize_with_format(tx, SerializationFormat::Bincode)
    }

    /// Désérialise une transaction depuis le réseau
    pub fn deserialize_transaction(data: &[u8]) -> Result<Transaction> {
        deserialize_with_format(data, SerializationFormat::Bincode)
    }

    /// Sérialise pour l'API JSON
    pub fn serialize_for_api<T: Serialize>(obj: &T) -> Result<String> {
        let data = serialize_with_format(obj, SerializationFormat::Json)?;
        Ok(String::from_utf8(data)
            .map_err(|_| SerializationError::UnsupportedFormat {
                format: "Invalid UTF-8".to_string(),
            })?)
    }

    /// Batch de sérialisation pour plusieurs objets
    pub fn serialize_batch<T: Serialize>(
        objects: &[T],
        format: SerializationFormat,
    ) -> Result<Vec<Vec<u8>>> {
        objects
            .iter()
            .map(|obj| serialize_with_format(obj, format))
            .collect()
    }
}

// Implémente Serializable pour nos types principaux
impl Serializable for crate::block::Block {}
impl Serializable for crate::transaction::Transaction {}
impl Serializable for crate::block::ArchiveBlock {}
impl Serializable for crate::crypto::Hash {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        name: String,
        value: u64,
        data: Vec<u8>,
    }

    impl Serializable for TestStruct {}

    fn create_test_object() -> TestStruct {
        TestStruct {
            name: "test".to_string(),
            value: 42,
            data: vec![1, 2, 3, 4, 5],
        }
    }

    #[test]
    fn test_bincode_serialization() {
        let obj = create_test_object();
        let data = obj.serialize(SerializationFormat::Bincode).unwrap();
        let deserialized: TestStruct = TestStruct::deserialize(&data, SerializationFormat::Bincode).unwrap();
        assert_eq!(obj, deserialized);
    }

    #[test]
    fn test_cbor_serialization() {
        let obj = create_test_object();
        let data = obj.serialize(SerializationFormat::Cbor).unwrap();
        let deserialized: TestStruct = TestStruct::deserialize(&data, SerializationFormat::Cbor).unwrap();
        assert_eq!(obj, deserialized);
    }

    #[test]
    fn test_json_serialization() {
        let obj = create_test_object();
        let data = obj.serialize(SerializationFormat::Json).unwrap();
        let deserialized: TestStruct = TestStruct::deserialize(&data, SerializationFormat::Json).unwrap();
        assert_eq!(obj, deserialized);
    }

    #[test]
    fn test_serialized_data_wrapper() {
        let obj = create_test_object();
        let wrapped = SerializedData::from_object(
            &obj,
            SerializationFormat::Bincode,
            CompressionAlgorithm::Gzip,
        ).unwrap();
        
        assert_eq!(wrapped.format, SerializationFormat::Bincode);
        assert_eq!(wrapped.compression, CompressionAlgorithm::Gzip);
        assert!(wrapped.compression_ratio() <= 1.0);
        
        let recovered: TestStruct = wrapped.to_object().unwrap();
        assert_eq!(obj, recovered);
    }

    #[test]
    fn test_compression() {
        let data = b"Hello, World! This is a test string for compression.".repeat(100);
        
        let compressed = compress_data(&data, CompressionAlgorithm::Gzip).unwrap();
        assert!(compressed.len() < data.len());
        
        let decompressed = decompress_data(&compressed, CompressionAlgorithm::Gzip).unwrap();
        assert_eq!(data, decompressed);
    }

    #[test]
    fn test_serialized_size() {
        let obj = create_test_object();
        let size_bincode = obj.serialized_size(SerializationFormat::Bincode).unwrap();
        let size_cbor = obj.serialized_size(SerializationFormat::Cbor).unwrap();
        let size_json = obj.serialized_size(SerializationFormat::Json).unwrap();
        
        // Bincode should be most compact, JSON least compact
        assert!(size_bincode <= size_cbor);
        assert!(size_cbor < size_json);
    }

    #[test]
    fn test_blockchain_serialization() {
        use crate::block::{BlockBuilder, Block};
        use crate::crypto::HashAlgorithm;
        
        let block = BlockBuilder::new(0, Hash::zero(), HashAlgorithm::Blake3)
            .build()
            .unwrap();
        
        let serialized = blockchain_serialization::serialize_block(&block).unwrap();
        let deserialized = blockchain_serialization::deserialize_block(&serialized).unwrap();
        
        assert_eq!(block.height(), deserialized.height());
        assert_eq!(block.hash(), deserialized.hash());
    }
}