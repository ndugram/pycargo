use crate::config::Config;
use crate::python::python_for_project;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn run(project_dir: &Path, config: &Config, args: &[String]) -> Result<()> {
    let python = python_for_project(project_dir, config.package.python.as_deref())?;

    println!(
        "     {} {} v{} (tests)",
        "Testing".green().bold(),
        config.package.name,
        config.package.version
    );

    let status = Command::new(&python)
        .args(["-m", "pytest"])
        .args(args)
        .current_dir(project_dir)
        .status();

    match status {
        Ok(s) if s.success() => Ok(()),
        Ok(s) => {
            let code = s.code().unwrap_or(1);
            anyhow::bail!("tests failed (exit code {})", code)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            anyhow::bail!(
                "pytest not found\nrun `pycargo add pytest --dev` to install it"
            )
        }
        Err(e) => Err(e.into()),
    }
}
