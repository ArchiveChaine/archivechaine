//! Types GraphQL supplémentaires pour ArchiveChain
//!
//! Contient les types GraphQL supplémentaires et les utilitaires de conversion.

use async_graphql::{Scalar, ScalarType, InputValueError, InputValueResult, Value};
use serde::{Serialize, Deserialize};

/// Type scalaire personnalisé pour les Hash
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HashType(pub String);

#[Scalar]
impl ScalarType for HashType {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Valide que c'est un hash hexadécimal valide
                if s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()) {
                    Ok(HashType(s))
                } else {
                    Err(InputValueError::custom("Invalid hash format. Must be 64 character hexadecimal string."))
                }
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.clone())
    }
}

impl From<crate::Hash> for HashType {
    fn from(hash: crate::Hash) -> Self {
        HashType(hash.to_string())
    }
}

impl From<HashType> for String {
    fn from(hash: HashType) -> Self {
        hash.0
    }
}

/// Type scalaire pour les adresses de nœuds
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeAddress(pub String);

#[Scalar]
impl ScalarType for NodeAddress {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Valide le format d'adresse de nœud
                if s.len() >= 10 && s.len() <= 100 {
                    Ok(NodeAddress(s))
                } else {
                    Err(InputValueError::custom("Invalid node address format"))
                }
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.clone())
    }
}

/// Type scalaire pour les montants de tokens
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenAmountType {
    pub amount: String,
    pub decimals: u8,
}

#[Scalar]
impl ScalarType for TokenAmountType {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Parse le montant (format: "1.234" ou "1234")
                if let Ok(_) = s.parse::<f64>() {
                    Ok(TokenAmountType {
                        amount: s,
                        decimals: 18, // Décimales par défaut
                    })
                } else {
                    Err(InputValueError::custom("Invalid token amount format"))
                }
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.amount.clone())
    }
}

/// Type scalaire pour les timestamps Unix
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnixTimestamp(pub i64);

#[Scalar]
impl ScalarType for UnixTimestamp {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::Number(n) => {
                if let Some(i) = n.as_i64() {
                    Ok(UnixTimestamp(i))
                } else {
                    Err(InputValueError::custom("Invalid timestamp format"))
                }
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::Number(self.0.into())
    }
}

impl From<chrono::DateTime<chrono::Utc>> for UnixTimestamp {
    fn from(datetime: chrono::DateTime<chrono::Utc>) -> Self {
        UnixTimestamp(datetime.timestamp())
    }
}

impl From<UnixTimestamp> for chrono::DateTime<chrono::Utc> {
    fn from(timestamp: UnixTimestamp) -> Self {
        chrono::DateTime::from_timestamp(timestamp.0, 0)
            .unwrap_or_else(chrono::Utc::now)
    }
}

/// Type scalaire pour les durées
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DurationType(pub String);

#[Scalar]
impl ScalarType for DurationType {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Valide le format de durée (ex: "1h", "30m", "5s")
                if s.is_empty() {
                    return Err(InputValueError::custom("Duration cannot be empty"));
                }
                
                let last_char = s.chars().last().unwrap();
                if !['s', 'm', 'h', 'd'].contains(&last_char) {
                    return Err(InputValueError::custom("Duration must end with s, m, h, or d"));
                }
                
                let number_part = &s[..s.len()-1];
                if let Err(_) = number_part.parse::<u64>() {
                    return Err(InputValueError::custom("Invalid duration format"));
                }
                
                Ok(DurationType(s))
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::String(self.0.clone())
    }
}

/// Type pour les coordonnées géographiques
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeoCoordinates {
    pub latitude: f64,
    pub longitude: f64,
}

#[Scalar]
impl ScalarType for GeoCoordinates {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::Object(obj) => {
                let lat = obj.get("latitude")
                    .and_then(|v| v.as_number())
                    .and_then(|n| n.as_f64())
                    .ok_or_else(|| InputValueError::custom("Missing or invalid latitude"))?;
                
                let lng = obj.get("longitude")
                    .and_then(|v| v.as_number())
                    .and_then(|n| n.as_f64())
                    .ok_or_else(|| InputValueError::custom("Missing or invalid longitude"))?;
                
                if lat < -90.0 || lat > 90.0 {
                    return Err(InputValueError::custom("Latitude must be between -90 and 90"));
                }
                
                if lng < -180.0 || lng > 180.0 {
                    return Err(InputValueError::custom("Longitude must be between -180 and 180"));
                }
                
                Ok(GeoCoordinates {
                    latitude: lat,
                    longitude: lng,
                })
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        let mut map = serde_json::Map::new();
        map.insert("latitude".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(self.latitude).unwrap()));
        map.insert("longitude".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(self.longitude).unwrap()));
        Value::Object(map.into_iter().collect())
    }
}

/// Type pour les versions sémantiques
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SemanticVersion {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
    pub pre_release: Option<String>,
}

#[Scalar]
impl ScalarType for SemanticVersion {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(s) => {
                // Parse le format semver (ex: "1.2.3", "1.2.3-alpha.1")
                let parts: Vec<&str> = s.split('-').collect();
                let version_part = parts[0];
                let pre_release = if parts.len() > 1 { Some(parts[1].to_string()) } else { None };
                
                let version_numbers: Vec<&str> = version_part.split('.').collect();
                if version_numbers.len() != 3 {
                    return Err(InputValueError::custom("Version must have format major.minor.patch"));
                }
                
                let major = version_numbers[0].parse::<u32>()
                    .map_err(|_| InputValueError::custom("Invalid major version"))?;
                let minor = version_numbers[1].parse::<u32>()
                    .map_err(|_| InputValueError::custom("Invalid minor version"))?;
                let patch = version_numbers[2].parse::<u32>()
                    .map_err(|_| InputValueError::custom("Invalid patch version"))?;
                
                Ok(SemanticVersion {
                    major,
                    minor,
                    patch,
                    pre_release,
                })
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        let version_string = if let Some(ref pre) = self.pre_release {
            format!("{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            format!("{}.{}.{}", self.major, self.minor, self.patch)
        };
        Value::String(version_string)
    }
}

impl std::fmt::Display for SemanticVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(ref pre) = self.pre_release {
            write!(f, "{}.{}.{}-{}", self.major, self.minor, self.patch, pre)
        } else {
            write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
        }
    }
}

/// Utilitaires pour la conversion de types
pub struct TypeConverter;

impl TypeConverter {
    /// Convertit un DTO d'archive vers le type GraphQL
    pub fn archive_dto_to_graphql(dto: crate::api::types::ArchiveDto) -> super::schema::Archive {
        super::schema::Archive {
            id: dto.archive_id,
            url: dto.url,
            status: dto.status.into(),
            metadata: super::schema::ArchiveMetadata {
                title: dto.metadata.title,
                description: dto.metadata.description,
                tags: dto.metadata.tags,
                content_type: dto.metadata.mime_type,
                language: dto.metadata.language,
                author: dto.metadata.author,
                published_at: dto.metadata.published_at,
            },
            storage_info: super::schema::StorageInfo {
                replicas: dto.storage_info.replicas as i32,
                locations: dto.storage_info.locations,
                integrity_score: dto.storage_info.integrity_score,
                last_verified: dto.storage_info.last_verified,
            },
            created_at: dto.created_at,
            completed_at: dto.completed_at,
            size: dto.size as i64,
            cost: super::schema::TokenAmount {
                amount: "0.001".to_string(), // TODO: Calculer le vrai coût
                currency: "ARC".to_string(),
            },
        }
    }

    /// Convertit un DTO de nœud vers le type GraphQL
    pub fn node_dto_to_graphql(dto: crate::api::types::NodeInfo) -> super::schema::Node {
        super::schema::Node {
            id: dto.node_id,
            status: dto.status.into(),
            region: dto.region,
            capacity: super::schema::StorageCapacity {
                total: dto.capacity.total as i64,
                used: dto.capacity.used as i64,
                available: dto.capacity.available as i64,
            },
            performance: super::schema::NodePerformance {
                bandwidth: dto.performance.bandwidth as i64,
                latency: dto.performance.latency as i32,
                reliability_score: dto.performance.reliability_score,
            },
            last_seen: dto.last_seen,
        }
    }

    /// Convertit des statistiques réseau vers le type GraphQL
    pub fn network_stats_to_graphql(stats: crate::api::types::NetworkStats) -> super::schema::NetworkStats {
        super::schema::NetworkStats {
            total_nodes: stats.network.total_nodes as i32,
            active_nodes: stats.network.active_nodes as i32,
            total_storage: stats.network.total_storage,
            available_storage: stats.network.available_storage,
            current_block_height: stats.network.current_block_height as i64,
            total_archives: stats.archives.total_archives as i64,
            archives_today: stats.archives.archives_today as i32,
            average_archive_time: stats.performance.average_archive_time,
            success_rate: stats.performance.success_rate,
        }
    }
}

/// Macros pour créer facilement des types GraphQL
#[macro_export]
macro_rules! create_graphql_connection {
    ($edge_type:ident, $node_type:ident) => {
        #[derive(async_graphql::SimpleObject)]
        pub struct $edge_type {
            pub node: $node_type,
            pub cursor: String,
        }
    };
}

#[macro_export]
macro_rules! create_graphql_payload {
    ($payload_type:ident, $data_type:ident) => {
        #[derive(async_graphql::SimpleObject)]
        pub struct $payload_type {
            pub data: Option<$data_type>,
            pub errors: Vec<String>,
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_graphql::Value;

    #[test]
    fn test_hash_type_valid() {
        let valid_hash = "a".repeat(64);
        let result = HashType::parse(Value::String(valid_hash.clone()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, valid_hash);
    }

    #[test]
    fn test_hash_type_invalid() {
        let invalid_hash = "invalid";
        let result = HashType::parse(Value::String(invalid_hash.to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_token_amount_valid() {
        let amount = "123.456";
        let result = TokenAmountType::parse(Value::String(amount.to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().amount, amount);
    }

    #[test]
    fn test_token_amount_invalid() {
        let invalid_amount = "not_a_number";
        let result = TokenAmountType::parse(Value::String(invalid_amount.to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_unix_timestamp() {
        let timestamp = 1640995200i64; // 2022-01-01 00:00:00 UTC
        let result = UnixTimestamp::parse(Value::Number(timestamp.into()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, timestamp);
    }

    #[test]
    fn test_duration_type_valid() {
        let duration = "1h";
        let result = DurationType::parse(Value::String(duration.to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, duration);
    }

    #[test]
    fn test_duration_type_invalid() {
        let invalid_duration = "1x";
        let result = DurationType::parse(Value::String(invalid_duration.to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_geo_coordinates_valid() {
        let mut map = std::collections::HashMap::new();
        map.insert("latitude".to_string(), Value::Number(45.5.into()));
        map.insert("longitude".to_string(), Value::Number(-73.6.into()));
        
        let result = GeoCoordinates::parse(Value::Object(map));
        assert!(result.is_ok());
        
        let coords = result.unwrap();
        assert_eq!(coords.latitude, 45.5);
        assert_eq!(coords.longitude, -73.6);
    }

    #[test]
    fn test_geo_coordinates_invalid_range() {
        let mut map = std::collections::HashMap::new();
        map.insert("latitude".to_string(), Value::Number(100.0.into())); // Invalid: > 90
        map.insert("longitude".to_string(), Value::Number(0.0.into()));
        
        let result = GeoCoordinates::parse(Value::Object(map));
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_version_valid() {
        let version = "1.2.3";
        let result = SemanticVersion::parse(Value::String(version.to_string()));
        assert!(result.is_ok());
        
        let semver = result.unwrap();
        assert_eq!(semver.major, 1);
        assert_eq!(semver.minor, 2);
        assert_eq!(semver.patch, 3);
        assert!(semver.pre_release.is_none());
    }

    #[test]
    fn test_semantic_version_with_prerelease() {
        let version = "1.2.3-alpha.1";
        let result = SemanticVersion::parse(Value::String(version.to_string()));
        assert!(result.is_ok());
        
        let semver = result.unwrap();
        assert_eq!(semver.major, 1);
        assert_eq!(semver.minor, 2);
        assert_eq!(semver.patch, 3);
        assert_eq!(semver.pre_release, Some("alpha.1".to_string()));
    }

    #[test]
    fn test_semantic_version_invalid() {
        let invalid_version = "1.2";
        let result = SemanticVersion::parse(Value::String(invalid_version.to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_semantic_version_display() {
        let semver = SemanticVersion {
            major: 1,
            minor: 2,
            patch: 3,
            pre_release: Some("alpha.1".to_string()),
        };
        assert_eq!(semver.to_string(), "1.2.3-alpha.1");

        let semver_stable = SemanticVersion {
            major: 1,
            minor: 0,
            patch: 0,
            pre_release: None,
        };
        assert_eq!(semver_stable.to_string(), "1.0.0");
    }

    #[test]
    fn test_node_address_valid() {
        let address = "node_1234567890";
        let result = NodeAddress::parse(Value::String(address.to_string()));
        assert!(result.is_ok());
        assert_eq!(result.unwrap().0, address);
    }

    #[test]
    fn test_node_address_invalid() {
        let short_address = "abc";
        let result = NodeAddress::parse(Value::String(short_address.to_string()));
        assert!(result.is_err());
    }
}