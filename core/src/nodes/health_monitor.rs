//! Moniteur de santé pour les nœuds ArchiveChain
//!
//! Le Health Monitor assure la surveillance continue de tous les nœuds :
//! - Monitoring temps réel de la santé des nœuds
//! - Détection automatique des pannes et anomalies
//! - Système d'alertes et de notifications
//! - Mécanismes de récupération automatique
//! - Collecte et analyse des métriques de performance

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::{RwLock, Mutex};

use crate::consensus::NodeId;
use crate::error::Result;

/// Configuration du Health Monitor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitorConfig {
    /// Intervalle de vérification de santé
    pub check_interval: Duration,
    /// Timeout pour les checks de santé
    pub check_timeout: Duration,
    /// Nombre de checks échoués avant alerte
    pub failure_threshold: u32,
    /// Intervalle d'alertes
    pub alert_interval: Duration,
    /// Récupération automatique activée
    pub auto_recovery_enabled: bool,
    /// Nombre maximum de tentatives de récupération
    pub max_recovery_attempts: u32,
    /// Configuration des métriques
    pub metrics_config: MetricsCollectionConfig,
    /// Configuration des alertes
    pub alert_config: AlertConfig,
    /// Configuration de la récupération automatique
    pub recovery_config: AutoRecoveryConfig,
}

/// Configuration de collecte des métriques
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsCollectionConfig {
    /// Collecte activée
    pub enabled: bool,
    /// Intervalle de collecte
    pub collection_interval: Duration,
    /// Taille de l'historique
    pub history_size: usize,
    /// Métriques à collecter
    pub metrics_to_collect: Vec<MetricType>,
    /// Agrégation des métriques
    pub aggregation_enabled: bool,
    /// Fenêtre d'agrégation
    pub aggregation_window: Duration,
}

/// Types de métriques à collecter
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum MetricType {
    /// Utilisation CPU
    CpuUsage,
    /// Utilisation mémoire
    MemoryUsage,
    /// Utilisation stockage
    StorageUsage,
    /// Latence réseau
    NetworkLatency,
    /// Bande passante
    Bandwidth,
    /// Nombre de connexions
    ConnectionCount,
    /// Taux d'erreur
    ErrorRate,
    /// Temps de réponse
    ResponseTime,
}

/// Configuration des alertes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Alertes activées
    pub enabled: bool,
    /// Canaux d'alerte
    pub alert_channels: Vec<AlertChannel>,
    /// Seuils d'alerte
    pub thresholds: AlertThresholds,
    /// Escalade d'alertes
    pub escalation_enabled: bool,
    /// Délai d'escalade
    pub escalation_delay: Duration,
}

/// Canaux d'alerte
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertChannel {
    /// Log système
    Log,
    /// Email
    Email,
    /// Webhook
    Webhook,
    /// Slack
    Slack,
    /// SMS
    SMS,
}

/// Seuils d'alerte
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertThresholds {
    /// CPU critique (%)
    pub cpu_critical: f64,
    /// CPU avertissement (%)
    pub cpu_warning: f64,
    /// Mémoire critique (%)
    pub memory_critical: f64,
    /// Mémoire avertissement (%)
    pub memory_warning: f64,
    /// Stockage critique (%)
    pub storage_critical: f64,
    /// Stockage avertissement (%)
    pub storage_warning: f64,
    /// Latence critique (ms)
    pub latency_critical: u64,
    /// Latence avertissement (ms)
    pub latency_warning: u64,
    /// Taux d'erreur critique (%)
    pub error_rate_critical: f64,
    /// Taux d'erreur avertissement (%)
    pub error_rate_warning: f64,
}

/// Configuration de récupération automatique
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoRecoveryConfig {
    /// Récupération activée
    pub enabled: bool,
    /// Actions de récupération
    pub recovery_actions: Vec<RecoveryAction>,
    /// Délai entre tentatives
    pub retry_delay: Duration,
    /// Backoff exponentiel
    pub exponential_backoff: bool,
    /// Délai maximum entre tentatives
    pub max_retry_delay: Duration,
}

/// Actions de récupération
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryAction {
    /// Redémarrage du nœud
    RestartNode,
    /// Nettoyage de cache
    ClearCache,
    /// Resynchronisation
    Resynchronize,
    /// Réinitialisation de connexions
    ResetConnections,
    /// Changement de mode
    SwitchMode,
}

/// Statut de santé d'un nœud
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Nœud en bonne santé
    Healthy,
    /// Avertissement détecté
    Warning,
    /// État critique
    Critical,
    /// Nœud non répondant
    Unresponsive,
    /// En cours de récupération
    Recovering,
}

/// Informations de santé d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    /// Statut de santé
    pub status: HealthStatus,
    /// Temps de fonctionnement
    pub uptime: Duration,
    /// Utilisation CPU
    pub cpu_usage: f64,
    /// Utilisation mémoire
    pub memory_usage: f64,
    /// Utilisation stockage
    pub storage_usage: f64,
    /// Latence réseau
    pub network_latency: Duration,
    /// Taux d'erreur
    pub error_rate: f64,
    /// Dernière vérification
    pub last_check: SystemTime,
}

/// Métriques de performance d'un nœud
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Identifiant du nœud
    pub node_id: NodeId,
    /// Timestamp de collecte
    pub timestamp: chrono::DateTime<chrono::Utc>,
    /// Métriques collectées
    pub metrics: HashMap<MetricType, f64>,
    /// Métadonnées additionnelles
    pub metadata: HashMap<String, String>,
}

/// Alerte de santé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthAlert {
    /// Identifiant unique de l'alerte
    pub alert_id: String,
    /// Nœud concerné
    pub node_id: NodeId,
    /// Type d'alerte
    pub alert_type: AlertType,
    /// Niveau de sévérité
    pub severity: AlertSeverity,
    /// Message descriptif
    pub message: String,
    /// Timestamp de création
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Statut de l'alerte
    pub status: AlertStatus,
    /// Actions recommandées
    pub recommended_actions: Vec<RecoveryAction>,
}

/// Types d'alerte
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertType {
    /// Utilisation excessive de ressources
    HighResourceUsage,
    /// Latence élevée
    HighLatency,
    /// Taux d'erreur élevé
    HighErrorRate,
    /// Nœud non répondant
    NodeUnresponsive,
    /// Problème de connectivité
    ConnectivityIssue,
    /// Espace disque faible
    LowDiskSpace,
    /// Problème de synchronisation
    SyncIssue,
}

/// Niveaux de sévérité d'alerte
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

/// Statut d'une alerte
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Active
    Active,
    /// Acquittée
    Acknowledged,
    /// Résolue
    Resolved,
    /// Fermée
    Closed,
}

/// Système de récupération automatique
#[derive(Debug)]
pub struct AutoRecoverySystem {
    /// Configuration
    config: AutoRecoveryConfig,
    /// Tentatives de récupération en cours
    active_recoveries: Arc<RwLock<HashMap<NodeId, RecoveryAttempt>>>,
    /// Historique des récupérations
    recovery_history: Arc<RwLock<Vec<RecoveryRecord>>>,
}

/// Tentative de récupération
#[derive(Debug, Clone)]
pub struct RecoveryAttempt {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Action de récupération
    pub action: RecoveryAction,
    /// Tentative numéro
    pub attempt_number: u32,
    /// Heure de début
    pub started_at: SystemTime,
    /// Statut
    pub status: RecoveryStatus,
}

/// Statut de récupération
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryStatus {
    /// En cours
    InProgress,
    /// Réussie
    Successful,
    /// Échouée
    Failed,
    /// Annulée
    Cancelled,
}

/// Enregistrement de récupération
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryRecord {
    /// Nœud concerné
    pub node_id: NodeId,
    /// Action effectuée
    pub action: RecoveryAction,
    /// Nombre de tentatives
    pub total_attempts: u32,
    /// Heure de début
    pub started_at: chrono::DateTime<chrono::Utc>,
    /// Heure de fin
    pub completed_at: chrono::DateTime<chrono::Utc>,
    /// Résultat final
    pub final_status: RecoveryStatus,
    /// Détails
    pub details: String,
}

/// Système d'alertes
#[derive(Debug)]
pub struct AlertSystem {
    /// Configuration
    config: AlertConfig,
    /// Alertes actives
    active_alerts: Arc<RwLock<HashMap<String, HealthAlert>>>,
    /// Historique des alertes
    alert_history: Arc<RwLock<VecDeque<HealthAlert>>>,
    /// Canaux d'alerte configurés
    alert_channels: Vec<AlertChannel>,
}

/// Moniteur de santé principal
pub struct HealthMonitor {
    /// Configuration
    config: HealthMonitorConfig,
    /// État de santé des nœuds
    node_health: Arc<RwLock<HashMap<NodeId, NodeHealth>>>,
    /// Métriques de performance
    performance_metrics: Arc<RwLock<HashMap<NodeId, VecDeque<PerformanceMetrics>>>>,
    /// Système d'alertes
    alert_system: Arc<Mutex<AlertSystem>>,
    /// Système de récupération automatique
    auto_recovery: Arc<Mutex<AutoRecoverySystem>>,
    /// Dernière vérification globale
    last_global_check: Arc<Mutex<SystemTime>>,
    /// Statistiques du monitoring
    monitoring_stats: Arc<RwLock<MonitoringStats>>,
}

/// Statistiques du monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringStats {
    /// Nombre total de checks
    pub total_health_checks: u64,
    /// Checks réussis
    pub successful_checks: u64,
    /// Checks échoués
    pub failed_checks: u64,
    /// Alertes générées
    pub alerts_generated: u64,
    /// Récupérations tentées
    pub recoveries_attempted: u64,
    /// Récupérations réussies
    pub recoveries_successful: u64,
    /// Temps moyen de check
    pub average_check_time: Duration,
    /// Disponibilité moyenne
    pub average_availability: f64,
}

impl Default for HealthMonitorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(30),
            check_timeout: Duration::from_secs(10),
            failure_threshold: 3,
            alert_interval: Duration::from_secs(300), // 5 minutes
            auto_recovery_enabled: true,
            max_recovery_attempts: 3,
            metrics_config: MetricsCollectionConfig::default(),
            alert_config: AlertConfig::default(),
            recovery_config: AutoRecoveryConfig::default(),
        }
    }
}

impl Default for MetricsCollectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            collection_interval: Duration::from_secs(60),
            history_size: 1000,
            metrics_to_collect: vec![
                MetricType::CpuUsage,
                MetricType::MemoryUsage,
                MetricType::StorageUsage,
                MetricType::NetworkLatency,
                MetricType::ErrorRate,
            ],
            aggregation_enabled: true,
            aggregation_window: Duration::from_secs(300), // 5 minutes
        }
    }
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            alert_channels: vec![AlertChannel::Log],
            thresholds: AlertThresholds::default(),
            escalation_enabled: true,
            escalation_delay: Duration::from_secs(1800), // 30 minutes
        }
    }
}

impl Default for AlertThresholds {
    fn default() -> Self {
        Self {
            cpu_critical: 90.0,
            cpu_warning: 75.0,
            memory_critical: 90.0,
            memory_warning: 75.0,
            storage_critical: 95.0,
            storage_warning: 85.0,
            latency_critical: 1000, // 1 seconde
            latency_warning: 500,   // 500ms
            error_rate_critical: 10.0, // 10%
            error_rate_warning: 5.0,   // 5%
        }
    }
}

impl Default for AutoRecoveryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            recovery_actions: vec![
                RecoveryAction::ClearCache,
                RecoveryAction::ResetConnections,
                RecoveryAction::RestartNode,
            ],
            retry_delay: Duration::from_secs(60),
            exponential_backoff: true,
            max_retry_delay: Duration::from_secs(600), // 10 minutes
        }
    }
}

impl HealthMonitor {
    /// Crée un nouveau moniteur de santé
    pub async fn new(config: HealthMonitorConfig) -> Result<Self> {
        let alert_system = AlertSystem {
            config: config.alert_config.clone(),
            active_alerts: Arc::new(RwLock::new(HashMap::new())),
            alert_history: Arc::new(RwLock::new(VecDeque::new())),
            alert_channels: config.alert_config.alert_channels.clone(),
        };

        let auto_recovery = AutoRecoverySystem {
            config: config.recovery_config.clone(),
            active_recoveries: Arc::new(RwLock::new(HashMap::new())),
            recovery_history: Arc::new(RwLock::new(Vec::new())),
        };

        let monitoring_stats = MonitoringStats {
            total_health_checks: 0,
            successful_checks: 0,
            failed_checks: 0,
            alerts_generated: 0,
            recoveries_attempted: 0,
            recoveries_successful: 0,
            average_check_time: Duration::ZERO,
            average_availability: 1.0,
        };

        Ok(Self {
            config,
            node_health: Arc::new(RwLock::new(HashMap::new())),
            performance_metrics: Arc::new(RwLock::new(HashMap::new())),
            alert_system: Arc::new(Mutex::new(alert_system)),
            auto_recovery: Arc::new(Mutex::new(auto_recovery)),
            last_global_check: Arc::new(Mutex::new(SystemTime::now())),
            monitoring_stats: Arc::new(RwLock::new(monitoring_stats)),
        })
    }

    /// Effectue un check de santé sur un nœud spécifique
    pub async fn check_node_health(&self, node_id: &NodeId, node: &dyn super::Node) -> Result<NodeHealth> {
        let check_start = SystemTime::now();
        
        // Met à jour les statistiques
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.total_health_checks += 1;
        }

        // Effectue le check de santé
        match tokio::time::timeout(self.config.check_timeout, node.health_check()).await {
            Ok(Ok(health)) => {
                // Check réussi
                {
                    let mut node_health = self.node_health.write().await;
                    node_health.insert(node_id.clone(), health.clone());
                }

                // Met à jour les statistiques de succès
                {
                    let mut stats = self.monitoring_stats.write().await;
                    stats.successful_checks += 1;
                    let check_time = check_start.elapsed().unwrap_or(Duration::ZERO);
                    stats.average_check_time = (stats.average_check_time + check_time) / 2;
                }

                // Vérifie les seuils d'alerte
                self.check_alert_thresholds(node_id, &health).await?;

                Ok(health)
            },
            Ok(Err(e)) => {
                // Erreur lors du check
                {
                    let mut stats = self.monitoring_stats.write().await;
                    stats.failed_checks += 1;
                }

                // Crée une alerte
                self.create_alert(node_id, AlertType::NodeUnresponsive, AlertSeverity::Critical, 
                    format!("Erreur lors du health check: {}", e)).await?;

                Err(e)
            },
            Err(_) => {
                // Timeout
                {
                    let mut stats = self.monitoring_stats.write().await;
                    stats.failed_checks += 1;
                }

                // Crée une alerte de timeout
                self.create_alert(node_id, AlertType::NodeUnresponsive, AlertSeverity::Critical,
                    "Timeout lors du health check".to_string()).await?;

                Err(crate::error::CoreError::Timeout {
                    message: "Health check timeout".to_string(),
                })
            }
        }
    }

    /// Vérifie les seuils d'alerte pour un nœud
    async fn check_alert_thresholds(&self, node_id: &NodeId, health: &NodeHealth) -> Result<()> {
        let thresholds = &self.config.alert_config.thresholds;

        // Vérifie CPU
        if health.cpu_usage > thresholds.cpu_critical {
            self.create_alert(node_id, AlertType::HighResourceUsage, AlertSeverity::Critical,
                format!("Utilisation CPU critique: {:.1}%", health.cpu_usage * 100.0)).await?;
        } else if health.cpu_usage > thresholds.cpu_warning {
            self.create_alert(node_id, AlertType::HighResourceUsage, AlertSeverity::Warning,
                format!("Utilisation CPU élevée: {:.1}%", health.cpu_usage * 100.0)).await?;
        }

        // Vérifie mémoire
        if health.memory_usage > thresholds.memory_critical {
            self.create_alert(node_id, AlertType::HighResourceUsage, AlertSeverity::Critical,
                format!("Utilisation mémoire critique: {:.1}%", health.memory_usage * 100.0)).await?;
        } else if health.memory_usage > thresholds.memory_warning {
            self.create_alert(node_id, AlertType::HighResourceUsage, AlertSeverity::Warning,
                format!("Utilisation mémoire élevée: {:.1}%", health.memory_usage * 100.0)).await?;
        }

        // Vérifie stockage
        if health.storage_usage > thresholds.storage_critical {
            self.create_alert(node_id, AlertType::LowDiskSpace, AlertSeverity::Critical,
                format!("Espace disque critique: {:.1}%", health.storage_usage * 100.0)).await?;
        } else if health.storage_usage > thresholds.storage_warning {
            self.create_alert(node_id, AlertType::LowDiskSpace, AlertSeverity::Warning,
                format!("Espace disque faible: {:.1}%", health.storage_usage * 100.0)).await?;
        }

        // Vérifie latence
        let latency_ms = health.network_latency.as_millis() as u64;
        if latency_ms > thresholds.latency_critical {
            self.create_alert(node_id, AlertType::HighLatency, AlertSeverity::Critical,
                format!("Latence critique: {}ms", latency_ms)).await?;
        } else if latency_ms > thresholds.latency_warning {
            self.create_alert(node_id, AlertType::HighLatency, AlertSeverity::Warning,
                format!("Latence élevée: {}ms", latency_ms)).await?;
        }

        // Vérifie taux d'erreur
        if health.error_rate > thresholds.error_rate_critical {
            self.create_alert(node_id, AlertType::HighErrorRate, AlertSeverity::Critical,
                format!("Taux d'erreur critique: {:.1}%", health.error_rate * 100.0)).await?;
        } else if health.error_rate > thresholds.error_rate_warning {
            self.create_alert(node_id, AlertType::HighErrorRate, AlertSeverity::Warning,
                format!("Taux d'erreur élevé: {:.1}%", health.error_rate * 100.0)).await?;
        }

        Ok(())
    }

    /// Crée une nouvelle alerte
    async fn create_alert(&self, node_id: &NodeId, alert_type: AlertType, severity: AlertSeverity, message: String) -> Result<()> {
        let alert_id = uuid::Uuid::new_v4().to_string();
        
        let alert = HealthAlert {
            alert_id: alert_id.clone(),
            node_id: node_id.clone(),
            alert_type: alert_type.clone(),
            severity: severity.clone(),
            message: message.clone(),
            created_at: chrono::Utc::now(),
            status: AlertStatus::Active,
            recommended_actions: self.get_recommended_actions(&alert_type),
        };

        // Ajoute à la liste des alertes actives
        {
            let mut alert_system = self.alert_system.lock().await;
            alert_system.active_alerts.write().await.insert(alert_id, alert.clone());
            
            // Ajoute à l'historique
            let mut history = alert_system.alert_history.write().await;
            history.push_back(alert.clone());
            
            // Garde seulement les 1000 dernières alertes
            if history.len() > 1000 {
                history.pop_front();
            }
        }

        // Met à jour les statistiques
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.alerts_generated += 1;
        }

        // Envoie l'alerte via les canaux configurés
        self.send_alert_notification(&alert).await?;

        // Déclenche la récupération automatique si activée
        if self.config.auto_recovery_enabled && severity >= AlertSeverity::Error {
            self.trigger_auto_recovery(node_id, &alert_type).await?;
        }

        log::warn!("Alerte créée: {} - {} - {}", alert_type_to_string(&alert_type), severity_to_string(&severity), message);
        Ok(())
    }

    /// Envoie une notification d'alerte
    async fn send_alert_notification(&self, alert: &HealthAlert) -> Result<()> {
        let alert_system = self.alert_system.lock().await;
        
        for channel in &alert_system.alert_channels {
            match channel {
                AlertChannel::Log => {
                    log::error!("ALERTE [{}]: {} - {}", 
                        severity_to_string(&alert.severity),
                        alert_type_to_string(&alert.alert_type),
                        alert.message
                    );
                },
                AlertChannel::Email => {
                    // Simulation d'envoi d'email
                    log::info!("Email d'alerte envoyé pour: {}", alert.alert_id);
                },
                AlertChannel::Webhook => {
                    // Simulation d'appel webhook
                    log::info!("Webhook d'alerte appelé pour: {}", alert.alert_id);
                },
                AlertChannel::Slack => {
                    // Simulation de notification Slack
                    log::info!("Notification Slack envoyée pour: {}", alert.alert_id);
                },
                AlertChannel::SMS => {
                    // Simulation d'envoi SMS
                    log::info!("SMS d'alerte envoyé pour: {}", alert.alert_id);
                },
            }
        }

        Ok(())
    }

    /// Déclenche la récupération automatique
    async fn trigger_auto_recovery(&self, node_id: &NodeId, alert_type: &AlertType) -> Result<()> {
        let mut auto_recovery = self.auto_recovery.lock().await;
        
        // Vérifie si une récupération est déjà en cours
        {
            let active_recoveries = auto_recovery.active_recoveries.read().await;
            if active_recoveries.contains_key(node_id) {
                log::debug!("Récupération déjà en cours pour le nœud {:?}", node_id);
                return Ok(());
            }
        }

        // Détermine l'action de récupération appropriée
        let recovery_action = match alert_type {
            AlertType::HighResourceUsage => RecoveryAction::ClearCache,
            AlertType::HighLatency => RecoveryAction::ResetConnections,
            AlertType::NodeUnresponsive => RecoveryAction::RestartNode,
            AlertType::ConnectivityIssue => RecoveryAction::ResetConnections,
            AlertType::SyncIssue => RecoveryAction::Resynchronize,
            _ => RecoveryAction::RestartNode, // Action par défaut
        };

        // Démarre la tentative de récupération
        let recovery_attempt = RecoveryAttempt {
            node_id: node_id.clone(),
            action: recovery_action.clone(),
            attempt_number: 1,
            started_at: SystemTime::now(),
            status: RecoveryStatus::InProgress,
        };

        {
            let mut active_recoveries = auto_recovery.active_recoveries.write().await;
            active_recoveries.insert(node_id.clone(), recovery_attempt);
        }

        // Met à jour les statistiques
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.recoveries_attempted += 1;
        }

        log::info!("Récupération automatique démarrée pour {:?}: {:?}", node_id, recovery_action);

        // Simule l'exécution de l'action de récupération
        // Dans la réalité, cela appellerait les méthodes appropriées du nœud
        tokio::spawn(self.execute_recovery_action(node_id.clone(), recovery_action));

        Ok(())
    }

    /// Exécute une action de récupération
    async fn execute_recovery_action(&self, node_id: NodeId, action: RecoveryAction) -> Result<()> {
        log::info!("Exécution de l'action de récupération {:?} pour {:?}", action, node_id);

        // Simulation de l'exécution de l'action
        match action {
            RecoveryAction::RestartNode => {
                tokio::time::sleep(Duration::from_secs(5)).await;
            },
            RecoveryAction::ClearCache => {
                tokio::time::sleep(Duration::from_secs(2)).await;
            },
            RecoveryAction::ResetConnections => {
                tokio::time::sleep(Duration::from_secs(3)).await;
            },
            RecoveryAction::Resynchronize => {
                tokio::time::sleep(Duration::from_secs(10)).await;
            },
            RecoveryAction::SwitchMode => {
                tokio::time::sleep(Duration::from_secs(1)).await;
            },
        }

        // Marque la récupération comme terminée
        {
            let auto_recovery = self.auto_recovery.lock().await;
            let mut active_recoveries = auto_recovery.active_recoveries.write().await;
            if let Some(mut attempt) = active_recoveries.remove(&node_id) {
                attempt.status = RecoveryStatus::Successful;
                
                // Ajoute à l'historique
                let mut history = auto_recovery.recovery_history.write().await;
                history.push(RecoveryRecord {
                    node_id: node_id.clone(),
                    action,
                    total_attempts: attempt.attempt_number,
                    started_at: chrono::DateTime::from_timestamp(
                        attempt.started_at.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or(Duration::ZERO).as_secs() as i64, 0
                    ).unwrap_or_else(chrono::Utc::now),
                    completed_at: chrono::Utc::now(),
                    final_status: RecoveryStatus::Successful,
                    details: "Récupération automatique réussie".to_string(),
                });
            }
        }

        // Met à jour les statistiques
        {
            let mut stats = self.monitoring_stats.write().await;
            stats.recoveries_successful += 1;
        }

        log::info!("Action de récupération {:?} terminée avec succès pour {:?}", action, node_id);
        Ok(())
    }

    /// Obtient les actions recommandées pour un type d'alerte
    fn get_recommended_actions(&self, alert_type: &AlertType) -> Vec<RecoveryAction> {
        match alert_type {
            AlertType::HighResourceUsage => vec![RecoveryAction::ClearCache],
            AlertType::HighLatency => vec![RecoveryAction::ResetConnections],
            AlertType::HighErrorRate => vec![RecoveryAction::RestartNode],
            AlertType::NodeUnresponsive => vec![RecoveryAction::RestartNode],
            AlertType::ConnectivityIssue => vec![RecoveryAction::ResetConnections],
            AlertType::LowDiskSpace => vec![RecoveryAction::ClearCache],
            AlertType::SyncIssue => vec![RecoveryAction::Resynchronize],
        }
    }

    /// Collecte les métriques de performance
    pub async fn collect_metrics(&self, node_id: &NodeId, node: &dyn super::Node) -> Result<()> {
        if !self.config.metrics_config.enabled {
            return Ok(());
        }

        // Obtient les métriques du nœud
        let node_metrics = node.get_metrics().await?;
        let general_metrics = node_metrics.general_metrics();

        // Crée les métriques de performance
        let mut metrics_map = HashMap::new();
        for metric_type in &self.config.metrics_config.metrics_to_collect {
            let value = match metric_type {
                MetricType::CpuUsage => general_metrics.cpu_usage,
                MetricType::MemoryUsage => general_metrics.memory_usage,
                MetricType::StorageUsage => general_metrics.storage_usage,
                MetricType::NetworkLatency => general_metrics.average_latency.as_millis() as f64,
                MetricType::Bandwidth => (general_metrics.bandwidth_in + general_metrics.bandwidth_out) as f64,
                MetricType::ConnectionCount => general_metrics.active_connections as f64,
                MetricType::ErrorRate => if general_metrics.messages_processed > 0 {
                    general_metrics.error_count as f64 / general_metrics.messages_processed as f64
                } else {
                    0.0
                },
                MetricType::ResponseTime => general_metrics.average_latency.as_millis() as f64,
            };
            metrics_map.insert(metric_type.clone(), value);
        }

        let performance_metrics = PerformanceMetrics {
            node_id: node_id.clone(),
            timestamp: chrono::Utc::now(),
            metrics: metrics_map,
            metadata: HashMap::new(),
        };

        // Stocke les métriques
        {
            let mut metrics_store = self.performance_metrics.write().await;
            let node_metrics = metrics_store.entry(node_id.clone()).or_insert_with(VecDeque::new);
            node_metrics.push_back(performance_metrics);

            // Garde seulement l'historique configuré
            while node_metrics.len() > self.config.metrics_config.history_size {
                node_metrics.pop_front();
            }
        }

        Ok(())
    }

    /// Obtient les statistiques de monitoring
    pub async fn get_monitoring_stats(&self) -> MonitoringStats {
        let stats = self.monitoring_stats.read().await;
        stats.clone()
    }

    /// Obtient la santé de tous les nœuds
    pub async fn get_all_node_health(&self) -> HashMap<NodeId, NodeHealth> {
        let node_health = self.node_health.read().await;
        node_health.clone()
    }

    /// Obtient les alertes actives
    pub async fn get_active_alerts(&self) -> Vec<HealthAlert> {
        let alert_system = self.alert_system.lock().await;
        let alerts = alert_system.active_alerts.read().await;
        alerts.values().cloned().collect()
    }
}

/// Convertit un type d'alerte en chaîne
fn alert_type_to_string(alert_type: &AlertType) -> &'static str {
    match alert_type {
        AlertType::HighResourceUsage => "Utilisation ressources élevée",
        AlertType::HighLatency => "Latence élevée",
        AlertType::HighErrorRate => "Taux d'erreur élevé",
        AlertType::NodeUnresponsive => "Nœud non répondant",
        AlertType::ConnectivityIssue => "Problème connectivité",
        AlertType::LowDiskSpace => "Espace disque faible",
        AlertType::SyncIssue => "Problème synchronisation",
    }
}

/// Convertit un niveau de sévérité en chaîne
fn severity_to_string(severity: &AlertSeverity) -> &'static str {
    match severity {
        AlertSeverity::Info => "INFO",
        AlertSeverity::Warning => "WARNING",
        AlertSeverity::Error => "ERROR",
        AlertSeverity::Critical => "CRITICAL",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_monitor_creation() {
        let config = HealthMonitorConfig::default();
        let monitor = HealthMonitor::new(config).await;
        assert!(monitor.is_ok());
    }

    #[test]
    fn test_alert_thresholds() {
        let thresholds = AlertThresholds::default();
        assert!(thresholds.cpu_critical > thresholds.cpu_warning);
        assert!(thresholds.memory_critical > thresholds.memory_warning);
        assert!(thresholds.storage_critical > thresholds.storage_warning);
    }

    #[test]
    fn test_recovery_actions() {
        let action = RecoveryAction::RestartNode;
        assert_eq!(action, RecoveryAction::RestartNode);
        assert_ne!(action, RecoveryAction::ClearCache);
    }

    #[test]
    fn test_health_status() {
        let status = HealthStatus::Healthy;
        assert_eq!(status, HealthStatus::Healthy);
        assert_ne!(status, HealthStatus::Critical);
    }

    #[test]
    fn test_alert_severity_ordering() {
        assert!(AlertSeverity::Critical > AlertSeverity::Error);
        assert!(AlertSeverity::Error > AlertSeverity::Warning);
        assert!(AlertSeverity::Warning > AlertSeverity::Info);
    }
}