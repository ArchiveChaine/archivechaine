# Déploiement Docker Compose ArchiveChain

Ce dossier contient des configurations Docker Compose pour déployer rapidement un environnement ArchiveChain local ou de développement.

## Configurations Disponibles

```
docker_compose/
├── README.md                    # Ce fichier
├── basic/                       # Configuration de base
│   ├── docker-compose.yml      # Stack minimale
│   ├── .env.example            # Variables d'environnement
│   └── config/                 # Configurations des nœuds
├── full_stack/                 # Stack complète avec monitoring
│   ├── docker-compose.yml
│   ├── monitoring.yml          # Services de monitoring
│   ├── .env.example
│   └── config/
├── development/                # Environnement de développement
│   ├── docker-compose.yml
│   ├── docker-compose.dev.yml
│   └── scripts/
└── production/                 # Configuration production-ready
    ├── docker-compose.yml
    ├── docker-compose.prod.yml
    ├── nginx/
    └── ssl/
```

## Démarrage Rapide

### Configuration de Base

```bash
cd examples/infrastructure/docker_compose/basic
cp .env.example .env
# Éditez .env avec vos paramètres
docker-compose up -d
```

Cette configuration lance :
- 1 Full Archive Node
- 1 Gateway Node avec APIs
- 1 Base de données RocksDB
- Interface web de base

### Stack Complète

```bash
cd examples/infrastructure/docker_compose/full_stack
cp .env.example .env
docker-compose -f docker-compose.yml -f monitoring.yml up -d
```

Cette configuration ajoute :
- Prometheus pour les métriques
- Grafana pour la visualisation
- ELK Stack pour les logs
- Redis pour le cache
- Nginx comme reverse proxy

## Configurations Détaillées

### Configuration de Base

#### docker-compose.yml
```yaml
version: '3.8'

services:
  # Full Archive Node
  archivechain-node:
    image: archivechain/node:latest
    container_name: archivechain-node
    restart: unless-stopped
    
    ports:
      - "9090:9090"   # P2P
      - "9999:9999"   # Metrics
    
    volumes:
      - archivechain_data:/data
      - ./config/node.toml:/config/node.toml:ro
      - ./keys:/keys:ro
    
    environment:
      - RUST_LOG=archivechain=info
      - ARCHIVECHAIN_CONFIG=/config/node.toml
    
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9999/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    
    networks:
      - archivechain_network

  # Gateway Node avec APIs
  archivechain-gateway:
    image: archivechain/gateway:latest
    container_name: archivechain-gateway
    restart: unless-stopped
    
    ports:
      - "8080:8080"   # REST API
      - "8081:8081"   # GraphQL
      - "8082:8082"   # WebSocket
      - "9091:9091"   # gRPC
    
    volumes:
      - ./config/gateway.toml:/config/gateway.toml:ro
      - ./keys:/keys:ro
    
    environment:
      - RUST_LOG=archivechain_gateway=info
      - ARCHIVECHAIN_CONFIG=/config/gateway.toml
      - ARCHIVECHAIN_NODE_URL=http://archivechain-node:9090
    
    depends_on:
      archivechain-node:
        condition: service_healthy
    
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/v1/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    
    networks:
      - archivechain_network

  # Interface Web
  archivechain-ui:
    image: archivechain/ui:latest
    container_name: archivechain-ui
    restart: unless-stopped
    
    ports:
      - "3000:3000"
    
    environment:
      - REACT_APP_API_URL=http://localhost:8080/v1
      - REACT_APP_GRAPHQL_URL=http://localhost:8081/graphql
      - REACT_APP_WS_URL=ws://localhost:8082
    
    depends_on:
      archivechain-gateway:
        condition: service_healthy
    
    networks:
      - archivechain_network

volumes:
  archivechain_data:
    driver: local

networks:
  archivechain_network:
    driver: bridge
```

#### .env.example
```bash
# Configuration ArchiveChain
ARCHIVECHAIN_NETWORK=devnet
ARCHIVECHAIN_LOG_LEVEL=info
ARCHIVECHAIN_DATA_DIR=./data

# Configuration des nœuds
NODE_IDENTITY_KEY=./keys/node_identity.key
VALIDATOR_KEY=./keys/validator.key
JWT_SECRET=your-jwt-secret-change-in-production

# Configuration réseau
P2P_PORT=9090
API_PORT=8080
GRAPHQL_PORT=8081
WEBSOCKET_PORT=8082
GRPC_PORT=9091
UI_PORT=3000

# Bootstrap peers pour devnet
BOOTSTRAP_PEERS=/dns4/devnet-bootstrap-1.archivechain.org/tcp/9090/p2p/12D3KooW...,/dns4/devnet-bootstrap-2.archivechain.org/tcp/9090/p2p/12D3KooW...

# Configuration base de données
DB_PATH=./data/db
DB_CACHE_SIZE=1GB

# Configuration stockage
STORAGE_CAPACITY=100GB
REPLICATION_FACTOR=3
COMPRESSION_ENABLED=true

# Monitoring (pour stack complète)
PROMETHEUS_PORT=9090
GRAFANA_PORT=3001
ELASTICSEARCH_PORT=9200
KIBANA_PORT=5601
```

### Configuration de Développement

#### docker-compose.dev.yml
```yaml
version: '3.8'

services:
  archivechain-node:
    build:
      context: ../../..
      dockerfile: docker/Dockerfile.dev
    volumes:
      - ../../../core:/app/core:ro
      - archivechain_dev_data:/data
    environment:
      - RUST_LOG=archivechain=debug,libp2p=info
      - CARGO_INCREMENTAL=1
    command: cargo run --bin archivechain-node -- --config /config/dev.toml

  archivechain-gateway:
    build:
      context: ../../..
      dockerfile: docker/Dockerfile.gateway.dev
    volumes:
      - ../../../gateway:/app/gateway:ro
    environment:
      - RUST_LOG=archivechain_gateway=debug
    command: cargo run --bin archivechain-gateway -- --config /config/dev.toml

  # Hot reload pour le développement
  file-watcher:
    image: alpine:latest
    volumes:
      - ../../../:/app:ro
    command: |
      sh -c "
        apk add --no-cache inotify-tools curl
        while true; do
          inotifywait -r -e modify /app/core/src /app/gateway/src
          echo 'Code change detected, restarting services...'
          curl -X POST http://archivechain-node:9999/admin/reload || true
          curl -X POST http://archivechain-gateway:8080/admin/reload || true
        done
      "
    depends_on:
      - archivechain-node
      - archivechain-gateway

volumes:
  archivechain_dev_data:
    driver: local
```

### Configuration avec Monitoring

#### monitoring.yml
```yaml
version: '3.8'

services:
  # Prometheus pour les métriques
  prometheus:
    image: prom/prometheus:latest
    container_name: archivechain-prometheus
    restart: unless-stopped
    
    ports:
      - "9090:9090"
    
    volumes:
      - ./monitoring/prometheus.yml:/etc/prometheus/prometheus.yml:ro
      - ./monitoring/alerts.yml:/etc/prometheus/alerts.yml:ro
      - prometheus_data:/prometheus
    
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
      - '--web.console.libraries=/etc/prometheus/console_libraries'
      - '--web.console.templates=/etc/prometheus/consoles'
      - '--storage.tsdb.retention.time=30d'
      - '--web.enable-lifecycle'
    
    networks:
      - archivechain_network

  # Grafana pour la visualisation
  grafana:
    image: grafana/grafana:latest
    container_name: archivechain-grafana
    restart: unless-stopped
    
    ports:
      - "3001:3000"
    
    volumes:
      - grafana_data:/var/lib/grafana
      - ./monitoring/grafana/dashboards:/etc/grafana/provisioning/dashboards:ro
      - ./monitoring/grafana/datasources:/etc/grafana/provisioning/datasources:ro
    
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin123
      - GF_USERS_ALLOW_SIGN_UP=false
      - GF_INSTALL_PLUGINS=grafana-piechart-panel
    
    depends_on:
      - prometheus
    
    networks:
      - archivechain_network

  # Redis pour le cache
  redis:
    image: redis:alpine
    container_name: archivechain-redis
    restart: unless-stopped
    
    ports:
      - "6379:6379"
    
    volumes:
      - redis_data:/data
    
    command: redis-server --appendonly yes --maxmemory 512mb --maxmemory-policy allkeys-lru
    
    networks:
      - archivechain_network

  # Nginx reverse proxy
  nginx:
    image: nginx:alpine
    container_name: archivechain-nginx
    restart: unless-stopped
    
    ports:
      - "80:80"
      - "443:443"
    
    volumes:
      - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
      - ./nginx/ssl:/etc/nginx/ssl:ro
    
    depends_on:
      - archivechain-gateway
    
    networks:
      - archivechain_network

volumes:
  prometheus_data:
  grafana_data:
  redis_data:
```

## Scripts Utilitaires

### Démarrage et arrêt
```bash
#!/bin/bash
# scripts/start.sh

set -e

ENVIRONMENT=${1:-basic}

echo "🚀 Démarrage d'ArchiveChain ($ENVIRONMENT)..."

case $ENVIRONMENT in
  "basic")
    cd basic && docker-compose up -d
    ;;
  "full")
    cd full_stack && docker-compose -f docker-compose.yml -f monitoring.yml up -d
    ;;
  "dev")
    cd development && docker-compose -f docker-compose.yml -f docker-compose.dev.yml up -d
    ;;
  *)
    echo "Environnement non reconnu: $ENVIRONMENT"
    echo "Environnements disponibles: basic, full, dev"
    exit 1
    ;;
esac

echo "✅ ArchiveChain démarré!"
echo "🌐 Interface web: http://localhost:3000"
echo "📊 API REST: http://localhost:8080/v1"
echo "📈 Métriques: http://localhost:9999/metrics"

if [ "$ENVIRONMENT" = "full" ]; then
  echo "📊 Grafana: http://localhost:3001 (admin/admin123)"
  echo "🔍 Prometheus: http://localhost:9090"
fi
```

### Vérification de santé
```bash
#!/bin/bash
# scripts/health_check.sh

echo "🏥 Vérification de santé d'ArchiveChain..."

services=(
  "http://localhost:9999/health"
  "http://localhost:8080/v1/health"
  "http://localhost:3000"
)

for url in "${services[@]}"; do
  if curl -f -s "$url" > /dev/null; then
    echo "✅ $url"
  else
    echo "❌ $url"
  fi
done

# Vérifier les métriques
echo "📊 Métriques du nœud:"
curl -s http://localhost:9999/metrics | grep -E "(archivechain_|up )" | head -5
```

### Sauvegarde
```bash
#!/bin/bash
# scripts/backup.sh

BACKUP_DIR="./backups/$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP_DIR"

echo "💾 Sauvegarde d'ArchiveChain vers $BACKUP_DIR..."

# Sauvegarder les données des volumes
docker run --rm -v archivechain_data:/data -v "$(pwd)/$BACKUP_DIR":/backup alpine \
  tar czf /backup/archivechain_data.tar.gz -C /data .

# Sauvegarder les configurations
cp -r config "$BACKUP_DIR/"
cp -r keys "$BACKUP_DIR/"
cp .env "$BACKUP_DIR/"

echo "✅ Sauvegarde terminée: $BACKUP_DIR"
```

## Configuration des Nœuds

### node.toml
```toml
[node]
type = "full_archive"
identity_key = "/keys/node_identity.key"
data_dir = "/data"
log_level = "info"

[storage]
capacity = "100GB"
replication_factor = 3
compression = true

[consensus]
participate = true
validator_key = "/keys/validator.key"

[network]
listen_addr = "0.0.0.0:9090"
max_peers = 50
bootstrap_peers = [
  "/dns4/devnet-bootstrap-1.archivechain.org/tcp/9090/p2p/12D3KooWCRscMgHgEo3ojm8ovzheydpvTEqsDtq7Wby38cMHrYjt"
]

[performance]
max_concurrent_archives = 10
cache_size = "1GB"
```

### gateway.toml
```toml
[api]
enabled = true

[api.rest]
port = 8080
cors_enabled = true

[api.graphql]
port = 8081

[api.websocket]
port = 8082

[api.grpc]
port = 9091

[security]
jwt_secret_file = "/keys/jwt_secret"
rate_limit_enabled = true

[cache]
redis_url = "redis://redis:6379"
ttl_default = "1h"
```

## Dépannage

### Problèmes courants

**Port déjà utilisé**
```bash
# Vérifier les ports utilisés
netstat -tulpn | grep :8080

# Arrêter les services existants
docker-compose down
```

**Volumes corrompus**
```bash
# Supprimer et recréer les volumes
docker-compose down -v
docker volume prune
docker-compose up -d
```

**Problèmes de permissions**
```bash
# Corriger les permissions des clés
chmod 600 keys/*
chmod 700 keys/
```

### Logs de débogage
```bash
# Voir les logs en temps réel
docker-compose logs -f

# Logs d'un service spécifique
docker-compose logs -f archivechain-node

# Logs avec horodatage
docker-compose logs -t archivechain-gateway
```

## Personnalisation

### Ajouter un nouveau service
1. Modifiez `docker-compose.yml`
2. Ajoutez la configuration dans `config/`
3. Mettez à jour les variables d'environnement
4. Redémarrez la stack

### Changer les ports
1. Modifiez les ports dans `docker-compose.yml`
2. Mettez à jour `.env`
3. Redémarrez les services concernés

### Ajouter du monitoring personnalisé
1. Ajoutez votre dashboard Grafana dans `monitoring/grafana/dashboards/`
2. Configurez les alertes dans `monitoring/alerts.yml`
3. Redémarrez Prometheus et Grafana

## Production

⚠️ **Important** : Ces configurations sont conçues pour le développement. Pour la production :

1. Utilisez des secrets sécurisés
2. Configurez HTTPS avec de vrais certificats
3. Implémentez une sauvegarde automatique
4. Configurez des alertes appropriées
5. Utilisez des images taguées plutôt que `latest`

Voir [Configuration Production](../../../docs/OPERATIONS.md) pour plus de détails.