use clap::{Parser, Subcommand};
use std::path::PathBuf;
use anyhow::Result;

mod file_discovery;
mod io;
mod analysis;
mod search;
mod extraction;
mod integrations;
mod platform;

use platform::FerretPlatform;

#[derive(Parser)]
#[command(name = "ferret")]
#[command(about = "A powerful file analysis and search tool for cleaning up messy shared drives")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Analyze directory for duplicates and similar files
    Analyze {
        /// Directory to analyze
        path: PathBuf,
        /// Output format (json, text)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Search for files and content
    Search {
        /// Search query
        query: String,
        /// Directory to search in
        path: PathBuf,
        /// Maximum number of results
        #[arg(short, long, default_value = "100")]
        limit: usize,
    },
    /// Index directory for fast searching
    Index {
        /// Directory to index
        path: PathBuf,
        /// Index location
        #[arg(short, long, default_value = "./ferret_index")]
        index_path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Analyze { path, format } => {
            if !path.exists() {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
            
            let mut platform = FerretPlatform::new()?;
            let results = platform.analyze_directory(&path).await?;
            
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&results)?),
                _ => println!("{:#?}", results),
            }
        }
        Commands::Search { query, path, limit } => {
            if query.is_empty() {
                eprintln!("Error: Search query cannot be empty");
                std::process::exit(1);
            }
            
            if limit == 0 {
                eprintln!("Error: Limit must be greater than 0");
                std::process::exit(1);
            }
            
            if !path.exists() {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
            
            let mut platform = FerretPlatform::new()?;
            let results = platform.search(&query, &path, limit).await?;
            
            for (i, result) in results.iter().enumerate() {
                if i > 0 {
                    println!(); // Add spacing between results
                }
                
                // Header with file info
                println!("🔍 {} (Score: {:.1}, {} matches, {} bytes, .{})", 
                    result.path.display(), 
                    result.score,
                    result.match_count,
                    result.file_size,
                    result.file_type
                );
                
                // Context before
                for (j, context_line) in result.context_before.iter().enumerate() {
                    let base_line = result.line_number.unwrap_or(0);
                    let line_num = base_line.saturating_sub((result.context_before.len() - j) as u64);
                    println!("  {:3} │ {}", line_num, context_line);
                }
                
                // Main match line (highlighted)
                if let Some(line_num) = result.line_number {
                    println!("  {:3} │ {} ← MATCH", line_num, result.snippet);
                }
                
                // Context after
                for (j, context_line) in result.context_after.iter().enumerate() {
                    let line_num = result.line_number.unwrap_or(0) + (j + 1) as u64;
                    println!("  {:3} │ {}", line_num, context_line);
                }
            }
        }
        Commands::Index { path, index_path } => {
            if !path.exists() {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
            
            let mut platform = FerretPlatform::new()?;
            platform.index_directory(&path, &index_path).await?;
            println!("Indexing complete for: {}", path.display());
        }
    }
    
    Ok(())
}
