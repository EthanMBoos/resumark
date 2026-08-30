# Resumark

A side-project brief for building a fast, local-first resume editor in Rust and WebAssembly, then writing about what the project reveals about performance, document rendering, and choosing where Rust actually helps.

## Implementation plans

This document describes the product and its technical thesis. The executable build sequence lives in the [implementation roadmap](implementation-roadmap.md), with the project-specific Rust conventions in the [Rust engineering guide](rust-engineering-guide.md).

The implementation plans deliberately build a working native compiler and test Typst in WebAssembly before investing in the full editor. Broad test, CI, and release scaffolding follows the usable product loop instead of preceding it.

## Scope discipline

This is a small side project aiming for a good-enough v1, not an attempt to
design a general document platform. Code earns its place when it enables the
next visible workflow, protects user work, or preserves a boundary that would
be costly to retrofit. A hypothetical future feature is not enough.

For v1:

- Prefer one concrete implementation over a trait or configurable framework.
- Keep code beside its only caller until a second real consumer needs a shared
  boundary.
- Add focused checks for security, data loss, and preview/export parity; defer
  broad matrices and release scaffolding.
- Measure suspected performance problems once, then optimize only the ones a
  person can actually feel.
- Stop at one resume, one theme, Letter/A4, local saving, Markdown portability,
  preview, diagnostics, and PDF export.

The intentionally durable pieces are the typed document model, safe Typst
boundary, and shared compiled result for preview and PDF. Most other structure
should remain easy to replace.

## Project thesis

The interesting version of this project is not "rewrite a React resume builder in Rust."

It is:

> Build a static, local-first resume compiler that turns Markdown into an exact page preview and an accessible PDF, entirely in the browser.

The public website stays deliberately boring: one hand-written HTML page, one CSS file, a few local images, and a link to the editor. The editor is a separate Rust/WASM application. There are no accounts, no database server, and ideally no network requests after the application has loaded.

Rust does not matter because parsing a two-page Markdown file is computationally difficult. It matters because it makes it possible to share a typed document model, renderer, browser application, and native CLI—and because Typst can replace an expensive headless Chromium PDF pipeline.

The real performance thesis is:

> Rust lets the project replace a headless browser with a deterministic local document compiler.

## Why this is worth building

The reference project gets the central interaction right: write Markdown, see the resume, export a PDF. Its weaknesses are everything around that loop:

- A desktop-only editor
- Fragile `localStorage` hydration
- A server endpoint that accepts rendered HTML
- A fresh unsandboxed Chromium process for every PDF
- Separate CSS implementations for browser preview and PDF output
- No source import/export, history, page awareness, or multiple documents
- No strong reason to exist beyond "Markdown on the left, PDF on the right"

This project can be smaller operationally while being more ambitious technically:

- Static deployment
- Local-first persistence
- Exact preview/export parity
- Tagged, selectable-text PDF output
- A reusable native CLI
- A typed intermediate representation that enables future resume profiles and checks

## Proposed architecture

```text
Static public site
    site/index.html + site.css
              │
              └── /app/
                    Rust/WASM application
                            │
                     Markdown source
                            │
                    pulldown-cmark
                            │
                   typed RenderDocument
                            │
                 Typst worker + themes
                       ┌────┴────┐
                  SVG preview   PDF
```

Everything after the browser loads should run locally if the Typst/WASM experiment is successful.

## Public website

Keep the marketing site intentionally plain:

```text
site/
├── index.html
├── styles.css
├── favicon.svg
└── screenshots/
```

Constraints:

- No framework
- No runtime JavaScript
- No external font requests
- No analytics initially
- Semantic HTML
- Responsive CSS
- Local screenshots with explicit dimensions
- A primary link to `/app/`

This isolates the editor's larger WASM and font assets from the first public page. Someone can understand the project before downloading a document compiler.

## Web application

Use [Leptos](https://book.leptos.dev/) in client-side-rendered mode. It provides Rust components and fine-grained signals while still producing static deployment artifacts. Use Trunk for development and release builds.

"All Rust" means no meaningful hand-written JavaScript. The browser will still require HTML, CSS, Web APIs, a generated JavaScript loader, and `wasm-bindgen` glue to instantiate the WASM modules.

Leptos is the default choice because this is a web-only application. Dioxus would become more compelling if native desktop and mobile applications became an explicit goal.

### Editor

Start with a native `<textarea>`.

That provides keyboard input, selection, undo, spellchecking, mobile behavior, clipboard support, input-method editor support, and accessibility without introducing a large editor dependency.

Small enhancements can be implemented around it:

- Tab indentation
- Markdown formatting shortcuts
- Find and replace
- Line and column status
- Source-offset diagnostics
- Optional synchronized scrolling

These are post-v1 ideas, not an initial checklist. Do not build a
syntax-highlighting editor in the first version. If that eventually becomes
important, accepting CodeMirror as the project's one JavaScript dependency is
probably better than maintaining an editor engine.

## Markdown pipeline

Use [`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark) as a small, allocation-conscious CommonMark parser with source-offset support.

Convert parser events into a project-owned representation:

```rust
struct RenderDocument {
    metadata: Metadata,
    blocks: Vec<Block>,
}

enum Block {
    Heading {
        level: u8,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List {
        ordered: bool,
        items: Vec<Vec<Block>>,
    },
    Divider,
}

enum Inline {
    Text(String),
    Strong(Vec<Inline>),
    Emphasis(Vec<Inline>),
    Link {
        label: Vec<Inline>,
        url: String,
    },
    Code(String),
}
```

This model is the contract between parsing, validation, themes, the web application, and the CLI. It must not know anything about Leptos, the DOM, SVG, or PDF.

Initial Markdown policy:

- Disable raw HTML
- Support headings, paragraphs, lists, emphasis, links, code, and dividers
- Allow only `https:`, `http:`, `mailto:`, and `tel:` links
- Put explicit limits on document size and nesting depth
- Preserve source offsets for useful error messages

## Typst rendering

[Typst](https://typst.app/) is the key technical bet. Its compiler is written in Rust, lays content out into fixed pages, and can export a compiled document as PDF or one SVG per page. Current Typst also writes tagged PDFs by default and can enforce PDF/UA-1 accessibility requirements.

The rendering pipeline should be:

```text
Markdown
  → typed Rust model
  → JSON
  → trusted theme.typ
  → Typst PagedDocument
  → SVG pages or PDF
```

Do not interpolate user content into Typst source. Serialize `RenderDocument` with Serde and expose it as `/resume.json` inside an in-memory virtual filesystem. A trusted theme reads that file and constructs semantic Typst elements.

The custom Typst `World` should expose only:

- `/resume.json`
- The selected trusted theme
- Bundled, appropriately licensed fonts
- Explicitly validated user assets, if image support is added later

It should have no access to the user's filesystem, packages, or network.

Keep all direct Typst APIs behind one crate and pin the exact Typst version. The library is pre-1.0 and future upgrades may require adapter changes.

### Preview and export

Compile once and reuse the result:

- Export page SVGs for the browser preview.
- Export the same `PagedDocument` to PDF on demand.
- Keep the most recent valid compiled document in memory.
- Show diagnostics without replacing the last valid preview.

Display generated SVG through Blob URLs rather than injecting SVG strings directly into the application DOM.

Run Typst in a dedicated Web Worker:

- Keep typesetting off the UI thread.
- Start with one request and one response for the browser fixture.
- Add debouncing and a small revision number only when live editing creates
  overlapping work.
- Add worker timeout/restart behavior only if a real hang is observed.
- Add incremental compiler state only after measurements show it materially
  improves editing.

## PDF quality and ATS claims

Typst provides a much better foundation than browser printing or a manually constructed PDF:

- Selectable Unicode text
- Semantic headings and lists
- Tagged PDFs by default
- PDF/UA-1 validation
- Embedded fonts
- Stable paper geometry
- Exact preview/export layout

This is not enough to promise that every applicant tracking system will parse every document correctly. The product should make narrower, testable claims:

- Text extraction preserves the expected order.
- Links remain links.
- Headings and lists are tagged semantically.
- A single-column theme has a predictable reading order.
- The document passes the selected PDF standard checks.

After the usable workflow exists, automated checks can extract text from
generated fixtures. PDF/UA builds can also be checked with
[veraPDF](https://verapdf.org/), which Typst's accessibility documentation
recommends.

## Persistence

For one small resume, use one versioned JSON value in `localStorage` containing
the Markdown and settings. Load it synchronously before installing autosave so
sample content can never overwrite restored content.

Keep this implementation in the web application. Do not add a storage trait,
repository, database schema, migration framework, or IndexedDB dependency for
v1. Save after a short idle delay, report storage failures honestly, and always
offer Markdown download so the browser is not the only copy.

IndexedDB becomes worthwhile only if multiple documents, revision history,
large assets, or observed storage limits make the single record inadequate.

## Workspace layout

Keep the v1 workspace close to the current layout. Add the browser application
when the WASM spike begins, but do not split storage or worker protocol crates
until duplicated code or independently built consumers make that separation
clearly simpler.

```text
resumark/
├── Cargo.toml
├── site/
│   ├── index.html
│   └── styles.css
├── apps/
│   ├── web/
│   └── cli/
├── crates/
│   ├── resumark-core/
│   └── resumark-render-typst/
├── themes/
│   └── minimal.typ
└── fonts/
```

Responsibilities:

- `resumark-core`: Serializable model, Markdown parsing, validation, and diagnostics
- `resumark-render-typst`: In-memory `World`, trusted theme, SVG/PDF export
- `web`: Leptos UI and worker coordination
- `cli`: Native compilation, validation, and inspection commands

## Native CLI

The native CLI is a development harness for the shared compiler during v1:

```bash
resume build resume.md
resume build resume.md --paper a4
resume inspect resume.md
```

A polished command surface, preview launcher, profile generation, packaging, and
CI use can wait until the browser product is useful. The harness still earns
its place by exercising the same parser, model, theme, and renderer.

## Performance plan

Rust/WASM does not guarantee a smaller or faster application. WASM binaries are often larger than equivalent JavaScript bundles, and the Typst compiler plus fonts will dominate the download.

For the browser spike, measure enough to catch a clearly impractical result:

- Keep the public page independent from the application bundle.
- Compile Typst in one worker with the existing four font faces.
- Record production artifact size and rough cold render time on the development
  machine.
- Open the spike in one current desktop browser and confirm it remains usable.

If the result is obviously too large, slow, or incompatible, investigate before
building the editor. Brotli budgets, mobile benchmarks, peak-memory work,
incremental timing, caching policy, and `wasm-opt` tuning belong after a usable
editing loop makes them relevant.

## Fallback architecture

Client-side Typst is the biggest uncertainty. Community browser wrappers exist, but they are comparatively young. The project should prove the official Rust crates under `wasm32-unknown-unknown` before committing to a particular wrapper.

If browser compilation proves too large or unreliable, preserve the shared
model and renderer while deciding on a fallback. A small native service is one
possible shape:

```text
Leptos app → typed document JSON → Axum → native Typst → PDF/SVG
```

Do not design or scaffold this service during the browser spike. If it becomes
necessary, retain the typed-document and restricted-renderer boundaries and
scope the service from the observed failure. A server fallback would sacrifice
some privacy and offline use, but it would not invalidate the core compiler.

## Security and privacy

For the fully local build:

- Set a restrictive content security policy.
- Use no third-party scripts or fonts.
- Make `connect-src 'none'` possible.
- Reject raw HTML and dangerous URL schemes.
- Keep templates trusted and bundled.
- Restrict document, nesting, and asset sizes.
- Do not support arbitrary user Typst code initially.

The product should be able to say, truthfully:

> Your resume never leaves your browser.

That is a real differentiator for a document containing addresses, phone numbers, employment history, and other personal information.

## Post-v1 testing ideas

The lists below are a backlog, not requirements for the first usable version.
Before v1, verification should stay proportional to each slice: manually use
the visible workflow and automate only dangerous boundaries or regressions.

### Rust core

- Unit tests for every Markdown construct
- Source-offset and diagnostic tests
- Property tests for nested Markdown
- Fuzzing for parser-to-model conversion
- URL scheme validation tests
- Theme setting validation

### Rendering

- Golden SVG fixtures for each theme
- Extracted PDF text assertions
- Link preservation assertions
- Page-count and overflow tests
- A4 and Letter fixtures
- Unicode and right-to-left fixtures
- PDF/UA-1 conformance checks
- Deterministic metadata where possible

### Browser

- `localStorage` load/save recovery tests
- Save/reload recovery tests
- Worker cancellation and stale-result tests
- Keyboard and mobile interaction tests
- Chrome, Firefox, and Safari smoke tests
- Performance and bundle-size budgets in CI

The production code can remain Rust even if Playwright is used as pragmatic test-only tooling. "All Rust" is not valuable enough to justify weaker browser testing.

## Milestones

### Milestone 1: native compiler spike

Before building the UI:

1. Parse one realistic resume with `pulldown-cmark`.
2. Convert it into `RenderDocument`.
3. Render one trusted Typst theme.
4. Export PDF and page SVGs.
5. Assert the PDF's extracted text order.
6. Add Letter/A4 selection and an overflow diagnostic.

This validates the central architecture using normal native Rust, without WebAssembly friction.

### Milestone 2: browser compiler spike

- Minimal browser page
- Compile the rendering crate for `wasm32-unknown-unknown`
- Load the trusted theme and four font faces in a worker
- Display fixture SVG pages and download its PDF
- Record approximate artifact size and first-render time on the development machine

This is the project's go/no-go point for a completely static architecture.

### Milestone 3: editing loop

- Native Markdown textarea
- Responsive edit/preview layout
- Live worker rendering with simple stale-response protection if needed
- Source diagnostics
- Letter/A4 control
- PDF download from the same compile as the visible preview

### Milestone 4: good-enough local v1

- One resume saved as one versioned `localStorage` record
- Honest autosave status and recovery after reload
- Markdown import and export
- One bundled theme
- Manual desktop and narrow-layout check

Stop here and use the application before expanding its scope.

### Later product depth, only if use justifies it

- Multiple named resumes
- Version history and restore
- Page-break controls
- Page overflow and orphan-heading warnings
- Accessible PDF export
- Transparent ATS-oriented checks
- CLI release
- Static hosting and offline caching

### Possible differentiating features

- A master career inventory
- Tagged content blocks
- Job-specific profiles
- Profile diffs
- Deterministic CI builds
- JSON Resume import/export
- Shareable static HTML resumes
- Trusted custom theme packages

## Features to avoid initially

- Accounts and cloud sync
- Collaboration
- DOCX import/export
- Arbitrary custom CSS or Typst code
- Images and multi-column layouts
- A fake universal "ATS score"
- Generic AI rewriting
- A custom rich-text editor
- Dozens of fonts and themes

The first version should prove that a local Rust document compiler can be fast, safe, exact, and pleasant to use.

## Article direction

The strongest article is not a build log titled "I Made a Resume Builder in Rust."

The sharper thesis is:

> Rust did not make the text editor faster. It made the browser unnecessary to the document pipeline.

Possible working titles:

- **The Fast Part Wasn't Markdown**
- **Replacing Headless Chrome With a Document Compiler**
- **What "All Rust" Actually Means in a Browser**
- **A Resume Builder With No Backend**
- **The Compiler Behind a Two-Page Document**
- **Rust, WebAssembly, and the Cost of Avoiding JavaScript**

Possible narrative structure:

1. Start with the original Markdown resume project and its attractive simplicity.
2. Follow PDF export until the simple app launches an entire browser on a server.
3. Ask where performance and complexity actually live.
4. Introduce Typst as a document compiler, not a PDF library.
5. Build one typed pipeline that emits both preview and PDF.
6. Move it into WebAssembly and confront the cold-start and bundle-size cost.
7. End with measured tradeoffs rather than a language victory lap.

Useful measurements to collect opportunistically after the workflow exists:

- Static landing-page transfer size and render time
- Leptos editor WASM size
- Typst compiler WASM size before and after optimization
- Font payload size
- Cold and incremental render latency
- Browser memory during compilation
- Native CLI versus browser compilation time
- Typst PDF size versus Chromium PDF size
- Text extraction and accessibility results
- Chromium server memory eliminated by the final architecture

The article should be honest if the Rust/WASM application has a slower cold start than a JavaScript implementation. That tension is the interesting part.

The likely closing idea:

> The win was not replacing JavaScript with Rust. The win was replacing a screenshot-shaped workflow with a compiler-shaped one.

## Initial research references

- [Leptos Book](https://book.leptos.dev/)
- [Leptos: optimizing WASM binary size](https://book.leptos.dev/deployment/binary_size.html)
- [`wasm-bindgen` guide](https://rustwasm.github.io/docs/wasm-bindgen/)
- [`web-sys` browser API bindings](https://wasm-bindgen.github.io/wasm-bindgen/api/web_sys/)
- [`pulldown-cmark`](https://github.com/pulldown-cmark/pulldown-cmark)
- [Typst repository](https://github.com/typst/typst)
- [Typst compiler crate](https://docs.rs/typst/latest/typst/)
- [Typst PDF documentation](https://typst.app/docs/reference/pdf/)
- [Typst accessibility guide](https://typst.app/docs/guides/accessibility/)
- [Typst compiler architecture](https://github.com/typst/typst/blob/main/docs/dev/architecture.md)
- [`typst-pdf`](https://docs.rs/typst-pdf/latest/typst_pdf/)
- [`typst-svg`](https://docs.rs/typst-svg/latest/typst_svg/)
- [JSON Resume schema](https://jsonresume.org/schema)

## First next step

Create the Cargo workspace and stop after one vertical slice works:

```text
resume.md
  → pulldown-cmark
  → RenderDocument
  → trusted Typst theme
  → resume.pdf + page-1.svg
```

Do that natively first. Then compile exactly the same rendering crate to WebAssembly and measure it before designing the rest of the product.
