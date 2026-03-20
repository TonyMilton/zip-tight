use std::fs::{self, File};
use std::io::{BufReader, Read, Write};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
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

    // Resolve absolute output path for exclusion from the file list
    let output_abs = fs::canonicalize(&output).unwrap_or_else(|_| {
        // Output doesn't exist yet — resolve its parent and append the filename
        let parent = output.parent().map(|p| {
            if p.as_os_str().is_empty() {
                std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
            } else {
                fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf())
            }
        }).unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        parent.join(output.file_name().unwrap_or(output.as_os_str()))
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

        // Skip the output zip file itself to avoid including it in the archive
        if let Ok(canonical) = fs::canonicalize(path) {
            if canonical == output_abs {
                if cli.verbose {
                    let rel = path.strip_prefix(&source).unwrap_or(path);
                    eprintln!("  skip (output file) {}", rel.display());
                }
                continue;
            }
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

    // Calculate total bytes for progress tracking
    let total_bytes: u64 = files
        .iter()
        .map(|rel| {
            fs::metadata(source.join(rel))
                .map(|m| m.len())
                .unwrap_or(0)
        })
        .sum();

    let pb = ProgressBar::new(total_bytes);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:30}] {bytes}/{total_bytes} ({bytes_per_sec})")
            .unwrap()
            .progress_chars("##-"),
    );
    pb.set_message("Compressing");

    let mut buf = [0u8; 64 * 1024];
    for rel in &files {
        let full_path = source.join(rel);

        // Use forward slashes in zip paths for portability
        let zip_path = rel.to_string_lossy().replace('\\', "/");

        zip_writer
            .start_file(&zip_path, options)
            .with_context(|| format!("Failed to add file to archive: {}", zip_path))?;

        let file = File::open(&full_path)
            .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
        let mut reader = BufReader::new(file);

        loop {
            let n = reader
                .read(&mut buf)
                .with_context(|| format!("Failed to read file: {}", full_path.display()))?;
            if n == 0 {
                break;
            }
            zip_writer
                .write_all(&buf[..n])
                .with_context(|| format!("Failed to write file to archive: {}", zip_path))?;
            pb.inc(n as u64);
        }
    }

    zip_writer.finish().context("Failed to finalize zip archive")?;

    pb.finish_with_message("Done");
    println!("Created {} with {} files", output.display(), files.len());

    Ok(())
}
