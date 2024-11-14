mod cli;
mod config;
mod file_scanner;
mod report;

use async_std::task;
use cli::parse_args;
use file_scanner::FileScanner;
use report::ReportGenerator;
use std::path::Path;

fn main() {
    // Parse arguments
    let args = parse_args();

    // Initialize and run analyses
    let mut file_scanner = FileScanner::new(args.directory);
    task::block_on(async {
        file_scanner.run_all_analyses().await;
    });

    // Generate report
    // output_path = "path/idk/directory"
    //ReportGenerator::generate(&file_scanner, output_path);
    //println!("Report generated: {:?}", output_path);
}
