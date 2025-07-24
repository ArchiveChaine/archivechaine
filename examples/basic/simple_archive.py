#!/usr/bin/env python3
"""
Exemple Simple d'Archivage ArchiveChain
======================================

Cet exemple montre comment archiver une URL simple avec ArchiveChain.
Parfait pour débuter avec l'API ArchiveChain.

Prérequis:
    pip install archivechain-sdk python-dotenv

Usage:
    python simple_archive.py https://example.com
"""

import asyncio
import os
import sys
from dotenv import load_dotenv
from archivechain import ArchiveChainClient, ArchiveMetadata, ArchiveOptions

# Charger les variables d'environnement
load_dotenv()

async def archive_url(url: str) -> None:
    """Archive une URL avec métadonnées de base."""
    
    # Configuration du client ArchiveChain
    client = ArchiveChainClient(
        api_key=os.getenv('ARCHIVECHAIN_API_KEY'),
        api_url=os.getenv('ARCHIVECHAIN_API_URL', 'https://api.archivechain.org/v1'),
        network=os.getenv('ARCHIVECHAIN_NETWORK', 'mainnet')
    )
    
    try:
        print(f"🔗 Archivage de: {url}")
        
        # Définir les métadonnées de l'archive
        metadata = ArchiveMetadata(
            title=f"Archive de {url}",
            description=f"Archive automatique créée le {asyncio.get_event_loop().time()}",
            tags=["example", "simple", "python"],
            priority="normal"
        )
        
        # Options d'archivage
        options = ArchiveOptions(
            include_assets=True,      # Inclure CSS, JS, images
            max_depth=2,              # Profondeur maximale de crawling
            preserve_javascript=False, # Ne pas préserver le JavaScript
            timeout=30                # Timeout en secondes
        )
        
        # Créer l'archive
        print("📦 Création de l'archive en cours...")
        archive_result = await client.archives.create(
            url=url,
            metadata=metadata,
            options=options
        )
        
        print(f"✅ Archive créée avec succès!")
        print(f"   ID: {archive_result.archive_id}")
        print(f"   Statut: {archive_result.status}")
        print(f"   Coût estimé: {archive_result.cost_estimation.total_cost} ARC")
        
        # Surveiller le progrès de l'archivage
        print("\n📊 Surveillance du progrès...")
        async with client.stream.archive_updates(archive_result.archive_id) as stream:
            async for update in stream:
                if update.status == "processing":
                    print(f"   Progrès: {update.progress}% - {update.phase}")
                elif update.status == "completed":
                    print(f"🎉 Archive terminée!")
                    print(f"   Taille finale: {update.final_size} bytes")
                    print(f"   URL de visualisation: {update.access_urls.view}")
                    break
                elif update.status == "failed":
                    print(f"❌ Échec de l'archivage: {update.error}")
                    break
    
    except Exception as e:
        print(f"❌ Erreur lors de l'archivage: {e}")
        sys.exit(1)
    
    finally:
        await client.close()

async def get_archive_info(archive_id: str) -> None:
    """Récupère les informations d'une archive existante."""
    
    client = ArchiveChainClient(
        api_key=os.getenv('ARCHIVECHAIN_API_KEY'),
        api_url=os.getenv('ARCHIVECHAIN_API_URL', 'https://api.archivechain.org/v1')
    )
    
    try:
        print(f"🔍 Récupération des informations pour l'archive: {archive_id}")
        
        archive = await client.archives.get(archive_id)
        
        print(f"\n📋 Informations de l'archive:")
        print(f"   URL originale: {archive.url}")
        print(f"   Titre: {archive.metadata.title}")
        print(f"   Statut: {archive.status}")
        print(f"   Taille: {archive.size} bytes")
        print(f"   Créée le: {archive.created_at}")
        print(f"   Répliques: {len(archive.replicas)}")
        print(f"   Score d'intégrité: {archive.storage_info.integrity_score}")
        print(f"   URL de visualisation: {archive.access_urls.view}")
        
    except Exception as e:
        print(f"❌ Erreur lors de la récupération: {e}")
    
    finally:
        await client.close()

def main():
    """Point d'entrée principal."""
    if len(sys.argv) < 2:
        print("Usage: python simple_archive.py <URL> [archive_id_to_check]")
        print("\nExemples:")
        print("  python simple_archive.py https://example.com")
        print("  python simple_archive.py info arc_1234567890abcdef")
        sys.exit(1)
    
    # Vérifier la configuration
    api_key = os.getenv('ARCHIVECHAIN_API_KEY')
    if not api_key:
        print("❌ ARCHIVECHAIN_API_KEY non définie dans l'environnement")
        print("   Créez un fichier .env avec votre clé API:")
        print("   ARCHIVECHAIN_API_KEY=your-api-key-here")
        sys.exit(1)
    
    # Déterminer l'action à effectuer
    if sys.argv[1] == "info" and len(sys.argv) >= 3:
        # Afficher les informations d'une archive
        asyncio.run(get_archive_info(sys.argv[2]))
    else:
        # Archiver une URL
        url = sys.argv[1]
        
        # Validation basique de l'URL
        if not url.startswith(('http://', 'https://')):
            print("❌ URL invalide. Doit commencer par http:// ou https://")
            sys.exit(1)
        
        asyncio.run(archive_url(url))

if __name__ == "__main__":
    main()