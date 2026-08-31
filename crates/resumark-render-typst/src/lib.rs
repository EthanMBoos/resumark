//! Typst rendering for Resumark documents and themes.

#![forbid(unsafe_code)]

mod world;

use std::fmt;

use resumark_core::{Diagnostic, DiagnosticCode, PaperSize, RenderDocument};
use resumark_theme::{ThemeFile, ThemeFileError, ThemeRange};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use typst::WorldExt;
use typst::foundations::Bytes;
use typst_layout::PagedDocument;

use crate::world::ResumarkWorld;

const BUNDLED_FONT_BYTES: &[&[u8]] = &[
    include_bytes!("../../../fonts/computer-modern/cmunrm.otf"),
    include_bytes!("../../../fonts/computer-modern/cmunti.otf"),
    include_bytes!("../../../fonts/computer-modern/cmunbx.otf"),
    include_bytes!("../../../fonts/computer-modern/cmunbi.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-Regular.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-Italic.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-Bold.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-BoldItalic.otf"),
    include_bytes!("../../../fonts/source-sans/SourceSans3-Regular.otf"),
    include_bytes!("../../../fonts/source-sans/SourceSans3-It.otf"),
    include_bytes!("../../../fonts/source-sans/SourceSans3-Semibold.otf"),
    include_bytes!("../../../fonts/source-sans/SourceSans3-Bold.otf"),
];

/// A theme shipped with Resumark.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledTheme {
    Minimal,
    Modern,
    Compact,
    Jakes,
}

impl BundledTheme {
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Minimal => "minimal",
            Self::Modern => "modern",
            Self::Compact => "compact",
            Self::Jakes => "jakes",
        }
    }

    #[must_use]
    pub const fn source(self) -> &'static str {
        match self {
            Self::Minimal => include_str!("../../../themes/minimal.typ"),
            Self::Modern => include_str!("../../../themes/modern.typ"),
            Self::Compact => include_str!("../../../themes/compact.typ"),
            Self::Jakes => include_str!("../../../themes/jakes.typ"),
        }
    }

    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Minimal, Self::Modern, Self::Compact, Self::Jakes]
    }

    pub fn file(self) -> Result<ThemeFile, ThemeFileError> {
        ThemeFile::parse(self.source())
    }
}

/// The bundled or user-supplied theme used for a compilation.
#[derive(Debug, Clone, PartialEq)]
pub enum ThemeSelection {
    Bundled(BundledTheme),
    Custom(ThemeFile),
}

impl Default for ThemeSelection {
    fn default() -> Self {
        Self::Bundled(BundledTheme::Minimal)
    }
}

impl ThemeSelection {
    fn file(&self) -> Result<ThemeFile, ThemeFileError> {
        match self {
            Self::Bundled(theme) => theme.file(),
            Self::Custom(theme) => Ok(theme.clone()),
        }
    }
}

/// Bundles the project templates and fonts and compiles resume documents.
pub struct Renderer {
    fonts: Vec<typst::text::Font>,
}

/// A successful Typst compilation retained for all output formats.
pub struct CompiledDocument {
    document: PagedDocument,
    diagnostics: Vec<Diagnostic>,
}

/// Settings that affect one Typst compilation.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderOptions {
    pub paper: PaperSize,
    pub max_pages: Option<usize>,
    pub theme: ThemeSelection,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::Letter,
            max_pages: Some(2),
            theme: ThemeSelection::default(),
        }
    }
}

#[derive(Serialize)]
struct ThemeInput<'a> {
    document: &'a RenderDocument,
    settings: ThemeSettings,
    theme: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize)]
struct ThemeSettings {
    paper: PaperSize,
}

/// One Typst compiler message, with a byte range when it points into the theme.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemeDiagnostic {
    pub message: String,
    pub range: Option<ThemeRange>,
    pub hints: Vec<String>,
}

/// Structured Typst errors from compiling a theme.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeCompileError {
    pub diagnostics: Vec<ThemeDiagnostic>,
}

impl fmt::Display for ThemeCompileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let messages = self
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        formatter.write_str(&messages)
    }
}

impl std::error::Error for ThemeCompileError {}

/// A failure while initializing or compiling the rendering pipeline.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("the bundled Resumark fonts could not be loaded")]
    MissingFonts,

    #[error("the resume model could not be serialized for Typst")]
    Serialize(#[source] serde_json::Error),

    #[error("a bundled renderer asset has an invalid path")]
    InvalidBundledPath,

    #[error("the maximum page count must be at least one when specified")]
    InvalidPageLimit,

    #[error("the theme file is invalid: {0}")]
    ThemeFile(#[from] ThemeFileError),

    #[error("Typst could not compile the resume theme:\n{0}")]
    Compile(ThemeCompileError),
}

/// A failure while exporting a compiled document.
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Typst could not export the compiled document as PDF: {0}")]
    Pdf(String),
}

impl Renderer {
    /// Loads all fonts bundled by Resumark.
    pub fn new() -> Result<Self, RenderError> {
        let fonts = BUNDLED_FONT_BYTES
            .iter()
            .flat_map(|data| typst::text::Font::iter(Bytes::new(*data)))
            .collect::<Vec<_>>();

        if fonts.len() != BUNDLED_FONT_BYTES.len() {
            return Err(RenderError::MissingFonts);
        }

        Ok(Self { fonts })
    }

    /// Compiles a document once, retaining the paged result for every export.
    pub fn compile(
        &self,
        document: &RenderDocument,
        options: &RenderOptions,
    ) -> Result<CompiledDocument, RenderError> {
        if options.max_pages == Some(0) {
            return Err(RenderError::InvalidPageLimit);
        }

        let theme = options.theme.file()?;
        let input = ThemeInput {
            document,
            settings: ThemeSettings {
                paper: options.paper,
            },
            theme: theme.control_values(),
        };
        let json = serde_json::to_vec_pretty(&input).map_err(RenderError::Serialize)?;
        let world = ResumarkWorld::new(json, theme.source().to_owned(), self.fonts.clone())
            .ok_or(RenderError::InvalidBundledPath)?;
        let result = typst::compile::<PagedDocument>(&world);

        match result.output {
            Ok(document) => {
                let diagnostics = page_limit_diagnostics(document.pages().len(), options.max_pages);
                Ok(CompiledDocument {
                    document,
                    diagnostics,
                })
            }
            Err(diagnostics) => Err(RenderError::Compile(ThemeCompileError {
                diagnostics: diagnostics
                    .iter()
                    .map(|diagnostic| ThemeDiagnostic {
                        message: diagnostic.message.to_string(),
                        range: (diagnostic.span.id() == Some(world.theme_id()))
                            .then(|| world.range(diagnostic.span))
                            .flatten()
                            .map(|range| ThemeRange {
                                start: range.start,
                                end: range.end,
                            }),
                        hints: diagnostic
                            .hints
                            .iter()
                            .map(|hint| hint.v.to_string())
                            .collect(),
                    })
                    .collect(),
            })),
        }
    }
}

impl CompiledDocument {
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.document.pages().len()
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn svg_pages(&self) -> Vec<String> {
        self.document
            .pages()
            .iter()
            .map(|page| typst_svg::svg(page, &typst_svg::SvgOptions::default()))
            .collect()
    }

    pub fn pdf(&self) -> Result<Vec<u8>, ExportError> {
        typst_pdf::pdf(&self.document, &typst_pdf::PdfOptions::default())
            .map_err(|errors| ExportError::Pdf(format!("{errors:?}")))
    }
}

fn page_limit_diagnostics(page_count: usize, max_pages: Option<usize>) -> Vec<Diagnostic> {
    let Some(max_pages) = max_pages.filter(|limit| page_count > *limit) else {
        return Vec::new();
    };

    vec![
        Diagnostic::warning(
            DiagnosticCode::PageLimitExceeded,
            format!(
                "the resume rendered as {page_count} pages; the configured maximum is {max_pages}"
            ),
            None,
        )
        .with_help("shorten the content or raise the maximum page count"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_loads_the_bundled_font_faces() {
        let renderer = Renderer::new().expect("the bundled fonts should decode");
        assert_eq!(renderer.fonts.len(), BUNDLED_FONT_BYTES.len());
    }

    #[test]
    fn paper_size_reaches_the_theme() {
        let letter = first_page_size(PaperSize::Letter);
        let a4 = first_page_size(PaperSize::A4);

        assert_close(letter, (612.0, 792.0));
        assert_close(a4, (595.28, 841.89));
    }

    #[test]
    fn every_bundled_theme_compiles() {
        for theme in BundledTheme::all() {
            compile_test_document(ThemeSelection::Bundled(*theme))
                .unwrap_or_else(|error| panic!("{} failed: {error}", theme.id()));
        }
    }

    #[test]
    fn theme_errors_point_into_custom_source() {
        let mut source = BundledTheme::Minimal.source().to_owned();
        source.push_str("\n#this-function-does-not-exist()\n");
        let theme = ThemeFile::parse(source).expect("the manifest is valid");
        let error = match compile_test_document(ThemeSelection::Custom(theme)) {
            Ok(_) => panic!("the Typst source should fail"),
            Err(error) => error,
        };
        let RenderError::Compile(error) = error else {
            panic!("expected a Typst compile error");
        };

        assert!(
            error
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.range.is_some())
        );
    }

    #[test]
    fn page_limit_is_a_non_fatal_warning() {
        let diagnostics = page_limit_diagnostics(3, Some(2));

        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagnosticCode::PageLimitExceeded);
        assert_eq!(diagnostics[0].severity, resumark_core::Severity::Warning);
    }

    #[test]
    fn page_limit_can_be_disabled() {
        assert!(page_limit_diagnostics(3, None).is_empty());
    }

    fn compile_test_document(theme: ThemeSelection) -> Result<CompiledDocument, RenderError> {
        let analysis = resumark_core::analyze_markdown(
            "# Jane Doe\n\nEngineer",
            &resumark_core::ParseLimits::default(),
        );
        let document = analysis.document.expect("the test Markdown is valid");
        Renderer::new()?.compile(
            &document,
            &RenderOptions {
                paper: PaperSize::Letter,
                max_pages: None,
                theme,
            },
        )
    }

    fn first_page_size(paper: PaperSize) -> (f64, f64) {
        let options = RenderOptions {
            paper,
            ..RenderOptions::default()
        };
        let analysis = resumark_core::analyze_markdown(
            "# Jane Doe\n\nEngineer",
            &resumark_core::ParseLimits::default(),
        );
        let document = analysis.document.expect("the test Markdown is valid");
        let renderer = Renderer::new().expect("the bundled fonts should decode");
        let compiled = renderer
            .compile(&document, &options)
            .expect("the bundled theme should compile");
        let size = compiled.document.pages()[0].frame.size();
        drop(compiled);
        (size.x.to_pt(), size.y.to_pt())
    }

    fn assert_close(actual: (f64, f64), expected: (f64, f64)) {
        assert!((actual.0 - expected.0).abs() < 0.01, "{actual:?}");
        assert!((actual.1 - expected.1).abs() < 0.01, "{actual:?}");
    }
}
