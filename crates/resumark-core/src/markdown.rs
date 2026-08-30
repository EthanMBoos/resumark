use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use thiserror::Error;

use crate::model::{Block, DocumentMetadata, Inline, ListItem, RenderDocument, append_plain_text};

/// A Markdown construct that the current vertical slice cannot represent.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum ParseError {
    #[error("this first Resumark slice does not support Markdown {construct}")]
    Unsupported { construct: &'static str },

    #[error("the Markdown document must contain a level-one heading for the resume name")]
    MissingTitle,

    #[error("the Markdown parser produced an unexpected event sequence")]
    UnexpectedStructure,
}

/// Converts supported Markdown into Resumark's owned render model.
///
/// The first level-one heading becomes the document title. The Stage 0 parser
/// accepts the constructs used by the realistic fixture; complete policy,
/// source ranges, and user diagnostics belong to Stage 1.
///
/// # Errors
///
/// Returns an error for unsupported Markdown, malformed event nesting, or a
/// document without a level-one heading.
pub fn parse_markdown(source: &str) -> Result<RenderDocument, ParseError> {
    let mut builder = DocumentBuilder::new();

    for event in Parser::new(source) {
        builder.push(event)?;
    }

    builder.finish()
}

struct DocumentBuilder {
    stack: Vec<Container>,
}

enum Container {
    Document(Vec<Block>),
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List {
        start: Option<u64>,
        items: Vec<ListItem>,
    },
    ListItem(Vec<Block>),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Link {
        destination: String,
        label: Vec<Inline>,
    },
}

impl DocumentBuilder {
    fn new() -> Self {
        Self {
            stack: vec![Container::Document(Vec::new())],
        }
    }

    fn push(&mut self, event: Event<'_>) -> Result<(), ParseError> {
        match event {
            Event::Start(tag) => self.start(tag),
            Event::End(tag) => self.end(tag),
            Event::Text(text) => self.push_inline(Inline::Text {
                value: text.into_string(),
            }),
            Event::Code(code) => self.push_inline(Inline::Code {
                value: code.into_string(),
            }),
            Event::SoftBreak => self.push_inline(Inline::SoftBreak),
            Event::HardBreak => self.push_inline(Inline::HardBreak),
            Event::Rule => self.push_block(Block::Divider),
            Event::Html(_) | Event::InlineHtml(_) => Err(ParseError::Unsupported {
                construct: "raw HTML",
            }),
            Event::FootnoteReference(_) => Err(ParseError::Unsupported {
                construct: "footnotes",
            }),
            Event::TaskListMarker(_) => Err(ParseError::Unsupported {
                construct: "task-list markers",
            }),
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                Err(ParseError::Unsupported { construct: "math" })
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>) -> Result<(), ParseError> {
        let container = match tag {
            Tag::Paragraph => Container::Paragraph(Vec::new()),
            Tag::Heading { level, .. } => Container::Heading {
                level: heading_level(level),
                content: Vec::new(),
            },
            Tag::List(start) => Container::List {
                start,
                items: Vec::new(),
            },
            Tag::Item => Container::ListItem(Vec::new()),
            Tag::Emphasis => Container::Emphasis(Vec::new()),
            Tag::Strong => Container::Strong(Vec::new()),
            Tag::Link { dest_url, .. } => Container::Link {
                destination: dest_url.into_string(),
                label: Vec::new(),
            },
            Tag::BlockQuote(_) => return unsupported("block quotes"),
            Tag::CodeBlock(_) => return unsupported("code blocks"),
            Tag::HtmlBlock => return unsupported("raw HTML"),
            Tag::FootnoteDefinition(_) => return unsupported("footnotes"),
            Tag::Strikethrough => return unsupported("strikethrough"),
            Tag::Image { .. } => return unsupported("images"),
            Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {
                return unsupported("tables");
            }
            Tag::MetadataBlock(_) => return unsupported("metadata blocks"),
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => return unsupported("extended Markdown"),
        };

        self.stack.push(container);
        Ok(())
    }

    fn end(&mut self, tag: TagEnd) -> Result<(), ParseError> {
        match tag {
            TagEnd::Paragraph => {
                let Container::Paragraph(content) = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                self.push_block(Block::Paragraph { content })
            }
            TagEnd::Heading(_) => {
                let Container::Heading { level, content } = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                self.push_block(Block::Heading { level, content })
            }
            TagEnd::List(_) => {
                let Container::List { start, items } = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                self.push_block(Block::List {
                    ordered: start.is_some(),
                    start,
                    items,
                })
            }
            TagEnd::Item => {
                let Container::ListItem(blocks) = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                let Some(Container::List { items, .. }) = self.stack.last_mut() else {
                    return Err(ParseError::UnexpectedStructure);
                };
                items.push(ListItem { blocks });
                Ok(())
            }
            TagEnd::Emphasis => {
                let Container::Emphasis(content) = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                self.push_inline(Inline::Emphasis { content })
            }
            TagEnd::Strong => {
                let Container::Strong(content) = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                self.push_inline(Inline::Strong { content })
            }
            TagEnd::Link => {
                let Container::Link { destination, label } = self.pop()? else {
                    return Err(ParseError::UnexpectedStructure);
                };
                self.push_inline(Inline::Link { destination, label })
            }
            _ => Err(ParseError::UnexpectedStructure),
        }
    }

    fn pop(&mut self) -> Result<Container, ParseError> {
        if self.stack.len() <= 1 {
            return Err(ParseError::UnexpectedStructure);
        }
        self.stack.pop().ok_or(ParseError::UnexpectedStructure)
    }

    fn push_block(&mut self, block: Block) -> Result<(), ParseError> {
        match self.stack.last_mut() {
            Some(Container::Document(blocks)) | Some(Container::ListItem(blocks)) => {
                blocks.push(block);
                Ok(())
            }
            _ => Err(ParseError::UnexpectedStructure),
        }
    }

    fn push_inline(&mut self, inline: Inline) -> Result<(), ParseError> {
        match self.stack.last_mut() {
            Some(Container::Heading { content, .. })
            | Some(Container::Paragraph(content))
            | Some(Container::Strong(content))
            | Some(Container::Emphasis(content)) => {
                content.push(inline);
                Ok(())
            }
            Some(Container::Link { label, .. }) => {
                label.push(inline);
                Ok(())
            }
            // CommonMark omits paragraph tags around tight list-item text.
            Some(Container::ListItem(blocks)) => {
                if let Some(Block::Paragraph { content }) = blocks.last_mut() {
                    content.push(inline);
                } else {
                    blocks.push(Block::Paragraph {
                        content: vec![inline],
                    });
                }
                Ok(())
            }
            _ => Err(ParseError::UnexpectedStructure),
        }
    }

    fn finish(mut self) -> Result<RenderDocument, ParseError> {
        if self.stack.len() != 1 {
            return Err(ParseError::UnexpectedStructure);
        }

        let Some(Container::Document(blocks)) = self.stack.pop() else {
            return Err(ParseError::UnexpectedStructure);
        };

        let title = first_title(&blocks).ok_or(ParseError::MissingTitle)?;
        Ok(RenderDocument {
            metadata: DocumentMetadata { title },
            blocks,
        })
    }
}

fn first_title(blocks: &[Block]) -> Option<String> {
    for block in blocks {
        if let Block::Heading { level: 1, content } = block {
            let mut title = String::new();
            append_plain_text(content, &mut title);
            if !title.trim().is_empty() {
                return Some(title.trim().to_owned());
            }
        }
    }
    None
}

fn heading_level(level: HeadingLevel) -> u8 {
    match level {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

fn unsupported<T>(construct: &'static str) -> Result<T, ParseError> {
    Err(ParseError::Unsupported { construct })
}
