//! Application Binary Interface (ABI) pour les smart contracts ArchiveChain

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::crypto::Hash;
use crate::contracts::{ContractError, ContractResult};

/// Type de données supporté par l'ABI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AbiType {
    /// Entier non signé 8 bits
    U8,
    /// Entier non signé 16 bits
    U16,
    /// Entier non signé 32 bits
    U32,
    /// Entier non signé 64 bits
    U64,
    /// Entier signé 8 bits
    I8,
    /// Entier signé 16 bits
    I16,
    /// Entier signé 32 bits
    I32,
    /// Entier signé 64 bits
    I64,
    /// Booléen
    Bool,
    /// Hash (32 bytes)
    Hash,
    /// Adresse (32 bytes)
    Address,
    /// Chaîne de caractères
    String,
    /// Tableau de bytes
    Bytes,
    /// Tableau d'un type donné
    Array(Box<AbiType>),
    /// Tuple de types
    Tuple(Vec<AbiType>),
    /// Structure avec champs nommés
    Struct {
        name: String,
        fields: Vec<AbiField>,
    },
}

/// Champ d'une structure ABI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiField {
    pub name: String,
    pub type_info: AbiType,
    pub description: Option<String>,
}

/// Paramètre d'une fonction
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiParameter {
    pub name: String,
    pub type_info: AbiType,
    pub description: Option<String>,
}

/// Fonction d'un contrat
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiFunction {
    pub name: String,
    pub inputs: Vec<AbiParameter>,
    pub outputs: Vec<AbiParameter>,
    pub description: Option<String>,
    pub payable: bool,
    pub view_only: bool,
}

/// Event d'un contrat
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbiEvent {
    pub name: String,
    pub inputs: Vec<AbiParameter>,
    pub description: Option<String>,
    pub indexed_count: usize,
}

/// Interface complète d'un smart contract
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractAbi {
    /// Nom du contrat
    pub name: String,
    /// Version de l'ABI
    pub version: String,
    /// Description du contrat
    pub description: Option<String>,
    /// Fonctions du contrat
    pub functions: Vec<AbiFunction>,
    /// Events du contrat
    pub events: Vec<AbiEvent>,
    /// Types personnalisés
    pub types: Vec<AbiType>,
    /// Hash de l'ABI pour vérification
    pub hash: Hash,
}

/// Appel de fonction encodé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractCall {
    /// Nom de la fonction
    pub function_name: String,
    /// Arguments encodés
    pub args: Vec<AbiValue>,
    /// Valeur envoyée (en tokens ARC)
    pub value: u64,
    /// Gas limite
    pub gas_limit: u64,
}

/// Event émis par un contrat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    /// Nom de l'event
    pub name: String,
    /// Données de l'event
    pub data: Vec<AbiValue>,
    /// Topics indexés
    pub topics: Vec<Hash>,
    /// Adresse du contrat émetteur
    pub contract_address: Hash,
    /// Hash de la transaction
    pub transaction_hash: Hash,
    /// Numéro du bloc
    pub block_number: u64,
}

/// Valeur typée pour l'ABI
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AbiValue {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    Bool(bool),
    Hash(Hash),
    Address(Hash),
    String(String),
    Bytes(Vec<u8>),
    Array(Vec<AbiValue>),
    Tuple(Vec<AbiValue>),
    Struct {
        name: String,
        fields: HashMap<String, AbiValue>,
    },
}

/// Erreur spécifique à l'ABI
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum ContractError {
    #[error("Type ABI invalide: {message}")]
    InvalidType { message: String },

    #[error("Encodage échoué: {message}")]
    EncodingFailed { message: String },

    #[error("Décodage échoué: {message}")]
    DecodingFailed { message: String },

    #[error("Fonction non trouvée: {function}")]
    FunctionNotFound { function: String },

    #[error("Event non trouvé: {event}")]
    EventNotFound { event: String },

    #[error("Nombre d'arguments incorrect: attendu {expected}, reçu {actual}")]
    ArgumentCountMismatch { expected: usize, actual: usize },

    #[error("Type d'argument incorrect: attendu {expected:?}, reçu {actual:?}")]
    ArgumentTypeMismatch { expected: AbiType, actual: AbiType },
}

impl ContractAbi {
    /// Crée un nouvel ABI
    pub fn new(name: String, version: String) -> Self {
        let mut abi = Self {
            name,
            version,
            description: None,
            functions: Vec::new(),
            events: Vec::new(),
            types: Vec::new(),
            hash: Hash::zero(),
        };
        abi.hash = abi.calculate_hash();
        abi
    }

    /// Ajoute une fonction à l'ABI
    pub fn add_function(&mut self, function: AbiFunction) {
        self.functions.push(function);
        self.hash = self.calculate_hash();
    }

    /// Ajoute un event à l'ABI
    pub fn add_event(&mut self, event: AbiEvent) {
        self.events.push(event);
        self.hash = self.calculate_hash();
    }

    /// Trouve une fonction par son nom
    pub fn get_function(&self, name: &str) -> ContractResult<&AbiFunction> {
        self.functions.iter()
            .find(|f| f.name == name)
            .ok_or(ContractError::FunctionNotFound { 
                function: name.to_string() 
            })
    }

    /// Trouve un event par son nom
    pub fn get_event(&self, name: &str) -> ContractResult<&AbiEvent> {
        self.events.iter()
            .find(|e| e.name == name)
            .ok_or(ContractError::EventNotFound { 
                event: name.to_string() 
            })
    }

    /// Encode un appel de fonction
    pub fn encode_function_call(
        &self,
        function_name: &str,
        args: &[AbiValue],
    ) -> ContractResult<Vec<u8>> {
        let function = self.get_function(function_name)?;
        
        if args.len() != function.inputs.len() {
            return Err(ContractError::ArgumentCountMismatch {
                expected: function.inputs.len(),
                actual: args.len(),
            });
        }

        // Vérifie les types d'arguments
        for (i, (arg, param)) in args.iter().zip(&function.inputs).enumerate() {
            if !self.value_matches_type(arg, &param.type_info) {
                return Err(ContractError::ArgumentTypeMismatch {
                    expected: param.type_info.clone(),
                    actual: self.value_to_type(arg),
                });
            }
        }

        // Encode les arguments
        let mut encoded = Vec::new();
        
        // Sélecteur de fonction (4 premiers bytes du hash du nom)
        let function_selector = self.function_selector(function_name);
        encoded.extend_from_slice(&function_selector);
        
        // Encode chaque argument
        for arg in args {
            let arg_bytes = self.encode_value(arg)?;
            encoded.extend_from_slice(&arg_bytes);
        }

        Ok(encoded)
    }

    /// Décode le résultat d'une fonction
    pub fn decode_function_result(
        &self,
        function_name: &str,
        data: &[u8],
    ) -> ContractResult<Vec<AbiValue>> {
        let function = self.get_function(function_name)?;
        
        let mut offset = 0;
        let mut results = Vec::new();
        
        for output in &function.outputs {
            let (value, new_offset) = self.decode_value(&output.type_info, data, offset)?;
            results.push(value);
            offset = new_offset;
        }

        Ok(results)
    }

    /// Encode un event
    pub fn encode_event(
        &self,
        event_name: &str,
        args: &[AbiValue],
    ) -> ContractResult<(Vec<Hash>, Vec<u8>)> {
        let event = self.get_event(event_name)?;
        
        if args.len() != event.inputs.len() {
            return Err(ContractError::ArgumentCountMismatch {
                expected: event.inputs.len(),
                actual: args.len(),
            });
        }

        let mut topics = Vec::new();
        let mut data = Vec::new();

        // Premier topic est toujours le hash de l'event
        topics.push(self.event_selector(event_name));

        // Encode les arguments indexés comme topics et les autres comme data
        for (i, (arg, param)) in args.iter().zip(&event.inputs).enumerate() {
            if i < event.indexed_count {
                // Argument indexé -> topic
                let topic_bytes = self.encode_value(arg)?;
                let topic_hash = crate::crypto::compute_blake3(&topic_bytes);
                topics.push(topic_hash);
            } else {
                // Argument non indexé -> data
                let arg_bytes = self.encode_value(arg)?;
                data.extend_from_slice(&arg_bytes);
            }
        }

        Ok((topics, data))
    }

    /// Calcule le sélecteur d'une fonction (4 premiers bytes du hash)
    fn function_selector(&self, function_name: &str) -> [u8; 4] {
        let hash = crate::crypto::compute_blake3(function_name.as_bytes());
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&hash.as_bytes()[..4]);
        selector
    }

    /// Calcule le sélecteur d'un event
    fn event_selector(&self, event_name: &str) -> Hash {
        crate::crypto::compute_blake3(event_name.as_bytes())
    }

    /// Encode une valeur ABI en bytes
    fn encode_value(&self, value: &AbiValue) -> ContractResult<Vec<u8>> {
        match value {
            AbiValue::U8(v) => Ok(vec![*v]),
            AbiValue::U16(v) => Ok(v.to_le_bytes().to_vec()),
            AbiValue::U32(v) => Ok(v.to_le_bytes().to_vec()),
            AbiValue::U64(v) => Ok(v.to_le_bytes().to_vec()),
            AbiValue::I8(v) => Ok(vec![*v as u8]),
            AbiValue::I16(v) => Ok(v.to_le_bytes().to_vec()),
            AbiValue::I32(v) => Ok(v.to_le_bytes().to_vec()),
            AbiValue::I64(v) => Ok(v.to_le_bytes().to_vec()),
            AbiValue::Bool(v) => Ok(vec![if *v { 1 } else { 0 }]),
            AbiValue::Hash(h) => Ok(h.as_bytes().to_vec()),
            AbiValue::Address(a) => Ok(a.as_bytes().to_vec()),
            AbiValue::String(s) => {
                let bytes = s.as_bytes();
                let mut encoded = (bytes.len() as u32).to_le_bytes().to_vec();
                encoded.extend_from_slice(bytes);
                Ok(encoded)
            }
            AbiValue::Bytes(b) => {
                let mut encoded = (b.len() as u32).to_le_bytes().to_vec();
                encoded.extend_from_slice(b);
                Ok(encoded)
            }
            AbiValue::Array(arr) => {
                let mut encoded = (arr.len() as u32).to_le_bytes().to_vec();
                for item in arr {
                    let item_bytes = self.encode_value(item)?;
                    encoded.extend_from_slice(&item_bytes);
                }
                Ok(encoded)
            }
            AbiValue::Tuple(tuple) => {
                let mut encoded = Vec::new();
                for item in tuple {
                    let item_bytes = self.encode_value(item)?;
                    encoded.extend_from_slice(&item_bytes);
                }
                Ok(encoded)
            }
            AbiValue::Struct { fields, .. } => {
                let mut encoded = Vec::new();
                // Encode les champs dans l'ordre alphabétique pour la consistance
                let mut sorted_fields: Vec<_> = fields.iter().collect();
                sorted_fields.sort_by_key(|(name, _)| *name);
                
                for (_, value) in sorted_fields {
                    let field_bytes = self.encode_value(value)?;
                    encoded.extend_from_slice(&field_bytes);
                }
                Ok(encoded)
            }
        }
    }

    /// Décode une valeur ABI depuis des bytes
    fn decode_value(&self, abi_type: &AbiType, data: &[u8], offset: usize) -> ContractResult<(AbiValue, usize)> {
        match abi_type {
            AbiType::U8 => {
                if offset >= data.len() {
                    return Err(ContractError::DecodingFailed {
                        message: "Not enough data for U8".to_string(),
                    });
                }
                Ok((AbiValue::U8(data[offset]), offset + 1))
            }
            AbiType::U32 => {
                if offset + 4 > data.len() {
                    return Err(ContractError::DecodingFailed {
                        message: "Not enough data for U32".to_string(),
                    });
                }
                let mut bytes = [0u8; 4];
                bytes.copy_from_slice(&data[offset..offset + 4]);
                Ok((AbiValue::U32(u32::from_le_bytes(bytes)), offset + 4))
            }
            AbiType::String => {
                if offset + 4 > data.len() {
                    return Err(ContractError::DecodingFailed {
                        message: "Not enough data for string length".to_string(),
                    });
                }
                
                let mut len_bytes = [0u8; 4];
                len_bytes.copy_from_slice(&data[offset..offset + 4]);
                let len = u32::from_le_bytes(len_bytes) as usize;
                
                if offset + 4 + len > data.len() {
                    return Err(ContractError::DecodingFailed {
                        message: "Not enough data for string content".to_string(),
                    });
                }
                
                let string_bytes = &data[offset + 4..offset + 4 + len];
                let string = String::from_utf8(string_bytes.to_vec())
                    .map_err(|e| ContractError::DecodingFailed {
                        message: format!("Invalid UTF-8: {}", e),
                    })?;
                
                Ok((AbiValue::String(string), offset + 4 + len))
            }
            // Autres types...
            _ => Err(ContractError::DecodingFailed {
                message: format!("Decoding not implemented for type: {:?}", abi_type),
            }),
        }
    }

    /// Vérifie si une valeur correspond à un type ABI
    fn value_matches_type(&self, value: &AbiValue, abi_type: &AbiType) -> bool {
        match (value, abi_type) {
            (AbiValue::U8(_), AbiType::U8) => true,
            (AbiValue::U16(_), AbiType::U16) => true,
            (AbiValue::U32(_), AbiType::U32) => true,
            (AbiValue::U64(_), AbiType::U64) => true,
            (AbiValue::Bool(_), AbiType::Bool) => true,
            (AbiValue::Hash(_), AbiType::Hash) => true,
            (AbiValue::Address(_), AbiType::Address) => true,
            (AbiValue::String(_), AbiType::String) => true,
            (AbiValue::Bytes(_), AbiType::Bytes) => true,
            _ => false,
        }
    }

    /// Obtient le type ABI d'une valeur
    fn value_to_type(&self, value: &AbiValue) -> AbiType {
        match value {
            AbiValue::U8(_) => AbiType::U8,
            AbiValue::U16(_) => AbiType::U16,
            AbiValue::U32(_) => AbiType::U32,
            AbiValue::U64(_) => AbiType::U64,
            AbiValue::I8(_) => AbiType::I8,
            AbiValue::I16(_) => AbiType::I16,
            AbiValue::I32(_) => AbiType::I32,
            AbiValue::I64(_) => AbiType::I64,
            AbiValue::Bool(_) => AbiType::Bool,
            AbiValue::Hash(_) => AbiType::Hash,
            AbiValue::Address(_) => AbiType::Address,
            AbiValue::String(_) => AbiType::String,
            AbiValue::Bytes(_) => AbiType::Bytes,
            AbiValue::Array(arr) => {
                if let Some(first) = arr.first() {
                    AbiType::Array(Box::new(self.value_to_type(first)))
                } else {
                    AbiType::Array(Box::new(AbiType::U8))
                }
            }
            AbiValue::Tuple(tuple) => {
                AbiType::Tuple(tuple.iter().map(|v| self.value_to_type(v)).collect())
            }
            AbiValue::Struct { name, fields } => {
                let abi_fields = fields.iter().map(|(field_name, field_value)| {
                    AbiField {
                        name: field_name.clone(),
                        type_info: self.value_to_type(field_value),
                        description: None,
                    }
                }).collect();
                
                AbiType::Struct {
                    name: name.clone(),
                    fields: abi_fields,
                }
            }
        }
    }

    /// Calcule le hash de l'ABI
    fn calculate_hash(&self) -> Hash {
        let serialized = bincode::serialize(self).unwrap_or_default();
        crate::crypto::compute_blake3(&serialized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi_creation() {
        let abi = ContractAbi::new("TestContract".to_string(), "1.0.0".to_string());
        assert_eq!(abi.name, "TestContract");
        assert_eq!(abi.version, "1.0.0");
        assert!(!abi.hash.is_zero());
    }

    #[test]
    fn test_function_addition() {
        let mut abi = ContractAbi::new("TestContract".to_string(), "1.0.0".to_string());
        
        let function = AbiFunction {
            name: "transfer".to_string(),
            inputs: vec![
                AbiParameter {
                    name: "to".to_string(),
                    type_info: AbiType::Address,
                    description: None,
                },
                AbiParameter {
                    name: "amount".to_string(),
                    type_info: AbiType::U64,
                    description: None,
                },
            ],
            outputs: vec![
                AbiParameter {
                    name: "success".to_string(),
                    type_info: AbiType::Bool,
                    description: None,
                },
            ],
            description: None,
            payable: false,
            view_only: false,
        };
        
        abi.add_function(function);
        assert_eq!(abi.functions.len(), 1);
        
        let found_function = abi.get_function("transfer");
        assert!(found_function.is_ok());
    }

    #[test]
    fn test_value_encoding() {
        let abi = ContractAbi::new("Test".to_string(), "1.0".to_string());
        
        let value = AbiValue::U32(42);
        let encoded = abi.encode_value(&value).unwrap();
        assert_eq!(encoded, vec![42, 0, 0, 0]); // Little endian
        
        let string_value = AbiValue::String("hello".to_string());
        let encoded_string = abi.encode_value(&string_value).unwrap();
        assert_eq!(encoded_string[..4], [5, 0, 0, 0]); // Length prefix
        assert_eq!(&encoded_string[4..], b"hello");
    }

    #[test]
    fn test_function_call_encoding() {
        let mut abi = ContractAbi::new("TestContract".to_string(), "1.0.0".to_string());
        
        let function = AbiFunction {
            name: "test".to_string(),
            inputs: vec![
                AbiParameter {
                    name: "value".to_string(),
                    type_info: AbiType::U32,
                    description: None,
                },
            ],
            outputs: vec![],
            description: None,
            payable: false,
            view_only: false,
        };
        
        abi.add_function(function);
        
        let args = vec![AbiValue::U32(123)];
        let encoded = abi.encode_function_call("test", &args).unwrap();
        
        // Vérifie que l'encodage contient le sélecteur (4 bytes) + arguments
        assert!(encoded.len() >= 8); // 4 bytes selector + 4 bytes for u32
    }

    #[test]
    fn test_type_matching() {
        let abi = ContractAbi::new("Test".to_string(), "1.0".to_string());
        
        assert!(abi.value_matches_type(&AbiValue::U32(42), &AbiType::U32));
        assert!(!abi.value_matches_type(&AbiValue::U32(42), &AbiType::U64));
        assert!(abi.value_matches_type(&AbiValue::String("test".to_string()), &AbiType::String));
    }
}