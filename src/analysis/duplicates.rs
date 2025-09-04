use std::collections::HashMap;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use anyhow::Result;
use tokio::io::AsyncReadExt;
use crate::file_discovery::FileGroup;

/// Detects exact duplicates using SHA-256 hashing
/// This is a critical component for identifying files with identical content
/// It processes file groups and finds files that have the same hash
pub struct SmartDuplicateDetector;

impl SmartDuplicateDetector {
    /// Creates a new SmartDuplicateDetector instance
    pub fn new() -> Self {
        Self
    }
    
    /// Detects exact duplicates within file groups
    /// This is the main entry point for duplicate detection
    /// 
    /// # Arguments
    /// * `file_groups` - Groups of similar files to analyze
    /// 
    /// # Returns
    /// * `Result<DuplicateResults>` - Results containing all found duplicates
    pub async fn detect_duplicates(&self, file_groups: &[FileGroup]) -> Result<DuplicateResults> {
        let mut results = DuplicateResults::new();
        
        // Process each group of similar files
        for group in file_groups {
            // Only process groups that have multiple files (potential duplicates)
            if group.is_potential_duplicate() {
                // Find actual duplicates within this group using SHA-256 hashing
                let duplicate_sets = self.find_exact_duplicates_in_group(group).await?;
                
                // If duplicates were found, add them to the results
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
    
    /// Finds exact duplicates within a file group using SHA-256 hashing
    /// This is the core algorithm for identifying files with identical content
    /// 
    /// # Arguments
    /// * `group` - File group to analyze for duplicates
    /// 
    /// # Returns
    /// * `Result<Vec<Vec<PathBuf>>>` - List of duplicate sets (each set contains files with same content)
    async fn find_exact_duplicates_in_group(&self, group: &FileGroup) -> Result<Vec<Vec<PathBuf>>> {
        // Hash all files in the group and group by hash
        let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();
        
        for file_path in &group.variants {
            // Skip if file doesn't exist or can't be read
            if !file_path.exists() {
                log::warn!("File doesn't exist: {:?}", file_path);
                continue;
            }
            
            // Calculate SHA-256 hash of the file content
            match self.hash_file(file_path).await {
                Ok(hash) => {
                    // Group files by their hash - files with same hash are duplicates
                    hash_map.entry(hash)
                        .or_insert_with(Vec::new)
                        .push(file_path.clone());
                }
                Err(e) => {
                    // Log hash failures but continue processing other files
                    log::warn!("Failed to hash file {:?}: {}", file_path, e);
                }
            }
        }
        
        // Return only groups with 2+ files (actual duplicates)
        // Files with the same hash are considered exact duplicates
        Ok(hash_map
            .into_values()
            .filter(|group| group.len() > 1)
            .collect())
    }
    
    /// Calculates SHA-256 hash of a file for duplicate detection
    /// This is the core method for identifying files with identical content
    /// 
    /// # Arguments
    /// * `path` - Path to the file to hash
    /// 
    /// # Returns
    /// * `Result<String>` - Hexadecimal representation of the SHA-256 hash
    async fn hash_file(&self, path: &Path) -> Result<String> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut hasher = Sha256::new();
        let mut buffer = vec![0u8; 8192]; // 8KB buffer for efficient reading
        
        // Read file in chunks to handle large files efficiently
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break; // End of file reached
            }
            // Update hash with the chunk we just read
            hasher.update(&buffer[..bytes_read]);
        }
        
        // Return hexadecimal representation of the hash
        Ok(format!("{:x}", hasher.finalize()))
    }
}

/// Results of duplicate detection analysis
/// Contains statistics and all found duplicate groups
#[derive(Debug, serde::Serialize)]
pub struct DuplicateResults {
    /// Total number of duplicate files found (excluding originals)
    pub total_duplicates: usize,
    /// Total space wasted by duplicate files in bytes
    pub space_wasted: u64,
    /// All groups of duplicate files found
    pub duplicate_groups: Vec<DuplicateGroup>,
}

impl DuplicateResults {
    /// Creates a new empty DuplicateResults instance
    pub fn new() -> Self {
        Self {
            total_duplicates: 0,
            space_wasted: 0,
            duplicate_groups: Vec::new(),
        }
    }
    
    /// Adds a duplicate group and updates statistics
    /// This method calculates the total duplicates and wasted space
    /// 
    /// # Arguments
    /// * `group` - Duplicate group to add
    pub fn add_duplicate_group(&mut self, group: DuplicateGroup) {
        // Calculate statistics for this group
        for duplicate_set in &group.duplicate_sets {
            if duplicate_set.len() > 1 {
                // Count actual duplicate files (all but one original)
                self.total_duplicates += duplicate_set.len() - 1;
                
                // Calculate wasted space (size of all duplicates except the first)
                for file in duplicate_set.iter().skip(1) {
                    if let Ok(metadata) = std::fs::metadata(file) {
                        self.space_wasted += metadata.len();
                    }
                }
            }
        }
        
        // Add the group to our results
        self.duplicate_groups.push(group);
    }
}

/// Represents a group of duplicate files with the same base name
/// Each group can contain multiple sets of identical files
#[derive(Debug, serde::Serialize)]
pub struct DuplicateGroup {
    /// The base name used for grouping these files
    pub base_name: String,
    /// Each inner vec contains files that are identical to each other
    pub duplicate_sets: Vec<Vec<PathBuf>>,
}