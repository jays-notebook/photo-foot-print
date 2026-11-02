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

$installers = @(Get-ChildItem (Join-Path $ReleaseDir 'bundle/nsis/*-setup.exe'))
if ($installers.Count -ne 1 -or $installers[0].Length -eq 0) {
    throw 'Expected exactly one nonempty NSIS installer'
}
Get-FileHash $installers[0].FullName -Algorithm SHA256 | Format-List

# A fresh per-run path avoids modifying any pre-existing installation.
$installDir = Join-Path ([System.IO.Path]::GetTempPath()) ('pfp smoke ' + [guid]::NewGuid())
$app = $null
try {
    # NSIS requires /D last; the unquoted remainder supports spaces in the path.
    $setup = Start-Process $installers[0].FullName -ArgumentList "/S /D=$installDir" -Wait -PassThru
    if ($setup.ExitCode -ne 0) { throw "Installer failed: $($setup.ExitCode)" }
    $installedApp = Join-Path $installDir 'photo-foot-print.exe'
    if (-not (Test-Path $installedApp)) { throw 'Installed application is missing' }
    if ((Get-FileHash $installedApp).Hash -ne (Get-FileHash $appPath).Hash) {
        throw 'Installed application differs from the verified build'
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
