# Bundled fonts

The Stage 0 renderer embeds the Libertinus Serif family through the pinned
`typst-assets` crate instead of copying the same binary files into this
repository. The family is distributed under the SIL Open Font License 1.1.

The dependency also contains Typst's fallback font set, but the rendering
adapter filters it out and exposes only Libertinus Serif to the in-memory Typst
world. Before release, replace this note with the final font inventory and
include every upstream license text beside the production assets.

- Libertinus project: <https://github.com/alerque/libertinus>
- Typst assets: <https://github.com/typst/typst-assets>
