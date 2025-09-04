use std::path::{Path, PathBuf};
use anyhow::Result;
// use tantivy::{
//     collector::TopDocs,
//     directory::MmapDirectory,
//     query::QueryParser,
//     schema::{Schema, Field, STORED, TEXT},
//     Index, IndexReader, IndexWriter, ReloadPolicy, Term,
// };
use grep_regex::RegexMatcher;
use ignore::WalkBuilder;
use crate::extraction::document::DocumentExtractor;

pub struct RipgrepSearchEngine {
    index_path: std::path::PathBuf,
}

pub struct RipgrepIntegration;

impl RipgrepIntegration {
    pub async fn search_with_ripgrep(&self, pattern: &str, path: &Path) -> Result<Vec<SearchResult>> {
        use grep_searcher::SearcherBuilder;
        use grep_searcher::sinks::UTF8;
        
        let matcher = RegexMatcher::new(pattern)?;
        let extractor = DocumentExtractor::new();
        let mut file_results: std::collections::HashMap<PathBuf, Vec<(u64, String)>> = std::collections::HashMap::new();
        
        // First pass: collect all matches per file
        for entry in WalkBuilder::new(path).build() {
            let entry = entry?;
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
            
            let file_path = entry.path();
            let file_extension = file_path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_lowercase();
            
            // For document files, extract content first
            if matches!(file_extension.as_str(), "pdf" | "docx" | "xlsx" | "pptx") {
                match extractor.extract_content(file_path).await {
                    Ok(content) => {
                        // Search in extracted content
                        let lines: Vec<&str> = content.lines().collect();
                        for (line_num, line) in lines.iter().enumerate() {
                            if line.to_lowercase().contains(&pattern.to_lowercase()) {
                                file_results.entry(file_path.to_path_buf())
                                    .or_insert_with(Vec::new)
                                    .push((line_num as u64 + 1, line.to_string()));
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to extract content from {}: {}", file_path.display(), e);
                        continue;
                    }
                }
            } else {
                // For text files, use ripgrep directly
                let mut searcher = SearcherBuilder::new().build();
                
                // Only search text files, skip binary files
                if let Ok(content) = std::fs::read_to_string(file_path) {
                    let lines: Vec<&str> = content.lines().collect();
                    for (line_num, line) in lines.iter().enumerate() {
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
        let mut results = Vec::new();
        for (file_path, matches) in file_results {
            let file_size = std::fs::metadata(&file_path).map(|m| m.len()).unwrap_or(0);
            let file_type = file_path.extension()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            let match_count = matches.len();
            
            // Get file content for context
            let file_content = if matches!(file_type.as_str(), "pdf" | "docx" | "xlsx" | "pptx") {
                extractor.extract_content(&file_path).await.unwrap_or_default()
            } else {
                std::fs::read_to_string(&file_path).unwrap_or_default()
            };
            
            let lines: Vec<&str> = file_content.lines().collect();
            
            // Group nearby matches to avoid duplicate results
            let mut processed_lines = std::collections::HashSet::new();
            
            for (line_num, line_content) in &matches {
                if processed_lines.contains(line_num) {
                    continue;
                }
                
                // Calculate context (3 lines before and after)
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
                
                // Calculate relevance score
                let score = self.calculate_relevance_score(line_content, pattern, &file_path);
                
                // Mark nearby lines as processed to avoid duplicates
                for i in line_num.saturating_sub(2)..=line_num + 2 {
                    processed_lines.insert(i);
                }
                
                results.push(SearchResult {
                    path: file_path.clone(),
                    score,
                    snippet: line_content.clone(),
                    line_number: Some(*line_num),
                    content: Some(line_content.clone()),
                    context_before,
                    context_after,
                    match_count,
                    file_size,
                    file_type: file_type.clone(),
                });
            }
        }
        
        // Sort by relevance score (highest first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        
        Ok(results)
    }
    
    fn calculate_relevance_score(&self, line: &str, pattern: &str, file_path: &Path) -> f32 {
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
}


impl RipgrepSearchEngine {
    pub fn new(index_path: &Path) -> Result<Self> {
        Ok(Self {
            index_path: index_path.to_path_buf(),
        })
    }
    
    pub async fn index_file(&mut self, path: &Path, _metadata: &FileMetadata) -> Result<()> {
        // Simple file-based index implementation
        use std::collections::HashMap;
        use serde_json;
        
        // Create index directory if it doesn't exist
        std::fs::create_dir_all(&self.index_path)?;
        
        // Read file content for indexing
        let content = match tokio::fs::read_to_string(path).await {
            Ok(content) => content,
            Err(_) => return Ok(()), // Skip binary files or files that can't be read
        };
        
        // Store in a simple JSON index file
        let index_file = self.index_path.join("ferret_index.json");
        let mut index: HashMap<String, String> = if index_file.exists() {
            let data = tokio::fs::read_to_string(&index_file).await?;
            serde_json::from_str(&data).unwrap_or_default()
        } else {
            HashMap::new()
        };
        
        // Add file to index
        index.insert(path.to_string_lossy().to_string(), content);
        
        // Write back to index file
        let json = serde_json::to_string_pretty(&index)?;
        tokio::fs::write(&index_file, json).await?;
        
        Ok(())
    }
    
    pub fn search(&self, _query_str: &str, _limit: usize) -> Result<Vec<SearchResult>> {
        // Temporarily simplified - return empty results
        log::info!("Search temporarily disabled - tantivy not available");
        Ok(Vec::new())
    }
    
    pub fn commit(&mut self) -> Result<()> {
        // Temporarily simplified
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub path: PathBuf,
    pub score: f32,
    pub snippet: String,
    pub line_number: Option<u64>,
    pub content: Option<String>,
    pub context_before: Vec<String>,
    pub context_after: Vec<String>,
    pub match_count: usize,
    pub file_size: u64,
    pub file_type: String,
}

#[derive(Debug)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub is_binary: bool,
}
