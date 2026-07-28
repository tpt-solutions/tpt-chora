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
    /// Generate shell completions, e.g. `tpt-chora completions bash > completions.bash`.
    Completions {
        shell: Shell,
    },
    /// Watch a project directory and re-render a PNG snapshot on every
    /// .eidos/shader/asset file change.
    Preview {
        project_dir: String,
    },
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
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
