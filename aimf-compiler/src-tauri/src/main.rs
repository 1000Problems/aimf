#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[cfg(feature = "tauri")]
mod tauri_app {
    use std::path::PathBuf;
    use aimf_compiler::types::CompilerConfig;

    #[tauri::command]
    pub fn compile_project(path: String, repo: Option<String>, branch: Option<String>) -> Result<String, String> {
        let config = CompilerConfig {
            root: PathBuf::from(&path),
            repo,
            branch,
            ..CompilerConfig::default()
        };

        if !config.root.exists() {
            return Err(format!("Path does not exist: {}", path));
        }

        Ok(aimf_compiler::compile(&config))
    }

    #[tauri::command]
    pub fn save_aimf(path: String, content: String) -> Result<(), String> {
        std::fs::write(&path, &content).map_err(|e| format!("Failed to write: {}", e))
    }

    pub fn run() {
        tauri::Builder::default()
            .invoke_handler(tauri::generate_handler![compile_project, save_aimf])
            .run(tauri::generate_context!())
            .expect("error while running tauri application");
    }
}

fn main() {
    #[cfg(feature = "tauri")]
    tauri_app::run();

    #[cfg(not(feature = "tauri"))]
    {
        eprintln!("AIMF Compiler: use --features cli for CLI mode or --features tauri for desktop app");
        std::process::exit(1);
    }
}
