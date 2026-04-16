#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::path::PathBuf;
use aimf_compiler::types::CompilerConfig;

#[tauri::command]
fn compile_project(path: String, repo: Option<String>, branch: Option<String>) -> Result<aimf_compiler::CompileResult, String> {
    let config = CompilerConfig {
        root: PathBuf::from(&path),
        repo,
        branch,
        ..CompilerConfig::default()
    };

    // Verify the path exists
    if !config.root.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    Ok(aimf_compiler::compile_with_stats(&config))
}

#[tauri::command]
fn save_aimf(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, &content).map_err(|e| format!("Failed to write: {}", e))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .invoke_handler(tauri::generate_handler![compile_project, save_aimf])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
