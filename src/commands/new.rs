use crate::config::{Config, Package, CONFIG_FILE};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::path::Path;

pub fn run(name: &str) -> Result<()> {
    let project_dir = Path::new(name);
    if project_dir.exists() {
        anyhow::bail!("directory `{}` already exists", name);
    }

    std::fs::create_dir_all(project_dir.join("src"))?;
    std::fs::create_dir_all(project_dir.join("tests"))?;

    let config = Config {
        package: Package {
            name: name.to_string(),
            version: "0.1.0".to_string(),
            python: Some("3".to_string()),
            main: Some("src/main.py".to_string()),
            description: None,
        },
        dependencies: HashMap::new(),
        dev_dependencies: {
            let mut m = HashMap::new();
            m.insert("pytest".to_string(), "*".to_string());
            m
        },
    };

    config.save(project_dir)?;

    std::fs::write(
        project_dir.join("src/main.py"),
        format!(
            r#"def main():
    print("Hello from {}!")


if __name__ == "__main__":
    main()
"#,
            name
        ),
    )?;

    std::fs::write(
        project_dir.join("tests/test_main.py"),
        r#"def test_placeholder():
    assert True
"#,
    )?;

    std::fs::write(
        project_dir.join(".gitignore"),
        ".venv/\n__pycache__/\n*.pyc\n*.pyo\ndist/\nbuild/\n*.egg-info/\n",
    )?;

    println!(
        "     {} python package `{}`",
        "Created".green().bold(),
        name
    );
    println!(
        "      {} {}",
        "Config".cyan(),
        project_dir.join(CONFIG_FILE).display()
    );
    println!(
        "        {} {}",
        "Main".cyan(),
        project_dir.join("src/main.py").display()
    );

    Ok(())
}
