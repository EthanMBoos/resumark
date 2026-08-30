# Rust structure

Keep the Rust readable to someone who does not use Rust every day. Prefer
normal data types, named steps, and a small number of modules.

## Workspace

```text
apps/cli                    Native build and inspect commands
apps/web                    Browser page, worker, and shared messages
crates/resumark-core        Model, Markdown parser, and diagnostics
crates/resumark-render-typst Typst compile, SVG, and PDF
examples/resume.md          Jane Doe example
themes/minimal.typ          Current theme
fonts/libertinus            Bundled font files
```

Dependency direction:

```text
cli -> core
cli -> renderer -> core
web page -> web messages -> core
web worker -> web messages and renderer -> core
```

## Boundaries

- Core has no Typst, browser, filesystem, or CLI types.
- The renderer owns all direct Typst use.
- The worker parses and renders.
- The page owns file input, DOM state, settings, and Blob URLs.
- The CLI owns native file I/O and terminal output.
- User Markdown is always data, never generated Typst source.

Keep helpers beside the code that uses them. Skip `utils`, `services`, manager
objects, repository layers, and general extension points.

## Core API

```rust
pub fn analyze_markdown(source: &str, limits: &ParseLimits) -> Analysis;
```

`Analysis` contains ordered diagnostics and an optional `RenderDocument`. The
document uses owned `String`, `Vec`, structs, and enums. Source ranges are
half-open UTF-8 byte offsets from the original Markdown.

Enable only supported Markdown features. Check control characters before
calling `pulldown-cmark`; its offset iterator has a known failure for some C0
controls.

Use diagnostics for source problems. Use normal errors for broken bundled
assets, compilation, PDF export, file I/O, worker messages, and browser APIs.

## Renderer API

```rust
let renderer = Renderer::new()?;
let compiled = renderer.compile(&document, &options)?;
let pages = compiled.svg_pages();
let pdf = compiled.pdf()?;
```

`CompiledDocument` keeps the Typst paged document private. SVG and PDF must be
exported from that same object. Typst receives the resume data, settings,
bundled theme, and fonts as one bundle.

Keep the Typst crate versions pinned together because their Rust APIs are
pre-1.0.

## Browser worker

The page waits for `RenderResponse::Ready` before sending its first request.
This prevents a Trunk worker-loader race.

For each selected file or changed render setting:

1. Send the Markdown and settings to the worker.
2. Show source diagnostics when the request is rejected.
3. Clear any preview and PDF that belonged to a different source.
4. Install successful SVG and PDF Blob URLs.
5. Revoke the URLs they replaced.

The browser session is temporary. Do not add an editor, persistent document
state, request revisions, cancellation, or worker restart logic without a
reproduced need.

## Style

- Keep functions short and the main path easy to scan.
- Use named variables and early returns.
- Use a `for` loop when it reads better than an iterator chain.
- Use an explicit clone at a browser ownership boundary.
- Add a trait only for two real implementations or a required boundary.
- Use `thiserror` in libraries and `anyhow` for CLI context.
- Avoid `panic!`, `unwrap`, and `expect` on user input or browser state.
- Comment surprising rules and invariants, not syntax.
- Add dependencies only for current code.

## Checks

```sh
cargo fmt --all -- --check
cargo test --workspace --locked --offline
cargo clippy --workspace --all-targets --locked --offline -- -D warnings
```

For browser work, run the Playwright process in
[`AGENTS.md`](../AGENTS.md).

Before finishing a change, check that preview and PDF share one compile, unsafe
input is rejected, stale output is cleared, browser and Typst types stayed out
of core, and every new layer or dependency has a current use.
