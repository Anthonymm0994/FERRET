use std::path::Path;
use anyhow::Result;
use zip::ZipArchive;
use std::io::Read;

pub struct ArchiveProcessor;

impl ArchiveProcessor {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn check_archives_for_duplicates(&self, archive_path: &Path) -> Result<Vec<ArchiveDuplicateInfo>> {
        let extension = archive_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match extension.as_str() {
            "zip" => self.process_zip_archive(archive_path).await,
            "rar" => self.process_rar_archive(archive_path).await,
            "7z" => self.process_7z_archive(archive_path).await,
            _ => Ok(Vec::new()),
        }
    }
    
    async fn process_zip_archive(&self, path: &Path) -> Result<Vec<ArchiveDuplicateInfo>> {
        let file = tokio::fs::File::open(path).await?;
        let mut archive = ZipArchive::new(file.into_std().await)?;
        let mut duplicates = Vec::new();
        
        for i in 0..archive.len() {
            let file = archive.by_index(i)?;
            let file_name = file.name().to_string();
            
            if self.is_document_file(&file_name) {
                // Extract and analyze the file
                let mut content = String::new();
                let mut reader = file;
                reader.read_to_string(&mut content)?;
                
                // Here you would check for duplicates against your main file database
                // For now, just collect the information
                duplicates.push(ArchiveDuplicateInfo {
                    archive_path: path.to_path_buf(),
                    file_name,
                    content_length: content.len(),
                    is_duplicate: false, // Would be determined by actual duplicate checking
                });
            }
        }
        
        Ok(duplicates)
    }
    
    async fn process_rar_archive(&self, path: &Path) -> Result<Vec<ArchiveDuplicateInfo>> {
        // RAR processing requires external tools
        log::warn!("RAR archive processing not implemented: {:?}", path);
        log::info!("Consider installing unrar or 7zip for RAR support");
        Ok(Vec::new())
    }
    
    async fn process_7z_archive(&self, path: &Path) -> Result<Vec<ArchiveDuplicateInfo>> {
        // 7z processing requires external tools
        log::warn!("7z archive processing not implemented: {:?}", path);
        log::info!("Consider installing 7zip for 7z support");
        Ok(Vec::new())
    }
    
    fn is_document_file(&self, filename: &str) -> bool {
        let ext = std::path::Path::new(filename)
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        matches!(ext.as_str(), 
            "txt" | "md" | "rst" | "doc" | "docx" | "pdf" | "xls" | "xlsx" | 
            "ppt" | "pptx" | "rtf" | "odt" | "ods" | "odp"
        )
    }
}

pub struct ArchiveDuplicateInfo {
    pub archive_path: std::path::PathBuf,
    pub file_name: String,
    pub content_length: usize,
    pub is_duplicate: bool,
}
