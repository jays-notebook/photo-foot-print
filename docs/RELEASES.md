# Downloads and installation

Download packages from [GitHub Releases](https://github.com/jays-notebook/photo-foot-print/releases/latest).
v1.1.1 replaces the blank application icon with a photo frame and map pin.
It provides:

| Asset | Platform |
| --- | --- |
| `photo-foot-print_1.1.1_x64-setup.exe` | Windows 10/11 x64, per-user NSIS installer |
| `photo-foot-print_1.1.1_macos-universal.app.zip` | macOS, Apple Silicon and Intel |
| `SHA256SUMS` | SHA-256 hashes of both packages |

The application uses the operating system's webview: WebKit on macOS and
Microsoft Evergreen WebView2 on Windows. No Chromium or fixed WebView2
runtime is bundled. Linux, Windows ARM64 and 32-bit Windows are unsupported.

## Windows

Run the `x64-setup.exe` installer. It installs for the current user and checks
for Evergreen WebView2. If the runtime is missing, the installer downloads
Microsoft's bootstrapper; that step requires internet access. Rust, Node.js,
NASM and Visual Studio are not needed to use a downloaded package.

The installer and application are unsigned. Windows may show an
unknown-publisher or SmartScreen warning. Verify the download source and
checksum before deciding whether to run it. Use Windows Installed apps to
uninstall the application.

## macOS

Extract the ZIP and move `photo-foot-print.app` to Applications. The same
bundle contains `arm64` and `x86_64` executables for Apple Silicon and Intel.

The bundle has an ad-hoc signature for binary integrity, not an Apple
Developer ID signature, and is not notarized. Gatekeeper may prevent the
downloaded app from opening. Consult your macOS security settings or your
administrator; do not disable system security protections to install it.

## Verify a download

Download `SHA256SUMS` from the same release and compare the package's hash
with its entry. On Windows PowerShell:

```powershell
Get-FileHash .\photo-foot-print_1.1.1_x64-setup.exe -Algorithm SHA256
```

On macOS:

```sh
shasum -a 256 photo-foot-print_1.1.1_macos-universal.app.zip
```

Checksums detect a damaged or mismatched download; they do not replace
publisher signing. Updates are installed manually from GitHub Releases.

## Validation scope

The native CI workflow builds and checks release packages. Windows Server
2022 CI validates the x64 GUI binary, installs it, opens a native window and
uninstalls it. macOS packaging checks both executable architectures, bundle
version and signature integrity. See each release's notes and linked CI
results for the checks completed for that version.

Manual Windows 10 and Windows 11 desktop acceptance remains pending,
including map and thumbnail rendering and confirmation-dialog keyboard
behavior. The [desktop validation procedure](WINDOWS.md#desktop-validation-still-required)
records these checks separately from Server CI. Use copies of scanner JPEGs
for acceptance testing. Private scanner fixtures are never distributed with
the application or release assets.
