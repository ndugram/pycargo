<p align="center">
  <span style="font-size:80px">🐍📦</span>
</p>
<h1 align="center">pycargo</h1>
<p align="center">
    <em>Cargo-style project manager for Python — written in Rust.</em>
</p>
<p align="center">
<a href="https://github.com/ndugram/pycargo/actions/workflows/release.yml" target="_blank">
    <img src="https://github.com/ndugram/pycargo/actions/workflows/release.yml/badge.svg" alt="Release">
</a>
<a href="https://github.com/ndugram/pycargo/releases/latest" target="_blank">
    <img src="https://img.shields.io/github/v/release/ndugram/pycargo?color=%23orange&label=version" alt="Latest release">
</a>
<a href="https://github.com/ndugram/pycargo/releases" target="_blank">
    <img src="https://img.shields.io/github/downloads/ndugram/pycargo/total?color=%23orange" alt="Downloads">
</a>
<a href="https://github.com/ndugram/pycargo" target="_blank">
    <img src="https://img.shields.io/github/stars/ndugram/pycargo?style=social" alt="GitHub Stars">
</a>
</p>

---

**Source Code**: <a href="https://github.com/ndugram/pycargo" target="_blank">https://github.com/ndugram/pycargo</a>

---

**pycargo** is a fast, Rust-powered CLI tool that brings the Cargo experience to Python projects — project creation, dependency management, virtual environments, building, running, and testing, all from one command.

Key features:

- **`pycargo new`** — scaffold a new Python project with sensible defaults.
- **`pycargo build`** — compile Python source to `.pyc` bytecode with syntax checking.
- **`pycargo run`** — run your project, automatically using the project's virtualenv.
- **`pycargo test`** — run pytest with one command.
- **`pycargo add`** — add a dependency, install it, and update `Pycargo.toml` in one step.
- **`pycargo install`** — install all dependencies from `Pycargo.toml` into a `.venv`.
- **`Pycargo.toml`** — single config file for package metadata and dependencies.

## Installation

Download the binary for your platform from the [latest release](https://github.com/ndugram/pycargo/releases/latest) and put it on your `PATH`:

```console
# macOS / Linux
chmod +x pycargo-*
sudo mv pycargo-* /usr/local/bin/pycargo

# Verify
pycargo --version
```

Or build from source (requires Rust):

```console
cargo install --git https://github.com/ndugram/pycargo
```

## Quick Start

```console
# Create a new project
pycargo new my_app
cd my_app

# Build (compiles .py → .pyc, checks syntax)
pycargo build

# Run
pycargo run

# Run with arguments
pycargo run -- --debug --verbose

# Add a dependency (installs it + updates Pycargo.toml)
pycargo add requests
pycargo add pytest --dev

# Install all dependencies from Pycargo.toml
pycargo install
pycargo install --dev

# Run tests
pycargo test
pycargo test -- -v -k "test_auth"
```

## Pycargo.toml

```toml
[package]
name = "my_app"
version = "0.1.0"
python = "3"
main = "src/main.py"
description = "My Python application"

[dependencies]
requests = "2.32.3"
pydantic = "2.7.0"

[dev-dependencies]
pytest = "*"
```

## Project Structure

`pycargo new my_app` generates:

```
my_app/
├── Pycargo.toml
├── .gitignore
├── src/
│   └── main.py
└── tests/
    └── test_main.py
```

## Commands

| Command | Description |
|---|---|
| `pycargo new <name>` | Create a new Python project |
| `pycargo build` | Compile source to `.pyc` bytecode |
| `pycargo run [-- <args>]` | Run the project |
| `pycargo test [-- <args>]` | Run tests with pytest |
| `pycargo add <pkg> [--dev]` | Add and install a dependency |
| `pycargo install [--dev]` | Install all dependencies |

## Contributing

Contributions are welcome! Please open an issue or pull request.

Found a security issue? Contact [yap8572@gmail.com](mailto:yap8572@gmail.com) directly.

## License

MIT
