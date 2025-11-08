# Tideflow Agent Guidelines

## Current architecture snapshot
- **Frontend (React + Zustand):**
  - Entry point is `src/main.tsx`; app shell lives in `src/App.tsx` and wires the editor split view (CodeMirror editor + PDF iframe). State is centralized inside `src/stores/editorStore.ts` (document content, compile status, scroll sync data) and `src/stores/uiStore.ts` (preview visibility, modals, toast notifications).
  - Typing flows propagate through the hooks layered under `src/components/Editor`. In particular `src/hooks/useContentManagement.ts` owns the debounced auto-render loop: it calls `renderTypst` from `src/api.ts`, updates compile status in Zustand, and lets the queue in `renderTypst` coalesce fast edits so the Rust side never compiles stale buffers.
  - Compile status events (`compiled`, `compile-error`, `typst-query-*`) are listened for in `src/components/PDFPreview` and related hooks to refresh the iframe, manage scroll sync, and surface diagnostics.

- **Tauri backend (Rust):**
  - The command layer is defined in `src-tauri/src/commands`. File I/O lives in `file_ops.rs`; render/export endpoints are in `render_ops.rs` and forward to `src-tauri/src/renderer.rs`.
  - `renderer.rs` orchestrates preprocessing + Typst compilation. Markdown is first transformed by `preprocessor::preprocess_markdown` (currently only injects invisible `#label` anchors used for editor↔PDF scroll sync). The render pipeline helpers in `src-tauri/src/render_pipeline.rs` set up a working directory, copy `tideflow.typ`, sync theme assets, and launch the bundled Typst binary with `TYPST_PACKAGE_PATH` pointed at both packaged and user directories.
  - Successful renders produce a PDF path and a `SourceMapPayload` that the frontend consumes for scroll sync; on failure the command emits events the UI is already listening to.

- **Typst template + assets:**
  - `src-tauri/content/tideflow.typ` defines the rendering contract. It imports `@preview/cmarker:0.1.6`'s `render` function, applies the selected theme (`src/themes.ts` mirrors the Typst themes), configures TOC/cover/page layout based on `prefs.json`, and finally calls `#render(md_content, ...)` with custom scopes for things like safe links and HTML `<img>` nodes.
  - Bundled Typst packages live under `src-tauri/content/typst/packages`. At runtime the app copies them into the user profile (`utils::initialization`) so Typst can locate them offline.

- **Persistence & preferences:**
  - Document session restoration is handled in `src/utils/session.ts`. Preferences are persisted through `src-tauri/src/preferences.rs` and surfaced to the UI via Zustand selectors; saving preferences triggers a re-render by updating `prefs.json` before the next compile.

- **Build/test commands:**
  - `npm run tauri:dev` launches the dev UI with the Rust backend.
  - `npm run lint` runs the ESLint suite.
  - `npm run test` is currently a placeholder (no Jest/Vitest tests ship with the repo).

## TikZ rendering integration strategy
1. **Expose a Markdown affordance.**
   - Treat fenced code blocks tagged `tikz` (```` ```tikz ... ``` ````) as TikZ drawings. Detect them inside the Rust preprocessor *before* anchor injection so offsets stay consistent. Recommended approach: add a new transformation in `preprocessor.rs` that scans the Markdown AST via `pulldown_cmark` and replaces each TikZ fence with an HTML comment placeholder such as `<!--raw-typst #tikz_diagram("anchor-id")[#tikz.render(block: "...")] -->`. Because `preprocess_markdown` output is only used for rendering, the user-facing Markdown file remains unchanged.
   - Keep the original code fence text somewhere Typst can read it. One pragmatic option is to emit a paired raw block: insert a unique sentinel like `<!--raw-typst #tikz-render(
    raw("...escaped tikz...")
  ) -->` immediately after the code fence. Remember to HTML-escape or JSON-escape line breaks and backslashes so Typst receives valid string literals.

2. **Extend the Typst template.**
   - Update `src-tauri/content/tideflow.typ` to import the TikZ helper package (e.g. `#import "@preview/tikz:0.1.0": tikz`). Provide a small wrapper function (`#let tikz_render(diagram: str, scale: auto = none, ..)`) that converts the TikZ source into an image usable inside the flow (`tikz.render(diagram: diagram, scale: scale, preamble: ..., format: vector)` depending on the package API).
   - Expose that helper in the `render(..., scope: ( ... ))` call so Markdown HTML replacements can reference it. For example, inside `render(... html: (...))` register a handler for a custom `<tikz>` tag, *or* simpler: ensure the raw Typst comment the preprocessor injects calls `#tikz_render("...")` directly.

3. **Bundle the TikZ Typst package offline.**
   - Place the TikZ package directory under `src-tauri/content/typst/packages/preview/tikz/<version>` (matching the package’s expected layout). Update release packaging scripts if necessary so the folder ships with builds. The existing `collect_typst_package_paths` will then make it available automatically.
   - If the package relies on external binaries (e.g., LaTeX), document/handle that dependency. Ideally choose a pure-Typst TikZ renderer to stay offline-first; otherwise add a guard that surfaces a clear UI error if the toolchain is missing.

4. **Synchronise with frontend expectations.**
   - When injecting raw Typst comments, ensure `scrubRawTypstAnchors` in `src/utils/scrubAnchors.ts` either ignores TikZ markers or the markers never hit disk. Because TikZ placeholders are added only in the in-memory render string, no scrub changes should be required.
   - Add syntax highlighting for `tikz` fences by updating the CodeMirror language list in `src/components/Editor/extensions/syntax.ts` (if present). Even a fallback to the existing TeX highlighter improves UX.
   - Surface compile diagnostics from TikZ failures by letting Typst errors bubble through the existing `compile-error` event; add user-friendly copy in `src/components/StatusBar` if TikZ-specific failures need different messaging.

5. **Testing & regression checks.**
   - Create a focused Markdown sample (e.g., `content/tikz-sample.md`) that contains a TikZ fence and add it to the manual regression checklist. Optionally wire a Playwright/e2e smoke test that renders the sample and asserts the produced PDF contains the expected label (requires PDF text extraction).
   - During development run `npm run tauri:dev`, paste the sample TikZ block, and confirm the preview updates without hanging the render queue. Watch the Tauri console for errors thrown by the TikZ package.

## Additional guardrails
- When editing Rust code, follow the existing style (rustfmt default). When editing TypeScript, keep imports sorted loosely by path length, and prefer existing logger utilities (`logger.createScoped`).
- Avoid modifying `scrubRawTypstAnchors` unless you also update every write path that relies on it (`writeMarkdownFile`, `exportCleanMarkdown`).
- Any new Typst helper should be placed near the existing ones in `tideflow.typ`; keep cover/TOC logic unchanged unless you have explicit instructions to reorganise them.
- Remember the project is offline-first: any new dependency must either ship with the repo or degrade gracefully when offline.
