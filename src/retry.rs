use std::path::PathBuf;
use std::time::{Duration, Instant};
use anyhow::Result;
use tokio::time::sleep;

pub struct RetryManager {
    locked_files: Vec<LockedFile>,
    max_retries: usize,
    retry_delay: Duration,
}

struct LockedFile {
    path: PathBuf,
    error: String,
    first_seen: Instant,
    retry_count: usize,
}

impl Default for RetryManager {
    fn default() -> Self {
        Self {
            locked_files: Vec::new(),
            max_retries: 3,
            retry_delay: Duration::from_secs(5),
        }
    }
}

impl RetryManager {
    pub fn add_locked_file(&mut self, path: PathBuf, error: String) {
        self.locked_files.push(LockedFile {
            path,
            error,
            first_seen: Instant::now(),
            retry_count: 0,
        });
    }
    
    pub async fn retry_locked_files<F, Fut, T>(&mut self, mut processor: F) -> Vec<ProcessResult>
    where
        F: FnMut(PathBuf) -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut results = Vec::new();
        let mut to_retry = Vec::new();
        
        // Move files to retry queue
        std::mem::swap(&mut self.locked_files, &mut to_retry);
        
        for mut locked_file in to_retry {
            if locked_file.retry_count >= self.max_retries {
                results.push(ProcessResult::Failed {
                    path: locked_file.path,
                    error: format!("Max retries exceeded: {}", locked_file.error),
                });
                continue;
            }
            
            // Check if file is still locked
            if is_file_locked(&locked_file.path) {
                locked_file.retry_count += 1;
                self.locked_files.push(locked_file);
                continue;
            }
            
            // Try to process the file
            match processor(locked_file.path.clone()).await {
                Ok(_) => {
                    results.push(ProcessResult::Success {
                        path: locked_file.path,
                    });
                }
                Err(e) => {
                    if self.is_lock_error(&e) {
                        locked_file.retry_count += 1;
                        self.locked_files.push(locked_file);
                    } else {
                        results.push(ProcessResult::Failed {
                            path: locked_file.path,
                            error: e.to_string(),
                        });
                    }
                }
            }
        }
        
        // Wait before next retry cycle
        if !self.locked_files.is_empty() {
            sleep(self.retry_delay).await;
        }
        
        results
    }
    
    fn is_lock_error(&self, error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        error_str.contains("access denied") ||
        error_str.contains("being used by another process") ||
        error_str.contains("sharing violation") ||
        error_str.contains("file is locked")
    }
}

#[cfg(windows)]
pub fn is_file_locked(path: &std::path::Path) -> bool {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;
    use winapi::um::winnt::FILE_SHARE_READ;
    
    match OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
    {
        Ok(_) => false, // File is not locked
        Err(_) => true, // File is locked or doesn't exist
    }
}

#[cfg(not(windows))]
pub fn is_file_locked(_path: &std::path::Path) -> bool {
    // On non-Windows systems, we can't easily detect file locks
    // Return false to assume files are not locked
    false
}

fn path_to_wide_string(path: &std::path::Path) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;
    
    path.as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[derive(Debug)]
pub enum ProcessResult {
    Success { path: PathBuf },
    Failed { path: PathBuf, error: String },
}
