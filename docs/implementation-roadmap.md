# Resumark implementation roadmap

This document turns the product direction in [project-plan.md](project-plan.md) into a practical build order for a side project. The goal is to reach a useful local-first version quickly without hiding the project's largest risk: whether Typst can compile resumes reliably in a browser worker at an acceptable size and speed.

Each stage should leave behind something visible and usable. Verification stays focused on the behavior introduced in that stage. A broader test matrix, CI, accessibility validation, and release hardening come only after the complete local workflow exists.

## Target for the first usable version

The first usable local version lets one person:

- Write a resume in a native Markdown textarea.
- See fixed-page previews produced by Typst.
- Choose Letter or A4 paper and a bundled theme.
- Export the same compiled document as a selectable-text PDF.
- See useful source diagnostics while keeping the last valid preview visible.
- Reload the browser without losing work.
- Import and export the Markdown source so the browser is never the only copy.

It runs as a static application, performs compilation in a Web Worker, and makes no network requests after its assets are loaded. Multiple resumes, revision history, the polished CLI, offline installation, and stronger PDF accessibility checks can follow once this loop is dependable.

## Build order at a glance

```text
Native Markdown-to-PDF/SVG slice
  -> Trustworthy core and renderer
    -> Browser/WASM feasibility gate
        -> Single-document editor
          -> Local persistence and source portability
            -> First usable local v1
              -> Optional local product depth
                -> Hardening, CLI, and release
```

The native compiler proves the document pipeline with ordinary Rust tooling. The browser feasibility stage comes before a substantial Leptos application or storage layer so that a failed WASM experiment does not strand unrelated UI work.

## Stage 0: Build the native happy path

**Status:** Implemented. The fixture builds into a two-page Letter PDF and two
SVG previews through the project-owned model and restricted Typst adapter.

### Visible outcome

Running one command turns a realistic Markdown resume into a PDF and one SVG per page. The first slice is deliberately narrow, but it exercises the real parser, typed model, trusted Typst theme, and export path rather than placeholder scaffolding.

### Work

- Create a Cargo workspace with only:
  - `crates/resumark-core` for the model, Markdown parsing, validation, and diagnostics
  - `crates/resumark-render-typst` for the Typst adapter
  - `apps/cli` for the native entry point
- Add `apps/web` only when the WASM stage begins. Add a separate storage crate only if the persistence implementation becomes large enough to justify it.
- Pin the Rust toolchain and the exact Typst crate family version. Keep shared dependency versions and release profile settings in the workspace manifest.
- Add one realistic Markdown resume fixture. It should be long enough to exercise two pages and include headings, paragraphs, nested lists, emphasis, links, a divider, and non-ASCII text.
- Define the smallest owned `RenderDocument` that can represent the fixture and convert the required `pulldown-cmark` events into it. Unsupported events may return a clear diagnostic until Stage 1.
- Implement the restricted in-memory Typst world with `/resume.json`, one trusted theme, and one appropriately licensed bundled font family.
- Serialize the document as data; never interpolate user text into Typst source.
- Compile once and export both the default Letter PDF and page SVGs from that result.
- Give the CLI a small `build` command that reads the fixture and writes those artifacts. It is a development harness, not the complete command-line product.

### Minimal verification

- Run the CLI against the fixture and open the generated PDF and SVG pages.
- Extract PDF text with a local tool and confirm the name, one heading, and one list item appear in reading order.

### Completion gate

The realistic fixture produces a selectable-text PDF and matching SVG pages through real project-owned types. Typst-specific code is contained in one adapter crate.

### Defer

- Complete Markdown coverage, rich diagnostics, A4, overflow policy, and performance work.
- Leptos, Trunk, IndexedDB, Web Workers, CI, fuzzing, and release packaging.
- Extra themes, fonts, commands, and sample documents.

## Stage 1: Make the core trustworthy and readable

**Status:** Implemented. The core now produces a source-ranged owned model and
ordered diagnostics, enforces the v1 Markdown and link policy, applies named
size and nesting limits, and protects the parser from disallowed control input.
The native harness can print the formatted model with `resume inspect`.

### Visible outcome

The CLI handles the complete v1 Markdown policy and prints source-oriented diagnostics instead of dependency errors or panics. Its formatted JSON remains easy to inspect when developing themes.

### Work

- In `resumark-core`, finish the small, owned data types for `RenderDocument`, document settings, blocks, and inline content. Prefer ordinary enums, `String`, and `Vec` over lifetime-heavy or generic public types.
- Return an `Analysis` containing an optional render document plus ordered diagnostics. Put UTF-8 byte ranges on named block and inline nodes so source locations stay explicit without a generic span abstraction or parallel-map bookkeeping.
- Keep Markdown parsing in a focused `markdown` module within `resumark-core`; convert `pulldown-cmark` offset events into the project-owned model without exposing parser events publicly.
- Support only the initial policy: headings, paragraphs, ordered and unordered lists, emphasis, strong text, links, inline code, and dividers.
- Detect raw HTML instead of passing it through. Validate link schemes and allow only `https`, `http`, `mailto`, and `tel`.
- Add explicit document-size and nesting-depth limits. Keep the values named in one options type so they can be revised after real usage.
- Expose one obvious parser entry point. For example, `analyze_markdown(source, limits)` should return the document and ordered diagnostics without requiring callers to understand parser internals.
- Make unsupported constructs and malformed input produce plain-language diagnostics with source ranges.

### Minimal verification

- Parse the realistic fixture and inspect its formatted JSON.
- Keep small syntax examples nearby for manual inspection while the model is changing.
- Add regression tests only for boundaries where a failure would be dangerous or difficult to notice: raw HTML, unsafe link schemes, input limits, and the known control-character parser panic.

### Completion gate

The realistic fixture has a stable, human-reviewable typed representation; invalid input reports where the problem occurred; and neither the CLI nor later renderers need to know about `pulldown-cmark` events.

### Defer

- Front matter or a custom metadata language. For v1, keep paper, theme, and other settings separate from Markdown.
- Property tests and fuzzing.
- Resume-specific scoring, profiles, or a more semantic career schema.
- Images, tables, raw HTML, and arbitrary embedded Typst.

## Stage 2: Finish the native rendering boundary

### Visible outcome

The native compiler supports the complete v1 settings and containment rules: Letter/A4, clear overflow feedback, matching preview/PDF output, and no access beyond bundled assets.

### Work

- Complete the restricted in-memory Typst `World` in `resumark-render-typst`.
- Expose only `/resume.json`, one trusted theme, and a minimal set of appropriately licensed bundled fonts. Do not expose the host filesystem, package lookup, or network access.
- Serialize `RenderDocument` with Serde and let the trusted theme read it as data. Never interpolate user text into Typst source.
- Refine the plain single-column theme for predictable reading order. Add theme settings only as they become necessary to complete the fixture.
- Keep all direct Typst APIs private to the rendering crate. Present the rest of the workspace with a small façade such as a renderer that accepts a document and render options and returns an opaque compiled result.
- Preserve the Stage 0 rule that PDF bytes and page SVGs come from the same Typst paged document.
- Add Letter and A4 page selection.
- Detect content that creates more pages than requested or otherwise exceeds the chosen v1 page policy, and return a clear warning without discarding valid output.
- Add explicit output-directory and paper-size arguments to the temporary CLI harness.

### Minimal verification

- Build the realistic fixture for Letter and A4.
- Open the generated PDF and SVG pages and visually compare their pagination.
- Extract PDF text with a local command-line tool and confirm that the name, section headings, list items, and links appear in the intended reading order.
- Confirm that a theme request cannot read an arbitrary local file.

### Completion gate

The same native compile result produces matching page previews and a selectable-text PDF, the realistic fixture reads in the expected order, and all Typst-specific complexity is contained inside the rendering crate.

### Defer

- Multiple themes and broad golden-file coverage.
- PDF/UA conformance claims and exhaustive accessibility checks.
- Incremental compilation tuning.
- A polished CLI interface.

## Stage 3: Prove the browser compiler before building the product

### Visible outcome

A minimal local browser page sends the realistic document to a dedicated Web Worker, receives SVG pages, displays them, and downloads a PDF. It also records enough timing and bundle information to choose the static architecture with evidence.

### Work

- Add the smallest possible `apps/web` harness and configure Trunk for a client-rendered WASM build. This is an experiment screen, not the editor design.
- Compile the existing model, parser, and renderer for `wasm32-unknown-unknown`; do not create a second rendering implementation.
- Load the Typst compiler, trusted theme, and bundled fonts in a dedicated worker so typesetting never blocks the UI thread.
- Define explicit serializable worker messages. Requests carry a monotonically increasing revision, Markdown or parsed document input, and settings. Responses carry the revision, SVG pages or PDF bytes, diagnostics, and useful timing values.
- Discard responses older than the most recently requested revision.
- Add a time budget for compilation. When a worker exceeds it, terminate and recreate the worker and keep the last successful result available.
- Display SVGs through Blob URLs and revoke superseded URLs.
- Measure production-build assets using one named compression format, preferably Brotli: editor shell, compiler WASM, fonts, and total initial/local payload.
- Measure cold initialization, first preview, warm preview, PDF export, and approximate peak memory on one named midrange phone as well as a development desktop.
- Write down the result as a short architecture decision in this document or a linked note.

### Minimal verification

- Edit the fixture rapidly and confirm that an older worker response never replaces a newer result.
- Trigger a deliberate timeout and confirm that a replacement worker can render again without reloading the page.
- Reload once after assets have been cached and confirm that compilation performs no document-data network request.
- Record actual size and timing numbers from a production build.

### Completion gate

Choose one path explicitly:

- **Static path:** Continue when the worker is reliable and the measured experience is acceptable. The project plan's provisional warning thresholds are 8–10 MB Brotli-compressed for compiler plus fonts and roughly three seconds to first preview on the chosen midrange phone. These are decision aids, not promises.
- **Server fallback:** If the worker is too large, slow, or unstable, keep the shared crates and plan a small Axum compilation service accepting typed document JSON. Make that choice before building persistence or a polished editor.

### Defer

- Full editor layout and interaction design.
- IndexedDB and autosave.
- Service workers and offline installation.
- Cross-browser matrices and performance automation.

## Stage 4: Build the single-document editing loop

### Visible outcome

The application is pleasant enough to use for a real editing session: typing Markdown updates fixed-page previews, diagnostics point back to the source, and the current document exports as PDF.

### Work

- Replace the experiment screen with a small Leptos CSR application.
- Use a native `<textarea>` with clear labels and sensible keyboard, spellcheck, clipboard, IME, and mobile behavior. Do not add syntax highlighting.
- Build a responsive two-pane layout that becomes a simple editor/preview switcher on narrow screens.
- Debounce compilation around 150–250 milliseconds and keep worker revision handling outside presentation components.
- Keep the most recent valid compiled document in the worker or application state. Invalid edits update diagnostics but do not blank the preview or destroy the last exportable result.
- Show page SVGs with clear page boundaries, page count, compilation state, and source diagnostics.
- Add Letter/A4 and one theme control. If a second theme is added, it must use the same document JSON rather than special parser behavior.
- Download PDF bytes produced from the same latest valid compiled document.
- Keep browser and Typst adapters behind small modules; Leptos components should mostly consume plain state and callbacks.

### Minimal verification

- Manually exercise a full edit-preview-export session with the realistic fixture.
- Introduce invalid HTML and an unsafe link, confirm the diagnostic location, then correct them and confirm rendering recovers.
- Type rapidly enough to overlap requests and confirm the preview ends on the newest source.
- Check one desktop and one narrow/mobile layout, including keyboard focus and control labels.

### Completion gate

A resume can be written and exported without UI freezes; preview and PDF share pagination; invalid source never replaces the last valid preview; and the native textarea remains usable with keyboard and mobile input.

### Defer

- Multiple named documents and revision history.
- Find/replace, synchronized scrolling, formatting shortcuts, and syntax highlighting.
- Page-break controls and advanced layout warnings.
- Detailed browser automation.

## Stage 5: Make local work durable and portable

### Visible outcome

Refreshing or reopening the application restores the resume, while import and export ensure the user can always keep an independent copy.

### Work

- Add an explicit application hydration state: opening database, ready, saving, and save error. Do not create or save a default document until the database load has completed.
- Store the current document and settings in IndexedDB behind a small project-owned interface. Keep browser-specific request/event types out of Leptos components.
- Save after roughly 400 milliseconds of inactivity and display honest saving, saved, and error states.
- Give stored records and exported project files an explicit schema version from their first implementation.
- Add Markdown import and download.
- Add a project-file download that includes Markdown and settings, plus matching import with clear validation errors.
- Snapshot the current content before destructive in-app operations such as importing over it or resetting to the sample.
- Prefer a simple single-document store for this stage. The data shape may allow IDs, but do not build document management until the core recovery path is trustworthy.

### Minimal verification

- Edit, wait for saved status, reload, and confirm the exact source and settings return.
- Reload immediately during an edit and confirm behavior is understandable even if the last debounce has not completed.
- Simulate a storage failure and confirm the application remains usable and does not claim the content was saved.
- Export Markdown and a project file, clear the browser data manually, then restore from each format.
- Confirm first load does not overwrite an existing record with sample content.

### Completion gate

A normal reload restores work, save failures are visible, settings round-trip with source, and a user can recover the resume without relying on one browser profile. Reaching this gate is the first usable local v1; stop here and use it for real resume work before deciding which later features matter.

### Defer

- Multiple resumes, revision browsing, automatic compaction, and migrations beyond the initial schema reader.
- Cloud sync, accounts, and collaboration.
- Storage abstractions intended for hypothetical non-browser backends.

## Stage 6: Add local product depth if real use justifies it

### Visible outcome

The application grows from a dependable single-resume tool into a small local document manager, but only after using the Stage 5 version reveals that these features are worthwhile.

### Work

- Add two or three named local documents with create, rename, switch, duplicate, and delete flows. Require a clear confirmation before deletion and offer a source download where practical.
- Add lightweight recoverable revisions: periodic snapshots plus snapshots before import, reset, and document deletion. Set a small named retention limit rather than designing indefinite history.
- Add restore UI that shows timestamp and document name and snapshots the current state before restoring an older version.
- Add a second bundled theme only if it demonstrates that the model/theme boundary works without adding theme-specific Markdown rules.
- Add explicit page-count and overflow feedback. Add an orphan-heading warning only if it can be derived reliably from the compiled page result.
- Improve empty, loading, compiling, saving, and error states without adding a component framework.
- Confirm production behavior with local fonts, no third-party scripts, no analytics, and a restrictive content security policy compatible with the worker and Blob previews.
- Update user-facing text to make narrow, testable claims: local processing, selectable text, link preservation, and preview/export parity. Do not advertise a universal ATS score or accessibility conformance that has not been validated.

### Minimal verification

- Use the application for a complete manual workflow: create, edit, reload, switch documents, restore a revision, export source, export PDF, and import into a clean browser profile.
- Check that deleting or replacing content has a recovery path.
- Test the production build offline after its static assets have loaded once, even if installable offline caching is still deferred.
- Extract text from PDFs made with both paper sizes and every included theme.

### Completion gate

The local application is useful for ongoing resume work, protects against common accidental loss, produces readable exports from all bundled themes, and does not depend on a backend or third-party runtime resources.

### Defer

- Master career inventories, tagged profiles, profile diffs, and JSON Resume.
- Images, multi-column templates, user-authored Typst, and third-party theme packages.
- Shareable hosting, accounts, and sync.
- AI writing tools and a custom rich-text editor.

## Stage 7: Harden, automate, and prepare a public release

### Visible outcome

The static application and native CLI are reproducibly buildable, tested on supported platforms, and ready to publish with measured claims.

### Work

- Turn the temporary native harness into the documented CLI commands that are already supported by the shared core: `build`, `check`, and JSON export. Add `preview` only after defining whether it writes files or opens a local viewer.
- Add broad core tests: parser construct coverage, source-offset cases, nested Markdown property tests, URL validation, and focused fuzz targets.
- Add rendering fixtures for themes, Letter/A4, overflow, Unicode, right-to-left text where fonts support it, links, and multi-page reading order.
- Normalize unstable metadata before comparing render artifacts. Prefer semantic assertions such as page count, extracted text, link targets, and selected SVG structure over large unexplained snapshots.
- Validate tagged/PDF/UA output with the pinned Typst version and veraPDF before making accessibility claims. Treat failures as product findings, not test exceptions.
- Add browser automation for hydration, save/reload, stale worker responses, timeout recovery, import/export, document switching, keyboard interaction, and representative mobile layouts. Playwright is acceptable test-only tooling.
- Add smoke coverage for current Chrome, Firefox, and Safari, with explicit supported-version notes.
- Add release measurements for Brotli asset size, cold and warm rendering latency, PDF export latency, and memory on the named mobile target.
- Add CI only now that the useful workflow is known: formatting, Clippy, native tests, WASM build, browser smoke tests, asset budgets, and PDF checks. Keep slow fuzzing or extended browser runs on a scheduled/manual job if necessary.
- Add the plain static marketing page, production hosting configuration, caching headers, CSP, and optional offline service worker. Keep the landing page independent of editor/compiler assets.
- Document bundled font and theme licenses and produce reproducible release artifacts.

### Minimal verification

- Run the complete local release command from a clean checkout.
- Run the full automated suite and inspect its generated PDFs and browser screenshots once before release.
- Test the deployed production build from a fresh profile, then repeat the main workflow without a network connection where offline support is promised.
- Compare measured sizes and latencies to the Stage 3 baseline and document meaningful regressions.

### Completion gate

The published claims are backed by repeatable checks, the supported browser workflow passes end to end, the CLI uses the same model and renderer as the browser, and another person can build and run the project from its documentation.

### Defer

- Any feature that does not strengthen the core Markdown-to-preview-to-PDF workflow.
- Larger profile-generation and import/export ecosystems until real users demonstrate demand.

## Rust readability rules for every stage

Readability is a product constraint because this project is also meant to make Rust easier to follow.

- Keep boundaries directional inside and across crates: the core model module knows nothing about parsing or rendering; the core Markdown module depends on that model; the renderer depends only on the core's public types; applications assemble them.
- Give each crate one small public façade and keep implementation modules private by default.
- Prefer owned values and explicit structs/enums over clever borrowing, type-level machinery, or generic public APIs.
- Prefer named intermediate variables, ordinary `for` loops, and small helper functions when iterator chains become difficult to scan.
- Avoid a trait until it represents a real boundary, such as storage across browser and test implementations, or until a second implementation exists.
- Use explicit imports rather than a custom prelude that hides where types originate.
- Use `thiserror` for library error enums and `anyhow` for application-level context in CLI code. Errors shown to users should be converted into project diagnostics rather than exposing dependency messages.
- Keep asynchronous code at browser and worker boundaries. Parsing and model transformations should remain synchronous and deterministic.
- Put worker request and response types in a small shared module; do not send internal Typst objects across the boundary.
- Write comments for invariants, security boundaries, and surprising tradeoffs. Do not narrate obvious syntax.
- Add a short example to each important public entry point and keep the realistic fixture usable as an end-to-end example.
- Run `cargo fmt` routinely. Use normal Clippy warnings and a few selected lints; do not enable the entire pedantic lint group if it makes straightforward code noisier.

## Decisions to record when they become relevant

Do not block the first stage on every future choice. Resolve each item immediately before its dependent work:

- Before Stage 1: exact size and nesting defaults, and how source ranges count UTF-8 offsets.
- Before Stage 2: Typst version, initial font license/coverage, default paper size, and desired page-count policy.
- Before Stage 3: supported browser baseline, production compression measurement, named mobile benchmark device, and worker timeout.
- Before Stage 5: project-file format, schema version field, autosave semantics, and behavior when IndexedDB is unavailable.
- Before Stage 6: revision interval and retention limit, deletion recovery behavior, and whether a second theme is worth supporting.
- Before Stage 7: hosting target, offline promise, public browser support, accessibility claim, and release asset/performance budgets.

The central rule is simple: keep moving toward a real local resume export, but stop and decide when an unresolved choice would change the architecture or risk losing user work.
