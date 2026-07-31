# GravixLayer CLI installer for Windows (PowerShell)
# Usage: irm 'https://cli.gravixlayer.ai/install.ps1' | iex
#
# Downloads the correct pre-built binary from GitHub Releases, verifies the
# SHA-256 checksum, installs to %LOCALAPPDATA%\gravixlayer\bin (added to the
# user PATH), and creates a grx.exe alias alongside gravixlayer.exe.
#
# Environment overrides (set before running):
#   $env:GRAVIXLAYER_VERSION     — install a specific version tag (e.g. v0.3.0)
#   $env:GRAVIXLAYER_INSTALL_DIR — override the installation directory
#   $env:GRAVIXLAYER_NO_VERIFY   — set to any non-empty value to skip checksum

[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'

$Repo       = 'gravixlayer/gravixlayer-cli'
$BinaryName = 'gravixlayer'
$SymlinkName = 'grx'

# ---------------------------------------------------------------------------
# Helper functions
# ---------------------------------------------------------------------------

function Write-Info  { param($Msg) Write-Host "[info]  $Msg" -ForegroundColor Cyan }
function Write-Ok    { param($Msg) Write-Host "[ok]    $Msg" -ForegroundColor Green }
function Write-Warn  { param($Msg) Write-Host "[warn]  $Msg" -ForegroundColor Yellow }
function Write-Fail  { param($Msg) Write-Error "[error] $Msg" }

# ---------------------------------------------------------------------------
# Detect architecture
# ---------------------------------------------------------------------------

function Get-Platform {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    switch ($arch) {
        'X64'   { return 'x86_64-pc-windows-msvc' }
        'Arm64' { return 'aarch64-pc-windows-msvc' }
        default { Write-Fail "Unsupported Windows architecture: $arch" }
    }
}

# ---------------------------------------------------------------------------
# Resolve version
# ---------------------------------------------------------------------------

function Resolve-CliVersion {
    $envVersion = $env:GRAVIXLAYER_VERSION
    if ($envVersion) {
        if ($envVersion -notmatch '^v') { return "v$envVersion" }
        return $envVersion
    }

    $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"
    try {
        $response = Invoke-RestMethod -Uri $apiUrl -UseBasicParsing -Headers @{
            'User-Agent' = 'gravixlayer-installer/1.0'
            'Accept'     = 'application/vnd.github+json'
        }
        if ($response.tag_name) { return $response.tag_name }
    } catch {
        Write-Fail "Failed to fetch the latest release version: $_"
    }
    Write-Fail "Could not determine the latest release version"
}

# ---------------------------------------------------------------------------
# Download with progress
# ---------------------------------------------------------------------------

function Download-File {
    param(
        [string]$Url,
        [string]$Destination
    )
    Write-Info "Downloading $Url"
    $wc = New-Object System.Net.WebClient
    $wc.Headers.Add('User-Agent', 'gravixlayer-installer/1.0')
    $wc.DownloadFile($Url, $Destination)
}

# ---------------------------------------------------------------------------
# Checksum verification
# ---------------------------------------------------------------------------

function Verify-Checksum {
    param(
        [string]$FilePath,
        [string]$ExpectedHash
    )
    if ($env:GRAVIXLAYER_NO_VERIFY) {
        Write-Warn "Checksum verification skipped"
        return
    }
    $actual = (Get-FileHash -Path $FilePath -Algorithm SHA256).Hash.ToLower()
    $expected = $ExpectedHash.Trim().ToLower() -replace '\s.*', ''  # remove filename suffix if present
    if ($actual -ne $expected) {
        Write-Fail "Checksum mismatch!`n  expected: $expected`n  actual:   $actual"
    }
    Write-Ok "Checksum verified"
}

# ---------------------------------------------------------------------------
# Add directory to user PATH if not already present
# ---------------------------------------------------------------------------

function Add-ToUserPath {
    param([string]$Dir)
    $currentPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if ($currentPath -split ';' -notcontains $Dir) {
        [Environment]::SetEnvironmentVariable('Path', "$currentPath;$Dir", 'User')
        Write-Info "Added $Dir to user PATH (restart your shell to apply)"
    }
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

function Install-GravixLayerCli {
    $platform = Get-Platform
    $version  = Resolve-CliVersion

    Write-Info "Installing gravixlayer $version for $platform"

    $releasesBase = "https://github.com/$Repo/releases/download/$version"
    $tarballName  = "$BinaryName-$version-$platform.zip"
    $tarballUrl   = "$releasesBase/$tarballName"
    $checksumUrl  = "$tarballUrl.sha256"

    $installDir = if ($env:GRAVIXLAYER_INSTALL_DIR) {
        $env:GRAVIXLAYER_INSTALL_DIR
    } else {
        Join-Path $env:LOCALAPPDATA 'gravixlayer\bin'
    }

    # Create install dir
    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    $tmpDir     = Join-Path ([System.IO.Path]::GetTempPath()) "gravixlayer-install-$([System.Guid]::NewGuid())"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        $zipPath      = Join-Path $tmpDir $tarballName
        $checksumPath = Join-Path $tmpDir "$tarballName.sha256"

        Download-File -Url $tarballUrl -Destination $zipPath

        # Checksum is mandatory unless GRAVIXLAYER_NO_VERIFY is set.
        if ($env:GRAVIXLAYER_NO_VERIFY) {
            Write-Warn "Checksum verification skipped (GRAVIXLAYER_NO_VERIFY is set)"
        } else {
            try {
                Download-File -Url $checksumUrl -Destination $checksumPath
            } catch {
                Write-Fail "Checksum file not available at $checksumUrl. Refusing to install without verification. Set GRAVIXLAYER_NO_VERIFY=1 to override (not recommended)."
            }
            $expectedHash = Get-Content $checksumPath -Raw
            Verify-Checksum -FilePath $zipPath -ExpectedHash $expectedHash
        }

        # Extract
        Write-Info "Extracting archive"
        Expand-Archive -Path $zipPath -DestinationPath $tmpDir -Force

        $binaryPath = Join-Path $tmpDir "$BinaryName.exe"
        if (-not (Test-Path $binaryPath)) {
            Write-Fail "Binary not found in archive: $BinaryName.exe"
        }

        # Install main binary
        $destBinary = Join-Path $installDir "$BinaryName.exe"
        Copy-Item -Path $binaryPath -Destination $destBinary -Force

        # Create grx.exe alias (copy — symlinks require elevation on Windows)
        $destAlias = Join-Path $installDir "$SymlinkName.exe"
        Copy-Item -Path $binaryPath -Destination $destAlias -Force

        Write-Ok "gravixlayer $version installed to $destBinary"
        Write-Ok "grx.exe alias created at $destAlias"

        # Update user PATH
        Add-ToUserPath -Dir $installDir

        # Verify
        try {
            $ver = & $destBinary --version 2>&1
            Write-Info "Installed version: $ver"
        } catch {}

        Write-Host ""
        Write-Host "Get started:" -ForegroundColor White
        Write-Host "  gravixlayer auth login        # save your API key"
        Write-Host "  gravixlayer doctor            # verify local install"
        Write-Host "  gravixlayer runtime create    # spin up a sandbox"
        Write-Host "  gravixlayer --help"
        Write-Host ""
        Write-Host "Documentation: https://docs.gravixlayer.ai"
    } finally {
        Remove-Item -Recurse -Force -ErrorAction SilentlyContinue $tmpDir
    }
}

Install-GravixLayerCli
