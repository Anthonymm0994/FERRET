use ferret::platform::FerretPlatform;
use std::path::Path;
use tempfile::TempDir;
use std::fs;

#[tokio::test]
async fn test_basic_analysis() {
    // Create a temporary directory with test files
    let temp_dir = TempDir::new().unwrap();
    let test_path = temp_dir.path();
    
    // Create some test files
    fs::write(test_path.join("test1.txt"), "Hello, world!").unwrap();
    fs::write(test_path.join("test2.txt"), "Hello, world!").unwrap();
    fs::write(test_path.join("different.txt"), "Different content").unwrap();
    
    // Test the platform
    let mut platform = FerretPlatform::new().unwrap();
    let results = platform.analyze_directory(test_path).await.unwrap();
    
    // Should find 2 duplicate files
    assert!(results.duplicate_results.total_duplicates >= 2);
    assert!(results.duplicate_results.space_wasted > 0);
}
