use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Resource Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    MRM,   // Mermaid diagram
    DEC,   // Decision document
    DIR,   // Source directory (logical unit)
    CFG,   // Configuration file
    DOC,   // Documentation
    SPEC,  // Specification
    SRC,   // Single source file
    TASK,  // Task document
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::MRM => "MRM",
            Self::DEC => "DEC",
            Self::DIR => "DIR",
            Self::CFG => "CFG",
            Self::DOC => "DOC",
            Self::SPEC => "SPEC",
            Self::SRC => "SRC",
            Self::TASK => "TASK",
        }
    }
}

// ─── Navigation Entry ───────────────────────────────────────────

/// A single entry in the @NAV section — one resource Claude should know about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NavEntry {
    pub id: String,              // Short, meaningful: ARCH, AUTH, D001
    pub resource_type: ResourceType,
    pub path: PathBuf,           // Relative to project root
    pub about: String,           // One-line description, max 80 chars
    pub tokens: usize,           // Estimated token cost to load
    pub load_hints: Vec<String>, // "design", "task:auth", "reference", etc.
}

// ─── Working Context ────────────────────────────────────────────

/// A key-value pair in the @CTX section.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CtxEntry {
    pub key: String,
    pub value: String,
}

// ─── AIMF Document ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimfDocument {
    pub nav: Vec<NavEntry>,
    pub ctx: Vec<CtxEntry>,
}

// ─── Compiler Configuration ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    pub root: PathBuf,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub session_logs: Option<PathBuf>,  // Optional: path to .claude/projects/ session dir
    pub ignore_patterns: Vec<String>,
    pub include_source_dirs: bool,      // Whether to add DIR entries for source directories
    pub max_about_len: usize,           // Max length for about field (default 80)
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            repo: None,
            branch: None,
            session_logs: None,
            ignore_patterns: vec![
                ".git".into(),
                "node_modules".into(),
                "target".into(),
                "__pycache__".into(),
                ".DS_Store".into(),
                "dist".into(),
                "build".into(),
                "gen".into(),
            ],
            include_source_dirs: true,
            max_about_len: 80,
        }
    }
}
