# Product

Resumark turns a user-owned Markdown resume into a styled PDF.

The intended habit is:

```text
Keep resume.md locally
  -> edit it in any Markdown editor
  -> open it in the Resumark web app or CLI
  -> choose a theme and paper size
  -> download the PDF
```

Resumark is the renderer, not the editor. It does not modify or store the
Markdown file. If the preview reveals a typo, fix the local file and open it
again.

## V1

- Choose or drop one `.md` file
- Try the Jane Doe example
- Preview Letter or A4 pages
- Choose between at least two bundled themes
- Show source-based errors
- Download the PDF
- Replace or reopen the Markdown file after an external change
- Run the same compiler from the CLI

V1 has no text editor, server, accounts, saved documents, sync, analytics,
rich-text UI, or user-written Typst.

## Document pipeline

```text
Markdown
  -> parser and validation
  -> owned resume model
  -> Typst theme and bundled fonts
  -> SVG pages and PDF
```

Markdown is passed to Typst as data. The renderer uses only project-owned
themes and fonts. The SVG preview and PDF come from the same compiled document.

Typst runs in a Web Worker so compilation does not block the page. The current
worker is about 29.9 MB raw and 10 MB compressed. Startup plus the first render
was about 220 ms on the development Mac. Optimize only if normal use exposes a
problem.

## Markdown

Supported now:

- Headings and paragraphs
- Ordered and unordered lists
- Emphasis and strong text
- Links and inline code
- Dividers

Raw HTML and unsafe link schemes are rejected. Images, tables, front matter,
and raw Typst are outside V1.

## Privacy and safety

- Keep resume content in the browser.
- Use no third-party scripts or fonts.
- Reject raw HTML and unsafe links.
- Limit source size and nesting.
- Keep arbitrary Typst code out of user files.
- Add a content security policy before publishing.

Do not claim universal applicant-tracking compatibility or PDF accessibility
conformance without testing the generated files.
