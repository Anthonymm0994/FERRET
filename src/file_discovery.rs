use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use anyhow::Result;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

pub struct FileDiscovery {
    grouper: SmartGrouper,
    fd_integration: FdIntegration,
}

impl FileDiscovery {
    pub fn new() -> Self {
        Self {
            grouper: SmartGrouper::new(),
            fd_integration: FdIntegration,
        }
    }
    
    pub async fn discover_files(&self, root_path: &Path) -> Result<Vec<FileGroup>> {
        // Use fd for fast file discovery
        let files = self.fd_integration.find_files("*", root_path).await?;
        
        // Group files by normalized names
        let groups = self.grouper.group_files(files);
        
        Ok(groups)
    }
}

pub struct SmartGrouper {
    matcher: SkimMatcherV2,
    threshold: i64,
}

impl SmartGrouper {
    pub fn new() -> Self {
        Self {
            matcher: SkimMatcherV2::default(),
            threshold: 60, // Fuzzy matching threshold
        }
    }
    
    pub fn group_files(&self, files: Vec<PathBuf>) -> Vec<FileGroup> {
        let mut groups: HashMap<String, FileGroup> = HashMap::new();
        
        for file in files {
            let normalized = self.normalize_filename(&file);
            
            // Try to find existing group with similar name
            let mut found_group = false;
            for (canonical_name, group) in &mut groups {
                if let Some(score) = self.matcher.fuzzy_match(&normalized, canonical_name) {
                    if score >= self.threshold {
                        group.variants.push(file.clone());
                        found_group = true;
                        break;
                    }
                }
            }
            
            if !found_group {
                groups.insert(normalized.clone(), FileGroup {
                    canonical_name: normalized,
                    variants: vec![file],
                });
            }
        }
        
        groups.into_values().collect()
    }
    
    fn normalize_filename(&self, path: &Path) -> String {
        let stem = path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        
        // For now, use a simpler approach that keeps more of the filename
        // Remove common suffixes like numbers, but keep the base name
        let normalized = stem
            .to_lowercase()
            .replace("_", " ")
            .replace("-", " ");
        
        // Remove trailing numbers and common patterns
        let cleaned = regex::Regex::new(r"\s*(v\d+|copy|backup|final|draft|\d+)$")
            .unwrap()
            .replace(&normalized, "")
            .trim()
            .to_string();
        
        if cleaned.is_empty() {
            stem.to_lowercase()
        } else {
            cleaned
        }
    }
    
    fn tokenize_filename(&self, filename: &str) -> Vec<String> {
        // Split on common separators and extract meaningful tokens
        regex::Regex::new(r"[-_\s]+")
            .unwrap()
            .split(filename)
            .map(|s| s.to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
    
    fn calculate_entropy(&self, segment: &str) -> f64 {
        // Calculate Shannon entropy to identify variable vs stable segments
        let mut counts = std::collections::HashMap::new();
        for ch in segment.chars() {
            *counts.entry(ch).or_insert(0) += 1;
        }
        
        let len = segment.len() as f64;
        counts.values()
            .map(|&count| {
                let p = count as f64 / len;
                -p * p.log2()
            })
            .sum()
    }
}

pub struct FdIntegration;

impl FdIntegration {
    pub async fn find_files(&self, pattern: &str, root_path: &Path) -> Result<Vec<PathBuf>> {
        // Try fd first - it's 10x faster than walkdir
        if self.is_fd_available().await {
            self.find_files_with_fd(pattern, root_path).await
        } else {
            // Fallback to walkdir
            log::warn!("fd not available, falling back to walkdir");
            self.fallback_find_files(pattern, root_path).await
        }
    }
    
    async fn is_fd_available(&self) -> bool {
        Command::new("fd")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    async fn find_files_with_fd(&self, pattern: &str, root_path: &Path) -> Result<Vec<PathBuf>> {
        let output = Command::new("fd")
            .args(&[
                "--type", "f",
                "--hidden",
                "--no-ignore",
                pattern,
                root_path.to_str().unwrap()
            ])
            .output()?;
        
        if !output.status.success() {
            return Err(anyhow::anyhow!("fd command failed"));
        }
        
        let output_str = String::from_utf8(output.stdout)?;
        let files: Vec<PathBuf> = output_str
            .lines()
            .map(|line| PathBuf::from(line.trim()))
            .collect();
        
        Ok(files)
    }
    
    async fn fallback_find_files(&self, pattern: &str, root_path: &Path) -> Result<Vec<PathBuf>> {
        use walkdir::WalkDir;
        use regex::Regex;
        
        // Handle special patterns
        let regex_pattern = match pattern {
            "*" => ".*",  // Convert glob * to regex .*
            _ => pattern,
        };
        
        let regex = Regex::new(regex_pattern)?;
        let mut files = Vec::new();
        
        for entry in WalkDir::new(root_path)
            .into_iter()
            .filter_map(Result::ok)
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

pub struct FileGroup {
    pub canonical_name: String,
    pub variants: Vec<PathBuf>,
}

impl FileGroup {
    pub fn get_potential_duplicates(&self) -> Vec<&PathBuf> {
        if self.variants.len() > 1 {
            self.variants.iter().collect()
        } else {
            Vec::new()
        }
    }
    
    pub fn is_potential_duplicate(&self) -> bool {
        self.variants.len() > 1
    }
}
