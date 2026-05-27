use crate::config::Config;
use crate::python::venv_pip;
use crate::commands::install::ensure_venv;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;
use std::process::Command;

pub fn run(
    project_dir: &Path,
    config: &mut Config,
    packages: &[String],
    dev: bool,
) -> Result<()> {
    ensure_venv(project_dir, config)?;

    let pip = venv_pip(project_dir);

    println!(
        "      {} {}",
        "Adding".green().bold(),
        packages.join(", ")
    );

    let status = Command::new(&pip)
        .arg("install")
        .args(packages)
        .status()?;

    if !status.success() {
        anyhow::bail!("pip install failed");
    }

    // resolve actual installed version
    for pkg in packages {
        let pkg_name = pkg.split(['=', '<', '>', '!']).next().unwrap_or(pkg);
        let version = get_installed_version(&pip, pkg_name).unwrap_or_else(|| "*".to_string());

        if dev {
            config
                .dev_dependencies
                .insert(pkg_name.to_string(), version.clone());
        } else {
            config
                .dependencies
                .insert(pkg_name.to_string(), version.clone());
        }

        println!(
            "       {} {}=={} to {}",
            "Added".green().bold(),
            pkg_name,
            version,
            if dev { "dev-dependencies" } else { "dependencies" }
        );
    }

    config.save(project_dir)?;
    Ok(())
}

fn get_installed_version(pip: &Path, package: &str) -> Option<String> {
    let out = Command::new(pip)
        .args(["show", package])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if let Some(ver) = line.strip_prefix("Version: ") {
            return Some(ver.trim().to_string());
        }
    }
    None
}
