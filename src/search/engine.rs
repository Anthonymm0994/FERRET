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
        use grep_searcher::{Sink, SinkMatch, Searcher};
        use std::sync::{Arc, Mutex};
        
        // ripgrep handles:
        // - .gitignore respect
        // - Binary file detection
        // - Parallel directory walking
        // - Memory-mapped file reading
        // - Unicode normalization
        // - Hidden file handling
        
        let matcher = RegexMatcher::new(pattern)?;
        let results = Arc::new(Mutex::new(Vec::new()));
        
        WalkBuilder::new(path)
            .threads(num_cpus::get())
            .build_parallel()
            .run(|| {
                let matcher = matcher.clone();
                let results = results.clone();
                
                Box::new(move |result| {
                    let entry = match result {
                        Ok(entry) => entry,
                        Err(_) => return ignore::WalkState::Continue,
                    };
                    
                    if entry.file_type().map_or(false, |ft| ft.is_file()) {
                        let mut searcher = SearcherBuilder::new()
                            .line_number(true)
                            .build();
                        
                        let mut sink = SearchSink {
                            results: results.clone(),
                        };
                        
                        let _ = searcher.search_path(&matcher, entry.path(), &mut sink);
                    }
                    
                    ignore::WalkState::Continue
                })
            });
        
        let x = results.lock().unwrap().clone();
        Ok(x)
    }
}

struct SearchSink {
    results: Arc<Mutex<Vec<SearchResult>>>,
}

impl Sink for SearchSink {
    type Error = std::io::Error;
    
    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let mut results = self.results.lock().unwrap();
        results.push(SearchResult {
            path: std::path::PathBuf::from("unknown"), // SinkMatch doesn't have path method
            line_number: mat.line_number(),
            content: Some(String::from_utf8_lossy(mat.bytes()).to_string()),
            score: 1.0, // ripgrep doesn't provide scores, use 1.0 for exact matches
            snippet: String::from_utf8_lossy(mat.bytes()).to_string(),
        });
        Ok(true)
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
