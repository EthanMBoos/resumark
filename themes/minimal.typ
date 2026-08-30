/* resumark-theme
{
  "version": 1,
  "name": "Minimal",
  "description": "A centered serif layout with quiet rules and compact spacing.",
  "controls": [
    { "kind": "font", "key": "font_family", "label": "Font", "group": "Typography", "value": "Libertinus Serif", "options": ["Libertinus Serif", "Source Sans 3"] },
    { "kind": "number", "key": "body_size_pt", "label": "Body size", "group": "Typography", "value": 11.5, "min": 8.0, "max": 14.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "title_size_pt", "label": "Title size", "group": "Typography", "value": 25.0, "min": 18.0, "max": 36.0, "step": 0.5, "unit": "pt" },
    { "kind": "number", "key": "body_leading_em", "label": "Line spacing", "group": "Typography", "value": 0.38, "min": 0.0, "max": 1.2, "step": 0.02, "unit": "em" },
    { "kind": "number", "key": "page_margin_x_in", "label": "Side margin", "group": "Page", "value": 0.78, "min": 0.35, "max": 1.25, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "page_margin_y_in", "label": "Top and bottom margin", "group": "Page", "value": 0.72, "min": 0.35, "max": 1.25, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "paragraph_gap_pt", "label": "Paragraph gap", "group": "Spacing", "value": 3.0, "min": 0.0, "max": 12.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "section_gap_pt", "label": "Section gap", "group": "Spacing", "value": 9.0, "min": 0.0, "max": 20.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "entry_gap_pt", "label": "Entry gap", "group": "Spacing", "value": 10.0, "min": 0.0, "max": 20.0, "step": 0.25, "unit": "pt" },
    { "kind": "color", "key": "text_color", "label": "Text", "group": "Color", "value": "#20242a" },
    { "kind": "color", "key": "muted_color", "label": "Muted text", "group": "Color", "value": "#4f5963" },
    { "kind": "color", "key": "accent_color", "label": "Accent", "group": "Color", "value": "#235c82" }
  ]
}
*/

#import "/resumark/v1.typ": paper-name, render-inlines

#let render(resume, settings, theme) = {
  let color(value) = rgb(value.slice(1))
  let config = (
    page-margin-x: theme.at("page_margin_x_in") * 1in,
    page-margin-y: theme.at("page_margin_y_in") * 1in,
    body-size: theme.at("body_size_pt") * 1pt,
    body-leading: theme.at("body_leading_em") * 1em,
    summary-leading: 0.28em,
    metadata-size: 10.5pt,
    title-size: theme.at("title_size_pt") * 1pt,
    section-size: 12pt,
    entry-title-size: 11pt,
    title-to-contact-gap: 6pt,
    contact-to-summary-gap: 7pt,
    paragraph-gap: theme.at("paragraph_gap_pt") * 1pt,
    section-gap-before: theme.at("section_gap_pt") * 1pt,
    section-rule-gap: 2pt,
    section-content-gap: 2pt,
    entry-gap-before: theme.at("entry_gap_pt") * 1pt,
    entry-title-gap-after: 2.25pt,
    list-attachment-gap: 1.5pt,
    list-item-gap: 4.5pt,
    divider-gap: 5pt,
    text-color: color(theme.at("text_color")),
    muted-color: color(theme.at("muted_color")),
    accent-color: color(theme.at("accent_color")),
  )

  set document(title: resume.metadata.title, author: resume.metadata.title)
  set page(
    paper: paper-name(settings),
    margin: (x: config.page-margin-x, y: config.page-margin-y),
  )
  set text(
    font: theme.at("font_family"),
    size: config.body-size,
    fill: config.text-color,
    lang: "en",
  )
  set par(justify: false, leading: config.body-leading)
  show list: set par(leading: config.list-attachment-gap)
  show enum: set par(leading: config.list-attachment-gap)
  show list: set block(above: config.list-attachment-gap, below: 0pt)
  show enum: set block(above: config.list-attachment-gap, below: 0pt)
  show link: set text(fill: config.accent-color)
  show raw: set text(font: theme.at("font_family"))
  show emph: set text(size: config.metadata-size, fill: config.muted-color)
  show title: it => align(center, block(
    below: config.title-to-contact-gap,
    text(size: config.title-size, weight: "bold", it.body),
  ))
  show heading.where(level: 1): it => block(
    sticky: true,
    above: config.section-gap-before,
    below: config.section-content-gap,
  )[
    #grid(
      columns: 1fr,
      rows: (auto, auto),
      row-gutter: config.section-rule-gap,
      text(size: config.section-size, weight: "bold", upper(it.body)),
      line(length: 100%, stroke: 0.55pt + config.muted-color),
    )
  ]
  show heading.where(level: 2): it => block(
    sticky: true,
    above: 0pt,
    below: config.entry-title-gap-after,
    text(size: config.entry-title-size, weight: "bold", it.body),
  )

  let render-paragraph(
    entry,
    index,
    root: false,
    inside-list: false,
    keep-with-next: false,
  ) = {
    let body = render-inlines(entry.at("content"))
    if root and index == 1 {
      align(center, block(
        breakable: false,
        below: config.contact-to-summary-gap,
        par(leading: config.body-leading, body),
      ))
    } else if root and index == 2 {
      block(breakable: false, par(leading: config.summary-leading, body))
    } else if inside-list {
      block(
        breakable: false,
        sticky: keep-with-next,
        par(leading: config.body-leading, body),
      )
    } else {
      block(
        breakable: false,
        sticky: keep-with-next,
        below: config.paragraph-gap,
        par(leading: config.body-leading, body),
      )
    }
  }

  let render-blocks(blocks, root: false, inside-list: false) = {
    for (index, entry) in blocks.enumerate() {
      let kind = entry.at("type")
      let previous = if index > 0 { blocks.at(index - 1) } else { none }
      if kind == "heading" {
        let level = entry.at("level")
        let body = render-inlines(entry.at("content"))
        if level == 1 {
          title(body)
        } else if level == 2 {
          heading(level: 1, outlined: true, body)
        } else {
          let follows-section-heading = (
            previous != none
              and previous.at("type") == "heading"
              and previous.at("level") == 2
          )
          if not follows-section-heading {
            v(config.entry-gap-before, weak: true)
          }
          heading(level: 2, outlined: true, body)
        }
      } else if kind == "paragraph" {
        let follows-entry-heading = (
          previous != none
            and previous.at("type") == "heading"
            and previous.at("level") >= 3
        )
        render-paragraph(
          entry,
          index,
          root: root,
          inside-list: inside-list,
          keep-with-next: follows-entry-heading,
        )
      } else if kind == "list" {
        let items = entry.at("items").map(item => render-blocks(
          item.at("blocks"),
          inside-list: true,
        ))
        let list-kind = entry.at("list_kind")
        if list-kind.at("type") == "ordered" {
          enum(start: list-kind.at("start"), spacing: config.list-item-gap, ..items)
        } else {
          list(marker: [•], spacing: config.list-item-gap, ..items)
        }
      } else if kind == "divider" {
        v(config.divider-gap)
      }
    }
  }

  render-blocks(resume.blocks, root: true)
}
