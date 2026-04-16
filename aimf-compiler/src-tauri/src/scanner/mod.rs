use std::fs;
use std::path::{Path, PathBuf};
use sha2::{Sha256, Digest};
use walkdir::WalkDir;

use crate::types::CompilerConfig;

/// A raw file entry discovered during scanning.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    pub path: PathBuf,       // relative to root
    pub abs_path: PathBuf,   // absolute path for reading
    pub size: u64,
    pub hash: String,        // 8-char truncated SHA-256
}

/// Walk the project directory and return all non-ignored files.
pub fn scan(config: &CompilerConfig) -> Vec<ScannedFile> {
    let root = &config.root;
    let mut files = Vec::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }

        let abs_path = entry.path().to_path_buf();
        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(&abs_path)
            .to_path_buf();

        // Check ignore patterns
        let rel_str = rel_path.to_string_lossy();
        if should_ignore(&rel_str, &config.ignore_patterns) {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let hash = compute_hash(&abs_path);

        files.push(ScannedFile {
            path: rel_path,
            abs_path,
            size,
            hash,
        });
    }

    // Sort by path for deterministic output
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}

/// Check if a path matches any ignore pattern.
fn should_ignore(path: &str, patterns: &[String]) -> bool {
    for pattern in patterns {
        if let Ok(matcher) = glob::Pattern::new(pattern) {
            if matcher.matches(path) {
                return true;
            }
        }
        // Also check if the pattern is a directory prefix
        if path.starts_with(pattern.trim_end_matches("/**")) {
            return true;
        }
    }
    false
}

/// Compute a truncated SHA-256 hash (8 hex chars).
fn compute_hash(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => {
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            let result = hasher.finalize();
            hex::encode(&result[..4]) // 8 hex chars = 4 bytes
        }
        Err(_) => "00000000".to_string(),
    }
}

// Note: we depend on the `hex` crate implicitly via sha2.
// Add `hex = "0.4"` to Cargo.toml if not pulled transitively.
