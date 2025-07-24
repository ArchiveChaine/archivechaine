# Spécifications des APIs et Protocoles ArchiveChain

## Vue d'ensemble des APIs

ArchiveChain expose plusieurs interfaces API pour différents cas d'usage :

1. **REST API** - Interface HTTP standard pour les intégrations tierces
2. **GraphQL API** - Requêtes flexibles et optimisées
3. **WebSocket API** - Communication temps réel et streaming
4. **gRPC API** - Communication haute performance inter-services
5. **P2P Protocol** - Protocoles de communication entre nœuds

## 1. REST API

### Base URL et Versioning
```
Base URL: https://api.archivechain.org/v1
Versioning: Path-based (/v1/, /v2/, etc.)
Content-Type: application/json
Authentication: Bearer token (JWT)
```

### Endpoints Principaux

#### Archives Management
```http
# Soumettre une nouvelle archive
POST /archives
Content-Type: application/json
Authorization: Bearer {token}

{
  "url": "https://example.com/page.html",
  "metadata": {
    "title": "Example Page",
    "description": "Description of the page",
    "tags": ["web", "example"],
    "priority": "high"
  },
  "options": {
    "include_assets": true,
    "max_depth": 3,
    "preserve_javascript": false
  }
}

Response 201:
{
  "archive_id": "arc_1234567890abcdef",
  "status": "pending",
  "estimated_completion": "2024-01-15T10:30:00Z",
  "cost_estimation": {
    "storage_cost": "0.001 ARC",
    "processing_cost": "0.0005 ARC"
  }
}
```

```http
# Récupérer une archive
GET /archives/{archive_id}
Authorization: Bearer {token}

Response 200:
{
  "archive_id": "arc_1234567890abcdef",
  "url": "https://example.com/page.html",
  "status": "completed",
  "created_at": "2024-01-15T10:00:00Z",
  "completed_at": "2024-01-15T10:30:00Z",
  "size": 2048576,
  "metadata": {
    "title": "Example Page",
    "description": "Description of the page",
    "mime_type": "text/html",
    "language": "en"
  },
  "storage_info": {
    "replicas": 5,
    "locations": ["us-east", "eu-west", "asia-pacific"],
    "integrity_score": 0.99
  },
  "access_urls": {
    "view": "https://gateway.archivechain.org/view/arc_1234567890abcdef",
    "download": "https://gateway.archivechain.org/download/arc_1234567890abcdef",
    "raw": "https://gateway.archivechain.org/raw/arc_1234567890abcdef"
  }
}
```

```http
# Lister les archives
GET /archives?page=1&limit=50&status=completed&tag=web
Authorization: Bearer {token}

Response 200:
{
  "archives": [
    {
      "archive_id": "arc_1234567890abcdef",
      "url": "https://example.com/page.html",
      "status": "completed",
      "created_at": "2024-01-15T10:00:00Z",
      "size": 2048576
    }
  ],
  "pagination": {
    "page": 1,
    "limit": 50,
    "total": 1250,
    "has_next": true
  }
}
```

#### Search API
```http
# Recherche d'archives
GET /search?q=example&type=url&limit=20
Authorization: Bearer {token}

Response 200:
{
  "query": "example",
  "results": [
    {
      "archive_id": "arc_1234567890abcdef",
      "url": "https://example.com/page.html",
      "title": "Example Page",
      "snippet": "This is an example page showing...",
      "relevance_score": 0.95,
      "archived_at": "2024-01-15T10:00:00Z"
    }
  ],
  "facets": {
    "domains": {
      "example.com": 15,
      "test.org": 8
    },
    "content_types": {
      "text/html": 20,
      "application/pdf": 3
    }
  },
  "total_results": 1250,
  "search_time_ms": 45
}
```

#### Network Statistics
```http
# Statistiques du réseau
GET /network/stats
Authorization: Bearer {token}

Response 200:
{
  "network": {
    "total_nodes": 1247,
    "active_nodes": 1198,
    "total_storage": "15.7 TB",
    "available_storage": "8.3 TB",
    "current_block_height": 245671
  },
  "archives": {
    "total_archives": 567890,
    "archives_today": 1234,
    "total_size": "12.4 TB",
    "average_replication": 4.2
  },
  "performance": {
    "average_archive_time": "2.3 minutes",
    "network_latency": "45ms",
    "success_rate": 0.987
  }
}
```

### Codes d'erreur HTTP
```http
200 OK - Succès
201 Created - Ressource créée
400 Bad Request - Paramètres invalides
401 Unauthorized - Authentification requise
403 Forbidden - Permissions insuffisantes
404 Not Found - Ressource non trouvée
409 Conflict - Conflit (archive déjà existante)
422 Unprocessable Entity - Validation échouée
429 Too Many Requests - Limite de taux atteinte
500 Internal Server Error - Erreur serveur
503 Service Unavailable - Service temporairement indisponible
```

## 2. GraphQL API

### Schema Principal
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
  
  # Search
  searchArchives(
    query: String!
    filters: SearchFilters
    first: Int
    after: String
  ): SearchConnection!
  
  # Network
  networkStats: NetworkStats!
  nodes(status: NodeStatus): [Node!]!
  
  # User
  me: User!
  myUsage: UsageStats!
}

type Mutation {
  # Archives
  createArchive(input: CreateArchiveInput!): CreateArchivePayload!
  updateArchive(id: ID!, input: UpdateArchiveInput!): UpdateArchivePayload!
  deleteArchive(id: ID!): DeleteArchivePayload!
  
  # User
  updateProfile(input: UpdateProfileInput!): UpdateProfilePayload!
}

type Subscription {
  # Real-time updates
  archiveStatusUpdated(archiveId: ID!): Archive!
  newArchiveCreated: Archive!
  networkStatsUpdated: NetworkStats!
}

type Archive {
  id: ID!
  url: String!
  status: ArchiveStatus!
  metadata: ArchiveMetadata!
  storageInfo: StorageInfo!
  createdAt: DateTime!
  completedAt: DateTime
  size: Int!
  cost: TokenAmount!
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
}

type StorageInfo {
  replicas: Int!
  locations: [String!]!
  integrityScore: Float!
  lastVerified: DateTime!
}
```

### Requêtes Exemples
```graphql
# Récupérer des archives avec métadonnées
query GetArchives($first: Int!, $filter: ArchiveFilter) {
  archives(first: $first, filter: $filter) {
    edges {
      node {
        id
        url
        status
        metadata {
          title
          tags
          contentType
        }
        storageInfo {
          replicas
          integrityScore
        }
        createdAt
      }
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}

# Recherche avec facettes
query SearchWithFacets($query: String!, $filters: SearchFilters) {
  searchArchives(query: $query, filters: $filters) {
    edges {
      node {
        id
        url
        metadata {
          title
          snippet
        }
        relevanceScore
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
    }
  }
}

# Souscription aux mises à jour
subscription ArchiveUpdates($archiveId: ID!) {
  archiveStatusUpdated(archiveId: $archiveId) {
    id
    status
    metadata {
      title
    }
    storageInfo {
      replicas
      integrityScore
    }
  }
}
```

## 3. WebSocket API

### Connection et Authentification
```javascript
// Connexion WebSocket
const ws = new WebSocket('wss://api.archivechain.org/v1/ws');

// Authentification
ws.send(JSON.stringify({
  type: 'auth',
  token: 'eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...'
}));
```

### Types de Messages
```javascript
// Subscribe aux mises à jour d'archives
{
  "type": "subscribe",
  "channel": "archive_updates",
  "archive_id": "arc_1234567890abcdef"
}

// Subscribe aux statistiques réseau
{
  "type": "subscribe",
  "channel": "network_stats",
  "interval": 30 // secondes
}

// Subscribe aux nouveaux blocs
{
  "type": "subscribe",
  "channel": "new_blocks"
}

// Unsubscribe
{
  "type": "unsubscribe",
  "channel": "archive_updates",
  "archive_id": "arc_1234567890abcdef"
}
```

### Messages de Réponse
```javascript
// Mise à jour statut archive
{
  "type": "archive_update",
  "archive_id": "arc_1234567890abcdef",
  "status": "completed",
  "progress": 100,
  "data": {
    "size": 2048576,
    "replicas": 5,
    "integrity_score": 0.99
  },
  "timestamp": "2024-01-15T10:30:00Z"
}

// Statistiques réseau
{
  "type": "network_stats",
  "data": {
    "active_nodes": 1198,
    "total_storage": "15.7 TB",
    "current_block_height": 245671,
    "network_latency": "45ms"
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
    "archives": 12
  }
}
```

## 4. gRPC API

### Service Definitions
```protobuf
syntax = "proto3";
package archivechain.v1;

// Service principal d'archivage
service ArchiveService {
  // Archives
  rpc CreateArchive(CreateArchiveRequest) returns (CreateArchiveResponse);
  rpc GetArchive(GetArchiveRequest) returns (Archive);
  rpc ListArchives(ListArchivesRequest) returns (ListArchivesResponse);
  rpc SearchArchives(SearchRequest) returns (SearchResponse);
  
  // Streaming
  rpc StreamArchiveUpdates(StreamRequest) returns (stream ArchiveUpdate);
  rpc StreamNetworkStats(StreamRequest) returns (stream NetworkStats);
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
}

enum ArchiveStatus {
  ARCHIVE_STATUS_UNSPECIFIED = 0;
  ARCHIVE_STATUS_PENDING = 1;
  ARCHIVE_STATUS_PROCESSING = 2;
  ARCHIVE_STATUS_COMPLETED = 3;
  ARCHIVE_STATUS_FAILED = 4;
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
}
```

## 5. P2P Protocol Specifications

### Protocol Stack
```
Application Layer: ArchiveChain Protocol
Transport Layer: QUIC / TCP
Network Layer: libp2p
Discovery: mDNS + DHT (Kademlia)
```

### Message Types
```rust
// Messages de consensus
pub enum ConsensusMessage {
    BlockProposal(Block),
    BlockVote(BlockVote),
    ProofOfArchive(ArchiveProof),
    ValidatorUpdate(ValidatorInfo),
}

// Messages de synchronisation
pub enum SyncMessage {
    BlockRequest(BlockRange),
    BlockResponse(Vec<Block>),
    StateRequest(StateKey),
    StateResponse(StateValue),
}

// Messages de réseau
pub enum NetworkMessage {
    Ping(PingMessage),
    Pong(PongMessage),
    PeerDiscovery(PeerInfo),
    ArchiveAnnouncement(ArchiveMetadata),
}
```

### Protocol Buffers Definitions
```protobuf
// P2P Network Protocol
message NetworkMessage {
  oneof message_type {
    PingMessage ping = 1;
    PongMessage pong = 2;
    BlockMessage block = 3;
    TransactionMessage transaction = 4;
    ArchiveMessage archive = 5;
  }
}

message BlockMessage {
  Block block = 1;
  bytes signature = 2;
  string validator_id = 3;
}

message ArchiveMessage {
  string archive_id = 1;
  string url = 2;
  bytes content_hash = 3;
  repeated string replica_locations = 4;
  int64 size = 5;
}
```

## 6. Authentification et Sécurité

### JWT Token Structure
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
    "node_id": "node_abcdef1234567890",
    "rate_limit": {
      "requests_per_hour": 1000,
      "storage_limit_gb": 100
    }
  }
}
```

### Rate Limiting
```yaml
# Configuration des limites par scope
rate_limits:
  public:
    requests_per_minute: 60
    requests_per_hour: 1000
    concurrent_requests: 10
  
  authenticated:
    requests_per_minute: 300
    requests_per_hour: 10000
    concurrent_requests: 50
    
  premium:
    requests_per_minute: 1000
    requests_per_hour: 50000
    concurrent_requests: 200
```

### API Keys et Scopes
```yaml
# Scopes disponibles
scopes:
  - archives:read      # Lecture des archives
  - archives:write     # Création/modification d'archives
  - archives:delete    # Suppression d'archives
  - search:read        # Recherche d'archives
  - network:read       # Statistiques du réseau
  - node:manage        # Gestion de nœud
  - admin:all          # Accès administrateur
```

## 7. Documentation OpenAPI

### Structure de la Documentation
```yaml
openapi: 3.0.3
info:
  title: ArchiveChain API
  description: API pour l'archivage web décentralisé
  version: 1.0.0
  contact:
    name: ArchiveChain Support
    url: https://archivechain.org/support
    email: api@archivechain.org
  license:
    name: MIT
    url: https://opensource.org/licenses/MIT

servers:
  - url: https://api.archivechain.org/v1
    description: Production
  - url: https://testnet-api.archivechain.org/v1
    description: Testnet

security:
  - BearerAuth: []

components:
  securitySchemes:
    BearerAuth:
      type: http
      scheme: bearer
      bearerFormat: JWT
```

## 8. SDKs et Clients

### SDK JavaScript/TypeScript
```typescript
import { ArchiveChainClient } from '@archivechain/sdk';

const client = new ArchiveChainClient({
  apiKey: 'your-api-key',
  baseUrl: 'https://api.archivechain.org/v1'
});

// Créer une archive
const archive = await client.archives.create({
  url: 'https://example.com',
  options: {
    includeAssets: true,
    maxDepth: 3
  }
});

// Rechercher des archives
const results = await client.search.query('example', {
  contentType: 'text/html',
  dateRange: {
    start: '2024-01-01',
    end: '2024-01-31'
  }
});

// WebSocket streaming
client.subscribe('archive_updates', archive.id, (update) => {
  console.log('Archive status:', update.status);
});
```

### SDK Python
```python
from archivechain import ArchiveChainClient

client = ArchiveChainClient(
    api_key='your-api-key',
    base_url='https://api.archivechain.org/v1'
)

# Créer une archive
archive = client.archives.create(
    url='https://example.com',
    options={
        'include_assets': True,
        'max_depth': 3
    }
)

# Rechercher avec pagination
for result in client.search.paginate('example'):
    print(f"Found: {result.url} - {result.title}")

# Async support
import asyncio

async def main():
    async with client.stream('archive_updates', archive.id) as stream:
        async for update in stream:
            print(f"Status: {update.status}")

asyncio.run(main())
```

### SDK Rust
```rust
use archivechain_sdk::{ArchiveChainClient, CreateArchiveOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = ArchiveChainClient::new("your-api-key")?;
    
    // Créer une archive
    let archive = client.archives().create("https://example.com")
        .with_options(CreateArchiveOptions {
            include_assets: true,
            max_depth: Some(3),
            ..Default::default()
        })
        .await?;
    
    // Stream des mises à jour
    let mut stream = client.stream().archive_updates(archive.id).await?;
    while let Some(update) = stream.next().await {
        println!("Archive status: {:?}", update.status);
    }
    
    Ok(())
}
```

## 9. Tests et Monitoring

### Health Checks
```http
GET /health
Response 200:
{
  "status": "healthy",
  "version": "1.0.0",
  "timestamp": "2024-01-15T10:30:00Z",
  "checks": {
    "database": "healthy",
    "blockchain": "healthy",
    "storage": "healthy",
    "network": "healthy"
  },
  "uptime": "7d 14h 23m"
}
```

### Métriques API
```http
GET /metrics
Content-Type: text/plain

# HELP api_requests_total Total number of API requests
# TYPE api_requests_total counter
api_requests_total{method="GET",endpoint="/archives",status="200"} 1234

# HELP api_request_duration_seconds API request duration
# TYPE api_request_duration_seconds histogram
api_request_duration_seconds_bucket{le="0.1"} 100
api_request_duration_seconds_bucket{le="0.5"} 1000