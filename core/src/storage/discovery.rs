//! Système de découverte de contenu pour ArchiveChain
//! 
//! Implémente :
//! - DHT (Distributed Hash Table) pour la recherche rapide
//! - Index de contenu hiérarchique
//! - Cache de recherche distribué
//! - Tracking de popularité en temps réel
//! - Recherche sémantique et indexation

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, BTreeMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::crypto::Hash;
use crate::consensus::NodeId;
use crate::error::Result;
use super::{ContentMetadata, StorageNodeInfo};

/// Configuration du système de découverte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    /// Taille du cache de recherche
    pub search_cache_size: usize,
    /// TTL du cache de recherche
    pub search_cache_ttl: Duration,
    /// Intervalle de mise à jour de popularité
    pub popularity_update_interval: Duration,
    /// Seuil de popularité pour le cache chaud
    pub hot_cache_threshold: u64,
    /// Profondeur maximale de l'index hiérarchique
    pub max_index_depth: u32,
    /// Nombre maximum de résultats par recherche
    pub max_search_results: usize,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            search_cache_size: 10000,
            search_cache_ttl: Duration::from_secs(300), // 5 minutes
            popularity_update_interval: Duration::from_secs(60), // 1 minute
            hot_cache_threshold: 100, // 100 accès/heure
            max_index_depth: 5,
            max_search_results: 100,
        }
    }
}

/// Requête de recherche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    /// Termes de recherche
    pub terms: Vec<String>,
    /// Filtres par type de contenu
    pub content_type_filter: Option<String>,
    /// Filtres par tags
    pub tag_filters: Vec<String>,
    /// Période de temps (from, to)
    pub time_range: Option<(SystemTime, SystemTime)>,
    /// Taille minimale du contenu
    pub min_size: Option<u64>,
    /// Taille maximale du contenu
    pub max_size: Option<u64>,
    /// Nombre maximum de résultats
    pub limit: Option<usize>,
    /// Offset pour la pagination
    pub offset: Option<usize>,
}

impl SearchQuery {
    /// Crée une nouvelle requête de recherche
    pub fn new(terms: Vec<String>) -> Self {
        Self {
            terms,
            content_type_filter: None,
            tag_filters: Vec::new(),
            time_range: None,
            min_size: None,
            max_size: None,
            limit: None,
            offset: None,
        }
    }

    /// Ajoute un filtre de type de contenu
    pub fn with_content_type(mut self, content_type: String) -> Self {
        self.content_type_filter = Some(content_type);
        self
    }

    /// Ajoute des filtres de tags
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tag_filters = tags;
        self
    }

    /// Ajoute une plage de temps
    pub fn with_time_range(mut self, from: SystemTime, to: SystemTime) -> Self {
        self.time_range = Some((from, to));
        self
    }

    /// Génère une clé de cache pour cette requête
    pub fn cache_key(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.terms.hash(&mut hasher);
        self.content_type_filter.hash(&mut hasher);
        self.tag_filters.hash(&mut hasher);
        
        format!("search_{:016x}", hasher.finish())
    }
}

/// Résultat de recherche
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Score de pertinence (0.0-1.0)
    pub relevance_score: f64,
    /// Métadonnées du contenu
    pub metadata: ContentMetadata,
    /// Nœuds où le contenu est disponible
    pub available_nodes: Vec<NodeId>,
    /// Timestamp de dernière mise à jour
    pub last_updated: SystemTime,
}

/// Résultats de recherche complets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResults {
    /// Résultats trouvés
    pub results: Vec<SearchResult>,
    /// Nombre total de résultats (avant pagination)
    pub total_count: usize,
    /// Temps de recherche
    pub search_time: Duration,
    /// Source de la recherche (cache/index)
    pub source: SearchSource,
}

/// Source d'un résultat de recherche
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SearchSource {
    /// Résultat depuis le cache
    Cache,
    /// Résultat depuis l'index principal
    Index,
    /// Résultat depuis la DHT
    DHT,
}

/// DHT (Distributed Hash Table) pour ArchiveChain
#[derive(Debug)]
pub struct DistributedHashTable {
    /// Table locale des hash -> métadonnées
    local_table: HashMap<Hash, DHTEntry>,
    /// Nœuds voisins dans la DHT
    neighbors: Vec<NodeId>,
    /// Configuration
    config: DiscoveryConfig,
}

/// Entrée dans la DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DHTEntry {
    /// Hash du contenu
    pub content_hash: Hash,
    /// Métadonnées
    pub metadata: ContentMetadata,
    /// Nœuds stockant le contenu
    pub storage_nodes: Vec<NodeId>,
    /// Timestamp de dernière mise à jour
    pub last_updated: SystemTime,
    /// Nombre d'accès
    pub access_count: u64,
}

impl DistributedHashTable {
    /// Crée une nouvelle DHT
    pub fn new(config: DiscoveryConfig) -> Self {
        Self {
            local_table: HashMap::new(),
            neighbors: Vec::new(),
            config,
        }
    }

    /// Ajoute une entrée à la DHT
    pub fn put(&mut self, content_hash: Hash, metadata: ContentMetadata, storage_nodes: Vec<NodeId>) {
        let entry = DHTEntry {
            content_hash,
            metadata,
            storage_nodes,
            last_updated: SystemTime::now(),
            access_count: 0,
        };
        
        self.local_table.insert(content_hash, entry);
    }

    /// Récupère une entrée de la DHT
    pub fn get(&mut self, content_hash: &Hash) -> Option<&mut DHTEntry> {
        if let Some(entry) = self.local_table.get_mut(content_hash) {
            entry.access_count += 1;
            Some(entry)
        } else {
            None
        }
    }

    /// Recherche dans la DHT
    pub fn search(&self, query: &SearchQuery) -> Vec<SearchResult> {
        let mut results = Vec::new();

        for entry in self.local_table.values() {
            if self.matches_query(entry, query) {
                let relevance_score = self.calculate_relevance(&entry.metadata, query);
                
                results.push(SearchResult {
                    content_hash: entry.content_hash,
                    relevance_score,
                    metadata: entry.metadata.clone(),
                    available_nodes: entry.storage_nodes.clone(),
                    last_updated: entry.last_updated,
                });
            }
        }

        // Trie par score de pertinence
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        results
    }

    /// Vérifie si une entrée correspond à une requête
    fn matches_query(&self, entry: &DHTEntry, query: &SearchQuery) -> bool {
        // Filtre par type de contenu
        if let Some(ref content_type) = query.content_type_filter {
            if !entry.metadata.content_type.contains(content_type) {
                return false;
            }
        }

        // Filtre par tags
        if !query.tag_filters.is_empty() {
            let entry_tags: std::collections::HashSet<_> = entry.metadata.tags.iter().collect();
            let query_tags: std::collections::HashSet<_> = query.tag_filters.iter().collect();
            
            if entry_tags.intersection(&query_tags).count() == 0 {
                return false;
            }
        }

        // Filtre par taille
        if let Some(min_size) = query.min_size {
            if entry.metadata.size < min_size {
                return false;
            }
        }

        if let Some(max_size) = query.max_size {
            if entry.metadata.size > max_size {
                return false;
            }
        }

        // Filtre par temps
        if let Some((from, to)) = query.time_range {
            let entry_time = entry.metadata.created_at.timestamp() as u64;
            let from_time = from.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            let to_time = to.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
            
            if entry_time < from_time || entry_time > to_time {
                return false;
            }
        }

        true
    }

    /// Calcule la pertinence d'un contenu pour une requête
    fn calculate_relevance(&self, metadata: &ContentMetadata, query: &SearchQuery) -> f64 {
        let mut score = 0.0;
        let mut factors = 0;

        // Score basé sur les termes de recherche
        for term in &query.terms {
            let term_lower = term.to_lowercase();
            
            // Recherche dans le titre
            if let Some(ref title) = metadata.title {
                if title.to_lowercase().contains(&term_lower) {
                    score += 1.0;
                }
            }

            // Recherche dans la description
            if let Some(ref description) = metadata.description {
                if description.to_lowercase().contains(&term_lower) {
                    score += 0.8;
                }
            }

            // Recherche dans les tags
            for tag in &metadata.tags {
                if tag.to_lowercase().contains(&term_lower) {
                    score += 0.6;
                }
            }

            factors += 1;
        }

        // Normalise le score
        if factors > 0 {
            score /= factors as f64;
        }

        // Bonus pour la popularité
        let popularity_bonus = (metadata.popularity as f64).log10().max(0.0) / 10.0;
        score += popularity_bonus;

        score.min(1.0)
    }

    /// Met à jour la liste des voisins
    pub fn update_neighbors(&mut self, neighbors: Vec<NodeId>) {
        self.neighbors = neighbors;
    }

    /// Obtient les statistiques de la DHT
    pub fn get_stats(&self) -> DHTStats {
        DHTStats {
            total_entries: self.local_table.len(),
            total_accesses: self.local_table.values().map(|e| e.access_count).sum(),
            neighbor_count: self.neighbors.len(),
        }
    }
}

/// Index de contenu hiérarchique
#[derive(Debug)]
pub struct ContentIndex {
    /// Index par type de contenu
    content_type_index: HashMap<String, Vec<Hash>>,
    /// Index par tags
    tag_index: HashMap<String, Vec<Hash>>,
    /// Index temporel (année -> mois -> jour)
    temporal_index: BTreeMap<u32, BTreeMap<u32, BTreeMap<u32, Vec<Hash>>>>,
    /// Index de taille (plages de taille)
    size_index: BTreeMap<u64, Vec<Hash>>,
    /// Métadonnées complètes
    metadata_store: HashMap<Hash, ContentMetadata>,
}

impl ContentIndex {
    /// Crée un nouvel index
    pub fn new() -> Self {
        Self {
            content_type_index: HashMap::new(),
            tag_index: HashMap::new(),
            temporal_index: BTreeMap::new(),
            size_index: BTreeMap::new(),
            metadata_store: HashMap::new(),
        }
    }

    /// Ajoute du contenu à l'index
    pub fn add_content(&mut self, content_hash: Hash, metadata: ContentMetadata) {
        // Index par type de contenu
        self.content_type_index
            .entry(metadata.content_type.clone())
            .or_insert_with(Vec::new)
            .push(content_hash);

        // Index par tags
        for tag in &metadata.tags {
            self.tag_index
                .entry(tag.clone())
                .or_insert_with(Vec::new)
                .push(content_hash);
        }

        // Index temporel
        let datetime = metadata.created_at;
        let year = datetime.year() as u32;
        let month = datetime.month();
        let day = datetime.day();

        self.temporal_index
            .entry(year)
            .or_insert_with(BTreeMap::new)
            .entry(month)
            .or_insert_with(BTreeMap::new)
            .entry(day)
            .or_insert_with(Vec::new)
            .push(content_hash);

        // Index par taille (buckets de 1MB)
        let size_bucket = metadata.size / (1024 * 1024);
        self.size_index
            .entry(size_bucket)
            .or_insert_with(Vec::new)
            .push(content_hash);

        // Stocke les métadonnées
        self.metadata_store.insert(content_hash, metadata);
    }

    /// Recherche dans l'index
    pub fn search(&self, query: &SearchQuery) -> Vec<Hash> {
        let mut candidates: Option<std::collections::HashSet<Hash>> = None;

        // Filtre par type de contenu
        if let Some(ref content_type) = query.content_type_filter {
            if let Some(content_hashes) = self.content_type_index.get(content_type) {
                let set: std::collections::HashSet<_> = content_hashes.iter().cloned().collect();
                candidates = Some(match candidates {
                    Some(existing) => existing.intersection(&set).cloned().collect(),
                    None => set,
                });
            } else {
                return Vec::new(); // Aucun contenu de ce type
            }
        }

        // Filtre par tags
        for tag in &query.tag_filters {
            if let Some(tag_hashes) = self.tag_index.get(tag) {
                let set: std::collections::HashSet<_> = tag_hashes.iter().cloned().collect();
                candidates = Some(match candidates {
                    Some(existing) => existing.intersection(&set).cloned().collect(),
                    None => set,
                });
            } else {
                return Vec::new(); // Aucun contenu avec ce tag
            }
        }

        // Si aucun filtre spécifique, commence avec tous les contenus
        if candidates.is_none() {
            candidates = Some(self.metadata_store.keys().cloned().collect());
        }

        let mut results: Vec<_> = candidates.unwrap().into_iter().collect();

        // Filtre par taille et temps
        results.retain(|hash| {
            if let Some(metadata) = self.metadata_store.get(hash) {
                // Filtre par taille
                if let Some(min_size) = query.min_size {
                    if metadata.size < min_size {
                        return false;
                    }
                }
                if let Some(max_size) = query.max_size {
                    if metadata.size > max_size {
                        return false;
                    }
                }

                // Filtre par temps
                if let Some((from, to)) = query.time_range {
                    let entry_time = metadata.created_at.timestamp() as u64;
                    let from_time = from.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    let to_time = to.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
                    
                    if entry_time < from_time || entry_time > to_time {
                        return false;
                    }
                }

                true
            } else {
                false
            }
        });

        results
    }

    /// Obtient les métadonnées d'un contenu
    pub fn get_metadata(&self, content_hash: &Hash) -> Option<&ContentMetadata> {
        self.metadata_store.get(content_hash)
    }

    /// Obtient les statistiques de l'index
    pub fn get_stats(&self) -> IndexStats {
        IndexStats {
            total_content: self.metadata_store.len(),
            content_types: self.content_type_index.len(),
            unique_tags: self.tag_index.len(),
            temporal_range: self.get_temporal_range(),
        }
    }

    /// Obtient la plage temporelle du contenu indexé
    fn get_temporal_range(&self) -> Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)> {
        let dates: Vec<_> = self.metadata_store.values()
            .map(|m| m.created_at)
            .collect();

        if dates.is_empty() {
            None
        } else {
            let min_date = dates.iter().min().unwrap();
            let max_date = dates.iter().max().unwrap();
            Some((*min_date, *max_date))
        }
    }
}

/// Cache de recherche LRU
#[derive(Debug)]
pub struct SearchCache {
    /// Entrées du cache
    cache: HashMap<String, CacheEntry>,
    /// Ordre d'accès (LRU)
    access_order: VecDeque<String>,
    /// Taille maximale du cache
    max_size: usize,
    /// TTL des entrées
    ttl: Duration,
}

/// Entrée de cache
#[derive(Debug, Clone)]
struct CacheEntry {
    /// Résultats de recherche
    results: SearchResults,
    /// Timestamp de création
    created_at: SystemTime,
    /// Nombre d'accès
    access_count: u64,
}

impl SearchCache {
    /// Crée un nouveau cache
    pub fn new(max_size: usize, ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            access_order: VecDeque::new(),
            max_size,
            ttl,
        }
    }

    /// Ajoute des résultats au cache
    pub fn put(&mut self, query: &SearchQuery, results: SearchResults) {
        let key = query.cache_key();
        
        // Retire l'ancienne entrée si elle existe
        if self.cache.contains_key(&key) {
            self.access_order.retain(|k| k != &key);
        }

        // Ajoute la nouvelle entrée
        let entry = CacheEntry {
            results,
            created_at: SystemTime::now(),
            access_count: 0,
        };

        self.cache.insert(key.clone(), entry);
        self.access_order.push_back(key);

        // Éviction LRU si nécessaire
        self.evict_if_needed();
    }

    /// Récupère des résultats du cache
    pub fn get(&mut self, query: &SearchQuery) -> Option<SearchResults> {
        let key = query.cache_key();
        
        if let Some(entry) = self.cache.get_mut(&key) {
            // Vérifie l'expiration
            if entry.created_at.elapsed().unwrap_or(Duration::MAX) > self.ttl {
                self.cache.remove(&key);
                self.access_order.retain(|k| k != &key);
                return None;
            }

            // Met à jour l'ordre d'accès
            self.access_order.retain(|k| k != &key);
            self.access_order.push_back(key);
            
            entry.access_count += 1;
            
            let mut results = entry.results.clone();
            results.source = SearchSource::Cache;
            Some(results)
        } else {
            None
        }
    }

    /// Éviction LRU
    fn evict_if_needed(&mut self) {
        while self.cache.len() > self.max_size {
            if let Some(oldest_key) = self.access_order.pop_front() {
                self.cache.remove(&oldest_key);
            }
        }
    }

    /// Nettoie les entrées expirées
    pub fn cleanup_expired(&mut self) {
        let now = SystemTime::now();
        let expired_keys: Vec<_> = self.cache.iter()
            .filter(|(_, entry)| {
                now.duration_since(entry.created_at).unwrap_or(Duration::ZERO) > self.ttl
            })
            .map(|(key, _)| key.clone())
            .collect();

        for key in expired_keys {
            self.cache.remove(&key);
            self.access_order.retain(|k| k != &key);
        }
    }

    /// Obtient les statistiques du cache
    pub fn get_stats(&self) -> CacheStats {
        CacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            hit_rate: 0.0, // À implémenter avec un compteur de hits/miss
        }
    }
}

/// Tracker de popularité
#[derive(Debug)]
pub struct PopularityTracker {
    /// Compteurs d'accès par contenu
    access_counts: HashMap<Hash, u64>,
    /// Timestamps des accès récents
    recent_accesses: HashMap<Hash, VecDeque<SystemTime>>,
    /// Fenêtre de temps pour la popularité
    time_window: Duration,
}

impl PopularityTracker {
    /// Crée un nouveau tracker
    pub fn new(time_window: Duration) -> Self {
        Self {
            access_counts: HashMap::new(),
            recent_accesses: HashMap::new(),
            time_window,
        }
    }

    /// Enregistre un accès à un contenu
    pub fn record_access(&mut self, content_hash: Hash) {
        let now = SystemTime::now();
        
        // Incrémente le compteur global
        *self.access_counts.entry(content_hash).or_insert(0) += 1;
        
        // Ajoute l'accès récent
        let recent = self.recent_accesses.entry(content_hash).or_insert_with(VecDeque::new);
        recent.push_back(now);
        
        // Nettoie les anciens accès
        self.cleanup_old_accesses(content_hash);
    }

    /// Obtient la popularité récente d'un contenu
    pub fn get_recent_popularity(&mut self, content_hash: &Hash) -> u64 {
        self.cleanup_old_accesses(*content_hash);
        self.recent_accesses.get(content_hash)
            .map(|accesses| accesses.len() as u64)
            .unwrap_or(0)
    }

    /// Obtient la popularité totale d'un contenu
    pub fn get_total_popularity(&self, content_hash: &Hash) -> u64 {
        self.access_counts.get(content_hash).copied().unwrap_or(0)
    }

    /// Nettoie les anciens accès
    fn cleanup_old_accesses(&mut self, content_hash: Hash) {
        if let Some(accesses) = self.recent_accesses.get_mut(&content_hash) {
            let cutoff = SystemTime::now() - self.time_window;
            
            while let Some(&front_time) = accesses.front() {
                if front_time < cutoff {
                    accesses.pop_front();
                } else {
                    break;
                }
            }
        }
    }

    /// Obtient les contenus les plus populaires
    pub fn get_top_content(&mut self, limit: usize) -> Vec<(Hash, u64)> {
        // Nettoie d'abord tous les accès anciens
        let content_hashes: Vec<_> = self.recent_accesses.keys().cloned().collect();
        for hash in content_hashes {
            self.cleanup_old_accesses(hash);
        }

        // Collecte et trie par popularité récente
        let mut popularities: Vec<_> = self.recent_accesses.iter()
            .map(|(hash, accesses)| (*hash, accesses.len() as u64))
            .collect();

        popularities.sort_by(|a, b| b.1.cmp(&a.1));
        popularities.truncate(limit);
        popularities
    }
}

/// Système principal de découverte de contenu
#[derive(Debug)]
pub struct ContentDiscovery {
    /// DHT pour la recherche distribuée
    dht: DistributedHashTable,
    /// Index de contenu local
    content_index: ContentIndex,
    /// Cache de recherche
    search_cache: SearchCache,
    /// Tracker de popularité
    popularity_tracker: PopularityTracker,
    /// Configuration
    config: DiscoveryConfig,
}

impl ContentDiscovery {
    /// Crée un nouveau système de découverte
    pub fn new(config: DiscoveryConfig) -> Self {
        let search_cache = SearchCache::new(config.search_cache_size, config.search_cache_ttl);
        let popularity_tracker = PopularityTracker::new(Duration::from_secs(3600)); // 1 heure
        
        Self {
            dht: DistributedHashTable::new(config.clone()),
            content_index: ContentIndex::new(),
            search_cache,
            popularity_tracker,
            config,
        }
    }

    /// Ajoute du contenu au système de découverte
    pub fn add_content(&mut self, content_hash: Hash, metadata: ContentMetadata, storage_nodes: Vec<NodeId>) {
        self.dht.put(content_hash, metadata.clone(), storage_nodes);
        self.content_index.add_content(content_hash, metadata);
    }

    /// Recherche du contenu
    pub async fn search(&mut self, query: SearchQuery) -> Result<SearchResults> {
        let start_time = SystemTime::now();

        // Vérifie d'abord le cache
        if let Some(cached_results) = self.search_cache.get(&query) {
            return Ok(cached_results);
        }

        // Recherche dans l'index local
        let content_hashes = self.content_index.search(&query);
        let mut results = Vec::new();

        for hash in content_hashes {
            if let Some(metadata) = self.content_index.get_metadata(&hash) {
                if let Some(dht_entry) = self.dht.get(&hash) {
                    let relevance_score = self.calculate_search_relevance(metadata, &query);
                    
                    results.push(SearchResult {
                        content_hash: hash,
                        relevance_score,
                        metadata: metadata.clone(),
                        available_nodes: dht_entry.storage_nodes.clone(),
                        last_updated: dht_entry.last_updated,
                    });
                }
            }
        }

        // Trie par pertinence
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(std::cmp::Ordering::Equal));

        // Applique la pagination
        let total_count = results.len();
        let offset = query.offset.unwrap_or(0);
        let limit = query.limit.unwrap_or(self.config.max_search_results).min(self.config.max_search_results);

        if offset < results.len() {
            results = results.into_iter().skip(offset).take(limit).collect();
        } else {
            results = Vec::new();
        }

        let search_time = start_time.elapsed().unwrap_or(Duration::ZERO);
        let search_results = SearchResults {
            results,
            total_count,
            search_time,
            source: SearchSource::Index,
        };

        // Met en cache les résultats
        self.search_cache.put(&query, search_results.clone());

        Ok(search_results)
    }

    /// Calcule la pertinence d'un résultat de recherche
    fn calculate_search_relevance(&mut self, metadata: &ContentMetadata, query: &SearchQuery) -> f64 {
        let base_relevance = self.dht.calculate_relevance(metadata, query);
        
        // Bonus pour la popularité récente
        let recent_popularity = self.popularity_tracker.get_recent_popularity(&metadata.content_hash);
        let popularity_bonus = (recent_popularity as f64).log10().max(0.0) / 20.0;
        
        (base_relevance + popularity_bonus).min(1.0)
    }

    /// Enregistre un accès à un contenu
    pub fn record_content_access(&mut self, content_hash: Hash) {
        self.popularity_tracker.record_access(content_hash);
    }

    /// Obtient les contenus les plus populaires
    pub fn get_popular_content(&mut self, limit: usize) -> Vec<(Hash, u64)> {
        self.popularity_tracker.get_top_content(limit)
    }

    /// Nettoie les caches et données expirées
    pub fn cleanup(&mut self) {
        self.search_cache.cleanup_expired();
    }

    /// Obtient les statistiques du système de découverte
    pub fn get_stats(&self) -> DiscoveryStats {
        DiscoveryStats {
            dht_stats: self.dht.get_stats(),
            index_stats: self.content_index.get_stats(),
            cache_stats: self.search_cache.get_stats(),
        }
    }
}

/// Statistiques de la DHT
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DHTStats {
    /// Nombre total d'entrées
    pub total_entries: usize,
    /// Nombre total d'accès
    pub total_accesses: u64,
    /// Nombre de nœuds voisins
    pub neighbor_count: usize,
}

/// Statistiques de l'index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexStats {
    /// Nombre total de contenus
    pub total_content: usize,
    /// Nombre de types de contenu différents
    pub content_types: usize,
    /// Nombre de tags uniques
    pub unique_tags: usize,
    /// Plage temporelle du contenu
    pub temporal_range: Option<(chrono::DateTime<chrono::Utc>, chrono::DateTime<chrono::Utc>)>,
}

/// Statistiques du cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Taille actuelle du cache
    pub size: usize,
    /// Taille maximale du cache
    pub max_size: usize,
    /// Taux de hit du cache
    pub hit_rate: f64,
}

/// Statistiques globales du système de découverte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryStats {
    /// Statistiques de la DHT
    pub dht_stats: DHTStats,
    /// Statistiques de l'index
    pub index_stats: IndexStats,
    /// Statistiques du cache
    pub cache_stats: CacheStats,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::Hash;

    fn create_test_metadata() -> ContentMetadata {
        super::super::ContentMetadata {
            content_hash: Hash::zero(),
            size: 1024 * 1024,
            content_type: "text/html".to_string(),
            importance: super::super::replication::ContentImportance::Medium,
            popularity: 500,
            created_at: chrono::Utc::now(),
            preferred_regions: vec!["eu-west-1".to_string()],
            redundancy_level: 3,
            tags: vec!["web".to_string(), "article".to_string()],
        }
    }

    #[test]
    fn test_search_query() {
        let query = SearchQuery::new(vec!["test".to_string()])
            .with_content_type("text/html".to_string())
            .with_tags(vec!["web".to_string()]);

        assert_eq!(query.terms, vec!["test"]);
        assert_eq!(query.content_type_filter, Some("text/html".to_string()));
        assert_eq!(query.tag_filters, vec!["web"]);
    }

    #[test]
    fn test_dht_operations() {
        let config = DiscoveryConfig::default();
        let mut dht = DistributedHashTable::new(config);
        let metadata = create_test_metadata();
        let content_hash = Hash::zero();

        dht.put(content_hash, metadata.clone(), vec![NodeId::from(Hash::zero())]);
        
        let entry = dht.get(&content_hash);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().access_count, 1);
    }

    #[test]
    fn test_content_index() {
        let mut index = ContentIndex::new();
        let metadata = create_test_metadata();
        let content_hash = Hash::zero();

        index.add_content(content_hash, metadata.clone());

        let query = SearchQuery::new(vec![])
            .with_content_type("text/html".to_string());
        
        let results = index.search(&query);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], content_hash);
    }

    #[test]
    fn test_search_cache() {
        let mut cache = SearchCache::new(10, Duration::from_secs(300));
        let query = SearchQuery::new(vec!["test".to_string()]);
        let results = SearchResults {
            results: vec![],
            total_count: 0,
            search_time: Duration::from_millis(50),
            source: SearchSource::Index,
        };

        cache.put(&query, results.clone());
        let cached = cache.get(&query);
        
        assert!(cached.is_some());
        assert_eq!(cached.unwrap().source, SearchSource::Cache);
    }

    #[test]
    fn test_popularity_tracker() {
        let mut tracker = PopularityTracker::new(Duration::from_secs(3600));
        let content_hash = Hash::zero();

        tracker.record_access(content_hash);
        tracker.record_access(content_hash);

        assert_eq!(tracker.get_total_popularity(&content_hash), 2);
        assert_eq!(tracker.get_recent_popularity(&content_hash), 2);
    }

    #[test]
    fn test_content_discovery() {
        let config = DiscoveryConfig::default();
        let mut discovery = ContentDiscovery::new(config);
        let metadata = create_test_metadata();
        let content_hash = Hash::zero();

        discovery.add_content(content_hash, metadata, vec![NodeId::from(Hash::zero())]);
        discovery.record_content_access(content_hash);

        let popular = discovery.get_popular_content(5);
        assert_eq!(popular.len(), 1);
        assert_eq!(popular[0].0, content_hash);
    }
}