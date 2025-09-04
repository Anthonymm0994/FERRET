use std::path::{Path, PathBuf};
use anyhow::Result;
// Future enhancement: Tantivy integration for advanced full-text search
// Tantivy would provide more sophisticated search capabilities like:
// - BM25 relevance scoring
// - Boolean queries and phrase search
// - Faceted search and filtering
// - Distributed search across multiple indexes
// use tantivy::{
//     collector::TopDocs,
//     directory::MmapDirectory,
//     query::QueryParser,
//     schema::{Schema, Field, STORED, TEXT},
//     Index, IndexReader, IndexWriter, ReloadPolicy, Term,
// };
use ignore::WalkBuilder;  // Fast file traversal with .gitignore support
use crate::extraction::document::DocumentExtractor;  // For extracting text from documents
use serde::{Deserialize, Serialize};  // For JSON serialization of the index

/// Represents a single file entry in our search index
/// Contains all metadata needed for fast searching without re-reading files
#[derive(Debug, Serialize, Deserialize)]
struct IndexEntry {
    /// Full text content of the file (extracted from documents or raw text)
    content: String,
    /// File size in bytes - used for relevance scoring and filtering
    file_size: u64,
    /// Last modified timestamp (Unix epoch seconds) - used for freshness scoring
    modified: u64,
    /// File extension/type - used for filtering and relevance scoring
    file_type: String,
}

/// Persistent search engine that maintains a JSON-based index
/// This allows for fast repeated searches without re-scanning the filesystem
/// The index stores file content and metadata for instant retrieval
pub struct RipgrepSearchEngine {
    /// Directory where the search index files are stored
    index_path: std::path::PathBuf,
}

/// Real-time search integration using ripgrep-like functionality
/// This performs live filesystem searches without requiring pre-built indexes
/// Used as fallback when no index exists or for one-off searches
pub struct RipgrepIntegration;

impl RipgrepIntegration {
    /// Performs a real-time search across files in the given directory
    /// This method scans files on-demand without requiring a pre-built index
    /// 
    /// # Arguments
    /// * `pattern` - The search term to look for (case-insensitive)
    /// * `path` - Directory to search within
    /// 
    /// # Returns
    /// * `Result<Vec<SearchResult>>` - List of search results with context and metadata
    pub async fn search_with_ripgrep(&self, pattern: &str, path: &Path) -> Result<Vec<SearchResult>> {
        let extractor = DocumentExtractor::new();
        // Map file paths to their matching lines for efficient processing
        let mut file_results: std::collections::HashMap<PathBuf, Vec<(u64, String)>> = std::collections::HashMap::new();
        
        // First pass: collect all matches per file
        // We use WalkBuilder (from ignore crate) for efficient filesystem traversal
        // This respects .gitignore and other ignore files automatically
        for entry in WalkBuilder::new(path).build() {
            let entry = entry?;
            // Skip directories and non-files (symlinks, etc.)
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
            
            let file_path = entry.path();
            let file_extension = file_path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            // Handle document files (PDF, DOCX, etc.) that need content extraction
            // These files can't be read as plain text, so we extract their content first
            if matches!(file_extension.as_str(), "pdf" | "docx" | "xlsx" | "pptx") {
                match extractor.extract_content(file_path).await {
                    Ok(content) => {
                        // Search in the extracted text content
                        let lines: Vec<&str> = content.lines().collect();
                        for (line_num, line) in lines.iter().enumerate() {
                            // Case-insensitive substring matching
                            if line.to_lowercase().contains(&pattern.to_lowercase()) {
                                file_results.entry(file_path.to_path_buf())
                                    .or_insert_with(Vec::new)
                                    .push((line_num as u64 + 1, line.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        // Log extraction failures but continue processing other files
                        log::warn!("Failed to extract content from {}: {}", file_path.display(), e);
                        continue;
                    }
                }
            } else {
                // For plain text files, read directly without extraction
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    let lines: Vec<&str> = content.lines().collect();
                    for (line_num, line) in lines.iter().enumerate() {
                        // Case-insensitive substring matching
                        if line.to_lowercase().contains(&pattern.to_lowercase()) {
                            file_results.entry(file_path.to_path_buf())
                                .or_insert_with(Vec::new)
                                .push((line_num as u64 + 1, line.to_string()));
                        }
                    }
                }
            }
        }
        
        // Second pass: create rich search results with context
        // Now we process each file's matches to create SearchResult objects with context
        let mut results = Vec::new();
        for (file_path, matches) in file_results {
            // Get file metadata for the search result
            let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
            let file_type = file_path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let match_count = matches.len();
            
            // Re-read file content to provide context around matches
            // For document files, we need to extract content again
            // For text files, we can read directly
            let file_content = if matches!(file_type.as_str(), "pdf" | "docx" | "xlsx" | "pptx") {
                extractor.extract_content(&file_path).await.unwrap_or_default()
            } else {
                std::fs::read_to_string(&file_path).unwrap_or_default()
            };
            
            let lines: Vec<&str> = file_content.lines().collect();
            
            // Group nearby matches to avoid duplicate results
            // If multiple matches are close together, we only show one result
            // to avoid cluttering the output with overlapping context
            let mut processed_lines = std::collections::HashSet::new();
            
            for (line_num, line_content) in &matches {
                // Skip if we've already processed a nearby line
                if processed_lines.contains(line_num) {
                    continue;
                }
                
                // Calculate context lines (3 lines before and after the match)
                // This helps users understand the match in context
                let context_before: Vec<String> = lines
                    .iter()
                    .skip((line_num.saturating_sub(4)) as usize)
                    .take(3)
                    .map(|s| s.to_string())
                    .collect();
                
                let context_after: Vec<String> = lines
                    .iter()
                    .skip((line_num + 1) as usize)
                    .take(3)
                    .map(|s| s.to_string())
                    .collect();
                
                // Calculate relevance score based on match quality and file properties
                let score = self.calculate_relevance_score(line_content, pattern, &file_path);
                
                // Mark nearby lines as processed to avoid duplicates
                // This prevents showing multiple results for the same context
                for i in line_num.saturating_sub(2)..=line_num + 2 {
                    processed_lines.insert(i);
                }
                
                // Create the search result with all context and metadata
                results.push(SearchResult {
                    path: file_path.clone(),
                    score,
                    snippet: line_content.clone(),
                    line_number: Some(*line_num),
                    context_before,
                    context_after,
                    match_count,
                    file_size,
                    file_type: file_type.clone(),
                });
            }
        }
        
        // Sort by relevance score (highest first) to show most relevant results first
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }
    
    /// Calculates a relevance score for a search match
    /// Higher scores indicate more relevant matches that should appear first
    /// 
    /// # Arguments
    /// * `line` - The line containing the match
    /// * `pattern` - The search pattern that was matched
    /// * `file_path` - Path to the file containing the match
    /// 
    /// # Returns
    /// * `f32` - Relevance score (higher = more relevant)
    fn calculate_relevance_score(&self, line: &str, pattern: &str, file_path: &Path) -> f32 {
        let mut score = 1.0; // Base score for any match
        
        // Boost score for exact matches (case-insensitive)
        if line.to_lowercase().contains(&pattern.to_lowercase()) {
            score += 2.0;
        }
        
        // Boost score for multiple occurrences in the same line
        // This indicates the line is highly relevant to the search term
        let occurrences = line.to_lowercase().matches(&pattern.to_lowercase()).count();
        score += occurrences as f32 * 0.5;
        
        // Boost score for filename matches - these are often very relevant
        // If the search term appears in the filename, it's likely very important
        if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
            if filename.to_lowercase().contains(&pattern.to_lowercase()) {
                score += 1.5;
            }
        }
        
        // Boost score for shorter lines (more precise matches)
        // Shorter lines with matches are often more focused and relevant
        if line.len() < 100 {
            score += 0.5;
        }
        
        score
    }
}


impl RipgrepSearchEngine {
    /// Creates a new search engine with the specified index directory
    /// The index directory is where all search index files will be stored
    /// 
    /// # Arguments
    /// * `index_path` - Directory where the search index will be stored
    /// 
    /// # Returns
    /// * `Result<Self>` - The new search engine instance
    pub fn new(index_path: &Path) -> Result<Self> {
        Ok(Self {
            index_path: index_path.to_path_buf(),
        })
    }
    
    /// Indexes a single file for fast searching
    /// This extracts content from the file and stores it with metadata in the index
    /// 
    /// # Arguments
    /// * `path` - Path to the file to index
    /// 
    /// # Returns
    /// * `Result<()>` - Success or error
    pub async fn index_file(&mut self, path: &Path) -> Result<()> {
        use std::collections::HashMap;
        use serde_json;
        
        // Create index directory if it doesn't exist
        std::fs::create_dir_all(&self.index_path)?;
        
        // Extract content using DocumentExtractor
        // This handles different file types (PDF, DOCX, etc.) automatically
        let extractor = DocumentExtractor::new();
        let content = match extractor.extract_content(path).await {
            Ok(content) => content,
            Err(_) => return Ok(()), // Skip files that can't be processed
        };
        
        // Get file metadata for the index entry
        let metadata = std::fs::metadata(path)?;
        let file_size = metadata.len();
        let modified = metadata.modified()?;
        let file_type = path.extension()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();
        
        // Create enhanced index entry with all metadata
        let index_entry = IndexEntry {
            content,
            file_size,
            modified: modified.duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
            file_type,
        };
        
        // Load existing index or create new one
        let index_file = self.index_path.join("ferret_index.json");
        let mut index: HashMap<String, IndexEntry> = if index_file.exists() {
            let data = tokio::fs::read_to_string(&index_file).await?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        // Add file to index (overwrites if already exists)
        index.insert(path.to_string_lossy().to_string(), index_entry);
        
        // Write updated index back to disk
        let json = serde_json::to_string_pretty(&index)?;
        tokio::fs::write(&index_file, json).await?;
        
        Ok(())
    }
    
    /// Searches through the pre-built index for fast results
    /// This method is much faster than real-time search as it doesn't read files from disk
    /// 
    /// # Arguments
    /// * `query_str` - The search term to look for (case-insensitive)
    /// * `limit` - Maximum number of results to return
    /// 
    /// # Returns
    /// * `Result<Vec<SearchResult>>` - List of search results with context and metadata
    pub async fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>> {
        use std::collections::HashMap;
        use serde_json;
        
        let index_file = self.index_path.join("ferret_index.json");
        if !index_file.exists() {
            return Ok(Vec::new());
        }
        
        // Load the entire index from disk
        // This is fast because we only read one JSON file instead of many individual files
        let data = tokio::fs::read_to_string(&index_file).await?;
        let index: HashMap<String, IndexEntry> = serde_json::from_str(&data)?;
        
        let mut results = Vec::new();
        let query_lower = query_str.to_lowercase();
        
        // Search through each indexed file
        for (file_path_str, entry) in index {
            let file_path = std::path::PathBuf::from(&file_path_str);
            let lines: Vec<&str> = entry.content.lines().collect();
            let mut matches = Vec::new();
            
            // Search in the pre-extracted content
            for (line_num, line) in lines.iter().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    matches.push((line_num as u64 + 1, line.to_string()));
                }
            }
            
            if !matches.is_empty() {
                let match_count = matches.len();
                
                // Create search results for each match with context
                for (line_num, line_content) in matches {
                    // Calculate context lines (3 before and after)
                    let context_before: Vec<String> = lines
                        .iter()
                        .skip((line_num.saturating_sub(4)) as usize)
                        .take(3)
                        .map(|s| s.to_string())
                        .collect();
                    
                    let context_after: Vec<String> = lines
                        .iter()
                        .skip((line_num + 1) as usize)
                        .take(3)
                        .map(|s| s.to_string())
                        .collect();
                    
                    // Calculate relevance score using the same algorithm as real-time search
                    let score = self.calculate_relevance_score(&line_content, query_str, &file_path);
                    
                    // Create search result with all metadata from the index
                    results.push(SearchResult {
                        path: file_path.clone(),
                        score,
                        snippet: line_content.clone(),
                        line_number: Some(line_num),
                        context_before,
                        context_after,
                        match_count,
                        file_size: entry.file_size,
                        file_type: entry.file_type.clone(),
                    });
                }
            }
        }
        
        // Sort by relevance score and apply limit
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        results.truncate(limit);
        
        Ok(results)
    }
    
    fn calculate_relevance_score(&self, line: &str, pattern: &str, file_path: &std::path::Path) -> f32 {
        let mut score = 1.0;
        
        // Boost score for exact matches
        if line.to_lowercase().contains(&pattern.to_lowercase()) {
            score += 2.0;
        }
        
        // Boost score for multiple occurrences in the line
        let occurrences = line.to_lowercase().matches(&pattern.to_lowercase()).count();
        score += occurrences as f32 * 0.5;
        
        // Boost score for filename matches
        if let Some(filename) = file_path.file_name().and_then(|n| n.to_str()) {
            if filename.to_lowercase().contains(&pattern.to_lowercase()) {
                score += 1.5;
            }
        }
        
        // Boost score for shorter lines (more precise matches)
        if line.len() < 100 {
            score += 0.5;
        }
        
        score
    }
    
    pub fn commit(&mut self) -> Result<()> {
        // Index is automatically committed on each file addition
        Ok(())
    }
}

/// Represents a single search result with context and metadata
/// This structure contains all the information needed to display a search result
/// to the user, including the matching text, context, and file information
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// Path to the file containing the match
    pub path: PathBuf,
    /// Relevance score for this result (higher = more relevant)
    pub score: f32,
    /// The actual line of text that matched the search query
    pub snippet: String,
    /// Line number where the match occurred (if available)
    pub line_number: Option<u64>,
    /// Lines before the match for context (typically 3 lines)
    pub context_before: Vec<String>,
    /// Lines after the match for context (typically 3 lines)
    pub context_after: Vec<String>,
    /// Total number of matches found in this file
    pub match_count: usize,
    /// Size of the file in bytes
    pub file_size: u64,
    /// File extension/type (e.g., "txt", "pdf", "docx")
    pub file_type: String,
}

