use pulldown_cmark::{Event, HeadingLevel as MarkdownHeadingLevel, Options, Parser, Tag, TagEnd};
use serde::{Deserialize, Serialize};

use crate::diagnostic::{Diagnostic, DiagnosticCode, Severity, SourceRange};
use crate::model::{
    Block, BlockNode, DocumentMetadata, HeadingLevel, Inline, InlineNode, InvalidLinkTarget,
    LinkTarget, ListItem, ListKind, RenderDocument, append_plain_text,
};

/// Named limits applied before and during Markdown analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ParseLimits {
    pub max_source_bytes: usize,
    pub max_nesting_depth: usize,
}

impl Default for ParseLimits {
    fn default() -> Self {
        Self {
            max_source_bytes: 256 * 1024,
            max_nesting_depth: 16,
        }
    }
}

/// The usable document, when valid, and every source diagnostic found.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Analysis {
    pub document: Option<RenderDocument>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Analysis {
    #[must_use]
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error)
    }
}

/// Analyzes Markdown into Resumark's renderer-independent document model.
///
/// Diagnostics use UTF-8 byte ranges from `source`. Error-severity diagnostics
/// prevent a render document from being returned, while analysis itself does
/// not panic or print.
#[must_use]
pub fn analyze_markdown(source: &str, limits: &ParseLimits) -> Analysis {
    let diagnostics = validate_source(source, limits);
    if !diagnostics.is_empty() {
        return Analysis {
            document: None,
            diagnostics,
        };
    }

    let parser = Parser::new_ext(source, markdown_options()).into_offset_iter();
    let mut builder = DocumentBuilder::new(*limits);

    for (event, range) in parser {
        if let Err(diagnostic) = builder.push(event, range.into()) {
            builder.diagnostics.push(diagnostic);
            builder.structure_failed = true;
            break;
        }
    }

    builder.finish()
}

fn markdown_options() -> Options {
    // Parse known extensions so we can diagnose them explicitly. Never use
    // Options::all(): new dependency syntax should require a product decision.
    Options::ENABLE_TABLES
        | Options::ENABLE_FOOTNOTES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_HEADING_ATTRIBUTES
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS
        | Options::ENABLE_PLUSES_DELIMITED_METADATA_BLOCKS
        | Options::ENABLE_MATH
        | Options::ENABLE_DEFINITION_LIST
        | Options::ENABLE_SUPERSCRIPT
        | Options::ENABLE_SUBSCRIPT
        | Options::ENABLE_WIKILINKS
}

fn validate_source(source: &str, limits: &ParseLimits) -> Vec<Diagnostic> {
    if source.len() > limits.max_source_bytes {
        return vec![
            Diagnostic::error(
                DiagnosticCode::DocumentTooLarge,
                format!(
                    "the Markdown source is {} bytes; the current limit is {} bytes",
                    source.len(),
                    limits.max_source_bytes
                ),
                Some(SourceRange::new(0, source.len())),
            )
            .with_help("shorten the source or deliberately raise ParseLimits::max_source_bytes"),
        ];
    }

    source
        .char_indices()
        .filter(|(_, character)| is_disallowed_control(*character))
        .map(|(start, character)| {
            Diagnostic::error(
                DiagnosticCode::DisallowedControlCharacter,
                format!(
                    "the Markdown source contains disallowed control character U+{:04X}",
                    u32::from(character)
                ),
                Some(SourceRange::new(start, start + character.len_utf8())),
            )
            .with_help("remove the control character; tabs and ordinary line breaks are allowed")
        })
        .collect()
}

fn is_disallowed_control(character: char) -> bool {
    (character <= '\u{1f}' && !matches!(character, '\t' | '\n' | '\r')) || character == '\u{7f}'
}

struct DocumentBuilder {
    limits: ParseLimits,
    stack: Vec<Container>,
    ignored_depth: usize,
    diagnostics: Vec<Diagnostic>,
    structure_failed: bool,
}

enum Container {
    Document(Vec<BlockNode>),
    Heading {
        range: SourceRange,
        level: HeadingLevel,
        content: Vec<InlineNode>,
    },
    Paragraph {
        range: SourceRange,
        content: Vec<InlineNode>,
    },
    List {
        range: SourceRange,
        list_kind: ListKind,
        items: Vec<ListItem>,
    },
    ListItem {
        range: SourceRange,
        blocks: Vec<BlockNode>,
    },
    Strong {
        range: SourceRange,
        content: Vec<InlineNode>,
    },
    Emphasis {
        range: SourceRange,
        content: Vec<InlineNode>,
    },
    Link {
        range: SourceRange,
        destination: Option<LinkTarget>,
        label: Vec<InlineNode>,
    },
}

impl DocumentBuilder {
    fn new(limits: ParseLimits) -> Self {
        Self {
            limits,
            stack: vec![Container::Document(Vec::new())],
            ignored_depth: 0,
            diagnostics: Vec::new(),
            structure_failed: false,
        }
    }

    fn push(&mut self, event: Event<'_>, range: SourceRange) -> Result<(), Diagnostic> {
        if self.ignored_depth > 0 {
            match event {
                Event::Start(_) => self.ignored_depth += 1,
                Event::End(_) => self.ignored_depth -= 1,
                _ => {}
            }
            return Ok(());
        }

        match event {
            Event::Start(tag) => self.start(tag, range),
            Event::End(tag) => self.end(tag, range),
            Event::Text(text) => self.push_inline(InlineNode {
                range,
                kind: Inline::Text {
                    value: text.into_string(),
                },
            }),
            Event::Code(code) => self.push_inline(InlineNode {
                range,
                kind: Inline::Code {
                    value: code.into_string(),
                },
            }),
            Event::SoftBreak => self.push_inline(InlineNode {
                range,
                kind: Inline::SoftBreak,
            }),
            Event::HardBreak => self.push_inline(InlineNode {
                range,
                kind: Inline::HardBreak,
            }),
            Event::Rule => self.push_block(BlockNode {
                range,
                kind: Block::Divider,
            }),
            Event::Html(_) | Event::InlineHtml(_) => {
                self.diagnostics.push(raw_html_diagnostic(range));
                Ok(())
            }
            Event::FootnoteReference(_) => {
                self.report_leaf_unsupported("footnotes", range);
                Ok(())
            }
            Event::TaskListMarker(_) => {
                self.report_leaf_unsupported("task-list markers", range);
                Ok(())
            }
            Event::InlineMath(_) | Event::DisplayMath(_) => {
                self.report_leaf_unsupported("math", range);
                Ok(())
            }
        }
    }

    fn start(&mut self, tag: Tag<'_>, range: SourceRange) -> Result<(), Diagnostic> {
        let next_depth = self.stack.len();
        if next_depth > self.limits.max_nesting_depth {
            self.diagnostics.push(
                Diagnostic::error(
                    DiagnosticCode::NestingTooDeep,
                    format!(
                        "Markdown nesting exceeds the configured depth of {}",
                        self.limits.max_nesting_depth
                    ),
                    Some(range),
                )
                .with_help("reduce nested lists or nested inline formatting"),
            );
            self.ignored_depth = 1;
            return Ok(());
        }

        let container = match tag {
            Tag::Paragraph => Container::Paragraph {
                range,
                content: Vec::new(),
            },
            Tag::Heading {
                level,
                id,
                classes,
                attrs,
            } => {
                if id.is_some() || !classes.is_empty() || !attrs.is_empty() {
                    return self.ignore_unsupported("heading attributes", range);
                }
                Container::Heading {
                    range,
                    level: heading_level(level),
                    content: Vec::new(),
                }
            }
            Tag::List(start) => Container::List {
                range,
                list_kind: start.map_or(ListKind::Unordered, |start| ListKind::Ordered { start }),
                items: Vec::new(),
            },
            Tag::Item => Container::ListItem {
                range,
                blocks: Vec::new(),
            },
            Tag::Emphasis => Container::Emphasis {
                range,
                content: Vec::new(),
            },
            Tag::Strong => Container::Strong {
                range,
                content: Vec::new(),
            },
            Tag::Link { dest_url, .. } => {
                let destination = match LinkTarget::parse(dest_url.into_string()) {
                    Ok(destination) => Some(destination),
                    Err(error) => {
                        self.diagnostics.push(link_diagnostic(&error, range));
                        None
                    }
                };
                Container::Link {
                    range,
                    destination,
                    label: Vec::new(),
                }
            }
            Tag::HtmlBlock => {
                self.diagnostics.push(raw_html_diagnostic(range));
                self.ignored_depth = 1;
                return Ok(());
            }
            Tag::BlockQuote(_) => return self.ignore_unsupported("block quotes", range),
            Tag::CodeBlock(_) => return self.ignore_unsupported("code blocks", range),
            Tag::FootnoteDefinition(_) => return self.ignore_unsupported("footnotes", range),
            Tag::Strikethrough => return self.ignore_unsupported("strikethrough", range),
            Tag::Image { .. } => return self.ignore_unsupported("images", range),
            Tag::Table(_) | Tag::TableHead | Tag::TableRow | Tag::TableCell => {
                return self.ignore_unsupported("tables", range);
            }
            Tag::MetadataBlock(_) => return self.ignore_unsupported("metadata blocks", range),
            Tag::DefinitionList
            | Tag::DefinitionListTitle
            | Tag::DefinitionListDefinition
            | Tag::Superscript
            | Tag::Subscript => return self.ignore_unsupported("extended Markdown", range),
        };

        self.stack.push(container);
        Ok(())
    }

    fn end(&mut self, tag: TagEnd, range: SourceRange) -> Result<(), Diagnostic> {
        match tag {
            TagEnd::Paragraph => {
                let Container::Paragraph {
                    range: start,
                    content,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };
                self.push_block(BlockNode {
                    range: start.cover(range),
                    kind: Block::Paragraph { content },
                })
            }
            TagEnd::Heading(_) => {
                let Container::Heading {
                    range: start,
                    level,
                    content,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };
                self.push_block(BlockNode {
                    range: start.cover(range),
                    kind: Block::Heading { level, content },
                })
            }
            TagEnd::List(_) => {
                let Container::List {
                    range: start,
                    list_kind,
                    items,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };
                self.push_block(BlockNode {
                    range: start.cover(range),
                    kind: Block::List { list_kind, items },
                })
            }
            TagEnd::Item => {
                let Container::ListItem {
                    range: start,
                    blocks,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };
                let Some(Container::List { items, .. }) = self.stack.last_mut() else {
                    return Err(unexpected_structure(range));
                };
                items.push(ListItem {
                    range: start.cover(range),
                    blocks,
                });
                Ok(())
            }
            TagEnd::Emphasis => {
                let Container::Emphasis {
                    range: start,
                    content,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };
                self.push_inline(InlineNode {
                    range: start.cover(range),
                    kind: Inline::Emphasis { content },
                })
            }
            TagEnd::Strong => {
                let Container::Strong {
                    range: start,
                    content,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };
                self.push_inline(InlineNode {
                    range: start.cover(range),
                    kind: Inline::Strong { content },
                })
            }
            TagEnd::Link => {
                let Container::Link {
                    range: start,
                    destination,
                    label,
                } = self.pop(range)?
                else {
                    return Err(unexpected_structure(range));
                };

                if let Some(destination) = destination {
                    self.push_inline(InlineNode {
                        range: start.cover(range),
                        kind: Inline::Link { destination, label },
                    })
                } else {
                    self.push_inlines(label)
                }
            }
            _ => Err(unexpected_structure(range)),
        }
    }

    fn pop(&mut self, range: SourceRange) -> Result<Container, Diagnostic> {
        if self.stack.len() <= 1 {
            return Err(unexpected_structure(range));
        }
        self.stack.pop().ok_or_else(|| unexpected_structure(range))
    }

    fn push_block(&mut self, block: BlockNode) -> Result<(), Diagnostic> {
        match self.stack.last_mut() {
            Some(Container::Document(blocks)) | Some(Container::ListItem { blocks, .. }) => {
                blocks.push(block);
                Ok(())
            }
            _ => Err(unexpected_structure(block.range)),
        }
    }

    fn push_inline(&mut self, inline: InlineNode) -> Result<(), Diagnostic> {
        match self.stack.last_mut() {
            Some(Container::Heading { content, .. })
            | Some(Container::Paragraph { content, .. })
            | Some(Container::Strong { content, .. })
            | Some(Container::Emphasis { content, .. }) => {
                content.push(inline);
                Ok(())
            }
            Some(Container::Link { label, .. }) => {
                label.push(inline);
                Ok(())
            }
            // CommonMark omits paragraph tags around tight list-item text.
            Some(Container::ListItem { blocks, .. }) => {
                if let Some(BlockNode {
                    range,
                    kind: Block::Paragraph { content },
                }) = blocks.last_mut()
                {
                    *range = range.cover(inline.range);
                    content.push(inline);
                } else {
                    blocks.push(BlockNode {
                        range: inline.range,
                        kind: Block::Paragraph {
                            content: vec![inline],
                        },
                    });
                }
                Ok(())
            }
            _ => Err(unexpected_structure(inline.range)),
        }
    }

    fn push_inlines(&mut self, inlines: Vec<InlineNode>) -> Result<(), Diagnostic> {
        for inline in inlines {
            self.push_inline(inline)?;
        }
        Ok(())
    }

    fn ignore_unsupported(
        &mut self,
        construct: &'static str,
        range: SourceRange,
    ) -> Result<(), Diagnostic> {
        self.report_leaf_unsupported(construct, range);
        self.ignored_depth = 1;
        Ok(())
    }

    fn report_leaf_unsupported(&mut self, construct: &'static str, range: SourceRange) {
        self.diagnostics.push(
            Diagnostic::error(
                DiagnosticCode::UnsupportedMarkdown,
                format!("Resumark does not support Markdown {construct}"),
                Some(range),
            )
            .with_help(
                "use headings, paragraphs, lists, emphasis, links, inline code, or dividers",
            ),
        );
    }

    fn finish(mut self) -> Analysis {
        let blocks = if self.structure_failed {
            None
        } else if self.stack.len() != 1 {
            self.diagnostics
                .push(unexpected_structure(SourceRange::new(0, 0)));
            None
        } else {
            match self.stack.pop() {
                Some(Container::Document(blocks)) => Some(blocks),
                _ => {
                    self.diagnostics
                        .push(unexpected_structure(SourceRange::new(0, 0)));
                    None
                }
            }
        };

        let document = blocks.and_then(|blocks| {
            let Some(title) = first_title(&blocks) else {
                self.diagnostics.push(
                    Diagnostic::error(
                        DiagnosticCode::MissingTitle,
                        "the Markdown document needs a level-one heading for the resume name",
                        None,
                    )
                    .with_help("add a heading such as `# Jane Doe` at the start of the document"),
                );
                return None;
            };

            Some(RenderDocument {
                metadata: DocumentMetadata { title },
                blocks,
            })
        });

        self.diagnostics.sort_by_key(|diagnostic| {
            (
                diagnostic.range.map_or(usize::MAX, |range| range.start),
                diagnostic.code,
            )
        });

        let has_errors = self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == Severity::Error);

        Analysis {
            document: if has_errors { None } else { document },
            diagnostics: self.diagnostics,
        }
    }
}

fn first_title(blocks: &[BlockNode]) -> Option<String> {
    for block in blocks {
        if let Block::Heading {
            level: HeadingLevel::One,
            content,
        } = &block.kind
        {
            let mut title = String::new();
            append_plain_text(content, &mut title);
            if !title.trim().is_empty() {
                return Some(title.trim().to_owned());
            }
        }
    }
    None
}

fn heading_level(level: MarkdownHeadingLevel) -> HeadingLevel {
    match level {
        MarkdownHeadingLevel::H1 => HeadingLevel::One,
        MarkdownHeadingLevel::H2 => HeadingLevel::Two,
        MarkdownHeadingLevel::H3 => HeadingLevel::Three,
        MarkdownHeadingLevel::H4 => HeadingLevel::Four,
        MarkdownHeadingLevel::H5 => HeadingLevel::Five,
        MarkdownHeadingLevel::H6 => HeadingLevel::Six,
    }
}

fn raw_html_diagnostic(range: SourceRange) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::UnsupportedRawHtml,
        "raw HTML is not allowed in Resumark Markdown",
        Some(range),
    )
    .with_help("write the content with supported Markdown instead")
}

fn link_diagnostic(error: &InvalidLinkTarget, range: SourceRange) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::UnsafeLinkScheme,
        error.to_string(),
        Some(range),
    )
    .with_help("use an https, http, mailto, or tel link")
}

fn unexpected_structure(range: SourceRange) -> Diagnostic {
    Diagnostic::error(
        DiagnosticCode::UnexpectedMarkdownStructure,
        "the Markdown parser produced an unexpected event sequence",
        Some(range),
    )
    .with_help("simplify the Markdown around this location and try again")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn realistic_fixture_has_a_ranged_document() {
        let source = include_str!("../../../fixtures/resume.md");
        let analysis = analyze_markdown(source, &ParseLimits::default());

        assert!(analysis.diagnostics.is_empty());
        let document = analysis.document.expect("the fixture should be valid");
        assert_eq!(document.metadata.title, "Jane Doe");

        let title = document.blocks.first().expect("the fixture has a title");
        assert_eq!(
            source[title.range.start..title.range.end].trim(),
            "# Jane Doe"
        );
    }

    #[test]
    fn raw_html_is_rejected_with_its_source_range() {
        let source = "# Jane Doe\n\n<div>not allowed</div>\n";
        let analysis = analyze_markdown(source, &ParseLimits::default());

        assert!(analysis.document.is_none());
        let diagnostic = diagnostic(&analysis, DiagnosticCode::UnsupportedRawHtml);
        let range = diagnostic.range.expect("raw HTML should have a range");
        assert!(source[range.start..range.end].contains("<div>"));
    }

    #[test]
    fn unsafe_link_scheme_is_rejected() {
        let source = "# Jane Doe\n\n[open](javascript:alert(1))\n";
        let analysis = analyze_markdown(source, &ParseLimits::default());

        assert!(analysis.document.is_none());
        let diagnostic = diagnostic(&analysis, DiagnosticCode::UnsafeLinkScheme);
        assert!(diagnostic.message.contains("javascript"));
    }

    #[test]
    fn known_markdown_extensions_are_diagnosed_instead_of_treated_as_text() {
        let source = "# Jane Doe\n\n| Skill | Level |\n| --- | --- |\n| Rust | Advanced |\n";
        let analysis = analyze_markdown(source, &ParseLimits::default());

        assert!(analysis.document.is_none());
        let diagnostic = diagnostic(&analysis, DiagnosticCode::UnsupportedMarkdown);
        assert!(diagnostic.message.contains("tables"));
    }

    #[test]
    fn supported_link_schemes_remain_in_the_document() {
        let source = concat!(
            "# Jane Doe\n\n",
            "[web](https://example.com) ",
            "[plain](http://example.com) ",
            "[email](mailto:jane@example.com) ",
            "[phone](tel:+12125550142)\n",
        );
        let analysis = analyze_markdown(source, &ParseLimits::default());

        assert!(analysis.diagnostics.is_empty());
        assert!(analysis.document.is_some());
    }

    #[test]
    fn source_size_is_checked_before_parsing() {
        let source = "# Jane Doe\n";
        let limits = ParseLimits {
            max_source_bytes: source.len() - 1,
            ..ParseLimits::default()
        };
        let analysis = analyze_markdown(source, &limits);

        assert!(analysis.document.is_none());
        diagnostic(&analysis, DiagnosticCode::DocumentTooLarge);
    }

    #[test]
    fn nesting_depth_is_explicitly_limited() {
        let source = "# Jane Doe\n\n- outer\n  - inner\n";
        let limits = ParseLimits {
            max_nesting_depth: 2,
            ..ParseLimits::default()
        };
        let analysis = analyze_markdown(source, &limits);

        assert!(analysis.document.is_none());
        diagnostic(&analysis, DiagnosticCode::NestingTooDeep);
    }

    #[test]
    fn control_characters_are_rejected_before_the_markdown_parser_runs() {
        let source = "# Jane Doe\n\ninvalid\u{0}text\n";
        let analysis = analyze_markdown(source, &ParseLimits::default());

        assert!(analysis.document.is_none());
        let diagnostic = diagnostic(&analysis, DiagnosticCode::DisallowedControlCharacter);
        assert_eq!(
            diagnostic.range,
            Some(SourceRange::new(
                source.find('\u{0}').expect("the fixture has a null byte"),
                source.find('\u{0}').expect("the fixture has a null byte") + 1,
            ))
        );
    }

    fn diagnostic(analysis: &Analysis, code: DiagnosticCode) -> &Diagnostic {
        analysis
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == code)
            .unwrap_or_else(|| panic!("expected diagnostic {code}"))
    }
}
