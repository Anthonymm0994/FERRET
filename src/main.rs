use clap::{Parser, Subcommand};  // Command-line argument parsing
use std::path::PathBuf;  // Cross-platform path handling
use anyhow::Result;  // Error handling

// Module declarations for all FERRET components
mod file_discovery;  // File discovery and intelligent grouping
mod io;  // I/O utilities and file operations
mod analysis;  // Duplicate detection and file analysis
mod search;  // Search engine and content search
mod extraction;  // Document content extraction
mod integrations;  // External tool integrations
mod platform;  // Main platform orchestration

use platform::FerretPlatform;  // Main platform for coordinating all functionality

/// Command-line interface for FERRET
/// This defines the CLI structure and all available commands
#[derive(Parser)]
#[command(name = "ferret")]
#[command(about = "A powerful file analysis and search tool for cleaning up messy shared drives")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

/// Available FERRET commands
/// Each command provides a specific functionality for file analysis and search
#[derive(Subcommand)]
enum Commands {
    /// Analyze directory for duplicates and similar files
    /// This is the main analysis command that finds duplicate files and groups similar ones
    Analyze {
        /// Directory to analyze
        path: PathBuf,
        /// Output format (json, text)
        #[arg(short, long, default_value = "text")]
        format: String,
    },
    /// Search for files and content
    /// This command searches for text within files, including document content
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
    /// This command creates a persistent index for much faster subsequent searches
    Index {
        /// Directory to index
        path: PathBuf,
        /// Index location
        #[arg(short, long, default_value = "./ferret_index")]
        index_path: PathBuf,
    },
}

/// Main entry point for FERRET
/// This function parses command-line arguments and executes the appropriate command
#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging for debugging and error reporting
    env_logger::init();
    
    // Parse command-line arguments
    let cli = Cli::parse();
    
    // Execute the requested command
    match cli.command {
        Commands::Analyze { path, format } => {
            // Validate that the path exists
            if !path.exists() {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
            
            // Create platform and analyze the directory
            let mut platform = FerretPlatform::new()?;
            let results = platform.analyze_directory(&path).await?;
            
            // Output results in the requested format
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&results)?),
                _ => println!("{:#?}", results),
            }
        }
        Commands::Search { query, path, limit } => {
            // Validate search query
            if query.is_empty() {
                eprintln!("Error: Search query cannot be empty");
                std::process::exit(1);
            }
            
            // Validate limit parameter
            if limit == 0 {
                eprintln!("Error: Limit must be greater than 0");
                std::process::exit(1);
            }
            
            // Validate that the path exists
            if !path.exists() {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
            
            // Create platform for search
            let mut platform = FerretPlatform::new()?;
            
            // Check if there's an index in the current directory
            // If an index exists, use it for much faster searching
            let index_path = std::path::Path::new("./ferret_index");
            if index_path.exists() {
                // Load the existing index for fast searching
                let engine = crate::search::engine::RipgrepSearchEngine::new(index_path)?;
                platform.set_search_engine(engine);
            }
            
            // Perform the search
            let results = platform.search(&query, &path, limit).await?;
            
            // Display search results with rich formatting
            for (i, result) in results.iter().enumerate() {
                if i > 0 {
                    println!(); // Add spacing between results
                }
                
                // Header with file info and metadata
                println!("🔍 {} (Score: {:.1}, {} matches, {} bytes, .{})", 
                    result.path.display(), 
                    result.score,
                    result.match_count,
                    result.file_size,
                    result.file_type
                );
                
                // Context lines before the match
                for (j, context_line) in result.context_before.iter().enumerate() {
                    let base_line = result.line_number.unwrap_or(0);
                    let line_num = base_line.saturating_sub((result.context_before.len() - j) as u64);
                    println!("  {:3} │ {}", line_num, context_line);
                }
                
                // Main match line (highlighted with arrow)
                if let Some(line_num) = result.line_number {
                    println!("  {:3} │ {} ← MATCH", line_num, result.snippet);
                }
                
                // Context lines after the match
                for (j, context_line) in result.context_after.iter().enumerate() {
                    let line_num = result.line_number.unwrap_or(0) + (j + 1) as u64;
                    println!("  {:3} │ {}", line_num, context_line);
                }
            }
        }
        Commands::Index { path, index_path } => {
            // Validate that the path exists
            if !path.exists() {
                eprintln!("Error: Path does not exist: {}", path.display());
                std::process::exit(1);
            }
            
            // Create platform and index the directory
            let mut platform = FerretPlatform::new()?;
            platform.index_directory(&path, &index_path).await?;
            println!("Indexing complete for: {}", path.display());
        }
    }
    
    Ok(())
}
