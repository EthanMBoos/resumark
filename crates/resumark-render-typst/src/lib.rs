//! The Typst renderer for Resumark.
//!
//! This crate is the only place project code imports Typst. It accepts the
//! renderer-independent model from `resumark-core`, gives Typst the bundled
//! template and fonts, and exports PDF and SVG from one compilation.
//!
//! ```no_run
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let source = "# Ada Lovelace\n\nProgrammer";
//! let analysis = resumark_core::analyze_markdown(source, &resumark_core::ParseLimits::default());
//! let document = analysis.document.expect("the example Markdown is valid");
//! let compiled = resumark_render_typst::Renderer::new()?.compile(
//!     &document,
//!     &resumark_render_typst::RenderOptions::default(),
//! )?;
//! let pdf = compiled.pdf()?;
//! let pages = compiled.svg_pages();
//! assert!(!pdf.is_empty() && !pages.is_empty());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod world;

use resumark_core::{Diagnostic, DiagnosticCode, PaperSize, RenderDocument};
use serde::Serialize;
use thiserror::Error;
use typst::foundations::Bytes;
use typst_layout::PagedDocument;

use crate::world::ResumarkWorld;

const BUNDLED_FONT_BYTES: &[&[u8]] = &[
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-Regular.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-Italic.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-Bold.otf"),
    include_bytes!("../../../fonts/libertinus/LibertinusSerif-BoldItalic.otf"),
];

/// Bundles the project template and fonts and compiles resume documents.
pub struct Renderer {
    fonts: Vec<typst::text::Font>,
}

/// A successful Typst compilation retained for all output formats.
pub struct CompiledDocument {
    document: PagedDocument,
    diagnostics: Vec<Diagnostic>,
}

/// Settings that affect one Typst compilation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderOptions {
    pub paper: PaperSize,
    pub max_pages: Option<usize>,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            paper: PaperSize::Letter,
            max_pages: Some(2),
        }
    }
}

#[derive(Serialize)]
struct ThemeInput<'a> {
    document: &'a RenderDocument,
    settings: ThemeSettings,
}

#[derive(Serialize)]
struct ThemeSettings {
    paper: PaperSize,
}

/// A failure while initializing or compiling the trusted rendering pipeline.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("the bundled Resumark fonts could not be loaded")]
    MissingFonts,

    #[error("the resume model could not be serialized for the trusted theme")]
    Serialize(#[source] serde_json::Error),

    #[error("a bundled renderer asset has an invalid path")]
    InvalidBundledPath,

    #[error("the maximum page count must be at least one when specified")]
    InvalidPageLimit,

    #[error("Typst could not compile the trusted resume theme:\n{0}")]
    Compile(String),
}

/// A failure while exporting a compiled document.
#[derive(Debug, Error)]
pub enum ExportError {
    #[error("Typst could not export the compiled document as PDF: {0}")]
    Pdf(String),
}

impl Renderer {
    /// Loads the four licensed Libertinus Serif faces bundled by Resumark.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::MissingFonts`] if any bundled face is missing or
    /// cannot be decoded by Typst.
    pub fn new() -> Result<Self, RenderError> {
        let fonts = BUNDLED_FONT_BYTES
            .iter()
            .flat_map(|data| typst::text::Font::iter(Bytes::new(*data)))
            .filter(|font| font.info().family == "Libertinus Serif")
            .collect::<Vec<_>>();

        if fonts.len() != BUNDLED_FONT_BYTES.len() {
            return Err(RenderError::MissingFonts);
        }

        Ok(Self { fonts })
    }

    /// Compiles a document once, retaining the paged result for every export.
    ///
    /// # Errors
    ///
    /// Returns an error if JSON serialization or trusted-theme compilation
    /// fails.
    pub fn compile(
        &self,
        document: &RenderDocument,
        options: &RenderOptions,
    ) -> Result<CompiledDocument, RenderError> {
        if options.max_pages == Some(0) {
            return Err(RenderError::InvalidPageLimit);
        }

        let input = ThemeInput {
            document,
            settings: ThemeSettings {
                paper: options.paper,
            },
        };
        let json = serde_json::to_vec_pretty(&input).map_err(RenderError::Serialize)?;
        let world =
            ResumarkWorld::new(json, self.fonts.clone()).ok_or(RenderError::InvalidBundledPath)?;
        let result = typst::compile::<PagedDocument>(&world);

        match result.output {
            Ok(document) => {
                let diagnostics = page_limit_diagnostics(document.pages().len(), options.max_pages);
                Ok(CompiledDocument {
                    document,
                    diagnostics,
                })
            }
            Err(diagnostics) => {
                let messages = diagnostics
                    .iter()
                    .map(|diagnostic| diagnostic.message.to_string())
                    .collect::<Vec<_>>()
                    .join("\n");
                Err(RenderError::Compile(messages))
            }
        }
    }
}

impl CompiledDocument {
    /// Number of pages shared by every export format.
    #[must_use]
    pub fn page_count(&self) -> usize {
        self.document.pages().len()
    }

    /// Non-fatal diagnostics produced after successful layout.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Exports every page as a self-contained SVG string.
    #[must_use]
    pub fn svg_pages(&self) -> Vec<String> {
        self.document
            .pages()
            .iter()
            .map(|page| typst_svg::svg(page, &typst_svg::SvgOptions::default()))
            .collect()
    }

    /// Exports the same paged document as a selectable-text PDF.
    ///
    /// # Errors
    ///
    /// Returns an error if Typst's PDF exporter rejects the compiled document.
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
    fn renderer_loads_only_the_four_bundled_font_faces() {
        let renderer = Renderer::new().expect("the bundled fonts should decode");

        assert_eq!(renderer.fonts.len(), 4);
        assert!(
            renderer
                .fonts
                .iter()
                .all(|font| font.info().family == "Libertinus Serif")
        );
    }

    #[test]
    fn paper_size_reaches_the_trusted_theme() {
        let letter = first_page_size(PaperSize::Letter);
        let a4 = first_page_size(PaperSize::A4);

        assert_close(letter, (612.0, 792.0));
        assert_close(a4, (595.28, 841.89));
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

    fn first_page_size(paper: PaperSize) -> (f64, f64) {
        let analysis = resumark_core::analyze_markdown(
            "# Jane Doe\n\nEngineer",
            &resumark_core::ParseLimits::default(),
        );
        let document = analysis.document.expect("the test Markdown is valid");
        let renderer = Renderer::new().expect("the bundled fonts should decode");
        let options = RenderOptions {
            paper,
            max_pages: None,
        };
        let compiled = renderer
            .compile(&document, &options)
            .expect("the trusted theme should compile");
        let size = compiled.document.pages()[0].frame.size();
        (size.x.to_pt(), size.y.to_pt())
    }

    fn assert_close(actual: (f64, f64), expected: (f64, f64)) {
        assert!((actual.0 - expected.0).abs() < 0.01, "{actual:?}");
        assert!((actual.1 - expected.1).abs() < 0.01, "{actual:?}");
    }
}
