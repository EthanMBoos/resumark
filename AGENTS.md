# Working rules

Resumark turns a user-owned Markdown resume into a styled PDF. The source is
edited outside Resumark in the user's Markdown editor.

- Build the choose, preview, style, and download flow first.
- Do not add a text editor, accounts, storage, autosave, versions, or document
  collections.
- Add code for current behavior, not possible future features.
- Prefer plain structs, enums, owned values, and short functions.
- Avoid generic traits and extra crates until a real second use needs them.

## Rules that cannot break

- User Markdown is data and is never inserted into Typst source.
- Preview SVGs and the PDF come from the same compile.
- Resume content stays in the browser.
- Invalid source has no active preview or PDF download.
- Replaced Blob URLs are revoked.
- Typst and browser types stay out of the core model.

## Browser verification

Install once:

```sh
npm install
npx playwright install chromium
```

For each run:

1. Check port 8080: `lsof -nP -iTCP:8080 -sTCP:LISTEN`.
2. Stop an old Resumark process if it owns the port. Do not kill an unknown
   process.
3. Run `npm run test:web` from the repository root.
4. Check the preview, PDF download, and browser console.
5. Inspect the screenshot under `target/` after visible changes.
6. Check port 8080 again. It must be clear.

Playwright must start its own server. Do not reuse an existing one.
