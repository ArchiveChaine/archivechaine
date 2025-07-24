# Structure du Projet ArchiveChain

## Arborescence Complète

```
archivechain/
├── Cargo.toml                      # Workspace Rust principal
├── README.md                       # Documentation principale
├── LICENSE                         # Licence du projet
├── ARCHITECTURE.md                 # Architecture technique
├── CHANGELOG.md                    # Journal des modifications
│
├── core/                           # Module blockchain core
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs                  # Interface publique du module
│   │   ├── block/
│   │   │   ├── mod.rs
│   │   │   ├── header.rs           # En-tête de bloc
│   │   │   ├── body.rs             # Corps du bloc
│   │   │   └── archive_metadata.rs # Métadonnées d'archives
│   │   ├── transaction/
│   │   │   ├── mod.rs
│   │   │   ├── pool.rs             # Pool de transactions
│   │   │   ├── validation.rs       # Validation des transactions
│   │   │   └── types.rs            # Types de transactions
│   │   ├── state/
│   │   │   ├── mod.rs
│   │   │   ├── machine.rs          # Machine d'état
│   │   │   ├── merkle.rs           # Arbre de Merkle
│   │   │   └── storage.rs          # Stockage d'état
│   │   ├── crypto/
│   │   │   ├── mod.rs
│   │   │   ├── hash.rs             # Fonctions de hachage
│   │   │   ├── signature.rs        # Signatures numériques
│   │   │   └── keys.rs             # Gestion des clés
│   │   └── error.rs                # Types d'erreurs
│   └── tests/
│       ├── integration/
│       └── unit/
│
├── consensus/                      # Module Proof of Archive
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── poa/
│   │   │   ├── mod.rs
│   │   │   ├── engine.rs           # Moteur de consensus PoA
│   │   │   └── config.rs           # Configuration consensus
│   │   ├── proofs/
│   │   │   ├── mod.rs
│   │   │   ├── storage.rs          # Proof of Storage
│   │   │   ├── bandwidth.rs        # Proof of Bandwidth
│   │   │   └── longevity.rs        # Proof of Longevity
│   │   ├── validator/
│   │   │   ├── mod.rs
│   │   │   ├── selection.rs        # Sélection des validateurs
│   │   │   ├── rewards.rs          # Calcul des récompenses
│   │   │   └── penalties.rs        # Système de pénalités
│   │   └── metrics.rs              # Métriques de consensus
│   └── tests/
│
├── storage/                        # Module de stockage
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── engine/
│   │   │   ├── mod.rs
│   │   │   ├── interface.rs        # Interface de stockage
│   │   │   └── local.rs            # Stockage local
│   │   ├── compression/
│   │   │   ├── mod.rs
│   │   │   ├── algorithms.rs       # Algorithmes de compression
│   │   │   └── web_optimized.rs    # Compression optimisée web
│   │   ├── replication/
│   │   │   ├── mod.rs
│   │   │   ├── strategy.rs         # Stratégies de réplication
│   │   │   ├── geographic.rs       # Réplication géographique
│   │   │   └── smart.rs            # Réplication intelligente
│   │   ├── integrity/
│   │   │   ├── mod.rs
│   │   │   ├── checker.rs          # Vérificateur d'intégrité
│   │   │   ├── repair.rs           # Réparation automatique
│   │   │   └── monitoring.rs       # Monitoring continu
│   │   └── lifecycle/
│   │       ├── mod.rs
│   │       ├── manager.rs          # Gestionnaire de cycle de vie
│   │       ├── policies.rs         # Politiques de rétention
│   │       └── cleanup.rs          # Nettoyage automatique
│   └── tests/
│
├── network/                        # Module réseau P2P
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── p2p/
│   │   │   ├── mod.rs
│   │   │   ├── swarm.rs            # Gestionnaire de swarm
│   │   │   ├── discovery.rs        # Découverte de pairs
│   │   │   └── routing.rs          # Routage des messages
│   │   ├── protocols/
│   │   │   ├── mod.rs
│   │   │   ├── gossip.rs           # Protocole de gossip
│   │   │   ├── sync.rs             # Synchronisation
│   │   │   └── request_response.rs # Requête-réponse
│   │   ├── transport/
│   │   │   ├── mod.rs
│   │   │   ├── tcp.rs              # Transport TCP
│   │   │   ├── quic.rs             # Transport QUIC
│   │   │   └── websocket.rs        # Transport WebSocket
│   │   └── security/
│   │       ├── mod.rs
│   │       ├── tls.rs              # Configuration TLS
│   │       └── auth.rs             # Authentification
│   └── tests/
│
├── smart-contracts/                # Module smart contracts
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── vm/
│   │   │   ├── mod.rs
│   │   │   ├── wasm.rs             # Runtime WASM
│   │   │   ├── sandbox.rs          # Environnement sécurisé
│   │   │   └── limits.rs           # Limites de ressources
│   │   ├── contracts/
│   │   │   ├── mod.rs
│   │   │   ├── archive_bounty.rs   # Contrat de bounty
│   │   │   ├── preservation_pool.rs # Pool de préservation
│   │   │   └── content_validator.rs # Validateur de contenu
│   │   ├── abi/
│   │   │   ├── mod.rs
│   │   │   ├── types.rs            # Types ABI
│   │   │   └── encoding.rs         # Encodage ABI
│   │   └── gas/
│   │       ├── mod.rs
│   │       ├── meter.rs            # Compteur de gas
│   │       └── pricing.rs          # Tarification gas
│   ├── contracts/                  # Contrats WASM pré-compilés
│   │   ├── archive_bounty.wasm
│   │   ├── preservation_pool.wasm
│   │   └── content_validator.wasm
│   └── tests/
│
├── search/                         # Module de recherche DHT
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── dht/
│   │   │   ├── mod.rs
│   │   │   ├── kademlia.rs         # Implémentation Kademlia
│   │   │   ├── routing_table.rs    # Table de routage
│   │   │   └── storage.rs          # Stockage DHT
│   │   ├── index/
│   │   │   ├── mod.rs
│   │   │   ├── builder.rs          # Construction d'index
│   │   │   ├── keywords.rs         # Index par mots-clés
│   │   │   └── semantic.rs         # Index sémantique
│   │   ├── query/
│   │   │   ├── mod.rs
│   │   │   ├── parser.rs           # Parseur de requêtes
│   │   │   ├── executor.rs         # Exécuteur de requêtes
│   │   │   └── ranking.rs          # Système de ranking
│   │   └── cache/
│   │       ├── mod.rs
│   │       ├── lru.rs              # Cache LRU
│   │       └── distributed.rs     # Cache distribué
│   └── tests/
│
├── nodes/                          # Module types de nœuds
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── types/
│   │   │   ├── mod.rs
│   │   │   ├── full_archive.rs     # Nœud Full Archive
│   │   │   ├── light_storage.rs    # Nœud Light Storage
│   │   │   ├── relay.rs            # Nœud Relay
│   │   │   └── gateway.rs          # Nœud Gateway
│   │   ├── manager/
│   │   │   ├── mod.rs
│   │   │   ├── lifecycle.rs        # Cycle de vie des nœuds
│   │   │   ├── discovery.rs        # Découverte de types
│   │   │   └── health.rs           # Monitoring de santé
│   │   ├── config/
│   │   │   ├── mod.rs
│   │   │   ├── profiles.rs         # Profils de configuration
│   │   │   └── validation.rs       # Validation config
│   │   └── coordination/
│   │       ├── mod.rs
│   │       ├── load_balancing.rs   # Équilibrage de charge
│   │       └── failover.rs         # Basculement automatique
│   └── tests/
│
├── api/                           # Module API publique
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── rest/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs           # Serveur REST
│   │   │   ├── routes/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── archives.rs     # Endpoints archives
│   │   │   │   ├── search.rs       # Endpoints recherche
│   │   │   │   ├── nodes.rs        # Endpoints nœuds
│   │   │   │   └── stats.rs        # Endpoints statistiques
│   │   │   └── middleware/
│   │   │       ├── mod.rs
│   │   │       ├── auth.rs         # Authentification
│   │   │       ├── rate_limit.rs   # Limitation de taux
│   │   │       └── cors.rs         # Configuration CORS
│   │   ├── graphql/
│   │   │   ├── mod.rs
│   │   │   ├── schema.rs           # Schéma GraphQL
│   │   │   ├── resolvers/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── archive.rs      # Résolveurs archives
│   │   │   │   ├── search.rs       # Résolveurs recherche
│   │   │   │   └── network.rs      # Résolveurs réseau
│   │   │   └── subscriptions.rs    # Souscriptions temps réel
│   │   ├── websocket/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs           # Serveur WebSocket
│   │   │   ├── handlers.rs         # Gestionnaires d'événements
│   │   │   └── broadcast.rs        # Diffusion d'événements
│   │   └── docs/
│   │       ├── mod.rs
│   │       ├── openapi.rs          # Génération OpenAPI
│   │       └── examples.rs         # Exemples d'utilisation
│   └── tests/
│
├── tokenomics/                    # Module économie du token
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── token/
│   │   │   ├── mod.rs
│   │   │   ├── arc.rs              # Token ARC principal
│   │   │   ├── supply.rs           # Gestion de l'offre
│   │   │   └── distribution.rs     # Distribution initiale
│   │   ├── incentives/
│   │   │   ├── mod.rs
│   │   │   ├── archive.rs          # Incitations archivage
│   │   │   ├── validation.rs       # Incitations validation
│   │   │   └── storage.rs          # Incitations stockage
│   │   ├── deflation/
│   │   │   ├── mod.rs
│   │   │   ├── burning.rs          # Mécanisme de burn
│   │   │   ├── fees.rs             # Gestion des frais
│   │   │   └── triggers.rs         # Déclencheurs automatiques
│   │   ├── rewards/
│   │   │   ├── mod.rs
│   │   │   ├── calculator.rs       # Calculateur de récompenses
│   │   │   ├── distributor.rs      # Distributeur de récompenses
│   │   │   └── vesting.rs          # Périodes d'acquisition
│   │   └── governance/
│   │       ├── mod.rs
│   │       ├── voting.rs           # Système de vote
│   │       ├── proposals.rs        # Gestion des propositions
│   │       └── treasury.rs         # Trésorerie communautaire
│   └── tests/
│
├── tools/                         # Outils de développement
│   ├── cli/                       # Interface en ligne de commande
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── commands/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── node.rs         # Commandes nœud
│   │   │   │   ├── archive.rs      # Commandes archivage
│   │   │   │   ├── search.rs       # Commandes recherche
│   │   │   │   └── wallet.rs       # Commandes portefeuille
│   │   │   ├── config/
│   │   │   │   ├── mod.rs
│   │   │   │   └── loader.rs       # Chargeur de configuration
│   │   │   └── utils/
│   │   │       ├── mod.rs
│   │   │       ├── logger.rs       # Configuration logs
│   │   │       └── validator.rs    # Validation des entrées
│   │   └── tests/
│   │
│   ├── keygen/                    # Générateur de clés
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── generator.rs        # Génération de clés
│   │       └── formats.rs          # Formats d'export
│   │
│   └── benchmark/                 # Outils de benchmark
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── consensus.rs        # Benchmark consensus
│           ├── storage.rs          # Benchmark stockage
│           └── network.rs          # Benchmark réseau
│
├── tests/                         # Tests d'intégration globaux
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── end_to_end.rs          # Tests bout en bout
│   │   ├── performance.rs          # Tests de performance
│   │   └── security.rs             # Tests de sécurité
│   ├── fixtures/
│   │   ├── configs/                # Configurations de test
│   │   ├── data/                   # Données de test
│   │   └── keys/                   # Clés de test
│   └── utils/
│       ├── mod.rs
│       ├── setup.rs                # Configuration des tests
│       └── helpers.rs              # Fonctions d'aide
│
├── docs/                          # Documentation technique
│   ├── architecture/
│   │   ├── README.md
│   │   ├── consensus.md            # Documentation consensus
│   │   ├── storage.md              # Documentation stockage
│   │   └── network.md              # Documentation réseau
│   ├── api/
│   │   ├── README.md
│   │   ├── rest.md                 # Documentation REST API
│   │   ├── graphql.md              # Documentation GraphQL
│   │   └── websocket.md            # Documentation WebSocket
│   ├── deployment/
│   │   ├── README.md
│   │   ├── docker.md               # Déploiement Docker
│   │   ├── kubernetes.md           # Déploiement Kubernetes
│   │   └── cloud.md                # Déploiement cloud
│   ├── development/
│   │   ├── README.md
│   │   ├── setup.md                # Configuration développement
│   │   ├── testing.md              # Guide des tests
│   │   └── contributing.md         # Guide de contribution
│   └── examples/
│       ├── README.md
│       ├── basic_usage.md          # Utilisation de base
│       ├── advanced.md             # Utilisation avancée
│       └── integrations.md         # Intégrations tierces
│
├── config/                        # Configurations par défaut
│   ├── node/
│   │   ├── full_archive.toml       # Config nœud Full Archive
│   │   ├── light_storage.toml      # Config nœud Light Storage
│   │   ├── relay.toml              # Config nœud Relay
│   │   └── gateway.toml            # Config nœud Gateway
│   ├── network/
│   │   ├── mainnet.toml            # Configuration mainnet
│   │   ├── testnet.toml            # Configuration testnet
│   │   └── devnet.toml             # Configuration devnet
│   └── consensus/
│       ├── poa.toml                # Configuration PoA
│       └── genesis.toml            # Configuration genesis
│
├── scripts/                       # Scripts d'automatisation
│   ├── build.sh                   # Script de build
│   ├── test.sh                    # Script de tests
│   ├── deploy.sh                  # Script de déploiement
│   ├── setup_dev.sh               # Configuration développement
│   └── benchmark.sh               # Script de benchmark
│
├── docker/                        # Configurations Docker
│   ├── Dockerfile                 # Image principale
│   ├── docker-compose.yml         # Composition multi-services
│   ├── node/
│   │   ├── Dockerfile.full        # Image nœud Full Archive
│   │   ├── Dockerfile.light       # Image nœud Light Storage
│   │   ├── Dockerfile.relay       # Image nœud Relay
│   │   └── Dockerfile.gateway     # Image nœud Gateway
│   └── scripts/
│       ├── entrypoint.sh          # Point d'entrée
│       └── healthcheck.sh         # Vérification de santé
│
└── .github/                       # Configuration GitHub
    ├── workflows/
    │   ├── ci.yml                 # Intégration continue
    │   ├── release.yml            # Publication de releases
    │   └── security.yml           # Audit de sécurité
    ├── ISSUE_TEMPLATE/
    │   ├── bug_report.md          # Template bug report
    │   └── feature_request.md     # Template demande de fonctionnalité
    └── PULL_REQUEST_TEMPLATE.md   # Template pull request
```

## Technologies et Dépendances

### Workspace Principal
- **Rust** 1.70+ (édition 2021)
- **Cargo** pour la gestion des dépendances
- **Workspace** pour la structure modulaire

### Dépendances Principales par Module

#### Core
- `serde` : Sérialisation/désérialisation
- `blake3` : Fonction de hachage
- `ed25519-dalek` : Signatures numériques
- `merkle` : Arbres de Merkle
- `thiserror` : Gestion d'erreurs

#### Consensus
- `async-trait` : Traits asynchrones
- `rand` : Génération aléatoire
- `chrono` : Gestion du temps
- `futures` : Programmation asynchrone

#### Storage
- `lz4` : Compression rapide
- `zstd` : Compression haute performance
- `async-fs` : Système de fichiers asynchrone
- `crc` : Sommes de contrôle

#### Network
- `libp2p` : Stack P2P complète
- `tokio` : Runtime asynchrone
- `quinn` : Implémentation QUIC
- `rustls` : TLS en Rust

#### Smart Contracts
- `wasmtime` : Runtime WASM
- `wasm-bindgen` : Bindings WASM
- `serde-wasm-bindgen` : Sérialisation WASM

#### Search
- `tantivy` : Moteur de recherche
- `regex` : Expressions régulières
- `unicode-normalization` : Normalisation Unicode

#### API
- `actix-web` : Framework web
- `async-graphql` : Serveur GraphQL
- `tokio-tungstenite` : WebSocket
- `utoipa` : Génération OpenAPI

#### Tokenomics
- `rust_decimal` : Calculs décimaux précis
- `bigdecimal` : Arithmétique haute précision

## Instructions de Build

```bash
# Build complet du workspace
cargo build --release

# Build d'un module spécifique
cargo build -p archivechain-core

# Tests complets
cargo test --all

# Génération de documentation
cargo doc --all --no-deps
```

## Configuration de Développement

1. **Rust** 1.70+ installé
2. **Git** pour le contrôle de version
3. **Docker** pour les tests d'intégration
4. **IDE** avec support Rust (VS Code + rust-analyzer recommandé)

## Principes d'Organisation

- **Modularité** : Chaque module est indépendant avec son propre `Cargo.toml`
- **Testabilité** : Tests unitaires et d'intégration séparés
- **Documentation** : Documentation exhaustive dans `/docs`
- **Configuration** : Fichiers de config externalisés dans `/config`
- **Outils** : Utilitaires de développement dans `/tools`
- **CI/CD** : Pipelines automatisés dans `.github/workflows`