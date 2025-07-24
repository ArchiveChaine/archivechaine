/**
 * Exemple d'Archivage en Lot ArchiveChain
 * =======================================
 * 
 * Cet exemple montre comment archiver plusieurs URLs en une seule opération,
 * avec gestion des erreurs et surveillance du progrès.
 * 
 * Prérequis:
 *   npm install @archivechain/sdk dotenv
 * 
 * Usage:
 *   node batch_archive.js urls.txt
 *   node batch_archive.js "https://example.com,https://github.com,https://stackoverflow.com"
 */

const { ArchiveChainClient, ArchiveMetadata, ArchiveOptions } = require('@archivechain/sdk');
const fs = require('fs').promises;
const path = require('path');
require('dotenv').config();

class BatchArchiver {
    constructor() {
        this.client = new ArchiveChainClient({
            apiKey: process.env.ARCHIVECHAIN_API_KEY,
            apiUrl: process.env.ARCHIVECHAIN_API_URL || 'https://api.archivechain.org/v1',
            network: process.env.ARCHIVECHAIN_NETWORK || 'mainnet',
            timeout: 30000,
            retries: 3
        });
    }

    /**
     * Archive une liste d'URLs en lot
     * @param {string[]} urls - Liste des URLs à archiver
     * @param {Object} options - Options d'archivage
     */
    async archiveBatch(urls, options = {}) {
        try {
            console.log(`🚀 Démarrage de l'archivage en lot pour ${urls.length} URLs`);
            
            // Valider les URLs
            const validUrls = this.validateUrls(urls);
            if (validUrls.length === 0) {
                throw new Error('Aucune URL valide trouvée');
            }

            // Créer la requête de lot
            const batchRequest = {
                urls: validUrls,
                metadata: new ArchiveMetadata({
                    title: `Archive en lot - ${new Date().toISOString()}`,
                    description: `Archivage automatique de ${validUrls.length} URLs`,
                    tags: ['batch', 'javascript', 'automated'],
                    priority: options.priority || 'normal'
                }),
                options: new ArchiveOptions({
                    include_assets: options.includeAssets !== false,
                    max_depth: options.maxDepth || 2,
                    preserve_javascript: options.preserveJavascript || false,
                    timeout: options.timeout || 30,
                    parallel_downloads: options.parallelDownloads || 5
                })
            };

            // Soumettre la requête de lot
            console.log('📦 Soumission de la requête de lot...');
            const batchResult = await this.client.archives.createBatch(batchRequest);
            
            console.log(`✅ Lot créé avec succès!`);
            console.log(`   ID du lot: ${batchResult.batch_id}`);
            console.log(`   URLs acceptées: ${batchResult.accepted_count}/${validUrls.length}`);
            console.log(`   Coût estimé total: ${batchResult.total_cost_estimation} ARC`);

            // Surveiller le progrès
            await this.monitorBatchProgress(batchResult.batch_id);

        } catch (error) {
            console.error('❌ Erreur lors de l\'archivage en lot:', error.message);
            throw error;
        }
    }

    /**
     * Surveille le progrès d'un lot d'archivage
     * @param {string} batchId - ID du lot à surveiller
     */
    async monitorBatchProgress(batchId) {
        console.log('\n📊 Surveillance du progrès du lot...');
        
        const progressStream = this.client.stream.batchProgress(batchId);
        const startTime = Date.now();
        
        try {
            for await (const progress of progressStream) {
                const elapsed = Math.round((Date.now() - startTime) / 1000);
                
                switch (progress.status) {
                    case 'processing':
                        const percentage = Math.round((progress.completed / progress.total) * 100);
                        process.stdout.write(`\r   Progrès: ${percentage}% (${progress.completed}/${progress.total}) - ${elapsed}s`);
                        break;
                        
                    case 'completed':
                        console.log(`\n🎉 Lot terminé avec succès!`);
                        await this.displayBatchResults(progress.results);
                        return;
                        
                    case 'partial_failure':
                        console.log(`\n⚠️  Lot partiellement réussi`);
                        await this.displayBatchResults(progress.results);
                        return;
                        
                    case 'failed':
                        console.log(`\n❌ Échec du lot: ${progress.error}`);
                        return;
                }
            }
        } catch (error) {
            console.error('\n❌ Erreur lors de la surveillance:', error.message);
        }
    }

    /**
     * Affiche les résultats détaillés du lot
     * @param {Object[]} results - Résultats de chaque archive
     */
    async displayBatchResults(results) {
        console.log('\n📋 Résultats détaillés:');
        
        const successful = results.filter(r => r.status === 'completed');
        const failed = results.filter(r => r.status === 'failed');
        
        console.log(`   ✅ Réussis: ${successful.length}`);
        console.log(`   ❌ Échoués: ${failed.length}`);
        
        if (successful.length > 0) {
            console.log('\n✅ Archives réussies:');
            successful.forEach(result => {
                console.log(`   • ${result.url}`);
                console.log(`     ID: ${result.archive_id}`);
                console.log(`     Taille: ${this.formatBytes(result.size)}`);
                console.log(`     Voir: ${result.access_urls.view}`);
                console.log('');
            });
        }
        
        if (failed.length > 0) {
            console.log('❌ Échecs:');
            failed.forEach(result => {
                console.log(`   • ${result.url}: ${result.error}`);
            });
        }

        // Calculer les statistiques
        const totalSize = successful.reduce((sum, r) => sum + r.size, 0);
        const totalCost = successful.reduce((sum, r) => sum + r.cost, 0);
        
        console.log('\n📊 Statistiques:');
        console.log(`   Taille totale archivée: ${this.formatBytes(totalSize)}`);
        console.log(`   Coût total: ${totalCost} ARC`);
        console.log(`   Taux de réussite: ${Math.round((successful.length / results.length) * 100)}%`);
    }

    /**
     * Valide une liste d'URLs
     * @param {string[]} urls - URLs à valider
     * @returns {string[]} URLs valides
     */
    validateUrls(urls) {
        const urlRegex = /^https?:\/\/.+/;
        const validUrls = [];
        
        urls.forEach(url => {
            const cleanUrl = url.trim();
            if (urlRegex.test(cleanUrl)) {
                validUrls.push(cleanUrl);
            } else {
                console.warn(`⚠️  URL invalide ignorée: ${cleanUrl}`);
            }
        });
        
        return validUrls;
    }

    /**
     * Formate une taille en bytes en format lisible
     * @param {number} bytes - Taille en bytes
     * @returns {string} Taille formatée
     */
    formatBytes(bytes) {
        if (bytes === 0) return '0 B';
        const k = 1024;
        const sizes = ['B', 'KB', 'MB', 'GB'];
        const i = Math.floor(Math.log(bytes) / Math.log(k));
        return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
    }

    /**
     * Lit les URLs depuis un fichier
     * @param {string} filePath - Chemin vers le fichier
     * @returns {string[]} Liste des URLs
     */
    async readUrlsFromFile(filePath) {
        try {
            const content = await fs.readFile(filePath, 'utf8');
            return content
                .split('\n')
                .map(line => line.trim())
                .filter(line => line.length > 0 && !line.startsWith('#'));
        } catch (error) {
            throw new Error(`Impossible de lire le fichier ${filePath}: ${error.message}`);
        }
    }

    /**
     * Ferme proprement le client
     */
    async close() {
        await this.client.close();
    }
}

/**
 * Fonction principale
 */
async function main() {
    // Vérifier les arguments
    if (process.argv.length < 3) {
        console.log('Usage: node batch_archive.js <urls_file|urls_list>');
        console.log('\nExemples:');
        console.log('  node batch_archive.js urls.txt');
        console.log('  node batch_archive.js "https://example.com,https://github.com"');
        process.exit(1);
    }

    // Vérifier la configuration
    const apiKey = process.env.ARCHIVECHAIN_API_KEY;
    if (!apiKey) {
        console.error('❌ ARCHIVECHAIN_API_KEY non définie dans l\'environnement');
        console.error('   Créez un fichier .env avec votre clé API:');
        console.error('   ARCHIVECHAIN_API_KEY=your-api-key-here');
        process.exit(1);
    }

    const archiver = new BatchArchiver();
    
    try {
        const input = process.argv[2];
        let urls = [];

        // Déterminer si l'input est un fichier ou une liste d'URLs
        if (input.includes(',')) {
            // Liste d'URLs séparées par des virgules
            urls = input.split(',');
        } else if (await fs.access(input).then(() => true).catch(() => false)) {
            // Fichier existant
            urls = await archiver.readUrlsFromFile(input);
        } else {
            // URL unique
            urls = [input];
        }

        if (urls.length === 0) {
            console.error('❌ Aucune URL trouvée à archiver');
            process.exit(1);
        }

        // Options d'archivage (peuvent être personnalisées)
        const options = {
            includeAssets: true,
            maxDepth: 2,
            preserveJavascript: false,
            priority: 'normal',
            parallelDownloads: 5
        };

        await archiver.archiveBatch(urls, options);
        
    } catch (error) {
        console.error('❌ Erreur fatale:', error.message);
        process.exit(1);
    } finally {
        await archiver.close();
    }
}

// Gestion des signaux pour fermeture propre
process.on('SIGINT', async () => {
    console.log('\n🛑 Interruption reçue, fermeture en cours...');
    process.exit(0);
});

process.on('SIGTERM', async () => {
    console.log('\n🛑 Arrêt demandé, fermeture en cours...');
    process.exit(0);
});

// Exécuter le programme principal
if (require.main === module) {
    main().catch(error => {
        console.error('❌ Erreur non gérée:', error);
        process.exit(1);
    });
}

module.exports = { BatchArchiver };