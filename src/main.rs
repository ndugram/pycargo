mod commands;
mod config;
mod python;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "pycargo",
    version,
    about = "Python project manager — like Cargo, but for Python",
    long_about = None
)]
struct Cli {
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Create new Python project
    New {
        /// Project name
        name: String,
    },
    /// Compile Python source to bytecode
    Build,
    /// Run the project
    Run {
        /// Arguments passed to the Python script
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run tests with pytest
    Test {
        /// Arguments passed to pytest
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Add dependency to Pycargo.toml and install it
    Add {
        /// Package(s) to add
        packages: Vec<String>,
        /// Add as dev dependency
        #[arg(long)]
        dev: bool,
    },
    /// Install all dependencies from Pycargo.toml
    Install {
        /// Also install dev-dependencies
        #[arg(long)]
        dev: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Cmd::New { name } => {
            commands::new::run(&name)?;
        }
        Cmd::Build => {
            let root = config::find_project_root()?;
            let cfg = config::Config::load(&root)?;
            commands::build::run(&root, &cfg)?;
        }
        Cmd::Run { args } => {
            let root = config::find_project_root()?;
            let cfg = config::Config::load(&root)?;
            commands::run::run(&root, &cfg, &args)?;
        }
        Cmd::Test { args } => {
            let root = config::find_project_root()?;
            let cfg = config::Config::load(&root)?;
            commands::test::run(&root, &cfg, &args)?;
        }
        Cmd::Add { packages, dev } => {
            let root = config::find_project_root()?;
            let mut cfg = config::Config::load(&root)?;
            if packages.is_empty() {
                anyhow::bail!("specify at least one package, e.g. `pycargo add requests`");
            }
            commands::add::run(&root, &mut cfg, &packages, dev)?;
        }
        Cmd::Install { dev } => {
            let root = config::find_project_root()?;
            let cfg = config::Config::load(&root)?;
            commands::install::run(&root, &cfg, dev)?;
        }
    }
    Ok(())
}
