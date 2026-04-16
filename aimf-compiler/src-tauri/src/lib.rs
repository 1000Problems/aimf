pub mod types;
pub mod scanner;
pub mod classifier;
pub mod partitioner;
pub mod hot_strategy;
pub mod emitter;

use types::CompilerConfig;
use scanner::scan;
use partitioner::partition;
use emitter::emit;

/// Run the full AIMF compilation pipeline.
pub fn compile(config: &CompilerConfig) -> String {
    // Stage 1: Scan
    let files = scan(config);

    // Stage 2+3: Classify + Partition (combined)
    let partition_result = partition(&files, config);

    // Stage 4: Emit
    emit(config, &files, &partition_result)
}

/// Compile and return structured metadata (for Tauri UI).
pub fn compile_with_stats(config: &CompilerConfig) -> CompileResult {
    let files = scan(config);
    let partition_result = partition(&files, config);
    let aimf = emit(config, &files, &partition_result);

    CompileResult {
        aimf,
        total_files: partition_result.resources.len(),
        hot_count: partition_result.hot_ids.len(),
        group_count: partition_result.groups.len(),
        estimated_tokens: 0, // TODO: implement token estimation
    }
}

#[derive(serde::Serialize)]
pub struct CompileResult {
    pub aimf: String,
    pub total_files: usize,
    pub hot_count: usize,
    pub group_count: usize,
    pub estimated_tokens: usize,
}
