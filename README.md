# Resumark

Keep your resume as a Markdown file you control. Edit it in Obsidian, VS Code,
Sublime, or any other Markdown editor. Resumark turns that file into a styled
PDF without storing it.

Use the web app to choose a theme and preview the pages. Use the CLI for the
same workflow from a terminal.

Rust parses the Markdown and gives Typst the resume data, theme, settings, and
fonts. One Typst compile produces both the PDF and SVG preview.

## CLI

```sh
cargo run --package resumark-cli -- \
  build examples/resume.md \
  --paper letter \
  --output-dir target/letter
```

This writes `resume.pdf` and one SVG per page. Use `--paper a4` for A4.

Inspect the parsed document without rendering it:

```sh
cargo run --package resumark-cli -- inspect examples/resume.md
```

## Browser check

```sh
npm install
npx playwright install chromium
npm run test:web
```

Playwright builds the release WASM and runs the browser workflow on port 8080.
See [AGENTS.md](AGENTS.md) for the full check.

## Resume spacing

Edit the `theme` dictionary at the top of
[`themes/minimal.typ`](themes/minimal.typ). `leading` controls line spacing and
`gap` controls space between blocks.

## Docs

- [Product](docs/project-plan.md)
- [Build order](docs/implementation-roadmap.md)
- [Rust structure](docs/rust-engineering-guide.md)
