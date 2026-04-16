use std::path::PathBuf;
use clap::Parser;
use aimf_compiler::types::CompilerConfig;

#[derive(Parser, Debug)]
#[command(name = "aimf", about = "Generate AIMF navigation manifests from project directories")]
struct Args {
    /// Path to the project root directory
    #[arg(short, long, default_value = ".")]
    path: String,

    /// GitHub repo identifier (owner/repo)
    #[arg(short, long)]
    repo: Option<String>,

    /// Branch name
    #[arg(short, long)]
    branch: Option<String>,

    /// Path to Claude Code session logs directory (for enrichment)
    #[arg(short, long)]
    sessions: Option<String>,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<String>,

    /// Include source directory entries
    #[arg(long, default_value = "true")]
    include_dirs: bool,
}

fn main() {
    let args = Args::parse();

    let config = CompilerConfig {
        root: PathBuf::from(&args.path),
        repo: args.repo,
        branch: args.branch,
        session_logs: args.sessions.map(PathBuf::from),
        include_source_dirs: args.include_dirs,
        ..CompilerConfig::default()
    };

    if !config.root.exists() {
        eprintln!("Error: path does not exist: {}", args.path);
        std::process::exit(1);
    }

    let aimf = aimf_compiler::compile(&config);

    match args.output {
        Some(path) => {
            std::fs::write(&path, &aimf).expect("Failed to write output file");
            eprintln!("Wrote AIMF manifest to {}", path);
        }
        None => {
            print!("{}", aimf);
        }
    }
}
