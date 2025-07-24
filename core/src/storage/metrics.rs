//! Système de métriques et monitoring pour le stockage ArchiveChain
//! 
//! Implémente :
//! - Statistiques de stockage en temps réel
//! - Métriques de performance et latence
//! - Monitoring de la santé des nœuds
//! - Alertes de capacité et disponibilité
//! - Collecte et agrégation de données

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, Instant};
use tokio::sync::{RwLock, Mutex};
use tokio::time::{interval, sleep};
use crate::crypto::Hash;
use crate::consensus::NodeId;
use crate::error::Result;
use super::{
    ContentMetadata, StorageNodeInfo, NodeStatus,
    replication::ReplicationMetrics,
    distribution::DistributionStats,
    discovery::DiscoveryStats,
    bandwidth::BandwidthStats,
};

/// Configuration du système de métriques
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Intervalle de collecte des métriques
    pub collection_interval: Duration,
    /// Taille de l'historique des métriques
    pub history_size: usize,
    /// Intervalle d'agrégation des données
    pub aggregation_interval: Duration,
    /// Activation du monitoring en temps réel
    pub real_time_monitoring: bool,
    /// Seuils d'alerte
    pub alert_thresholds: AlertThresholds,
    /// Rétention des métriques détaillées
    pub detailed_metrics_retention: Duration,
    /// Export des métriques activé
    pub metrics_export_enabled: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            collection_interval: Duration::from_secs(30), // 30 secondes
            history_size: 2880, // 24h avec collecte toutes les 30s
            aggregation_interval: Duration::from_secs(300), // 5 minutes
            real_time_monitoring: true,
            alert_thresholds: AlertThresholds::default(),
            detailed_metrics_retention: Duration::from_secs(7 * 24 * 3600), // 7 jours
            metrics_export_enabled: false,
        }
    }
}

/// Seuils d'alerte pour les métriques
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// Seuil de capacité critique (%)
    pub critical_capacity_threshold: f64,
    /// Seuil de latence élevée (ms)
    pub high_latency_threshold: u32,
    /// Seuil de disponibilité faible (%)
    pub low_availability_threshold: f64,
    /// Seuil d'erreurs critiques (/heure)
    pub critical_error_rate: u32,
    /// Seuil de nœuds hors ligne (%)
    pub offline_nodes_threshold: f64,
    /// Seuil de bande passante saturée (%)
    pub bandwidth_saturation_threshold: f64,
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            critical_capacity_threshold: 90.0,
            high_latency_threshold: 1000, // 1 seconde
            low_availability_threshold: 95.0,
            critical_error_rate: 100,
            offline_nodes_threshold: 10.0,
            bandwidth_saturation_threshold: 85.0,
        }
    }
}

/// Métriques actuelles du système
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMetrics {
    /// Timestamp de collecte
    pub timestamp: SystemTime,
    /// Métriques de performance
    pub performance: PerformanceMetrics,
    /// Métriques de santé
    pub health: HealthMetrics,
    /// Métriques de capacité
    pub capacity: CapacityMetrics,
    /// Métriques de réseau
    pub network: NetworkMetrics,
    /// Métriques d'erreurs
    pub errors: ErrorMetrics,
}

/// Métriques de performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Latence moyenne d'accès (ms)
    pub average_access_latency: u32,
    /// Latence médiane d'accès (ms)
    pub median_access_latency: u32,
    /// Latence P95 d'accès (ms)
    pub p95_access_latency: u32,
    /// Débit moyen (bytes/sec)
    pub average_throughput: u64,
    /// Débit de pointe (bytes/sec)
    pub peak_throughput: u64,
    /// Nombre d'opérations par seconde
    pub operations_per_second: f64,
    /// Temps de réponse moyen du système
    pub average_response_time: Duration,
    /// Taux de succès des opérations (%)
    pub success_rate: f64,
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            average_access_latency: 0,
            median_access_latency: 0,
            p95_access_latency: 0,
            average_throughput: 0,
            peak_throughput: 0,
            operations_per_second: 0.0,
            average_response_time: Duration::ZERO,
            success_rate: 100.0,
        }
    }
}

/// Métriques de santé du système
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMetrics {
    /// Nombre de nœuds actifs
    pub active_nodes: u32,
    /// Nombre de nœuds total
    pub total_nodes: u32,
    /// Pourcentage de nœuds en ligne
    pub nodes_online_percentage: f64,
    /// Nombre de nœuds défaillants
    pub failed_nodes: u32,
    /// Score de santé globale (0-100)
    pub overall_health_score: u8,
    /// Disponibilité du système (%)
    pub system_availability: f64,
    /// Temps de fonctionnement (uptime)
    pub uptime: Duration,
    /// Nombre de redémarrages
    pub restart_count: u32,
}

impl Default for HealthMetrics {
    fn default() -> Self {
        Self {
            active_nodes: 0,
            total_nodes: 0,
            nodes_online_percentage: 0.0,
            failed_nodes: 0,
            overall_health_score: 100,
            system_availability: 100.0,
            uptime: Duration::ZERO,
            restart_count: 0,
        }
    }
}

/// Métriques de capacité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityMetrics {
    /// Capacité totale (bytes)
    pub total_capacity: u64,
    /// Capacité utilisée (bytes)
    pub used_capacity: u64,
    /// Capacité disponible (bytes)
    pub available_capacity: u64,
    /// Pourcentage d'utilisation
    pub usage_percentage: f64,
    /// Taux de croissance de l'utilisation (/jour)
    pub growth_rate_per_day: f64,
    /// Estimation de saturation
    pub estimated_full_date: Option<SystemTime>,
    /// Nombre de contenus stockés
    pub content_count: u64,
    /// Taille moyenne des contenus
    pub average_content_size: u64,
}

impl Default for CapacityMetrics {
    fn default() -> Self {
        Self {
            total_capacity: 0,
            used_capacity: 0,
            available_capacity: 0,
            usage_percentage: 0.0,
            growth_rate_per_day: 0.0,
            estimated_full_date: None,
            content_count: 0,
            average_content_size: 0,
        }
    }
}

/// Métriques de réseau
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Bande passante totale d'upload (bytes/sec)
    pub total_upload_bandwidth: u64,
    /// Bande passante totale de download (bytes/sec)
    pub total_download_bandwidth: u64,
    /// Utilisation de bande passante d'upload (%)
    pub upload_bandwidth_usage: f64,
    /// Utilisation de bande passante de download (%)
    pub download_bandwidth_usage: f64,
    /// Nombre de connexions actives
    pub active_connections: u32,
    /// Latence réseau moyenne inter-nœuds (ms)
    pub average_network_latency: u32,
    /// Perte de paquets (%)
    pub packet_loss_rate: f64,
    /// Nombre de transferts en cours
    pub active_transfers: u32,
}

impl Default for NetworkMetrics {
    fn default() -> Self {
        Self {
            total_upload_bandwidth: 0,
            total_download_bandwidth: 0,
            upload_bandwidth_usage: 0.0,
            download_bandwidth_usage: 0.0,
            active_connections: 0,
            average_network_latency: 0,
            packet_loss_rate: 0.0,
            active_transfers: 0,
        }
    }
}

/// Métriques d'erreurs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMetrics {
    /// Nombre d'erreurs totales dans la dernière heure
    pub total_errors_last_hour: u32,
    /// Taux d'erreurs (/heure)
    pub error_rate_per_hour: f64,
    /// Erreurs critiques
    pub critical_errors: u32,
    /// Erreurs de réseau
    pub network_errors: u32,
    /// Erreurs de stockage
    pub storage_errors: u32,
    /// Erreurs de validation
    pub validation_errors: u32,
    /// Temps moyen de récupération (MTTR)
    pub mean_time_to_recovery: Duration,
    /// Dernière erreur critique
    pub last_critical_error: Option<SystemTime>,
}

impl Default for ErrorMetrics {
    fn default() -> Self {
        Self {
            total_errors_last_hour: 0,
            error_rate_per_hour: 0.0,
            critical_errors: 0,
            network_errors: 0,
            storage_errors: 0,
            validation_errors: 0,
            mean_time_to_recovery: Duration::ZERO,
            last_critical_error: None,
        }
    }
}

/// Point de données historique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsDataPoint {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Métriques à ce moment
    pub metrics: CurrentMetrics,
}

/// Collecteur de métriques
#[derive(Debug)]
pub struct MetricsCollector {
    /// Configuration
    config: MetricsConfig,
    /// Historique des métriques
    history: RwLock<VecDeque<MetricsDataPoint>>,
    /// Métriques actuelles
    current_metrics: RwLock<CurrentMetrics>,
    /// Compteurs d'événements
    event_counters: Mutex<EventCounters>,
    /// Timestamp de démarrage
    start_time: SystemTime,
    /// Dernière collecte
    last_collection: Mutex<SystemTime>,
}

/// Compteurs d'événements
#[derive(Debug, Default)]
struct EventCounters {
    /// Operations réussies
    successful_operations: u64,
    /// Operations échouées
    failed_operations: u64,
    /// Bytes transférés
    bytes_transferred: u64,
    /// Nombre de redémarrages
    restart_count: u32,
    /// Latences mesurées
    latency_measurements: VecDeque<u32>,
    /// Erreurs par type
    error_counts: HashMap<ErrorType, u32>,
}

/// Types d'erreurs
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorType {
    /// Erreur réseau
    Network,
    /// Erreur de stockage
    Storage,
    /// Erreur de validation
    Validation,
    /// Erreur de consensus
    Consensus,
    /// Erreur système
    System,
}

impl MetricsCollector {
    /// Crée un nouveau collecteur de métriques
    pub fn new(config: MetricsConfig) -> Self {
        let current_metrics = CurrentMetrics {
            timestamp: SystemTime::now(),
            performance: PerformanceMetrics::default(),
            health: HealthMetrics::default(),
            capacity: CapacityMetrics::default(),
            network: NetworkMetrics::default(),
            errors: ErrorMetrics::default(),
        };

        Self {
            config,
            history: RwLock::new(VecDeque::new()),
            current_metrics: RwLock::new(current_metrics),
            event_counters: Mutex::new(EventCounters::default()),
            start_time: SystemTime::now(),
            last_collection: Mutex::new(SystemTime::now()),
        }
    }

    /// Enregistre une opération réussie
    pub async fn record_successful_operation(&self, latency_ms: u32, bytes_transferred: u64) {
        let mut counters = self.event_counters.lock().await;
        counters.successful_operations += 1;
        counters.bytes_transferred += bytes_transferred;
        counters.latency_measurements.push_back(latency_ms);

        // Garde seulement les 1000 dernières mesures
        if counters.latency_measurements.len() > 1000 {
            counters.latency_measurements.pop_front();
        }
    }

    /// Enregistre une opération échouée
    pub async fn record_failed_operation(&self, error_type: ErrorType) {
        let mut counters = self.event_counters.lock().await;
        counters.failed_operations += 1;
        *counters.error_counts.entry(error_type).or_insert(0) += 1;
    }

    /// Met à jour les métriques avec les données des nœuds
    pub async fn update_node_metrics(&self, nodes: &HashMap<NodeId, StorageNodeInfo>) {
        let mut metrics = self.current_metrics.write().await;
        metrics.timestamp = SystemTime::now();

        // Calcule les métriques de santé
        let total_nodes = nodes.len() as u32;
        let active_nodes = nodes.values()
            .filter(|node| node.status == NodeStatus::Active)
            .count() as u32;
        let failed_nodes = nodes.values()
            .filter(|node| node.status == NodeStatus::Failed)
            .count() as u32;

        metrics.health.total_nodes = total_nodes;
        metrics.health.active_nodes = active_nodes;
        metrics.health.failed_nodes = failed_nodes;
        metrics.health.nodes_online_percentage = if total_nodes > 0 {
            (active_nodes as f64 / total_nodes as f64) * 100.0
        } else {
            0.0
        };

        // Calcule les métriques de capacité
        let total_capacity: u64 = nodes.values().map(|n| n.total_capacity).sum();
        let used_capacity: u64 = nodes.values().map(|n| n.used_capacity).sum();
        
        metrics.capacity.total_capacity = total_capacity;
        metrics.capacity.used_capacity = used_capacity;
        metrics.capacity.available_capacity = total_capacity.saturating_sub(used_capacity);
        metrics.capacity.usage_percentage = if total_capacity > 0 {
            (used_capacity as f64 / total_capacity as f64) * 100.0
        } else {
            0.0
        };

        // Calcule les métriques de réseau
        let total_bandwidth: u64 = nodes.values().map(|n| n.available_bandwidth).sum();
        let average_latency = if !nodes.is_empty() {
            nodes.values().map(|n| n.average_latency).sum::<u32>() / nodes.len() as u32
        } else {
            0
        };

        metrics.network.total_upload_bandwidth = total_bandwidth;
        metrics.network.total_download_bandwidth = total_bandwidth;
        metrics.network.average_network_latency = average_latency;

        // Met à jour les métriques de performance
        let counters = self.event_counters.lock().await;
        if !counters.latency_measurements.is_empty() {
            let mut sorted_latencies: Vec<_> = counters.latency_measurements.iter().copied().collect();
            sorted_latencies.sort_unstable();

            metrics.performance.average_access_latency = 
                sorted_latencies.iter().sum::<u32>() / sorted_latencies.len() as u32;
            
            metrics.performance.median_access_latency = sorted_latencies[sorted_latencies.len() / 2];
            
            let p95_index = (sorted_latencies.len() as f64 * 0.95) as usize;
            metrics.performance.p95_access_latency = sorted_latencies[p95_index.min(sorted_latencies.len() - 1)];
        }

        // Calcule les métriques d'erreurs
        let total_operations = counters.successful_operations + counters.failed_operations;
        metrics.performance.success_rate = if total_operations > 0 {
            (counters.successful_operations as f64 / total_operations as f64) * 100.0
        } else {
            100.0
        };

        let total_errors: u32 = counters.error_counts.values().sum();
        metrics.errors.total_errors_last_hour = total_errors;
        metrics.errors.network_errors = *counters.error_counts.get(&ErrorType::Network).unwrap_or(&0);
        metrics.errors.storage_errors = *counters.error_counts.get(&ErrorType::Storage).unwrap_or(&0);
        metrics.errors.validation_errors = *counters.error_counts.get(&ErrorType::Validation).unwrap_or(&0);

        // Calcule l'uptime
        metrics.health.uptime = SystemTime::now().duration_since(self.start_time).unwrap_or_default();
        metrics.health.restart_count = counters.restart_count;

        // Score de santé global
        metrics.health.overall_health_score = self.calculate_health_score(&metrics).await;
    }

    /// Calcule le score de santé global
    async fn calculate_health_score(&self, metrics: &CurrentMetrics) -> u8 {
        let mut score = 100.0;

        // Pénalité pour les nœuds hors ligne
        if metrics.health.nodes_online_percentage < 90.0 {
            score -= (90.0 - metrics.health.nodes_online_percentage) * 2.0;
        }

        // Pénalité pour l'utilisation de capacité élevée
        if metrics.capacity.usage_percentage > 80.0 {
            score -= (metrics.capacity.usage_percentage - 80.0) * 1.5;
        }

        // Pénalité pour la latence élevée
        if metrics.performance.average_access_latency > 500 {
            score -= ((metrics.performance.average_access_latency - 500) as f64 / 10.0);
        }

        // Pénalité pour le taux d'erreurs
        if metrics.performance.success_rate < 99.0 {
            score -= (99.0 - metrics.performance.success_rate) * 5.0;
        }

        score.max(0.0).min(100.0) as u8
    }

    /// Collecte et sauvegarde un point de données
    pub async fn collect_metrics_snapshot(&self) -> Result<()> {
        let current_metrics = self.current_metrics.read().await.clone();
        let data_point = MetricsDataPoint {
            timestamp: SystemTime::now(),
            metrics: current_metrics,
        };

        let mut history = self.history.write().await;
        history.push_back(data_point);

        // Limite la taille de l'historique
        while history.len() > self.config.history_size {
            history.pop_front();
        }

        *self.last_collection.lock().await = SystemTime::now();
        Ok(())
    }

    /// Obtient les métriques actuelles
    pub async fn get_current_metrics(&self) -> CurrentMetrics {
        self.current_metrics.read().await.clone()
    }

    /// Obtient l'historique des métriques
    pub async fn get_metrics_history(&self, duration: Duration) -> Vec<MetricsDataPoint> {
        let history = self.history.read().await;
        let cutoff = SystemTime::now() - duration;

        history.iter()
            .filter(|point| point.timestamp > cutoff)
            .cloned()
            .collect()
    }

    /// Nettoie les données anciennes
    pub async fn cleanup_old_data(&self) {
        let cutoff = SystemTime::now() - self.config.detailed_metrics_retention;
        let mut history = self.history.write().await;
        
        while let Some(front) = history.front() {
            if front.timestamp < cutoff {
                history.pop_front();
            } else {
                break;
            }
        }

        // Nettoie les compteurs d'erreurs (garde seulement la dernière heure)
        let mut counters = self.event_counters.lock().await;
        counters.error_counts.clear();
    }
}

/// Gestionnaire d'alertes
#[derive(Debug)]
pub struct AlertManager {
    /// Configuration des seuils
    thresholds: AlertThresholds,
    /// Alertes actives
    active_alerts: RwLock<HashMap<AlertType, Alert>>,
    /// Historique des alertes
    alert_history: RwLock<VecDeque<Alert>>,
    /// Callbacks d'alerte
    alert_callbacks: RwLock<Vec<AlertCallback>>,
}

/// Type d'alerte
#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    /// Capacité critique
    CriticalCapacity,
    /// Latence élevée
    HighLatency,
    /// Disponibilité faible
    LowAvailability,
    /// Nœuds hors ligne
    NodesOffline,
    /// Taux d'erreurs élevé
    HighErrorRate,
    /// Bande passante saturée
    BandwidthSaturated,
    /// Santé système dégradée
    SystemHealthDegraded,
}

/// Alerte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Type d'alerte
    pub alert_type: AlertType,
    /// Niveau de sévérité
    pub severity: AlertSeverity,
    /// Message d'alerte
    pub message: String,
    /// Valeur qui a déclenché l'alerte
    pub trigger_value: f64,
    /// Seuil configuré
    pub threshold: f64,
    /// Timestamp de déclenchement
    pub triggered_at: SystemTime,
    /// Alerte toujours active
    pub is_active: bool,
    /// Timestamp de résolution
    pub resolved_at: Option<SystemTime>,
}

/// Niveau de sévérité d'alerte
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Information
    Info,
    /// Avertissement
    Warning,
    /// Erreur
    Error,
    /// Critique
    Critical,
}

/// Callback d'alerte
type AlertCallback = Box<dyn Fn(&Alert) + Send + Sync>;

impl AlertManager {
    /// Crée un nouveau gestionnaire d'alertes
    pub fn new(thresholds: AlertThresholds) -> Self {
        Self {
            thresholds,
            active_alerts: RwLock::new(HashMap::new()),
            alert_history: RwLock::new(VecDeque::new()),
            alert_callbacks: RwLock::new(Vec::new()),
        }
    }

    /// Vérifie les métriques et déclenche les alertes
    pub async fn check_alerts(&self, metrics: &CurrentMetrics) -> Vec<Alert> {
        let mut new_alerts = Vec::new();

        // Vérifie la capacité critique
        if metrics.capacity.usage_percentage > self.thresholds.critical_capacity_threshold {
            let alert = Alert {
                alert_type: AlertType::CriticalCapacity,
                severity: AlertSeverity::Critical,
                message: format!(
                    "Capacité critique atteinte: {:.1}%",
                    metrics.capacity.usage_percentage
                ),
                trigger_value: metrics.capacity.usage_percentage,
                threshold: self.thresholds.critical_capacity_threshold,
                triggered_at: SystemTime::now(),
                is_active: true,
                resolved_at: None,
            };
            new_alerts.push(alert);
        }

        // Vérifie la latence élevée
        if metrics.performance.average_access_latency > self.thresholds.high_latency_threshold {
            let alert = Alert {
                alert_type: AlertType::HighLatency,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Latence élevée détectée: {}ms",
                    metrics.performance.average_access_latency
                ),
                trigger_value: metrics.performance.average_access_latency as f64,
                threshold: self.thresholds.high_latency_threshold as f64,
                triggered_at: SystemTime::now(),
                is_active: true,
                resolved_at: None,
            };
            new_alerts.push(alert);
        }

        // Vérifie la disponibilité faible
        if metrics.performance.success_rate < self.thresholds.low_availability_threshold {
            let alert = Alert {
                alert_type: AlertType::LowAvailability,
                severity: AlertSeverity::Error,
                message: format!(
                    "Disponibilité faible: {:.1}%",
                    metrics.performance.success_rate
                ),
                trigger_value: metrics.performance.success_rate,
                threshold: self.thresholds.low_availability_threshold,
                triggered_at: SystemTime::now(),
                is_active: true,
                resolved_at: None,
            };
            new_alerts.push(alert);
        }

        // Vérifie les nœuds hors ligne
        if metrics.health.nodes_online_percentage < (100.0 - self.thresholds.offline_nodes_threshold) {
            let alert = Alert {
                alert_type: AlertType::NodesOffline,
                severity: AlertSeverity::Warning,
                message: format!(
                    "Trop de nœuds hors ligne: {:.1}% en ligne",
                    metrics.health.nodes_online_percentage
                ),
                trigger_value: 100.0 - metrics.health.nodes_online_percentage,
                threshold: self.thresholds.offline_nodes_threshold,
                triggered_at: SystemTime::now(),
                is_active: true,
                resolved_at: None,
            };
            new_alerts.push(alert);
        }

        // Traite les nouvelles alertes
        for alert in &new_alerts {
            self.activate_alert(alert.clone()).await;
        }

        new_alerts
    }

    /// Active une alerte
    async fn activate_alert(&self, alert: Alert) {
        let mut active_alerts = self.active_alerts.write().await;
        let mut alert_history = self.alert_history.write().await;

        // Ajoute à l'historique
        alert_history.push_back(alert.clone());
        
        // Limite l'historique à 1000 alertes
        if alert_history.len() > 1000 {
            alert_history.pop_front();
        }

        // Ajoute aux alertes actives
        active_alerts.insert(alert.alert_type.clone(), alert.clone());

        // Exécute les callbacks
        let callbacks = self.alert_callbacks.read().await;
        for callback in callbacks.iter() {
            callback(&alert);
        }
    }

    /// Résout une alerte
    pub async fn resolve_alert(&self, alert_type: AlertType) -> Option<Alert> {
        let mut active_alerts = self.active_alerts.write().await;
        
        if let Some(mut alert) = active_alerts.remove(&alert_type) {
            alert.is_active = false;
            alert.resolved_at = Some(SystemTime::now());
            Some(alert)
        } else {
            None
        }
    }

    /// Obtient les alertes actives
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        let active_alerts = self.active_alerts.read().await;
        active_alerts.values().cloned().collect()
    }

    /// Obtient l'historique des alertes
    pub async fn get_alert_history(&self, duration: Duration) -> Vec<Alert> {
        let alert_history = self.alert_history.read().await;
        let cutoff = SystemTime::now() - duration;

        alert_history.iter()
            .filter(|alert| alert.triggered_at > cutoff)
            .cloned()
            .collect()
    }

    /// Ajoute un callback d'alerte
    pub async fn add_alert_callback<F>(&self, callback: F)
    where
        F: Fn(&Alert) + Send + Sync + 'static,
    {
        let mut callbacks = self.alert_callbacks.write().await;
        callbacks.push(Box::new(callback));
    }
}

/// Moniteur de capacité
#[derive(Debug)]
pub struct CapacityMonitor {
    /// Données historiques d'utilisation
    usage_history: RwLock<VecDeque<CapacityDataPoint>>,
    /// Tendances calculées
    trends: RwLock<CapacityTrends>,
}

/// Point de données de capacité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityDataPoint {
    /// Timestamp
    pub timestamp: SystemTime,
    /// Capacité utilisée
    pub used_capacity: u64,
    /// Capacité totale
    pub total_capacity: u64,
    /// Pourcentage d'utilisation
    pub usage_percentage: f64,
}

/// Tendances de capacité
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapacityTrends {
    /// Croissance quotidienne (bytes/jour)
    pub daily_growth: f64,
    /// Croissance hebdomadaire (bytes/semaine)
    pub weekly_growth: f64,
    /// Projection de saturation
    pub projected_full_date: Option<SystemTime>,
    /// Tendance d'utilisation
    pub usage_trend: UsageTrend,
}

/// Tendance d'utilisation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UsageTrend {
    /// En croissance
    Growing,
    /// Stable
    Stable,
    /// En décroissance
    Declining,
    /// Données insuffisantes
    Unknown,
}

impl CapacityMonitor {
    /// Crée un nouveau moniteur de capacité
    pub fn new() -> Self {
        Self {
            usage_history: RwLock::new(VecDeque::new()),
            trends: RwLock::new(CapacityTrends {
                daily_growth: 0.0,
                weekly_growth: 0.0,
                projected_full_date: None,
                usage_trend: UsageTrend::Unknown,
            }),
        }
    }

    /// Enregistre un point de données de capacité
    pub async fn record_capacity(&self, used_capacity: u64, total_capacity: u64) {
        let data_point = CapacityDataPoint {
            timestamp: SystemTime::now(),
            used_capacity,
            total_capacity,
            usage_percentage: if total_capacity > 0 {
                (used_capacity as f64 / total_capacity as f64) * 100.0
            } else {
                0.0
            },
        };

        let mut history = self.usage_history.write().await;
        history.push_back(data_point);

        // Garde seulement 30 jours de données
        let cutoff = SystemTime::now() - Duration::from_secs(30 * 24 * 3600);
        while let Some(front) = history.front() {
            if front.timestamp < cutoff {
                history.pop_front();
            } else {
                break;
            }
        }

        // Recalcule les tendances
        self.calculate_trends().await;
    }

    /// Calcule les tendances de capacité
    async fn calculate_trends(&self) {
        let history = self.usage_history.read().await;
        
        if history.len() < 7 {
            return; // Pas assez de données
        }

        let data_points: Vec<_> = history.iter().collect();
        
        // Calcule la croissance quotidienne
        let daily_growth = if data_points.len() >= 2 {
            let recent = data_points[data_points.len() - 1];
            let week_ago_index = if data_points.len() >= 7 {
                data_points.len() - 7
            } else {
                0
            };
            let older = data_points[week_ago_index];
            
            let days_diff = recent.timestamp.duration_since(older.timestamp)
                .unwrap_or_default().as_secs() as f64 / (24.0 * 3600.0);
            
            if days_diff > 0.0 {
                (recent.used_capacity as f64 - older.used_capacity as f64) / days_diff
            } else {
                0.0
            }
        } else {
            0.0
        };

        // Détermine la tendance
        let usage_trend = if daily_growth > 0.01 {
            UsageTrend::Growing
        } else if daily_growth < -0.01 {
            UsageTrend::Declining
        } else {
            UsageTrend::Stable
        };

        // Projette la date de saturation
        let projected_full_date = if daily_growth > 0.0 && !data_points.is_empty() {
            let latest = data_points[data_points.len() - 1];
            let remaining_capacity = latest.total_capacity.saturating_sub(latest.used_capacity) as f64;
            let days_to_full = remaining_capacity / daily_growth;
            
            if days_to_full > 0.0 && days_to_full < 365.0 * 5.0 { // Max 5 ans
                Some(SystemTime::now() + Duration::from_secs((days_to_full * 24.0 * 3600.0) as u64))
            } else {
                None
            }
        } else {
            None
        };

        let mut trends = self.trends.write().await;
        trends.daily_growth = daily_growth;
        trends.weekly_growth = daily_growth * 7.0;
        trends.projected_full_date = projected_full_date;
        trends.usage_trend = usage_trend;
    }

    /// Obtient les tendances actuelles
    pub async fn get_trends(&self) -> CapacityTrends {
        self.trends.read().await.clone()
    }

    /// Obtient l'historique de capacité
    pub async fn get_usage_history(&self, duration: Duration) -> Vec<CapacityDataPoint> {
        let history = self.usage_history.read().await;
        let cutoff = SystemTime::now() - duration;

        history.iter()
            .filter(|point| point.timestamp > cutoff)
            .cloned()
            .collect()
    }
}

/// Système principal de métriques et monitoring
pub struct StorageMetrics {
    /// Configuration
    config: MetricsConfig,
    /// Collecteur de métriques
    collector: MetricsCollector,
    /// Gestionnaire d'alertes
    alert_manager: AlertManager,
    /// Moniteur de capacité
    capacity_monitor: CapacityMonitor,
}

impl StorageMetrics {
    /// Crée un nouveau système de métriques
    pub fn new(config: MetricsConfig) -> Self {
        let collector = MetricsCollector::new(config.clone());
        let alert_manager = AlertManager::new(config.alert_thresholds.clone());
        let capacity_monitor = CapacityMonitor::new();

        Self {
            config,
            collector,
            alert_manager,
            capacity_monitor,
        }
    }

    /// Enregistre une opération de stockage
    pub async fn record_storage_operation(&self, size: u64, replicas: u32) {
        let latency = 50; // Latence simulée
        self.collector.record_successful_operation(latency, size).await;
    }

    /// Enregistre une opération de récupération
    pub async fn record_retrieval_operation(&self, size: u64) {
        let latency = 30; // Latence simulée
        self.collector.record_successful_operation(latency, size).await;
    }

    /// Enregistre une erreur
    pub async fn record_error(&self, error_type: ErrorType) {
        self.collector.record_failed_operation(error_type).await;
    }

    /// Met à jour avec les données des nœuds
    pub async fn update_node_data(&self, nodes: &HashMap<NodeId, StorageNodeInfo>) {
        self.collector.update_node_metrics(nodes).await;
        
        // Met à jour le moniteur de capacité
        let total_capacity: u64 = nodes.values().map(|n| n.total_capacity).sum();
        let used_capacity: u64 = nodes.values().map(|n| n.used_capacity).sum();
        self.capacity_monitor.record_capacity(used_capacity, total_capacity).await;
    }

    /// Collecte un snapshot des métriques
    pub async fn collect_snapshot(&self) -> Result<()> {
        self.collector.collect_metrics_snapshot().await
    }

    /// Vérifie les alertes
    pub async fn check_alerts(&self) -> Result<Vec<Alert>> {
        let current_metrics = self.collector.get_current_metrics().await;
        Ok(self.alert_manager.check_alerts(&current_metrics).await)
    }

    /// Obtient les métriques actuelles
    pub async fn get_current_metrics(&self) -> CurrentMetrics {
        self.collector.get_current_metrics().await
    }

    /// Obtient l'historique des métriques
    pub async fn get_metrics_history(&self, duration: Duration) -> Vec<MetricsDataPoint> {
        self.collector.get_metrics_history(duration).await
    }

    /// Obtient les alertes actives
    pub async fn get_active_alerts(&self) -> Vec<Alert> {
        self.alert_manager.get_active_alerts().await
    }

    /// Obtient les tendances de capacité
    pub async fn get_capacity_trends(&self) -> CapacityTrends {
        self.capacity_monitor.get_trends().await
    }

    /// Nettoie les données anciennes
    pub async fn cleanup(&self) {
        self.collector.cleanup_old_data().await;
    }

    /// Obtient un rapport complet du système
    pub async fn get_system_report(&self) -> SystemReport {
        let current_metrics = self.get_current_metrics().await;
        let active_alerts = self.get_active_alerts().await;
        let capacity_trends = self.get_capacity_trends().await;

        SystemReport {
            timestamp: SystemTime::now(),
            metrics: current_metrics,
            active_alerts,
            capacity_trends,
            system_status: self.calculate_system_status(&current_metrics, &active_alerts).await,
        }
    }

    /// Calcule le statut global du système
    async fn calculate_system_status(&self, metrics: &CurrentMetrics, alerts: &[Alert]) -> SystemStatus {
        let has_critical_alerts = alerts.iter().any(|a| a.severity == AlertSeverity::Critical);
        
        if has_critical_alerts {
            return SystemStatus::Critical;
        }

        if metrics.health.overall_health_score < 70 {
            SystemStatus::Degraded
        } else if metrics.health.overall_health_score < 90 {
            SystemStatus::Warning
        } else {
            SystemStatus::Healthy
        }
    }
}

/// Statut global du système
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemStatus {
    /// Système en bonne santé
    Healthy,
    /// Avertissement
    Warning,
    /// Performance dégradée
    Degraded,
    /// État critique
    Critical,
}

/// Rapport complet du système
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemReport {
    /// Timestamp du rapport
    pub timestamp: SystemTime,
    /// Métriques actuelles
    pub metrics: CurrentMetrics,
    /// Alertes actives
    pub active_alerts: Vec<Alert>,
    /// Tendances de capacité
    pub capacity_trends: CapacityTrends,
    /// Statut global
    pub system_status: SystemStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_collector() {
        let config = MetricsConfig::default();
        let collector = MetricsCollector::new(config);

        collector.record_successful_operation(100, 1024).await;
        collector.record_failed_operation(ErrorType::Network).await;

        let metrics = collector.get_current_metrics().await;
        assert_eq!(metrics.performance.success_rate, 50.0); // 1 succès, 1 échec
    }

    #[tokio::test]
    async fn test_alert_manager() {
        let thresholds = AlertThresholds::default();
        let alert_manager = AlertManager::new(thresholds);

        let mut metrics = CurrentMetrics {
            timestamp: SystemTime::now(),
            performance: PerformanceMetrics::default(),
            health: HealthMetrics::default(),
            capacity: CapacityMetrics {
                usage_percentage: 95.0, // Au-dessus du seuil
                ..Default::default()
            },
            network: NetworkMetrics::default(),
            errors: ErrorMetrics::default(),
        };

        let alerts = alert_manager.check_alerts(&metrics).await;
        assert!(!alerts.is_empty());
        assert_eq!(alerts[0].alert_type, AlertType::CriticalCapacity);
    }

    #[tokio::test]
    async fn test_capacity_monitor() {
        let monitor = CapacityMonitor::new();

        // Enregistre plusieurs points de données
        monitor.record_capacity(1000, 10000).await;
        monitor.record_capacity(1100, 10000).await;
        monitor.record_capacity(1200, 10000).await;

        let history = monitor.get_usage_history(Duration::from_secs(3600)).await;
        assert_eq!(history.len(), 3);
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::Error);
        assert!(AlertSeverity::Error > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }

    #[tokio::test]
    async fn test_storage_metrics() {
        let config = MetricsConfig::default();
        let metrics = StorageMetrics::new(config);

        metrics.record_storage_operation(1024, 3).await;
        metrics.record_retrieval_operation(512).await;

        let current = metrics.get_current_metrics().await;
        assert!(current.performance.success_rate > 0.0);
    }
}