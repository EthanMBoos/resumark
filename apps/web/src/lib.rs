//! Plain messages exchanged by the browser page and render worker.

#![forbid(unsafe_code)]

use resumark_core::{Diagnostic, PaperSize};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct RenderRequest {
    pub markdown: String,
    pub paper: PaperSize,
    pub max_pages: Option<usize>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RenderResponse {
    Ready,
    Rendered {
        svg_pages: Vec<String>,
        pdf: Vec<u8>,
        diagnostics: Vec<Diagnostic>,
    },
    Rejected {
        diagnostics: Vec<Diagnostic>,
    },
    Failed {
        message: String,
    },
}
