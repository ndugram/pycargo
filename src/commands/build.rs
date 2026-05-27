use crate::config::Config;
use crate::python::python_for_project;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

pub fn run(project_dir: &Path, config: &Config) -> Result<()> {
    let python = python_for_project(project_dir, config.package.python.as_deref())?;

    let src_dir = project_dir.join("src");
    if !src_dir.exists() {
        anyhow::bail!("no `src/` directory found");
    }

    println!(
        "   {} {} v{}",
        "Compiling".green().bold(),
        config.package.name,
        config.package.version
    );

    let t = Instant::now();

    // syntax check first
    let check = Command::new(&python)
        .args(["-m", "py_compile"])
        .arg(
            config
                .package
                .main
                .as_deref()
                .map(|m| project_dir.join(m))
                .unwrap_or_else(|| src_dir.join("main.py")),
        )
        .current_dir(project_dir)
        .output()?;

    if !check.status.success() {
        let stderr = String::from_utf8_lossy(&check.stderr);
        anyhow::bail!("syntax error:\n{}", stderr.trim());
    }

    // compile all .py files to __pycache__/*.pyc
    let status = Command::new(&python)
        .args(["-m", "compileall", "-q", "src"])
        .current_dir(project_dir)
        .status()?;

    if !status.success() {
        anyhow::bail!("compilation failed");
    }

    let elapsed = t.elapsed();
    println!(
        "   {} {} v{} ({:.2}s)",
        "Finished".green().bold(),
        config.package.name,
        config.package.version,
        elapsed.as_secs_f64()
    );

    Ok(())
}
