use sha2::{Sha256, Digest};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;  // Import WalkDir for recursive traversal

pub struct FileHasher {
    pub duplicates: HashMap<String, Vec<PathBuf>>,
}

impl FileHasher {
    pub fn new() -> Self {
        FileHasher {
            duplicates: HashMap::new(),
        }
    }

    // Calculate SHA-256 hash of a file
    pub fn calculate_hash<P: AsRef<Path>>(&self, file_path: P) -> io::Result<String> {
        let mut file = fs::File::open(&file_path)?;
        let mut hasher = Sha256::new();
        let mut buffer = Vec::new();

        file.read_to_end(&mut buffer)?;
        hasher.update(buffer);

        Ok(format!("{:x}", hasher.finalize()))
    }

    // Scan the directory and subdirectories for duplicate files
    pub fn scan_directory<P: AsRef<Path>>(&mut self, dir: P) -> io::Result<()> {
        for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
            let path = entry.path().to_path_buf();

            if path.is_file() {
                match self.calculate_hash(&path) {
                    Ok(hash) => {
                        // Add file path to the list of duplicates based on hash
                        self.duplicates.entry(hash).or_insert_with(Vec::new).push(path);
                    }
                    Err(e) => eprintln!("Error hashing file {:?}: {}", path, e),
                }
            }
        }
        Ok(())
    }

    // Retrieve detected duplicate files (files with the same hash)
    pub fn get_duplicates(&self) -> Vec<Vec<PathBuf>> {
        self.duplicates
            .values()
            .filter(|paths| paths.len() > 1)
            .cloned()
            .collect()
    }
}
