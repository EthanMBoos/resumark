//! Resumark's renderer-independent document model and Markdown parser.
//!
//! This crate owns plain, serializable values. It deliberately knows nothing
//! about Typst, command-line arguments, filesystems, browsers, or UI state.
//!
//! ```
//! let document = resumark_core::parse_markdown("# Ada Lovelace\n\nProgrammer")?;
//! assert_eq!(document.metadata.title, "Ada Lovelace");
//! # Ok::<(), resumark_core::ParseError>(())
//! ```

#![forbid(unsafe_code)]

mod markdown;
mod model;

pub use markdown::{ParseError, parse_markdown};
pub use model::{Block, Inline, ListItem, RenderDocument};
