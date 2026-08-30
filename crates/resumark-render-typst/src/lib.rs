//! The restricted Typst adapter for Resumark.
//!
//! This crate is the only place project code imports Typst. It accepts the
//! renderer-independent model from `resumark-core`, exposes only trusted
//! in-memory assets to Typst, and exports PDF and SVG from one compilation.
//!
//! ```no_run
//! # fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let source = "# Ada Lovelace\n\nProgrammer";
//! let document = resumark_core::parse_markdown(source)?;
//! let compiled = resumark_render_typst::Renderer::new()?.compile(&document)?;
//! let pdf = compiled.pdf()?;
//! let pages = compiled.svg_pages();
//! assert!(!pdf.is_empty() && !pages.is_empty());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod world;

use resumark_core::RenderDocument;
use thiserror::Error;
use typst::foundations::Bytes;
use typst_layout::PagedDocument;

use crate::world::ResumarkWorld;

/// Creates restricted Typst worlds and compiles project-owned documents.
pub struct Renderer {
    fonts: Vec<typst::text::Font>,
}

/// A successful Typst compilation retained for all output formats.
pub struct CompiledDocument {
    document: PagedDocument,
}

/// A failure while initializing or compiling the trusted rendering pipeline.
#[derive(Debug, Error)]
pub enum RenderError {
    #[error("the bundled Resumark fonts could not be loaded")]
    MissingFonts,

    #[error("the resume model could not be serialized for the trusted theme")]
    Serialize(#[source] serde_json::Error),

    #[error("a trusted in-memory asset has an invalid virtual path")]
    InvalidBundledPath,

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
    /// Loads the licensed fonts bundled by the pinned Typst asset package.
    ///
    /// # Errors
    ///
    /// Returns [`RenderError::MissingFonts`] if none of the bundled font bytes
    /// can be decoded by Typst.
    pub fn new() -> Result<Self, RenderError> {
        let fonts = typst_assets::fonts()
            .flat_map(|data| typst::text::Font::iter(Bytes::new(data)))
            .filter(|font| font.info().family == "Libertinus Serif")
            .collect::<Vec<_>>();

        if fonts.is_empty() {
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
    pub fn compile(&self, document: &RenderDocument) -> Result<CompiledDocument, RenderError> {
        let json = serde_json::to_vec_pretty(document).map_err(RenderError::Serialize)?;
        let world =
            ResumarkWorld::new(json, self.fonts.clone()).ok_or(RenderError::InvalidBundledPath)?;
        let result = typst::compile::<PagedDocument>(&world);

        match result.output {
            Ok(document) => Ok(CompiledDocument { document }),
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
