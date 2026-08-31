# Resumark

Keep a resume as a Markdown file. Edit it in Obsidian, VS Code, Sublime, or any
other Markdown editor. Open that file in Resumark, choose a theme, and download
the PDF. Nothing is uploaded or saved.

The web app includes four starter themes. Each theme has simple controls for
fonts, sizes, spacing, margins, and colors. Download any starter as one `.typ`
file to make a custom theme, then open that theme in the app or use it from the
CLI.

## CLI

Build with the default theme:

```sh
cargo run --package resumark-cli -- \
  build examples/resume.md \
  --paper letter \
  --output-dir target/letter
```

List the starter themes and choose one:

```sh
cargo run --package resumark-cli -- themes
cargo run --package resumark-cli -- \
  build examples/resume.md --theme modern
```

Export a starter, edit it, and build with the custom file:

```sh
cargo run --package resumark-cli -- export-theme minimal my-theme.typ
cargo run --package resumark-cli -- \
  build examples/resume.md --theme-file my-theme.typ
```

Each build writes one PDF and one SVG per page. Use `--paper a4` for A4.

Inspect the parsed resume without rendering it:

```sh
cargo run --package resumark-cli -- inspect examples/resume.md
```

## Web app

```sh
npm run dev:web
```

This starts the local app on port 8080 and opens it in the browser. The page
starts empty so the normal flow begins by opening a local `resume.md` file.

## Themes

Most custom themes should start as a copy of Default, Modern, Compact, or
Jake's Resume. A theme is one `.typ` file with a JSON manifest at the top:

```typst
/* resumark-theme
{
  "version": 1,
  "name": "My Theme",
  "description": "A short description.",
  "controls": [
    {
      "kind": "color",
      "key": "accent_color",
      "label": "Accent",
      "group": "Color",
      "value": "#235C82"
    }
  ]
}
*/
```

Controls can be numbers, `#RRGGBB` colors, or a choice between the bundled
Libertinus Serif and Source Sans 3 fonts. The web app updates values in this
manifest.

The Typst source imports any helpers it needs and exports one function:

```typst
#import "/resumark/v1.typ": paper-name, render-inlines

#let render(resume, settings, theme) = {
  set page(paper: paper-name(settings))
  // Build the document from resume.metadata and resume.blocks.
}
```

The starter files under [`themes/`](themes/) are the full examples. Jake's
Resume is based on [Jake Gutierrez's MIT-licensed LaTeX
template](https://github.com/jakegut/resume).
Themes can use the resume data, settings, helper API, and bundled fonts. Local
files, package imports, network access, and uploaded fonts are not available.

## Browser check

```sh
npm install
npx playwright install chromium
npm run test:web
```

Playwright builds the release WASM and runs the file-to-PDF workflow on port
8080. See [AGENTS.md](AGENTS.md) for the full check.
