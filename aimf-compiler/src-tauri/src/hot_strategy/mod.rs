use std::fs;
use std::path::Path;

use crate::types::HotStrategy;

/// Apply a hot strategy to a file and return the payload string.
pub fn apply_strategy(abs_path: &Path, strategy: &HotStrategy) -> String {
    match strategy {
        HotStrategy::Full => read_full(abs_path),
        HotStrategy::Head(n) => read_head(abs_path, *n),
        HotStrategy::Signature => extract_signatures(abs_path),
        HotStrategy::Summary => {
            // Summary requires LLM inference — for now, fall back to head
            let head = read_head(abs_path, 80);
            format!("[summary pending]\n{}", head)
        }
        HotStrategy::Delta => {
            // Delta requires git diff — placeholder
            "[delta: no previous compilation found]".to_string()
        }
    }
}

/// Read the entire file content.
fn read_full(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|_| "[unreadable]".to_string())
}

/// Read the first N lines of a file.
fn read_head(path: &Path, n: usize) -> String {
    match fs::read_to_string(path) {
        Ok(content) => {
            let lines: Vec<&str> = content.lines().take(n).collect();
            let truncated = lines.len() < content.lines().count();
            let mut result = lines.join("\n");
            if truncated {
                result.push_str("\n... [truncated]");
            }
            result
        }
        Err(_) => "[unreadable]".to_string(),
    }
}

/// Extract function/struct/class/trait signatures from source code.
/// This is a lightweight heuristic parser — not a full AST.
fn extract_signatures(path: &Path) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return "[unreadable]".to_string(),
    };

    let ext = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    let mut signatures = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if is_signature_line(trimmed, &ext) {
            signatures.push(trimmed.to_string());
        }
    }

    if signatures.is_empty() {
        // Fallback to head if no signatures detected
        return read_head(path, 30);
    }

    signatures.join("\n")
}

/// Heuristic: does this line look like a function/struct/class declaration?
fn is_signature_line(line: &str, ext: &str) -> bool {
    if line.is_empty() || line.starts_with("//") || line.starts_with('#') || line.starts_with("/*") {
        return false;
    }

    match ext.as_ref() {
        // Rust
        "rs" => {
            line.starts_with("pub fn ")
                || line.starts_with("fn ")
                || line.starts_with("pub struct ")
                || line.starts_with("struct ")
                || line.starts_with("pub enum ")
                || line.starts_with("enum ")
                || line.starts_with("pub trait ")
                || line.starts_with("trait ")
                || line.starts_with("impl ")
                || line.starts_with("pub mod ")
                || line.starts_with("mod ")
                || line.starts_with("pub type ")
                || line.starts_with("type ")
        }
        // Python
        "py" => {
            line.starts_with("def ")
                || line.starts_with("async def ")
                || line.starts_with("class ")
                || line.starts_with("    def ")
                || line.starts_with("    async def ")
        }
        // TypeScript / JavaScript
        "ts" | "js" | "tsx" | "jsx" => {
            line.starts_with("export ")
                || line.starts_with("function ")
                || line.starts_with("async function ")
                || line.starts_with("class ")
                || line.starts_with("interface ")
                || line.starts_with("type ")
                || line.starts_with("const ")
                || line.contains("=> {")
        }
        // Go
        "go" => {
            line.starts_with("func ")
                || line.starts_with("type ")
                || line.starts_with("package ")
        }
        // Java / Kotlin
        "java" | "kt" => {
            line.contains("class ")
                || line.contains("interface ")
                || (line.contains("(") && (
                    line.starts_with("public ")
                    || line.starts_with("private ")
                    || line.starts_with("protected ")
                    || line.starts_with("fun ")
                ))
        }
        _ => false,
    }
}
