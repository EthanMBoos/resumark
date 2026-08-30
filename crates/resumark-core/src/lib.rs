//! Resumark's renderer-independent document model and Markdown parser.
//!
//! This crate owns plain, serializable values. It deliberately knows nothing
//! about Typst, command-line arguments, filesystems, browsers, or UI state.
//!
//! ```
//! use resumark_core::{ParseLimits, analyze_markdown};
//!
//! let analysis = analyze_markdown("# Ada Lovelace\n\nProgrammer", &ParseLimits::default());
//! assert_eq!(analysis.document.unwrap().metadata.title, "Ada Lovelace");
//! ```

#![forbid(unsafe_code)]

mod diagnostic;
mod markdown;
mod model;

pub use diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceRange};
pub use markdown::{Analysis, ParseLimits, analyze_markdown};
pub use model::{
    Block, BlockNode, HeadingLevel, Inline, InlineNode, InvalidHeadingLevel, InvalidLinkTarget,
    LinkTarget, ListItem, ListKind, RenderDocument,
};
