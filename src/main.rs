mod ast;
mod bytecode;
mod compiler;
mod error;
mod lexer;
mod parser;
mod vm;

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Instant;

use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "pycargo", version, about = "Python compiler — written in Rust")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Compile .py → .pyc bytecode
    Build {
        /// Source file (default: main.py)
        file: Option<PathBuf>,
    },
    /// Compile and run a Python file
    Run {
        /// Source file (default: main.py)
        file: Option<PathBuf>,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("{} {}", "error:".red().bold(), e);
        std::process::exit(1);
    }
}

fn resolve_file(given: Option<PathBuf>) -> Result<PathBuf, String> {
    let path = given.unwrap_or_else(|| PathBuf::from("main.py"));
    if !path.exists() {
        return Err(format!("file not found: {}", path.display()));
    }
    Ok(path)
}

fn compile_file(path: &Path) -> Result<Rc<bytecode::CodeObject>, Box<dyn std::error::Error>> {
    let src = std::fs::read_to_string(path)?;
    let tokens = lexer::tokenize(&src)?;
    let ast = parser::parse(tokens)?;
    let co = compiler::compile_module(&ast)?;
    Ok(Rc::new(co))
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.cmd {
        Cmd::Build { file } => {
            let path = resolve_file(file).map_err(|e| e)?;
            let t = Instant::now();

            eprint!("  {} {} ... ", "Compiling".green().bold(), path.display());

            let co = compile_file(&path)?;

            let out_path = path.with_extension("pyc");
            let bytes = bytecode::serialize(&co);
            std::fs::write(&out_path, bytes)?;

            eprintln!("{}", "done".green());
            eprintln!(
                "   {} {} ({} instructions, {:.2}s)",
                "Written".cyan().bold(),
                out_path.display(),
                co.code.len(),
                t.elapsed().as_secs_f64()
            );
        }

        Cmd::Run { file } => {
            let path = resolve_file(file).map_err(|e| e)?;

            // Try loading existing .pyc first if it's newer
            let pyc_path = path.with_extension("pyc");
            let co = if pyc_path.exists() && is_newer(&pyc_path, &path) {
                let bytes = std::fs::read(&pyc_path)?;
                match bytecode::deserialize(&bytes) {
                    Some(co) => Rc::new(co),
                    None => compile_file(&path)?,
                }
            } else {
                compile_file(&path)?
            };

            let mut machine = vm::VM::new();
            machine.run(co)?;
        }
    }
    Ok(())
}

fn is_newer(a: &Path, b: &Path) -> bool {
    let ta = std::fs::metadata(a).and_then(|m| m.modified()).ok();
    let tb = std::fs::metadata(b).and_then(|m| m.modified()).ok();
    match (ta, tb) {
        (Some(ta), Some(tb)) => ta > tb,
        _ => false,
    }
}
