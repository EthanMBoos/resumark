# Jane Doe

[jane.doe@example.com](mailto:jane.doe@example.com) · [janedoe.dev](https://janedoe.dev) · New York, NY · +1 212 555 0142

Product-minded software engineer with eight years of experience building dependable tools for people who work with complex information. I care about **clear interfaces**, measurable outcomes, and systems that remain understandable after the first release.

## Experience

### Staff Software Engineer — Northstar Tools

*January 2022–Present · New York, NY*

- Led a four-person team rebuilding a document workflow used by 18,000 weekly users; reduced median processing time from 11 seconds to **1.8 seconds** while preserving accessible, selectable output.
- Designed a typed ingestion pipeline that turned loosely structured customer files into a stable internal model shared by the web application and native batch tools.
- Partnered with design and support to simplify failure messages, cutting document-related support conversations by 34%.
- Introduced small operational improvements:
  - added request tracing around the two slowest boundaries;
  - wrote an incident guide that a new teammate could follow;
  - removed three services after measurements showed a single process was enough.
- Mentored engineers through architecture reviews and readable Rust patterns, including explicit ownership boundaries and concrete error types.

### Senior Software Engineer — Harbor Systems

*June 2018–December 2021 · Boston, MA*

- Built local-first editing and recovery features for field teams working with unreliable connectivity across the United States and México.
- Replaced a headless-browser export service with a deterministic document compiler, lowering infrastructure cost by 62% and eliminating a recurring class of pagination bugs.
- Shipped a versioned import/export format so customers could retain an independent copy of their work and recover from damaged browser storage.
- Worked with security engineers to constrain user-controlled links and assets without turning the product into a general-purpose sandbox.
- Presented performance findings to product and engineering peers, separating measured improvements from attractive but unsupported claims.

### Software Engineer — Juniper Analytics

*August 2015–May 2018 · Providence, RI*

- Developed reporting tools for nonprofit program managers and researchers.
- Improved keyboard navigation and screen-reader labels across the report editor after observing real user sessions.
- Maintained data imports for CSV, JSON, and a small partner API while keeping validation errors tied to source rows.
- Helped migrate the primary application from a tightly coupled deployment to independently testable library and application boundaries.

---

## Selected Projects

### Resumark — Local-first resume compiler

- Building a Rust and WebAssembly application that converts native Markdown into exact SVG page previews and selectable-text PDFs through Typst.
- Keeps user content local, passes a project-owned typed model between parser and renderer, and treats preview/export parity as a core invariant.
- Uses a deliberately small command-line harness during development: `resume build fixtures/resume.md`.

### Field Notes — Offline research notebook

- Created a compact progressive web application for interview notes with explicit save state and portable plain-text exports.
- Tested recovery behavior on mobile devices under interrupted connections and browser storage failures.

## Education

### Brown University

**B.Sc. in Computer Science**, 2015

Coursework included programming languages, human-computer interaction, distributed systems, and information visualization.

## Skills

- **Languages:** Rust, TypeScript, Python, SQL, HTML, CSS
- **Systems:** WebAssembly, document rendering, local-first storage, command-line applications
- **Practice:** product discovery, performance measurement, accessibility, technical writing, mentoring

## Community

- Volunteer résumé reviewer for early-career developers transitioning from service work into technology roles.
- Occasional speaker on making systems code readable to engineers who are still learning ownership and borrowing.
- Conversational Spanish; comfortable collaborating across English- and Spanish-speaking teams.

## Writing and Talks

- **“Local First Is a Recovery Promise, Not a Cache Strategy”** — practical notes on hydration order, visible save state, and source export for small browser applications.
- **“One Document, Two Outputs”** — conference talk showing how a shared compiled representation prevents subtle pagination differences between preview and PDF.
- Published a short internal series that introduced Rust ownership through application boundaries and concrete examples instead of lifetime notation first.

## Professional Development

- W3C Web Accessibility Initiative coursework covering keyboard interaction, semantic document structure, and responsible conformance claims.
- Facilitated quarterly incident-review workshops focused on system improvements, readable timelines, and follow-up work with named owners.
