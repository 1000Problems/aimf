use std::path::Path;
use crate::types::ResourceType;

/// Classify a file into a ResourceType based on its path and extension.
pub fn classify(path: &Path) -> ResourceType {
    let path_str = path.to_string_lossy().to_lowercase();
    let ext = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let filename = path.file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Test files — check before SRC since test files are also source
    if is_test(&path_str, &filename) {
        return ResourceType::TEST;
    }

    // Mermaid diagrams
    if ext == "mmd" || ext == "mermaid" {
        return ResourceType::MRM;
    }

    // Source code
    if is_source(&ext) {
        return ResourceType::SRC;
    }

    // Configuration
    if is_config(&ext, &filename) {
        return ResourceType::CFG;
    }

    // Documentation
    if is_doc(&ext, &filename) {
        return ResourceType::DOC;
    }

    // Binary / asset files
    if is_binary(&ext) {
        return ResourceType::BIN;
    }

    // Meta files
    if is_meta(&filename, &path_str) {
        return ResourceType::META;
    }

    ResourceType::OTHER
}

/// Generate a short semantic hint for a resource.
pub fn generate_hint(path: &Path, resource_type: &ResourceType) -> String {
    let filename = path.file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_default();
    let parent = path.parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let hint = match resource_type {
        ResourceType::SRC => {
            if is_entry_point(path) {
                format!("entry point")
            } else if !parent.is_empty() {
                format!("{} module", parent.replace('/', "."))
            } else {
                format!("source file")
            }
        }
        ResourceType::CFG => {
            if filename.contains("cargo") || filename.contains("package") {
                "build config and dependencies".to_string()
            } else if filename.contains("tsconfig") || filename.contains("jest") {
                "toolchain config".to_string()
            } else {
                format!("configuration")
            }
        }
        ResourceType::DOC => {
            if filename.starts_with("readme") {
                "project overview".to_string()
            } else if filename.contains("changelog") {
                "change history".to_string()
            } else if filename.contains("contributing") {
                "contribution guide".to_string()
            } else {
                "documentation".to_string()
            }
        }
        ResourceType::TEST => "test suite".to_string(),
        ResourceType::MRM => "architecture/flow diagram".to_string(),
        ResourceType::BIN => "binary asset".to_string(),
        ResourceType::META => "project metadata".to_string(),
        ResourceType::OTHER => "project file".to_string(),
    };

    // Truncate to 60 chars
    if hint.len() > 60 {
        format!("{}...", &hint[..57])
    } else {
        hint
    }
}

/// Check if a file is an entry point.
pub fn is_entry_point(path: &Path) -> bool {
    let filename = path.file_name()
        .map(|f| f.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    matches!(
        filename.as_str(),
        "main.rs" | "main.py" | "main.go" | "main.ts" | "main.js"
        | "index.ts" | "index.js" | "index.tsx" | "index.jsx"
        | "app.py" | "app.ts" | "app.js" | "app.rs"
        | "lib.rs" | "mod.rs"
        | "server.ts" | "server.js" | "server.py"
    )
}

fn is_test(path: &str, filename: &str) -> bool {
    path.contains("test/") || path.contains("tests/")
        || path.contains("__tests__/") || path.contains("spec/")
        || filename.contains("_test.") || filename.contains(".test.")
        || filename.contains("_spec.") || filename.contains(".spec.")
        || filename.starts_with("test_")
}

fn is_source(ext: &str) -> bool {
    matches!(
        ext,
        "rs" | "py" | "js" | "ts" | "tsx" | "jsx" | "go" | "java"
        | "kt" | "scala" | "rb" | "php" | "c" | "cpp" | "h" | "hpp"
        | "cs" | "swift" | "m" | "mm" | "zig" | "nim" | "lua"
        | "ex" | "exs" | "erl" | "hrl" | "clj" | "cljs" | "vue"
        | "svelte" | "dart" | "r" | "jl" | "hs" | "ml" | "fs"
        | "elm" | "v" | "sol" | "move" | "cairo"
    )
}

fn is_config(ext: &str, filename: &str) -> bool {
    matches!(ext, "json" | "yaml" | "yml" | "toml" | "ini" | "cfg" | "conf")
        || filename == ".env"
        || filename == ".env.example"
        || filename.ends_with("rc")
        || filename.starts_with(".")
            && (filename.contains("config") || filename.contains("rc"))
}

fn is_doc(ext: &str, filename: &str) -> bool {
    matches!(ext, "md" | "rst" | "txt" | "adoc" | "org")
        || filename == "license"
        || filename == "licence"
        || filename.starts_with("readme")
        || filename.starts_with("changelog")
        || filename.starts_with("contributing")
}

fn is_binary(ext: &str) -> bool {
    matches!(
        ext,
        "png" | "jpg" | "jpeg" | "gif" | "svg" | "ico" | "webp"
        | "pdf" | "zip" | "tar" | "gz" | "bz2" | "7z" | "rar"
        | "woff" | "woff2" | "ttf" | "eot" | "otf"
        | "mp3" | "mp4" | "wav" | "avi" | "mov" | "webm"
        | "wasm" | "so" | "dll" | "dylib" | "exe" | "bin"
        | "db" | "sqlite" | "sqlite3"
    )
}

fn is_meta(filename: &str, path: &str) -> bool {
    matches!(
        filename,
        ".gitignore" | ".gitattributes" | ".editorconfig"
        | ".prettierrc" | ".eslintrc" | ".eslintignore"
        | "dockerfile" | "docker-compose.yml" | "docker-compose.yaml"
        | "makefile" | "justfile" | "taskfile.yml"
        | ".github" | ".gitlab-ci.yml" | ".circleci"
    ) || path.contains(".github/")
        || path.contains(".circleci/")
        || path.contains(".gitlab/")
}
