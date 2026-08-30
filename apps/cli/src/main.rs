//! Native entry point for exercising the real Resumark compilation pipeline.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use resumark_core::{
    Diagnostic, ParseLimits, RenderDocument, Severity, SourceRange, analyze_markdown,
};
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

    /// Print the analyzed, renderer-independent document as formatted JSON.
    Inspect {
        /// Markdown resume to analyze.
        input: PathBuf,
    },
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    match arguments.command {
        Command::Build { input, output } => build(&input, &output),
        Command::Inspect { input } => inspect(&input),
    }
}

fn build(input: &Path, output: &Path) -> Result<()> {
    let markdown = read_markdown(input)?;
    let document = valid_document(input, &markdown)?;

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

fn inspect(input: &Path) -> Result<()> {
    let markdown = read_markdown(input)?;
    let document = valid_document(input, &markdown)?;
    let json = serde_json::to_string_pretty(&document)
        .context("could not serialize the analyzed resume")?;
    println!("{json}");
    Ok(())
}

fn read_markdown(input: &Path) -> Result<String> {
    fs::read_to_string(input).with_context(|| format!("could not read {}", input.display()))
}

fn valid_document(input: &Path, markdown: &str) -> Result<RenderDocument> {
    let analysis = analyze_markdown(markdown, &ParseLimits::default());
    print_diagnostics(input, markdown, &analysis.diagnostics);

    let Some(document) = analysis.document else {
        let error_count = analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.severity == Severity::Error)
            .count();
        bail!("{} contains {} error(s)", input.display(), error_count);
    };

    Ok(document)
}

fn print_diagnostics(input: &Path, source: &str, diagnostics: &[Diagnostic]) {
    for diagnostic in diagnostics {
        if let Some(range) = diagnostic.range {
            let location = source_location(source, range);
            eprintln!(
                "{}[{}] at {}:{}:{}",
                diagnostic.severity,
                diagnostic.code,
                input.display(),
                location.line,
                location.column
            );
            eprintln!("  {}", diagnostic.message);
            if !location.text.is_empty() {
                eprintln!("  | {}", location.text);
                eprintln!("  | {:width$}^", "", width = location.column - 1);
            }
        } else {
            eprintln!(
                "{}[{}] in {}",
                diagnostic.severity,
                diagnostic.code,
                input.display()
            );
            eprintln!("  {}", diagnostic.message);
        }

        if let Some(help) = &diagnostic.help {
            eprintln!("  help: {help}");
        }
    }
}

struct SourceLocation<'a> {
    line: usize,
    column: usize,
    text: &'a str,
}

fn source_location(source: &str, range: SourceRange) -> SourceLocation<'_> {
    let mut offset = range.start.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }

    let before = &source[..offset];
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    let line_end = source[offset..]
        .find('\n')
        .map_or(source.len(), |index| offset + index);

    SourceLocation {
        line: before.bytes().filter(|byte| *byte == b'\n').count() + 1,
        column: source[line_start..offset].chars().count() + 1,
        text: source[line_start..line_end].trim_end_matches('\r'),
    }
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
