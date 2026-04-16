use std::collections::HashMap;
use std::fs;
use std::path::Path;
use regex::Regex;

/// Mine Claude Code session logs for context about project resources.
///
/// This reads .jsonl session files and extracts:
/// - Mermaid diagrams and the conversation text surrounding them
/// - File paths mentioned near mermaid diagrams
/// - Useful context that enriches @NAV descriptions
///
/// No AI, no token consumption — pure regex and string matching.
pub fn mine_sessions(session_dir: &Path) -> HashMap<String, String> {
    let mut enrichments: HashMap<String, String> = HashMap::new();

    // Find all .jsonl files
    let jsonl_files: Vec<_> = match fs::read_dir(session_dir) {
        Ok(entries) => entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension()
                    .map(|ext| ext == "jsonl")
                    .unwrap_or(false)
            })
            .map(|e| e.path())
            .collect(),
        Err(_) => return enrichments,
    };

    for jsonl_path in &jsonl_files {
        if let Ok(content) = fs::read_to_string(jsonl_path) {
            extract_mermaid_context(&content, &mut enrichments);
        }
    }

    enrichments
}

/// Extract mermaid diagrams and their surrounding conversation context.
fn extract_mermaid_context(content: &str, enrichments: &mut HashMap<String, String>) {
    // Regex for mermaid code blocks — handles both real newlines and escaped \n
    let mermaid_re = Regex::new(r"```mermaid(?:\\n|\n)([\s\S]*?)(?:\\n|\n)```")
        .unwrap();

    for cap in mermaid_re.captures_iter(content) {
        let raw_mermaid = &cap[1];

        // Unescape JSONL escapes
        let mermaid = raw_mermaid
            .replace("\\n", "\n")
            .replace("\\t", "\t");

        // Extract diagram type
        let diagram_type = mermaid.lines()
            .filter(|l| !l.trim().is_empty() && !l.starts_with("%%"))
            .next()
            .map(|l| l.split_whitespace().next().unwrap_or(""))
            .unwrap_or("")
            .to_lowercase();

        // Extract title from frontmatter if present
        let title = extract_title_from_mermaid(&mermaid);

        // Look for file paths mentioned near this mermaid block
        let context_window = get_context_window(content, cap.get(0).unwrap().start(), 500);
        let mentioned_files = extract_file_paths(&context_window);

        // Build enrichment: associate mermaid context with mentioned files
        for file_path in &mentioned_files {
            if file_path.ends_with(".mermaid") || file_path.ends_with(".mmd") {
                let desc = match &title {
                    Some(t) => t.clone(),
                    None => format!("{} diagram", diagram_type),
                };
                enrichments.entry(file_path.clone())
                    .or_insert(desc);
            }
        }

        // Also try to match by diagram type to known file patterns
        if let Some(t) = &title {
            // If the title mentions a concept, look for matching files
            let key_words: Vec<&str> = t.split_whitespace()
                .filter(|w| w.len() > 3)
                .collect();
            for word in key_words {
                let lower = word.to_lowercase();
                if !lower.is_empty() {
                    // Store as a potential enrichment key (file path may match)
                    enrichments.entry(format!("*{}", lower))
                        .or_insert(t.clone());
                }
            }
        }
    }
}

/// Get a window of text around a position in the content.
fn get_context_window(content: &str, pos: usize, window: usize) -> String {
    let start = pos.saturating_sub(window);
    let end = (pos + window).min(content.len());
    content[start..end].to_string()
}

/// Extract file paths from text using patterns from FOUNDATION.md.
fn extract_file_paths(text: &str) -> Vec<String> {
    let path_re = Regex::new(
        r#"(?:^|[\s`'"])([a-zA-Z0-9_./-]{3,200}\.(?:rs|ts|tsx|js|py|toml|json|md|mermaid|mmd|sql|yaml|yml))"#
    ).unwrap();

    let mut paths: Vec<String> = Vec::new();
    for cap in path_re.captures_iter(text) {
        let path = cap[1].to_string();
        if !path.contains("http") && !paths.contains(&path) {
            paths.push(path);
        }
    }

    paths
}

/// Extract title from mermaid frontmatter.
fn extract_title_from_mermaid(content: &str) -> Option<String> {
    let mut in_frontmatter = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "---" {
            if in_frontmatter { return None; }
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && trimmed.starts_with("title:") {
            return Some(trimmed.split_once(':')?.1.trim().to_string());
        }
    }
    None
}
