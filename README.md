# photo-foot-print

A native macOS desktop app for one-photo-at-a-time geotagging of
film/slide-scanner-output JPEGs (NORITSU-class scanners, typically
Photoshop-processed) that lack EXIF GPS coordinates. Pick a photo, click a spot
on a map (or paste lat/lng), and the GPS lands in the original JPEG.

Built for a personal film workflow — a macOS-only, local developer build (no
signing, no notarization, no installer).

## Status

v1.0 shipped 2026-06-23. macOS local developer build.

## Features

- Open a folder via the native file dialog.
- Scrolling photo list with thumbnails and a HAS GPS / MISSING GPS badge per file.
- Select a photo to preview it full-size.
- Pick coordinates by clicking/dragging a pin on a Leaflet + OpenStreetMap map,
  or by pasting `lat, lng`.
- Edit or add the EXIF capture time (`DateTimeOriginal`, with `DateTimeDigitized`
  written alongside).
- In-place atomic EXIF write into the original JPEG via `little_exif` surgical
  APP1 replacement — the compressed image bytes are left untouched.
- Reuse the last-used pin across photos and sessions.
- OSMF-compliant Rust-side tile cache proxy (`pfp-tile://`, 7-day cache).

## Run (dev)

```bash
# Prerequisites (one-time):
cargo install tauri-cli --version "^2" --locked
brew install exiftool git-lfs
git lfs install --local

# exiftool + git-lfs are required for the metadata-preservation gate
# (`make test-gate`). See docs/FIXTURES.md for sourcing a scanner-output fixture.

npm install
cargo tauri dev
```

## Project layout

- `crates/src-tauri/` — Tauri 2 host binary + IPC boundary
- `crates/pfp-exif/` — EXIF read + surgical in-place write (the load-bearing crate)
- `crates/pfp-photos/` — folder listing + thumbnail decode/cache
- `crates/pfp-tiles/` — OSM tile cache proxy
- `crates/pfp-state/` — persisted app state / last-used pin
- `src/` — Svelte 5 frontend (no SvelteKit)
- `tests/fixtures/scanner/` — representative scanner-output JPEGs, stored via Git LFS

## Architectural constraints

- Tauri 2 + OS-native WebKit webview only — no bundled Chromium under any circumstance.
- The four `pfp-*` lib crates carry zero `tauri::*` symbols (preserves the future-Windows seam).
- macOS-only for v1; cross-platform packaging is post-v1 by design.
