/* resumark-theme
{
  "version": 1,
  "name": "Modern",
  "description": "A clean sans-serif layout with a strong accent and open spacing.",
  "controls": [
    { "kind": "font", "key": "font_family", "label": "Font", "group": "Typography", "value": "Source Sans 3", "options": ["Source Sans 3", "Libertinus Serif"] },
    { "kind": "number", "key": "body_size_pt", "label": "Body size", "group": "Typography", "value": 10.5, "min": 8.0, "max": 14.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "title_size_pt", "label": "Title size", "group": "Typography", "value": 28.0, "min": 18.0, "max": 38.0, "step": 0.5, "unit": "pt" },
    { "kind": "number", "key": "body_leading_em", "label": "Line spacing", "group": "Typography", "value": 0.5, "min": 0.0, "max": 1.2, "step": 0.02, "unit": "em" },
    { "kind": "number", "key": "page_margin_x_in", "label": "Side margin", "group": "Page", "value": 0.72, "min": 0.35, "max": 1.25, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "page_margin_y_in", "label": "Top and bottom margin", "group": "Page", "value": 0.68, "min": 0.35, "max": 1.25, "step": 0.01, "unit": "in" },
    { "kind": "number", "key": "section_gap_pt", "label": "Section gap", "group": "Spacing", "value": 12.0, "min": 0.0, "max": 24.0, "step": 0.25, "unit": "pt" },
    { "kind": "number", "key": "entry_gap_pt", "label": "Entry gap", "group": "Spacing", "value": 9.0, "min": 0.0, "max": 20.0, "step": 0.25, "unit": "pt" },
    { "kind": "color", "key": "text_color", "label": "Text", "group": "Color", "value": "#17212B" },
    { "kind": "color", "key": "muted_color", "label": "Muted text", "group": "Color", "value": "#536170" },
    { "kind": "color", "key": "accent_color", "label": "Accent", "group": "Color", "value": "#136F8A" }
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
  show title: it => block(below: 6pt)[
    #text(size: theme.at("title_size_pt") * 1pt, weight: "bold", fill: accent-color, it.body)
    #v(3pt)
    #line(length: 100%, stroke: 1.4pt + accent-color)
  ]
  show heading.where(level: 1): it => block(
    sticky: true,
    above: section-gap,
    below: 4pt,
  )[
    #text(size: 11.5pt, weight: "semibold", fill: accent-color, tracking: 0.06em, upper(it.body))
  ]
  show heading.where(level: 2): it => block(
    sticky: true,
    above: 0pt,
    below: 2pt,
    text(size: 11pt, weight: "semibold", it.body),
  )
  show list: set block(above: 1.5pt, below: 0pt)
  show enum: set block(above: 1.5pt, below: 0pt)

  let render-paragraph(entry, index, root: false, inside-list: false, sticky: false) = {
    let body = render-inlines(entry.at("content"))
    if root and index == 1 {
      block(breakable: false, below: 8pt, text(fill: muted-color, body))
    } else if root and index == 2 {
      block(breakable: false, below: 1pt, par(leading: 0.42em, body))
    } else if inside-list {
      block(breakable: false, sticky: sticky, body)
    } else {
      block(breakable: false, sticky: sticky, below: 2.5pt, body)
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
          enum(start: list-kind.at("start"), spacing: 4pt, ..items)
        } else {
          list(marker: [–], spacing: 4pt, ..items)
        }
      } else if kind == "divider" {
        v(5pt)
      }
    }
  }

  render-blocks(resume.blocks, root: true)
}
