pub mod types;
pub mod scanner;
pub mod classifier;
pub mod curator;
pub mod emitter;
pub mod session_miner;

use std::collections::HashMap;
use types::{AimfDocument, CompilerConfig};

/// Run the AIMF v2 compilation pipeline.
///
/// Scans a project directory, discovers meaningful resources,
/// classifies them, and emits a lightweight navigation manifest.
pub fn compile(config: &CompilerConfig) -> String {
    let doc = compile_to_doc(config);
    emitter::emit(&doc)
}

/// Compile and return the structured document (for Tauri UI or programmatic use).
pub fn compile_to_doc(config: &CompilerConfig) -> AimfDocument {
    // Stage 1: Scan for meaningful resources
    let discovered = scanner::scan(config);

    // Stage 2: Mine session logs for enrichment (optional)
    let enrichments = match &config.session_logs {
        Some(session_dir) => session_miner::mine_sessions(session_dir),
        None => HashMap::new(),
    };

    // Stage 3: Curate — classify, describe, assign IDs, estimate tokens
    let (nav, ctx) = curator::curate(&discovered, config, &enrichments);

    AimfDocument { nav, ctx }
}

/// Generate a task handoff document for Sonnet.
pub fn generate_task(
    config: &CompilerConfig,
    task_title: &str,
    task_body: &str,
    resource_ids: &[String], // Which @NAV entries to include
) -> String {
    let doc = compile_to_doc(config);

    // Filter nav to only requested resources
    let filtered_nav: Vec<_> = doc.nav.into_iter()
        .filter(|n| resource_ids.contains(&n.id) || resource_ids.iter().any(|r| r == "*"))
        .collect();

    emitter::emit_task(&filtered_nav, &doc.ctx, task_title, task_body)
}
