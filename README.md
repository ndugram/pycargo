<p align="center">
  <span style="font-size:80px">🐍⚡</span>
</p>
<h1 align="center">pycargo</h1>
<p align="center">
    <em>Python compiler with its own bytecode VM — written in Rust.</em>
</p>
<p align="center">
<a href="https://github.com/ndugram/pycargo/actions/workflows/release.yml" target="_blank">
    <img src="https://github.com/ndugram/pycargo/actions/workflows/release.yml/badge.svg" alt="Release">
</a>
<a href="https://github.com/ndugram/pycargo/releases/latest" target="_blank">
    <img src="https://img.shields.io/github/v/release/ndugram/pycargo?color=%23FF6B35&label=version" alt="Latest release">
</a>
<a href="https://github.com/ndugram/pycargo/releases" target="_blank">
    <img src="https://img.shields.io/github/downloads/ndugram/pycargo/total?color=%23FF6B35" alt="Downloads">
</a>
<a href="https://github.com/ndugram/pycargo" target="_blank">
    <img src="https://img.shields.io/github/stars/ndugram/pycargo?style=social" alt="GitHub Stars">
</a>
</p>

---

**Source Code**: <a href="https://github.com/ndugram/pycargo" target="_blank">https://github.com/ndugram/pycargo</a>

---

**pycargo** is a Python compiler written entirely in Rust. It compiles Python source files to custom bytecode (`.pyc`) and executes them on its own stack-based VM — no CPython, no pip, no dependencies.

Pipeline: `.py` → **Lexer** → **AST** → **Compiler** → **Bytecode** → **VM**

Key features:

- **Own bytecode format** — `.pyc` files use a custom binary format, not CPython's.
- **Own VM** — stack-based virtual machine implemented in Rust, zero Python runtime.
- **Smart caching** — `pycargo run` reuses existing `.pyc` if source hasn't changed.
- **Full compiler pipeline** — lexer, recursive-descent parser, AST compiler, bytecode VM.
- **Zero dependencies at runtime** — single binary, works anywhere.

## Installation

Download the binary for your platform from the [latest release](https://github.com/ndugram/pycargo/releases/latest):

```console
# macOS / Linux
chmod +x pycargo-*
sudo mv pycargo-* /usr/local/bin/pycargo

pycargo --version
```

Build from source (requires Rust):

```console
cargo install --git https://github.com/ndugram/pycargo
```

## Usage

```console
# Compile .py → .pyc bytecode
pycargo build hello.py
#   Compiling hello.py ... done
#    Written hello.pyc (42 instructions, 0.00s)

# Run (compiles if needed, reuses .pyc if up to date)
pycargo run hello.py
```

## Supported Python Subset

```python
# Variables & arithmetic
x = 10
y = 3.14
result = x ** 2 + y

# Strings
greeting = "Hello, " + "world!"
print(greeting)

# if / elif / else
if x > 5:
    print("big")
elif x == 5:
    print("five")
else:
    print("small")

# while + break / continue
i = 0
while i < 10:
    if i == 5:
        break
    i += 1

# for + range
for n in range(1, 6):
    print(n)

# Functions & recursion
def factorial(n):
    if n <= 1:
        return 1
    return n * factorial(n - 1)

print(factorial(10))  # 3628800

# Lists
nums = [1, 2, 3, 4, 5]
print(nums[2])        # 3
print(len(nums))      # 5
nums[0] = 99

# Nested functions
def make_adder(n):
    def add(x):
        return x + n
    return add
```

## Built-in Functions

| Function | Description |
|---|---|
| `print(*args)` | Print to stdout |
| `range(stop)` / `range(start, stop[, step])` | Generate integer sequence |
| `len(x)` | Length of string or list |
| `int(x)` | Convert to integer |
| `float(x)` | Convert to float |
| `str(x)` | Convert to string |
| `bool(x)` | Convert to boolean |
| `abs(x)` | Absolute value |
| `max(...)` | Maximum value |
| `min(...)` | Minimum value |
| `type(x)` | Return type name |

## Bytecode Format

`pycargo build` produces a `.pyc` file with a custom binary format:

```
[4 bytes]  magic: "PYCO"
[4 bytes]  version: u32 (little-endian)
[N bytes]  serialized CodeObject tree
```

Each `CodeObject` contains a constant pool, a name table, and a flat instruction list. Nested functions are stored as `Code` constants inside the parent's constant pool.

## Architecture

```
src/
  lexer.rs      — tokenizer (handles indentation, INDENT/DEDENT)
  ast.rs        — AST node types
  parser.rs     — recursive-descent parser
  compiler.rs   — AST → bytecode (with loop/jump patching)
  bytecode.rs   — Op enum, CodeObject, binary serialization
  vm.rs         — stack-based VM + Value type + builtins
  main.rs       — CLI (build / run)
```

## Commands

| Command | Description |
|---|---|
| `pycargo build <file.py>` | Compile to `<file.pyc>` |
| `pycargo run <file.py>` | Compile (if needed) and run |

## Contributing

Contributions are welcome. Open an issue or pull request.

Security issues: contact [yap8572@gmail.com](mailto:yap8572@gmail.com) directly.

## License

MIT
