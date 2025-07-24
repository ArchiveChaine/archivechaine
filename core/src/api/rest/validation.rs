//! Module de validation pour l'API REST ArchiveChain
//!
//! Contient les validateurs pour tous les types de requêtes REST.

use crate::api::{ApiError, ApiResult};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use url::Url;

/// Trait pour la validation des données d'entrée
pub trait Validator {
    type Error;
    
    fn validate(&self) -> Result<(), Self::Error>;
}

/// Erreur de validation avec détails
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub field: String,
    pub code: String,
    pub message: String,
    pub value: Option<serde_json::Value>,
}

impl ValidationError {
    pub fn new(field: &str, code: &str, message: &str) -> Self {
        Self {
            field: field.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            value: None,
        }
    }

    pub fn with_value(field: &str, code: &str, message: &str, value: serde_json::Value) -> Self {
        Self {
            field: field.to_string(),
            code: code.to_string(),
            message: message.to_string(),
            value: Some(value),
        }
    }
}

/// Résultat de validation avec erreurs détaillées
pub type ValidationResult = Result<(), Vec<ValidationError>>;

/// Validateur pour les URLs
pub struct UrlValidator;

impl UrlValidator {
    /// Valide qu'une chaîne est une URL valide
    pub fn validate_url(url: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if url.trim().is_empty() {
            errors.push(ValidationError::new("url", "required", "URL is required"));
            return Err(errors);
        }

        match Url::parse(url) {
            Ok(parsed_url) => {
                // Vérifie que le schéma est supporté
                if !["http", "https"].contains(&parsed_url.scheme()) {
                    errors.push(ValidationError::new(
                        "url", 
                        "invalid_scheme", 
                        "Only HTTP and HTTPS URLs are supported"
                    ));
                }

                // Vérifie qu'il y a un host
                if parsed_url.host_str().is_none() {
                    errors.push(ValidationError::new(
                        "url", 
                        "missing_host", 
                        "URL must contain a valid host"
                    ));
                }

                // Vérifie les domaines bloqués
                if let Some(host) = parsed_url.host_str() {
                    if Self::is_blocked_domain(host) {
                        errors.push(ValidationError::new(
                            "url", 
                            "blocked_domain", 
                            "This domain is not allowed"
                        ));
                    }
                }
            }
            Err(_) => {
                errors.push(ValidationError::new("url", "invalid_format", "Invalid URL format"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Vérifie si un domaine est bloqué
    fn is_blocked_domain(host: &str) -> bool {
        let blocked_domains = [
            "localhost",
            "127.0.0.1",
            "0.0.0.0",
            "169.254.0.0", // Link-local
            "10.0.0.0",    // Private networks
            "172.16.0.0",
            "192.168.0.0",
        ];

        blocked_domains.iter().any(|&blocked| host.starts_with(blocked))
    }

    /// Valide une liste d'URLs
    pub fn validate_urls(urls: &[String]) -> ValidationResult {
        let mut all_errors = Vec::new();

        for (index, url) in urls.iter().enumerate() {
            if let Err(mut errors) = Self::validate_url(url) {
                // Préfixe le champ avec l'index
                for error in &mut errors {
                    error.field = format!("urls[{}].{}", index, error.field);
                }
                all_errors.extend(errors);
            }
        }

        if all_errors.is_empty() {
            Ok(())
        } else {
            Err(all_errors)
        }
    }
}

/// Validateur pour les métadonnées
pub struct MetadataValidator;

impl MetadataValidator {
    /// Valide les métadonnées d'archive
    pub fn validate_archive_metadata(metadata: &std::collections::HashMap<String, String>) -> ValidationResult {
        let mut errors = Vec::new();

        // Vérifie la taille totale des métadonnées
        let total_size: usize = metadata.iter()
            .map(|(k, v)| k.len() + v.len())
            .sum();

        if total_size > 10_000 {
            errors.push(ValidationError::new(
                "metadata", 
                "too_large", 
                "Total metadata size cannot exceed 10KB"
            ));
        }

        // Valide chaque clé et valeur
        for (key, value) in metadata {
            if key.is_empty() {
                errors.push(ValidationError::new(
                    "metadata.key", 
                    "empty", 
                    "Metadata keys cannot be empty"
                ));
            }

            if key.len() > 100 {
                errors.push(ValidationError::with_value(
                    "metadata.key", 
                    "too_long", 
                    "Metadata keys cannot exceed 100 characters",
                    serde_json::Value::String(key.clone())
                ));
            }

            if value.len() > 1000 {
                errors.push(ValidationError::with_value(
                    "metadata.value", 
                    "too_long", 
                    "Metadata values cannot exceed 1000 characters",
                    serde_json::Value::String(value.clone())
                ));
            }

            // Vérifie les caractères interdits
            if key.contains('\0') || value.contains('\0') {
                errors.push(ValidationError::new(
                    "metadata", 
                    "invalid_chars", 
                    "Metadata cannot contain null characters"
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Valide les tags
    pub fn validate_tags(tags: &[String]) -> ValidationResult {
        let mut errors = Vec::new();

        if tags.len() > 20 {
            errors.push(ValidationError::new(
                "tags", 
                "too_many", 
                "Cannot have more than 20 tags"
            ));
        }

        let mut seen_tags = HashSet::new();
        for (index, tag) in tags.iter().enumerate() {
            if tag.trim().is_empty() {
                errors.push(ValidationError::new(
                    &format!("tags[{}]", index), 
                    "empty", 
                    "Tags cannot be empty"
                ));
                continue;
            }

            let normalized_tag = tag.trim().to_lowercase();
            
            if normalized_tag.len() > 50 {
                errors.push(ValidationError::with_value(
                    &format!("tags[{}]", index), 
                    "too_long", 
                    "Tags cannot exceed 50 characters",
                    serde_json::Value::String(tag.clone())
                ));
            }

            if seen_tags.contains(&normalized_tag) {
                errors.push(ValidationError::with_value(
                    &format!("tags[{}]", index), 
                    "duplicate", 
                    "Duplicate tags are not allowed",
                    serde_json::Value::String(tag.clone())
                ));
            }

            seen_tags.insert(normalized_tag);

            // Vérifie les caractères autorisés (lettres, chiffres, tirets, underscores)
            if !tag.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_' || c == ' ') {
                errors.push(ValidationError::with_value(
                    &format!("tags[{}]", index), 
                    "invalid_chars", 
                    "Tags can only contain letters, numbers, spaces, hyphens and underscores",
                    serde_json::Value::String(tag.clone())
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validateur pour les paramètres de recherche
pub struct SearchValidator;

impl SearchValidator {
    /// Valide une requête de recherche
    pub fn validate_search_query(query: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if query.trim().is_empty() {
            errors.push(ValidationError::new(
                "query", 
                "required", 
                "Search query is required"
            ));
            return Err(errors);
        }

        if query.len() > 1000 {
            errors.push(ValidationError::new(
                "query", 
                "too_long", 
                "Search query cannot exceed 1000 characters"
            ));
        }

        // Vérifie les caractères dangereux
        if query.contains('\0') {
            errors.push(ValidationError::new(
                "query", 
                "invalid_chars", 
                "Search query cannot contain null characters"
            ));
        }

        // Limite le nombre de termes
        let terms: Vec<&str> = query.split_whitespace().collect();
        if terms.len() > 50 {
            errors.push(ValidationError::new(
                "query", 
                "too_many_terms", 
                "Search query cannot contain more than 50 terms"
            ));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Valide les filtres de recherche
    pub fn validate_search_filters(filters: &crate::api::types::SearchFilters) -> ValidationResult {
        let mut errors = Vec::new();

        // Valide content_type
        if let Some(content_type) = &filters.content_type {
            if !Self::is_valid_content_type(content_type) {
                errors.push(ValidationError::with_value(
                    "filters.content_type", 
                    "invalid", 
                    "Invalid content type",
                    serde_json::Value::String(content_type.clone())
                ));
            }
        }

        // Valide domain
        if let Some(domain) = &filters.domain {
            if let Err(mut domain_errors) = Self::validate_domain(domain) {
                for error in &mut domain_errors {
                    error.field = format!("filters.{}", error.field);
                }
                errors.extend(domain_errors);
            }
        }

        // Valide date_range
        if let Some(date_range) = &filters.date_range {
            if date_range.start >= date_range.end {
                errors.push(ValidationError::new(
                    "filters.date_range", 
                    "invalid_range", 
                    "Start date must be before end date"
                ));
            }

            let now = chrono::Utc::now();
            if date_range.end > now {
                errors.push(ValidationError::new(
                    "filters.date_range.end", 
                    "future_date", 
                    "End date cannot be in the future"
                ));
            }
        }

        // Valide size_range
        if let Some(size_range) = &filters.size_range {
            if size_range.min >= size_range.max {
                errors.push(ValidationError::new(
                    "filters.size_range", 
                    "invalid_range", 
                    "Minimum size must be less than maximum size"
                ));
            }

            if size_range.max > 1_000_000_000 { // 1GB
                errors.push(ValidationError::new(
                    "filters.size_range.max", 
                    "too_large", 
                    "Maximum size cannot exceed 1GB"
                ));
            }
        }

        // Valide tags
        if let Err(mut tag_errors) = MetadataValidator::validate_tags(&filters.tags) {
            for error in &mut tag_errors {
                error.field = format!("filters.{}", error.field);
            }
            errors.extend(tag_errors);
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Vérifie si un content-type est valide
    fn is_valid_content_type(content_type: &str) -> bool {
        let valid_types = [
            "text/html", "text/plain", "text/css", "text/javascript",
            "application/json", "application/pdf", "application/xml",
            "image/jpeg", "image/png", "image/gif", "image/webp",
            "video/mp4", "video/webm", "audio/mpeg", "audio/wav",
        ];

        valid_types.contains(&content_type) || 
        content_type.starts_with("text/") ||
        content_type.starts_with("application/") ||
        content_type.starts_with("image/") ||
        content_type.starts_with("video/") ||
        content_type.starts_with("audio/")
    }

    /// Valide un nom de domaine
    fn validate_domain(domain: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if domain.is_empty() {
            errors.push(ValidationError::new("domain", "required", "Domain is required"));
            return Err(errors);
        }

        if domain.len() > 253 {
            errors.push(ValidationError::new("domain", "too_long", "Domain cannot exceed 253 characters"));
        }

        // Vérifie le format du domaine (basique)
        if !domain.chars().all(|c| c.is_alphanumeric() || c == '.' || c == '-') {
            errors.push(ValidationError::new("domain", "invalid_chars", "Domain contains invalid characters"));
        }

        if domain.starts_with('.') || domain.ends_with('.') {
            errors.push(ValidationError::new("domain", "invalid_format", "Domain cannot start or end with a dot"));
        }

        if domain.contains("..") {
            errors.push(ValidationError::new("domain", "invalid_format", "Domain cannot contain consecutive dots"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Validateur pour les identifiants
pub struct IdValidator;

impl IdValidator {
    /// Valide un ID d'archive
    pub fn validate_archive_id(archive_id: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if archive_id.is_empty() {
            errors.push(ValidationError::new("archive_id", "required", "Archive ID is required"));
            return Err(errors);
        }

        if !archive_id.starts_with("arc_") {
            errors.push(ValidationError::new("archive_id", "invalid_format", "Archive ID must start with 'arc_'"));
        }

        if archive_id.len() != 36 { // "arc_" + 32 character UUID
            errors.push(ValidationError::new("archive_id", "invalid_length", "Archive ID must be 36 characters long"));
        }

        let uuid_part = &archive_id[4..];
        if !uuid_part.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push(ValidationError::new("archive_id", "invalid_chars", "Archive ID contains invalid characters"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    /// Valide un ID de nœud
    pub fn validate_node_id(node_id: &str) -> ValidationResult {
        let mut errors = Vec::new();

        if node_id.is_empty() {
            errors.push(ValidationError::new("node_id", "required", "Node ID is required"));
            return Err(errors);
        }

        if node_id.len() != 64 { // Hash de 32 bytes en hex
            errors.push(ValidationError::new("node_id", "invalid_length", "Node ID must be 64 characters long"));
        }

        if !node_id.chars().all(|c| c.is_ascii_hexdigit()) {
            errors.push(ValidationError::new("node_id", "invalid_chars", "Node ID must be hexadecimal"));
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Convertit les erreurs de validation en ApiError
pub fn validation_errors_to_api_error(errors: Vec<ValidationError>) -> ApiError {
    let error_response = crate::api::error::ValidationErrorResponse::new(
        errors.into_iter().map(|e| crate::api::error::ValidationError::new(e.field, e.message)).collect()
    );
    
    ApiError::Validation(serde_json::to_string(&error_response).unwrap_or_else(|_| "Validation failed".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_url_validation() {
        // URLs valides
        assert!(UrlValidator::validate_url("https://example.com").is_ok());
        assert!(UrlValidator::validate_url("http://example.com/path").is_ok());
        
        // URLs invalides
        assert!(UrlValidator::validate_url("").is_err());
        assert!(UrlValidator::validate_url("not-a-url").is_err());
        assert!(UrlValidator::validate_url("ftp://example.com").is_err());
        assert!(UrlValidator::validate_url("https://localhost").is_err());
        assert!(UrlValidator::validate_url("https://127.0.0.1").is_err());
    }

    #[test]
    fn test_metadata_validation() {
        let mut metadata = HashMap::new();
        metadata.insert("title".to_string(), "Test Archive".to_string());
        metadata.insert("description".to_string(), "A test archive".to_string());
        
        assert!(MetadataValidator::validate_archive_metadata(&metadata).is_ok());

        // Métadonnées trop grandes
        let mut large_metadata = HashMap::new();
        large_metadata.insert("large_key".to_string(), "x".repeat(10000));
        assert!(MetadataValidator::validate_archive_metadata(&large_metadata).is_err());

        // Clé vide
        let mut empty_key_metadata = HashMap::new();
        empty_key_metadata.insert("".to_string(), "value".to_string());
        assert!(MetadataValidator::validate_archive_metadata(&empty_key_metadata).is_err());
    }

    #[test]
    fn test_tags_validation() {
        // Tags valides
        let valid_tags = vec!["web".to_string(), "archive".to_string(), "test-tag".to_string()];
        assert!(MetadataValidator::validate_tags(&valid_tags).is_ok());

        // Tags dupliqués
        let duplicate_tags = vec!["web".to_string(), "WEB".to_string()];
        assert!(MetadataValidator::validate_tags(&duplicate_tags).is_err());

        // Trop de tags
        let too_many_tags: Vec<String> = (0..25).map(|i| format!("tag{}", i)).collect();
        assert!(MetadataValidator::validate_tags(&too_many_tags).is_err());

        // Tag vide
        let empty_tag = vec!["".to_string()];
        assert!(MetadataValidator::validate_tags(&empty_tag).is_err());

        // Tag trop long
        let long_tag = vec!["x".repeat(100)];
        assert!(MetadataValidator::validate_tags(&long_tag).is_err());
    }

    #[test]
    fn test_search_query_validation() {
        // Requête valide
        assert!(SearchValidator::validate_search_query("test query").is_ok());

        // Requête vide
        assert!(SearchValidator::validate_search_query("").is_err());
        assert!(SearchValidator::validate_search_query("   ").is_err());

        // Requête trop longue
        let long_query = "x ".repeat(1000);
        assert!(SearchValidator::validate_search_query(&long_query).is_err());

        // Trop de termes
        let many_terms = (0..100).map(|i| format!("term{}", i)).collect::<Vec<_>>().join(" ");
        assert!(SearchValidator::validate_search_query(&many_terms).is_err());
    }

    #[test]
    fn test_archive_id_validation() {
        // ID valide
        assert!(IdValidator::validate_archive_id("arc_1234567890abcdef1234567890abcdef").is_ok());

        // ID invalide
        assert!(IdValidator::validate_archive_id("").is_err());
        assert!(IdValidator::validate_archive_id("invalid_id").is_err());
        assert!(IdValidator::validate_archive_id("arc_short").is_err());
        assert!(IdValidator::validate_archive_id("arc_1234567890abcdef1234567890abcdeg").is_err()); // g n'est pas hex
    }

    #[test]
    fn test_node_id_validation() {
        // ID de nœud valide (64 caractères hex)
        let valid_node_id = "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";
        assert!(IdValidator::validate_node_id(valid_node_id).is_ok());

        // ID invalide
        assert!(IdValidator::validate_node_id("").is_err());
        assert!(IdValidator::validate_node_id("short").is_err());
        assert!(IdValidator::validate_node_id("1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdeg").is_err()); // g n'est pas hex
    }

    #[test]
    fn test_domain_validation() {
        // Domaines valides
        assert!(SearchValidator::validate_domain("example.com").is_ok());
        assert!(SearchValidator::validate_domain("sub.example.com").is_ok());

        // Domaines invalides
        assert!(SearchValidator::validate_domain("").is_err());
        assert!(SearchValidator::validate_domain(".example.com").is_err());
        assert!(SearchValidator::validate_domain("example.com.").is_err());
        assert!(SearchValidator::validate_domain("example..com").is_err());
    }

    #[test]
    fn test_content_type_validation() {
        // Types valides
        assert!(SearchValidator::is_valid_content_type("text/html"));
        assert!(SearchValidator::is_valid_content_type("application/json"));
        assert!(SearchValidator::is_valid_content_type("image/jpeg"));

        // Types invalides
        assert!(!SearchValidator::is_valid_content_type("invalid/type"));
    }
}