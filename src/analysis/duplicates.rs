use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use anyhow::Result;
use tokio::io::AsyncReadExt;
use crate::io::network_aware::NetworkAwareIO;
use crate::file_discovery::FileGroup;

pub struct SmartDuplicateDetector {
    io_adapter: NetworkAwareIO,
    fuzzy_threshold: f32,
}

impl SmartDuplicateDetector {
    pub fn new() -> Self {
        Self {
            io_adapter: NetworkAwareIO::default(),
            fuzzy_threshold: 0.8, // 80% similarity threshold
        }
    }
    
    pub async fn detect_duplicates(&self, file_groups: &[FileGroup]) -> Result<DuplicateResults> {
        let mut results = DuplicateResults::new();
        
        for group in file_groups {
            if group.is_potential_duplicate() {
                // Find actual duplicates within this group
                let duplicate_sets = self.find_exact_duplicates_in_group(group).await?;
                
                if !duplicate_sets.is_empty() {
                    results.add_duplicate_group(DuplicateGroup {
                        base_name: group.canonical_name.clone(),
                        duplicate_sets,
                    });
                }
            }
        }
        
        Ok(results)
    }
    
    async fn find_exact_duplicates_in_group(&self, group: &FileGroup) -> Result<Vec<Vec<PathBuf>>> {
        // Hash all files in the group
        let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
        
        for file_path in &group.variants {
            // Skip if file doesn't exist or can't be read
            if !file_path.exists() {
                log::warn!("File doesn't exist: {:?}", file_path);
                continue;
            }
            
            match self.hash_file(file_path).await {
                Ok(hash) => {
                    hash_map.entry(hash)
                        .or_insert_with(Vec::new)
                        .push(file_path.clone());
                }
                Err(e) => {
                    log::warn!("Failed to hash file {:?}: {}", file_path, e);
                }
            }
        }
        
        // Return only groups with 2+ files (actual duplicates)
        Ok(hash_map
            .into_values()
            .filter(|group| group.len() > 1)
            .collect())
    }
    
    async fn hash_file(&self, path: &Path) -> Result<String> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    // Legacy method for compatibility - now calls the new implementation
    pub async fn analyze_group(&self, group: &FileGroup) -> Result<DuplicateGroup> {
        let duplicate_sets = self.find_exact_duplicates_in_group(group).await?;
        
        Ok(DuplicateGroup {
            base_name: group.canonical_name.clone(),
            duplicate_sets,
        })
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DuplicateResults {
    pub total_duplicates: usize,
    pub space_wasted: u64,
    pub duplicate_groups: Vec<DuplicateGroup>,
}

impl DuplicateResults {
    pub fn new() -> Self {
        Self {
            total_duplicates: 0,
            space_wasted: 0,
            duplicate_groups: Vec::new(),
        }
    }
    
    pub fn add_duplicate_group(&mut self, group: DuplicateGroup) {
        // Calculate stats
        for duplicate_set in &group.duplicate_sets {
            if duplicate_set.len() > 1 {
                // Count actual duplicate files (all but one original)
                self.total_duplicates += duplicate_set.len() - 1;
                
                // Calculate wasted space
                for file in duplicate_set.iter().skip(1) {
                    if let Ok(metadata) = std::fs::metadata(file) {
                        self.space_wasted += metadata.len();
                    }
                }
            }
        }
        
        self.duplicate_groups.push(group);
    }
}

#[derive(Debug, serde::Serialize)]
pub struct DuplicateGroup {
    pub base_name: String,
    pub duplicate_sets: Vec<Vec<PathBuf>>,  // Each inner vec contains identical files
}