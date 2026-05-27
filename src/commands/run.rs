use crate::config::Config;
use crate::python::python_for_project;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn run(project_dir: &Path, config: &Config, args: &[String]) -> Result<()> {
    let python = python_for_project(project_dir, config.package.python.as_deref())?;

    let main = config.main_file(project_dir);
    if !main.exists() {
        anyhow::bail!(
            "main file `{}` not found\nset [package] main = \"path/to/main.py\" in Pycargo.toml",
            main.display()
        );
    }

    println!(
        "    {} `{} {}`",
        "Running".green().bold(),
        config.package.name,
        config.package.version
    );

    let status = Command::new(&python)
        .arg(&main)
        .args(args)
        .current_dir(project_dir)
        .status()?;

    if !status.success() {
        let code = status.code().unwrap_or(1);
        anyhow::bail!("process exited with code {}", code);
    }

    Ok(())
}
