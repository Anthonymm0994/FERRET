// src/file_scanner/aging.rs
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, Duration};

pub struct FileAgeAnalyzer {
    pub recent_files: Vec<PathBuf>,
    pub stale_files: Vec<PathBuf>,
    pub old_files: Vec<PathBuf>,
    recent_threshold: Duration,
    stale_threshold: Duration,
}

impl FileAgeAnalyzer {
    // Initialize with custom thresholds (or use defaults)
    pub fn new(recent_days: u64, stale_days: u64) -> Self {
        FileAgeAnalyzer {
            recent_files: Vec::new(),
            stale_files: Vec::new(),
            old_files: Vec::new(),
            recent_threshold: Duration::from_secs(recent_days * 86400),
            stale_threshold: Duration::from_secs(stale_days * 86400),
        }
    }

    // Analyze the age of files in a directory (recursive)
    pub fn analyze_directory<P: AsRef<Path>>(&mut self, dir: P) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                self.categorize_file_age(&path)?;
            } else if path.is_dir() {
                // Recurse into subdirectories
                self.analyze_directory(&path)?;
            }
        }
        Ok(())
    }

    // Categorize a file based on its last modified time
    fn categorize_file_age(&mut self, file_path: &Path) -> io::Result<()> {
        let metadata = fs::metadata(file_path)?;
        let modified = metadata.modified()?;
        let now = SystemTime::now();

        // Calculate the file's age in seconds
        if let Ok(age) = now.duration_since(modified) {
            if age <= self.recent_threshold {
                self.recent_files.push(file_path.to_path_buf());
            } else if age <= self.stale_threshold {
                self.stale_files.push(file_path.to_path_buf());
            } else {
                self.old_files.push(file_path.to_path_buf());
            }
        }
        Ok(())
    }

    // Get a summary of categorized files
    pub fn summary(&self) -> (usize, usize, usize) {
        (
            self.recent_files.len(),
            self.stale_files.len(),
            self.old_files.len(),
        )
    }
}
