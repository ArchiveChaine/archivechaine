# Guide d'Installation ArchiveChain

## Table des Matières

- [Prérequis Système](#prérequis-système)
- [Installation Rapide](#installation-rapide)
- [Installation Détaillée](#installation-détaillée)
- [Configuration des Nœuds](#configuration-des-nœuds)
- [Démarrage du Réseau](#démarrage-du-réseau)
- [Vérification de l'Installation](#vérification-de-linstallation)
- [Résolution des Problèmes](#résolution-des-problèmes)
- [Mise à Jour](#mise-à-jour)

## Prérequis Système

### Configuration Minimale

| Composant | Spécification Minimale | Recommandée |
|-----------|------------------------|-------------|
| **OS** | Linux Ubuntu 20.04+, macOS 12+, Windows 10+ | Ubuntu 22.04 LTS |
| **CPU** | 4 cores @ 2.0 GHz | 8 cores @ 3.0 GHz |
| **RAM** | 8 GB | 32 GB |
| **Stockage** | 100 GB SSD | 1 TB NVMe SSD |
| **Réseau** | 100 Mbps | 1 Gbps |
| **Ports** | 8080, 9090, 9091 | Configurables |

### Configuration par Type de Nœud

#### Full Archive Node
```yaml
cpu_cores: 16+
memory: 64 GB+
storage: 10 TB+
bandwidth: 1 Gbps+
uptime: 99.9%+
```

#### Light Storage Node
```yaml
cpu_cores: 8+
memory: 16 GB+
storage: 1-10 TB
bandwidth: 500 Mbps+
uptime: 99%+
```

#### Relay Node
```yaml
cpu_cores: 8+
memory: 16 GB+
storage: 100 GB+
bandwidth: 2 Gbps+
connections: 1000+
```

#### Gateway Node
```yaml
cpu_cores: 16+
memory: 32 GB+
storage: 500 GB+
bandwidth: 1 Gbps+
load_balancer: recommandé
```

### Logiciels Requis

#### Rust (Obligatoire)
```bash
# Installation via rustup
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env

# Vérification
rustc --version  # rustc 1.70.0+
cargo --version  # cargo 1.70.0+
```

#### Git
```bash
# Ubuntu/Debian
sudo apt update && sudo apt install git

# macOS
brew install git

# Windows
# Télécharger depuis https://git-scm.com/download/win
```

#### Docker (Optionnel, recommandé)
```bash
# Ubuntu/Debian
sudo apt install docker.io docker-compose
sudo usermod -aG docker $USER

# macOS
brew install docker docker-compose

# Windows
# Télécharger Docker Desktop
```

## Installation Rapide

### Script d'Installation Automatique

```bash
# Télécharger et exécuter le script d'installation
curl -sSL https://install.archivechain.org | bash

# Ou télécharger manuellement
wget https://install.archivechain.org/install.sh
chmod +x install.sh
./install.sh
```

### Installation via Cargo

```bash
# Installation globale via Cargo
cargo install archivechain-node archivechain-cli

# Vérification
archivechain-node --version
archivechain-cli --version
```

### Installation via Binaires Pré-compilés

```bash
# Télécharger la dernière release
wget https://github.com/archivechain/archivechain/releases/latest/download/archivechain-linux-x64.tar.gz

# Extraire
tar -xzf archivechain-linux-x64.tar.gz

# Installer
sudo mv archivechain-* /usr/local/bin/
sudo chmod +x /usr/local/bin/archivechain-*
```

## Installation Détaillée

### 1. Cloner le Repository

```bash
# Cloner le projet principal
git clone https://github.com/archivechain/archivechain.git
cd archivechain

# Vérifier la branche
git checkout main
git pull origin main
```

### 2. Configuration de l'Environnement

```bash
# Variables d'environnement
export ARCHIVECHAIN_HOME="$HOME/.archivechain"
export ARCHIVECHAIN_LOG_LEVEL="info"
export RUST_LOG="archivechain=info"

# Ajouter au .bashrc/.zshrc
echo 'export ARCHIVECHAIN_HOME="$HOME/.archivechain"' >> ~/.bashrc
source ~/.bashrc
```

### 3. Compilation depuis les Sources

```bash
# Build en mode release (recommandé pour production)
cargo build --release

# Build en mode debug (pour développement)
cargo build

# Build avec optimisations spécifiques
RUSTFLAGS="-C target-cpu=native" cargo build --release

# Vérification des binaires
ls -la target/release/
# archivechain-node*
# archivechain-cli*
# archivechain-keygen*
```

### 4. Installation des Binaires

```bash
# Installation système
sudo cp target/release/archivechain-* /usr/local/bin/

# Installation utilisateur
mkdir -p ~/.local/bin
cp target/release/archivechain-* ~/.local/bin/
export PATH="$HOME/.local/bin:$PATH"

# Vérification
which archivechain-node
archivechain-node --version
```

## Configuration des Nœuds

### 1. Initialisation de Base

```bash
# Créer le répertoire de configuration
mkdir -p $ARCHIVECHAIN_HOME/{config,data,logs,keys}

# Initialiser la configuration
archivechain-cli config init --network testnet
```

### 2. Génération des Clés

```bash
# Générer une nouvelle identité de nœud
archivechain-keygen generate \
  --type node \
  --output $ARCHIVECHAIN_HOME/keys/node_identity.key

# Générer une clé de portefeuille
archivechain-keygen generate \
  --type wallet \
  --output $ARCHIVECHAIN_HOME/keys/wallet.key

# Afficher la clé publique
archivechain-keygen public-key \
  --private-key $ARCHIVECHAIN_HOME/keys/node_identity.key
```

### 3. Configuration par Type de Nœud

#### Full Archive Node

```bash
# Copier la configuration de base
cp config/node/full_archive.toml $ARCHIVECHAIN_HOME/config/node.toml

# Éditer la configuration
cat > $ARCHIVECHAIN_HOME/config/node.toml << EOF
[node]
type = "full_archive"
identity_key = "$ARCHIVECHAIN_HOME/keys/node_identity.key"
data_dir = "$ARCHIVECHAIN_HOME/data"
log_level = "info"

[storage]
capacity = "20TB"
replication_factor = 10
compression = true
deduplication = true

[network]
listen_addr = "0.0.0.0:9090"
bootstrap_peers = [
  "/ip4/18.191.45.2/tcp/9090/p2p/12D3KooWCRscMgHgEo3ojm8ovzheydpvTEqsDtq7Wby38cMHrYjt",
  "/ip4/52.35.32.10/tcp/9090/p2p/12D3KooWKnxJKRy2T2nwJP2NzJjkgFsZEJ2wz8WFmCJF5ZY8kHVk"
]

[consensus]
participate = true
weight = 1.0
validator_key = "$ARCHIVECHAIN_HOME/keys/validator.key"

[api]
enabled = false
EOF
```

#### Light Storage Node

```bash
cp config/node/light_storage.toml $ARCHIVECHAIN_HOME/config/node.toml

# Configuration spécialisée
cat >> $ARCHIVECHAIN_HOME/config/node.toml << EOF
[specialization]
type = "domain"
patterns = ["*.gov", "*.edu", "*.org"]
geographic_region = "europe-west"
content_types = ["text/html", "application/pdf"]
EOF
```

#### Relay Node

```bash
cp config/node/relay.toml $ARCHIVECHAIN_HOME/config/node.toml

# Configuration réseau optimisée
cat >> $ARCHIVECHAIN_HOME/config/node.toml << EOF
[relay]
max_connections = 1500
bandwidth_limit = "2GB/s"
routing_table_size = 10000
discovery_interval = "30s"
EOF
```

#### Gateway Node

```bash
cp config/node/gateway.toml $ARCHIVECHAIN_HOME/config/node.toml

# Configuration API complète
cat >> $ARCHIVECHAIN_HOME/config/node.toml << EOF
[api]
enabled = true
rest_port = 8080
graphql_port = 8081
websocket_port = 8082
grpc_port = 9091

[security]
rate_limit = 2000
cors_origins = ["*"]
jwt_secret_file = "$ARCHIVECHAIN_HOME/keys/jwt_secret"
EOF
```

### 4. Configuration Réseau

```bash
# Configuration testnet
cat > $ARCHIVECHAIN_HOME/config/network.toml << EOF
[network]
name = "testnet"
chain_id = "archivechain-testnet-1"

[genesis]
timestamp = "2024-01-01T00:00:00Z"
initial_supply = "100000000000000000" # 100B ARC
allocations = [
  { address = "arc1qjv4v5z...", amount = "15000000000000000" }, # 15B - Public sale
  { address = "arc1qkg8h3k...", amount = "25000000000000000" }, # 25B - Team (vested)
]

[consensus]
block_time = "6s"
validators_required = 21
finalization_blocks = 32

[economic]
base_archive_reward = "100000000" # 100 ARC
inflation_rate = 0.05
deflation_rate = 0.10
EOF
```

## Démarrage du Réseau

### 1. Premier Démarrage

```bash
# Démarrer le nœud en mode foreground
archivechain-node start \
  --config $ARCHIVECHAIN_HOME/config/node.toml \
  --network-config $ARCHIVECHAIN_HOME/config/network.toml

# Démarrer en arrière-plan
archivechain-node start \
  --config $ARCHIVECHAIN_HOME/config/node.toml \
  --daemon \
  --log-file $ARCHIVECHAIN_HOME/logs/node.log
```

### 2. Service Systemd (Linux)

```bash
# Créer le service systemd
sudo tee /etc/systemd/system/archivechain.service << EOF
[Unit]
Description=ArchiveChain Node
After=network.target

[Service]
Type=simple
User=$USER
ExecStart=/usr/local/bin/archivechain-node start --config $ARCHIVECHAIN_HOME/config/node.toml
Restart=always
RestartSec=5
Environment=ARCHIVECHAIN_HOME=$ARCHIVECHAIN_HOME
Environment=RUST_LOG=archivechain=info

[Install]
WantedBy=multi-user.target
EOF

# Activer et démarrer le service
sudo systemctl daemon-reload
sudo systemctl enable archivechain
sudo systemctl start archivechain

# Vérifier le statut
sudo systemctl status archivechain
```

### 3. Configuration Docker

```yaml
# docker-compose.yml
version: '3.8'

services:
  archivechain-node:
    image: archivechain/node:latest
    container_name: archivechain-node
    restart: unless-stopped
    ports:
      - "8080:8080"   # REST API
      - "9090:9090"   # P2P
      - "9091:9091"   # gRPC
    volumes:
      - ./data:/data
      - ./config:/config
      - ./logs:/logs
    environment:
      - ARCHIVECHAIN_HOME=/data
      - RUST_LOG=archivechain=info
    command: >
      start 
      --config /config/node.toml
      --network-config /config/network.toml
```

```bash
# Démarrer avec Docker Compose
docker-compose up -d

# Voir les logs
docker-compose logs -f archivechain-node
```

## Vérification de l'Installation

### 1. Statut du Nœud

```bash
# Vérifier que le nœud fonctionne
archivechain-cli node status

# Sortie attendue:
# Node ID: 12D3KooWCRscMgHgEo3ojm8ovzheydpvTEqsDtq7Wby38cMHrYjt
# Status: Running
# Block Height: 12547
# Peers: 15
# Uptime: 2h 34m 12s
```

### 2. Connectivité Réseau

```bash
# Vérifier les pairs connectés
archivechain-cli network peers

# Tester la connectivité
archivechain-cli network ping --peer 12D3KooWKnxJKRy2T2nwJP2NzJjkgFsZEJ2wz8WFmCJF5ZY8kHVk
```

### 3. API Endpoints (Gateway uniquement)

```bash
# Tester l'API REST
curl http://localhost:8080/health

# Réponse attendue:
# {
#   "status": "healthy",
#   "version": "1.0.0",
#   "timestamp": "2024-01-15T10:30:00Z",
#   "checks": {
#     "blockchain": "healthy",
#     "storage": "healthy",
#     "network": "healthy"
#   }
# }

# Tester l'API GraphQL
curl -X POST http://localhost:8081/graphql \
  -H "Content-Type: application/json" \
  -d '{"query": "{ networkStats { totalNodes activeNodes } }"}'
```

### 4. Synchronisation Blockchain

```bash
# Vérifier la synchronisation
archivechain-cli blockchain sync-status

# Créer un premier test d'archive
archivechain-cli archive create \
  --url "https://example.com" \
  --title "Test Archive"
```

## Résolution des Problèmes

### Problèmes Courants

#### 1. Erreur de Compilation Rust

```bash
# Erreur: rustc version insuffisante
error: package `archivechain-core` requires Rust 1.70.0

# Solution: Mettre à jour Rust
rustup update stable
rustc --version
```

#### 2. Ports déjà utilisés

```bash
# Erreur: Address already in use
Error: Failed to bind to address 0.0.0.0:9090: address already in use

# Solution: Vérifier les ports utilisés
sudo netstat -tulpn | grep :9090
sudo lsof -i :9090

# Changer le port dans la configuration
sed -i 's/9090/9095/g' $ARCHIVECHAIN_HOME/config/node.toml
```

#### 3. Problèmes de Permissions

```bash
# Erreur: Permission denied
Error: Permission denied (os error 13)

# Solution: Corriger les permissions
sudo chown -R $USER:$USER $ARCHIVECHAIN_HOME
chmod 700 $ARCHIVECHAIN_HOME/keys
chmod 600 $ARCHIVECHAIN_HOME/keys/*
```

#### 4. Échec de Connexion aux Pairs

```bash
# Erreur: No peers available
Warning: Failed to connect to bootstrap peers

# Solution: Vérifier la connectivité
ping -c 3 18.191.45.2
telnet 18.191.45.2 9090

# Vérifier la configuration firewall
sudo ufw allow 9090
sudo iptables -A INPUT -p tcp --dport 9090 -j ACCEPT
```

#### 5. Synchronisation Lente

```bash
# Problème: Synchronisation blockchain lente
Block height: 1250/12547 (9.9%)

# Solutions:
# 1. Augmenter les connexions pairs
echo 'max_peers = 50' >> $ARCHIVECHAIN_HOME/config/node.toml

# 2. Utiliser des pairs plus rapides
archivechain-cli network discover --region nearest

# 3. Activer la synchronisation rapide
echo 'fast_sync = true' >> $ARCHIVECHAIN_HOME/config/node.toml
```

### Outils de Diagnostic

#### 1. Logs Détaillés

```bash
# Activer les logs debug
export RUST_LOG="archivechain=debug,libp2p=info"
archivechain-node start --config $ARCHIVECHAIN_HOME/config/node.toml

# Analyser les logs
tail -f $ARCHIVECHAIN_HOME/logs/node.log | grep ERROR
journalctl -u archivechain -f
```

#### 2. Monitoring de Performance

```bash
# Métriques système
htop
iotop
nethogs

# Métriques nœud
archivechain-cli node metrics
archivechain-cli storage usage
archivechain-cli network bandwidth
```

#### 3. Tests de Connectivité

```bash
# Test réseau P2P
archivechain-cli network test-connectivity

# Test stockage
archivechain-cli storage benchmark

# Test APIs (Gateway uniquement)
archivechain-cli api test-endpoints
```

## Mise à Jour

### 1. Mise à Jour depuis les Sources

```bash
# Sauvegarder la configuration
cp -r $ARCHIVECHAIN_HOME/config $ARCHIVECHAIN_HOME/config.backup

# Arrêter le nœud
sudo systemctl stop archivechain

# Mettre à jour le code
cd archivechain
git fetch origin
git checkout v1.1.0  # ou latest
cargo build --release

# Installer les nouveaux binaires
sudo cp target/release/archivechain-* /usr/local/bin/

# Redémarrer
sudo systemctl start archivechain
```

### 2. Mise à Jour via Package Manager

```bash
# Mettre à jour via cargo
cargo install --force archivechain-node archivechain-cli

# Redémarrer le service
sudo systemctl restart archivechain
```

### 3. Migration de Configuration

```bash
# Vérifier la compatibilité
archivechain-cli config validate --file $ARCHIVECHAIN_HOME/config/node.toml

# Migrer si nécessaire
archivechain-cli config migrate \
  --from-version 1.0 \
  --to-version 1.1 \
  --config $ARCHIVECHAIN_HOME/config/node.toml
```

### 4. Mise à Jour Docker

```bash
# Mettre à jour l'image
docker-compose pull

# Redémarrer avec la nouvelle image
docker-compose down
docker-compose up -d

# Vérifier la nouvelle version
docker-compose exec archivechain-node archivechain-node --version
```

## Support et Ressources

### Documentation
- 📚 [Documentation complète](https://docs.archivechain.org)
- 🔧 [Configuration avancée](https://docs.archivechain.org/config)
- 🚀 [Guide de performance](https://docs.archivechain.org/performance)

### Assistance
- 💬 [Discord #support](https://discord.gg/archivechain-support)
- 📧 [support@archivechain.org](mailto:support@archivechain.org)
- 🐛 [Issues GitHub](https://github.com/archivechain/archivechain/issues)

### Monitoring
- 📊 [Status page](https://status.archivechain.org)
- 🔍 [Network explorer](https://explorer.archivechain.org)
- 📈 [Métriques publiques](https://metrics.archivechain.org)

---

*Dernière mise à jour: 23 juillet 2025*