param([Parameter(Mandatory = $true)][string]$Directory)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') {
    throw 'This install/uninstall smoke test is intended for an ephemeral GitHub Actions runner.'
}
$metadataFiles = @(Get-ChildItem -LiteralPath $Directory -Filter '*-setup.build.json')
if ($metadataFiles.Count -ne 1) { throw 'Expected one installer build record' }
$record = Get-Content -Raw -LiteralPath $metadataFiles[0].FullName | ConvertFrom-Json
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
    $shell = New-Object -ComObject WScript.Shell
    foreach ($link in @($startLink, $desktopLink)) {
        if (!(Test-Path $link)) { throw "Missing shortcut: $link" }
        $shortcut = $shell.CreateShortcut($link)
        $expected = Join-Path $installed 'rexafs.exe'
        Copy-Item -LiteralPath $link -Destination (Join-Path $logs ((Split-Path $link -Leaf) + '-' + (Split-Path (Split-Path $link -Parent) -Leaf) + '.lnk'))
        if ($shortcut.TargetPath -ne $expected) {
            throw "Shortcut $link points to '$($shortcut.TargetPath)', expected '$expected'"
        }
    }
}

Install-Checked 'install.log'
$sentinel = Join-Path $installed 'user-project.rxs'
$sentinelText = 'User-created project must survive reinstall and uninstall.'
Set-Content -LiteralPath $sentinel -Value $sentinelText -NoNewline
Install-Checked 'reinstall.log'
if ((Get-Content -Raw -LiteralPath $sentinel) -ne $sentinelText) { throw 'Reinstall changed user data' }

$executable = Join-Path $installed 'rexafs.exe'
Push-Location $env:RUNNER_TEMP
try {
    $identityText = & $executable --build-info
    if ($LASTEXITCODE -ne 0) { throw 'Installed build identity check failed' }
    $identity = ($identityText -join "`n") | ConvertFrom-Json
    foreach ($key in @('version', 'commit', 'channel', 'release_tag')) {
        if ($identity.$key -ne $record.source_build.$key) { throw "Wrong installed $key" }
    }
    & $executable --version
    if ($LASTEXITCODE -ne 0) { throw 'Installed --version failed' }
    & $executable --self-check
    if ($LASTEXITCODE -ne 0) { throw 'Installed packaged-data check failed' }
    & $executable --self-check-feff
    if ($LASTEXITCODE -ne 0) { throw 'Installed FEFF runner check failed' }
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
    run_id = $env:GITHUB_RUN_ID
    checks = @('install', 'all-payload-hashes', 'start-menu-shortcut', 'desktop-shortcut',
        'user-uninstall-registration', 'reinstall', 'build-identity', 'packaged-example',
        'embedded-feff-runner', 'uninstall', 'preserve-user-project', 'unicode-install-path')
    interactive_gui = 'not tested on hosted runner'
    signed = $false
}
$qualification | ConvertTo-Json -Depth 5 | Set-Content -Encoding utf8 -LiteralPath (Join-Path $Directory ($record.installer + '.qualification.json'))
Write-Output 'Windows installer: install, reinstall, installed checks and uninstall passed.'
