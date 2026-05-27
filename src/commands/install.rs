use crate::config::Config;
use crate::python::{find_python, venv_exists, venv_pip};
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn run(project_dir: &Path, config: &Config, dev: bool) -> Result<()> {
    ensure_venv(project_dir, config)?;

    let pip = venv_pip(project_dir);

    let mut packages: Vec<String> = config
        .dependencies
        .iter()
        .map(|(name, ver)| {
            if ver == "*" {
                name.clone()
            } else {
                format!("{}=={}", name, ver)
            }
        })
        .collect();

    if dev {
        for (name, ver) in &config.dev_dependencies {
            packages.push(if ver == "*" {
                name.clone()
            } else {
                format!("{}=={}", name, ver)
            });
        }
    }

    if packages.is_empty() {
        println!("{} no dependencies to install", "  Info".cyan().bold());
        return Ok(());
    }

    println!(
        "  {} dependencies...",
        "Installing".green().bold()
    );

    let status = Command::new(&pip)
        .args(["install", "--quiet"])
        .args(&packages)
        .status()?;

    if !status.success() {
        anyhow::bail!("pip install failed");
    }

    println!(
        "   {} {} package(s)",
        "Installed".green().bold(),
        packages.len()
    );
    Ok(())
}

pub fn ensure_venv(project_dir: &Path, config: &Config) -> Result<()> {
    if venv_exists(project_dir) {
        return Ok(());
    }

    let python_ver = config.package.python.as_deref();
    let python = find_python(python_ver)?;

    println!(
        "    {} virtual environment",
        "Creating".green().bold()
    );

    let venv_path = project_dir.join(".venv");
    let status = Command::new(&python)
        .args(["-m", "venv"])
        .arg(&venv_path)
        .status()?;

    if !status.success() {
        anyhow::bail!("failed to create virtual environment");
    }

    println!(
        "    {} .venv at {}",
        "Created".green().bold(),
        venv_path.display()
    );
    Ok(())
}
