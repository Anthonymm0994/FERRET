use std::path::Path;
use anyhow::Result;
use std::process::Command;

pub struct ToolIntegrations;

impl ToolIntegrations {
    pub fn new() -> Self {
        Self
    }
    
    pub async fn preview_file_with_bat(&self, path: &Path) -> Result<String> {
        let output = Command::new("bat")
            .args(&["--style", "numbers", "--color", "always", path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            // Fallback to basic file reading
            tokio::fs::read_to_string(path).await.map_err(Into::into)
        }
    }
    
    pub async fn preview_binary_with_hexyl(&self, path: &Path) -> Result<String> {
        let output = Command::new("hexyl")
            .args(&["--length", "512", path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Ok(format!("[Binary file: {} - hexyl not available]", path.display()))
        }
    }
    
    pub async fn analyze_csv_with_xsv(&self, path: &Path) -> Result<String> {
        let output = Command::new("xsv")
            .args(&["stats", path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            Ok(format!("[CSV file: {} - xsv not available]", path.display()))
        }
    }
    
    pub async fn format_json_with_jq(&self, path: &Path) -> Result<String> {
        let output = Command::new("jq")
            .args(&[".", path.to_str().unwrap()])
            .output()?;
        
        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            // Fallback to basic file reading
            tokio::fs::read_to_string(path).await.map_err(Into::into)
        }
    }
}
