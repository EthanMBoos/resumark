use serde::{Deserialize, Serialize};

/// A complete renderer input produced from one Markdown source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RenderDocument {
    pub metadata: DocumentMetadata,
    pub blocks: Vec<Block>,
}

/// Document-level values that do not belong to a particular rendered block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: String,
}

/// A block-level element in source order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph {
        content: Vec<Inline>,
    },
    List {
        ordered: bool,
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    Divider,
}

/// One list item, which can itself contain paragraphs or nested lists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ListItem {
    pub blocks: Vec<Block>,
}

/// Inline content nested inside a heading or paragraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inline {
    Text {
        value: String,
    },
    Strong {
        content: Vec<Inline>,
    },
    Emphasis {
        content: Vec<Inline>,
    },
    Link {
        destination: String,
        label: Vec<Inline>,
    },
    Code {
        value: String,
    },
    SoftBreak,
    HardBreak,
}

impl Inline {
    pub(crate) fn plain_text(&self, output: &mut String) {
        match self {
            Self::Text { value } | Self::Code { value } => output.push_str(value),
            Self::Strong { content } | Self::Emphasis { content } => {
                append_plain_text(content, output);
            }
            Self::Link { label, .. } => append_plain_text(label, output),
            Self::SoftBreak | Self::HardBreak => output.push(' '),
        }
    }
}

pub(crate) fn append_plain_text(inlines: &[Inline], output: &mut String) {
    for inline in inlines {
        inline.plain_text(output);
    }
}
