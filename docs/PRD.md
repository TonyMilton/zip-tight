# ZipTight - Product Requirements Document

## Overview

ZipTight is a fast, CLI-based zip archiver built in Rust that produces clean zip files from project directories by automatically excluding version control artifacts, dependency directories, and files matched by `.gitignore` rules.

## Problem

Zipping a project directory for sharing, deployment, or archival commonly includes unwanted files — `.git` folders, `node_modules`, build artifacts, and other ignored files. Manually excluding these is tedious and error-prone, especially in monorepos or projects with `.gitignore` files at multiple directory depths.

## Goals

- Produce a zip archive of a directory tree with intelligent default exclusions
- Respect `.gitignore` rules at every directory level, with proper cascading semantics
- Be fast enough to handle large monorepos without noticeable delay
- Require zero configuration for the common case

## Non-Goals

- Replacing general-purpose zip tools (e.g., `zip`, `7z`)
- Supporting archive formats other than zip
- GUI or interactive mode

## Functional Requirements

### FR-1: Default Exclusions

The following directories must always be excluded from the output archive, at any depth:

| Pattern          | Reason                        |
|------------------|-------------------------------|
| `.git/`          | Version control internals     |
| `node_modules/`  | JavaScript dependency tree    |
| `*.env`          | Environment secrets           |

### FR-2: `.gitignore` Support

- Parse and apply `.gitignore` files found at any directory depth in the source tree
- `.gitignore` rules apply to the directory they reside in and all of its descendants
- When multiple `.gitignore` files exist at different depths, rules cascade (deeper files add to, but do not override, ancestor rules) following standard Git semantics
- Support all standard `.gitignore` pattern syntax:
  - Glob patterns (`*.log`, `build/`)
  - Negation (`!important.log`)
  - Directory-only patterns (trailing `/`)
  - Rooted patterns (leading `/`)
  - `**` for recursive matching
  - Comments (`#`) and blank lines

### FR-3: CLI Interface

```
ziptight [OPTIONS] [SOURCE] [OUTPUT]
```

| Argument / Flag       | Description                                              | Default                          |
|-----------------------|----------------------------------------------------------|----------------------------------|
| `SOURCE`              | Path to the directory to zip                             | Current working directory (`.`)  |
| `OUTPUT`              | Path for the output zip file                             | `<source-dir-name>.zip`         |
| `--extra-exclude, -e` | Additional glob patterns to exclude (repeatable)         | None                             |
| `--no-default-excludes` | Disable built-in `.git`/`node_modules`/`*.env` exclusions | Off                              |
| `--no-gitignore`      | Do not read `.gitignore` files                           | Off                              |
| `--verbose, -v`       | Print each included/excluded file                        | Off                              |
| `--dry-run`           | List files that would be included without creating a zip | Off                              |
| `--version`           | Print version and exit                                   |                                  |
| `--help, -h`          | Print usage information                                  |                                  |

### FR-4: Output

- Produce a valid zip archive (deflate compression)
- Preserve relative directory structure inside the archive
- The archive root should be the contents of the source directory (no wrapping directory unless the source path implies one)

### FR-5: Symlinks

- Follow symlinks to files (include the target content)
- Do not follow symlinks to directories (skip them) to avoid cycles

## Non-Functional Requirements

### NFR-1: Performance

- Use the `ignore` crate's efficient single-threaded walker for directory traversal
- Target: zip a 10,000-file project tree in under 1 second on commodity hardware

### NFR-2: Correctness

- `.gitignore` matching must behave identically to `git` itself — use the `ignore` crate (or equivalent) rather than hand-rolling pattern matching

### NFR-3: Portability

- Compile and run on Linux, macOS, and Windows
- No runtime dependencies beyond the OS

## Technical Approach

| Concern              | Crate / Approach         |
|----------------------|--------------------------|
| CLI argument parsing | `clap` (derive)          |
| Directory walking    | `ignore` (from ripgrep)  |
| Zip creation         | `zip` (deflate)          |
| Error handling       | `anyhow`                 |

The `ignore` crate is the key dependency — it already implements Git-compatible `.gitignore` parsing, cascading at multiple depths, and default filtering of `.git` and hidden files. This eliminates the need to reimplement gitignore semantics.

Zip paths use forward slashes for cross-platform compatibility. File output is sorted deterministically by filename.

## Success Criteria

1. Running `ziptight` in a typical JS/TS project produces a zip that contains no `.git`, `node_modules`, or gitignored files
2. The resulting zip is valid and extractable by standard tools
3. Behavior matches `git ls-files` for which files are considered ignored
4. Wall-clock time is competitive with or faster than `zip -r` with manual excludes
