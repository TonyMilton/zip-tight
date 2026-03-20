# ZipTight

An opinionated, fast CLI zip archiver that produces clean zip files from project directories by automatically excluding `.git`, `node_modules`, `*.env` files, and files matched by `.gitignore` rules.

## Install

### From GitHub Releases (prebuilt binaries)

Download the latest binary for your platform from [Releases](https://github.com/TonyMilton/zip-tight/releases).

**macOS / Linux:**

```sh
# Example for macOS Apple Silicon — replace the URL for your platform
curl -L https://github.com/TonyMilton/zip-tight/releases/latest/download/ziptight-v0.1.0-aarch64-apple-darwin.tar.gz | tar xz
sudo mv ziptight /usr/local/bin/
```

If `/usr/local/bin` isn't in your PATH, add it to your shell profile:

```sh
# bash
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc && source ~/.bashrc

# zsh (macOS default)
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.zshrc && source ~/.zshrc
```

**Windows:**

Download the `.zip` from the releases page, extract `ziptight.exe`, and add its location to your PATH via System > Environment Variables.

### From source

```sh
cargo install --path .
```

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
```

## Options

| Flag | Description |
|---|---|
| `SOURCE` | Directory to zip (default: `.`) |
| `OUTPUT` | Output zip path (default: `<dir-name>.zip`) |
| `-e, --extra-exclude` | Additional glob patterns to exclude (repeatable) |
| `--no-default-excludes` | Include `.git/`, `node_modules/`, and `*.env` |
| `--no-gitignore` | Ignore `.gitignore` rules |
| `-v, --verbose` | Print each included file |
| `--dry-run` | List files without creating a zip |

## How it works

ZipTight walks the source directory using the [`ignore`](https://crates.io/crates/ignore) crate (from ripgrep), which provides Git-compatible `.gitignore` parsing with proper cascading at every directory depth. Files are compressed with deflate into a standard zip archive.

By default, `.git/`, `node_modules/`, and `*.env` files are excluded at any depth. Symlinks to files are followed; symlinks to directories are skipped to avoid cycles.

## License

MIT
