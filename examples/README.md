# Exemples et Tutoriels ArchiveChain

Ce dossier contient des exemples pratiques et des tutoriels pour différents cas d'usage d'ArchiveChain.

## Structure des Exemples

```
examples/
├── basic/                     # Exemples de base
│   ├── simple_archive.py      # Archive simple d'une URL
│   ├── batch_archive.js       # Archive en lot
│   └── search_archives.rs     # Recherche d'archives
├── advanced/                  # Exemples avancés
│   ├── website_crawler/       # Crawler de site complet
│   ├── news_archiver/         # Archivage automatique de news
│   └── bounty_system/         # Système de bounties
├── integration/               # Intégrations avec d'autres systèmes
│   ├── wordpress_plugin/      # Plugin WordPress
│   ├── github_action/         # GitHub Action
│   └── zapier_webhook/        # Intégration Zapier
├── smart_contracts/           # Exemples de smart contracts
│   ├── archive_bounty/        # Contrat de bounty d'archivage
│   └── preservation_pool/     # Pool de préservation
└── infrastructure/            # Déploiement d'infrastructure
    ├── docker_compose/        # Déploiement Docker Compose
    ├── kubernetes/             # Déploiement Kubernetes
    └── terraform/              # Infrastructure as Code
```

## Exemples par Niveau

### 🟢 Débutant
- [Archive Simple](./basic/simple_archive.py) - Archiver une seule URL
- [Recherche d'Archives](./basic/search_archives.rs) - Rechercher des archives existantes
- [Configuration de Base](./infrastructure/docker_compose/) - Lancer un nœud local

### 🟡 Intermédiaire
- [Archive en Lot](./basic/batch_archive.js) - Archiver plusieurs URLs
- [Crawler de Site Web](./advanced/website_crawler/) - Archiver un site complet
- [Plugin WordPress](./integration/wordpress_plugin/) - Intégration CMS

### 🔴 Avancé
- [Archivage Automatique de News](./advanced/news_archiver/) - Bot d'archivage intelligent
- [Système de Bounties](./smart_contracts/archive_bounty/) - Smart contracts
- [Déploiement Kubernetes](./infrastructure/kubernetes/) - Infrastructure production

## Prérequis

- **Pour les exemples Python** : Python 3.8+ et `pip install archivechain-sdk`
- **Pour les exemples JavaScript** : Node.js 16+ et `npm install @archivechain/sdk`
- **Pour les exemples Rust** : Rust 1.70+ et `cargo add archivechain-sdk`
- **Pour l'infrastructure** : Docker, Kubernetes, ou Terraform selon l'exemple

## Configuration Commune

Tous les exemples utilisent les variables d'environnement suivantes :

```bash
# API Configuration
export ARCHIVECHAIN_API_KEY="your-api-key-here"
export ARCHIVECHAIN_API_URL="https://api.archivechain.org/v1"
export ARCHIVECHAIN_NETWORK="mainnet"  # ou "testnet" pour les tests

# Optional: Advanced configuration
export ARCHIVECHAIN_TIMEOUT="30"
export ARCHIVECHAIN_RETRY_ATTEMPTS="3"
export ARCHIVECHAIN_LOG_LEVEL="info"
```

## Obtenir une Clé API

1. Créez un compte sur [ArchiveChain Dashboard](https://dashboard.archivechain.org)
2. Allez dans "API Keys" et créez une nouvelle clé
3. Configurez les permissions appropriées pour vos besoins
4. Copiez la clé dans votre fichier `.env`

## Support et Communauté

- 📚 [Documentation](https://docs.archivechain.org)
- 💬 [Discord](https://discord.gg/archivechain)
- 🐛 [Issues GitHub](https://github.com/archivechain/archivechain/issues)
- 📧 [Support](mailto:support@archivechain.org)

## Contribution

Pour contribuer avec de nouveaux exemples :

1. Créez un dossier approprié selon la structure
2. Incluez un `README.md` expliquant l'exemple
3. Ajoutez des commentaires détaillés dans le code
4. Testez votre exemple sur testnet
5. Créez une Pull Request

## Licence

Tous les exemples sont sous licence MIT, sauf indication contraire.