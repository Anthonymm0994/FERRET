use std::path::{Path, PathBuf};
use std::collections::HashMap;
use anyhow::Result;
use sha2::{Sha256, Digest};
use tokio::io::AsyncReadExt;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use grep_regex::RegexMatcher;
use grep_searcher::{Searcher, SearcherBuilder};
use ignore::WalkBuilder;

// Re-export the platform from the main crate
pub use ferret::platform::FerretPlatform;
pub use ferret::analysis::duplicates::{DuplicateResults, DuplicateGroup};
pub use ferret::search::engine::SearchResult;
