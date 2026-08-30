use serde::{Deserialize, Serialize};
use std::fmt;

/// A half-open UTF-8 byte range in the original Markdown source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceRange {
    pub start: usize,
    pub end: usize,
}

impl SourceRange {
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub(crate) fn cover(self, other: Self) -> Self {
        Self {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}

impl From<std::ops::Range<usize>> for SourceRange {
    fn from(range: std::ops::Range<usize>) -> Self {
        Self::new(range.start, range.end)
    }
}

/// Stable identifier for a problem the resume author can correct.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticCode {
    DisallowedControlCharacter,
    DocumentTooLarge,
    MissingTitle,
    NestingTooDeep,
    PageLimitExceeded,
    UnexpectedMarkdownStructure,
    UnsafeLinkScheme,
    UnsupportedMarkdown,
    UnsupportedRawHtml,
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let code = match self {
            Self::DisallowedControlCharacter => "disallowed_control_character",
            Self::DocumentTooLarge => "document_too_large",
            Self::MissingTitle => "missing_title",
            Self::NestingTooDeep => "nesting_too_deep",
            Self::PageLimitExceeded => "page_limit_exceeded",
            Self::UnexpectedMarkdownStructure => "unexpected_markdown_structure",
            Self::UnsafeLinkScheme => "unsafe_link_scheme",
            Self::UnsupportedMarkdown => "unsupported_markdown",
            Self::UnsupportedRawHtml => "unsupported_raw_html",
        };
        formatter.write_str(code)
    }
}

/// Importance of a source diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warning => formatter.write_str("warning"),
            Self::Error => formatter.write_str("error"),
        }
    }
}

/// A source-oriented problem that can be shown by the CLI or future editor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub range: Option<SourceRange>,
    pub help: Option<String>,
}

impl Diagnostic {
    pub(crate) fn error(
        code: DiagnosticCode,
        message: impl Into<String>,
        range: Option<SourceRange>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            range,
            help: None,
        }
    }

    /// Creates a non-fatal diagnostic that may accompany valid output.
    pub fn warning(
        code: DiagnosticCode,
        message: impl Into<String>,
        range: Option<SourceRange>,
    ) -> Self {
        Self {
            code,
            severity: Severity::Warning,
            message: message.into(),
            range,
            help: None,
        }
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}
