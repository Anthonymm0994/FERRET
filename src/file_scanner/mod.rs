pub mod aging;
pub mod hashing;
pub mod similarity;

use crate::file_scanner::aging::FileAgeAnalyzer;
use crate::file_scanner::hashing::FileHasher;
use crate::file_scanner::similarity::FileSimilarity;
use std::collections::{HashSet, HashMap};
use std::path::{Path, PathBuf};

pub struct FileScanner {
    pub root_directory: PathBuf,
    pub unique_files: HashSet<PathBuf>,
    pub duplicate_files: HashSet<PathBuf>,   // Track all duplicate files
    pub duplicate_groups: Vec<Vec<PathBuf>>, // Group duplicates for report display
    pub similarity_tool: FileSimilarity,
    pub recent_files: usize,
    pub stale_files: usize,
    pub old_files: usize,
}

impl FileScanner {
    pub fn new(root_directory: impl AsRef<Path>) -> Self {
        FileScanner {
            root_directory: root_directory.as_ref().to_path_buf(),
            unique_files: HashSet::new(),
            duplicate_files: HashSet::new(),
            duplicate_groups: Vec::new(),
            similarity_tool: FileSimilarity::new(),
            recent_files: 0,
            stale_files: 0,
            old_files: 0,
        }
    }

    pub async fn run_all_analyses(&mut self) {
        println!("Starting Duplicate Detection...");
        self.detect_duplicates();

        println!("\nStarting Aging Analysis...");
        self.run_aging_analysis();

        println!("\nStarting Similarity Scoring...");
        self.run_similarity_scoring().await;
    }

    fn detect_duplicates(&mut self) {
        let mut file_hasher = FileHasher::new();
        if let Ok(_) = file_hasher.scan_directory(&self.root_directory) {
            for group in file_hasher.get_duplicates() {
                self.duplicate_groups.push(group.clone());
                self.duplicate_files.extend(group); // Add all items in each group to duplicate_files
            }
        }
    }

    fn run_aging_analysis(&mut self) {
        let mut file_age_analyzer = FileAgeAnalyzer::new(30, 180);
        if let Ok(_) = file_age_analyzer.analyze_directory(&self.root_directory) {
            let (recent, stale, old) = file_age_analyzer.summary();
            self.recent_files = recent;
            self.stale_files = stale;
            self.old_files = old;
        }
    }

    async fn run_similarity_scoring(&mut self) {
        for entry in walkdir::WalkDir::new(&self.root_directory) {
            let path = entry.unwrap().into_path();
            if path.is_file() && !self.duplicate_files.contains(&path) {
                self.unique_files.insert(path);
            }
        }
        let files: Vec<PathBuf> = self.unique_files.iter().cloned().collect();
        self.similarity_tool.calculate_similarity(files, &self.duplicate_files).await;
    }

    pub fn file_age_summary(&self) -> (usize, usize, usize) {
        (self.recent_files, self.stale_files, self.old_files)
    }

    pub fn total_files_count(&self) -> usize {
        self.unique_files.len() + self.duplicate_files.len()
    }

    // Collect directory structure for interactive display in report
    pub fn collect_directory_structure(&self) -> Vec<(String, Vec<String>)> {
        let mut structure = Vec::new();
        let mut directories: HashMap<String, Vec<String>> = HashMap::new();

        for path in self.unique_files.iter().chain(self.duplicate_files.iter()) {
            let parent_dir = path.parent().unwrap_or_else(|| path.as_path()).display().to_string();
            directories.entry(parent_dir).or_insert_with(Vec::new).push(path.display().to_string());
        }

        for (dir, files) in directories {
            structure.push((dir, files));
        }
        structure
    }

    pub fn grouped_duplicates(&self) -> Vec<Vec<PathBuf>> {
        self.duplicate_groups.clone()
    }
}
