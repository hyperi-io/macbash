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

$asset = "$binary-windows-$arch.exe"
$url   = "$downloadBase/$versionPath/$asset"
$sumsUrl = "$downloadBase/$versionPath/checksums.sha256"

$tmpDir = New-Item -ItemType Directory -Path (Join-Path $env:TEMP ("$binary-install-" + [Guid]::NewGuid().Guid))
try {
    $tmpBinary = Join-Path $tmpDir "$binary.exe"
    Write-Host "Downloading $binary $versionPath (windows-$arch)..."
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $tmpBinary
    } catch {
        Write-Error "Download failed: $url`n$_"
        exit 1
    }

    # Checksum verify if available
    $sumsFile = Join-Path $tmpDir 'checksums.sha256'
    try {
        Invoke-WebRequest -UseBasicParsing -Uri $sumsUrl -OutFile $sumsFile -ErrorAction Stop
        $line = Select-String -Path $sumsFile -Pattern ([regex]::Escape($asset)) | Select-Object -First 1
        if ($line) {
            $expected = ($line.Line -split '\s+')[0]
            $actual = (Get-FileHash -Algorithm SHA256 -Path $tmpBinary).Hash.ToLower()
            if ($expected.ToLower() -ne $actual) {
                Write-Error "Checksum mismatch for $asset`n  expected: $expected`n  actual:   $actual"
                exit 1
            }
            Write-Host "Checksum verified."
        }
    } catch {
        # checksums file is optional
    }

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
