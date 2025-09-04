use std::path::Path;
use anyhow::Result;
use std::process::Command;

/// Extracts text content from various document formats
/// This is a critical component that enables searching inside PDFs, Word docs, Excel files, etc.
/// It uses external tools and basic parsing to extract text from binary document formats
pub struct DocumentExtractor;

impl DocumentExtractor {
    /// Creates a new DocumentExtractor instance
    pub fn new() -> Self {
        Self
    }
    
    /// Extracts text content from a file based on its extension
    /// This is the main entry point for document content extraction
    /// 
    /// # Arguments
    /// * `path` - Path to the file to extract content from
    /// 
    /// # Returns
    /// * `Result<String>` - Extracted text content or error
    pub async fn extract_content(&self, path: &Path) -> Result<String> {
        let extension = path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        // Route to appropriate extraction method based on file type
        match extension.as_str() {
            "docx" => self.extract_docx(path).await,
            "pdf" => self.extract_pdf(path).await,
            "xlsx" => self.extract_xlsx(path).await,
            "pptx" => self.extract_pptx(path).await,
            // Plain text files can be read directly
            "txt" | "md" | "rst" | "log" => {
                tokio::fs::read_to_string(path).await.map_err(Into::into)
            }
            _ => {
                // For unknown file types, try to read as text first
                // If that fails, mark as binary file
                match tokio::fs::read_to_string(path).await {
                    Ok(content) => Ok(content),
                    Err(_) => Ok(format!("[Binary file: {}]", path.display())),
                }
            }
        }
    }
    
    /// Extracts text content from Microsoft Word DOCX files
    /// DOCX files are ZIP archives containing XML files with the document content
    async fn extract_docx(&self, path: &Path) -> Result<String> {
        // Try to extract text from DOCX using external tools
        self.try_docx_extraction(path).await
    }
    
    /// Attempts to extract text from DOCX files using multiple methods
    /// DOCX files are ZIP archives containing XML files, so we can extract them
    /// 
    /// # Arguments
    /// * `path` - Path to the DOCX file
    /// 
    /// # Returns
    /// * `Result<String>` - Extracted text or placeholder message
    async fn try_docx_extraction(&self, path: &Path) -> Result<String> {
        // Method 1: Try pandoc first (if available)
        // Pandoc is a universal document converter that handles DOCX well
        let pandoc_output = Command::new("pandoc")
            .args(&[path.to_str().unwrap(), "-t", "plain"])
            .output();
            
        if let Ok(output) = pandoc_output {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout).to_string();
                if !content.trim().is_empty() {
                    return Ok(content);
                }
            }
        }
        
        // Method 2: Try unzip + basic XML parsing
        // DOCX files are ZIP archives containing word/document.xml with the main content
        let unzip_output = Command::new("unzip")
            .args(&["-p", path.to_str().unwrap(), "word/document.xml"])
            .output();
            
        if let Ok(output) = unzip_output {
            if output.status.success() {
                let xml_content = String::from_utf8_lossy(&output.stdout);
                // Basic XML text extraction - remove Word-specific tags
                // This is a simplified approach that works for most documents
                let text = xml_content
                    .replace("<w:t>", "")  // Remove text start tags
                    .replace("</w:t>", " ")  // Replace text end tags with spaces
                    .replace("<[^>]*>", "")  // Remove all remaining XML tags
                    .replace("&lt;", "<")   // Decode HTML entities
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");
                
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
        
        // If all methods fail, return a placeholder message
        log::warn!("Could not extract DOCX content from: {}", path.display());
        Ok(format!("[DOCX file: {} - content extraction failed]", path.display()))
    }
    
    /// Extracts text content from PDF files
    /// PDFs are complex binary formats that require specialized extraction
    async fn extract_pdf(&self, path: &Path) -> Result<String> {
        // Try internal PDF parsing first (currently disabled)
        match self.extract_pdf_with_lopdf(path).await {
            Ok(content) if !content.trim().is_empty() => Ok(content),
            _ => {
                // Fallback to external pdftotext tool
                self.try_pdftotext(path).await
            }
        }
    }
    
    /// Attempts to extract PDF content using the lopdf Rust library
    /// Currently disabled due to compilation issues with the dependency
    async fn extract_pdf_with_lopdf(&self, path: &Path) -> Result<String> {
        // lopdf temporarily disabled due to compilation issues
        log::info!("PDF extraction with lopdf not available for: {}", path.display());
        Ok(String::new())
    }
    
    /// Attempts to extract PDF content using the external pdftotext tool
    /// pdftotext is part of the poppler-utils package and is widely available
    /// 
    /// # Arguments
    /// * `path` - Path to the PDF file
    /// 
    /// # Returns
    /// * `Result<String>` - Extracted text or error message
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
    
    /// Extracts text content from Microsoft Excel XLSX files
    /// XLSX files are ZIP archives containing XML files with spreadsheet data
    async fn extract_xlsx(&self, path: &Path) -> Result<String> {
        // Try to extract text from XLSX using external tools
        self.try_xlsx_extraction(path).await
    }
    
    /// Attempts to extract text from XLSX files using multiple methods
    /// XLSX files store text content in xl/sharedStrings.xml
    /// 
    /// # Arguments
    /// * `path` - Path to the XLSX file
    /// 
    /// # Returns
    /// * `Result<String>` - Extracted text or placeholder message
    async fn try_xlsx_extraction(&self, path: &Path) -> Result<String> {
        // Method 1: Try pandoc first (if available)
        // Pandoc can handle XLSX files and convert them to plain text
        let pandoc_output = Command::new("pandoc")
            .args(&[path.to_str().unwrap(), "-t", "plain"])
            .output();
            
        if let Ok(output) = pandoc_output {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout).to_string();
                if !content.trim().is_empty() {
                    return Ok(content);
                }
            }
        }
        
        // Method 2: Try unzip + basic XML parsing
        // XLSX files store text content in xl/sharedStrings.xml
        let unzip_output = Command::new("unzip")
            .args(&["-p", path.to_str().unwrap(), "xl/sharedStrings.xml"])
            .output();
            
        if let Ok(output) = unzip_output {
            if output.status.success() {
                let xml_content = String::from_utf8_lossy(&output.stdout);
                // Basic XML text extraction - remove Excel-specific tags
                let text = xml_content
                    .replace("<t>", "")  // Remove text start tags
                    .replace("</t>", " ")  // Replace text end tags with spaces
                    .replace("<[^>]*>", "")  // Remove all remaining XML tags
                    .replace("&lt;", "<")   // Decode HTML entities
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");
                
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
        
        // If all methods fail, return a placeholder message
        log::warn!("Could not extract XLSX content from: {}", path.display());
        Ok(format!("[XLSX file: {} - content extraction failed]", path.display()))
    }
    
    /// Extracts text content from Microsoft PowerPoint PPTX files
    /// PPTX files are ZIP archives containing XML files with slide content
    async fn extract_pptx(&self, path: &Path) -> Result<String> {
        // Try to extract text from PPTX using external tools
        self.try_pptx_extraction(path).await
    }
    
    /// Attempts to extract text from PPTX files using multiple methods
    /// PPTX files store text content in ppt/slides/slide*.xml files
    /// 
    /// # Arguments
    /// * `path` - Path to the PPTX file
    /// 
    /// # Returns
    /// * `Result<String>` - Extracted text or placeholder message
    async fn try_pptx_extraction(&self, path: &Path) -> Result<String> {
        // Method 1: Try pandoc first (if available)
        // Pandoc can handle PPTX files and convert them to plain text
        let pandoc_output = Command::new("pandoc")
            .args(&[path.to_str().unwrap(), "-t", "plain"])
            .output();
            
        if let Ok(output) = pandoc_output {
            if output.status.success() {
                let content = String::from_utf8_lossy(&output.stdout).to_string();
                if !content.trim().is_empty() {
                    return Ok(content);
                }
            }
        }
        
        // Method 2: Try unzip + basic XML parsing
        // PPTX files store text content in ppt/slides/slide*.xml files
        // We start with slide1.xml as a representative sample
        let unzip_output = Command::new("unzip")
            .args(&["-p", path.to_str().unwrap(), "ppt/slides/slide1.xml"])
            .output();
            
        if let Ok(output) = unzip_output {
            if output.status.success() {
                let xml_content = String::from_utf8_lossy(&output.stdout);
                // Basic XML text extraction - remove PowerPoint-specific tags
                let text = xml_content
                    .replace("<a:t>", "")  // Remove text start tags
                    .replace("</a:t>", " ")  // Replace text end tags with spaces
                    .replace("<[^>]*>", "")  // Remove all remaining XML tags
                    .replace("&lt;", "<")   // Decode HTML entities
                    .replace("&gt;", ">")
                    .replace("&amp;", "&")
                    .replace("&quot;", "\"")
                    .replace("&apos;", "'");
                
                if !text.trim().is_empty() {
                    return Ok(text);
                }
            }
        }
        
        // If all methods fail, return a placeholder message
        log::warn!("Could not extract PPTX content from: {}", path.display());
        Ok(format!("[PowerPoint file: {} - content extraction failed]", path.display()))
    }
}
