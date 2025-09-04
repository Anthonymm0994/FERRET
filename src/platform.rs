use std::path::Path;
use anyhow::Result;

use crate::file_discovery::FileDiscovery;  // For discovering and grouping files
use crate::analysis::duplicates::{SmartDuplicateDetector, DuplicateResults};  // For duplicate detection
use crate::search::engine::{RipgrepSearchEngine, SearchResult};  // For search functionality

/// Main platform that orchestrates all FERRET functionality
/// This is the central hub that coordinates file discovery, duplicate detection, and search
/// It provides a unified interface for all the tool's capabilities
pub struct FerretPlatform {
    /// Handles file discovery and intelligent grouping
    file_discovery: FileDiscovery,
    /// Detects exact duplicates using SHA-256 hashing
    duplicate_detector: SmartDuplicateDetector,
    /// Optional search engine for indexed searches (faster than real-time)
    search_engine: Option<RipgrepSearchEngine>,
}

impl FerretPlatform {
    /// Creates a new FerretPlatform instance with all components initialized
    /// This is the main entry point for creating a FERRET platform
    /// 
    /// # Returns
    /// * `Result<Self>` - The new platform instance
    pub fn new() -> Result<Self> {
        Ok(Self {
            file_discovery: FileDiscovery::new(),
            duplicate_detector: SmartDuplicateDetector::new(),
            search_engine: None,
        })
    }
    
    /// Sets the search engine for indexed searches
    /// This enables faster searches when an index is available
    /// 
    /// # Arguments
    /// * `engine` - The search engine to use for indexed searches
    pub fn set_search_engine(&mut self, engine: RipgrepSearchEngine) {
        self.search_engine = Some(engine);
    }
    
    /// Analyzes a directory for duplicates and file organization
    /// This is the main analysis function that discovers files and finds duplicates
    /// 
    /// # Arguments
    /// * `path` - Directory to analyze
    /// 
    /// # Returns
    /// * `Result<AnalysisResults>` - Analysis results including duplicate statistics
    pub async fn analyze_directory(&mut self, path: &Path) -> Result<AnalysisResults> {
        // Step 1: Discover and group files by similarity
        let file_groups = self.file_discovery.discover_files(path).await?;
        
        // Step 2: Detect exact duplicates within similar file groups
        let duplicate_results = self.duplicate_detector.detect_duplicates(&file_groups).await?;
        
        // Step 3: Calculate statistics and return results
        Ok(AnalysisResults {
            total_files: file_groups.iter().map(|g| g.variants.len()).sum(),
            total_groups: file_groups.len(),
            duplicate_results,
        })
    }
    
    /// Searches for text within files in a directory
    /// This method provides both indexed and real-time search capabilities
    /// 
    /// # Arguments
    /// * `query` - Search term to look for
    /// * `path` - Directory to search within
    /// * `limit` - Maximum number of results to return
    /// 
    /// # Returns
    /// * `Result<Vec<SearchResult>>` - List of search results with context
    pub async fn search(&mut self, query: &str, path: &Path, limit: usize) -> Result<Vec<SearchResult>> {
        if let Some(ref engine) = self.search_engine {
            // Use indexed search if available (much faster)
            engine.search(query, limit).await
        } else {
            // Fallback to real-time search using ripgrep integration
            let integration = crate::search::engine::RipgrepIntegration;
            let mut results = integration.search_with_ripgrep(query, path).await?;
            
            // Apply limit to results
            results.truncate(limit);
            Ok(results)
        }
    }
    
    /// Indexes a directory for fast searching
    /// This creates a persistent index that enables much faster subsequent searches
    /// 
    /// # Arguments
    /// * `path` - Directory to index
    /// * `index_path` - Where to store the index files
    /// 
    /// # Returns
    /// * `Result<()>` - Success or error
    pub async fn index_directory(&mut self, path: &Path, index_path: &Path) -> Result<()> {
        // Create a new search engine for this index
        let mut engine = RipgrepSearchEngine::new(index_path)?;
        
        // Discover all files in the directory
        let file_groups = self.file_discovery.discover_files(path).await?;
        
        // Index each file for fast searching
        for group in file_groups {
            for file in group.variants {
                engine.index_file(&file).await?;
            }
        }
        
        // Commit the index and set it as the active search engine
        engine.commit()?;
        self.search_engine = Some(engine);
        
        Ok(())
    }
}

/// Results of directory analysis
/// Contains statistics about files found and duplicates detected
#[derive(Debug, serde::Serialize)]
pub struct AnalysisResults {
    /// Total number of files discovered
    pub total_files: usize,
    /// Total number of file groups (similar files grouped together)
    pub total_groups: usize,
    /// Detailed duplicate detection results
    pub duplicate_results: DuplicateResults,
}


