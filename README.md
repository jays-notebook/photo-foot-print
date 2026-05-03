# photo-foot-print

A native macOS desktop app for one-photo-at-a-time geotagging of camera JPEGs that lack EXIF GPS coordinates. Pick a photo, click a spot on a map, the GPS lands in the original JPEG.

## Status

Phase 1 (Bootstrap + EXIF Spike Gate) — see `.planning/phases/01-bootstrap-exif-spike-gate/`.

## Run (dev)

```bash
# Prerequisites (one-time):
cargo install tauri-cli --version "^2" --locked
brew install exiftool git-lfs
git lfs install --local

# Sourcing fixtures: see docs/FIXTURES.md (required for the MakerNote-preservation gate).

npm install
cargo tauri dev
```

## Project layout

- `crates/src-tauri/` — Tauri 2 host binary
- `crates/pfp-exif/` — EXIF read + surgical write (the load-bearing crate)
- `crates/pfp-photos/`, `crates/pfp-tiles/`, `crates/pfp-state/` — Phase 2/3/4 lib crates (skeletons in Phase 1)
- `src/` — Svelte 5 frontend (no SvelteKit)
- `tests/fixtures/` — Sony / Canon / Nikon DSLR JPEGs, stored via Git LFS

## Architectural constraints

- Tauri 2 + OS-native WebKit webview only — no bundled Chromium under any circumstance.
- The four `pfp-*` lib crates carry zero `tauri::*` symbols (preserves the future-Windows seam).
- macOS-only for v1; cross-platform packaging is post-v1 by design.
