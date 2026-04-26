$ErrorActionPreference = "Stop"

$RootDir = Split-Path -Parent $PSScriptRoot
Set-Location $RootDir

$VsBuildToolsBootstrapper = "https://aka.ms/vs/17/release/vs_buildtools.exe"
$WebView2PackageId = "Microsoft.EdgeWebView2Runtime"
$NodePackageId = "OpenJS.NodeJS.LTS"
$GhPackageId = "GitHub.cli"
$RustupPackageId = "Rustlang.Rustup"
$VsVcToolsWorkload = "Microsoft.VisualStudio.Workload.VCTools"

function Write-Step {
  param([string]$Message)
  Write-Host ""
  Write-Host "[setup-windows] $Message"
}

function Test-Command {
  param([string]$Name)
  $null -ne (Get-Command $Name -ErrorAction SilentlyContinue)
}

function Test-IsAdministrator {
  $identity = [Security.Principal.WindowsIdentity]::GetCurrent()
  $principal = New-Object Security.Principal.WindowsPrincipal($identity)
  return $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
}

function Ensure-Elevated {
  if (Test-IsAdministrator) {
    return
  }

  Write-Step "Restarting setup with administrator rights"
  Start-Process powershell -Verb RunAs -ArgumentList @(
    "-ExecutionPolicy", "Bypass",
    "-File", "`"$PSCommandPath`""
  ) | Out-Null
  exit 0
}

function Add-ToPathIfPresent {
  param([string]$Candidate)
  if (Test-Path $Candidate) {
    if (-not ($env:Path -split ";" | Where-Object { $_ -eq $Candidate })) {
      $env:Path = "$Candidate;$env:Path"
    }
  }
}

function Ensure-WinGet {
  if (Test-Command "winget") {
    return
  }

  Write-Step "Registering App Installer so WinGet becomes available"
  Add-AppxPackage -RegisterByFamilyName -MainPackage Microsoft.DesktopAppInstaller_8wekyb3d8bbwe

  if (-not (Test-Command "winget")) {
    throw "winget is required for automatic Windows setup."
  }
}

function Invoke-WinGetInstall {
  param(
    [string]$PackageId,
    [string[]]$ExtraArgs = @()
  )

  $arguments = @(
    "install",
    "--id", $PackageId,
    "-e",
    "--accept-package-agreements",
    "--accept-source-agreements",
    "--disable-interactivity"
  ) + $ExtraArgs

  & winget @arguments
}

function Get-VSWherePath {
  $candidates = @(
    "$env:ProgramFiles(x86)\Microsoft Visual Studio\Installer\vswhere.exe",
    "$env:ProgramFiles\Microsoft Visual Studio\Installer\vswhere.exe"
  )

  foreach ($candidate in $candidates) {
    if ($candidate -and (Test-Path $candidate)) {
      return $candidate
    }
  }

  return $null
}

function Test-VcToolsInstalled {
  $vswhere = Get-VSWherePath
  if (-not $vswhere) {
    return $false
  }

  $installationPath = & $vswhere -latest -products * -requires $VsVcToolsWorkload -property installationPath
  return -not [string]::IsNullOrWhiteSpace(($installationPath | Select-Object -First 1))
}

function Test-WebView2Installed {
  $registryPaths = @(
    "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKCU:\Software\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}",
    "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}"
  )

  foreach ($registryPath in $registryPaths) {
    if (Test-Path $registryPath) {
      $version = (Get-ItemProperty -Path $registryPath -Name pv -ErrorAction SilentlyContinue).pv
      if ($version -and $version -ne "0.0.0.0") {
        return $true
      }
    }
  }

  return $false
}

function Install-VcTools {
  if (Test-VcToolsInstalled) {
    return
  }

  Write-Step "Installing Visual Studio Build Tools with Desktop development with C++"
  $bootstrapper = Join-Path $env:TEMP "vs_buildtools.exe"
  Invoke-WebRequest -Uri $VsBuildToolsBootstrapper -OutFile $bootstrapper
  & $bootstrapper `
    --quiet `
    --wait `
    --norestart `
    --nocache `
    --add $VsVcToolsWorkload `
    --includeRecommended

  if (-not (Test-VcToolsInstalled)) {
    throw "Visual Studio Build Tools installation did not complete successfully."
  }
}

function Install-WebView2 {
  if (Test-WebView2Installed) {
    return
  }

  Write-Step "Installing WebView2 Runtime"
  Invoke-WinGetInstall -PackageId $WebView2PackageId

  if (-not (Test-WebView2Installed)) {
    throw "WebView2 Runtime installation did not complete successfully."
  }
}

function Install-Node {
  if (Test-Command "node" -and Test-Command "npm") {
    return
  }

  Write-Step "Installing Node.js LTS"
  Invoke-WinGetInstall -PackageId $NodePackageId
  Refresh-ToolPaths

  if (-not (Test-Command "node") -or -not (Test-Command "npm")) {
    throw "Node.js installation did not become available on PATH."
  }
}

function Install-Rust {
  if (Test-Command "cargo" -and Test-Command "rustc" -and Test-Command "rustup") {
    rustup default stable-msvc | Out-Null
    return
  }

  Write-Step "Installing Rust with rustup"
  Invoke-WinGetInstall -PackageId $RustupPackageId
  Refresh-ToolPaths

  if (-not (Test-Command "cargo") -or -not (Test-Command "rustc") -or -not (Test-Command "rustup")) {
    throw "Rust installation did not become available on PATH."
  }

  rustup default stable-msvc | Out-Null
}

function Install-GhCli {
  if (Test-Command "gh") {
    return
  }

  Write-Step "Installing GitHub CLI"
  Invoke-WinGetInstall -PackageId $GhPackageId
  Refresh-ToolPaths
}

function Install-ClaudeCli {
  if (Test-Command "claude") {
    return
  }

  Write-Step "Installing Claude Code CLI"
  powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://claude.ai/install.ps1 | iex"
  Refresh-ToolPaths
}

function Install-CodexCli {
  if (Test-Command "codex") {
    return
  }

  Write-Step "Installing Codex CLI"
  & npm install -g @openai/codex
  Refresh-ToolPaths
}

function Refresh-ToolPaths {
  Add-ToPathIfPresent "$env:ProgramFiles\nodejs"
  Add-ToPathIfPresent "$env:LOCALAPPDATA\Programs\nodejs"
  Add-ToPathIfPresent "$env:APPDATA\npm"
  Add-ToPathIfPresent "$env:USERPROFILE\.cargo\bin"
}

Ensure-Elevated
Ensure-WinGet
Refresh-ToolPaths

Install-VcTools
Install-WebView2
Install-Node
Install-Rust
Install-GhCli
Install-ClaudeCli
Install-CodexCli

Write-Step "Installing npm workspace dependencies"
npm install

Write-Step "Running doctor"
npm run doctor

Write-Step "Running smoke checks"
npm run smoke

Write-Step "Setup complete. Next command: npm run dev:tauri"
