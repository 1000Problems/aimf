use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ─── Resource Types ─────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ResourceType {
    SRC,
    CFG,
    DOC,
    TEST,
    MRM,
    BIN,
    META,
    OTHER,
}

impl ResourceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SRC => "SRC",
            Self::CFG => "CFG",
            Self::DOC => "DOC",
            Self::TEST => "TEST",
            Self::MRM => "MRM",
            Self::BIN => "BIN",
            Self::META => "META",
            Self::OTHER => "OTHER",
        }
    }
}

// ─── Load Strategy ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadWhen {
    Always,
    OnEdit,
    OnRequest,
    OnError,
    OnGroup,
    Never,
}

impl LoadWhen {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::OnEdit => "on_edit",
            Self::OnRequest => "on_request",
            Self::OnError => "on_error",
            Self::OnGroup => "on_group",
            Self::Never => "never",
        }
    }
}

// ─── Hot Payload Strategy ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HotStrategy {
    Full,
    Summary,
    Head(usize),      // first N lines
    Signature,        // function/struct signatures only
    Delta,            // diff since last compile
}

// ─── Core Data Structures ───────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub id: String,           // F1, F2, ...
    pub resource_type: ResourceType,
    pub load_when: LoadWhen,
    pub path: PathBuf,        // relative to root
    pub size: u64,
    pub hash: String,         // 8-char hex
    pub group_id: String,     // G1, G2, ... or "-"
    pub hint: String,         // semantic hint, max 60 chars
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Group {
    pub id: String,           // G1, G2, ...
    pub label: String,
    pub patterns: Vec<String>, // glob patterns
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotEntry {
    pub key: String,          // ResourceID or special key (CTX, ARCH, DEPS, etc.)
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdBlock {
    pub resource_id: String,
    pub payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    pub version: u32,
    pub resource_count: usize,
    pub hot_count: usize,
    pub group_count: usize,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub timestamp: String,
    pub root: PathBuf,
    pub shard: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AimfDocument {
    pub header: Header,
    pub groups: Vec<Group>,
    pub resources: Vec<Resource>,
    pub hot_entries: Vec<HotEntry>,
    pub cold_blocks: Vec<ColdBlock>,
}

// ─── Compiler Configuration ─────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompilerConfig {
    pub root: PathBuf,
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub max_hot_files: usize,
    pub max_hot_file_size: u64,      // bytes — files larger than this get summarized
    pub summary_threshold: u64,      // bytes — files larger than this get head-only
    pub token_budget: usize,         // max tokens for hot memory section
    pub ignore_patterns: Vec<String>,
    pub custom_groups: Option<Vec<GroupConfig>>,
    pub shard_threshold: usize,      // resource count to trigger sharding
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroupConfig {
    pub label: String,
    pub patterns: Vec<String>,
}

impl Default for CompilerConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            repo: None,
            branch: None,
            max_hot_files: 15,
            max_hot_file_size: 2048,
            summary_threshold: 20480,
            token_budget: 100_000,
            ignore_patterns: vec![
                ".git/**".into(),
                "node_modules/**".into(),
                "target/**".into(),
                "__pycache__/**".into(),
                "*.pyc".into(),
                ".DS_Store".into(),
                "*.lock".into(),
                "dist/**".into(),
                "build/**".into(),
            ],
            custom_groups: None,
            shard_threshold: 500,
        }
    }
}
