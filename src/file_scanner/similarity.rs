use async_std::fs::File;
use async_std::prelude::*;
use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::path::{PathBuf};
use std::sync::{Arc, Mutex};

// Similarity parameters
const SIMILARITY_THRESHOLD: f64 = 0.7; // Back to 70% threshold
const MIN_SHINGLE_SIZE: usize = 2;     // Minimum shingle size for small files
const MAX_SHINGLE_SIZE: usize = 10;    // Maximum shingle size for large files
const MATCH_SCORE: i32 = 3;
const MISMATCH_SCORE: i32 = -1;
const GAP_PENALTY: i32 = -2;
const MAX_FILE_SIZE: usize = 100 * 1024; // Max file size for detailed comparison

pub struct FileSimilarity {
    pub similarity_scores: Arc<Mutex<HashMap<(PathBuf, PathBuf), f64>>>,
}

impl FileSimilarity {
    pub fn new() -> Self {
        FileSimilarity {
            similarity_scores: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    // Async file reading with early filtering
    pub async fn load_files(&self, files: Vec<PathBuf>) -> Vec<(PathBuf, String)> {
        let mut file_contents = Vec::new();
        for file_path in files {
            if let Ok(mut file) = File::open(&file_path).await {
                let mut contents = String::new();
                file.read_to_string(&mut contents).await.unwrap();
                if self.is_relevant_size(&contents) {
                    println!("Loaded file: {:?} with size: {}", file_path, contents.len()); // Diagnostic log
                    file_contents.push((file_path, contents));
                }
            }
        }
        file_contents
    }

    // Filter based on file size heuristics
    fn is_relevant_size(&self, content: &str) -> bool {
        content.len() <= MAX_FILE_SIZE
    }

    // Determine shingle size dynamically based on file length
    fn determine_shingle_size(&self, content_length: usize) -> usize {
        let scaled_shingle_size = content_length / 50;
        scaled_shingle_size.clamp(MIN_SHINGLE_SIZE, MAX_SHINGLE_SIZE)
    }

    // Shingle files for approximate match, using dynamic shingle sizing
    fn shingle_file(&self, content: &str) -> HashSet<String> {
        let shingle_size = self.determine_shingle_size(content.len());
        let words: Vec<&str> = content.split_whitespace().collect();
        let mut shingles = HashSet::new();

        if words.len() < shingle_size {
            return shingles;
        }

        for i in 0..words.len().saturating_sub(shingle_size) {
            shingles.insert(words[i..i + shingle_size].join(" "));
        }
        shingles
    }

    // Approximate similarity with shingle-based Jaccard similarity
    fn shingle_similarity(&self, shingles1: &HashSet<String>, shingles2: &HashSet<String>) -> f64 {
        if shingles1.is_empty() || shingles2.is_empty() {
            return 0.0;
        }
        
        let intersection: HashSet<_> = shingles1.intersection(shingles2).collect();
        let union: HashSet<_> = shingles1.union(shingles2).collect();
        intersection.len() as f64 / union.len() as f64
    }

    // Full similarity process: combines shingle filtering and Smith-Waterman
    pub async fn calculate_similarity(&self, files: Vec<PathBuf>, duplicate_files: &HashSet<PathBuf>) {
        let file_contents = self.load_files(files).await;

        file_contents.par_iter().enumerate().for_each(|(i, (path1, content1))| {
            let shingles1 = self.shingle_file(content1);
            for (path2, content2) in file_contents.iter().skip(i + 1) {
                let shingles2 = self.shingle_file(content2);
                
                // Check if both files are in the duplicate list and assign 1.0 if so
                if duplicate_files.contains(path1) && duplicate_files.contains(path2) {
                    let mut scores = self.similarity_scores.lock().unwrap();
                    scores.insert((path1.clone(), path2.clone()), 1.0);
                    continue;
                }

                // Calculate the Jaccard similarity score if not exact duplicates
                let shingle_score = self.shingle_similarity(&shingles1, &shingles2);

                println!("Jaccard similarity between {:?} and {:?} = {:.2}", path1, path2, shingle_score); // Diagnostic log

                if shingle_score >= SIMILARITY_THRESHOLD {
                    let smith_waterman_score = self.smith_waterman_rolling(content1, content2);
                    println!("Smith-Waterman similarity between {:?} and {:?} = {:.2}", path1, path2, smith_waterman_score); // Diagnostic log
                    
                    let mut scores = self.similarity_scores.lock().unwrap();
                    scores.insert((path1.clone(), path2.clone()), smith_waterman_score);
                }
            }
        });
    }

    // Smith-Waterman algorithm with rolling matrix to reduce memory usage
    fn smith_waterman_rolling(&self, seq1: &str, seq2: &str) -> f64 {
        let rows = seq1.len() + 1;
        let cols = seq2.len() + 1;

        let mut prev_row = vec![0; cols];
        let mut current_row = vec![0; cols];
        let mut max_score = 0;

        for i in 1..rows {
            for j in 1..cols {
                let match_mismatch = if seq1.chars().nth(i - 1) == seq2.chars().nth(j - 1) {
                    MATCH_SCORE
                } else {
                    MISMATCH_SCORE
                };

                current_row[j] = std::cmp::max(
                    0,
                    std::cmp::max(
                        prev_row[j - 1] + match_mismatch,
                        std::cmp::max(
                            prev_row[j] + GAP_PENALTY,
                            current_row[j - 1] + GAP_PENALTY,
                        ),
                    ),
                );

                max_score = std::cmp::max(max_score, current_row[j]);
            }
            std::mem::swap(&mut prev_row, &mut current_row);
        }

        max_score as f64 / (seq1.len().max(seq2.len()) as f64 * MATCH_SCORE as f64) // Normalize
    }

    // Retrieve pairs with similarity scores above the threshold
    pub fn get_similar_files(&self) -> Vec<((PathBuf, PathBuf), f64)> {
        let scores = self.similarity_scores.lock().unwrap();
        scores
            .iter()
            .map(|(files, &score)| (files.clone(), score))
            .collect()
    }
}
