use std::fs;
use std::path::Path;

use crate::scanner::{Discovered, DiscoveredKind};
use crate::types::ResourceType;

/// Classify a discovered resource and generate its type + description.
pub fn classify(discovered: &Discovered) -> (ResourceType, String) {
    let resource_type = match discovered.kind {
        DiscoveredKind::Mermaid => ResourceType::MRM,
        DiscoveredKind::Decision => ResourceType::DEC,
        DiscoveredKind::SourceDir => ResourceType::DIR,
        DiscoveredKind::Config => ResourceType::CFG,
        DiscoveredKind::Documentation => ResourceType::DOC,
        DiscoveredKind::Specification => ResourceType::SPEC,
        DiscoveredKind::SourceFile => ResourceType::SRC,
        DiscoveredKind::Task => ResourceType::TASK,
    };

    let about = generate_about(discovered);
    (resource_type, about)
}

/// Generate a one-line description by reading the resource.
/// No AI — just heuristics on file content.
fn generate_about(discovered: &Discovered) -> String {
    let desc = match discovered.kind {
        DiscoveredKind::Mermaid => describe_mermaid(&discovered.abs_path),
        DiscoveredKind::Decision => describe_markdown(&discovered.abs_path, "decision"),
        DiscoveredKind::Documentation => describe_markdown(&discovered.abs_path, "documentation"),
        DiscoveredKind::Specification => describe_markdown(&discovered.abs_path, "specification"),
        DiscoveredKind::Config => describe_config(&discovered.abs_path),
        DiscoveredKind::SourceDir => describe_source_dir(&discovered.abs_path, &discovered.path),
        DiscoveredKind::SourceFile => describe_source_file(&discovered.abs_path),
        DiscoveredKind::Task => describe_markdown(&discovered.abs_path, "task"),
    };

    truncate(&desc, 80)
}

/// Describe a mermaid file by extracting diagram type and title.
fn describe_mermaid(path: &Path) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return "Mermaid diagram".into(),
    };

    // Extract title from frontmatter: ---\ntitle: X\n---
    let title = extract_yaml_title(&content);

    // Extract diagram type from first non-empty, non-comment line
    let diagram_type = content.lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("---") && !l.starts_with("title"))
        .next()
        .map(|l| l.split_whitespace().next().unwrap_or(""))
        .unwrap_or("");

    match (title, diagram_type) {
        (Some(t), _) => t,
        (None, dt) if !dt.is_empty() => format!("{} diagram", dt),
        _ => "Mermaid diagram".into(),
    }
}

/// Describe a markdown file by extracting the first heading or first sentence.
fn describe_markdown(path: &Path, fallback: &str) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return fallback.into(),
    };

    // Look for first # heading
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed.trim_start_matches('#').trim().to_string();
        }
    }

    // Fallback: first non-empty line
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() && !trimmed.starts_with("---") && !trimmed.starts_with("```") {
            return trimmed.to_string();
        }
    }

    fallback.into()
}

/// Describe a config file from its content.
fn describe_config(path: &Path) -> String {
    let filename = path.file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Read and try to extract project name or description
    if let Ok(content) = fs::read_to_string(path) {
        // Cargo.toml
        if filename == "cargo.toml" {
            if let Some(name) = extract_toml_field(&content, "name") {
                if let Some(desc) = extract_toml_field(&content, "description") {
                    return format!("{}: {}", name, desc);
                }
                return format!("Rust project: {}", name);
            }
        }
        // package.json
        if filename == "package.json" {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                let name = json.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                if let Some(desc) = json.get("description").and_then(|v| v.as_str()) {
                    return format!("{}: {}", name, desc);
                }
                return format!("Node project: {}", name);
            }
        }
    }

    match filename.as_str() {
        "cargo.toml" => "Rust dependencies and build config".into(),
        "package.json" => "Node dependencies and scripts".into(),
        "pyproject.toml" => "Python project config".into(),
        "go.mod" => "Go module dependencies".into(),
        "dockerfile" => "Container build definition".into(),
        "docker-compose.yml" | "docker-compose.yaml" => "Container orchestration".into(),
        "tsconfig.json" => "TypeScript compiler config".into(),
        "makefile" | "justfile" => "Build automation rules".into(),
        _ => "Configuration".into(),
    }
}

/// Describe a source directory by listing what it contains.
fn describe_source_dir(abs_path: &Path, rel_path: &Path) -> String {
    let dir_name = rel_path.to_string_lossy();

    // Count files by type
    let mut rs = 0u32;
    let mut py = 0u32;
    let mut ts = 0u32;
    let mut other = 0u32;
    let mut total = 0u32;

    if let Ok(entries) = fs::read_dir(abs_path) {
        for entry in entries.filter_map(|e| e.ok()) {
            if entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
                total += 1;
                let ext = entry.path().extension()
                    .map(|e| e.to_string_lossy().to_lowercase())
                    .unwrap_or_default();
                match ext.as_str() {
                    "rs" => rs += 1,
                    "py" => py += 1,
                    "ts" | "tsx" | "js" | "jsx" => ts += 1,
                    _ => other += 1,
                }
            }
        }
    }

    // Also count subdirs
    let subdirs: Vec<String> = fs::read_dir(abs_path)
        .into_iter()
        .flat_map(|entries| entries.filter_map(|e| e.ok()))
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            !name.starts_with('.')
        })
        .map(|e| e.file_name().to_string_lossy().to_string())
        .collect();

    let lang = if rs > 0 { "Rust" }
        else if py > 0 { "Python" }
        else if ts > 0 { "TypeScript" }
        else { "source" };

    if subdirs.is_empty() {
        format!("{} {} files in {}", total, lang, dir_name)
    } else {
        let sub_str = subdirs.join(", ");
        format!("{}: {} ({})", dir_name, sub_str, lang)
    }
}

/// Describe a source file by extracting key declarations.
fn describe_source_file(path: &Path) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return "Source file".into(),
    };

    // Look for a module doc comment or first function
    for line in content.lines().take(10) {
        let trimmed = line.trim();
        if trimmed.starts_with("//!") || trimmed.starts_with("///") {
            return trimmed.trim_start_matches('/').trim_start_matches('!').trim().to_string();
        }
        if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
            return trimmed.trim_matches('"').trim_matches('\'').trim().to_string();
        }
    }

    "Source file".into()
}

/// Generate a short ID from a resource path.
pub fn generate_id(path: &Path, kind: &DiscoveredKind, index: usize) -> String {
    let filename = path.file_stem()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();

    match kind {
        DiscoveredKind::Mermaid => {
            // Use filename: architecture.mermaid → ARCH, data-model.mermaid → DMOD
            abbreviate(&filename, 4).to_uppercase()
        }
        DiscoveredKind::Decision => {
            // Decision docs: 001-use-tauri.md → D001
            if filename.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                format!("D{}", &filename[..filename.find('-').unwrap_or(3).min(3)])
            } else {
                format!("D{:03}", index)
            }
        }
        DiscoveredKind::SourceDir => {
            // Directory: src/api → API, src/compiler → COMP
            let last = path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("DIR{}", index));
            abbreviate(&last, 4).to_uppercase()
        }
        DiscoveredKind::Config => {
            abbreviate(&filename, 4).to_uppercase()
        }
        DiscoveredKind::Documentation => {
            if filename.to_lowercase().starts_with("readme") {
                "READ".into()
            } else {
                abbreviate(&filename, 4).to_uppercase()
            }
        }
        DiscoveredKind::Specification => {
            abbreviate(&filename, 4).to_uppercase()
        }
        _ => {
            abbreviate(&filename, 4).to_uppercase()
        }
    }
}

/// Determine default load hints for a resource.
pub fn default_load_hints(kind: &DiscoveredKind) -> Vec<String> {
    match kind {
        DiscoveredKind::Mermaid => vec!["design".into()],
        DiscoveredKind::Decision => vec!["reference".into()],
        DiscoveredKind::SourceDir => vec!["task:*".into()],
        DiscoveredKind::Config => vec!["task:*".into()],
        DiscoveredKind::Documentation => vec!["reference".into()],
        DiscoveredKind::Specification => vec!["design".into(), "reference".into()],
        DiscoveredKind::SourceFile => vec!["task:*".into()],
        DiscoveredKind::Task => vec!["task:*".into()],
    }
}

// ─── Helpers ────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max - 3])
    }
}

fn abbreviate(s: &str, max_len: usize) -> String {
    let clean: String = s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect();
    if clean.len() <= max_len {
        clean
    } else {
        clean[..max_len].to_string()
    }
}

fn extract_yaml_title(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for line in content.lines() {
        if line.trim() == "---" {
            if in_frontmatter { return None; }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && line.trim_start().starts_with("title:") {
            return Some(line.split_once(':')?.1.trim().to_string());
        }
    }
    None
}

fn extract_toml_field<'a>(content: &'a str, field: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(field) && trimmed.contains('=') {
            let val = trimmed.split_once('=')?.1.trim().trim_matches('"');
            return Some(val.to_string());
        }
    }
    None
}
