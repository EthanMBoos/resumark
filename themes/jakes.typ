/* resumark-theme
{
  "version": 1,
  "name": "Jake's Resume",
  "description": "A compact developer resume based on Jake Gutierrez's popular LaTeX template.",
  "controls": [
    { "kind": "font", "key": "font_family", "label": "Font", "group": "Typography", "value": "CMU Serif", "options": ["CMU Serif", "Libertinus Serif", "Source Sans 3"] },
    { "kind": "number", "key": "body_size_pt", "label": "Body size", "group": "Typography", "value": 11.0, "min": 8.0, "max": 13.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "title_size_pt", "label": "Title size", "group": "Typography", "value": 25.0, "min": 18.0, "max": 32.0, "step": 0.5, "unit": "pt" },
    { "kind": "number", "key": "body_leading_em", "label": "Line spacing", "group": "Typography", "value": 0.4, "min": 0.0, "max": 1.0, "step": 0.02, "unit": "em" },
    { "kind": "number", "key": "page_margin_x_in", "label": "Side margin", "group": "Page", "value": 0.5, "min": 0.3, "max": 1.1, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "page_margin_y_in", "label": "Top and bottom margin", "group": "Page", "value": 0.5, "min": 0.3, "max": 1.1, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "section_gap_pt", "label": "Section gap", "group": "Spacing", "value": 10.0, "min": 0.0, "max": 18.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "entry_gap_pt", "label": "Entry gap", "group": "Spacing", "value": 8.0, "min": 0.0, "max": 16.0, "step": 0.25, "unit": "pt" },
    { "kind": "color", "key": "text_color", "label": "Text", "group": "Color", "value": "#111111" },
    { "kind": "color", "key": "muted_color", "label": "Secondary text", "group": "Color", "value": "#222222" },
    { "kind": "color", "key": "rule_color", "label": "Rules", "group": "Color", "value": "#111111" }
  ]
}
*/

// Inspired by Jake Gutierrez's MIT-licensed resume template:
// https://github.com/jakegut/resume

#import "/resumark/v1.typ": paper-name, render-inlines

#let render(resume, settings, theme) = {
  let color(value) = rgb(value.slice(1))
  let body-size = theme.at("body_size_pt") * 1pt
  let small-size = body-size - 1pt
  let leading = theme.at("body_leading_em") * 1em
  let text-color = color(theme.at("text_color"))
  let muted-color = color(theme.at("muted_color"))
  let rule-color = color(theme.at("rule_color"))
  let section-gap = theme.at("section_gap_pt") * 1pt
  let entry-gap = theme.at("entry_gap_pt") * 1pt
  let spacing = (
    title-contact: 5pt,
    section-content: 5pt,
    entry-indent: 11pt,
    entry-rows: 5pt,
    entry-content: 5pt,
    paragraph-rows: 4pt,
    paragraph-end: 1pt,
    list-attachment: 1.5pt,
    list-indent: 27pt,
    nested-list-indent: 12pt,
    list-body-indent: 6pt,
    list-items: 3pt,
  )

  let inline-text(nodes) = nodes.map(node => {
    let kind = node.at("type")
    if kind == "text" or kind == "code" {
      node.at("value")
    } else if kind == "strong" or kind == "emphasis" {
      inline-text(node.at("content"))
    } else if kind == "link" {
      inline-text(node.at("label"))
    } else if kind == "soft_break" or kind == "hard_break" {
      " "
    } else {
      ""
    }
  }).join()

  let split-pair(value, separator) = {
    let parts = value.split(separator)
    if parts.len() >= 2 {
      (parts.first(), parts.slice(1).join(separator))
    } else {
      (value, "")
    }
  }

  let split-last(value, separator) = {
    let parts = value.split(separator)
    if parts.len() >= 2 {
      (parts.slice(0, parts.len() - 1).join(separator), parts.last())
    } else {
      (value, "")
    }
  }

  let render-contact(nodes) = {
    for node in nodes {
      let kind = node.at("type")
      if kind == "link" {
        link(node.at("destination"), underline(render-inlines(node.at("label"))))
      } else if kind == "text" {
        node.at("value").replace(" · ", " | ")
      } else {
        render-inlines((node,))
      }
    }
  }

  let is-entry-metadata(entry) = {
    if entry == none or entry.at("type") != "paragraph" {
      false
    } else {
      let content = entry.at("content")
      content.len() == 1 and content.first().at("type") == "emphasis"
    }
  }

  let render-entry(heading, details: none) = {
    let heading-pair = split-pair(inline-text(heading.at("content")), " — ")
    if details == none {
      block(sticky: true, below: spacing.entry-content)[
        #pad(x: spacing.entry-indent)[
          #text(size: small-size, weight: "bold", heading-pair.first())
          #if heading-pair.last() != "" [ #sym.bar.v #text(size: small-size, style: "italic", heading-pair.last())]
        ]
      ]
    } else if is-entry-metadata(details) {
      let metadata-pair = split-pair(inline-text(details.at("content")), " · ")
      block(sticky: true, below: spacing.entry-content)[
        #pad(x: spacing.entry-indent)[
          #grid(
            columns: (1fr, auto),
            rows: (auto, auto),
            column-gutter: 10pt,
            row-gutter: spacing.entry-rows,
            strong(heading-pair.first()),
            align(right, metadata-pair.first()),
            text(size: small-size, style: "italic", fill: muted-color, heading-pair.last()),
            align(right, text(size: small-size, style: "italic", fill: muted-color, metadata-pair.last())),
          )
        ]
      ]
    } else {
      let detail-pair = split-last(inline-text(details.at("content")), ", ")
      block(sticky: true, below: spacing.entry-content)[
        #pad(x: spacing.entry-indent)[
          #grid(
            columns: (1fr, auto),
            rows: (auto, auto),
            column-gutter: 10pt,
            row-gutter: spacing.entry-rows,
            strong(inline-text(heading.at("content"))),
            [],
            text(size: small-size, style: "italic", fill: muted-color, detail-pair.first()),
            align(right, text(size: small-size, style: "italic", fill: muted-color, detail-pair.last())),
          )
        ]
      ]
    }
  }

  set document(title: resume.metadata.title, author: resume.metadata.title)
  set page(
    paper: paper-name(settings),
    margin: (
      x: theme.at("page_margin_x_in") * 1in,
      y: theme.at("page_margin_y_in") * 1in,
    ),
  )
  set text(font: theme.at("font_family"), size: body-size, fill: text-color, lang: "en")
  set par(justify: false, leading: leading)
  show link: set text(fill: text-color)
  show raw: set text(font: theme.at("font_family"))
  show title: it => align(center, block(below: spacing.title-contact,
    text(size: theme.at("title_size_pt") * 1pt, weight: "bold", smallcaps(it.body)),
  ))
  show heading.where(level: 1): it => block(
    sticky: true,
    above: section-gap,
    below: spacing.section-content,
  )[
    #grid(
      columns: 1fr,
      rows: (auto, auto),
      row-gutter: 1pt,
      text(size: body-size + 1pt, smallcaps(it.body)),
      line(length: 100%, stroke: 0.65pt + rule-color),
    )
  ]
  show list: set block(above: spacing.list-attachment, below: 0pt)
  show enum: set block(above: spacing.list-attachment, below: 0pt)

  let render-paragraph(entry, index, root: false, inside-list: false, sticky: false, below: 1pt) = {
    let body = render-inlines(entry.at("content"))
    if root and index == 1 {
      let contact = render-contact(entry.at("content"))
      align(center, block(breakable: false, below: 5pt, text(size: small-size, contact)))
    } else if root and index == 2 {
      block(breakable: false, below: 1pt, par(leading: 0.25em, body))
    } else if inside-list {
      block(breakable: false, sticky: sticky, body)
    } else {
      block(
        breakable: false,
        sticky: sticky,
        below: below,
        pad(x: spacing.entry-indent, body),
      )
    }
  }

  let render-blocks(blocks, root: false, inside-list: false) = {
    for (index, entry) in blocks.enumerate() {
      let kind = entry.at("type")
      let previous = if index > 0 { blocks.at(index - 1) } else { none }
      let next = if index + 1 < blocks.len() { blocks.at(index + 1) } else { none }
      if kind == "heading" {
        let level = entry.at("level")
        let body = render-inlines(entry.at("content"))
        if level == 1 {
          title(body)
        } else if level == 2 {
          heading(level: 1, outlined: true, body)
        } else {
          let follows-section = previous != none and previous.at("type") == "heading" and previous.at("level") == 2
          if not follows-section { v(entry-gap, weak: true) }
          let details = if next != none and next.at("type") == "paragraph" { next } else { none }
          render-entry(entry, details: details)
        }
      } else if kind == "paragraph" {
        let follows-entry = previous != none and previous.at("type") == "heading" and previous.at("level") >= 3
        if not follows-entry {
          let followed-by-paragraph = next != none and next.at("type") == "paragraph"
          let paragraph-gap = if followed-by-paragraph { spacing.paragraph-rows } else { spacing.paragraph-end }
          render-paragraph(entry, index, root: root, inside-list: inside-list, sticky: follows-entry, below: paragraph-gap)
        }
      } else if kind == "list" {
        let items = entry.at("items").map(item => render-blocks(item.at("blocks"), inside-list: true))
        let list-kind = entry.at("list_kind")
        let marker = text(size: 8pt, [•])
        let indent = if inside-list { spacing.nested-list-indent } else { spacing.list-indent }
        if list-kind.at("type") == "ordered" {
          text(size: small-size, enum(
            start: list-kind.at("start"),
            indent: indent,
            body-indent: spacing.list-body-indent,
            spacing: spacing.list-items,
            ..items,
          ))
        } else {
          text(size: small-size, list(
            marker: marker,
            indent: indent,
            body-indent: spacing.list-body-indent,
            spacing: spacing.list-items,
            ..items,
          ))
        }
      } else if kind == "divider" {
        v(3pt)
      }
    }
  }

  render-blocks(resume.blocks, root: true)
}
