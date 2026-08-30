//! Native entry point for exercising the real Resumark compilation pipeline.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use resumark_render_typst::Renderer;

#[derive(Debug, Parser)]
#[command(name = "resume", about = "Compile a Markdown resume with Resumark")]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Build one PDF and one SVG file per page.
    Build {
        /// Markdown resume to compile.
        input: PathBuf,

        /// PDF path. Page SVGs are written beside it.
        #[arg(short, long, default_value = "target/resume.pdf")]
        output: PathBuf,
    },
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    match arguments.command {
        Command::Build { input, output } => build(&input, &output),
    }
}

fn build(input: &Path, output: &Path) -> Result<()> {
    let markdown =
        fs::read_to_string(input).with_context(|| format!("could not read {}", input.display()))?;
    let document = resumark_core::parse_markdown(&markdown)
        .with_context(|| format!("could not parse {}", input.display()))?;

    let renderer = Renderer::new().context("could not initialize the Typst renderer")?;
    let compiled = renderer
        .compile(&document)
        .context("could not compile the resume")?;

    create_parent_directory(output)?;
    let pdf = compiled.pdf().context("could not export the resume PDF")?;
    fs::write(output, pdf).with_context(|| format!("could not write {}", output.display()))?;

    let svg_pages = compiled.svg_pages();
    for (index, svg) in svg_pages.iter().enumerate() {
        let path = svg_path(output, index + 1);
        fs::write(&path, svg).with_context(|| format!("could not write {}", path.display()))?;
    }

    println!(
        "Built {} and {} SVG page(s) from {}",
        output.display(),
        svg_pages.len(),
        input.display()
    );
    Ok(())
}

fn create_parent_directory(path: &Path) -> Result<()> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("could not create {}", parent.display()))?;
    }
    Ok(())
}

fn svg_path(pdf_path: &Path, page_number: usize) -> PathBuf {
    let stem = pdf_path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("resume");
    pdf_path.with_file_name(format!("{stem}-{page_number}.svg"))
}
