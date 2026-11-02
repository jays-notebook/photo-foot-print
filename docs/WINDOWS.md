# Windows 10/11 x64

The Windows application uses Microsoft Evergreen WebView2 and the same Rust
JPEG pipeline as macOS. It does not bundle Chromium or a fixed WebView2
runtime. Windows ARM64, 32-bit Windows, and Linux are not supported targets.

## Developer prerequisites

Install:

- Visual Studio 2022 Build Tools with **Desktop development with C++**, the
  MSVC x64/x86 compiler, and a Windows SDK. Use an x64 developer PowerShell
  if the compiler is not detected in a normal shell.
- [Rustup](https://rustup.rs/) with the MSVC host toolchain. The repository
  selects Rust 1.99.1, rustfmt and Clippy automatically. `rustc -vV` should
  report host `x86_64-pc-windows-msvc`.
- Node.js 22.12+ on the 22.x line, 24.x, or 26+.
- [NASM](https://www.nasm.us/) on PATH. The TLS backend's aws-lc-sys build uses
  it for x64 assembly; `nasm -v` must succeed before building. Restart the
  terminal after changing PATH. See the [AWS-LC Windows requirements](https://aws.github.io/aws-lc-rs/requirements/windows.html).
- Microsoft [Evergreen WebView2 Runtime](https://developer.microsoft.com/en-us/microsoft-edge/webview2/)
  for development. The installer also checks for it and downloads Microsoft's
  bootstrapper if missing, which requires internet access.

See [Tauri's prerequisites](https://v2.tauri.app/start/prerequisites/) for the
build tools and SDK installation details. ExifTool is needed only for the
private scanner metadata gate, not to use the application.

## Develop and build

From the repository root in PowerShell:

```powershell
rustup show active-toolchain
npm ci
npm run tauri -- dev
```

Build the unsigned per-user installer on Windows:

```powershell
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis --ci --no-sign -- --locked
```

The installer is written to
`target/x86_64-pc-windows-msvc/release/bundle/nsis/*-setup.exe`.
`tauri.windows.conf.json` selects NSIS, current-user installation and the
downloaded WebView2 bootstrapper. The ordinary macOS build still selects
the `.app` target. No certificate, public release or auto-update feed is
configured. Windows may show an unsigned-publisher/SmartScreen prompt;
signing is a separate release task.

State is stored beneath `%LOCALAPPDATA%\photo-foot-print`. Thumbnail cache
files are stored beside photos under `.pfp-thumbs`; read-only folders use the
existing memory-only fallback. Tile caching uses Tauri's per-user cache
directory. Folder and photo paths may contain spaces and Unicode characters.

## Safe saves

Save confirmation defaults to **Cancel**. Only the explicit native **Save**
button approves a write. The host preserves the raw dialog result so Esc,
window close, Alt-F4, or a dialog failure cannot be mistaken for Save.

EXIF writes use a sibling temporary file, validation and a write-capable
file flush, then a same-volume replacement with Windows WRITE_THROUGH.
Failed validation or replacement preserves the original and removes the
temporary file. A photo held by another application without delete-sharing,
or a read-only target, can fail to save; close the other application or make
the file writable and retry. macOS retains its existing full-flush path.
These are filesystem/OS durability operations, not a guarantee against every
hardware failure or arbitrary network filesystem behavior.

## Automated checks

These commands do not require GNU make or private photos:

```powershell
npm run check
npm test
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo test -p pfp-exif --test fault_injection --features fault-injection --locked
```

The `Native desktop` GitHub workflow runs on `windows-2022` with the x64 MSVC
target. It checks/tests both layers, runs fixture-free fault tests, builds
NSIS, validates the application's x64 GUI PE header, installs to a temporary
path containing spaces, starts a native window, and uninstalls. The installer
is retained as a CI artifact for 14 days; no private photo fixtures are
uploaded. Generated JPEG tests cover listing, thumbnail decoding, cache and
metadata round trips in Unicode/spaced paths.

For the real scanner metadata gate, install ExifTool, supply your own fixtures
and run the commands in [FIXTURES.md](FIXTURES.md). Synthetic smoke tests are
not a substitute for those preservation checks.

## Desktop validation still required

Windows Server 2022 CI is a native Windows build/runtime check. It does not
prove every interaction on Windows 10 and Windows 11 desktops. On both:

1. Install the application and open a folder containing copies of scanner JPEGs.
2. Confirm thumbnails and map tiles render; select a photo and pan/click the map.
3. Change GPS/time, then use initial Enter, Esc, Cancel, title-bar close, and
   Alt-F4 in separate confirmation dialogs. None should alter the photo.
4. Explicitly select Save; re-open the photo and confirm GPS/time. Test changing
   the selected photo while a save confirmation is pending.
5. Repeat with Unicode/spaced paths, a read-only photo and a photo locked by
   another application. Failed saves must leave originals unchanged.
6. Restart the app to check persisted state, then uninstall it.

Use copies for these checks. Record the Windows build and WebView2 version
with the result; do not treat the CI window-start check as this manual test.
