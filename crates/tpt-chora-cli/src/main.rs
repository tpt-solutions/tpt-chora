mod css_report;
mod doctor;
mod new_app;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "tpt-chora",
    about = "tpt-chora adoption tooling — diagnostics, scaffolding, and CSS compatibility reports",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Doctor,
    New {
        name: String,
    },
    CssReport {
        path: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Doctor => doctor::run(),
        Commands::New { name } => new_app::run(&name),
        Commands::CssReport { path } => css_report::run(&path),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
