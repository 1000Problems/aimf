use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::types::CompilerConfig;

/// A discovered resource — file or directory that might be worth indexing.
#[derive(Debug, Clone)]
pub struct Discovered {
    pub path: PathBuf,       // Relative to root
    pub abs_path: PathBuf,   // Absolute path for reading
    pub is_dir: bool,
    pub size_bytes: u64,     // For files: file size. For dirs: sum of contents.
    pub kind: DiscoveredKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DiscoveredKind {
    Mermaid,       // .mermaid, .mmd files
    Decision,      // Files in decisions/ or ADR-style docs
    Config,        // Cargo.toml, package.json, etc.
    Documentation, // README, docs, .md files
    Specification, // SPEC files, format definitions
    SourceDir,     // A source directory as a logical unit
    SourceFile,    // An important individual source file (entry points)
    Task,          // .task.md files
}

/// Scan a project directory for meaningful resources.
/// This is NOT a full file index — it discovers resources worth putting in @NAV.
pub fn scan(config: &CompilerConfig) -> Vec<Discovered> {
    let root = &config.root;
    let mut results = Vec::new();
    let mut source_dirs: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();

    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let abs_path = entry.path().to_path_buf();
        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(&abs_path)
            .to_path_buf();

        let rel_str = rel_path.to_string_lossy().to_string();

        // Skip ignored directories — check each path component
        if rel_path.components().any(|c| {
            let name = c.as_os_str().to_string_lossy();
            config.ignore_patterns.iter().any(|p| name == *p)
        }) {
            continue;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
        let ext = rel_path.extension()
            .map(|e| e.to_string_lossy().to_lowercase())
            .unwrap_or_default();
        let filename = rel_path.file_name()
            .map(|f| f.to_string_lossy().to_lowercase())
            .unwrap_or_default();

        // ── Mermaid files: always include
        if ext == "mermaid" || ext == "mmd" {
            results.push(Discovered {
                path: rel_path.clone(),
                abs_path: abs_path.clone(),
                is_dir: false,
                size_bytes: size,
                kind: DiscoveredKind::Mermaid,
            });
            continue;
        }

        // ── Decision documents
        if rel_str.contains("decision") || rel_str.contains("adr")
            || (ext == "md" && filename.starts_with("0"))  // 001-something.md
        {
            results.push(Discovered {
                path: rel_path.clone(),
                abs_path: abs_path.clone(),
                is_dir: false,
                size_bytes: size,
                kind: DiscoveredKind::Decision,
            });
            continue;
        }

        // ── Task documents
        if filename.ends_with(".task.md") || filename.starts_with("task-") {
            results.push(Discovered {
                path: rel_path.clone(),
                abs_path: abs_path.clone(),
                is_dir: false,
                size_bytes: size,
                kind: DiscoveredKind::Task,
            });
            continue;
        }

        // ── Specifications
        if filename.contains("spec") && ext == "md" {
            results.push(Discovered {
                path: rel_path.clone(),
                abs_path: abs_path.clone(),
                is_dir: false,
                size_bytes: size,
                kind: DiscoveredKind::Specification,
            });
            continue;
        }

        // ── Key config files (top-level only)
        if is_key_config(&filename, &rel_path) {
            results.push(Discovered {
                path: rel_path.clone(),
                abs_path: abs_path.clone(),
                is_dir: false,
                size_bytes: size,
                kind: DiscoveredKind::Config,
            });
            continue;
        }

        // ── README and key documentation
        if filename.starts_with("readme") || filename == "changelog.md"
            || filename == "contributing.md"
            || (ext == "md" && rel_str.starts_with("docs/"))
        {
            results.push(Discovered {
                path: rel_path.clone(),
                abs_path: abs_path.clone(),
                is_dir: false,
                size_bytes: size,
                kind: DiscoveredKind::Documentation,
            });
            continue;
        }

        // ── Track source directories for DIR entries
        if config.include_source_dirs && is_source_file(&ext) {
            if let Some(parent) = rel_path.parent() {
                // Use the first meaningful directory level
                let dir = meaningful_source_dir(parent);
                if !dir.as_os_str().is_empty() {
                    source_dirs.insert(dir);
                }
            }
        }
    }

    // ── Add source directory entries
    if config.include_source_dirs {
        for dir in source_dirs {
            let abs_dir = root.join(&dir);
            let size = dir_total_size(&abs_dir, &config.ignore_patterns);
            results.push(Discovered {
                path: dir,
                abs_path: abs_dir,
                is_dir: true,
                size_bytes: size,
                kind: DiscoveredKind::SourceDir,
            });
        }
    }

    // Sort by kind priority then path
    results.sort_by(|a, b| {
        kind_priority(&a.kind).cmp(&kind_priority(&b.kind))
            .then(a.path.cmp(&b.path))
    });

    results
}

fn kind_priority(kind: &DiscoveredKind) -> u8 {
    match kind {
        DiscoveredKind::Mermaid => 0,
        DiscoveredKind::Specification => 1,
        DiscoveredKind::Decision => 2,
        DiscoveredKind::Documentation => 3,
        DiscoveredKind::Config => 4,
        DiscoveredKind::SourceDir => 5,
        DiscoveredKind::SourceFile => 6,
        DiscoveredKind::Task => 7,
    }
}

fn is_key_config(filename: &str, rel_path: &Path) -> bool {
    let depth = rel_path.components().count();
    if depth > 2 { return false; } // Only top-level or one deep

    matches!(filename,
        "cargo.toml" | "package.json" | "pyproject.toml" | "go.mod"
        | "gemfile" | "requirements.txt" | "tsconfig.json"
        | "dockerfile" | "docker-compose.yml" | "docker-compose.yaml"
        | "makefile" | "justfile" | ".env.example"
    )
}

fn is_source_file(ext: &str) -> bool {
    matches!(ext,
        "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java"
        | "kt" | "rb" | "php" | "c" | "cpp" | "h" | "hpp" | "cs"
        | "swift" | "ex" | "exs" | "vue" | "svelte" | "dart"
    )
}

/// Get the meaningful source directory — e.g. for src/api/handlers/auth.rs,
/// return src/api/ (not src/api/handlers/ which is too granular).
fn meaningful_source_dir(path: &Path) -> PathBuf {
    let components: Vec<_> = path.components().collect();
    // Take first 2 levels max (e.g. src/api, lib/core)
    let take = components.len().min(2);
    components[..take].iter().collect()
}

/// Calculate total byte size of all files in a directory (recursive).
fn dir_total_size(dir: &Path, ignore: &[String]) -> u64 {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter(|e| {
            !e.path().components().any(|c| {
                let name = c.as_os_str().to_string_lossy();
                ignore.iter().any(|i| *name == **i)
            })
        })
        .map(|e| e.metadata().map(|m| m.len()).unwrap_or(0))
        .sum()
}
