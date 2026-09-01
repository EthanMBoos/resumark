/* resumark-theme
{
  "version": 1,
  "name": "Pirate",
  "description": "A dense blue-accented layout based on the Pirate King resume.",
  "controls": [
    { "kind": "font", "key": "font_family", "label": "Font", "group": "Typography", "value": "Nunito", "options": ["Nunito", "Source Sans 3"] },
    { "kind": "number", "key": "body_size_pt", "label": "Body size", "group": "Typography", "value": 9.0, "min": 8.0, "max": 12.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "title_size_pt", "label": "Title size", "group": "Typography", "value": 16.0, "min": 12.0, "max": 24.0, "step": 0.5, "unit": "pt" },
    { "kind": "number", "key": "body_leading_em", "label": "Line spacing", "group": "Typography", "value": 0.66, "min": 0.0, "max": 0.8, "step": 0.02, "unit": "em" },
    { "kind": "number", "key": "page_margin_x_in", "label": "Side margin", "group": "Page", "value": 0.3125, "min": 0.25, "max": 1.0, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "page_margin_y_in", "label": "Top and bottom margin", "group": "Page", "value": 0.58, "min": 0.25, "max": 1.0, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "section_gap_pt", "label": "Section gap", "group": "Spacing", "value": 10.5, "min": 0.0, "max": 18.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "entry_gap_pt", "label": "Entry gap", "group": "Spacing", "value": 13.0, "min": 0.0, "max": 18.0, "step": 0.25, "unit": "pt" },
    { "kind": "color", "key": "text_color", "label": "Text", "group": "Color", "value": "#0F0F0F" },
    { "kind": "color", "key": "rule_color", "label": "Accent", "group": "Color", "value": "#1155CC" },
    { "kind": "color", "key": "link_color", "label": "Links", "group": "Color", "value": "#45818E" }
  ]
}
*/

// Based on the Pirate King resume reference included with this project.

#import "/resumark/v1.typ": paper-name, render-inlines

#let render(resume, settings, theme) = {
  let color(value) = rgb(value.slice(1))
  let body-font = theme.at("font_family")
  let body-size = theme.at("body_size_pt") * 1pt
  let entry-size = body-size + 1pt
  let leading = theme.at("body_leading_em") * 1em
  let text-color = color(theme.at("text_color"))
  let accent-color = color(theme.at("rule_color"))
  let link-color = color(theme.at("link_color"))
  let section-gap = theme.at("section_gap_pt") * 1pt
  let entry-gap = theme.at("entry_gap_pt") * 1pt
  let spacing = (
    title-contact: 10pt,
    contact-summary: 6pt,
    summary-end: 6pt,
    section-content: 12pt,
    entry-content: 10.5pt,
    paragraph: 6pt,
    list-attachment: 0.5pt,
    list-items: 6pt,
    list-indent: 0pt,
    nested-list-indent: 0pt,
    list-body-indent: 7pt,
    content-indent: 4.5pt,
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

  let render-contact(nodes) = {
    for node in nodes {
      let kind = node.at("type")
      if kind == "link" {
        link(
          node.at("destination"),
          text(fill: link-color, underline(render-inlines(node.at("label")))),
        )
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

  let render-title(nodes) = {
    let title-size = theme.at("title_size_pt") * 1pt
    let words = inline-text(nodes).split(" ")
    block(
      width: 100%,
      below: spacing.title-contact,
      align(center, text(font: "Spectral", weight: 500, {
        for (index, word) in words.enumerate() {
          if index > 0 { [ ] }
          if word.len() > 0 {
            text(size: title-size, upper(word.slice(0, 1)))
            text(size: title-size - 2pt, upper(word.slice(1)))
          }
        }
      })),
    )
  }

  let render-entry(heading, details: none) = {
    let heading-pair = split-pair(inline-text(heading.at("content")), " — ")
    if details == none {
      block(sticky: true, below: spacing.entry-content)[
        #pad(x: spacing.content-indent)[
          #strong(heading-pair.first())#if heading-pair.last() != "" [: #heading-pair.last()]
        ]
      ]
    } else if is-entry-metadata(details) {
      let metadata-pair = split-pair(inline-text(details.at("content")), " · ")
      block(sticky: true, below: spacing.entry-content)[
        #pad(x: spacing.content-indent)[
          #grid(
            columns: (1.4fr, 1fr, auto, auto),
            column-gutter: 10pt,
            text(size: entry-size, weight: "bold", heading-pair.first()),
            align(center, text(size: entry-size, weight: "bold", heading-pair.last())),
            align(right, text(size: entry-size, style: "italic", metadata-pair.last())),
            align(right, text(size: entry-size, weight: "bold", metadata-pair.first())),
          )
        ]
      ]
    } else {
      block(sticky: true, below: spacing.entry-content)[
        #pad(x: spacing.content-indent)[
          #grid(
            columns: (1fr, auto),
            column-gutter: 10pt,
            text(size: entry-size, weight: "bold", inline-text(heading.at("content"))),
            align(right, render-inlines(details.at("content"))),
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
  set text(font: body-font, size: body-size, fill: text-color, lang: "en")
  set par(justify: false, leading: leading)
  show link: set text(fill: link-color)
  show raw: set text(font: body-font)
  show heading.where(level: 1): it => block(
    sticky: true,
    above: section-gap,
    below: spacing.section-content,
  )[
    #grid(
      columns: (auto, 1fr),
      column-gutter: 5pt,
      align: horizon,
      text(size: body-size + 2pt, weight: "bold", fill: accent-color, it.body),
      line(length: 100%, stroke: 0.45pt + rgb("#B7B7B7")),
    )
  ]
  show list: set block(above: spacing.list-attachment, below: 0pt)
  show enum: set block(above: spacing.list-attachment, below: 0pt)

  let render-paragraph(entry, index, root: false, inside-list: false, sticky: false, below: spacing.paragraph) = {
    let body = render-inlines(entry.at("content"))
    if root and index == 1 {
      block(
        width: 100%,
        breakable: false,
        below: spacing.contact-summary,
        align(center, text(
          font: "Spectral",
          size: body-size + 1pt,
          render-contact(entry.at("content")),
        )),
      )
    } else if root and index == 2 {
      block(
        breakable: false,
        below: spacing.summary-end,
        par(leading: leading, body),
      )
    } else if inside-list {
      block(breakable: false, sticky: sticky, body)
    } else {
      block(
        breakable: false,
        sticky: sticky,
        below: below,
        pad(x: spacing.content-indent, body),
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
          render-title(entry.at("content"))
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
          render-paragraph(entry, index, root: root, inside-list: inside-list)
        }
      } else if kind == "list" {
        if inside-list { v(5.5pt, weak: true) }
        let items = entry.at("items").map(item => render-blocks(item.at("blocks"), inside-list: true))
        let list-kind = entry.at("list_kind")
        let indent = if inside-list { spacing.nested-list-indent } else { spacing.list-indent }
        if list-kind.at("type") == "ordered" {
          enum(
            start: list-kind.at("start"),
            indent: indent,
            body-indent: spacing.list-body-indent,
            spacing: spacing.list-items,
            ..items,
          )
        } else {
          list(
            marker: text(size: body-size, fill: accent-color, [•]),
            indent: indent,
            body-indent: spacing.list-body-indent,
            spacing: spacing.list-items,
            ..items,
          )
        }
      } else if kind == "divider" {
        v(1pt)
      }
    }
  }

  render-blocks(resume.blocks, root: true)
}
