# YQN ZPL Forge WASM

Development slice based on upstream `rafael-arreola/zpl-forge` commit
`1bed1721cf8dde70e4f304710b923b0efc7f600a` (engine source matches crate 0.3.2).
Not a production-qualified release. Upstream `main` remains the baseline;
YQN changes are maintained on the development branch before release review.

## Responsibility

The fork owns the reusable Rust engine, optional Unicode PDF patch, JS bindings,
and package build. Components owns authentication, Artifact storage, Profiles,
routing, deadlines, capacity, retries, and runtime font asset loading. This
matches the YQN pdfcrop arrangement; no Components business model belongs here.

## Build and verify

Install Rust with the `wasm32-unknown-unknown` target and wasm-bindgen-cli 0.2.128.
No global Rust/Node configuration changes are required.

```sh
bash wasm/build.sh
node wasm/smoke.mjs
cargo test --manifest-path wasm/Cargo.toml --locked -p zpl-forge --lib
cargo test --manifest-path wasm/Cargo.toml --locked --lib
```

`WASM_BINDGEN` can specify an existing CLI executable and `CARGO_TARGET_DIR` an
external build directory. `bash wasm/build.sh --offline` uses cached dependencies.
Generated `wasm/pkg` includes ESM glue, WASM, declarations, source provenance and
licenses. The package is private until the release gate; no registry publication
or automatic cloud deployment is performed.

## JS byte interface

`ZplRenderer` owns built-in fonts; call `free()` when done. `setDefaultFont(bytes)`
replaces identifier `0` with a caller-provided font. Font bytes stay outside the
published WASM package and are not fetched by the engine.

- `renderPdf(zpl, widthDots, heightDots, dpi)` returns one PDF.
- `renderPng(zpl, widthDots, heightDots, dpi)` returns one PNG.
- `renderPdfPages(zpl, widthDots, heightDots, dpi, pagesJson)` renders the same
  template with one string-variable map per PDF page. It does not split arbitrary
  concatenated ZPL labels. One uppercase ^XA/^XZ format per template is required by the wrapper.

Initial wrapper limits: ZPL and variable JSON each 64 KiB, font 16 MiB, each canvas
dimension 1–4096 dots and area at most 4,194,304 dots², 1–32 variable-map pages,
output 16 MiB; nominal DPI is 203, 300 or 600. These bound the interface, not every
intermediate allocation in the parser/renderer. Application admission and hard
runtime CPU/memory limits remain necessary before untrusted public exposure.
Calls are synchronous, with no claimed mid-call cancellation. The output cap is
checked after encoding; this is not a pre-allocation memory cap.

## Unicode PDF behavior

The opt-in Rust `unicode-pdf` feature and `with_unicode_fonts()` backend mode use
TrueType CID fonts, ToUnicode maps and `subsetter` for used glyphs. Font 0 must be
supplied with a suitable CJK TrueType font for Chinese. Missing Unicode glyphs
fail; they are not silently replaced by `?`. The built-in CFF/OTF fonts retain
the existing WinAnsi path and reject non-WinAnsi text in this mode; Unicode CFF,
variable-font instantiation, font fallback and complex-script shaping are not
implemented. Native callers retain the original PDF behavior unless they opt in.
PNG remains the upstream raster engine; full missing-glyph preflight for PNG is
not implemented. Native imageproc parallelism is retained; wasm32 disables it.

Errors exported to JS are fixed codes, including `ZPL_INPUT_LIMIT`,
`ZPL_SINGLE_FORMAT_REQUIRED`, `ZPL_CANVAS_LIMIT`, `ZPL_RESOLUTION_INVALID`, `ZPL_INVALID`, `FONT_SIZE_LIMIT`,
`FONT_INVALID`, `ZPL_FONT_UNSUPPORTED`, `ZPL_RESOURCE_LIMIT`, `ZPL_RENDER_FAILED`,
`ZPL_VARIABLES_LIMIT`, `ZPL_VARIABLES_INVALID`, `ZPL_PAGE_LIMIT`, `ZPL_OUTPUT_LIMIT`.
No source text, native error details or credentials are exposed.

Engine code: MIT OR Apache-2.0. Bundled fonts retain their upstream license files
in `licenses/`; caller-supplied fonts require their own distribution rights.
Independent CLI, HTTP service, container and production routing are deferred.
