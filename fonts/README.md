# Bundled fonts

Resumark bundles only the four Libertinus Serif faces the trusted theme can
request:

- Regular
- Italic
- Bold
- Bold Italic

The files under [`libertinus/`](libertinus/) came from the pinned
`typst-assets` 0.15.1 distribution and are included directly so the future
WebAssembly compiler does not carry Typst's unrelated fallback fonts. The
family is distributed under the SIL Open Font License 1.1; its copyright
notice and complete license are in [`libertinus/OFL.txt`](libertinus/OFL.txt).

- Libertinus project: <https://github.com/alerque/libertinus>
- Typst assets: <https://github.com/typst/typst-assets>
