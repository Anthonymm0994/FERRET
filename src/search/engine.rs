use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use anyhow::Result;
// use tantivy::{
//     collector::TopDocs,
//     directory::MmapDirectory,
//     query::QueryParser,
//     schema::{Schema, Field, STORED, TEXT},
//     Index, IndexReader, IndexWriter, ReloadPolicy, Term,
// };
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, SearcherBuilder, Sink, SinkMatch};
use ignore::WalkBuilder;

pub struct RipgrepSearchEngine {
    // Temporarily simplified without tantivy
    document_extractor: crate::extraction::document::DocumentExtractor,
    index_path: std::path::PathBuf,
}

pub struct RipgrepIntegration;

impl RipgrepIntegration {
    pub fn search_with_ripgrep(&self, pattern: &str, path: &Path) -> Result<Vec<SearchResult>> {
        use grep_searcher::SearcherBuilder;
        use grep_searcher::sinks::UTF8;
        
        let matcher = RegexMatcher::new(pattern)?;
        let mut results = Vec::new();
        
        for entry in WalkBuilder::new(path).build() {
            let entry = entry?;
            if !entry.file_type().map_or(false, |ft| ft.is_file()) {
                continue;
            }
            
            let file_path = entry.path();
            let mut searcher = SearcherBuilder::new().build();
            
            // Use grep's sink with PROPER path handling
            searcher.search_path(
                &matcher,
                file_path,
                UTF8(|line_num, line| {
                    results.push(SearchResult {
                        path: file_path.to_path_buf(), // THIS WAS THE BUG - use actual path!
                        line_number: Some(line_num as u64),
                        snippet: line.to_string(),
                        score: 1.0,
                        content: Some(line.to_string()),
                    });
                    Ok(true)
                })
            )?;
        }
        
        Ok(results)
    }
}


impl RipgrepSearchEngine {
    pub fn new(index_path: &Path) -> Result<Self> {
        // Temporarily simplified without tantivy
        Ok(Self {
            document_extractor: crate::extraction::document::DocumentExtractor::new(),
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
}

#[derive(Debug)]
pub struct FileMetadata {
    pub size: u64,
    pub modified: std::time::SystemTime,
    pub is_binary: bool,
}
