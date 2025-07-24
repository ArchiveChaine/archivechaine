# Guide des APIs ArchiveChain

## Table des Matières

- [Vue d'Ensemble](#vue-densemble)
- [Authentification](#authentification)
- [REST API](#rest-api)
- [GraphQL API](#graphql-api)
- [WebSocket API](#websocket-api)
- [gRPC API](#grpc-api)
- [SDKs et Clients](#sdks-et-clients)
- [Codes d'Erreur](#codes-derreur)
- [Rate Limiting](#rate-limiting)
- [Exemples Pratiques](#exemples-pratiques)
- [Troubleshooting](#troubleshooting)

## Vue d'Ensemble

ArchiveChain expose **5 interfaces API** pour différents cas d'usage et besoins de performance :

| API | Usage Principal | Performance | Complexité |
|-----|-----------------|-------------|------------|
| **REST** | Intégrations simples, CRUD | Standard | Faible |
| **GraphQL** | Requêtes flexibles, optimisées | Élevée | Moyenne |
| **WebSocket** | Temps réel, streaming | Élevée | Moyenne |
| **gRPC** | Inter-services, haute performance | Très élevée | Élevée |
| **P2P** | Communication entre nœuds | Native | Très élevée |

### Endpoints de Base

```bash
# Production
https://api.archivechain.org/v1

# Testnet
https://testnet-api.archivechain.org/v1

# Développement local
http://localhost:8080/v1
```

## Authentification

### 1. Clés API

#### Génération
```bash
# Via CLI
archivechain-cli auth create-key \
  --name "Mon Application" \
  --scopes "archives:read,archives:write"

# Réponse
API Key: arc_1234567890abcdef1234567890abcdef
Secret: secret_abcdef1234567890abcdef1234567890
```

#### Utilisation
```bash
# Header HTTP
Authorization: Bearer arc_1234567890abcdef1234567890abcdef

# Query parameter (non recommandé en production)
?api_key=arc_1234567890abcdef1234567890abcdef
```

### 2. JWT Tokens

#### Structure
```json
{
  "header": {
    "alg": "EdDSA",
    "typ": "JWT"
  },
  "payload": {
    "sub": "user_1234567890",
    "iss": "archivechain.org",
    "aud": "api.archivechain.org",
    "exp": 1705320600,
    "iat": 1705234200,
    "scope": ["archives:read", "archives:write", "search:read"],
    "rate_limit": {
      "requests_per_hour": 1000,
      "storage_limit_gb": 100
    }
  }
}
```

#### Obtention
```javascript
// JavaScript/Node.js
const response = await fetch('https://api.archivechain.org/v1/auth/token', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    api_key: 'arc_1234567890abcdef1234567890abcdef',
    secret: 'secret_abcdef1234567890abcdef1234567890',
    expires_in: 3600
  })
});

const { token } = await response.json();
```

### 3. Scopes et Permissions

| Scope | Description | Niveau |
|-------|-------------|--------|
| `archives:read` | Lecture des archives | Standard |
| `archives:write` | Création/modification | Standard |
| `archives:delete` | Suppression d'archives | Élevé |
| `search:read` | Recherche d'archives | Standard |
| `network:read` | Statistiques réseau | Standard |
| `node:manage` | Gestion de nœud | Élevé |
| `admin:all` | Accès administrateur | Critique |

## REST API

### 1. Archives

#### Créer une Archive
```http
POST /v1/archives
Content-Type: application/json
Authorization: Bearer {token}

{
  "url": "https://example.com/article.html",
  "metadata": {
    "title": "Article Important",
    "description": "Description de l'article",
    "tags": ["news", "technology"],
    "priority": "high",
    "retention_period": "permanent"
  },
  "options": {
    "include_assets": true,
    "max_depth": 3,
    "preserve_javascript": false,
    "allowed_domains": ["example.com", "cdn.example.com"],
    "exclude_patterns": ["*.ads.*", "*tracker*"]
  }
}
```

**Réponse 201 Created:**
```json
{
  "archive_id": "arc_1234567890abcdef",
  "status": "pending",
  "estimated_completion": "2024-01-15T10:35:00Z",
  "cost_estimation": {
    "storage_cost": "0.001 ARC",
    "processing_cost": "0.0005 ARC",
    "total_cost": "0.0015 ARC"
  },
  "tracking_url": "https://api.archivechain.org/v1/archives/arc_1234567890abcdef/status"
}
```

#### Récupérer une Archive
```http
GET /v1/archives/{archive_id}
Authorization: Bearer {token}
```

**Réponse 200 OK:**
```json
{
  "archive_id": "arc_1234567890abcdef",
  "url": "https://example.com/article.html",
  "status": "completed",
  "created_at": "2024-01-15T10:00:00Z",
  "completed_at": "2024-01-15T10:30:00Z",
  "size": 2048576,
  "metadata": {
    "title": "Article Important",
    "description": "Description de l'article",
    "mime_type": "text/html",
    "language": "en",
    "author": "John Doe",
    "published_at": "2024-01-15T09:00:00Z"
  },
  "storage_info": {
    "replicas": 7,
    "locations": ["us-east-1", "eu-west-1", "ap-southeast-1"],
    "integrity_score": 0.999,
    "last_verified": "2024-01-15T10:25:00Z"
  },
  "access_urls": {
    "view": "https://gateway.archivechain.org/view/arc_1234567890abcdef",
    "download": "https://gateway.archivechain.org/download/arc_1234567890abcdef",
    "raw": "https://gateway.archivechain.org/raw/arc_1234567890abcdef"
  },
  "blockchain_info": {
    "block_height": 12547,
    "transaction_hash": "0x1234567890abcdef...",
    "confirmations": 32
  }
}
```

#### Lister les Archives
```http
GET /v1/archives?page=1&limit=50&status=completed&tag=news&sort=created_at:desc
Authorization: Bearer {token}
```

**Paramètres de requête:**
- `page` - Numéro de page (défaut: 1)
- `limit` - Éléments par page (max: 100)
- `status` - Filtrer par statut (`pending`, `processing`, `completed`, `failed`)
- `tag` - Filtrer par tag
- `url` - Filtrer par URL (recherche partielle)
- `date_from` - Date de début (ISO 8601)
- `date_to` - Date de fin (ISO 8601)
- `sort` - Tri (`created_at`, `size`, `title`) + direction (`:asc`, `:desc`)

#### Mettre à Jour une Archive
```http
PATCH /v1/archives/{archive_id}
Content-Type: application/json
Authorization: Bearer {token}

{
  "metadata": {
    "tags": ["news", "technology", "breaking"],
    "description": "Description mise à jour"
  },
  "retention_period": "10_years"
}
```

#### Supprimer une Archive
```http
DELETE /v1/archives/{archive_id}
Authorization: Bearer {token}
```

### 2. Recherche

#### Recherche Simple
```http
GET /v1/search?q=climate+change&limit=20&page=1
Authorization: Bearer {token}
```

#### Recherche Avancée
```http
POST /v1/search
Content-Type: application/json
Authorization: Bearer {token}

{
  "query": "climate change",
  "filters": {
    "content_type": ["text/html", "application/pdf"],
    "domains": ["*.edu", "*.gov"],
    "date_range": {
      "start": "2023-01-01",
      "end": "2024-01-01"
    },
    "language": ["en", "fr"],
    "size_range": {
      "min": 1024,
      "max": 10485760
    }
  },
  "sort": {
    "field": "relevance",
    "order": "desc"
  },
  "facets": ["domain", "content_type", "language", "date"],
  "highlight": {
    "fields": ["title", "content"],
    "max_length": 200
  }
}
```

**Réponse:**
```json
{
  "query": "climate change",
  "total_results": 1547,
  "search_time_ms": 23,
  "results": [
    {
      "archive_id": "arc_abcdef1234567890",
      "url": "https://climate.gov/evidence",
      "title": "Climate Change Evidence",
      "snippet": "Scientific evidence for <mark>climate change</mark>...",
      "relevance_score": 0.95,
      "archived_at": "2024-01-15T10:00:00Z",
      "size": 1024576,
      "content_type": "text/html",
      "language": "en"
    }
  ],
  "facets": {
    "domains": {
      "climate.gov": 45,
      "epa.gov": 32,
      "nature.com": 28
    },
    "content_types": {
      "text/html": 1200,
      "application/pdf": 347
    },
    "languages": {
      "en": 1400,
      "fr": 89,
      "de": 58
    }
  }
}
```

### 3. Réseau et Statistiques

#### Statistiques Générales
```http
GET /v1/network/stats
Authorization: Bearer {token}
```

**Réponse:**
```json
{
  "network": {
    "total_nodes": 1247,
    "active_nodes": 1198,
    "node_types": {
      "full_archive": 156,
      "light_storage": 892,
      "relay": 98,
      "gateway": 52
    },
    "total_storage": "15.7 TB",
    "available_storage": "8.3 TB",
    "current_block_height": 245671,
    "network_hash_rate": "1.2 TH/s"
  },
  "archives": {
    "total_archives": 567890,
    "archives_today": 1234,
    "total_size": "12.4 TB",
    "average_replication": 4.2,
    "success_rate": 0.987
  },
  "economic": {
    "total_supply": "100000000000",
    "circulating_supply": "15000000000",
    "staked_amount": "8500000000",
    "treasury_balance": "20000000000",
    "average_rewards": "150 ARC/day"
  },
  "performance": {
    "average_archive_time": "2.3 minutes",
    "network_latency": "45ms",
    "throughput": "250 archives/hour",
    "availability": 0.9999
  }
}
```

## GraphQL API

### 1. Schema Principal

```graphql
type Query {
  # Archives
  archive(id: ID!): Archive
  archives(
    filter: ArchiveFilter
    sort: ArchiveSort
    first: Int
    after: String
  ): ArchiveConnection!
  
  # Recherche
  searchArchives(
    query: String!
    filters: SearchFilters
    first: Int
    after: String
  ): SearchConnection!
  
  # Réseau
  networkStats: NetworkStats!
  nodes(status: NodeStatus, type: NodeType): [Node!]!
  
  # Utilisateur
  me: User!
  myUsage: UsageStats!
  myArchives(first: Int, after: String): ArchiveConnection!
}

type Mutation {
  # Archives
  createArchive(input: CreateArchiveInput!): CreateArchivePayload!
  updateArchive(id: ID!, input: UpdateArchiveInput!): UpdateArchivePayload!
  deleteArchive(id: ID!): DeleteArchivePayload!
  
  # Utilisateur
  updateProfile(input: UpdateProfileInput!): UpdateProfilePayload!
}

type Subscription {
  # Temps réel
  archiveStatusUpdated(archiveId: ID!): Archive!
  newArchiveCreated(userId: ID): Archive!
  networkStatsUpdated: NetworkStats!
  nodeStatusChanged(nodeId: ID): Node!
}
```

### 2. Types Principaux

```graphql
type Archive {
  id: ID!
  url: String!
  status: ArchiveStatus!
  metadata: ArchiveMetadata!
  storageInfo: StorageInfo!
  blockchainInfo: BlockchainInfo!
  createdAt: DateTime!
  completedAt: DateTime
  size: Int!
  cost: TokenAmount!
  accessUrls: AccessUrls!
}

enum ArchiveStatus {
  PENDING
  PROCESSING
  COMPLETED
  FAILED
  EXPIRED
}

type ArchiveMetadata {
  title: String
  description: String
  tags: [String!]!
  contentType: String!
  language: String
  author: String
  publishedAt: DateTime
  retentionPeriod: RetentionPeriod
}

type StorageInfo {
  replicas: Int!
  locations: [String!]!
  integrityScore: Float!
  lastVerified: DateTime!
  redundancyLevel: RedundancyLevel!
}

input CreateArchiveInput {
  url: String!
  metadata: ArchiveMetadataInput
  options: ArchiveOptionsInput
}

input ArchiveOptionsInput {
  includeAssets: Boolean = true
  maxDepth: Int = 3
  preserveJavascript: Boolean = false
  allowedDomains: [String!]
  excludePatterns: [String!]
  priority: Priority = NORMAL
}
```

### 3. Exemples de Requêtes

#### Récupérer des Archives avec Métadonnées
```graphql
query GetMyArchives($first: Int!, $filter: ArchiveFilter) {
  myArchives(first: $first, filter: $filter) {
    edges {
      node {
        id
        url
        status
        metadata {
          title
          tags
          contentType
          language
        }
        storageInfo {
          replicas
          integrityScore
          locations
        }
        createdAt
        size
        cost {
          amount
          currency
        }
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
    totalCount
  }
}
```

**Variables:**
```json
{
  "first": 20,
  "filter": {
    "status": "COMPLETED",
    "tags": ["important"],
    "createdAfter": "2024-01-01T00:00:00Z"
  }
}
```

#### Recherche avec Facettes
```graphql
query SearchWithFacets($query: String!, $filters: SearchFilters, $first: Int!) {
  searchArchives(query: $query, filters: $filters, first: $first) {
    edges {
      node {
        id
        url
        metadata {
          title
          snippet
          language
        }
        relevanceScore
        archivedAt
      }
    }
    facets {
      domains {
        value
        count
      }
      contentTypes {
        value
        count
      }
      languages {
        value
        count
      }
    }
    totalCount
    searchTimeMs
  }
}
```

#### Créer une Archive
```graphql
mutation CreateArchive($input: CreateArchiveInput!) {
  createArchive(input: $input) {
    archive {
      id
      url
      status
      estimatedCompletion
      costEstimation {
        storageCost
        processingCost
        totalCost
      }
    }
    errors {
      field
      message
    }
  }
}
```

**Variables:**
```json
{
  "input": {
    "url": "https://example.com/important-article",
    "metadata": {
      "title": "Article Important",
      "description": "Article à préserver",
      "tags": ["news", "important"]
    },
    "options": {
      "includeAssets": true,
      "maxDepth": 2,
      "priority": "HIGH"
    }
  }
}
```

### 4. Subscriptions en Temps Réel

```graphql
# Suivre le statut d'une archive
subscription ArchiveUpdates($archiveId: ID!) {
  archiveStatusUpdated(archiveId: $archiveId) {
    id
    status
    progress
    metadata {
      title
    }
    storageInfo {
      replicas
      integrityScore
    }
    completedAt
  }
}

# Nouvelles archives créées
subscription NewArchives {
  newArchiveCreated {
    id
    url
    metadata {
      title
      tags
    }
    createdAt
    creator {
      username
    }
  }
}

# Statistiques réseau en temps réel
subscription NetworkUpdates {
  networkStatsUpdated {
    network {
      activeNodes
      totalStorage
      currentBlockHeight
    }
    archives {
      totalArchives
      archivesToday
    }
    timestamp
  }
}
```

## WebSocket API

### 1. Connexion et Authentification

```javascript
// Établir la connexion WebSocket
const ws = new WebSocket('wss://api.archivechain.org/v1/ws');

// Authentification
ws.onopen = () => {
  ws.send(JSON.stringify({
    type: 'auth',
    token: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...'
  }));
};

// Gestion des messages
ws.onmessage = (event) => {
  const message = JSON.parse(event.data);
  handleMessage(message);
};
```

### 2. Types de Messages

#### Souscription aux Mises à Jour
```javascript
// S'abonner aux mises à jour d'une archive
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'archive_updates',
  archive_id: 'arc_1234567890abcdef',
  options: {
    include_progress: true,
    include_storage_info: true
  }
}));

// S'abonner aux statistiques réseau
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'network_stats',
  interval: 30 // secondes
}));

// S'abonner aux nouveaux blocs
ws.send(JSON.stringify({
  type: 'subscribe',
  channel: 'new_blocks',
  include_transactions: true
}));
```

#### Messages de Réponse
```javascript
// Mise à jour statut archive
{
  "type": "archive_update",
  "archive_id": "arc_1234567890abcdef",
  "status": "processing",
  "progress": 65,
  "phase": "downloading_assets",
  "data": {
    "downloaded_size": 1048576,
    "total_estimated_size": 1612800,
    "assets_completed": 12,
    "assets_total": 18
  },
  "timestamp": "2024-01-15T10:25:30Z"
}

// Archive terminée
{
  "type": "archive_completed",
  "archive_id": "arc_1234567890abcdef",
  "final_size": 1587200,
  "storage_info": {
    "replicas": 7,
    "locations": ["us-east-1", "eu-west-1", "ap-southeast-1"],
    "integrity_score": 0.999
  },
  "access_urls": {
    "view": "https://gateway.archivechain.org/view/arc_1234567890abcdef",
    "download": "https://gateway.archivechain.org/download/arc_1234567890abcdef"
  },
  "timestamp": "2024-01-15T10:30:00Z"
}

// Nouveau bloc
{
  "type": "new_block",
  "block": {
    "height": 245672,
    "hash": "0x1234567890abcdef...",
    "timestamp": "2024-01-15T10:30:00Z",
    "transactions": 25,
    "archives": 12,
    "validator": "node_abcdef1234567890",
    "size": 2048576
  }
}
```

### 3. Gestion des Erreurs et Reconnexion

```javascript
class ArchiveChainWebSocket {
  constructor(token) {
    this.token = token;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.subscriptions = new Set();
    this.connect();
  }

  connect() {
    this.ws = new WebSocket('wss://api.archivechain.org/v1/ws');
    
    this.ws.onopen = () => {
      console.log('Connected to ArchiveChain WebSocket');
      this.authenticate();
      this.resubscribe();
      this.reconnectAttempts = 0;
    };

    this.ws.onmessage = (event) => {
      const message = JSON.parse(event.data);
      this.handleMessage(message);
    };

    this.ws.onclose = (event) => {
      console.log('WebSocket closed:', event.code, event.reason);
      this.attemptReconnect();
    };

    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
    };
  }

  authenticate() {
    this.send({
      type: 'auth',
      token: this.token
    });
  }

  subscribe(channel, options = {}) {
    const subscription = { channel, ...options };
    this.subscriptions.add(subscription);
    this.send({
      type: 'subscribe',
      ...subscription
    });
  }

  send(message) {
    if (this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify(message));
    }
  }

  attemptReconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.pow(2, this.reconnectAttempts) * 1000; // Exponential backoff
      setTimeout(() => this.connect(), delay);
    }
  }

  resubscribe() {
    this.subscriptions.forEach(sub => {
      this.send({ type: 'subscribe', ...sub });
    });
  }
}
```

## gRPC API

### 1. Définitions des Services

```protobuf
syntax = "proto3";
package archivechain.v1;

import "google/protobuf/timestamp.proto";
import "google/protobuf/empty.proto";

// Service principal d'archivage
service ArchiveService {
  // Archives
  rpc CreateArchive(CreateArchiveRequest) returns (CreateArchiveResponse);
  rpc GetArchive(GetArchiveRequest) returns (Archive);
  rpc ListArchives(ListArchivesRequest) returns (ListArchivesResponse);
  rpc UpdateArchive(UpdateArchiveRequest) returns (Archive);
  rpc DeleteArchive(DeleteArchiveRequest) returns (google.protobuf.Empty);
  
  // Recherche
  rpc SearchArchives(SearchRequest) returns (SearchResponse);
  
  // Streaming
  rpc StreamArchiveUpdates(StreamRequest) returns (stream ArchiveUpdate);
  rpc StreamNetworkStats(google.protobuf.Empty) returns (stream NetworkStats);
}

// Messages
message Archive {
  string id = 1;
  string url = 2;
  ArchiveStatus status = 3;
  ArchiveMetadata metadata = 4;
  StorageInfo storage_info = 5;
  google.protobuf.Timestamp created_at = 6;
  google.protobuf.Timestamp completed_at = 7;
  int64 size = 8;
  TokenAmount cost = 9;
  AccessUrls access_urls = 10;
}

enum ArchiveStatus {
  ARCHIVE_STATUS_UNSPECIFIED = 0;
  ARCHIVE_STATUS_PENDING = 1;
  ARCHIVE_STATUS_PROCESSING = 2;
  ARCHIVE_STATUS_COMPLETED = 3;
  ARCHIVE_STATUS_FAILED = 4;
  ARCHIVE_STATUS_EXPIRED = 5;
}

message CreateArchiveRequest {
  string url = 1;
  ArchiveOptions options = 2;
  map<string, string> metadata = 3;
}

message ArchiveOptions {
  bool include_assets = 1;
  int32 max_depth = 2;
  bool preserve_javascript = 3;
  repeated string allowed_domains = 4;
  repeated string exclude_patterns = 5;
  Priority priority = 6;
}

enum Priority {
  PRIORITY_UNSPECIFIED = 0;
  PRIORITY_LOW = 1;
  PRIORITY_NORMAL = 2;
  PRIORITY_HIGH = 3;
  PRIORITY_URGENT = 4;
}
```

### 2. Client Rust

```rust
use archivechain_grpc::archive_service_client::ArchiveServiceClient;
use archivechain_grpc::{CreateArchiveRequest, ArchiveOptions, Priority};
use tonic::transport::Channel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connexion au service
    let channel = Channel::from_static("https://api.archivechain.org:9091")
        .connect()
        .await?;
    
    let mut client = ArchiveServiceClient::new(channel);

    // Créer une archive
    let request = tonic::Request::new(CreateArchiveRequest {
        url: "https://example.com".to_string(),
        options: Some(ArchiveOptions {
            include_assets: true,
            max_depth: 3,
            preserve_javascript: false,
            allowed_domains: vec!["example.com".to_string()],
            exclude_patterns: vec!["*.ads.*".to_string()],
            priority: Priority::High as i32,
        }),
        metadata: HashMap::from([
            ("title".to_string(), "Test Archive".to_string()),
            ("tags".to_string(), "test,demo".to_string()),
        ]),
    });

    let response = client.create_archive(request).await?;
    println!("Archive créée: {}", response.get_ref().archive_id);

    // Stream des mises à jour
    let stream_request = tonic::Request::new(StreamRequest {
        archive_id: response.get_ref().archive_id.clone(),
    });
    
    let mut stream = client.stream_archive_updates(stream_request).await?.into_inner();
    
    while let Some(update) = stream.message().await? {
        println!("Update: {:?}", update);
    }

    Ok(())
}
```

### 3. Client Go

```go
package main

import (
    "context"
    "log"
    "time"
    
    "google.golang.org/grpc"
    pb "github.com/archivechain/proto/go"
)

func main() {
    // Connexion
    conn, err := grpc.Dial("api.archivechain.org:9091", grpc.WithInsecure())
    if err != nil {
        log.Fatalf("Connexion échouée: %v", err)
    }
    defer conn.Close()

    client := pb.NewArchiveServiceClient(conn)

    // Créer une archive
    ctx, cancel := context.WithTimeout(context.Background(), time.Second*30)
    defer cancel()

    response, err := client.CreateArchive(ctx, &pb.CreateArchiveRequest{
        Url: "https://example.com",
        Options: &pb.ArchiveOptions{
            IncludeAssets: true,
            MaxDepth: 3,
            Priority: pb.Priority_PRIORITY_HIGH,
        },
        Metadata: map[string]string{
            "title": "Test Archive",
            "tags":  "test,demo",
        },
    })

    if err != nil {
        log.Fatalf("Erreur création archive: %v", err)
    }

    log.Printf("Archive créée: %s", response.ArchiveId)

    // Stream des mises à jour
    stream, err := client.StreamArchiveUpdates(ctx, &pb.StreamRequest{
        ArchiveId: response.ArchiveId,
    })

    if err != nil {
        log.Fatalf("Erreur stream: %v", err)
    }

    for {
        update, err := stream.Recv()
        if err != nil {
            log.Printf("Stream terminé: %v", err)
            break
        }
        log.Printf("Update: %v", update)
    }
}
```

## SDKs et Clients

### 1. SDK JavaScript/TypeScript

#### Installation
```bash
npm install @archivechain/sdk
# ou
yarn add @archivechain/sdk
```

#### Utilisation Basique
```typescript
import { ArchiveChainClient } from '@archivechain/sdk';

const client = new ArchiveChainClient({
  apiKey: 'arc_1234567890abcdef1234567890abcdef',
  baseUrl: 'https://api.archivechain.org/v1',
  network: 'mainnet' // ou 'testnet'
});

// Créer une archive
const archive = await client.archives.create({
  url: 'https://example.com',
  metadata: {
    title: 'Example Page',
    tags: ['web', 'demo']
  },
  options: {
    includeAssets: true,
    maxDepth: 3
  }
});

console.log(`Archive créée: ${archive.id}`);

// Suivre le progrès
client.archives.onStatusChange(archive.id, (status) => {
  console.log(`Statut: ${status.status}, Progrès: ${status.progress}%`);
});

// Rechercher
const results = await client.search.query('climate change', {
  filters: {
    contentType: ['text/html'],
    dateRange: {
      start: '2023-01-01',
      end: '2024-01-01'
    }
  },
  limit: 20
});

results.forEach(result => {
  console.log(`${result.title}: ${result.url}`);
});
```

#### Configuration Avancée
```typescript
const client = new ArchiveChainClient({
  apiKey: process.env.ARCHIVECHAIN_API_KEY,
  baseUrl: process.env.ARCHIVECHAIN_API_URL,
  timeout: 30000,
  retries: 3,
  rateLimit: {
    requestsPerSecond: 10,
    burstSize: 20
  },
  websocket: {
    autoConnect: true,
    reconnectAttempts: 5
  }
});

// Gestion d'erreurs globale
client.on('error', (error) => {
  console.error('Erreur API:', error);
});

// Monitoring des requêtes
client.on('request', (request) => {
  console.log(`Requête: ${request.method} ${request.url}`);
});
```

### 2. SDK Python

#### Installation
```bash
pip install archivechain-sdk
```

#### Utilisation Basique
```python
from archivechain import ArchiveChainClient
import asyncio

client = ArchiveChainClient(
    api_key='arc_1234567890abcdef1234567890abcdef',
    base_url='https://api.archivechain.org/v1'
)

# Créer une archive
archive = client.archives.create(
    url='https://example.com',
    metadata={
        'title': 'Example Page',
        'tags': ['web', 'demo']
    },
    options={
        'include_assets': True,
        'max_depth': 3
    }
)

print(f"Archive créée: {archive.id}")

# Recherche avec pagination
for result in client.search.paginate('climate change', limit=50):
    print(f"Trouvé: {result.url} - {result.title}")

# Async/await support
async def monitor_archive(archive_id):
    async with client.stream.archive_updates(archive_id) as stream:
        async for update in stream:
            print(f"Statut: {update.status}, Progrès: {update.progress}%")
            if update.status == 'completed':
                break

asyncio.run(monitor_archive(archive.id))
```

#### Configuration Avancée
```python
from archivechain import ArchiveChainClient, Config

config = Config(
    api_key=os.getenv('ARCHIVECHAIN_API_KEY'),
    base_url=os.getenv('ARCHIVECHAIN_API_URL'),
    timeout=30.0,
    retries=3,
    rate_limit=RateLimit(
        requests_per_second=10,
        burst_size=20
    )
)

client = ArchiveChainClient(config)

# Context manager pour gestion automatique des ressources
with client:
    # Opérations...
    pass
```

### 3. SDK Rust

#### Cargo.toml
```toml
[dependencies]
archivechain-sdk = "1.0.0"
tokio = { version = "1.0", features = ["full"] }
```

#### Utilisation
```rust
use archivechain_sdk::{ArchiveChainClient, CreateArchiveOptions};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ArchiveChainClient::builder()
        .api_key("arc_1234567890abcdef1234567890abcdef")
        .base_url("https://api.archivechain.org/v1")
        .timeout(std::time::Duration::from_secs(30))
        .build()?;
    
    // Créer une archive
    let archive = client.archives().create("https://example.com")
        .title("Example Page")
        .tags(vec!["web", "demo"])
        .include_assets(true)
        .max_depth(3)
        .send()
        .await?;
    
    println!("Archive créée: {}", archive.id);
    
    // Stream des mises à jour
    let mut stream = client.stream().archive_updates(&archive.id).await?;
    while let Some(update) = stream.next().await {
        println!("Statut: {:?}, Progrès: {}%", update.status, update.progress);
        if update.status == ArchiveStatus::Completed {
            break;
        }
    }
    
    Ok(())
}
```

## Codes d'Erreur

### HTTP Status Codes

| Code | Signification | Description |
|------|---------------|-------------|
| **200** | OK | Requête réussie |
| **201** | Created | Ressource créée avec succès |
| **202** | Accepted | Requête acceptée, traitement en cours |
| **400** | Bad Request | Paramètres de requête invalides |
| **401** | Unauthorized | Authentification requise ou invalide |
| **403** | Forbidden | Permissions insuffisantes |
| **404** | Not Found | Ressource non trouvée |
| **409** | Conflict | Conflit (ex: archive déjà existante) |
| **422** | Unprocessable Entity | Validation des données échouée |
| **429** | Too Many Requests | Limite de taux dépassée |
| **500** | Internal Server Error | Erreur serveur interne |
| **502** | Bad Gateway | Erreur de passerelle |
| **503** | Service Unavailable | Service temporairement indisponible |

### Codes d'Erreur Spécifiques

```json
{
  "error": {
    "code": "ARCHIVE_NOT_FOUND",
    "message": "Archive with ID 'arc_1234567890abcdef' not found",
    "details": {
      "archive_id": "arc_1234567890abcdef",
      "suggestion": "Verify the archive ID or check if it has been deleted"
    },
    "timestamp": "2024-01-15T10:30:00Z",
    "request_id": "req_abcdef1234567890"
  }
}
```

#### Liste des Codes d'Erreur

| Code | Description | Action Recommandée |
|------|-------------|-------------------|
| `INVALID_API_KEY` | Clé API invalide ou expirée | Régénérer la clé API |
| `INSUFFICIENT_PERMISSIONS` | Permissions insuffisantes | Vérifier les scopes requis |
| `ARCHIVE_NOT_FOUND` | Archive introuvable | Vérifier l'ID de l'archive |
| `INVALID_URL` | URL invalide ou inaccessible | Corriger l'URL source |
| `ARCHIVE_TOO_LARGE` | Archive dépasse la taille limite | Réduire le contenu ou upgrade |
| `STORAGE_QUOTA_EXCEEDED` | Quota de stockage dépassé | Upgrade du plan ou nettoyage |
| `RATE_LIMIT_EXCEEDED` | Limite de taux dépassée | Attendre ou upgrade du plan |
| `NETWORK_ERROR` | Erreur réseau temporaire | Réessayer après délai |
| `PROCESSING_FAILED` | Échec du traitement | Vérifier les logs d'erreur |
| `UNSUPPORTED_CONTENT_TYPE` | Type de contenu non supporté | Utiliser un format supporté |

## Rate Limiting

### Limites par Niveau

| Niveau | Requêtes/minute | Requêtes/heure | Concurrent | Stockage |
|--------|-----------------|----------------|------------|-----------|
| **Free** | 60 | 1,000 | 5 | 1 GB |
| **Developer** | 300 | 10,000 | 20 | 100 GB |
| **Professional** | 1,000 | 50,000 | 100 | 1 TB |
| **Enterprise** | 5,000 | 200,000 | 500 | 10 TB |

### Headers de Réponse

```http
X-RateLimit-Limit: 300
X-RateLimit-Remaining: 245
X-RateLimit-Reset: 1705234800
X-RateLimit-Retry-After: 60
```

### Gestion dans le Code

```javascript
// Détection automatique de rate limiting
client.on('rateLimited', (info) => {
  console.log(`Rate limited. Retry after: ${info.retryAfter}s`);
  // Le SDK attend automatiquement avant de réessayer
});

// Configuration du retry
const client = new ArchiveChainClient({
  apiKey: 'your-key',
  retryConfig: {
    maxRetries: 3,
    backoffFactor: 2,
    respectRateLimit: true
  }
});
```

## Exemples Pratiques

### 1. Archive Automatique de News

```javascript
// Surveillance automatique de flux RSS
class NewsArchiver {
  constructor(apiKey, feedUrl) {
    this.client = new ArchiveChainClient({ apiKey });
    this.feedUrl = feedUrl;
  }

  async startMonitoring() {
    setInterval(async () => {
      try {
        const feed = await this.fetchRSSFeed(this.feedUrl);
        for (const item of feed.items) {
          if (this.isBreakingNews(item)) {
            await this.archiveArticle(item);
          }
        }
      } catch (error) {
        console.error('Erreur monitoring:', error);
      }
    }, 5 * 60 * 1000); // Vérifier toutes les 5 minutes
  }

  async archiveArticle(item) {
    const archive = await this.client.archives.create({
      url: item.link,
      metadata: {
        title: item.title,
        description: item.description,
        tags: ['news', 'breaking', 'auto-archived'],
        priority: 'high'
      },
      options: {
        includeAssets: true,
        maxDepth: 2
      }
    });

    console.log(`Article archivé: ${item.title} (${archive.id})`);
    return archive;
  }

  isBreakingNews(item) {
    const breakingKeywords = ['breaking', 'urgent', 'alert'];
    return breakingKeywords.some(keyword => 
      item.title.toLowerCase().includes(keyword)
    );
  }
}

// Utilisation
const archiver = new NewsArchiver('your-api-key', 'https://feeds.reuters.com/reuters/breakingviews');
archiver.startMonitoring();
```

### 2. Archive de Site Web Complet

```python
import requests
from urllib.parse import urljoin, urlparse
from archivechain import ArchiveChainClient

class WebsiteCrawler:
    def __init__(self, api_key, start_url, max_depth=3):
        self.client = ArchiveChainClient(api_key=api_key)
        self.start_url = start_url
        self.max_depth = max_depth
        self.visited = set()
        self.domain = urlparse(start_url).netloc

    async def archive_website(self):
        """Archive un site web complet avec tous ses liens internes"""
        urls_to_archive = await self.discover_urls()
        
        # Créer un bounty pour inciter l'archivage
        bounty = await self.client.bounties.create(
            title=f"Archive complète de {self.domain}",
            description=f"Archivage de toutes les pages de {self.domain}",
            reward_per_archive=50,  # 50 ARC par page
            urls=urls_to_archive,
            deadline="2024-02-01T00:00:00Z"
        )
        
        print(f"Bounty créé: {bounty.id}")
        
        # Surveiller le progrès
        async with self.client.stream.bounty_updates(bounty.id) as stream:
            async for update in stream:
                print(f"Progrès: {update.completed}/{update.total} pages archivées")
                if update.status == 'completed':
                    break

    async def discover_urls(self):
        """Découvre toutes les URLs du site"""
        urls = []
        to_visit = [(self.start_url, 0)]
        
        while to_visit:
            url, depth = to_visit.pop(0)
            
            if url in self.visited or depth > self.max_depth:
                continue
                
            self.visited.add(url)
            urls.append(url)
            
            if depth < self.max_depth:
                # Extraire les liens de la page
                try:
                    response = requests.get(url, timeout=10)
                    links = self.extract_links(response.text, url)
                    for link in links:
                        if self.is_internal_link(link):
                            to_visit.append((link, depth + 1))
                except:
                    continue
        
        return urls

# Utilisation
crawler = WebsiteCrawler(
    api_key='your-api-key',
    start_url='https://example.com',
    max_depth=3
)
await crawler.archive_website()
```

### 3. Dashboard de Monitoring

```javascript
// Dashboard temps réel avec WebSocket
class ArchiveChainDashboard {
  constructor(apiKey) {
    this.client = new ArchiveChainClient({ apiKey });
    this.ws = null;
    this.stats = {};
  }

  async init() {
    // Connexion WebSocket pour les updates temps réel
    this.ws = this.client.createWebSocket();
    
    // S'abonner aux statistiques réseau
    this.ws.subscribe('network_stats', { interval: 30 });
    
    // S'abonner aux nouvelles archives
    this.ws.subscribe('new_archives');
    
    // Gestion des messages
    this.ws.on('message', this.handleMessage.bind(this));
    
    // Charger les données initiales
    await this.loadInitialData();
    
    // Mettre à jour l'interface
    this.updateUI();
  }

  async loadInitialData() {
    // Statistiques générales
    this.stats = await this.client.network.getStats();
    
    // Mes archives récentes
    this.myArchives = await this.client.archives.list({
      limit: 10,
      sort: 'created_at:desc'
    });
    
    // Archives tendances
    this.trendingArchives = await this.client.search.trending({
      period: '24h',
      limit: 10
    });
  }

  handleMessage(message) {
    switch (message.type) {
      case 'network_stats':
        this.stats = message.data;
        this.updateStatsDisplay();
        break;
        
      case 'new_archive':
        this.myArchives.unshift(message.data);
        this.updateArchivesList();
        break;
        
      case 'archive_completed':
        this.updateArchiveStatus(message.archive_id, 'completed');
        this.showNotification(`Archive ${message.archive_id} terminée!`);
        break;
    }
  }

  updateStatsDisplay() {
    document.getElementById('total-nodes').textContent = this.stats.network.total_nodes;
    document.getElementById('active-nodes').textContent = this.stats.network.active_nodes;
    document.getElementById('total-archives').textContent = this.stats.archives.total_archives;
    document.getElementById('network-storage').textContent = this.stats.network.total_storage;
    
    // Mettre à jour les graphiques
    this.updateCharts();
  }

  updateCharts() {
    // Graphique de répartition des nœuds
    const nodeChart = new Chart(document.getElementById('nodeChart'), {
      type: 'doughnut',
      data: {
        labels: ['Full Archive', 'Light Storage', 'Relay', 'Gateway'],
        datasets: [{
          data: [
            this.stats.network.node_types.full_archive,
            this.stats.network.node_types.light_storage,
            this.stats.network.node_types.relay,
            this.stats.network.node_types.gateway
          ],
          backgroundColor: ['#3498db', '#e74c3c', '#f39c12', '#2ecc71']
        }]
      }
    });
  }
}

// Initialisation
const dashboard = new ArchiveChainDashboard('your-api-key');
dashboard.init();
```

## Troubleshooting

### Problèmes Courants

#### 1. Erreurs d'Authentification
```bash
# Problème: 401 Unauthorized
curl -H "Authorization: Bearer invalid-token" \
  https://api.archivechain.org/v1/archives

# Solution: Vérifier la validité du token
archivechain-cli auth verify-token --token your-token

# Régénérer si nécessaire
archivechain-cli auth create-key --name "New API Key"
```

#### 2. Rate Limiting
```javascript
// Problème: 429 Too Many Requests
// Solution: Implémenter retry avec backoff
const client = new ArchiveChainClient({
  apiKey: 'your-key',
  retryConfig: {
    maxRetries: 5,
    backoffFactor: 2,
    jitter: true
  }
});

// Alternative: Utiliser un pool de clés API
const keyPool = ['key1', 'key2', 'key3'];
let currentKeyIndex = 0;

function getNextClient() {
  const key = keyPool[currentKeyIndex];
  currentKeyIndex = (currentKeyIndex + 1) % keyPool.length;
  return new ArchiveChainClient({ apiKey: key });
}
```

#### 3. Timeouts et Latence
```python
# Configuration pour améliorer les performances
client = ArchiveChainClient(
    api_key='your-key',
    timeout=60.0,  # Augmenter le timeout
    connection_pool_size=20,  # Plus de connexions simultanées
    retry_config=RetryConfig(
        max_retries=3,
        backoff_factor=1.5
    )
)

# Utiliser la compression pour réduire la bande passante
client.set_compression(True)
```

### Outils de Debug

#### 1. Logs Détaillés
```javascript
// Activer les logs debug
const client = new ArchiveChainClient({
  apiKey: 'your-key',
  debug: true,
  logger: console // ou winston/bunyan
});

// Intercepter toutes les requêtes
client.interceptors.request.use(request => {
  console.log('Request:', request);
  return request;
});

client.interceptors.response.use(response => {
  console.log('Response:', response);
  return response;
});
```

#### 2. Health Checks
```bash
# Vérifier la santé de l'API
curl https://api.archivechain.org/v1/health

# Tester la connectivité WebSocket
wscat -c wss://api.archivechain.org/v1/ws

# Ping des services gRPC
grpcurl api.archivechain.org:9091 list
```

#### 3. Monitoring des Performances
```javascript
// Mesurer les temps de réponse
const client = new ArchiveChainClient({ apiKey: 'your-key' });

client.interceptors.request.use(request => {
  request.startTime = Date.now();
  return request;
});

client.interceptors.response.use(response => {
  const duration = Date.now() - response.config.startTime;
  console.log(`${response.config.method.toUpperCase()} ${response.config.url}: ${duration}ms`);
  return response;
});
```

### Support et Ressources

- 📚 [Documentation API complète](https://docs.archivechain.org/api)
- 🎮 [Playground interactif](https://playground.archivechain.org)
- 💬 [Discord #api-support](https://discord.gg/archivechain-api)
- 📧 [api-support@archivechain.org](mailto:api-support@archivechain.org)
- 🐛 [Issues GitHub](https://github.com/archivechain/archivechain/issues)

---

*Dernière mise à jour: 23 juillet 2025*