use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use ignore::overrides::OverrideBuilder;
use ignore::WalkBuilder;
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

#[derive(Parser)]
#[command(name = "ziptight", version, about = "Fast zip archiver that respects .gitignore")]
struct Cli {
    /// Source directory to zip
    #[arg(default_value = ".")]
    source: PathBuf,

    /// Output zip file path
    output: Option<PathBuf>,

    /// Additional glob patterns to exclude (repeatable)
    #[arg(short = 'e', long = "extra-exclude")]
    extra_exclude: Vec<String>,

    /// Disable built-in .git/node_modules exclusions
    #[arg(long)]
    no_default_excludes: bool,

    /// Do not read .gitignore files
    #[arg(long)]
    no_gitignore: bool,

    /// Print each included/excluded file
    #[arg(short, long)]
    verbose: bool,

    /// List files that would be included without creating a zip
    #[arg(long)]
    dry_run: bool,

    /// Deflate compression level (1-9, where 1=fastest, 9=smallest) [default: 6]
    #[arg(short = 'l', long, value_parser = clap::value_parser!(u8).range(1..=9))]
    level: Option<u8>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Canonicalize and validate source directory
    let source = cli
        .source
        .canonicalize()
        .with_context(|| format!("Cannot access source directory: {}", cli.source.display()))?;

    if !source.is_dir() {
        bail!("Source path is not a directory: {}", source.display());
    }

    // Determine output path
    let output = cli.output.unwrap_or_else(|| {
        let dir_name = source
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "archive".to_string());
        PathBuf::from(format!("{}.zip", dir_name))
    });

    // Build overrides for default excludes and extra excludes
    let mut overrides = OverrideBuilder::new(&source);
    if !cli.no_default_excludes {
        overrides.add("!.git/").context("Invalid override pattern: .git/")?;
        overrides
            .add("!node_modules/")
            .context("Invalid override pattern: node_modules/")?;
        overrides
            .add("!*.env")
            .context("Invalid override pattern: *.env")?;
    }
    for pattern in &cli.extra_exclude {
        overrides
            .add(&format!("!{}", pattern))
            .with_context(|| format!("Invalid exclude pattern: {}", pattern))?;
    }
    let overrides = overrides.build().context("Failed to build overrides")?;

    // Build the directory walker
    let mut walker_builder = WalkBuilder::new(&source);
    walker_builder
        .hidden(false)
        .git_ignore(!cli.no_gitignore)
        .git_global(!cli.no_gitignore)
        .git_exclude(!cli.no_gitignore)
        .overrides(overrides)
        .follow_links(false)
        .sort_by_file_name(|a, b| a.cmp(b));

    // Collect files to include
    let mut files: Vec<PathBuf> = Vec::new();

    for entry in walker_builder.build() {
        let entry = match entry {
            Ok(e) => e,
            Err(err) => {
                eprintln!("Warning: {}", err);
                continue;
            }
        };

        let path = entry.path();

        // Skip the root directory itself
        if path == source {
            continue;
        }

        // Handle symlinks: follow file symlinks, skip directory symlinks
        if entry.path_is_symlink() {
            let metadata = match fs::metadata(path) {
                Ok(m) => m,
                Err(err) => {
                    eprintln!("Warning: cannot read symlink target {}: {}", path.display(), err);
                    continue;
                }
            };
            if metadata.is_dir() {
                if cli.verbose {
                    let rel = path.strip_prefix(&source).unwrap_or(path);
                    eprintln!("  skip (dir symlink) {}", rel.display());
                }
                continue;
            }
        }

        // Skip directories (they're created implicitly in the zip)
        if entry.file_type().map_or(false, |ft| ft.is_dir()) {
            continue;
        }

        let rel = path
            .strip_prefix(&source)
            .context("Failed to compute relative path")?
            .to_path_buf();

        if cli.verbose {
            println!("+ {}", rel.display());
        }

        files.push(rel);
    }

    // Dry run: just list files and exit
    if cli.dry_run {
        if !cli.verbose {
            for f in &files {
                println!("{}", f.display());
            }
        }
        println!("\n{} files would be included", files.len());
        return Ok(());
    }

    // Create the zip archive
    let zip_file = File::create(&output)
        .with_context(|| format!("Cannot create output file: {}", output.display()))?;
    let mut zip_writer = zip::ZipWriter::new(zip_file);
    let options = SimpleFileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .compression_level(cli.level.map(|l| l as i64));

    let mut buf = Vec::new();
    for rel in &files {
        let full_path = source.join(rel);

        // Use forward slashes in zip paths for portability
        let zip_path = rel.to_string_lossy().replace('\\', "/");

        zip_writer
            .start_file(&zip_path, options)
            .with_context(|| format!("Failed to add file to archive: {}", zip_path))?;

        buf.clear();
        File::open(&full_path)
            .and_then(|mut f| f.read_to_end(&mut buf))
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;

        zip_writer
            .write_all(&buf)
            .with_context(|| format!("Failed to write file to archive: {}", zip_path))?;
    }

    zip_writer.finish().context("Failed to finalize zip archive")?;

    println!("Created {} with {} files", output.display(), files.len());

    Ok(())
}
