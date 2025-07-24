# Crawler de Site Web ArchiveChain

Ce dossier contient un exemple avancé de crawler qui peut archiver un site web complet avec toutes ses ressources.

## Fonctionnalités

- 🕸️ **Crawling intelligent** avec respect du robots.txt
- 📄 **Archive complète** : HTML, CSS, JS, images, documents
- 🔗 **Suivi des liens** avec contrôle de profondeur
- ⚡ **Parallélisme** configurable pour des performances optimales
- 🛡️ **Rate limiting** pour respecter les serveurs
- 📊 **Rapports détaillés** avec métriques et statistiques
- 🎯 **Filtrage avancé** par type de contenu et taille
- 🔄 **Retry automatique** avec backoff exponentiel
- 💾 **Sauvegarde progressive** des résultats

## Structure

```
website_crawler/
├── README.md              # Ce fichier
├── crawler.py             # Script principal Python
├── crawler.js             # Version JavaScript/Node.js
├── config.yaml           # Configuration exemple
├── requirements.txt       # Dépendances Python
├── package.json          # Dépendances Node.js
└── examples/             # Exemples de configuration
    ├── news_site.yaml
    ├── documentation.yaml
    └── e_commerce.yaml
```

## Installation

### Version Python

```bash
cd examples/advanced/website_crawler
pip install -r requirements.txt
```

### Version Node.js

```bash
cd examples/advanced/website_crawler
npm install
```

## Configuration

Créez un fichier `config.yaml` basé sur l'exemple :

```yaml
# Configuration du crawler
crawler:
  max_depth: 3
  max_pages: 1000
  concurrent_downloads: 5
  delay_between_requests: 1.0
  timeout: 30
  user_agent: "ArchiveChain-Crawler/1.0"

# Filtres de contenu
filters:
  allowed_domains: []  # Vide = domaine de départ uniquement
  allowed_extensions: [".html", ".php", ".pdf", ".doc", ".docx"]
  max_file_size: 50MB
  exclude_patterns:
    - "/admin/"
    - "/private/"
    - "*.zip"
    - "*.exe"

# Options d'archivage
archive:
  include_assets: true
  preserve_javascript: false
  priority: "normal"
  tags: ["crawler", "automated"]

# Configuration ArchiveChain
archivechain:
  api_key: "${ARCHIVECHAIN_API_KEY}"
  api_url: "https://api.archivechain.org/v1"
  network: "mainnet"
```

## Usage

### Python

```bash
# Crawler simple
python crawler.py https://example.com

# Avec configuration personnalisée
python crawler.py --config config.yaml https://example.com

# Mode dry-run (simulation)
python crawler.py --dry-run --verbose https://example.com

# Avec sauvegarde des résultats
python crawler.py --output results.json https://example.com
```

### JavaScript

```bash
# Crawler simple
node crawler.js https://example.com

# Avec options avancées
node crawler.js --max-depth 5 --concurrent 10 https://example.com

# Mode interactif
node crawler.js --interactive
```

## Exemples de Configuration

### Site d'Actualités

```yaml
# examples/news_site.yaml
crawler:
  max_depth: 2
  max_pages: 500
  concurrent_downloads: 3
  delay_between_requests: 2.0

filters:
  allowed_extensions: [".html", ".php"]
  exclude_patterns:
    - "/comments/"
    - "/user/"
    - "?page="
    - "&sort="

archive:
  tags: ["news", "journalism", "current-events"]
  priority: "high"
  metadata:
    category: "News Archive"
    description: "Archive automatique de site d'actualités"
```

### Documentation Technique

```yaml
# examples/documentation.yaml
crawler:
  max_depth: 5
  max_pages: 2000
  concurrent_downloads: 8
  follow_external_docs: true

filters:
  allowed_extensions: [".html", ".md", ".pdf", ".txt"]
  allowed_domains: ["docs.example.com", "api.example.com"]

archive:
  include_assets: true
  preserve_javascript: true
  tags: ["documentation", "technical", "reference"]
  metadata:
    category: "Technical Documentation"
    language: "en"
```

### Site E-commerce

```yaml
# examples/e_commerce.yaml
crawler:
  max_depth: 4
  max_pages: 10000
  concurrent_downloads: 5
  respect_robots_txt: true

filters:
  exclude_patterns:
    - "/cart/"
    - "/checkout/"
    - "/account/"
    - "/admin/"
    - "?add-to-cart="
  allowed_extensions: [".html", ".php", ".pdf"]

archive:
  tags: ["e-commerce", "products", "catalog"]
  metadata:
    category: "E-commerce Catalog"
    preserve_prices: true
```

## Métriques et Rapports

Le crawler génère des rapports détaillés incluant :

- 📊 **Statistiques de crawling** : pages trouvées, temps total, vitesse
- 🔗 **Analyse des liens** : liens internes/externes, liens brisés
- 💾 **Utilisation des ressources** : taille totale, types de fichiers
- ❌ **Erreurs rencontrées** : 404, timeouts, erreurs de parsing
- 💰 **Coûts d'archivage** : estimation des coûts en ARC

### Exemple de Rapport

```json
{
  "crawl_summary": {
    "start_time": "2025-07-24T04:00:00Z",
    "end_time": "2025-07-24T04:15:32Z",
    "duration": "15m32s",
    "pages_crawled": 1247,
    "pages_archived": 1189,
    "errors": 58,
    "success_rate": 95.3
  },
  "content_analysis": {
    "total_size": "2.4 GB",
    "average_page_size": "1.9 MB",
    "file_types": {
      "html": 892,
      "pdf": 156,
      "images": 2341,
      "css": 67,
      "js": 123
    }
  },
  "archive_results": {
    "successful_archives": 1189,
    "failed_archives": 58,
    "total_cost": "1247.50 ARC",
    "average_cost_per_page": "1.05 ARC"
  }
}
```

## Bonnes Pratiques

### Respecter les Serveurs
- Toujours respecter le `robots.txt`
- Utiliser des délais appropriés entre les requêtes
- Limiter le nombre de connexions concurrentes
- Utiliser un User-Agent descriptif

### Optimiser les Performances
- Filtrer intelligemment le contenu
- Utiliser la mise en cache pour éviter les doublons
- Paralléliser les téléchargements de manière contrôlée
- Surveiller l'utilisation de la bande passante

### Gestion des Erreurs
- Implémenter un système de retry avec backoff
- Logger toutes les erreurs pour analyse
- Gérer gracieusement les timeouts
- Prévoir des mécanismes de reprise

## Limitations

- **Rate Limiting** : Respecte les limites des serveurs cibles
- **Contenu Dynamique** : Le JavaScript n'est pas exécuté par défaut
- **Authentification** : Ne gère pas les sites protégés par mot de passe
- **Sites SPA** : Les Single Page Applications nécessitent une configuration spéciale

## Dépannage

### Erreurs Communes

**Erreur de connexion SSL**
```bash
# Solution : ignorer les certificats non valides (développement uniquement)
python crawler.py --ignore-ssl https://example.com
```

**Rate limiting trop agressif**
```yaml
# Augmenter les délais dans config.yaml
crawler:
  delay_between_requests: 5.0
  concurrent_downloads: 2
```

**Mémoire insuffisante**
```yaml
# Réduire la charge de travail
crawler:
  max_pages: 500
  concurrent_downloads: 2
```

## Sécurité

- Ne jamais exposer vos clés API dans la configuration
- Utiliser des variables d'environnement pour les secrets
- Respecter les conditions d'utilisation des sites cibles
- Éviter le crawling de sites sans autorisation

## Support

Pour obtenir de l'aide :
- 📚 [Documentation complète](https://docs.archivechain.org/crawler)
- 💬 [Discord #crawler-support](https://discord.gg/archivechain)
- 🐛 [Signaler un bug](https://github.com/archivechain/archivechain/issues)