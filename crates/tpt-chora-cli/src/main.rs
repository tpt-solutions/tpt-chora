#![forbid(unsafe_code)]

mod audit;
mod css_report;
mod doctor;
mod new_app;
mod preview;

use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::{generate, Shell};

#[derive(Parser)]
#[command(
    name = "tpt-chora",
    about = "tpt-chora adoption tooling — diagnostics, scaffolding, and CSS compatibility reports",
    long_about = "tpt-chora CLI provides commands for diagnosing your tpt-chora setup,
scaffolding new projects, checking CSS compatibility, auditing supply-chain
security, and generating shell completions.

Usage examples:
  tpt-chora doctor              Report wgpu backend and adapter info
  tpt-chora new my-app          Scaffold a new tpt-chora project
  tpt-chora css-report style.css  Check CSS against Eidos IR compatibility
  tpt-chora audit               Run a consolidated health and security audit
  tpt-chora preview ./project   Dev-loop: re-render PNG on file changes
  tpt-chora completions bash > completions.bash
",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Report wgpu backend/adapter info, toolchain sanity, and Tier 1 fallback status.
    ///
    /// Example:
    ///   tpt-chora doctor
    Doctor,
    /// Scaffold a new tpt-chora project crate with a starter .eidos file.
    ///
    /// Example:
    ///   tpt-chora new my-app
    New { name: String },
    /// Run the Rosetta Stone CSS-to-Eidos transpiler over a CSS file and print a compatibility score.
    ///
    /// Example:
    ///   tpt-chora css-report ./styles/main.css
    CssReport { path: String },
    /// Generate shell completions for a given shell.
    ///
    /// Example:
    ///   tpt-chora completions bash > completions.bash
    ///   tpt-chora completions zsh > completions.zsh
    Completions { shell: Shell },
    /// Watch a project directory and re-render a PNG snapshot on every file change.
    ///
    /// Example:
    ///   tpt-chora preview ./my-project
    Preview { project_dir: String },
    /// Run a consolidated health and security audit: doctor diagnostics, cargo-deny check, and native-backend feature status.
    ///
    /// Example:
    ///   tpt-chora audit
    Audit,
}

fn main() {
    let cli = Cli::parse();
    let result: Result<(), Box<dyn std::error::Error>> = match cli.command {
        Commands::Doctor => doctor::run().map_err(Into::into),
        Commands::New { name } => new_app::run(&name).map_err(Into::into),
        Commands::CssReport { path } => css_report::run(&path).map_err(Into::into),
        Commands::Completions { shell } => {
            generate(
                shell,
                &mut Cli::command(),
                "tpt-chora",
                &mut std::io::stdout(),
            );
            Ok(())
        }
        Commands::Preview { project_dir } => preview::run(&project_dir).map_err(Into::into),
        Commands::Audit => audit::run().map_err(Into::into),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
