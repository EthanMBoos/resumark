//! Plain messages exchanged by the browser page and render worker.

#![forbid(unsafe_code)]

use resumark_core::{Diagnostic, PaperSize};
use resumark_render_typst::ThemeDiagnostic;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RenderRequest {
    pub revision: u64,
    pub markdown: String,
    pub paper: PaperSize,
    pub max_pages: Option<usize>,
    pub theme_source: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum RenderResponse {
    Ready,
    Rendered {
        revision: u64,
        svg_pages: Vec<String>,
        pdf: Vec<u8>,
        diagnostics: Vec<Diagnostic>,
    },
    ResumeRejected {
        revision: u64,
        diagnostics: Vec<Diagnostic>,
    },
    ThemeRejected {
        revision: u64,
        diagnostics: Vec<ThemeDiagnostic>,
    },
    Failed {
        revision: Option<u64>,
        message: String,
    },
}
