use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::State;
use ferret::platform::FerretPlatform;

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalysisResults {
    pub total_files: usize,
    pub total_groups: usize,
    pub duplicate_results: DuplicateResults,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateResults {
    pub total_duplicates: usize,
    pub space_wasted: u64,
    pub duplicate_groups: Vec<DuplicateGroup>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub base_name: String,
    pub duplicate_sets: Vec<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub path: String,
    pub line_number: Option<usize>,
    pub snippet: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub modified: String,
    pub is_directory: bool,
}

#[tauri::command]
pub async fn analyze_directory(
    path: String,
) -> Result<AnalysisResults, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    let mut platform = FerretPlatform::new()
        .map_err(|e| e.to_string())?;
    let results = platform.analyze_directory(&path).await
        .map_err(|e| e.to_string())?;

    // Convert PathBuf to String for serialization
    let duplicate_results = DuplicateResults {
        total_duplicates: results.duplicate_results.total_duplicates,
        space_wasted: results.duplicate_results.space_wasted,
        duplicate_groups: results.duplicate_results.duplicate_groups.into_iter().map(|group| {
            DuplicateGroup {
                base_name: group.base_name,
                duplicate_sets: group.duplicate_sets.into_iter().map(|set| {
                    set.into_iter().map(|path| path.to_string_lossy().to_string()).collect()
                }).collect(),
            }
        }).collect(),
    };

    Ok(AnalysisResults {
        total_files: results.total_files,
        total_groups: results.total_groups,
        duplicate_results,
    })
}

#[tauri::command]
pub async fn search_files(
    query: String,
    path: String,
    limit: usize,
) -> Result<Vec<SearchResult>, String> {
    if query.is_empty() {
        return Err("Search query cannot be empty".to_string());
    }

    if limit == 0 {
        return Err("Limit must be greater than 0".to_string());
    }

    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    let mut platform = FerretPlatform::new()
        .map_err(|e| e.to_string())?;
    let results = platform.search(&query, &path, limit).await
        .map_err(|e| e.to_string())?;

    let search_results = results.into_iter().map(|result| {
        SearchResult {
            path: result.path.to_string_lossy().to_string(),
            line_number: result.line_number,
            snippet: result.snippet,
        }
    }).collect();

    Ok(search_results)
}

#[tauri::command]
pub async fn index_directory(
    path: String,
    index_path: Option<String>,
) -> Result<String, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    let index_path = index_path.map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("./ferret_index"));

    let mut platform = FerretPlatform::new()
        .map_err(|e| e.to_string())?;
    platform.index_directory(&path, &index_path).await
        .map_err(|e| e.to_string())?;

    Ok(format!("Indexing complete for: {}", path.display()))
}

#[tauri::command]
pub async fn get_file_info(path: String) -> Result<FileInfo, String> {
    let path = PathBuf::from(path);
    if !path.exists() {
        return Err("Path does not exist".to_string());
    }

    let metadata = std::fs::metadata(&path)
        .map_err(|e| e.to_string())?;

    let modified = metadata.modified()
        .map_err(|e| e.to_string())?
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    Ok(FileInfo {
        path: path.to_string_lossy().to_string(),
        size: metadata.len(),
        modified: modified.to_string(),
        is_directory: metadata.is_dir(),
    })
}
