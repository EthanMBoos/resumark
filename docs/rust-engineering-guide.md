# Rust Engineering Guide

This guide describes how Resumark should be structured and written as it grows. The priority order is:

1. Build a working vertical slice.
2. Keep the code understandable to someone who does not read Rust fluently.
3. Preserve the boundaries that make the native CLI and browser application share the same compiler.
4. Optimize only after measuring the real Typst and WebAssembly costs.

This is a side project, not a framework. Add a crate, trait, generic abstraction, build tool, or test dependency only when the current stage needs it.

## Grow the workspace with the product

Start the native slice with three packages:

```text
apps/
  cli/                         Native command-line application
crates/
  resumark-core/               Model, Markdown parsing, validation, diagnostics
  resumark-render-typst/       The only code that calls Typst APIs
fixtures/
  resume.md                    One realistic end-to-end input
themes/
  minimal.typ                  The first trusted theme
fonts/                         Bundled fonts and their licenses
```

The initial dependency direction is deliberately small. In these diagrams, `A -> B` means that package A depends on package B:

```text
cli -> resumark-core
cli -> resumark-render-typst -> resumark-core
```

The renderer consumes a typed `RenderDocument`; it does not parse Markdown. The CLI coordinates parsing and rendering. This makes the intermediate model a real contract without splitting every module into its own crate.

When browser compilation begins, add only the packages needed to keep the editor shell and compiler payload separate:

```text
apps/
  web/                         Leptos CSR editor; no Typst dependency
  render-worker/               Typst compiler Web Worker
crates/
  resumark-worker-protocol/    Serializable request and response enums
```

The resulting dependency direction is:

```text
cli -> resumark-core
cli -> resumark-render-typst -> resumark-core
web -> resumark-core
web -> resumark-worker-protocol -> resumark-core
render-worker -> resumark-worker-protocol
render-worker -> resumark-render-typst -> resumark-core
```

`resumark-worker-protocol` is justified because two independently built WASM modules must agree on messages without making the worker depend on Leptos. It may depend on public model, settings, and diagnostic types from `resumark-core`; it must not contain rendering or UI behavior.

Keep IndexedDB code under `apps/web/src/storage/` initially. A separate storage crate or repository trait is useful only after a second implementation has a real consumer. Until then, a concrete `IndexedDbDocumentStore` is easier to follow than an async trait, boxed futures, or generic dependency injection.

### Boundary rules

- `resumark-core` contains no Typst, Leptos, browser, filesystem, or CLI types.
- `resumark-render-typst` contains every direct import from the `typst`, `typst-pdf`, and `typst-svg` crates.
- `render-worker` owns the current compiled Typst document. The editor never receives or understands it.
- `web` owns Leptos signals, browser events, Blob URLs, IndexedDB, and timers.
- `cli` and `web` convert library errors into user-facing output. Libraries do not print, log, or exit.
- Modules and fields are private by default. Use `pub(crate)` for collaboration inside a crate and `pub` only for the small cross-crate contract.

Do not create a general `common`, `utils`, `manager`, or `services` crate. Put a helper beside the behavior it supports until there is a clear, stable owner.

## Stage-aware engineering choices

### Native compiler slice

The first implementation should make this command work for one realistic fixture:

```text
resume build fixtures/resume.md --paper letter --output-dir target/letter
```

It should also write one SVG per page. At this stage:

- Use one trusted theme, one small licensed font family, Letter paper, and straightforward errors.
- Keep the core model owned and serializable from the start.
- Keep Typst behind its adapter from the start because its pre-1.0 Rust API is the main dependency risk.
- Use one end-to-end smoke check that builds the fixture and extracts a few expected text lines from the PDF.
- Do not pause for CI matrices, coverage, fuzzing, snapshots, migrations, or a complete command surface.

### Core hardening

After the happy path works, finish the supported Markdown subset, source ranges, link validation, nesting and size limits, structured diagnostics, A4 support, and overflow reporting. Add focused tests beside parsing and validation behavior as it becomes real. Keep large realistic inputs in `fixtures/`; keep small syntax examples directly in tests when that is easier to read.

### Browser compiler spike

Compile the same renderer to `wasm32-unknown-unknown` and run it in a minimal dedicated worker before building the complete editor. The spike must answer:

- Can the pinned Typst crates build without replacing the shared renderer?
- Can the worker load only the bundled theme, font, and document JSON?
- What are the compressed payload, cold initialization, first compile, warm compile, PDF export, and peak-memory costs?
- Does it work in current Chrome, Firefox, Safari, and at least one midrange phone?

Use the decision thresholds in the implementation roadmap. Treat changing to the Axum fallback as a product decision, not as a renderer rewrite.

### Editor and storage

Build the Leptos editor around the proven worker protocol. Add persistence only after edit, preview, diagnostics, and export work without it. Implement a single stored document and reliable hydration before revisions and multiple documents.

### Hardening and release

Add broad browser coverage, accessibility checks, CI, stricter Clippy policy, fuzzing, property tests, and performance budgets after there is a useful product to protect. A test should pay for itself by guarding a risky boundary or a bug that has occurred.

## Core data and APIs

Prefer concrete types with domain names over tuples, maps of untyped values, or clever generic containers. Public model types should own their strings and lists so callers do not carry lifetimes through the parser, renderer, worker, and UI.

A suitable first public surface is conceptually:

```rust
pub fn analyze_markdown(source: &str, limits: &ParseLimits) -> Analysis;

pub struct Analysis {
    pub document: Option<RenderDocument>,
    pub diagnostics: Vec<Diagnostic>,
}

pub struct RenderDocument {
    pub metadata: DocumentMetadata,
    pub blocks: Vec<BlockNode>,
}

pub struct BlockNode {
    pub range: SourceRange,
    pub kind: Block,
}

#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    Heading { level: HeadingLevel, content: Vec<InlineNode> },
    Paragraph { content: Vec<InlineNode> },
    List { kind: ListKind, items: Vec<ListItem> },
    Divider,
}
```

The exact fields may evolve with the first fixture, but preserve these properties:

- Use `HeadingLevel`, `PaperSize`, `ThemeId`, `ListKind`, and `Severity` instead of loosely constrained integers, strings, and booleans.
- Use a plain `SourceRange { start, end }` with UTF-8 byte offsets matching `pulldown-cmark`.
- Put ranges on named node types instead of wrapping every value in a generic `Spanned<T>` if the generic makes errors and JSON harder to inspect.
- Use Serde's explicitly tagged, snake-case enum representation. The resulting `/resume.json` should be readable by a person debugging a theme.
- Use a private field plus a validating constructor for a type such as `LinkTarget` when constructing an invalid value would break a security invariant.
- Do not add newtypes solely to avoid primitives. `Revision(u32)` is useful because it prevents unrelated numbers from entering the worker protocol; wrapping every name and count is not.
- Prefer an options struct over boolean arguments. `render(document, RenderOptions { paper, theme })` is clearer than `render(document, true, false)`.

For the initial Markdown format, derive the document title from the first level-one heading. Keep paper, trusted theme, and language in project settings rather than inventing general YAML front matter. If a real metadata use case appears, add a deliberately small schema then.

Enable only the exact `pulldown-cmark` options that Resumark supports. Never use `Options::all()`: a dependency update could otherwise add source syntax without a product decision.

As of the research date, `pulldown-cmark` has an open `into_offset_iter()` panic involving C0 control characters. Prevalidate source before parsing, reject disallowed controls with a project diagnostic, and keep the minimized upstream case as one early regression test. This boundary matters especially if release WASM later uses `panic = "abort"`; fuzzing can still wait until hardening.

### Renderer API

The Typst adapter should expose a narrow API resembling:

```rust
pub struct Renderer { /* trusted world and cached assets */ }
pub struct CompiledDocument { /* private Typst PagedDocument */ }

impl Renderer {
    pub fn new(assets: BundledAssets) -> Result<Self, RendererInitError>;
    pub fn compile(
        &self,
        document: &RenderDocument,
        options: &RenderOptions,
    ) -> Result<CompiledDocument, RenderError>;
}

impl CompiledDocument {
    pub fn svg_pages(&self) -> Result<Vec<String>, ExportError>;
    pub fn pdf(&self, options: &PdfOptions) -> Result<Vec<u8>, ExportError>;
}
```

The actual Typst `PagedDocument` stays private. Both preview SVG and exported PDF come from the same compiled document. Do not compile again during export unless the requested settings or source revision changed.

The custom Typst `World` exposes only the main trusted theme, `/resume.json`, and bundled font files. It rejects all other paths and all package requests. User content is serialized with Serde; it is never escaped into generated Typst source.

Typst's current `World` trait requires `Send + Sync` and asks the implementer to cache source, file, and font loads. Use immutable owned assets and the smallest necessary synchronization inside the adapter. Do not put `Rc<RefCell<_>>` in the renderer just because the first browser build is single-threaded.

Avoid incremental compilation state in the first native slice. Add it only if warm measurements show a meaningful benefit and it can remain behind `Renderer`.

## Diagnostics and errors

Document problems and software failures are different and should look different in code.

`Diagnostic` represents something the resume author can correct:

```rust
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub range: Option<SourceRange>,
    pub help: Option<String>,
}
```

Examples include unsupported raw HTML, an unsafe link scheme, excessive nesting, missing required title information, or content overflow. Stable codes such as `unsafe_link_scheme` are more useful to the web UI and CLI than matching message strings.

An error type represents an operation that failed: invalid bundled font data, a virtual-world file failure, Typst compilation failure, IndexedDB failure, worker transport failure, or filesystem I/O. Use `thiserror` for meaningful library error enums and preserve underlying sources. Use `anyhow` only in application entry points where the next action is to add context, display the chain, and exit or update UI state.

Rules for fallible code:

- Do not use `panic!`, `unwrap`, or `expect` on user input or browser state.
- A bundled asset can still be corrupt; return an initialization error rather than hiding that assumption in `expect`.
- Tests may use `expect` with a message that states the fixture invariant.
- Use `?` for propagation and add context where a boundary would otherwise lose meaning.
- Document the `Errors` section of public fallible functions.
- Keep user-facing prose out of low-level variants when the CLI and web UI need different presentation.
- Do not use `String`, `JsValue`, `Box<dyn Error>`, or `()` as the public library error type.

The WASM release profile may eventually use `panic = "abort"`. A panicking compile then kills the worker; it cannot be recovered with `catch_unwind`. The editor's timeout and worker-restart path is the operational safety net, while native regression tests prevent known panics.

## Async and worker behavior

Parsing, validation, and Typst compilation stay synchronous. The browser worker is the concurrency boundary. This keeps `Future`, `Pin`, `Send`, and executor types out of the core APIs.

Use `gloo-worker` initially for a typed worker bridge and its existing message codec. Keep the protocol independent of that library so the bridge can be replaced with thin `wasm-bindgen` and transferable buffers if measurement shows serialization copies matter.

A first protocol should be explicit:

```rust
pub struct Revision(u32);

pub enum WorkerRequest {
    Compile {
        revision: Revision,
        source: String,
        settings: RenderSettings,
    },
    ExportPdf {
        revision: Revision,
    },
}

pub enum WorkerResponse {
    Compiled {
        revision: Revision,
        pages: Vec<String>,
        diagnostics: Vec<Diagnostic>,
    },
    PdfReady {
        revision: Revision,
        bytes: Vec<u8>,
    },
    InvalidDocument {
        revision: Revision,
        diagnostics: Vec<Diagnostic>,
    },
    Failed {
        revision: Revision,
        error: WorkerError,
    },
}
```

Use a `u32` revision because it crosses JavaScript safely and cannot realistically wrap during one editor session. The worker retains the last valid `CompiledDocument` and its revision. `ExportPdf` succeeds only for that exact revision, so the downloaded PDF matches the visible preview.

A synchronous Typst compile cannot process a cancellation message while it is running. Implement cancellation behavior in the editor-side controller:

1. Debounce source edits.
2. Send at most one compile while the worker is busy.
3. Replace the single pending request with the newest edit rather than queueing every revision.
4. Ignore a response that is older than the newest requested revision.
5. After a response, immediately send the one pending request, if present.
6. If the compile exceeds its time budget, terminate the worker, create a new one, and resend only the newest request.

Keep the last valid preview visible during compilation and document errors. Revoke superseded SVG Blob URLs only after the replacement preview is installed, and revoke all remaining URLs when the preview is destroyed.

Use `spawn_local` only at browser edges such as worker setup, IndexedDB calls, and downloads. Do not add Tokio to the web or core crates.

## UI and persistence state

Keep independent concerns in separate small enums instead of one combinatorial application state or several ambiguous booleans:

```rust
enum LoadState { Loading, Ready, Failed(LoadError) }
enum SaveState { Saved, Pending, Saving, Failed(SaveError) }
enum CompileState { Starting, Compiling(Revision), Ready, Failed }
```

Leptos signals and effects belong in `apps/web`. Pass plain owned values to storage and worker controllers. Avoid wrappers that make an ordinary signal update require understanding a project-specific reactive framework.

Use `indexed_db_futures` for the first persistence implementation because its future- and Serde-based API is a reasonable fit for a single-threaded browser application. Contain its transaction and JavaScript types inside `IndexedDbDocumentStore`; UI components should see project-owned records and `StorageError`.

Prefer concrete inherent async methods such as `open`, `load_active_document`, `save_document`, and `append_revision`. Do not add a `DocumentRepository` async trait until another implementation is actually used.

Persistence rules:

- Finish database hydration before constructing or saving a default document.
- Perform each logically atomic change in one read-write transaction.
- Do not await unrelated work while an IndexedDB transaction is open; transaction lifetime is tied to the browser event loop.
- Handle blocked upgrades and `versionchange` by closing the old connection and showing a reload message.
- Autosave after the configured idle period; do not rely on asynchronous work from an unload handler.
- Report save failure without discarding the in-memory source.
- Keep Markdown and versioned project-file export available because browser persistence is not the only copy a user should trust.
- Add schema migrations when the second schema exists, not before. Test each migration from a real prior fixture.

Start with a fixed single-document record if that keeps the first persistence slice smaller. Introduce document identifiers, indexes, revision compaction, and multi-document queries together when multiple documents are built.

## Naming and code shape

Choose names that let a reader predict behavior without opening the implementation:

- Prefer `analyze_markdown`, `validate_link`, `compile`, `export_pdf`, `load_document`, and `save_document`.
- Avoid `process`, `handle`, `do_work`, `data`, `utils`, and `manager` unless those words describe a precise domain concept.
- Follow Rust's `new`, `from_`, `as_`, `to_`, and `into_` conventions. Getters do not need a `get_` prefix.
- Spell out project concepts. Reserve short names for tiny closures and conventional iterator variables.
- Prefer exhaustive `match` expressions for domain and application states.

Optimize for local comprehension:

- Keep a function near one screen when practical. Extract a named step when nesting or multiple responsibilities obscure the main path.
- Prefer early returns for invalid boundary conditions.
- Prefer concrete parameter and return types. Introduce a trait when callers need interchangeable behavior, not only to mock a function.
- Avoid public lifetime parameters by owning model data. Borrow inputs to avoid unnecessary copies at well-defined calls.
- A clear `clone` at a worker or UI ownership boundary is acceptable. Remove it only after profiling shows it matters.
- Avoid type aliases that merely hide a difficult generic type. Name a domain struct that owns the behavior instead.
- Avoid macros beyond conventional derives unless the expansion removes substantial repeated code.
- Avoid glob imports outside a test module or a dependency's documented prelude.
- Keep target-specific `cfg` branches in small platform modules rather than scattering them through domain code.
- Use comments to explain an invariant, safety boundary, or surprising dependency behavior—not to narrate Rust syntax.

Each library crate should have short crate-level `//!` documentation containing its responsibility, exclusions, dependency direction, and one end-to-end example using `?`. Public types should explain why they exist; public fallible methods should explain failure conditions. Keep implementation details private rather than documenting a large accidental API.

## Toolchain and dependencies

At workspace creation:

- Begin with Rust 1.98.0, the current stable release on the research date. Commit a `rust-toolchain.toml` containing that exact toolchain plus `rustfmt`, `clippy`, and `wasm32-unknown-unknown`; recheck the stable release before scaffolding if implementation begins later.
- Use Rust edition 2024 and Cargo resolver 3 for every workspace package.
- Commit `Cargo.lock`; this is an application workspace with reproducible builds.
- Centralize shared dependency versions in the root workspace manifest.
- Use `#![forbid(unsafe_code)]` in project-owned crates. Revisit only if a measured need has no safe implementation.
- Run `cargo fmt`, `cargo check --workspace`, and the current vertical-slice smoke command while building.

Initial dependency choices:

| Need | Choice | Notes |
| --- | --- | --- |
| Markdown events and byte ranges | `pulldown-cmark` 0.13 line | Enable only supported options. |
| Serialization | `serde`, `serde_json` | JSON is the trusted theme input and project interchange base. |
| URL parsing | `url` | Still enforce the explicit `http`, `https`, `mailto`, and `tel` allowlist. |
| Typed library errors | `thiserror` 2 | Add when an operation has multiple meaningful failures. |
| Application error context | `anyhow` 1 | CLI and top-level application boundaries only. |
| CLI parsing | `clap` 4 derive | Keep command definitions near their execution code. |
| Document compilation | matching `typst`, `typst-pdf`, and `typst-svg` versions | Pin exact versions because the Rust API is pre-1.0. |
| Web UI | Leptos 0.8 CSR with Trunk | Prefer the current stable line over the 0.9 beta; keep the framework-specific layer thin. |
| Worker bridge | `gloo-worker` 0.6 line | Start readable; replace the bridge only if measured copying/build friction justifies it. |
| Browser persistence | `indexed_db_futures` 0.6 line | Keep its non-obvious transaction behavior behind one concrete type. |

As of 2026-08-30, stable Leptos is 0.8.20, the current Typst Rust documentation is 0.15.1, `pulldown-cmark` is 0.13.4, `gloo-worker` is 0.6.0, and `indexed_db_futures` is 0.6.4. Recheck current releases and WASM compatibility when the workspace is created. Pin all three Typst crates to the same exact version.

Do not add `async-trait`, Tokio, an ORM-like storage layer, a dependency-injection framework, a logging facade, a snapshot framework, property testing, or fuzz infrastructure until implemented behavior demonstrates the need.

When the product reaches the hardening stage, add a small shared lint policy instead of enabling every Clippy pedantic or restriction lint. Good candidates are production uses of `unwrap`, `panic`, `todo`, `unimplemented`, and `dbg`, plus warnings for overly long functions. Allow a lint locally with a reason when the result is clearer than the lint's suggestion. Then automate formatting, checks, tests, the WASM build, browser smoke coverage, PDF checks, and bundle budgets in CI.

## Readability checklist

Before considering a chunk complete, ask:

- [ ] Can a reader identify the input, output, and owner of the new behavior from names alone?
- [ ] Did Typst, Leptos, browser, CLI, and storage types stay on their side of the workspace boundary?
- [ ] Is the public API concrete and small, without an unnecessary trait, generic, lifetime, macro, or newtype?
- [ ] Are invalid domain values rejected at a clear boundary?
- [ ] Are user-correctable diagnostics separate from operational errors?
- [ ] Can every production `unwrap`, `expect`, or `panic` be removed?
- [ ] Does async code exist only where an operation is actually asynchronous?
- [ ] Does a worker response include its revision, and can stale work be ignored safely?
- [ ] Are source, preview, and PDF derived from the same accepted revision?
- [ ] Does the main function or component still fit on one screen and read from high-level step to high-level step?
- [ ] Does each comment explain why a surprising choice exists?
- [ ] Is the smoke check proportional to the risky boundary introduced in this chunk?

## Primary references

Checked on 2026-08-30:

- [Latest stable Rust release](https://blog.rust-lang.org/releases/latest/), [Cargo resolver versions](https://doc.rust-lang.org/cargo/reference/resolver.html), and [rustup toolchain files](https://rust-lang.github.io/rustup/overrides.html) — the pinned Rust 1.98, edition 2024, resolver 3, and target/component setup.
- [Rust API Guidelines checklist](https://rust-lang.github.io/api-guidelines/checklist.html) — naming, common traits, crate documentation, examples, error documentation, and predictable APIs.
- [Rust API Guidelines: interoperability](https://rust-lang.github.io/api-guidelines/interoperability.html) — meaningful error types and `Send`/`Sync` expectations.
- [Clippy usage guidance](https://doc.rust-lang.org/stable/clippy/usage.html) and [lint configuration](https://doc.rust-lang.org/stable/clippy/lint_configuration.html) — why to select restriction lints rather than enable the entire group.
- [`pulldown-cmark::OffsetIter`](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.OffsetIter.html) and [`Options`](https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html) — source byte ranges and opt-in Markdown extensions.
- [`pulldown-cmark` control-character panic report](https://github.com/pulldown-cmark/pulldown-cmark/issues/1129) — the input prevalidation and targeted regression requirement.
- [`typst::World`](https://docs.rs/typst/latest/typst/trait.World.html) — the `Send + Sync` contract and caching responsibilities for embedded compilation.
- [`typst-pdf`](https://docs.rs/typst-pdf/latest/typst_pdf/) and [`typst-svg`](https://docs.rs/typst-svg/latest/typst_svg/) — exports from a compiled paged document.
- [Typst PDF reference](https://typst.app/docs/reference/pdf/) and [accessibility guide](https://typst.app/docs/guides/accessibility/) — tagged-PDF defaults, PDF/UA-1 checks, and the limits of automated accessibility validation.
- [`wasm-bindgen` Web Worker example](https://rustwasm.github.io/docs/wasm-bindgen/examples/wasm-in-web-worker.html) — worker loading and ownership boundaries.
- [`gloo-worker`](https://docs.rs/gloo-worker/latest/gloo_worker/) — typed worker bridges and documented serialization overhead.
- [`indexed_db_futures`](https://docs.rs/indexed_db_futures/latest/indexed_db_futures/) — future-based IndexedDB operations and transaction behavior.
- [MDN: Using IndexedDB](https://developer.mozilla.org/en-US/docs/Web/API/IndexedDB_API/Using_IndexedDB) — version changes, event-loop-bound transaction lifetime, shutdown behavior, and atomic writes.
- [Leptos 0.8 documentation](https://docs.rs/crate/leptos/0.8.20) and [maintainer status](https://github.com/leptos-rs/leptos/issues/4707) — the stable v1 choice and the reason to keep the UI boundary replaceable.
