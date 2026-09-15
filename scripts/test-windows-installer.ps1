param([Parameter(Mandatory = $true)][string]$Directory, [switch]$CheckUpdates)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
    throw 'This install/uninstall smoke test is intended for an ephemeral GitHub Actions runner.'
}
$metadataFiles = @(Get-ChildItem -LiteralPath $Directory -Filter '*-setup.build.json')
if ($metadataFiles.Count -ne 1) { throw 'Expected one installer build record' }
$record = Get-Content -Raw -LiteralPath $metadataFiles[0].FullName | ConvertFrom-Json
$targetPolicy = switch ($record.source_build.target) {
    'x86_64-pc-windows-msvc' { @{ Architecture = 'X64'; Machine = 0x8664 } }
    'aarch64-pc-windows-msvc' { @{ Architecture = 'Arm64'; Machine = 0xAA64 } }
    default { throw "Unsupported installer target: $($record.source_build.target)" }
}
$hostArchitecture = [Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
if ($hostArchitecture -ne $targetPolicy.Architecture) {
    throw "Native qualification requires $($targetPolicy.Architecture) Windows, got $hostArchitecture"
}
if ($hostArchitecture -eq 'Arm64' -and [Environment]::OSVersion.Version.Build -lt 22000) {
    throw 'The ARM64 package requires Windows 11 for its x64 FEFF10 helper.'
}
$setup = Join-Path (Resolve-Path $Directory) $record.installer
if ((Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant() -ne $record.sha256) {
    throw 'Installer checksum mismatch'
}
$product = if ($record.source_build.channel -eq 'nightly') { 'rexafs Nightly' } else { 'rexafs' }
$appId = if ($record.source_build.channel -eq 'nightly') { 'rexafs.desktop.nightly' } else { 'rexafs.desktop' }
$registry = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\${appId}_is1"
if (Test-Path $registry) { throw 'Refusing to overwrite an existing installation' }
$testRoot = Join-Path $env:RUNNER_TEMP ('rexafs installer smoke ' + [guid]::NewGuid())
$installed = Join-Path $testRoot 'Programs 日本語'
$logs = Join-Path (Resolve-Path $Directory) 'smoke-logs'
New-Item -ItemType Directory -Path $logs -Force | Out-Null
$programs = [Environment]::GetFolderPath('Programs')
$desktop = [Environment]::GetFolderPath('Desktop')
$startLink = Join-Path $programs "$product.lnk"
$desktopLink = Join-Path $desktop "$product.lnk"
if ((Test-Path $startLink) -or (Test-Path $desktopLink)) { throw 'Existing shortcuts found' }

# WScript.Shell exposes the legacy ANSI target and loses Japanese characters.
# Read the actual Unicode shell-link interface used by Explorer instead.
Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
using System.Runtime.InteropServices.ComTypes;
public static class InstallerShortcut {
    [ComImport, Guid("00021401-0000-0000-C000-000000000046")]
    private class ShellLink {}
    [ComImport, Guid("000214F9-0000-0000-C000-000000000046"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    private interface IShellLinkW {
        void GetPath([Out, MarshalAs(UnmanagedType.LPWStr)] StringBuilder path, int size, IntPtr data, int flags);
    }
    public static string Target(string filename) {
        object link = new ShellLink();
        try {
            ((IPersistFile)link).Load(filename, 0);
            var path = new StringBuilder(32768);
            ((IShellLinkW)link).GetPath(path, path.Capacity, IntPtr.Zero, 0);
            return path.ToString();
        } finally {
            Marshal.FinalReleaseComObject(link);
        }
    }
}
'@

function Install-Checked([string]$LogName) {
    $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-', '/TASKS=desktopicon',
        "/DIR=`"$installed`"", "/LOG=`"$(Join-Path $logs $LogName)`"")
    $process = Start-Process -FilePath $setup -ArgumentList $arguments -Wait -PassThru
    if ($process.ExitCode -ne 0) { throw "Installer failed: $($process.ExitCode)" }
    foreach ($file in $record.payload_sha256.PSObject.Properties) {
        $path = Join-Path $installed $file.Name
        if ((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash.ToLowerInvariant() -ne $file.Value) {
            throw "Installed bytes differ: $($file.Name)"
        }
    }
    $registration = Get-ItemProperty -Path $registry
    if ($registration.DisplayVersion -ne $record.source_build.version) { throw 'Wrong registered version' }
    foreach ($link in @($startLink, $desktopLink)) {
        if (!(Test-Path $link)) { throw "Missing shortcut: $link" }
        $target = [InstallerShortcut]::Target($link)
        $expected = Join-Path $installed 'rexafs.exe'
        Copy-Item -LiteralPath $link -Destination (Join-Path $logs ((Split-Path $link -Leaf) + '-' + (Split-Path (Split-Path $link -Parent) -Leaf) + '.lnk'))
        if ($target -ne $expected) {
            throw "Shortcut $link points to '$target', expected '$expected'"
        }
    }
}

function Get-PeMachine([string]$Path) {
    $stream = [IO.File]::OpenRead($Path)
    $reader = [IO.BinaryReader]::new($stream)
    try {
        if ($stream.Length -lt 64 -or $reader.ReadUInt16() -ne 0x5A4D) {
            throw "Invalid DOS header: $Path"
        }
        $stream.Position = 60
        $offset = $reader.ReadUInt32()
        if ($offset -lt 64 -or $offset + 24 -gt $stream.Length) {
            throw "Invalid PE header offset: $Path"
        }
        $stream.Position = $offset
        if ($reader.ReadUInt32() -ne 0x00004550) { throw "Invalid PE signature: $Path" }
        $reader.ReadUInt16()
    } finally {
        $reader.Dispose()
    }
}

Install-Checked 'install.log'
$sentinel = Join-Path $installed 'user-project.rxs'
$sentinelText = 'User-created project must survive reinstall and uninstall.'
Set-Content -LiteralPath $sentinel -Value $sentinelText -NoNewline
Install-Checked 'reinstall.log'
if ((Get-Content -Raw -LiteralPath $sentinel) -ne $sentinelText) { throw 'Reinstall changed user data' }

# Exercise the actual update helper against this disposable registered install,
# including a deliberately mismatched payload that must roll Setup back.
# release-build.yml requires this check; the separate historical installer job
# stages a previously published binary that predates the updater.
if ($CheckUpdates) {
    $bundle = Get-ChildItem -LiteralPath $Directory -Directory -Filter 'rexafs-*-pc-windows-msvc'
    if (@($bundle).Count -ne 1) { throw 'Expected one Windows source bundle' }
    uv run --no-project python scripts/smoke-desktop-updater.py $bundle.FullName --installed $installed --setup $setup
    if ($LASTEXITCODE -ne 0) { throw 'Windows one-button updater qualification failed' }
    if ((Get-ItemProperty -Path $registry).DisplayVersion -ne $record.source_build.version) { throw 'Updater lost the installed version registration' }
    foreach ($link in @($startLink, $desktopLink)) {
        if ([InstallerShortcut]::Target($link) -ne (Join-Path $installed 'rexafs.exe')) { throw 'Updater changed a shortcut target' }
    }
}

$executable = Join-Path $installed 'rexafs.exe'
if ((Get-PeMachine $executable) -ne $targetPolicy.Machine) {
    throw 'Installed desktop architecture does not match its target.'
}
if ($record.source_build.features -contains 'feff10-runner') {
    $helper = Join-Path $installed 'resources/feff10/feff10-rs.exe'
    if ((Get-PeMachine $helper) -ne 0x8664) { throw 'Expected the bundled x64 FEFF10 helper.' }
    $expectedRuntime = if ($hostArchitecture -eq 'Arm64') { 'windows-x64-emulation' } else { 'native' }
    if ($record.engine_execution.feff10.runtime -ne $expectedRuntime -or
        $record.engine_execution.feff10.target -ne 'x86_64-pc-windows-gnu') {
        throw 'FEFF10 helper execution metadata does not match its architecture.'
    }
}
function Invoke-InstalledCheck([string]$flag) {
    # PowerShell may launch a GUI-subsystem EXE asynchronously, even when its
    # output is assigned to a variable. Wait explicitly and retain both streams.
    $name = $flag.TrimStart('-')
    $stdout = Join-Path $logs "$name.stdout.log"
    $stderr = Join-Path $logs "$name.stderr.log"
    $check = Start-Process -FilePath $executable -ArgumentList $flag -WindowStyle Hidden `
        -RedirectStandardOutput $stdout -RedirectStandardError $stderr -Wait -PassThru
    if ($check.ExitCode -ne 0) {
        throw "Installed $flag failed ($($check.ExitCode)): $(Get-Content -Raw -LiteralPath $stderr)"
    }
    Get-Content -LiteralPath $stdout
}
Push-Location $env:RUNNER_TEMP
try {
    $identityText = Invoke-InstalledCheck '--build-info'
    $identity = ($identityText -join "`n") | ConvertFrom-Json
    foreach ($key in @('version', 'commit', 'channel', 'release_tag')) {
        if ($identity.$key -ne $record.source_build.$key) { throw "Wrong installed $key" }
    }
    $versionText = Invoke-InstalledCheck '--version'
    if (-not ($versionText -match '^rexafs ')) { throw 'Installed --version returned no version' }
    $selfCheckText = Invoke-InstalledCheck '--self-check'
    if (-not ($selfCheckText -match 'package check passed')) { throw 'Installed package check returned no result' }
    $feffCheckText = Invoke-InstalledCheck '--self-check-feff'
    Write-Output $versionText
    Write-Output $selfCheckText
    Write-Output $feffCheckText
} finally {
    Pop-Location
}

$uninstallLog = Join-Path $logs 'uninstall.log'
$uninstall = Start-Process -FilePath (Join-Path $installed 'unins000.exe') -ArgumentList @(
    '/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', "/LOG=`"$uninstallLog`""
) -Wait -PassThru
if ($uninstall.ExitCode -ne 0) { throw "Uninstall failed: $($uninstall.ExitCode)" }
if ((Test-Path $executable) -or (Test-Path $registry) -or (Test-Path $startLink) -or (Test-Path $desktopLink)) {
    throw 'Uninstall left program files, registration or shortcuts'
}
if ((Get-Content -Raw -LiteralPath $sentinel) -ne $sentinelText) { throw 'Uninstall changed user data' }
$qualification = [ordered]@{
    installer = $record.installer
    sha256 = $record.sha256
    source_commit = $record.source_build.commit
    packaging_commit = $record.packaging_commit
    target = $record.source_build.target
    host_architecture = $hostArchitecture
    desktop_pe_machine = ('0x{0:X4}' -f $targetPolicy.Machine)
    engine_execution = $record.engine_execution
    run_id = $env:GITHUB_RUN_ID
    checks = @('install', 'all-payload-hashes', 'start-menu-shortcut', 'desktop-shortcut',
        'user-uninstall-registration', 'reinstall', 'build-identity', 'packaged-example',
        'embedded-feff-runner', 'native-desktop-architecture', 'engine-architecture',
        'uninstall', 'preserve-user-project', 'unicode-install-path')
    update_checks = [bool]$CheckUpdates
    interactive_gui = 'not tested on hosted runner'
    signed = $false
}
$qualification | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $Directory ($record.installer + '.qualification.json'))
Write-Output 'Windows installer: install, reinstall, installed checks and uninstall passed.'
