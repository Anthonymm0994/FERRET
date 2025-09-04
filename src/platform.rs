use std::path::Path;
use anyhow::Result;

use crate::file_discovery::FileDiscovery;
use crate::analysis::duplicates::{SmartDuplicateDetector, DuplicateResults};
use crate::search::engine::{RipgrepSearchEngine, SearchResult};

pub struct FerretPlatform {
    file_discovery: FileDiscovery,
    duplicate_detector: SmartDuplicateDetector,
    search_engine: Option<RipgrepSearchEngine>,
}

impl FerretPlatform {
    pub fn new() -> Result<Self> {
        Ok(Self {
            file_discovery: FileDiscovery::new(),
            duplicate_detector: SmartDuplicateDetector::new(),
            search_engine: None,
        })
    }
    
    pub fn set_search_engine(&mut self, engine: RipgrepSearchEngine) {
        self.search_engine = Some(engine);
    }
    
    pub async fn analyze_directory(&mut self, path: &Path) -> Result<AnalysisResults> {
        let file_groups = self.file_discovery.discover_files(path).await?;
        let duplicate_results = self.duplicate_detector.detect_duplicates(&file_groups).await?;
        
        Ok(AnalysisResults {
            total_files: file_groups.iter().map(|g| g.variants.len()).sum(),
            total_groups: file_groups.len(),
            duplicate_results,
        })
    }
    
    pub async fn search(&mut self, query: &str, path: &Path, limit: usize) -> Result<Vec<SearchResult>> {
        if let Some(ref engine) = self.search_engine {
            engine.search(query, limit).await
        } else {
            // Fallback to ripgrep integration
            let integration = crate::search::engine::RipgrepIntegration;
            let mut results = integration.search_with_ripgrep(query, path).await?;
            
            // Apply limit
            results.truncate(limit);
            Ok(results)
        }
    }
    
    pub async fn index_directory(&mut self, path: &Path, index_path: &Path) -> Result<()> {
        let mut engine = RipgrepSearchEngine::new(index_path)?;
        
        // Discover all files
        let file_groups = self.file_discovery.discover_files(path).await?;
        
        for group in file_groups {
            for file in group.variants {
                engine.index_file(&file).await?;
            }
        }
        
        engine.commit()?;
        self.search_engine = Some(engine);
        
        Ok(())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct AnalysisResults {
    pub total_files: usize,
    pub total_groups: usize,
    pub duplicate_results: DuplicateResults,
}


