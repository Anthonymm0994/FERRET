use clap::{Arg, ArgMatches, Command};

pub struct Cli {
    pub directory: String,
    pub search_term: Option<String>,
    pub report_path: Option<String>,
}

pub fn build_cli() -> Command {
    Command::new("ferret")
        .about("Analyzes files in a directory for similarity, aging, and more")
        .arg(
            Arg::new("directory")
                .short('d')
                .long("directory")
                .help("Directory to scan")
                .required(true)
                .num_args(1),  // Replaces `takes_value(true)` in clap v4
        )
        .arg(
            Arg::new("search")
                .short('s')
                .long("search")
                .help("Search term to find within files")
                .num_args(1),  // Replaces `takes_value(true)`
        )
        .arg(
            Arg::new("report")
                .short('r')
                .long("report")
                .help("Path to save the report (CSV format)")
                .num_args(1),  // Replaces `takes_value(true)`
        )
}

pub fn parse_args() -> Cli {
    let matches = build_cli().get_matches();

    Cli {
        directory: matches.get_one::<String>("directory").unwrap().to_string(),
        search_term: matches.get_one::<String>("search").map(|s| s.to_string()),
        report_path: matches.get_one::<String>("report").map(|r| r.to_string()),
    }
}
