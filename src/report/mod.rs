use crate::file_scanner::FileScanner;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct ReportGenerator;

impl ReportGenerator {
    pub fn generate(scanner: &FileScanner, output_path: &Path) {
        // Create HTML report file
        let mut file = File::create(output_path).expect("Could not create report file.");

        // Write HTML header and title with dark mode styling
        writeln!(
            file,
            "<!DOCTYPE html>
            <html lang=\"en\">
            <head>
                <meta charset=\"UTF-8\">
                <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\">
                <title>File Analysis Report</title>
                <style>
                    body {{ font-family: Arial, sans-serif; margin: 20px; color: #e0e0e0; background-color: #121212; }}
                    h1, h2 {{ color: #ffcc00; }}
                    table {{ width: 100%; border-collapse: collapse; margin: 20px 0; }}
                    th, td {{ padding: 8px 12px; border: 1px solid #333; text-align: left; }}
                    th {{ background-color: #333; color: #ffcc00; }}
                    .section-title {{ color: #ffcc00; }}
                    .file-path {{ color: #9acd32; }}
                    .directory-structure {{ padding-left: 20px; }}
                    .collapsible {{
                        cursor: pointer;
                        padding: 10px;
                        background-color: #333;
                        border: none;
                        text-align: left;
                        outline: none;
                        color: #ffcc00;
                    }}
                    .collapsible:after {{
                        content: '\\002B';
                        font-weight: bold;
                        float: right;
                    }}
                    .active:after {{
                        content: '\\2212';
                    }}
                    .content {{
                        padding: 0 20px;
                        display: none;
                        overflow: hidden;
                        background-color: #222;
                    }}
                </style>
            </head>
            <body>
            <h1>File Analysis Report</h1>
            <p>Generated by Ferret - File Examination, Retrieval, and Redundancy Evaluation Tool</p>"
        ).unwrap();

        // Section: Interactive Directory Structure with Nested Collapsible Directories
        writeln!(file, "<h2 class=\"section-title\">Directory Structure</h2>").unwrap();
        writeln!(file, "<button class=\"collapsible\">View Files</button><div class=\"content\">").unwrap();
        let mut directories = scanner.collect_directory_structure();
        Self::write_directory_structure(&mut file, &mut directories, 0);
        writeln!(file, "</div>").unwrap();

        // Section 1: Duplicate Files
        writeln!(file, "<h2>Duplicate Files</h2>").unwrap();
        if scanner.duplicate_groups.is_empty() {
            writeln!(file, "<p>No duplicate files found.</p>").unwrap();
        } else {
            writeln!(file, "<table><tr><th>Duplicate Group</th><th>File Paths</th></tr>").unwrap();
            
            // Add `.into_iter()` to `grouped_duplicates` to allow for enumeration
            for (group_num, group) in scanner.grouped_duplicates().into_iter().enumerate() {
                let file_paths: String = group
                    .iter()
                    .map(|p| format!("{}", p.to_string_lossy()))
                    .collect::<Vec<String>>()
                    .join("<br>");
                
                writeln!(file, "<tr><td>Group {}</td><td>{}</td></tr>", group_num + 1, file_paths).unwrap();
            }
            writeln!(file, "</table>").unwrap();
        }


        // Section 2: Similar Files with Improved Matching
        writeln!(file, "<h2 class=\"section-title\">Similar Files (Above 70% Similarity)</h2>").unwrap();
        let mut similar_files = scanner.similarity_tool.get_similar_files();
        if similar_files.is_empty() {
            writeln!(file, "<p>No similar files found above the threshold.</p>").unwrap();
        } else {
            similar_files.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap()); // Sort by similarity score
            writeln!(file, "<table><tr><th>File 1</th><th>File 2</th><th>Similarity Score</th></tr>").unwrap();
            for ((path1, path2), score) in &similar_files {
                writeln!(
                    file,
                    "<tr><td>{}</td><td>{}</td><td>{:.2}</td></tr>",
                    path1.display(), path2.display(), score
                ).unwrap();
            }
            writeln!(file, "</table>").unwrap();
        }

        // Section 3: File Age Analysis
        writeln!(file, "<h2 class=\"section-title\">File Age Analysis</h2>").unwrap();
        let (recent, stale, old) = scanner.file_age_summary();
        writeln!(file, "<p>Recent files (last 30 days): {}</p>", recent).unwrap();
        writeln!(file, "<p>Stale files (30-180 days): {}</p>", stale).unwrap();
        writeln!(file, "<p>Old files (over 180 days): {}</p>", old).unwrap();

        // Section 4: Summary Statistics
        writeln!(file, "<h2 class=\"section-title\">Summary Statistics</h2>").unwrap();
        writeln!(file, "<p>Total files analyzed: {}</p>", scanner.total_files_count()).unwrap();
        writeln!(file, "<p>Total duplicates detected: {}</p>", scanner.duplicate_files.len()).unwrap();
        writeln!(file, "<p>Total similar files detected: {}</p>", similar_files.len()).unwrap();

        // Close HTML document with JavaScript for collapsible elements
        writeln!(file, "</body>
            <script>
                var coll = document.getElementsByClassName('collapsible');
                for (var i = 0; i < coll.length; i++) {{
                    coll[i].addEventListener('click', function() {{
                        this.classList.toggle('active');
                        var content = this.nextElementSibling;
                        if (content.style.display === 'block') {{
                            content.style.display = 'none';
                        }} else {{
                            content.style.display = 'block';
                        }}
                    }});
                }}
            </script>
            </html>").unwrap();
    }

     // Recursive function to write directory structure as collapsible items
     fn write_directory_structure(file: &mut File, directories: &mut Vec<(String, Vec<String>)>, level: usize) {
        for (directory, files) in directories {
            // Indentation based on directory depth
            writeln!(file, "<button class=\"collapsible\" style=\"margin-left: {}px\">{}</button>", level * 20, directory).unwrap();
            writeln!(file, "<div class=\"content\">").unwrap();
            for file_path in files {
                writeln!(file, "<p class=\"file-path\" style=\"margin-left: {}px\">{}</p>", (level + 1) * 20, file_path).unwrap();
            }
            writeln!(file, "</div>").unwrap();
        }
    }
}