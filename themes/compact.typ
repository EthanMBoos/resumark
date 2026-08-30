/* resumark-theme
{
  "version": 1,
  "name": "Compact",
  "description": "A dense sans-serif layout for fitting more experience on fewer pages.",
  "controls": [
    { "kind": "font", "key": "font_family", "label": "Font", "group": "Typography", "value": "Source Sans 3", "options": ["Source Sans 3", "Libertinus Serif"] },
    { "kind": "number", "key": "body_size_pt", "label": "Body size", "group": "Typography", "value": 9.5, "min": 8.0, "max": 13.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "title_size_pt", "label": "Title size", "group": "Typography", "value": 22.0, "min": 17.0, "max": 32.0, "step": 0.5, "unit": "pt" },
    { "kind": "number", "key": "body_leading_em", "label": "Line spacing", "group": "Typography", "value": 0.28, "min": 0.0, "max": 1.0, "step": 0.02, "unit": "em" },
    { "kind": "number", "key": "page_margin_x_in", "label": "Side margin", "group": "Page", "value": 0.52, "min": 0.3, "max": 1.1, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "page_margin_y_in", "label": "Top and bottom margin", "group": "Page", "value": 0.48, "min": 0.3, "max": 1.1, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "section_gap_pt", "label": "Section gap", "group": "Spacing", "value": 6.0, "min": 0.0, "max": 18.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "entry_gap_pt", "label": "Entry gap", "group": "Spacing", "value": 5.0, "min": 0.0, "max": 16.0, "step": 0.25, "unit": "pt" },
    { "kind": "color", "key": "text_color", "label": "Text", "group": "Color", "value": "#1F2328" },
    { "kind": "color", "key": "muted_color", "label": "Muted text", "group": "Color", "value": "#59636E" },
    { "kind": "color", "key": "accent_color", "label": "Accent", "group": "Color", "value": "#2F4858" }
  ]
}
*/

#import "/resumark/v1.typ": paper-name, render-inlines

#let render(resume, settings, theme) = {
  let color(value) = rgb(value.slice(1))
  let body-size = theme.at("body_size_pt") * 1pt
  let leading = theme.at("body_leading_em") * 1em
  let text-color = color(theme.at("text_color"))
  let muted-color = color(theme.at("muted_color"))
  let accent-color = color(theme.at("accent_color"))
  let section-gap = theme.at("section_gap_pt") * 1pt
  let entry-gap = theme.at("entry_gap_pt") * 1pt

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
  show link: set text(fill: accent-color)
  show raw: set text(font: theme.at("font_family"))
  show emph: set text(size: body-size - 0.5pt, fill: muted-color)
  show title: it => align(center, block(below: 2pt,
    text(size: theme.at("title_size_pt") * 1pt, weight: "bold", it.body),
  ))
  show heading.where(level: 1): it => block(
    sticky: true,
    above: section-gap,
    below: 2pt,
  )[
    #grid(
      columns: (auto, 1fr),
      column-gutter: 7pt,
      align: horizon,
      text(size: 10pt, weight: "bold", fill: accent-color, tracking: 0.04em, upper(it.body)),
      line(length: 100%, stroke: 0.6pt + muted-color),
    )
  ]
  show heading.where(level: 2): it => block(
    sticky: true,
    above: 0pt,
    below: 1pt,
    text(size: body-size + 0.25pt, weight: "semibold", it.body),
  )
  show list: set block(above: 0.5pt, below: 0pt)
  show enum: set block(above: 0.5pt, below: 0pt)

  let render-paragraph(entry, index, root: false, inside-list: false, sticky: false) = {
    let body = render-inlines(entry.at("content"))
    if root and index == 1 {
      align(center, block(breakable: false, below: 4pt, text(fill: muted-color, body)))
    } else if root and index == 2 {
      block(breakable: false, par(leading: 0.22em, body))
    } else if inside-list {
      block(breakable: false, sticky: sticky, body)
    } else {
      block(breakable: false, sticky: sticky, below: 1pt, body)
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
          let follows-section = previous != none and previous.at("type") == "heading" and previous.at("level") == 2
          if not follows-section { v(entry-gap, weak: true) }
          heading(level: 2, outlined: true, body)
        }
      } else if kind == "paragraph" {
        let follows-entry = previous != none and previous.at("type") == "heading" and previous.at("level") >= 3
        render-paragraph(entry, index, root: root, inside-list: inside-list, sticky: follows-entry)
      } else if kind == "list" {
        let items = entry.at("items").map(item => render-blocks(item.at("blocks"), inside-list: true))
        let list-kind = entry.at("list_kind")
        if list-kind.at("type") == "ordered" {
          enum(start: list-kind.at("start"), spacing: 2pt, ..items)
        } else {
          list(marker: [•], spacing: 2pt, ..items)
        }
      } else if kind == "divider" {
        v(3pt)
      }
    }
  }

  render-blocks(resume.blocks, root: true)
}
