use std::fmt;

use serde::{Deserialize, Deserializer, Serialize};

use crate::diagnostic::SourceRange;

/// A complete renderer input produced from one Markdown source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderDocument {
    pub metadata: DocumentMetadata,
    pub blocks: Vec<BlockNode>,
}

/// Document-level values that do not belong to a particular rendered block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
}

/// A block and the Markdown bytes that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockNode {
    pub range: SourceRange,
    #[serde(flatten)]
    pub kind: Block,
}

/// A block-level element in source order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Heading {
        level: HeadingLevel,
        content: Vec<InlineNode>,
    },
    Paragraph {
        content: Vec<InlineNode>,
    },
    List {
        list_kind: ListKind,
        items: Vec<ListItem>,
    },
    Divider,
}

/// A valid Markdown heading level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u8", into = "u8")]
pub enum HeadingLevel {
    One,
    Two,
    Three,
    Four,
    Five,
    Six,
}

impl From<HeadingLevel> for u8 {
    fn from(level: HeadingLevel) -> Self {
        match level {
            HeadingLevel::One => 1,
            HeadingLevel::Two => 2,
            HeadingLevel::Three => 3,
            HeadingLevel::Four => 4,
            HeadingLevel::Five => 5,
            HeadingLevel::Six => 6,
        }
    }
}

impl TryFrom<u8> for HeadingLevel {
    type Error = InvalidHeadingLevel;

    fn try_from(level: u8) -> Result<Self, Self::Error> {
        match level {
            1 => Ok(Self::One),
            2 => Ok(Self::Two),
            3 => Ok(Self::Three),
            4 => Ok(Self::Four),
            5 => Ok(Self::Five),
            6 => Ok(Self::Six),
            _ => Err(InvalidHeadingLevel(level)),
        }
    }
}

/// An invalid numeric Markdown heading level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvalidHeadingLevel(u8);

impl fmt::Display for InvalidHeadingLevel {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "heading level {} is outside 1 through 6", self.0)
    }
}

impl std::error::Error for InvalidHeadingLevel {}

/// Whether a Markdown list is unordered or begins at a particular number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ListKind {
    Unordered,
    Ordered { start: u64 },
}

/// One list item, which can itself contain paragraphs or nested lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    pub range: SourceRange,
    pub blocks: Vec<BlockNode>,
}

/// Inline content and the Markdown bytes that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineNode {
    pub range: SourceRange,
    #[serde(flatten)]
    pub kind: Inline,
}

/// Inline content nested inside a heading or paragraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    Text {
        value: String,
    },
    Strong {
        content: Vec<InlineNode>,
    },
    Emphasis {
        content: Vec<InlineNode>,
    },
    Link {
        destination: LinkTarget,
        label: Vec<InlineNode>,
    },
    Code {
        value: String,
    },
    SoftBreak,
    HardBreak,
}

/// A link destination whose scheme is allowed by Resumark's Markdown policy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct LinkTarget(String);

impl LinkTarget {
    /// Validates and retains a link destination.
    ///
    /// # Errors
    ///
    /// Returns [`InvalidLinkTarget`] when the destination has no scheme or its
    /// scheme is outside Resumark's `https`, `http`, `mailto`, and `tel` policy.
    pub fn parse(destination: String) -> Result<Self, InvalidLinkTarget> {
        let Some((scheme, _)) = destination.split_once(':') else {
            return Err(InvalidLinkTarget::MissingScheme);
        };

        if matches!(
            scheme.to_ascii_lowercase().as_str(),
            "https" | "http" | "mailto" | "tel"
        ) {
            Ok(Self(destination))
        } else {
            Err(InvalidLinkTarget::DisallowedScheme(scheme.to_owned()))
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for LinkTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let destination = String::deserialize(deserializer)?;
        Self::parse(destination).map_err(serde::de::Error::custom)
    }
}

/// Why a string cannot be used as a Resumark link destination.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InvalidLinkTarget {
    MissingScheme,
    DisallowedScheme(String),
}

impl fmt::Display for InvalidLinkTarget {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingScheme => formatter.write_str("the link destination has no scheme"),
            Self::DisallowedScheme(scheme) => {
                write!(formatter, "the link scheme `{scheme}` is not allowed")
            }
        }
    }
}

impl std::error::Error for InvalidLinkTarget {}

impl InlineNode {
    pub(crate) fn plain_text(&self, output: &mut String) {
        match &self.kind {
            Inline::Text { value } | Inline::Code { value } => output.push_str(value),
            Inline::Strong { content } | Inline::Emphasis { content } => {
                append_plain_text(content, output);
            }
            Inline::Link { label, .. } => append_plain_text(label, output),
            Inline::SoftBreak | Inline::HardBreak => output.push(' '),
        }
    }
}

pub(crate) fn append_plain_text(inlines: &[InlineNode], output: &mut String) {
    for inline in inlines {
        inline.plain_text(output);
    }
}
