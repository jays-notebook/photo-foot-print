# photo-foot-print

A native macOS and Windows 10/11 x64 desktop app for one-photo-at-a-time geotagging of
film/slide-scanner-output JPEGs (NORITSU-class scanners, typically
Photoshop-processed) that lack EXIF GPS coordinates. Pick a photo, click a spot
on a map (or paste lat/lng), and the GPS lands in the original JPEG.

Built for a personal film workflow, with a universal macOS app bundle and a
per-user Windows NSIS installer. Downloads are available through
[GitHub Releases](https://github.com/jays-notebook/photo-foot-print/releases/latest).
Windows builds are unsigned; macOS builds use ad-hoc signing without Apple
notarization. No automatic update service is configured.

## Status

v1.0 shipped on macOS on 2026-06-23. v1.1.0 introduced Windows 10/11 x64 support
using system WebView2 and a universal macOS download for Apple Silicon and
Intel Macs. v1.1.1 replaces the blank application icon with a photo frame and
map pin. See [downloads and installation](docs/RELEASES.md) for package and
signing details, and [Windows setup and validation](docs/WINDOWS.md) for the
distinction between native CI and desktop checks.

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

Install Rust through [rustup](https://rustup.rs/). The project pins Rust 1.99.1
in `rust-toolchain.toml`, including rustfmt and Clippy. Run commands from this
repository so rustup selects the pinned toolchain. The workspace's minimum
supported Rust version matches this tested toolchain.
Node.js 22.22.2+ on the 22.x line, 24.15+ on the 24.x line, or 26+ is required
by the frontend test tools. TypeScript stays on 6.x while `svelte-check` does
not support 7.x; Dependabot major TypeScript upgrades require a compatibility review.

On macOS, install the Xcode Command Line Tools and ExifTool for the optional
private scanner fixture gate:

```sh
xcode-select --install
brew install exiftool
```

On Windows, install the MSVC C++ build tools, Windows SDK, NASM, and WebView2
as described in [WINDOWS.md](docs/WINDOWS.md). Then, on either platform:

```sh
rustup show active-toolchain
npm ci --engine-strict
npm run tauri -- dev
```

The npm command uses the project-local Tauri CLI from the lockfile.
See [security maintenance](docs/SECURITY.md) for dependency audit commands,
automatic checks, and reviewed upstream findings.

## Project layout

- `crates/src-tauri/` — Tauri 2 host binary + IPC boundary
- `crates/pfp-exif/` — EXIF read + surgical in-place write (the load-bearing crate)
- `crates/pfp-photos/` — folder listing + thumbnail decode/cache
- `crates/pfp-tiles/` — OSM tile cache proxy
- `crates/pfp-state/` — persisted app state / last-used pin
- `src/` — Svelte 5 frontend (no SvelteKit)
- `tests/fixtures/scanner/` — private scanner-output JPEGs, supplied locally and ignored by Git
- `tests/support/` — deterministic synthetic JPEGs generated at test time for portable smoke tests

## Architectural constraints

- Tauri 2 + system WebKit on macOS and Evergreen WebView2 on Windows; no bundled Chromium or fixed WebView2 runtime.
- The four `pfp-*` library crates carry zero `tauri::*` symbols.
- JPEG-only, one-photo-at-a-time editing. Linux, Windows ARM64, batch editing, publisher signing and notarization are separate scope decisions.
