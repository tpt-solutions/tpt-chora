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
    New { name: String },
    CssReport { path: String },
}

fn main() {
    let cli = Cli::parse();
    let result: Result<(), Box<dyn std::error::Error>> = match cli.command {
        Commands::Doctor => doctor::run().map_err(Into::into),
        Commands::New { name } => new_app::run(&name).map_err(Into::into),
        Commands::CssReport { path } => css_report::run(&path).map_err(Into::into),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
