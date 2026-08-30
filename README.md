# Resumark

A local-first Markdown resume compiler built with Rust and WebAssembly.

The first native vertical slice is implemented. It parses the realistic fixture
into a project-owned model, compiles it with a restricted in-memory Typst world,
and exports PDF and per-page SVG from the same compiled document:

```sh
cargo run --package resumark-cli -- \
  build fixtures/resume.md \
  --output target/resume.pdf
```

The command writes `target/resume.pdf`, `target/resume-1.svg`, and subsequent
numbered SVG pages beside the PDF.

To inspect the source-aware document model without rendering it:

```sh
cargo run --package resumark-cli -- inspect fixtures/resume.md
```

The analyzer reports invalid links, raw HTML, unsafe control characters, and
configured input limits with stable diagnostic codes and Markdown line/column
locations. Valid nodes retain UTF-8 byte ranges for the future browser editor.

Theme sizes and spacing are intentionally centralized in the `theme` dictionary
at the top of [`themes/minimal.typ`](themes/minimal.typ). Values named `leading`
control wrapped-line spacing; values named `gap` control spacing between blocks
such as contact details, sections, jobs, paragraphs, and list items.

Planning documents:

- [Project plan](docs/project-plan.md) — product thesis, architecture, and article direction
- [Implementation roadmap](docs/implementation-roadmap.md) — build-first stages for a usable local v1
- [Rust engineering guide](docs/rust-engineering-guide.md) — readability-first conventions and technical boundaries
