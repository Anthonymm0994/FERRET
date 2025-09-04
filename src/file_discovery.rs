use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::Result;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

/// Main file discovery system that finds and groups files for analysis
/// This is a critical component that enables intelligent file grouping and duplicate detection
/// It combines fast file system traversal with smart grouping algorithms
pub struct FileDiscovery {
    /// Groups files by similarity using fuzzy matching
    grouper: SmartGrouper,
    /// Integrates with the `fd` command for fast file discovery
    fd_integration: FdIntegration,
}

impl FileDiscovery {
    /// Creates a new FileDiscovery instance with default settings
    pub fn new() -> Self {
        Self {
            grouper: SmartGrouper::new(),
            fd_integration: FdIntegration,
        }
    }
    
    /// Discovers all files in a directory and groups them by similarity
    /// This is the main entry point for file discovery and grouping
    /// 
    /// # Arguments
    /// * `root_path` - Directory to search for files
    /// 
    /// # Returns
    /// * `Result<Vec<FileGroup>>` - Groups of similar files
    pub async fn discover_files(&self, root_path: &Path) -> Result<Vec<FileGroup>> {
        // Use fd for fast file discovery - much faster than walkdir for large directories
        let files = self.fd_integration.find_files("*", root_path).await?;
        
        // Group files by normalized names using fuzzy matching
        // This groups files that are likely duplicates or variants
        let groups = self.grouper.group_files(files);
        
        Ok(groups)
    }
}

/// Groups files by similarity using fuzzy matching algorithms
/// This is critical for identifying potential duplicates and file variants
/// It normalizes filenames and groups files that are likely related
pub struct SmartGrouper {
    /// Fuzzy matcher for calculating string similarity
    matcher: SkimMatcherV2,
    /// Minimum similarity score for grouping files (0-100)
    threshold: i64,
}

impl SmartGrouper {
    /// Creates a new SmartGrouper with default settings
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            threshold: 60, // Fuzzy matching threshold - files must be 60% similar to group
        }
    }
    
    /// Groups files by normalized filename similarity
    /// This is the core algorithm for identifying potential duplicates
    /// 
    /// # Arguments
    /// * `files` - List of file paths to group
    /// 
    /// # Returns
    /// * `Vec<FileGroup>` - Groups of similar files
    pub fn group_files(&self, files: Vec<PathBuf>) -> Vec<FileGroup> {
        let mut groups: HashMap<String, FileGroup> = HashMap::new();
        
        // Process each file and group by similarity
        for file in files {
            // Normalize the filename to remove common variations
            // This handles things like "file_v1.txt" vs "file_v2.txt" vs "file_final.txt"
            let normalized = self.normalize_filename(&file);
            
            // Try to find existing group with similar name using fuzzy matching
            let mut found_group = false;
            for (canonical_name, group) in &mut groups {
                // Calculate similarity score between normalized names
                if let Some(score) = self.matcher.fuzzy_match(&normalized, canonical_name) {
                    if score >= self.threshold {
                        // Files are similar enough to group together
                        group.variants.push(file.clone());
                        found_group = true;
                        break;
                    }
                }
            }
            
            // If no similar group found, create a new group
            if !found_group {
                groups.insert(normalized.clone(), FileGroup {
                    canonical_name: normalized,
                    variants: vec![file],
                });
            }
        }
        
        // Convert HashMap values to Vec for return
        groups.into_values().collect()
    }
    
    /// Normalizes a filename to remove common variations and suffixes
    /// This is critical for grouping similar files that have different naming conventions
    /// 
    /// # Arguments
    /// * `path` - Path to the file to normalize
    /// 
    /// # Returns
    /// * `String` - Normalized filename for grouping
    fn normalize_filename(&self, path: &Path) -> String {
        // Extract the filename without extension
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        // Normalize the filename by:
        // 1. Converting to lowercase for case-insensitive matching
        // 2. Replacing separators with spaces for better word separation
        let normalized = stem
            .to_lowercase()
            .replace("_", " ")  // Replace underscores with spaces
            .replace("-", " "); // Replace hyphens with spaces
        
        // Remove trailing numbers and common patterns that indicate file variants
        // This regex removes version numbers, copy indicators, and common suffixes
        let cleaned = regex::Regex::new(r"\s*(v\d+|copy|backup|final|draft|\d+)$")
            .unwrap()
            .replace(&normalized, "")
            .trim()
            .to_string();
        
        // If cleaning removed everything, fall back to the original stem
        if cleaned.is_empty() {
            stem.to_lowercase()
        } else {
            cleaned
        }
    }
    
}

/// Integrates with the `fd` command for fast file discovery
/// `fd` is much faster than traditional directory traversal for large directories
/// This provides a significant performance boost when scanning many files
pub struct FdIntegration;

impl FdIntegration {
    /// Finds files using the `fd` command with fallback to walkdir
    /// This is much faster than traditional directory traversal
    /// 
    /// # Arguments
    /// * `pattern` - File pattern to search for (e.g., "*" for all files)
    /// * `root_path` - Directory to search in
    /// 
    /// # Returns
    /// * `Result<Vec<PathBuf>>` - List of found file paths
    pub async fn find_files(&self, pattern: &str, root_path: &Path) -> Result<Vec<PathBuf>> {
        // Try fd first - it's 10x faster than walkdir for large directories
        if self.is_fd_available().await {
            self.find_files_with_fd(pattern, root_path).await
        } else {
            // Fallback to walkdir if fd is not available
            // This ensures the tool works even without external dependencies
            log::warn!("fd not available, falling back to walkdir");
            self.fallback_find_files(pattern, root_path).await
        }
    }
    
    /// Checks if the `fd` command is available on the system
    /// This is used to determine whether to use fd or fallback to walkdir
    async fn is_fd_available(&self) -> bool {
        Command::new("fd")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    /// Uses the `fd` command to find files quickly
    /// This is much faster than traditional directory traversal
    /// 
    /// # Arguments
    /// * `pattern` - File pattern to search for
    /// * `root_path` - Directory to search in
    /// 
    /// # Returns
    /// * `Result<Vec<PathBuf>>` - List of found file paths
    async fn find_files_with_fd(&self, pattern: &str, root_path: &Path) -> Result<Vec<PathBuf>> {
        let output = Command::new("fd")
            .args(&[
                "--type", "f",      // Only files, not directories
                "--hidden",         // Include hidden files
                "--no-ignore",      // Don't respect .gitignore
                pattern,
                root_path.to_str().unwrap()
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("fd command failed"));
        }
        
        // Parse fd output into PathBuf list
        let output_str = String::from_utf8(output.stdout)?;
        let files: Vec<PathBuf> = output_str
            .lines()
            .map(|line| PathBuf::from(line.trim()))
            .collect();
        
        Ok(files)
    }
    
    /// Fallback method using walkdir when fd is not available
    /// This ensures the tool works even without external dependencies
    /// 
    /// # Arguments
    /// * `pattern` - File pattern to search for
    /// * `root_path` - Directory to search in
    /// 
    /// # Returns
    /// * `Result<Vec<PathBuf>>` - List of found file paths
    async fn fallback_find_files(&self, pattern: &str, root_path: &Path) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;
        use regex::Regex;
        
        // Convert glob patterns to regex patterns
        let regex_pattern = match pattern {
            "*" => ".*",  // Convert glob * to regex .*
            _ => pattern,
        };
        
        let regex = Regex::new(regex_pattern)?;
        let mut files = Vec::new();
        
        // Walk through directory tree and find matching files
        for entry in WalkDir::new(root_path)
            .into_iter()
            .filter_map(Result::ok)  // Skip entries that caused errors
        {
            if entry.file_type().is_file() {
                let path = entry.path();
                if let Some(filename) = path.file_name().and_then(|s| s.to_str()) {
                    if regex.is_match(filename) {
                        files.push(path.to_path_buf());
                    }
                }
            }
        }
        
        Ok(files)
    }
}

/// Represents a group of files that are similar or potentially duplicates
/// This is the core data structure for organizing files for analysis
pub struct FileGroup {
    /// The canonical name used for grouping (normalized filename)
    pub canonical_name: String,
    /// List of file paths that belong to this group
    pub variants: Vec<PathBuf>,
}

impl FileGroup {
    /// Checks if this group contains potential duplicates
    /// A group with more than one file is considered to have potential duplicates
    pub fn is_potential_duplicate(&self) -> bool {
        self.variants.len() > 1
    }
}
