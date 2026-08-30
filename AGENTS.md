# Resumark working rules

Resumark is a small side project whose immediate goal is a good-enough local
v1, not a reusable platform. Optimize for reaching a pleasant
Markdown-to-preview-to-PDF workflow with code that is easy to read.

## Code must earn its place

- Add code for behavior required by the current implementation slice, a
  boundary that would be expensive to retrofit, or a failure that would be
  dangerous and hard to notice.
- Do not add crates, traits, generic abstractions, protocols, configuration,
  test infrastructure, or extension points only because a future version might
  use them.
- Prefer the smallest concrete implementation. Refactor after a second real
  use case appears or the current code becomes difficult to understand.
- Favor a working vertical slice and manual use over broad scaffolding. Add
  focused checks for security, data loss, preview/export parity, and known
  regressions; comprehensive automation can follow a useful product.
- Keep Rust readable to someone who is not fluent in it: ordinary structs and
  enums, owned values, explicit names, short control flow, and few layers.
- Treat roadmap details as provisional. If a planned mechanism is larger than
  the user-visible behavior it enables, implement the simpler behavior and
  update the plan.

## v1 invariants worth protecting

- User Markdown is data and is never interpolated into Typst source.
- Preview SVGs and the downloaded PDF come from the same compilation.
- Resume content remains local and can be exported as Markdown.
- A failed parse, render, or save does not silently destroy the last good work.
- Typst and browser-specific types stay behind small, direct boundaries.

Everything else should be justified by the next visible product outcome.
