//! Native entry point for exercising the real Resumark compilation pipeline.

#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
use resumark_core::{
    Diagnostic, PaperSize, ParseLimits, RenderDocument, Severity, SourceRange, analyze_markdown,
};
use resumark_render_typst::{BundledTheme, RenderOptions, Renderer, ThemeSelection};
use resumark_theme::ThemeFile;

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

        /// Directory for the PDF and numbered SVG pages.
        #[arg(long, default_value = "target/rendered")]
        output_dir: PathBuf,

        /// Physical page size: letter or a4.
        #[arg(long, default_value_t)]
        paper: PaperSize,

        /// Warn when the rendered document exceeds this many pages.
        #[arg(long, default_value_t = 2, value_parser = parse_page_limit)]
        max_pages: usize,

        /// Bundled theme to use. Defaults to Jake's Resume.
        #[arg(long, value_enum, conflicts_with = "theme_file")]
        theme: Option<ThemeName>,

        /// Custom Resumark .typ theme file.
        #[arg(long, conflicts_with = "theme")]
        theme_file: Option<PathBuf>,
    },

    /// Print the analyzed, renderer-independent document as formatted JSON.
    Inspect {
        /// Markdown resume to analyze.
        input: PathBuf,
    },

    /// List the bundled starter themes.
    Themes,

    /// Copy a bundled theme to a file you can customize.
    ExportTheme {
        /// Bundled theme to copy.
        #[arg(value_enum)]
        theme: ThemeName,

        /// Destination .typ file.
        output: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ThemeName {
    Jakes,
    Modern,
    Pirate,
}

impl From<ThemeName> for BundledTheme {
    fn from(value: ThemeName) -> Self {
        match value {
            ThemeName::Jakes => Self::Jakes,
            ThemeName::Modern => Self::Modern,
            ThemeName::Pirate => Self::Pirate,
        }
    }
}

fn main() -> Result<()> {
    let arguments = Arguments::parse();

    match arguments.command {
        Command::Build {
            input,
            output_dir,
            paper,
            max_pages,
            theme,
            theme_file,
        } => build(
            &input,
            &output_dir,
            paper,
            max_pages,
            theme,
            theme_file.as_deref(),
        ),
        Command::Inspect { input } => inspect(&input),
        Command::Themes => list_themes(),
        Command::ExportTheme { theme, output } => export_theme(theme, &output),
    }
}

fn build(
    input: &Path,
    output_dir: &Path,
    paper: PaperSize,
    max_pages: usize,
    theme: Option<ThemeName>,
    theme_file: Option<&Path>,
) -> Result<()> {
    let markdown = read_markdown(input)?;
    let document = valid_document(input, &markdown)?;

    let renderer = Renderer::new().context("could not initialize the Typst renderer")?;
    let options = RenderOptions {
        paper,
        max_pages: Some(max_pages),
        theme: load_theme(theme, theme_file)?,
    };
    let compiled = renderer
        .compile(&document, &options)
        .context("could not compile the resume")?;
    print_diagnostics(input, &markdown, compiled.diagnostics());

    create_output_directory(output_dir)?;
    let output_name = input
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("resume");
    let pdf_path = output_dir.join(format!("{output_name}.pdf"));
    let pdf = compiled.pdf().context("could not export the resume PDF")?;
    fs::write(&pdf_path, pdf).with_context(|| format!("could not write {}", pdf_path.display()))?;

    let svg_pages = compiled.svg_pages();
    for (index, svg) in svg_pages.iter().enumerate() {
        let path = svg_path(output_dir, output_name, index + 1);
        fs::write(&path, svg).with_context(|| format!("could not write {}", path.display()))?;
    }

    println!(
        "Built {} and {} SVG page(s) for {} paper from {}",
        pdf_path.display(),
        compiled.page_count(),
        paper,
        input.display(),
    );
    Ok(())
}

fn load_theme(theme: Option<ThemeName>, theme_file: Option<&Path>) -> Result<ThemeSelection> {
    if let Some(path) = theme_file {
        let source = fs::read_to_string(path)
            .with_context(|| format!("could not read theme {}", path.display()))?;
        let theme = ThemeFile::parse(source)
            .with_context(|| format!("could not parse theme {}", path.display()))?;
        return Ok(ThemeSelection::Custom(theme));
    }

    Ok(ThemeSelection::Bundled(
        theme.map_or(BundledTheme::Jakes, Into::into),
    ))
}

fn list_themes() -> Result<()> {
    for theme in BundledTheme::all() {
        let file = theme.file().context("a bundled theme is invalid")?;
        println!("{}\t{}", theme.id(), file.manifest().description);
    }
    Ok(())
}

fn export_theme(theme: ThemeName, output: &Path) -> Result<()> {
    let theme = BundledTheme::from(theme);
    fs::write(output, theme.source())
        .with_context(|| format!("could not write {}", output.display()))?;
    println!("Exported {} to {}", theme.id(), output.display());
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

fn create_output_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("could not create {}", path.display()))
}

fn svg_path(output_dir: &Path, output_name: &str, page_number: usize) -> PathBuf {
    output_dir.join(format!("{output_name}-{page_number}.svg"))
}

fn parse_page_limit(value: &str) -> Result<usize, String> {
    let limit = value
        .parse::<usize>()
        .map_err(|_| "the maximum page count must be a positive integer".to_owned())?;
    if limit == 0 {
        return Err("the maximum page count must be at least one".to_owned());
    }
    Ok(limit)
}
