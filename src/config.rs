use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const CONFIG_FILE: &str = "Pycargo.toml";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub python: Option<String>,
    pub main: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Config {
    pub package: Package,
    #[serde(default)]
    pub dependencies: HashMap<String, String>,
    #[serde(rename = "dev-dependencies", default)]
    pub dev_dependencies: HashMap<String, String>,
}

impl Default for Package {
    fn default() -> Self {
        Package {
            name: String::from("my_project"),
            version: String::from("0.1.0"),
            python: Some(String::from("3")),
            main: Some(String::from("src/main.py")),
            description: None,
        }
    }
}

impl Config {
    pub fn load(dir: &Path) -> Result<Self> {
        let path = dir.join(CONFIG_FILE);
        if !path.exists() {
            bail!(
                "no {} found in `{}`\nrun `pycargo new` to create project",
                CONFIG_FILE,
                dir.display()
            );
        }
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let config: Config = toml::from_str(&content)
            .with_context(|| format!("invalid {}", CONFIG_FILE))?;
        Ok(config)
    }

    pub fn save(&self, dir: &Path) -> Result<()> {
        let path = dir.join(CONFIG_FILE);
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn main_file(&self, project_dir: &Path) -> PathBuf {
        let main = self
            .package
            .main
            .as_deref()
            .unwrap_or("src/main.py");
        project_dir.join(main)
    }
}

pub fn find_project_root() -> Result<PathBuf> {
    let mut current = std::env::current_dir()?;
    loop {
        if current.join(CONFIG_FILE).exists() {
            return Ok(current);
        }
        if !current.pop() {
            bail!(
                "could not find `{}` in current directory or any parent",
                CONFIG_FILE
            );
        }
    }
}
