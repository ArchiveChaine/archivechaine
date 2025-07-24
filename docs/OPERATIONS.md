# Guide Opérationnel ArchiveChain

## Table des Matières

- [Vue d'Ensemble](#vue-densemble)
- [Déploiement en Production](#déploiement-en-production)
- [Monitoring et Observabilité](#monitoring-et-observabilité)
- [Gestion des Incidents](#gestion-des-incidents)
- [Maintenance et Mises à Jour](#maintenance-et-mises-à-jour)
- [Sécurité Opérationnelle](#sécurité-opérationnelle)
- [Sauvegarde et Récupération](#sauvegarde-et-récupération)
- [Scaling et Performance](#scaling-et-performance)
- [Automation et CI/CD](#automation-et-cicd)
- [Runbooks](#runbooks)

## Vue d'Ensemble

Ce guide fournit toutes les informations nécessaires pour opérer ArchiveChain en production, incluant le déploiement, le monitoring, la maintenance et la résolution d'incidents.

### Environnements Recommandés

```yaml
Development:
  Purpose: Tests et développement local
  Infrastructure: Local/Docker
  Monitoring: Basique
  Backup: Non requis

Staging:
  Purpose: Tests d'intégration et validation
  Infrastructure: Cloud/On-premise réduit
  Monitoring: Complet
  Backup: Quotidien

Production:
  Purpose: Réseau principal
  Infrastructure: Multi-région avec HA
  Monitoring: 24/7 avec alertes
  Backup: Temps réel + rétention longue
```

## Déploiement en Production

### Architecture de Production

#### Infrastructure Multi-Région
```yaml
# Exemple de topologie production
Global Load Balancer:
  - Geographic routing
  - Health checks
  - DDoS protection

Regions:
  Primary (US-East):
    - 3x Full Archive Nodes (m5.4xlarge)
    - 2x Gateway Nodes (c5.2xlarge)
    - 5x Light Storage Nodes (m5.xlarge)
    - 2x Relay Nodes (c5.xlarge)
    
  Secondary (EU-West):
    - 2x Full Archive Nodes (m5.4xlarge)
    - 1x Gateway Node (c5.2xlarge)
    - 3x Light Storage Nodes (m5.xlarge)
    - 1x Relay Node (c5.xlarge)
    
  Tertiary (AP-Southeast):
    - 1x Full Archive Node (m5.4xlarge)
    - 1x Gateway Node (c5.2xlarge)
    - 2x Light Storage Nodes (m5.xlarge)
    - 1x Relay Node (c5.xlarge)

Storage:
  - Primary: AWS EBS gp3 avec snapshots
  - Backup: S3 Cross-Region Replication
  - Archive: Glacier Deep Archive

Networking:
  - VPC avec subnets privés/publics
  - NAT Gateways pour sortie sécurisée
  - VPN/Direct Connect pour accès admin
  - CloudFront pour APIs publiques
```

#### Configuration Kubernetes Production
```yaml
# k8s/production/namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: archivechain-prod
  labels:
    name: archivechain-prod
    environment: production

---
# k8s/production/full-archive-statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: archivechain-full-archive
  namespace: archivechain-prod
spec:
  serviceName: archivechain-full-archive
  replicas: 3
  podManagementPolicy: Parallel
  
  selector:
    matchLabels:
      app: archivechain-full-archive
      
  template:
    metadata:
      labels:
        app: archivechain-full-archive
        version: v1.0.0
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9999"
        
    spec:
      securityContext:
        runAsNonRoot: true
        runAsUser: 1000
        fsGroup: 1000
        
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values: ["archivechain-full-archive"]
            topologyKey: kubernetes.io/hostname
            
      containers:
      - name: archivechain-node
        image: archivechain/node:v1.0.0
        
        ports:
        - containerPort: 9090
          name: p2p
        - containerPort: 9999
          name: metrics
          
        env:
        - name: RUST_LOG
          value: "archivechain=info,libp2p=warn"
        - name: ARCHIVECHAIN_CONFIG
          value: "/config/full_archive.toml"
        - name: POD_IP
          valueFrom:
            fieldRef:
              fieldPath: status.podIP
              
        resources:
          requests:
            memory: "32Gi"
            cpu: "8"
            ephemeral-storage: "10Gi"
          limits:
            memory: "64Gi"
            cpu: "16"
            ephemeral-storage: "20Gi"
            
        volumeMounts:
        - name: config
          mountPath: /config
          readOnly: true
        - name: data
          mountPath: /data
        - name: keys
          mountPath: /keys
          readOnly: true
          
        livenessProbe:
          httpGet:
            path: /health
            port: 9999
          initialDelaySeconds: 30
          periodSeconds: 30
          timeoutSeconds: 10
          
        readinessProbe:
          httpGet:
            path: /ready
            port: 9999
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          
      volumes:
      - name: config
        configMap:
          name: archivechain-config
      - name: keys
        secret:
          secretName: archivechain-keys
          defaultMode: 0600
          
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd
      resources:
        requests:
          storage: 20Ti

---
# k8s/production/gateway-deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: archivechain-gateway
  namespace: archivechain-prod
spec:
  replicas: 3
  
  strategy:
    type: RollingUpdate
    rollingUpdate:
      maxUnavailable: 1
      maxSurge: 1
      
  selector:
    matchLabels:
      app: archivechain-gateway
      
  template:
    metadata:
      labels:
        app: archivechain-gateway
        version: v1.0.0
        
    spec:
      containers:
      - name: archivechain-gateway
        image: archivechain/gateway:v1.0.0
        
        ports:
        - containerPort: 8080
          name: rest-api
        - containerPort: 8081
          name: graphql
        - containerPort: 8082
          name: websocket
        - containerPort: 9091
          name: grpc
        - containerPort: 9999
          name: metrics
          
        env:
        - name: RUST_LOG
          value: "archivechain=info"
        - name: ARCHIVECHAIN_CONFIG
          value: "/config/gateway.toml"
          
        resources:
          requests:
            memory: "8Gi"
            cpu: "4"
          limits:
            memory: "16Gi"
            cpu: "8"
            
        volumeMounts:
        - name: config
          mountPath: /config
          readOnly: true
        - name: keys
          mountPath: /keys
          readOnly: true
          
      volumes:
      - name: config
        configMap:
          name: archivechain-gateway-config
      - name: keys
        secret:
          secretName: archivechain-gateway-keys

---
# k8s/production/service.yaml
apiVersion: v1
kind: Service
metadata:
  name: archivechain-gateway-service
  namespace: archivechain-prod
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: nlb
    service.beta.kubernetes.io/aws-load-balancer-cross-zone-load-balancing-enabled: "true"
spec:
  type: LoadBalancer
  selector:
    app: archivechain-gateway
  ports:
  - name: rest-api
    port: 80
    targetPort: 8080
    protocol: TCP
  - name: https
    port: 443
    targetPort: 8080
    protocol: TCP
  - name: grpc
    port: 9091
    targetPort: 9091
    protocol: TCP
```

### Configuration de Production

#### Configuration des Nœuds
```toml
# config/production/full_archive.toml
[node]
type = "full_archive"
identity_key = "/keys/node_identity.key"
data_dir = "/data/archivechain"
log_level = "info"
log_format = "json"

[storage]
capacity = "20TB"
replication_factor = 15
compression = true
deduplication = true
backup_enabled = true
backup_schedule = "0 */2 * * *"  # Every 2 hours
backup_retention = "30d"

[consensus]
participate = true
weight = 1.0
validator_key = "/keys/validator.key"
min_stake = "10000000000000000000000000"  # 10M ARC

[network]
listen_addr = "0.0.0.0:9090"
external_addr = "${POD_IP}:9090"
max_peers = 500
bandwidth_limit = "10GB/s"
bootstrap_peers = [
  "/dns4/bootstrap-1.archivechain.org/tcp/9090/p2p/12D3KooWCRscMgHgEo3ojm8ovzheydpvTEqsDtq7Wby38cMHrYjt",
  "/dns4/bootstrap-2.archivechain.org/tcp/9090/p2p/12D3KooWKnxJKRy2T2nwJP2NzJjkgFsZEJ2wz8WFmCJF5ZY8kHVk"
]

[performance]
max_concurrent_archives = 200
indexing_threads = 16
compression_level = 6
cache_size = "32GB"
batch_size = 2000
sync_interval = "30s"
checkpoint_interval = "300s"

[monitoring]
metrics_enabled = true
metrics_addr = "0.0.0.0:9999"
tracing_enabled = true
jaeger_endpoint = "http://jaeger-collector:14268/api/traces"

[security]
tls_enabled = true
tls_cert_file = "/keys/tls.crt"
tls_key_file = "/keys/tls.key"
rate_limit_enabled = true
rate_limit_requests_per_second = 1000
```

#### Configuration Gateway
```toml
# config/production/gateway.toml
[node]
type = "gateway"
identity_key = "/keys/gateway_identity.key"
data_dir = "/data/archivechain"
log_level = "info"
log_format = "json"

[api]
enabled = true

[api.rest]
port = 8080
max_connections = 20000
rate_limit = 5000
cors_enabled = true
cors_origins = ["https://app.archivechain.org", "https://dashboard.archivechain.org"]
swagger_ui = true
request_timeout = "30s"

[api.graphql]
port = 8081
max_query_depth = 20
query_complexity_limit = 2000
subscription_enabled = true
playground_enabled = false  # Disabled in production

[api.websocket]
port = 8082
max_connections = 10000
ping_interval = "30s"
compression = true
max_message_size = "1MB"

[api.grpc]
port = 9091
max_connections = 5000
reflection_enabled = false  # Disabled in production
keep_alive_time = "30s"
max_concurrent_streams = 1000

[security]
jwt_secret_file = "/keys/jwt_secret"
rate_limit_redis = "redis://redis-cluster:6379"
ddos_protection = true
waf_enabled = true
tls_enabled = true
tls_cert_file = "/keys/gateway_tls.crt"
tls_key_file = "/keys/gateway_tls.key"

[cache]
layers = ["memory", "redis", "disk"]
memory_size = "16GB"
redis_url = "redis://redis-cluster:6379"
redis_cluster_enabled = true
redis_pool_size = 50
disk_cache = "/data/cache"
ttl_default = "1h"
ttl_max = "24h"

[monitoring]
metrics_enabled = true
metrics_addr = "0.0.0.0:9999"
access_logs = true
error_logs = true
slow_query_threshold = "1s"
```

## Monitoring et Observabilité

### Stack de Monitoring

#### Prometheus Configuration
```yaml
# monitoring/prometheus/prometheus.yml
global:
  scrape_interval: 15s
  evaluation_interval: 15s
  external_labels:
    cluster: 'archivechain-prod'
    region: 'us-east-1'

rule_files:
  - "archivechain_alerts.yml"
  - "infrastructure_alerts.yml"

scrape_configs:
  # ArchiveChain nodes
  - job_name: 'archivechain-nodes'
    kubernetes_sd_configs:
    - role: pod
      namespaces:
        names: ['archivechain-prod']
    relabel_configs:
    - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_scrape]
      action: keep
      regex: true
    - source_labels: [__meta_kubernetes_pod_annotation_prometheus_io_port]
      action: replace
      target_label: __address__
      regex: (.+)
      replacement: ${1}:9999
    - source_labels: [__meta_kubernetes_pod_label_app]
      target_label: app
    - source_labels: [__meta_kubernetes_pod_label_version]
      target_label: version

  # Infrastructure
  - job_name: 'kubernetes-nodes'
    kubernetes_sd_configs:
    - role: node
    relabel_configs:
    - action: labelmap
      regex: __meta_kubernetes_node_label_(.+)

  # Redis
  - job_name: 'redis'
    static_configs:
    - targets: ['redis-exporter:9121']

alerting:
  alertmanagers:
  - kubernetes_sd_configs:
    - role: pod
      namespaces:
        names: ['monitoring']
    relabel_configs:
    - source_labels: [__meta_kubernetes_pod_label_app]
      action: keep
      regex: alertmanager

remote_write:
  - url: "https://prometheus-remote-write.monitoring.archivechain.org/api/v1/write"
    basic_auth:
      username: "prometheus"
      password_file: "/etc/prometheus/remote_write_password"
```

#### Alertes Critiques
```yaml
# monitoring/prometheus/archivechain_alerts.yml
groups:
- name: archivechain.critical
  interval: 30s
  rules:
  
  # Node health
  - alert: ArchiveChainNodeDown
    expr: up{job="archivechain-nodes"} == 0
    for: 1m
    labels:
      severity: critical
      team: archivechain
    annotations:
      summary: "ArchiveChain node {{ $labels.instance }} is down"
      description: "Node {{ $labels.instance }} has been down for more than 1 minute"
      runbook_url: "https://runbooks.archivechain.org/node-down"

  # Consensus issues
  - alert: ConsensusParticipationLow
    expr: archivechain_consensus_participation_rate < 0.7
    for: 5m
    labels:
      severity: critical
      team: archivechain
    annotations:
      summary: "Low consensus participation on {{ $labels.instance }}"
      description: "Consensus participation rate is {{ $value | humanizePercentage }}"
      runbook_url: "https://runbooks.archivechain.org/consensus-issues"

  # Storage issues
  - alert: HighStorageUsage
    expr: (archivechain_storage_used_bytes / archivechain_storage_total_bytes) > 0.85
    for: 5m
    labels:
      severity: warning
      team: archivechain
    annotations:
      summary: "High storage usage on {{ $labels.instance }}"
      description: "Storage usage is {{ $value | humanizePercentage }}"
      runbook_url: "https://runbooks.archivechain.org/storage-cleanup"

  - alert: StorageCorruption
    expr: archivechain_storage_corruption_detected > 0
    for: 0m
    labels:
      severity: critical
      team: archivechain
    annotations:
      summary: "Storage corruption detected on {{ $labels.instance }}"
      description: "{{ $value }} corrupted archives detected"
      runbook_url: "https://runbooks.archivechain.org/storage-corruption"

  # Network issues
  - alert: LowPeerCount
    expr: archivechain_network_peer_count < 5
    for: 2m
    labels:
      severity: warning
      team: archivechain
    annotations:
      summary: "Low peer count on {{ $labels.instance }}"
      description: "Only {{ $value }} peers connected"

  - alert: HighNetworkLatency
    expr: archivechain_network_average_latency_ms > 1000
    for: 5m
    labels:
      severity: warning
      team: archivechain
    annotations:
      summary: "High network latency on {{ $labels.instance }}"
      description: "Average latency is {{ $value }}ms"

  # API performance
  - alert: HighAPILatency
    expr: histogram_quantile(0.95, rate(archivechain_api_request_duration_seconds_bucket[5m])) > 2
    for: 3m
    labels:
      severity: warning
      team: archivechain
    annotations:
      summary: "High API latency on {{ $labels.instance }}"
      description: "95th percentile latency is {{ $value }}s"

  - alert: HighErrorRate
    expr: rate(archivechain_api_errors_total[5m]) / rate(archivechain_api_requests_total[5m]) > 0.05
    for: 2m
    labels:
      severity: critical
      team: archivechain
    annotations:
      summary: "High API error rate on {{ $labels.instance }}"
      description: "Error rate is {{ $value | humanizePercentage }}"

- name: archivechain.business
  interval: 1m
  rules:
  
  # Archive processing
  - alert: ArchiveProcessingStalled
    expr: increase(archivechain_archives_processed_total[10m]) == 0
    for: 10m
    labels:
      severity: warning
      team: archivechain
    annotations:
      summary: "Archive processing stalled on {{ $labels.instance }}"
      description: "No archives processed in the last 10 minutes"

  - alert: HighArchiveFailureRate
    expr: rate(archivechain_archives_failed_total[5m]) / rate(archivechain_archives_attempted_total[5m]) > 0.1
    for: 5m
    labels:
      severity: warning
      team: archivechain
    annotations:
      summary: "High archive failure rate on {{ $labels.instance }}"
      description: "Archive failure rate is {{ $value | humanizePercentage }}"

  # Economic metrics
  - alert: TokenSupplyAnomaly
    expr: abs(archivechain_token_supply_total - archivechain_token_supply_expected) > 1000000
    for: 1m
    labels:
      severity: critical
      team: archivechain
    annotations:
      summary: "Token supply anomaly detected"
      description: "Supply difference: {{ $value }} tokens"
```

#### Grafana Dashboards
```json
{
  "dashboard": {
    "title": "ArchiveChain - Network Overview",
    "tags": ["archivechain", "blockchain", "network"],
    "time": {
      "from": "now-1h",
      "to": "now"
    },
    "panels": [
      {
        "title": "Active Nodes",
        "type": "stat",
        "targets": [
          {
            "expr": "sum(up{job=\"archivechain-nodes\"})",
            "legendFormat": "Active Nodes"
          }
        ],
        "fieldConfig": {
          "defaults": {
            "color": {
              "mode": "thresholds"
            },
            "thresholds": {
              "steps": [
                {"color": "red", "value": 0},
                {"color": "yellow", "value": 10},
                {"color": "green", "value": 20}
              ]
            }
          }
        }
      },
      {
        "title": "Block Height",
        "type": "graph",
        "targets": [
          {
            "expr": "archivechain_blockchain_height",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Archive Processing Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(archivechain_archives_processed_total[5m])",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Network Latency",
        "type": "graph",
        "targets": [
          {
            "expr": "archivechain_network_average_latency_ms",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "Storage Usage",
        "type": "bargauge",
        "targets": [
          {
            "expr": "archivechain_storage_used_bytes / archivechain_storage_total_bytes * 100",
            "legendFormat": "{{ instance }}"
          }
        ]
      },
      {
        "title": "API Request Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(archivechain_api_requests_total[5m])",
            "legendFormat": "{{ method }} {{ endpoint }}"
          }
        ]
      }
    ]
  }
}
```

### Logging et Tracing

#### Structured Logging
```rust
// Configuration des logs pour production
use tracing::{info, error, debug, warn};
use tracing_subscriber::{layer::SubscriberExt, Registry};

pub fn setup_production_logging() -> Result<()> {
    let json_layer = tracing_subscriber::fmt::layer()
        .json()
        .with_target(true)
        .with_current_span(false)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    let jaeger_tracer = opentelemetry_jaeger::new_pipeline()
        .with_service_name("archivechain-node")
        .with_agent_endpoint("http://jaeger-agent:14268/api/traces")
        .install_batch(opentelemetry::runtime::Tokio)?;

    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(jaeger_tracer);

    let filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "archivechain=info,libp2p=warn,hyper=warn".into());

    let subscriber = Registry::default()
        .with(filter)
        .with(json_layer)
        .with(telemetry_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

// Exemple d'utilisation avec contexte
#[tracing::instrument(skip(self))]
pub async fn process_archive_request(&self, request: ArchiveRequest) -> Result<ArchiveId> {
    let archive_id = ArchiveId::generate();
    
    info!(
        archive_id = %archive_id,
        url = %request.url,
        size = request.expected_size,
        "Starting archive processing"
    );

    match self.download_content(&request.url).await {
        Ok(content) => {
            debug!(
                archive_id = %archive_id,
                actual_size = content.len(),
                "Content downloaded successfully"
            );
            
            self.store_archive(archive_id, content).await?;
            
            info!(
                archive_id = %archive_id,
                "Archive processing completed successfully"
            );
            
            Ok(archive_id)
        }
        Err(e) => {
            error!(
                archive_id = %archive_id,
                error = %e,
                "Failed to download content"
            );
            Err(e)
        }
    }
}
```

#### Log Aggregation avec ELK Stack
```yaml
# logging/filebeat.yml
filebeat.inputs:
- type: kubernetes
  node: ${NODE_NAME}
  hints.enabled: true
  hints.default_config:
    type: container
    paths:
    - /var/log/containers/*-${data.kubernetes.container.id}.log

processors:
- add_kubernetes_metadata:
    host: ${NODE_NAME}
    matchers:
    - logs_path:
        logs_path: "/var/log/containers/"

- add_host_metadata:
    when.not.contains.tags: forwarded

output.elasticsearch:
  hosts: ["elasticsearch-cluster:9200"]
  username: "filebeat"
  password: "${ELASTICSEARCH_PASSWORD}"
  index: "archivechain-logs-%{+yyyy.MM.dd}"
  template.name: "archivechain"
  template.pattern: "archivechain-logs-*"

setup.kibana:
  host: "kibana:5601"
  username: "filebeat"
  password: "${KIBANA_PASSWORD}"
```

## Gestion des Incidents

### Classification des Incidents

#### Niveaux de Sévérité
```yaml
P0 - Critique (Response: 15min, Resolution: 4h):
  - Réseau principal hors service
  - Perte de données critique
  - Compromission de sécurité majeure
  - Corruption de la blockchain

P1 - Élevé (Response: 1h, Resolution: 24h):
  - Dégradation significative des performances
  - APIs partiellement indisponibles
  - Problèmes de consensus intermittents
  - Perte de connectivité d'une région

P2 - Moyen (Response: 4h, Resolution: 72h):
  - Problèmes de performance mineurs
  - Features non-critiques indisponibles
  - Alertes de monitoring non-critiques

P3 - Faible (Response: 24h, Resolution: 1 semaine):
  - Problèmes cosmétiques
  - Optimisations de performance
  - Mises à jour de documentation
```

### Processus de Réponse aux Incidents

#### 1. Détection et Alerte
```yaml
Sources d'Alertes:
  - Prometheus/Alertmanager
  - Application logs (ERROR level)
  - User reports via support
  - Health check failures
  - Third-party monitoring (Pingdom, Datadog)

Escalation automatique:
  - PagerDuty integration
  - Slack notifications (#incidents)
  - Email alerts pour P0/P1
  - SMS pour P0 après 30min
```

#### 2. War Room et Communication
```markdown
# Incident Response Playbook

## Initial Response (First 15 minutes)
1. **Acknowledge** the incident in PagerDuty
2. **Create** incident channel in Slack (#incident-YYYY-MM-DD-XXX)
3. **Post** initial status to #incidents
4. **Assign** incident commander
5. **Start** gathering information

## Assessment (Next 30 minutes)
1. **Determine** scope and impact
2. **Classify** severity level
3. **Notify** stakeholders based on severity
4. **Create** incident document
5. **Begin** mitigation efforts

## Communication Templates
### P0 Initial Notice
"🚨 INCIDENT P0: [Brief Description]
Status: Investigating
Impact: [User-facing impact]
ETA: Investigating
Updates: Every 15 minutes"

### Resolution Notice
"✅ INCIDENT RESOLVED: [Brief Description]
Duration: [Start] - [End] ([Duration])
Root Cause: [Brief explanation]
Post-mortem: [Link when available]"
```

### Runbooks Automatisés

#### Common Issues Runbook
```bash
#!/bin/bash
# runbooks/node_recovery.sh

set -euo pipefail

INCIDENT_ID="$1"
NODE_ID="$2"
NAMESPACE="archivechain-prod"

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [RUNBOOK] $1"
}

log "Starting node recovery runbook for incident $INCIDENT_ID, node $NODE_ID"

# Step 1: Check node status
log "Checking node status..."
NODE_STATUS=$(kubectl get pod "$NODE_ID" -n "$NAMESPACE" -o jsonpath='{.status.phase}')
log "Node status: $NODE_STATUS"

if [ "$NODE_STATUS" = "Running" ]; then
    log "Node is running, checking health endpoint..."
    if kubectl exec "$NODE_ID" -n "$NAMESPACE" -- curl -f http://localhost:9999/health; then
        log "Health check passed, checking connectivity..."
        PEER_COUNT=$(kubectl exec "$NODE_ID" -n "$NAMESPACE" -- archivechain-cli network peers | jq length)
        log "Peer count: $PEER_COUNT"
        
        if [ "$PEER_COUNT" -lt 3 ]; then
            log "Low peer count, restarting networking..."
            kubectl exec "$NODE_ID" -n "$NAMESPACE" -- pkill -SIGUSR1 archivechain-node
            sleep 30
        fi
    else
        log "Health check failed, attempting graceful restart..."
        kubectl delete pod "$NODE_ID" -n "$NAMESPACE"
        
        # Wait for pod to restart
        kubectl wait --for=condition=Ready pod -l "app=archivechain-full-archive" -n "$NAMESPACE" --timeout=300s
    fi
else
    log "Node not running, checking for persistent volume issues..."
    PV_STATUS=$(kubectl get pvc "data-$NODE_ID" -n "$NAMESPACE" -o jsonpath='{.status.phase}')
    log "PV status: $PV_STATUS"
    
    if [ "$PV_STATUS" != "Bound" ]; then
        log "PV issue detected, escalating to storage team..."
        curl -X POST "$PAGERDUTY_WEBHOOK" \
            -H "Content-Type: application/json" \
            -d "{\"incident_id\": \"$INCIDENT_ID\", \"escalation\": \"storage-team\", \"details\": \"PV not bound for $NODE_ID\"}"
    fi
fi

log "Node recovery runbook completed"
```

#### Performance Degradation Runbook
```python
#!/usr/bin/env python3
# runbooks/performance_investigation.py

import requests
import json
import time
from datetime import datetime, timedelta

class PerformanceInvestigator:
    def __init__(self, prometheus_url, grafana_url):
        self.prometheus_url = prometheus_url
        self.grafana_url = grafana_url
        
    def investigate_slow_api(self, threshold_seconds=2.0):
        """Investigate slow API responses"""
        print(f"🔍 Investigating API performance (threshold: {threshold_seconds}s)")
        
        # Query Prometheus for slow endpoints
        query = f'histogram_quantile(0.95, rate(archivechain_api_request_duration_seconds_bucket[5m])) > {threshold_seconds}'
        result = self.query_prometheus(query)
        
        if result['data']['result']:
            print("❌ Slow endpoints detected:")
            for item in result['data']['result']:
                endpoint = item['metric'].get('endpoint', 'unknown')
                latency = float(item['value'][1])
                print(f"  - {endpoint}: {latency:.2f}s")
                
            # Get detailed breakdown
            self.analyze_endpoint_performance()
        else:
            print("✅ All endpoints performing within threshold")
            
    def analyze_endpoint_performance(self):
        """Analyze performance by endpoint"""
        endpoints = self.get_slow_endpoints()
        
        for endpoint in endpoints:
            print(f"\n📊 Analyzing {endpoint}...")
            
            # Check database performance
            db_query = f'rate(archivechain_db_query_duration_seconds_sum{{endpoint="{endpoint}"}}[5m]) / rate(archivechain_db_query_duration_seconds_count{{endpoint="{endpoint}"}}[5m])'
            db_result = self.query_prometheus(db_query)
            
            if db_result['data']['result']:
                db_latency = float(db_result['data']['result'][0]['value'][1])
                print(f"  Database latency: {db_latency:.3f}s")
                
                if db_latency > 0.5:
                    print("  🚨 Database performance issue detected")
                    self.suggest_db_optimizations(endpoint)
            
            # Check external service calls
            ext_query = f'rate(archivechain_external_request_duration_seconds_sum{{endpoint="{endpoint}"}}[5m]) / rate(archivechain_external_request_duration_seconds_count{{endpoint="{endpoint}"}}[5m])'
            ext_result = self.query_prometheus(ext_query)
            
            if ext_result['data']['result']:
                ext_latency = float(ext_result['data']['result'][0]['value'][1])
                print(f"  External service latency: {ext_latency:.3f}s")
                
    def suggest_db_optimizations(self, endpoint):
        """Suggest database optimizations"""
        print("  💡 Suggested optimizations:")
        print("    - Check for missing indexes")
        print("    - Analyze query execution plans")
        print("    - Consider connection pool tuning")
        print("    - Review query complexity")
        
    def query_prometheus(self, query):
        """Query Prometheus API"""
        url = f"{self.prometheus_url}/api/v1/query"
        params = {'query': query}
        response = requests.get(url, params=params)
        return response.json()

if __name__ == "__main__":
    investigator = PerformanceInvestigator(
        prometheus_url="http://prometheus:9090",
        grafana_url="http://grafana:3000"
    )
    
    investigator.investigate_slow_api()
```

## Maintenance et Mises à Jour

### Stratégie de Maintenance

#### Fenêtres de Maintenance Planifiées
```yaml
Schedule:
  Regular Maintenance:
    - Every Sunday 02:00-04:00 UTC (Low traffic period)
    - Monthly extended window: First Sunday 02:00-06:00 UTC
    
  Emergency Maintenance:
    - As needed for critical security patches
    - Maximum 15 minutes advance notice for P0 issues

Procedures:
  Pre-maintenance (T-24h):
    - Announce in community channels
    - Update status page
    - Prepare rollback procedures
    - Verify backup integrity
    
  During maintenance:
    - Follow blue-green deployment strategy
    - Monitor metrics continuously
    - Test critical paths before switching traffic
    
  Post-maintenance (T+2h):
    - Verify all systems healthy
    - Update status page
    - Document any issues encountered
    - Schedule follow-up if needed
```

#### Blue-Green Deployment Process
```bash
#!/bin/bash
# maintenance/blue_green_deployment.sh

set -euo pipefail

BLUE_NAMESPACE="archivechain-prod"
GREEN_NAMESPACE="archivechain-green"
NEW_VERSION="$1"

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [DEPLOY] $1"
}

# Phase 1: Deploy to green environment
log "Phase 1: Deploying version $NEW_VERSION to green environment"

kubectl create namespace "$GREEN_NAMESPACE" --dry-run=client -o yaml | kubectl apply -f -

# Copy secrets and configs
kubectl get secret archivechain-keys -n "$BLUE_NAMESPACE" -o yaml | \
    sed "s/namespace: $BLUE_NAMESPACE/namespace: $GREEN_NAMESPACE/" | \
    kubectl apply -f -

kubectl get configmap archivechain-config -n "$BLUE_NAMESPACE" -o yaml | \
    sed "s/namespace: $BLUE_NAMESPACE/namespace: $GREEN_NAMESPACE/" | \
    kubectl apply -f -

# Update image version in green deployment
kubectl patch statefulset archivechain-full-archive -n "$GREEN_NAMESPACE" \
    -p '{"spec":{"template":{"spec":{"containers":[{"name":"archivechain-node","image":"archivechain/node:'$NEW_VERSION'"}]}}}}'

# Wait for green deployment to be ready
log "Waiting for green deployment to be ready..."
kubectl wait --for=condition=Ready pod -l "app=archivechain-full-archive" -n "$GREEN_NAMESPACE" --timeout=600s

# Phase 2: Health checks on green environment
log "Phase 2: Running health checks on green environment"

GREEN_POD=$(kubectl get pods -n "$GREEN_NAMESPACE" -l "app=archivechain-full-archive" -o jsonpath='{.items[0].metadata.name}')

# Test health endpoint
if ! kubectl exec "$GREEN_POD" -n "$GREEN_NAMESPACE" -- curl -f http://localhost:9999/health; then
    log "Health check failed on green environment"
    exit 1
fi

# Test API functionality
if ! kubectl exec "$GREEN_POD" -n "$GREEN_NAMESPACE" -- curl -f http://localhost:8080/v1/health; then
    log "API health check failed on green environment"
    exit 1
fi

# Test consensus participation
sleep 60  # Wait for consensus to stabilize
CONSENSUS_RATE=$(kubectl exec "$GREEN_POD" -n "$GREEN_NAMESPACE" -- \
    curl -s http://localhost:9999/metrics | grep consensus_participation_rate | cut -d' ' -f2)

if (( $(echo "$CONSENSUS_RATE < 0.8" | bc -l) )); then
    log "Consensus participation rate too low: $CONSENSUS_RATE"
    exit 1
fi

# Phase 3: Switch traffic
log "Phase 3: Switching traffic to green environment"

# Update service selector to point to green pods
kubectl patch service archivechain-gateway-service -n "$BLUE_NAMESPACE" \
    -p '{"spec":{"selector":{"version":"'$NEW_VERSION'"}}}'

# Wait for traffic to stabilize
sleep 120

# Monitor error rates
ERROR_RATE=$(curl -s "http://prometheus:9090/api/v1/query?query=rate(archivechain_api_errors_total[5m])/rate(archivechain_api_requests_total[5m])" | \
    jq -r '.data.result[0].value[1]')

if (( $(echo "$ERROR_RATE > 0.01" | bc -l) )); then
    log "High error rate detected: $ERROR_RATE, rolling back..."
    # Rollback
    kubectl patch service archivechain-gateway-service -n "$BLUE_NAMESPACE" \
        -p '{"spec":{"selector":{"version":"'$PREVIOUS_VERSION'"}}}'
    exit 1
fi

# Phase 4: Cleanup old version
log "Phase 4: Cleaning up blue environment"
sleep 300  # Wait 5 minutes to ensure stability

# Scale down blue deployment
kubectl scale statefulset archivechain-full-archive -n "$BLUE_NAMESPACE" --replicas=0

# Update blue namespace to new version for next deployment
kubectl patch statefulset archivechain-full-archive -n "$BLUE_NAMESPACE" \
    -p '{"spec":{"template":{"spec":{"containers":[{"name":"archivechain-node","image":"archivechain/node:'$NEW_VERSION'"}]}}}}'

log "Deployment completed successfully"
```

### Stratégie de Sauvegarde

#### Backup Complet Automatisé
```bash
#!/bin/bash
# maintenance/backup_system.sh

set -euo pipefail

BACKUP_TYPE="${1:-incremental}"  # full, incremental, snapshot
RETENTION_DAYS=30
S3_BUCKET="s3://archivechain-backups-prod"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [BACKUP] $1"
}

create_blockchain_backup() {
    local backup_name="blockchain_${BACKUP_TYPE}_${TIMESTAMP}"
    log "Creating blockchain backup: $backup_name"
    
    # Get all full archive nodes
    NODES=$(kubectl get pods -n archivechain-prod -l "app=archivechain-full-archive" -o jsonpath='{.items[*].metadata.name}')
    
    for node in $NODES; do
        log "Backing up node: $node"
        
        # Create consistent snapshot
        kubectl exec "$node" -n archivechain-prod -- \
            archivechain-cli db snapshot --output "/data/snapshots/snapshot_${TIMESTAMP}.db"
        
        # Copy snapshot to backup storage
        kubectl cp "archivechain-prod/${node}:/data/snapshots/snapshot_${TIMESTAMP}.db" \
            "/tmp/snapshot_${node}_${TIMESTAMP}.db"
        
        # Compress and upload to S3
        gzip "/tmp/snapshot_${node}_${TIMESTAMP}.db"
        aws s3 cp "/tmp/snapshot_${node}_${TIMESTAMP}.db.gz" \
            "${S3_BUCKET}/blockchain/${backup_name}/${node}.db.gz"
        
        # Cleanup local files
        rm "/tmp/snapshot_${node}_${TIMESTAMP}.db.gz"
        
        log "Backup completed for node: $node"
    done
}

create_config_backup() {
    local backup_name="config_${TIMESTAMP}"
    log "Creating configuration backup: $backup_name"
    
    # Backup secrets
    kubectl get secrets -n archivechain-prod -o yaml > "/tmp/secrets_${TIMESTAMP}.yaml"
    
    # Backup configmaps
    kubectl get configmaps -n archivechain-prod -o yaml > "/tmp/configs_${TIMESTAMP}.yaml"
    
    # Backup deployments
    kubectl get statefulsets,deployments -n archivechain-prod -o yaml > "/tmp/deployments_${TIMESTAMP}.yaml"
    
    # Create archive
    tar -czf "/tmp/config_backup_${TIMESTAMP}.tar.gz" \
        "/tmp/secrets_${TIMESTAMP}.yaml" \
        "/tmp/configs_${TIMESTAMP}.yaml" \
        "/tmp/deployments_${TIMESTAMP}.yaml"
    
    # Upload to S3
    aws s3 cp "/tmp/config_backup_${TIMESTAMP}.tar.gz" \
        "${S3_BUCKET}/configs/"
    
    # Cleanup
    rm "/tmp/secrets_${TIMESTAMP}.yaml" \
       "/tmp/configs_${TIMESTAMP}.yaml" \
       "/tmp/deployments_${TIMESTAMP}.yaml" \
       "/tmp/config_backup_${TIMESTAMP}.tar.gz"
}

create_monitoring_backup() {
    local backup_name="monitoring_${TIMESTAMP}"
    log "Creating monitoring data backup: $backup_name"
    
    # Backup Prometheus data
    kubectl exec prometheus-0 -n monitoring -- \
        tar -czf "/prometheus/backup_${TIMESTAMP}.tar.gz" /prometheus/data
    
    kubectl cp "monitoring/prometheus-0:/prometheus/backup_${TIMESTAMP}.tar.gz" \
        "/tmp/prometheus_backup_${TIMESTAMP}.tar.gz"
    
    aws s3 cp "/tmp/prometheus_backup_${TIMESTAMP}.tar.gz" \
        "${S3_BUCKET}/monitoring/"
    
    # Cleanup
    rm "/tmp/prometheus_backup_${TIMESTAMP}.tar.gz"
    kubectl exec prometheus-0 -n monitoring -- \
        rm "/prometheus/backup_${TIMESTAMP}.tar.gz"
}

cleanup_old_backups() {
    log "Cleaning up backups older than $RETENTION_DAYS days"
    
    # Calculate cutoff date
    CUTOFF_DATE=$(date -d "-${RETENTION_DAYS} days" +%Y%m%d)
    
    # List and delete old backups
    aws s3 ls "${S3_BUCKET}/" --recursive | \
        awk '{print $4}' | \
        grep -E "_[0-9]{8}_" | \
        while read backup; do
            BACKUP_DATE=$(echo "$backup" | sed -n 's/.*_\([0-9]\{8\}\)_.*/\1/p')
            if [[ "$BACKUP_DATE" < "$CUTOFF_DATE" ]]; then
                log "Deleting old backup: $backup"
                aws s3 rm "${S3_BUCKET}/${backup}"
            fi
        done
}

verify_backup_integrity() {
    log "Verifying backup integrity"
    
    # Test restore on a subset of data
    aws s3 cp "${S3_BUCKET}/blockchain/blockchain_${BACKUP_TYPE}_${TIMESTAMP}/archivechain-full-archive-0.db.gz" \
        "/tmp/test_restore.db.gz"
    
    gunzip "/tmp/test_restore.db.gz"
    
    # Verify database integrity
    if archivechain-cli db verify --db-path "/tmp/test_restore.db"; then
        log "Backup integrity verification passed"
        rm "/tmp/test_restore.db"
    else
        log "ERROR: Backup integrity verification failed"
        exit 1
    fi
}

# Main execution
case "$BACKUP_TYPE" in
    "full")
        create_blockchain_backup
        create_config_backup
        create_monitoring_backup
        verify_backup_integrity
        cleanup_old_backups
        ;;
    "incremental")
        create_blockchain_backup
        verify_backup_integrity
        ;;
    "snapshot")
        create_blockchain_backup
        ;;
    *)
        log "Unknown backup type: $BACKUP_TYPE"
        exit 1
        ;;
esac

log "Backup process completed successfully"
```

## Sécurité Opérationnelle

### Gestion des Accès

#### RBAC Kubernetes
```yaml
# security/rbac.yaml
---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: archivechain-operator
rules:
- apiGroups: [""]
  resources: ["pods", "services", "configmaps", "secrets"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["apps"]
  resources: ["deployments", "statefulsets", "replicasets"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]
- apiGroups: ["monitoring.coreos.com"]
  resources: ["servicemonitors", "prometheusrules"]
  verbs: ["get", "list", "watch", "create", "update", "patch", "delete"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: ClusterRole
metadata:
  name: archivechain-viewer
rules:
- apiGroups: [""]
  resources: ["pods", "services", "configmaps"]
  verbs: ["get", "list", "watch"]
- apiGroups: ["apps"]
  resources: ["deployments", "statefulsets", "replicasets"]
  verbs: ["get", "list", "watch"]

---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: archivechain-operators
  namespace: archivechain-prod
subjects:
- kind: User
  name: alice@archivechain.org
  apiGroup: rbac.authorization.k8s.io
- kind: User
  name: bob@archivechain.org
  apiGroup: rbac.authorization.k8s.io
roleRef:
  kind: ClusterRole
  name: archivechain-operator
  apiGroup: rbac.authorization.k8s.io
```

#### Rotation des Clés Automatisée
```bash
#!/bin/bash
# security/key_rotation.sh

set -euo pipefail

KEY_TYPE="$1"  # node, validator, api, tls
NAMESPACE="archivechain-prod"
ROTATION_ID=$(date +%Y%m%d_%H%M%S)

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S') [KEY_ROTATION] $1"
}

rotate_node_keys() {
    log "Starting node key rotation for all nodes"
    
    NODES=$(kubectl get pods -n "$NAMESPACE" -l "app=archivechain-full-archive" -o jsonpath='{.items[*].metadata.name}')
    
    for node in $NODES; do
        log "Rotating keys for node: $node"
        
        # Generate new key pair
        kubectl exec "$node" -n "$NAMESPACE" -- \
            archivechain-keygen generate --type node --output "/tmp/new_identity_${ROTATION_ID}.key"
        
        # Backup old key
        kubectl exec "$node" -n "$NAMESPACE" -- \
            cp "/keys/node_identity.key" "/keys/node_identity_backup_${ROTATION_ID}.key"
        
        # Update configuration with new key
        kubectl exec "$node" -n "$NAMESPACE" -- \
            cp "/tmp/new_identity_${ROTATION_ID}.key" "/keys/node_identity.key"
        
        # Graceful restart to pick up new key
        kubectl exec "$node" -n "$NAMESPACE" -- \
            pkill -SIGUSR2 archivechain-node
        
        # Wait for node to restart with new key
        sleep 30
        
        # Verify node is healthy with new key
        if kubectl exec "$node" -n "$NAMESPACE" -- curl -f http://localhost:9999/health; then
            log "Key rotation successful for node: $node"
            
            # Clean up temp files
            kubectl exec "$node" -n "$NAMESPACE" -- \
                rm "/tmp/new_identity_${ROTATION_ID}.key"
        else
            log "ERROR: Key rotation failed for node: $node, rolling back"
            
            # Rollback to old key
            kubectl exec "$node" -n "$NAMESPACE" -- \
                cp "/keys/node_identity_backup_${ROTATION_ID}.key" "/keys/node_identity.key"
            
            kubectl exec "$node" -n "$NAMESPACE" -- \
                pkill -SIGUSR2 archivechain-node
            
            exit 1
        fi
    done
}

rotate_api_keys() {
    log "Starting API key rotation"
    
    # Generate new JWT secret
    NEW_SECRET=$(openssl rand -hex 32)
    
    # Update secret in Kubernetes
    kubectl patch secret archivechain-gateway-keys -n "$NAMESPACE" \
        -p '{"data":{"jwt_secret":"'$(echo -n "$NEW_SECRET" | base64)'"}}'
    
    # Rolling restart of gateway pods
    kubectl rollout restart deployment archivechain-gateway -n "$NAMESPACE"
    kubectl rollout status deployment archivechain-gateway -n "$NAMESPACE"
    
    log "API key rotation completed"
}

rotate_tls_certificates() {
    log "Starting TLS certificate rotation"
    
    # Generate new certificate
    openssl req -x509 -newkey rsa:4096 -keyout "/tmp/tls_${ROTATION_ID}.key" \
        -out "/tmp/tls_${ROTATION_ID}.crt" -days 365 -nodes \
        -subj "/C=US/ST=CA/L=SF/O=ArchiveChain/CN=*.archivechain.org"
    
    # Update Kubernetes secret
    kubectl create secret tls archivechain-tls-new \
        --cert="/tmp/tls_${ROTATION_ID}.crt" \
        --key="/tmp/tls_${ROTATION_ID}.key" \
        -n "$NAMESPACE"
    
    # Update ingress to use new certificate
    kubectl patch ingress archivechain-ingress -n "$NAMESPACE" \
        -p '{"spec":{"tls":[{"secretName":"archivechain-tls-new","hosts":["api.archivechain.org"]}]}}'
    
    # Verify new certificate is working
    sleep 30
    if curl -s https://api.archivechain.org/v1/health | grep -q "healthy"; then
        log "TLS certificate rotation successful"
        
        # Remove old secret
        kubectl delete secret archivechain-tls -n "$NAMESPACE" || true
        kubectl patch secret archivechain-tls-new -n "$NAMESPACE" \
            --type='merge' -p='{"metadata":{"name":"archivechain-tls"}}'
    else
        log "ERROR: TLS certificate rotation failed"
        exit 1
    fi
    
    # Cleanup
    rm "/tmp/tls_${ROTATION_ID}.key" "/tmp/tls_${ROTATION_ID}.crt"
}

# Main execution
case "$KEY_TYPE" in
    "node")
        rotate_node_keys
        ;;
    "api")
        rotate_api_keys
        ;;
    "tls")
        rotate_tls_certificates
        ;;
    "all")
        rotate_node_keys
        rotate_api_keys
        rotate_tls_certificates
        ;;
    *)
        log "Unknown key type: $KEY_TYPE"
        log "Valid types: node, api, tls, all"
        exit 1
        ;;
esac

log "Key rotation completed successfully"
```

## Runbooks

### Node Recovery
```markdown
# Runbook: Node Recovery

## Symptom
- Node marked as down in monitoring
- Node not responding to health checks
- Node disconnected from network

## Investigation Steps

### 1. Check Pod Status
```bash
kubectl get pods -n archivechain-prod -l app=archivechain-full-archive
kubectl describe pod <node-name> -n archivechain-prod
```

### 2. Check Logs
```bash
kubectl logs <node-name> -n archivechain-prod --tail=100
kubectl logs <node-name> -n archivechain-prod --previous
```

### 3. Check Resource Usage
```bash
kubectl top pod <node-name> -n archivechain-prod
kubectl exec <node-name> -n archivechain-prod -- df -h
```

## Resolution Steps

### If Pod is CrashLooping
1. Check for configuration issues in logs
2. Verify persistent volume is healthy
3. Check for corrupted data files
4. Restore from backup if necessary

### If Pod is OutOfMemory
1. Check memory requests/limits
2. Analyze memory usage patterns
3. Consider increasing resources
4. Check for memory leaks

### If Storage Issues
1. Check PVC status: `kubectl get pvc -n archivechain-prod`
2. Verify storage class availability
3. Check disk space on nodes
4. Consider storage cleanup or expansion

## Recovery Commands
```bash
# Graceful restart
kubectl delete pod <node-name> -n archivechain-prod

# Emergency restart with data preservation
kubectl patch statefulset archivechain-full-archive -n archivechain-prod \
  -p '{"spec":{"template":{"metadata":{"annotations":{"kubectl.kubernetes.io/restartedAt":"'$(date +%Y-%m-%dT%H:%M:%S%z)'"}}}}}'

# Scale down and up (last resort)
kubectl scale statefulset archivechain-full-archive -n archivechain-prod --replicas=0
kubectl scale statefulset archivechain-full-archive -n archivechain-prod --replicas=3
```
```

### Performance Degradation
```markdown
# Runbook: Performance Degradation

## Symptom
- High API latency (>2s)
- Slow block processing
- Database performance issues

## Investigation

### 1. Check System Metrics
```bash
# CPU usage
kubectl top pods -n archivechain-prod

# Memory usage
kubectl exec <pod> -n archivechain-prod -- free -h

# Disk I/O
kubectl exec <pod> -n archivechain-prod -- iostat 1 5
```

### 2. Check Application Metrics
```bash
# Query Prometheus for slow endpoints
curl "http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,%20rate(archivechain_api_request_duration_seconds_bucket[5m]))"

# Check database performance
curl "http://prometheus:9090/api/v1/query?query=rate(archivechain_db_query_duration_seconds_sum[5m])"
```

### 3. Analyze Logs
```bash
# Look for slow query logs
kubectl logs <pod> -n archivechain-prod | grep "SLOW"

# Check for errors
kubectl logs <pod> -n archivechain-prod | grep "ERROR"
```

## Resolution

### Database Optimization
```sql
-- Check for missing indexes
EXPLAIN ANALYZE SELECT * FROM archives WHERE url = 'example.com';

-- Analyze table statistics
ANALYZE TABLE archives;

-- Check connection pool
SHOW PROCESSLIST;
```

### Application Tuning
```bash
# Increase connection pool size
kubectl patch configmap archivechain-config -n archivechain-prod \
  --patch '{"data":{"database.pool_size":"50"}}'

# Increase cache size
kubectl patch configmap archivechain-config -n archivechain-prod \
  --patch '{"data":{"cache.size":"32GB"}}'
```

### Infrastructure Scaling
```bash
# Scale horizontally
kubectl scale deployment archivechain-gateway -n archivechain-prod --replicas=5

# Scale vertically (requires restart)
kubectl patch statefulset archivechain-full-archive -n archivechain-prod \
  -p '{"spec":{"template":{"spec":{"containers":[{"name":"archivechain-node","resources":{"requests":{"memory":"64Gi","cpu":"16"}}}]}}}}'
```
```

### Security Incident Response
```markdown
# Runbook: Security Incident Response

## Immediate Actions (First 15 minutes)

### 1. Contain the Threat
```bash
# If specific node compromised, isolate it
kubectl label node <node-name> compromised=true
kubectl cordon <node-name>

# Block suspicious traffic
kubectl apply -f - <<EOF
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: emergency-block
  namespace: archivechain-prod
spec:
  podSelector:
    matchLabels:
      app: archivechain-full-archive
  policyTypes:
  - Ingress
  ingress:
  - from:
    - namespaceSelector:
        matchLabels:
          name: archivechain-prod
EOF
```

### 2. Preserve Evidence
```bash
# Capture memory dump
kubectl exec <pod> -n archivechain-prod -- \
  gcore $(pgrep archivechain-node) /tmp/memdump.core

# Copy logs before they rotate
kubectl logs <pod> -n archivechain-prod > /tmp/incident-logs-$(date +%s).log

# Snapshot affected volumes
kubectl exec <pod> -n archivechain-prod -- \
  lvcreate -L1G -s -n incident-snapshot /dev/vg0/data
```

### 3. Assess Impact
```bash
# Check for unauthorized access
kubectl get events -n archivechain-prod --sort-by='.lastTimestamp'

# Review API access logs
kubectl logs <gateway-pod> -n archivechain-prod | grep "401\|403\|400"

# Check for data exfiltration
kubectl exec <pod> -n archivechain-prod -- \
  netstat -an | grep ESTABLISHED
```

## Investigation

### Forensic Analysis
```bash
# Check file integrity
kubectl exec <pod> -n archivechain-prod -- \
  find /data -type f -exec sha256sum {} \; > checksums.txt

# Analyze network connections
kubectl exec <pod> -n archivechain-prod -- \
  ss -tuln | grep :9090

# Check for rootkits
kubectl exec <pod> -n archivechain-prod -- \
  chkrootkit
```

### Recovery
```bash
# Rotate all credentials immediately
./security/key_rotation.sh all

# Update container images
kubectl set image statefulset/archivechain-full-archive \
  archivechain-node=archivechain/node:v1.0.1-security \
  -n archivechain-prod

# Restore from clean backup if necessary
./maintenance/restore_from_backup.sh <backup-timestamp>
```
```

---

*Dernière mise à jour: 24 juillet 2025*
*Version: 1.0.0*