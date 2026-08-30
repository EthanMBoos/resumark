#let paper-name(settings) = if settings.at("paper") == "a4" {
  "a4"
} else {
  "us-letter"
}

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
