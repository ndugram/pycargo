use anyhow::{bail, Result};
use std::path::{Path, PathBuf};
use std::process::Command;

pub fn find_python(requested: Option<&str>) -> Result<PathBuf> {
    let candidates: Vec<&str> = match requested {
        Some(v) => vec![
            Box::leak(format!("python{}", v).into_boxed_str()),
            Box::leak(format!("python{}", v.split('.').next().unwrap_or(v)).into_boxed_str()),
            "python3",
            "python",
        ],
        None => vec!["python3", "python"],
    };

    for name in candidates {
        if let Ok(out) = Command::new(name).arg("--version").output() {
            if out.status.success() {
                return Ok(PathBuf::from(name));
            }
        }
    }
    bail!("python not found — install Python and make sure it's on PATH")
}

pub fn venv_python(project_dir: &Path) -> PathBuf {
    let venv = project_dir.join(".venv");
    #[cfg(windows)]
    return venv.join("Scripts").join("python.exe");
    #[cfg(not(windows))]
    return venv.join("bin").join("python");
}

pub fn venv_pip(project_dir: &Path) -> PathBuf {
    let venv = project_dir.join(".venv");
    #[cfg(windows)]
    return venv.join("Scripts").join("pip.exe");
    #[cfg(not(windows))]
    return venv.join("bin").join("pip");
}

pub fn venv_exists(project_dir: &Path) -> bool {
    venv_python(project_dir).exists()
}

pub fn python_for_project(project_dir: &Path, requested: Option<&str>) -> Result<PathBuf> {
    if venv_exists(project_dir) {
        Ok(venv_python(project_dir))
    } else {
        find_python(requested)
    }
}
