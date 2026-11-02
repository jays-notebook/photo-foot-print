# Security maintenance

The application processes one local JPEG at a time on macOS and Windows 10/11
x64. The
WebView uses local application assets with CSP enabled. Photo I/O stays in
Rust; filesystem and shell plugins are not exposed to the frontend. Keep
these boundaries when updating dependencies.

Windows uses system Evergreen WebView2. Its custom thumbnail/tile image
origins are explicitly allowed in CSP; arbitrary HTTP origins, filesystem
access and shell permissions are not added. Native save confirmation accepts
only the raw explicit Save button result; dismissal and dialog errors never
approve a write.

## EXIF input validation

Before little_exif decodes the EXIF APP1 payload, the TIFF preflight checks
field lengths and offsets, nested EXIF/GPS/Interop directories, and embedded
thumbnail/strip ranges. It rejects repeated directory references (including
shared directories), more than 64 directories, nesting of 8 or more, more
than 4,096 entries, and a weighted 4 MiB decoding budget. The budget accounts
for repeated data and conversion overhead; it is not an exact process-memory
limit. Unusually complex metadata may therefore be rejected even if another
reader accepts it. Errors during save leave the original photo intact.

little_exif is pinned exactly because this preflight follows its decoder's
semantics. Review `tiff_preflight.rs` and run the scanner gates when updating
the parser. MakerNotes remain opaque; this does not add DSLR-specific support.

## Dependency checks

Use the committed Rust toolchain and a supported Node version (see
`package.json`). Install the audit tool once, then run the same policy as CI:

```sh
cargo install cargo-audit --version 0.22.2 --locked
make audit
```

On Windows PowerShell, run the same commands directly without GNU make:

```powershell
npm audit --omit=dev
npm audit --audit-level=moderate
cargo audit --ignore RUSTSEC-2026-0194 --ignore RUSTSEC-2026-0195
```

GitHub Actions runs these checks on pull requests, main pushes, and weekly.
Dependabot checks Cargo, npm, and GitHub Actions weekly. Actions are pinned to
commit hashes and run with read-only repository permissions. These workflows
do not require or upload private photo fixtures.

- npm runtime dependencies fail on any advisory. The complete npm tree fails
  on moderate or higher severity; low development findings remain in output
  and require review.
- Rust fails on vulnerability advisories except the two reviewed XML cases
  below. Maintenance/unsoundness warnings remain visible. The full lockfile is
  checked, including dependencies for platforms outside the shipped scope.
- A policy pass does **not** mean the raw audit has zero findings. To see all
  findings without exceptions, run `npm audit` and `cargo audit` individually.
  Both currently return a nonzero status for the cases below.

## Reviewed residual findings (2026-11-02)

### quick-xml 0.37.5 through little_exif 0.6.23

[RUSTSEC-2026-0194](https://rustsec.org/advisories/RUSTSEC-2026-0194.html)
and [RUSTSEC-2026-0195](https://rustsec.org/advisories/RUSTSEC-2026-0195.html)
are fixed in quick-xml 0.41+, but the latest little_exif release still requires
0.37.5. The separate plist dependency uses patched quick-xml 0.42.0.

The affected paths parse XML attributes/namespaces. The application's JPEG
reader passes only the EXIF APP1 segment to little_exif, and its JPEG writer
replaces EXIF while preserving other segments as bytes. In little_exif 0.6.23,
the only caller of `xmp::remove_exif_from_xmp` is the PNG writer, which this
application does not use. This is a reachability assessment, not an upstream
fix. `make audit-rust` explicitly excepts only these advisory IDs; no global
cargo-audit suppression file is installed.

The Windows extension uses the same JPEG-only reader/writer and introduces
no XML consumer, so this reachability assessment also applies on Windows.

Review/remove these exceptions when little_exif or its dependency requirement
changes. Reassess them before adding PNG, XML/XMP parsing, another quick-xml
consumer, or another supported platform. The separate TIFF allocation and
offset validation protects EXIF decoding; it does not patch quick-xml.

### deepmerge 4.3.1 through the Svelte build plugin

[GHSA-gcqx-w48m-w2h4](https://github.com/advisories/GHSA-gcqx-w48m-w2h4)
has no published fix at review time. npm reports two low-severity affected
nodes (`deepmerge` and `@sveltejs/vite-plugin-svelte`) for this one advisory.
The plugin merges trusted local build configuration; application photo/user
data does not flow through it. It is a development dependency, and the npm
runtime audit has zero findings. Keep it visible and update when a compatible
patch is published; do not override it with an incompatible package.

### Rust maintenance and platform warnings

The full lockfile also reports discontinued `async-std` (via the test-only
httpmock), unmaintained `paste` and `proc-macro-error`, and the `glib` iterator
unsoundness advisory RUSTSEC-2024-0429. The glib dependency is absent from the
macOS and Windows build graphs. These warnings are not suppressed or described as fixed;
review upstream replacements and reassess before supporting Linux.

## Upgrade verification

Run frontend checks/tests/build, Rust workspace tests, rustfmt, Clippy with
warnings denied, a local macOS Tauri build, and the Windows native CI described
in [WINDOWS.md](WINDOWS.md). For EXIF parsing/writing
changes, also run `make test-gate` and `make test-fault` with the local scanner
fixtures described in [FIXTURES.md](FIXTURES.md). An unavailable fixture is a
missing check, not a passing gate.
