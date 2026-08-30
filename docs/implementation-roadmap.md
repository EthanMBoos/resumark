# Build order

The target workflow is:

```text
Edit resume.md elsewhere -> web app or CLI -> preview themes -> download PDF
```

## Done

### 1. Native compiler

- Cargo workspace, CLI, Typst renderer, theme, and bundled fonts
- PDF and SVG output from one compile
- Letter and A4 output

### 2. Markdown model

- Owned document model with source ranges
- Supported Markdown blocks and inline content
- Errors for raw HTML, unsafe links, bad controls, size, and nesting
- `resume inspect` output

### 3. Complete renderer

- Self-contained Typst input
- PDF and SVG pagination parity
- Page-count warnings
- User content passed as data rather than Typst source

### 4. Browser compiler

- Parser and Typst renderer running in a Web Worker
- Worker-ready handshake
- SVG preview and PDF download
- Chromium Playwright check

## Next: web conversion flow

Replace the compiler demo with the first usable web app.

Build:

- Explain that the local Markdown file remains the source of truth
- Drop zone and file picker for one `.md` file
- A button to try `examples/resume.md`
- Filename, preview, and replace-file action
- Reopen the same file after it changes externally
- Letter/A4 selector
- Source errors with line and column
- No stale preview or PDF when a file is rejected
- PDF download named from the source file
- Blob URL cleanup after file or setting changes
- Wide and narrow layouts

Check:

- Choose a file and render it.
- Choose invalid Markdown and see its source error with no PDF available.
- Correct the local file, reopen it, and render again.
- Reopen the same filename and confirm the new contents are used.
- Switch paper size.
- Download a valid PDF with the expected filename.
- Run Playwright and inspect desktop and narrow screenshots.

Keep direct `web-sys` unless the page becomes harder to understand than a small
framework version.

## Then: themes

Add one second theme that looks meaningfully different from `minimal.typ`.

- Add the smallest theme identifier needed by the renderer and worker message.
- Show theme choices with useful names and previews.
- Re-render when the theme changes.
- Confirm the chosen theme matches in SVG and PDF.
- Add CLI theme selection through the same renderer API.

Keep themes bundled and project-owned. Do not build a plugin system, theme
language, marketplace, or user Typst support.

## V1 finish

- Use the web and CLI flows with real Markdown resumes.
- Fix layout, errors, keyboard use, and small-screen problems found in use.
- Check release builds in the browsers named in the release notes.
- Add hosting, cache headers, CSP, font licenses, and a short privacy statement.
