use std::path::Path;
use anyhow::Result;
use std::process::Command;

pub struct DocumentExtractor;

impl DocumentExtractor {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn extract_content(&self, path: &Path) -> Result<String> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        match extension.as_str() {
            "docx" => self.extract_docx(path).await,
            "pdf" => self.extract_pdf(path).await,
            "xlsx" => self.extract_xlsx(path).await,
            "pptx" => self.extract_pptx(path).await,
            "txt" | "md" | "rst" | "log" => {
                tokio::fs::read_to_string(path).await.map_err(Into::into)
            }
            _ => {
                // Try to read as text, fall back to binary indicator
                match tokio::fs::read_to_string(path).await {
                    Ok(content) => Ok(content),
                    Err(_) => Ok(format!("[Binary file: {}]", path.display())),
                }
            }
        }
    }
    
    async fn extract_docx(&self, path: &Path) -> Result<String> {
        // DOCX extraction temporarily disabled due to dependency issues
        log::info!("DOCX extraction not yet implemented for: {}", path.display());
        Ok(format!("[DOCX file: {} - content extraction not implemented]", path.display()))
    }
    
    async fn extract_pdf(&self, path: &Path) -> Result<String> {
        // Try lopdf first
        match self.extract_pdf_with_lopdf(path).await {
            Ok(content) if !content.trim().is_empty() => Ok(content),
            _ => {
                // Fallback to pdftotext
                self.try_pdftotext(path).await
            }
        }
    }
    
    async fn extract_pdf_with_lopdf(&self, path: &Path) -> Result<String> {
        // lopdf temporarily disabled due to compilation issues
        log::info!("PDF extraction with lopdf not available for: {}", path.display());
        Ok(String::new())
    }
    
    async fn try_pdftotext(&self, path: &Path) -> Result<String> {
        let output = Command::new("pdftotext")
            .args(&[path.to_str().unwrap(), "-"]) // Read from file, write to stdout
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            log::warn!("pdftotext failed for {}: {}", path.display(), String::from_utf8_lossy(&output.stderr));
            Ok(format!("[Failed to extract PDF content: pdftotext not available]"))
        }
    }
    
    async fn extract_xlsx(&self, path: &Path) -> Result<String> {
        // XLSX extraction temporarily simplified
        log::info!("XLSX extraction not yet fully implemented for: {}", path.display());
        Ok(format!("[XLSX file: {} - content extraction not implemented]", path.display()))
    }
    
    async fn extract_pptx(&self, path: &Path) -> Result<String> {
        // PowerPoint extraction is complex, for now return a placeholder
        log::info!("PowerPoint extraction not yet implemented for: {}", path.display());
        Ok(format!("[PowerPoint file: {} - content extraction not implemented]", path.display()))
    }
}
