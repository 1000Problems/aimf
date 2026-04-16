use std::path::PathBuf;
use clap::Parser;
use aimf_compiler::types::CompilerConfig;

#[derive(Parser, Debug)]
#[command(name = "aimf", about = "Compile a project into AIMF format")]
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

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<String>,

    /// Maximum number of hot files
    #[arg(long, default_value = "15")]
    max_hot: usize,

    /// Maximum hot file size in bytes
    #[arg(long, default_value = "2048")]
    max_hot_size: u64,

    /// Token budget for hot memory section
    #[arg(long, default_value = "100000")]
    token_budget: usize,
}

fn main() {
    let args = Args::parse();

    let config = CompilerConfig {
        root: PathBuf::from(&args.path),
        repo: args.repo,
        branch: args.branch,
        max_hot_files: args.max_hot,
        max_hot_file_size: args.max_hot_size,
        token_budget: args.token_budget,
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
            eprintln!("Wrote AIMF to {}", path);
        }
        None => {
            print!("{}", aimf);
        }
    }
}
