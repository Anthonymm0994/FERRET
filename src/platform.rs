use std::path::Path;
use anyhow::Result;
use futures::future::join_all;
use std::sync::Arc;

use crate::file_discovery::FileDiscovery;
use crate::analysis::duplicates::{SmartDuplicateDetector, DuplicateResults};
use crate::search::engine::{RipgrepSearchEngine, SearchResult};
use crate::retry::RetryManager;

pub struct FerretPlatform {
    file_discovery: FileDiscovery,
    duplicate_detector: SmartDuplicateDetector,
    search_engine: Option<RipgrepSearchEngine>,
    retry_manager: RetryManager,
}

impl FerretPlatform {
    pub fn new() -> Result<Self> {
        Ok(Self {
            file_discovery: FileDiscovery::new(),
            duplicate_detector: SmartDuplicateDetector::new(),
            search_engine: None,
            retry_manager: RetryManager::default(),
        })
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
    
    pub async fn analyze_directory_parallel(&mut self, path: &Path) -> Result<AnalysisResults> {
        let file_groups = self.file_discovery.discover_files(path).await?;
        let potential_duplicates = file_groups.iter()
            .flat_map(|group| group.get_potential_duplicates())
            .collect::<Vec<_>>();
        
        // Create progress bar for long operations
        let progress = indicatif::ProgressBar::new(potential_duplicates.len() as u64);
        progress.set_style(
            indicatif::ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})")
                .unwrap()
                .progress_chars("#>-")
        );
        progress.set_message("Analyzing file groups for duplicates...");
        
        // Process file groups in parallel using tokio (NO rayon!)
        let detector = Arc::new(SmartDuplicateDetector::new());
        let progress = Arc::new(progress);
        
        let handles: Vec<_> = file_groups
            .into_iter()
            .map(|group| {
                let detector = detector.clone();
                let progress = progress.clone();
                tokio::spawn(async move {
                    let result = detector.analyze_group(&group).await;
                    progress.inc(1);
                    result
                })
            })
            .collect();
        
        let results = join_all(handles).await;
        progress.finish_with_message("Analysis complete!");
        
        // Aggregate results
        let mut duplicate_groups = Vec::new();
        let mut total_duplicates = 0;
        let mut space_wasted = 0u64;
        
        for result in results {
            if let Ok(Ok(duplicate_group)) = result {
                // Count duplicates in each set
                for duplicate_set in &duplicate_group.duplicate_sets {
                    if duplicate_set.len() > 1 {
                        total_duplicates += duplicate_set.len() - 1;
                        // Calculate wasted space
                        for file in duplicate_set.iter().skip(1) {
                            if let Ok(metadata) = std::fs::metadata(file) {
                                space_wasted += metadata.len();
                            }
                        }
                    }
                }
                duplicate_groups.push(duplicate_group);
            }
        }
        
        let total_groups = duplicate_groups.len();
        let duplicate_results = DuplicateResults {
            total_duplicates,
            space_wasted,
            duplicate_groups,
        };
        
        Ok(AnalysisResults {
            total_files: total_duplicates,
            total_groups,
            duplicate_results,
        })
    }
    
    pub async fn search(&mut self, query: &str, path: &Path, limit: usize) -> Result<Vec<SearchResult>> {
        if let Some(ref engine) = self.search_engine {
            engine.search(query, limit)
        } else {
            // Fallback to ripgrep integration
            let integration = crate::search::engine::RipgrepIntegration;
            integration.search_with_ripgrep(query, path)
        }
    }
    
    pub async fn index_directory(&mut self, path: &Path, index_path: &Path) -> Result<()> {
        let mut engine = RipgrepSearchEngine::new(index_path)?;
        
        // Discover all files
        let file_groups = self.file_discovery.discover_files(path).await?;
        
        for group in file_groups {
            for file in group.variants {
                let metadata = crate::search::engine::FileMetadata {
                    size: std::fs::metadata(&file)?.len(),
                    modified: std::fs::metadata(&file)?.modified()?,
                    is_binary: false, // Would need proper binary detection
                };
                
                engine.index_file(&file, &metadata).await?;
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


