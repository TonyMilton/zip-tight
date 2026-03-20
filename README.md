# ZipTight

An opinionated, fast CLI zip archiver that produces clean zip files from project directories by automatically excluding `.git`, `node_modules`, `*.env` files, and files matched by `.gitignore` rules.

## Install

### Quick install (macOS / Linux)

```sh
curl -sL https://raw.githubusercontent.com/TonyMilton/zip-tight/main/install.sh | bash
```

This detects your OS and architecture, downloads the latest binary to `~/.local/bin`, and updates your shell PATH if needed. After installing, restart your shell or run:

```sh
# bash
source ~/.bashrc

# zsh
source ~/.zshrc
```

### From source

```sh
cargo install --path .
```

### Windows

Download the `.zip` from the [Releases](https://github.com/TonyMilton/zip-tight/releases) page, extract `ziptight.exe`, and add its location to your PATH via System > Environment Variables.

## Usage

```sh
# Zip current directory
ziptight

# Zip a specific directory
ziptight ./my-project

# Zip with a custom output path
ziptight ./my-project archive.zip

# Exclude additional patterns
ziptight -e "*.log" -e "dist/"

# Preview what would be included
ziptight --dry-run

# See every file as it's processed
ziptight --verbose

# Use maximum compression
ziptight --level 9

# Use fastest compression
ziptight --level 1
```

## Options

| Flag | Description |
|---|---|
| `SOURCE` | Directory to zip (default: `.`) |
| `OUTPUT` | Output zip path (default: `<dir-name>.zip`) |
| `-e, --extra-exclude` | Additional glob patterns to exclude (repeatable) |
| `--no-default-excludes` | Include `.git/`, `node_modules/`, and `*.env` |
| `--no-gitignore` | Ignore `.gitignore` rules |
| `-l, --level` | Deflate compression level 1-9 (1=fastest, 9=smallest, default: 6) |
| `-v, --verbose` | Print each included file |
| `--dry-run` | List files without creating a zip |

## How it works

ZipTight walks the source directory using the [`ignore`](https://crates.io/crates/ignore) crate (from ripgrep), which provides Git-compatible `.gitignore` parsing with proper cascading at every directory depth. Files are streamed in chunks and compressed with deflate into a standard zip archive, with a real-time progress bar showing bytes processed and throughput.

By default, `.git/`, `node_modules/`, and `*.env` files are excluded at any depth. The output zip file is automatically excluded if it resides within the source directory. Symlinks to files are followed; symlinks to directories are skipped to avoid cycles.

## License

MIT
