# Project:   ${PKG_NAME}
# File:      install.ps1 (rendered)
# Purpose:   Windows PowerShell installer
# Language:  PowerShell
#
# License:   Apache-2.0
# Copyright: (c) 2025-2026 HYPERI PTY LIMITED
#
# Status:    EXPERIMENTAL — best effort. Windows is not in the formal
#            CI test path. Use at your own risk; expect rough edges.
#
# Usage:
#   irm ${PKG_DOWNLOAD_BASE}/install.ps1 | iex
#   & ([scriptblock]::Create((irm ${PKG_DOWNLOAD_BASE}/install.ps1))) -User
#   & ([scriptblock]::Create((irm ${PKG_DOWNLOAD_BASE}/install.ps1))) -Version v1.2.3

[CmdletBinding()]
param(
    [switch]$User,
    [string]$Dir,
    [string]$Version = "latest"
)

$ErrorActionPreference = 'Stop'

$binary       = '${PKG_BINARY}'
$downloadBase = '${PKG_DOWNLOAD_BASE}'
$defaultDir   = "${PKG_WINDOWS_DEFAULT_DIR}"

# Every failure throws: `exit` would close the caller's session under `irm | iex`.
function Stop-Install([string]$Message) {
    throw "install.ps1: $Message"
}

# True once $From is saved to $To. Any HTTP or network error returns false.
function Save-Download([string]$From, [string]$To) {
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $From -OutFile $To
        return $true
    } catch {
        return $false
    }
}

# The SHA-256 that sums file $Path records for exactly the file named $Name, or nothing
# when that entry is missing, malformed or listed more than once.
function Get-RecordedSha256([string]$Path, [string]$Name) {
    $hashes = @()
    foreach ($line in @(Get-Content -LiteralPath $Path)) {
        $fields = @($line.Trim() -split '\s+')
        if ($fields.Count -eq 2 -and ($fields[1] -replace '^\*', '') -ceq $Name) {
            $hashes += $fields[0]
        }
    }
    if ($hashes.Count -eq 1 -and $hashes[0] -match '^[0-9a-fA-F]{64}$') {
        return $hashes[0].ToLowerInvariant()
    }
}

Write-Host "[experimental] Windows install is not yet in the formal CI test path." -ForegroundColor Yellow

if ($Dir) {
    $installDir = $Dir
} elseif ($User) {
    $installDir = $defaultDir
} else {
    $installDir = $defaultDir
}

# Detect architecture
$arch = switch ($env:PROCESSOR_ARCHITECTURE) {
    'AMD64' { 'amd64' }
    'ARM64' { 'arm64' }
    default { throw "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE" }
}

if ($Version -eq 'latest') {
    $versionPath = 'latest'
} elseif ($Version -match '^v') {
    $versionPath = $Version
} else {
    $versionPath = "v$Version"
}

$asset        = "$binary-windows-$arch.exe"
$releaseUrl   = "$downloadBase/$versionPath"
$platformSums = "$binary-windows-$arch.sha256"

$tmpDir = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("$binary-install-" + [Guid]::NewGuid().Guid))
try {
    $tmpBinary = Join-Path $tmpDir "$binary.exe"
    Write-Host "Downloading $binary $versionPath (windows-$arch)..."
    if (-not (Save-Download "$releaseUrl/$asset" $tmpBinary)) {
        Stop-Install "download failed: $releaseUrl/$asset"
    }

    # Every release publishes <binary>-windows-<arch>.sha256 (the .exe and the .zip), and
    # checksums.sha256 is the older combined file. The first one that names this asset
    # decides, so a mismatch never falls through.
    $expected = $null
    foreach ($sums in @($platformSums, 'checksums.sha256')) {
        $sumsFile = Join-Path $tmpDir $sums
        if (Save-Download "$releaseUrl/$sums" $sumsFile) {
            $expected = Get-RecordedSha256 $sumsFile $asset
        }
        if ($expected) {
            break
        }
    }
    if (-not $expected) {
        Stop-Install "no SHA-256 for $asset in $releaseUrl/$platformSums or $releaseUrl/checksums.sha256, refusing to install an unverified binary"
    }

    $actual = (Get-FileHash -Algorithm SHA256 -LiteralPath $tmpBinary).Hash.ToLowerInvariant()
    if ($expected -cne $actual) {
        Stop-Install "checksum mismatch for $asset, refusing to install (expected: $expected, actual: $actual)"
    }
    Write-Host "Checksum verified: sha256 $actual"

    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }
    $dest = Join-Path $installDir "$binary.exe"
    Move-Item -Force $tmpBinary $dest
    Write-Host "Installed to $dest"

    # PATH advice
    $userPath = [Environment]::GetEnvironmentVariable('Path', 'User')
    if (-not ($userPath -split ';' | Where-Object { $_ -eq $installDir })) {
        Write-Host ""
        Write-Host "Note: $installDir is not in your user PATH."
        Write-Host "  Add it with:"
        Write-Host "    [Environment]::SetEnvironmentVariable('Path', `"`$env:Path;$installDir`", 'User')"
    }
} finally {
    Remove-Item -Recurse -Force $tmpDir -ErrorAction SilentlyContinue
}
