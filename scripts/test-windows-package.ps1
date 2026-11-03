param(
    [string]$ReleaseDir = 'target/x86_64-pc-windows-msvc/release'
)

$ErrorActionPreference = 'Stop'

# Check the application PE, not the NSIS stub (which may itself be 32-bit).
$appPath = Join-Path $ReleaseDir 'photo-foot-print.exe'
$bytes = [System.IO.File]::ReadAllBytes((Resolve-Path $appPath))
if ($bytes.Length -lt 64 -or $bytes[0] -ne 0x4D -or $bytes[1] -ne 0x5A) {
    throw 'Application is not a PE executable'
}
$pe = [BitConverter]::ToInt32($bytes, 0x3C)
if ($pe -lt 0 -or $pe -gt $bytes.Length - 94 -or [BitConverter]::ToUInt32($bytes, $pe) -ne 0x4550) {
    throw 'Application has an invalid PE header'
}
if ([BitConverter]::ToUInt16($bytes, $pe + 4) -ne 0x8664) {
    throw 'Application is not x64'
}
if ([BitConverter]::ToUInt16($bytes, $pe + 24) -ne 0x20B -or
    [BitConverter]::ToUInt16($bytes, $pe + 92) -ne 2) {
    throw 'Application must be a PE32+ GUI executable without a console window'
}
$builtApp = Get-Item -LiteralPath $appPath
$builtAppHash = (Get-FileHash -LiteralPath $appPath -Algorithm SHA256).Hash
Write-Host ("Build EXE: {0} bytes; SHA256 {1}; {2}" -f $builtApp.Length, $builtAppHash, $builtApp.FullName)

# Model the unsigned NSIS patch without changing the build output on disk.
# Tauri replaces this fixed-length marker, packages the EXE, then restores it:
# https://github.com/tauri-apps/tauri/blob/tauri-cli-v2.12.4/crates/tauri-bundler/src/bundle.rs
# Latin1 preserves each byte as one character, keeping IndexOf offsets exact.
# Signed packages require separate validation because signing also changes bytes.
$bundleMarker = '__TAURI_BUNDLE_TYPE_VAR_UNK'
$nsisMarker = '__TAURI_BUNDLE_TYPE_VAR_NSS'
$binaryText = [System.Text.Encoding]::Latin1.GetString($bytes)
$markerOffset = $binaryText.IndexOf($bundleMarker, [System.StringComparison]::Ordinal)
if ($markerOffset -lt 0) { throw 'Tauri bundle-type marker is missing from the build EXE' }
if ($binaryText.IndexOf($bundleMarker, $markerOffset + $bundleMarker.Length, [System.StringComparison]::Ordinal) -ge 0) {
    throw 'Tauri bundle-type marker is not unique in the build EXE'
}
$markerBytes = [System.Text.Encoding]::ASCII.GetBytes($nsisMarker)
if ($markerBytes.Length -ne $bundleMarker.Length) { throw 'Tauri bundle-type marker lengths differ' }
[byte[]]$expectedBytes = $bytes.Clone()
[System.Array]::Copy($markerBytes, 0, $expectedBytes, $markerOffset, $markerBytes.Length)
$expectedAppHash = [System.Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData($expectedBytes))
Write-Host ("Expected NSIS EXE: {0} bytes; SHA256 {1}" -f $expectedBytes.Length, $expectedAppHash)

$version = (Get-Content -LiteralPath (Join-Path $PSScriptRoot '../package.json') -Raw | ConvertFrom-Json).version
$installer = Get-Item -LiteralPath (Join-Path $ReleaseDir "bundle/nsis/photo-foot-print_${version}_x64-setup.exe")
if ($installer.Length -eq 0) {
    throw 'Expected a nonempty NSIS installer for the current product version'
}
Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256 | Format-List

# A fresh per-run path avoids modifying any pre-existing installation.
$installDir = Join-Path ([System.IO.Path]::GetTempPath()) ('pfp smoke ' + [guid]::NewGuid())
$app = $null
try {
    # NSIS requires /D last; the unquoted remainder supports spaces in the path.
    $setup = Start-Process $installer.FullName -ArgumentList "/S /D=$installDir" -Wait -PassThru
    if ($setup.ExitCode -ne 0) { throw "Installer failed: $($setup.ExitCode)" }
    $installedApp = Join-Path $installDir 'photo-foot-print.exe'
    if (-not (Test-Path $installedApp)) { throw 'Installed application is missing' }
    $installedFile = Get-Item -LiteralPath $installedApp
    $installedAppHash = (Get-FileHash -LiteralPath $installedApp -Algorithm SHA256).Hash
    Write-Host ("Installed EXE: {0} bytes; SHA256 {1}; {2}" -f $installedFile.Length, $installedAppHash, $installedFile.FullName)
    if ($installedAppHash -ne $expectedAppHash) {
        throw 'Installed application differs from the expected unsigned NSIS build'
    }
    if (-not (Test-Path (Join-Path $installDir 'uninstall.exe') -PathType Leaf)) {
        throw 'Installed uninstaller is missing'
    }

    $app = Start-Process $installedApp -PassThru
    $deadline = [DateTime]::UtcNow.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 500
        $app.Refresh()
        if ($app.HasExited) { throw "Application exited during startup: $($app.ExitCode)" }
    } while ($app.MainWindowHandle -eq 0 -and [DateTime]::UtcNow -lt $deadline)
    if ($app.MainWindowHandle -eq 0) { throw 'Application did not create a native window' }
    Write-Host 'x64 GUI application installed and created a native window.'
} finally {
    if ($null -ne $app -and -not $app.HasExited) {
        $null = $app.CloseMainWindow()
        if (-not $app.WaitForExit(5000)) { $app.Kill(); $app.WaitForExit() }
    }
    $uninstaller = Join-Path $installDir 'uninstall.exe'
    if (Test-Path $uninstaller) {
        # NSIS normally spawns a temporary uninstaller and exits early. _?=
        # keeps execution synchronous for this disposable installation.
        $removed = Start-Process $uninstaller -ArgumentList "/S _?=$installDir" -Wait -PassThru
        if ($removed.ExitCode -ne 0) { throw "Uninstaller failed: $($removed.ExitCode)" }
        if (Test-Path (Join-Path $installDir 'photo-foot-print.exe')) {
            throw 'Uninstaller left the application executable behind'
        }
    }
    if (Test-Path $installDir) { Remove-Item $installDir -Recurse -Force }
}

# Export only the verified package; restored caches may contain older versions.
if ($env:GITHUB_OUTPUT) {
    Add-Content -LiteralPath $env:GITHUB_OUTPUT -Value "installer=$($installer.FullName)"
}
