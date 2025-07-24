//! # Exemple de Recherche d'Archives ArchiveChain
//! 
//! Cet exemple montre comment rechercher et filtrer des archives existantes
//! avec différents critères et options d'affichage.
//! 
//! ## Prérequis
//! 
//! Ajoutez à votre Cargo.toml :
//! ```toml
//! [dependencies]
//! archivechain-sdk = "1.0"
//! tokio = { version = "1.0", features = ["full"] }
//! clap = { version = "4.0", features = ["derive"] }
//! serde = { version = "1.0", features = ["derive"] }
//! chrono = "0.4"
//! dotenv = "0.15"
//! ```
//! 
//! ## Usage
//! 
//! ```bash
//! cargo run --bin search_archives -- --query "example.com"
//! cargo run --bin search_archives -- --tag "news" --limit 10
//! cargo run --bin search_archives -- --created-after "2024-01-01" --format json
//! ```

use archivechain_sdk::{
    ArchiveChainClient, SearchQuery, SearchFilters, SearchOptions, SearchResult,
    ArchiveStatus, Priority
};
use chrono::{DateTime, Utc, NaiveDate};
use clap::{Parser, ValueEnum};
use dotenv::dotenv;
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Terme de recherche (URL, titre, ou contenu)
    #[arg(short, long)]
    query: Option<String>,

    /// Filtrer par tag
    #[arg(short, long)]
    tag: Option<String>,

    /// Filtrer par créateur (adresse publique)
    #[arg(short, long)]
    creator: Option<String>,

    /// Filtrer par statut
    #[arg(short, long)]
    status: Option<ArchiveStatus>,

    /// Date de création minimale (YYYY-MM-DD)
    #[arg(long)]
    created_after: Option<String>,

    /// Date de création maximale (YYYY-MM-DD)
    #[arg(long)]
    created_before: Option<String>,

    /// Taille minimale en bytes
    #[arg(long)]
    min_size: Option<u64>,

    /// Taille maximale en bytes
    #[arg(long)]
    max_size: Option<u64>,

    /// Nombre maximum de résultats
    #[arg(short, long, default_value = "20")]
    limit: usize,

    /// Page de résultats (pour pagination)
    #[arg(short, long, default_value = "1")]
    page: usize,

    /// Format de sortie
    #[arg(short, long, default_value = "table")]
    format: OutputFormat,

    /// Inclure les métadonnées étendues
    #[arg(long)]
    extended: bool,

    /// Trier par
    #[arg(long, default_value = "created_at")]
    sort_by: SortField,

    /// Ordre de tri
    #[arg(long, default_value = "desc")]
    sort_order: SortOrder,
}

#[derive(ValueEnum, Clone, Debug)]
enum OutputFormat {
    Table,
    Json,
    Csv,
    Summary,
}

#[derive(ValueEnum, Clone, Debug)]
enum SortField {
    CreatedAt,
    Size,
    Title,
    Url,
    Status,
}

#[derive(ValueEnum, Clone, Debug)]
enum SortOrder {
    Asc,
    Desc,
}

#[derive(Debug, Serialize, Deserialize)]
struct ArchiveDisplay {
    id: String,
    url: String,
    title: Option<String>,
    status: String,
    size: u64,
    created_at: String,
    tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    extended_info: Option<ExtendedInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ExtendedInfo {
    creator: String,
    replicas: usize,
    integrity_score: f64,
    access_count: u64,
    last_accessed: Option<String>,
}

/// Client de recherche avec fonctionnalités avancées
struct ArchiveSearcher {
    client: ArchiveChainClient,
}

impl ArchiveSearcher {
    /// Crée une nouvelle instance du chercheur
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let api_key = env::var("ARCHIVECHAIN_API_KEY")
            .map_err(|_| "ARCHIVECHAIN_API_KEY non définie")?;
        
        let client = ArchiveChainClient::builder()
            .api_key(api_key)
            .api_url(env::var("ARCHIVECHAIN_API_URL")
                .unwrap_or_else(|_| "https://api.archivechain.org/v1".to_string()))
            .network(env::var("ARCHIVECHAIN_NETWORK")
                .unwrap_or_else(|_| "mainnet".to_string()))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .await?;

        Ok(Self { client })
    }

    /// Effectue une recherche avec les paramètres donnés
    pub async fn search(&self, args: &Args) -> Result<Vec<ArchiveDisplay>, Box<dyn Error>> {
        println!("🔍 Recherche d'archives...");

        // Construire la requête de recherche
        let mut query_builder = SearchQuery::builder();

        if let Some(ref query) = args.query {
            query_builder = query_builder.text(query);
        }

        // Construire les filtres
        let mut filters = SearchFilters::new();

        if let Some(ref tag) = args.tag {
            filters = filters.tag(tag);
        }

        if let Some(ref creator) = args.creator {
            filters = filters.creator(creator);
        }

        if let Some(status) = args.status {
            filters = filters.status(status);
        }

        if let Some(ref date_str) = args.created_after {
            let date = parse_date(date_str)?;
            filters = filters.created_after(date);
        }

        if let Some(ref date_str) = args.created_before {
            let date = parse_date(date_str)?;
            filters = filters.created_before(date);
        }

        if let Some(min_size) = args.min_size {
            filters = filters.min_size(min_size);
        }

        if let Some(max_size) = args.max_size {
            filters = filters.max_size(max_size);
        }

        // Options de recherche
        let options = SearchOptions::builder()
            .limit(args.limit)
            .offset((args.page - 1) * args.limit)
            .sort_by(convert_sort_field(&args.sort_by))
            .sort_order(convert_sort_order(&args.sort_order))
            .include_metadata(args.extended)
            .build();

        // Exécuter la recherche
        let search_result = self.client
            .search()
            .archives(query_builder.build())
            .filters(filters)
            .options(options)
            .execute()
            .await?;

        println!("📊 {} résultats trouvés", search_result.total_count);

        // Convertir les résultats pour l'affichage
        let mut archives = Vec::new();
        for archive in search_result.archives {
            let extended_info = if args.extended {
                Some(ExtendedInfo {
                    creator: archive.creator.to_string(),
                    replicas: archive.replicas.len(),
                    integrity_score: archive.storage_info.integrity_score,
                    access_count: archive.analytics.access_count,
                    last_accessed: archive.analytics.last_accessed
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string()),
                })
            } else {
                None
            };

            archives.push(ArchiveDisplay {
                id: archive.id.to_string(),
                url: archive.url,
                title: archive.metadata.title,
                status: archive.status.to_string(),
                size: archive.size,
                created_at: archive.created_at.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
                tags: archive.metadata.tags,
                extended_info,
            });
        }

        Ok(archives)
    }

    /// Affiche les statistiques de recherche
    pub async fn show_statistics(&self, args: &Args) -> Result<(), Box<dyn Error>> {
        println!("📈 Calcul des statistiques...");

        let stats = self.client
            .search()
            .statistics()
            .filters(build_filters_from_args(args))
            .execute()
            .await?;

        println!("\n📊 Statistiques de la recherche:");
        println!("   Total d'archives: {}", stats.total_archives);
        println!("   Taille totale: {}", format_bytes(stats.total_size));
        println!("   Archive la plus ancienne: {}", 
                 stats.oldest_archive.format("%Y-%m-%d"));
        println!("   Archive la plus récente: {}", 
                 stats.newest_archive.format("%Y-%m-%d"));
        
        println!("\n📋 Répartition par statut:");
        for (status, count) in stats.status_distribution {
            println!("   {}: {}", status, count);
        }

        println!("\n🏷️  Top 10 des tags:");
        for (tag, count) in stats.top_tags.iter().take(10) {
            println!("   {}: {}", tag, count);
        }

        Ok(())
    }
}

/// Affiche les résultats selon le format choisi
fn display_results(archives: &[ArchiveDisplay], format: &OutputFormat) -> Result<(), Box<dyn Error>> {
    match format {
        OutputFormat::Table => display_table(archives),
        OutputFormat::Json => display_json(archives),
        OutputFormat::Csv => display_csv(archives),
        OutputFormat::Summary => display_summary(archives),
    }
}

/// Affiche les résultats en format table
fn display_table(archives: &[ArchiveDisplay]) {
    if archives.is_empty() {
        println!("Aucun résultat trouvé.");
        return;
    }

    println!("\n📋 Résultats de la recherche:");
    println!("{:-<120}", "");
    println!("{:<20} {:<50} {:<15} {:<10} {:<20}", 
             "ID", "URL", "Statut", "Taille", "Créé le");
    println!("{:-<120}", "");

    for archive in archives {
        let short_id = if archive.id.len() > 18 {
            format!("{}...", &archive.id[..15])
        } else {
            archive.id.clone()
        };

        let short_url = if archive.url.len() > 48 {
            format!("{}...", &archive.url[..45])
        } else {
            archive.url.clone()
        };

        println!("{:<20} {:<50} {:<15} {:<10} {:<20}",
                 short_id,
                 short_url,
                 archive.status,
                 format_bytes(archive.size),
                 archive.created_at.split(' ').next().unwrap_or("")
        );

        if let Some(title) = &archive.title {
            let short_title = if title.len() > 60 {
                format!("{}...", &title[..57])
            } else {
                title.clone()
            };
            println!("     📄 {}", short_title);
        }

        if !archive.tags.is_empty() {
            println!("     🏷️  {}", archive.tags.join(", "));
        }

        if let Some(ref extended) = archive.extended_info {
            println!("     👤 Créateur: {}", 
                     if extended.creator.len() > 20 {
                         format!("{}...", &extended.creator[..17])
                     } else {
                         extended.creator.clone()
                     });
            println!("     🔄 Répliques: {} | 🛡️  Intégrité: {:.1}% | 👁️  Accès: {}",
                     extended.replicas,
                     extended.integrity_score * 100.0,
                     extended.access_count);
        }
        println!();
    }
}

/// Affiche les résultats en format JSON
fn display_json(archives: &[ArchiveDisplay]) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(archives)?;
    println!("{}", json);
    Ok(())
}

/// Affiche les résultats en format CSV
fn display_csv(archives: &[ArchiveDisplay]) -> Result<(), Box<dyn Error>> {
    println!("ID,URL,Title,Status,Size,Created,Tags");
    for archive in archives {
        println!("{},{},{},{},{},{},\"{}\"",
                 archive.id,
                 archive.url,
                 archive.title.as_deref().unwrap_or(""),
                 archive.status,
                 archive.size,
                 archive.created_at,
                 archive.tags.join(";")
        );
    }
    Ok(())
}

/// Affiche un résumé des résultats
fn display_summary(archives: &[ArchiveDisplay]) {
    if archives.is_empty() {
        println!("Aucun résultat trouvé.");
        return;
    }

    let total_size: u64 = archives.iter().map(|a| a.size).sum();
    let mut status_counts = std::collections::HashMap::new();
    let mut tag_counts = std::collections::HashMap::new();

    for archive in archives {
        *status_counts.entry(&archive.status).or_insert(0) += 1;
        for tag in &archive.tags {
            *tag_counts.entry(tag).or_insert(0) += 1;
        }
    }

    println!("\n📊 Résumé de la recherche:");
    println!("   📦 Total d'archives: {}", archives.len());
    println!("   💾 Taille totale: {}", format_bytes(total_size));
    println!("   📏 Taille moyenne: {}", format_bytes(total_size / archives.len() as u64));

    println!("\n📈 Répartition par statut:");
    for (status, count) in status_counts {
        println!("   {}: {} ({:.1}%)", 
                 status, 
                 count, 
                 (count as f64 / archives.len() as f64) * 100.0);
    }

    if !tag_counts.is_empty() {
        println!("\n🏷️  Tags les plus fréquents:");
        let mut sorted_tags: Vec<_> = tag_counts.into_iter().collect();
        sorted_tags.sort_by(|a, b| b.1.cmp(&a.1));
        
        for (tag, count) in sorted_tags.into_iter().take(10) {
            println!("   {}: {}", tag, count);
        }
    }
}

/// Utilitaires de conversion et formatage
fn parse_date(date_str: &str) -> Result<DateTime<Utc>, Box<dyn Error>> {
    let naive_date = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")?;
    Ok(DateTime::from_utc(naive_date.and_hms_opt(0, 0, 0).unwrap(), Utc))
}

fn convert_sort_field(field: &SortField) -> archivechain_sdk::SortField {
    match field {
        SortField::CreatedAt => archivechain_sdk::SortField::CreatedAt,
        SortField::Size => archivechain_sdk::SortField::Size,
        SortField::Title => archivechain_sdk::SortField::Title,
        SortField::Url => archivechain_sdk::SortField::Url,
        SortField::Status => archivechain_sdk::SortField::Status,
    }
}

fn convert_sort_order(order: &SortOrder) -> archivechain_sdk::SortOrder {
    match order {
        SortOrder::Asc => archivechain_sdk::SortOrder::Ascending,
        SortOrder::Desc => archivechain_sdk::SortOrder::Descending,
    }
}

fn build_filters_from_args(args: &Args) -> SearchFilters {
    let mut filters = SearchFilters::new();
    
    if let Some(ref tag) = args.tag {
        filters = filters.tag(tag);
    }
    
    if let Some(ref creator) = args.creator {
        filters = filters.creator(creator);
    }
    
    // Ajouter d'autres filtres selon les besoins
    
    filters
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", bytes, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Charger les variables d'environnement
    dotenv().ok();

    // Parser les arguments
    let args = Args::parse();

    // Vérifier la configuration
    if env::var("ARCHIVECHAIN_API_KEY").is_err() {
        eprintln!("❌ ARCHIVECHAIN_API_KEY non définie dans l'environnement");
        eprintln!("   Créez un fichier .env avec votre clé API:");
        eprintln!("   ARCHIVECHAIN_API_KEY=your-api-key-here");
        std::process::exit(1);
    }

    // Créer le chercheur
    let searcher = ArchiveSearcher::new().await?;

    // Effectuer la recherche
    let archives = searcher.search(&args).await?;

    // Afficher les résultats
    display_results(&archives, &args.format)?;

    // Afficher les statistiques si demandées
    if args.extended {
        searcher.show_statistics(&args).await?;
    }

    println!("\n✅ Recherche terminée.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_bytes() {
        assert_eq!(format_bytes(512), "512 B");
        assert_eq!(format_bytes(1024), "1.0 KB");
        assert_eq!(format_bytes(1536), "1.5 KB");
        assert_eq!(format_bytes(1048576), "1.0 MB");
    }

    #[test]
    fn test_parse_date() {
        let date = parse_date("2024-01-01").unwrap();
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), 1);
        assert_eq!(date.day(), 1);
    }
}