use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use anyhow::Result;
use tokio::time::timeout;

pub struct NetworkAwareIO {
    cache: HashMap<PathBuf, (FileContent, Instant)>,
    cache_ttl: Duration,
    network_timeout: Duration,
    local_timeout: Duration,
}

impl Default for NetworkAwareIO {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            cache_ttl: Duration::from_secs(300), // 5 minutes
            network_timeout: Duration::from_secs(30), // 30 seconds for network
            local_timeout: Duration::from_secs(5), // 5 seconds for local
        }
    }
}

impl NetworkAwareIO {
    pub async fn read_file_content(&self, path: &Path) -> Result<FileContent> {
        // Check cache first
        if let Some(cached) = self.get_cached_metadata(path) {
            return Ok(cached);
        }
        
        if self.is_network_path(path) {
            self.process_network_file(path).await
        } else {
            self.process_local_file(path).await
        }
    }
    
    async fn process_network_file(&self, path: &Path) -> Result<FileContent> {
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();
        
        // Adaptive reading based on file size
        let content = if file_size > 100 * 1024 * 1024 { // 100MB
            // Too large - just get metadata
            FileContent::TooLarge {
                size: file_size,
                modified: metadata.modified()?,
            }
        } else if file_size > 10 * 1024 * 1024 { // 10MB
            // Large file - get preview
            let preview = timeout(self.network_timeout, self.quick_sample(path)).await??;
            FileContent::Preview {
                content: preview,
                size: file_size,
                modified: metadata.modified()?,
            }
        } else {
            // Small file - read full content
            let content = timeout(self.network_timeout, tokio::fs::read_to_string(path)).await??;
            FileContent::Full {
                content,
                size: file_size,
                modified: metadata.modified()?,
            }
        };
        
        Ok(content)
    }
    
    async fn process_local_file(&self, path: &Path) -> Result<FileContent> {
        let metadata = tokio::fs::metadata(path).await?;
        let file_size = metadata.len();
        
        let content = if file_size > 50 * 1024 * 1024 { // 50MB
            FileContent::TooLarge {
                size: file_size,
                modified: metadata.modified()?,
            }
        } else {
            let content = timeout(self.local_timeout, tokio::fs::read_to_string(path)).await??;
            FileContent::Full {
                content,
                size: file_size,
                modified: metadata.modified()?,
            }
        };
        
        Ok(content)
    }
    
    fn is_network_path(&self, path: &Path) -> bool {
        // Check for UNC paths (Windows network drives)
        if let Some(path_str) = path.to_str() {
            path_str.starts_with("\\\\") || path_str.starts_with("//")
        } else {
            false
        }
    }
    
    async fn quick_sample(&self, path: &Path) -> Result<String> {
        let mut file = tokio::fs::File::open(path).await?;
        let mut buffer = vec![0u8; 1024]; // Read first 1KB
        
        use tokio::io::AsyncReadExt;
        let bytes_read = file.read(&mut buffer).await?;
        buffer.truncate(bytes_read);
        
        Ok(String::from_utf8_lossy(&buffer).to_string())
    }
    
    fn get_cached_metadata(&self, path: &Path) -> Option<FileContent> {
        self.cache.get(path)
            .filter(|(_, timestamp)| timestamp.elapsed() < self.cache_ttl)
            .map(|(content, _)| content.clone())
    }
}

#[derive(Debug, Clone)]
pub enum FileContent {
    Full {
        content: String,
        size: u64,
        modified: std::time::SystemTime,
    },
    Preview {
        content: String,
        size: u64,
        modified: std::time::SystemTime,
    },
    TooLarge {
        size: u64,
        modified: std::time::SystemTime,
    },
    NetworkUnavailable {
        size: u64,
        modified: std::time::SystemTime,
    },
}
