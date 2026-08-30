#let resume = json("/resume.json")

// All visual tuning lives here. "Leading" controls spacing between wrapped
// lines; "gap" values control spacing between distinct blocks.
#let theme = (
  // Page and type.
  page-margin-x: 0.78in,
  page-margin-y: 0.72in,
  body-size: 11.5pt,
  body-leading: 0.38em,
  summary-leading: 0.28em,
  metadata-size: 10.5pt,
  title-size: 25pt,
  section-size: 12pt,
  entry-title-size: 11pt,

  // Vertical rhythm. Adjust these before changing renderer code.
  title-to-contact-gap: 6pt, // Name -> contact row; clears large-title descenders.
  contact-to-summary-gap: 7pt, // Contact row -> introduction.
  paragraph-gap: 3pt, // Ordinary paragraph -> next block.
  section-gap-before: 9pt, // Previous content -> section heading.
  section-rule-gap: 2pt, // Section label -> underline.
  section-content-gap: 2pt, // Section underline -> first content.
  entry-gap-before: 10pt, // Previous entry -> next job/project title.
  entry-title-gap-after: 2.25pt, // Job/project title -> metadata.
  list-attachment-gap: 1.5pt, // Paragraph -> attached list, including nesting.
  list-item-gap: 4.5pt, // Space between bullets.
  divider-gap: 5pt, // Markdown divider -> following section.

  // Color.
  text-color: rgb("20242a"),
  muted-color: rgb("4f5963"),
  link-color: rgb("235c82"),
  rule-color: rgb("8090a0"),
)

#set document(title: resume.metadata.title, author: resume.metadata.title)
#set page(
  paper: "us-letter",
  margin: (x: theme.page-margin-x, y: theme.page-margin-y),
)
#set text(
  font: "Libertinus Serif",
  size: theme.body-size,
  fill: theme.text-color,
  lang: "en",
)
#set par(justify: false, leading: theme.body-leading)
#show list: set par(leading: theme.list-attachment-gap)
#show enum: set par(leading: theme.list-attachment-gap)
#show list: set block(above: theme.list-attachment-gap, below: 0pt)
#show enum: set block(above: theme.list-attachment-gap, below: 0pt)
#show link: set text(fill: theme.link-color)
#show raw: set text(font: "Libertinus Serif")
#show emph: set text(size: theme.metadata-size, fill: theme.muted-color)
#show title: it => align(center, block(
  below: theme.title-to-contact-gap,
  text(size: theme.title-size, weight: "bold", it.body),
))
#show heading.where(level: 1): it => block(
  sticky: true,
  above: theme.section-gap-before,
  below: theme.section-content-gap,
)[
  #grid(
    columns: 1fr,
    rows: (auto, auto),
    row-gutter: theme.section-rule-gap,
    text(size: theme.section-size, weight: "bold", upper(it.body)),
    line(length: 100%, stroke: 0.55pt + theme.rule-color),
  )
]
#show heading.where(level: 2): it => block(
  sticky: true,
  above: 0pt,
  below: theme.entry-title-gap-after,
  text(size: theme.entry-title-size, weight: "bold", it.body),
)

#let render-inlines(nodes) = {
  for node in nodes {
    let kind = node.at("type")
    if kind == "text" {
      node.at("value")
    } else if kind == "strong" {
      strong(render-inlines(node.at("content")))
    } else if kind == "emphasis" {
      emph(render-inlines(node.at("content")))
    } else if kind == "link" {
      link(node.at("destination"), render-inlines(node.at("label")))
    } else if kind == "code" {
      raw(node.at("value"))
    } else if kind == "soft_break" {
      " "
    } else if kind == "hard_break" {
      linebreak()
    }
  }
}

#let render-paragraph(
  entry,
  index,
  root: false,
  inside-list: false,
  keep-with-next: false,
) = {
  let body = render-inlines(entry.at("content"))

  // The root document starts with title, contact details, then summary.
  if root and index == 1 {
    align(center, block(
      breakable: false,
      below: theme.contact-to-summary-gap,
      par(leading: theme.body-leading, body),
    ))
  } else if root and index == 2 {
    block(
      breakable: false,
      par(leading: theme.summary-leading, body),
    )
  } else if inside-list {
    block(
      breakable: false,
      sticky: keep-with-next,
      par(leading: theme.body-leading, body),
    )
  } else {
    block(
      breakable: false,
      sticky: keep-with-next,
      below: theme.paragraph-gap,
      par(leading: theme.body-leading, body),
    )
  }
}

#let render-blocks(blocks, root: false, inside-list: false) = {
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
          v(theme.entry-gap-before, weak: true)
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
      if entry.at("ordered") {
        enum(start: entry.at("start"), spacing: theme.list-item-gap, ..items)
      } else {
        list(marker: [•], spacing: theme.list-item-gap, ..items)
      }
    } else if kind == "divider" {
      // Section headings already draw a rule. The Markdown divider provides
      // separation without creating a visually heavy double line.
      v(theme.divider-gap)
    }
  }
}

#render-blocks(resume.blocks, root: true)
