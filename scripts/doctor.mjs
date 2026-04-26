#!/usr/bin/env node

import { existsSync } from "node:fs"
import { execFileSync } from "node:child_process"
import path from "node:path"

const rootDir = process.cwd()
const isWindows = process.platform === "win32"
const wantsJson = process.argv.includes("--json")
const results = []

function run(command, args = []) {
  try {
    const stdout = execFileSync(command, args, {
      cwd: rootDir,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"]
    }).trim()
    return { ok: true, output: stdout }
  } catch (error) {
    const message =
      error instanceof Error && "stderr" in error
        ? String(error.stderr || error.message).trim()
        : error instanceof Error
          ? error.message
          : String(error)
    return { ok: false, output: message || "command failed" }
  }
}

function commandExists(command) {
  const lookup = isWindows ? run("where", [command]) : run("which", [command])
  return lookup.ok
}

function addCheck(name, { required, ok, detail, hint }) {
  results.push({ name, required, ok, detail, hint: ok ? null : hint })
}

function versionCheck(name, command, args = ["--version"], required = true) {
  const result = run(command, args)
  addCheck(name, {
    required,
    ok: result.ok,
    detail: result.ok ? firstLine(result.output) : "not found",
    hint: result.ok ? null : `Install ${name} and rerun setup.`
  })
}

function firstLine(text) {
  return text.split(/\r?\n/, 1)[0] || ""
}

function pathCheck(name, relativePath, required, hint) {
  const fullPath = path.join(rootDir, relativePath)
  const ok = existsSync(fullPath)
  addCheck(name, {
    required,
    ok,
    detail: relativePath,
    hint: ok ? null : hint
  })
}

function powershell(command) {
  return run("powershell", ["-NoProfile", "-Command", command])
}

function detectVsWhere() {
  const candidates = [
    process.env.ProgramFilesX86
      ? path.join(process.env.ProgramFilesX86, "Microsoft Visual Studio", "Installer", "vswhere.exe")
      : null,
    process.env.ProgramFiles
      ? path.join(process.env.ProgramFiles, "Microsoft Visual Studio", "Installer", "vswhere.exe")
      : null
  ].filter(Boolean)

  return candidates.find((candidate) => existsSync(candidate)) ?? null
}

function detectWindowsVcTools() {
  const vswhere = detectVsWhere()
  if (!vswhere) {
    return { ok: false, detail: "vswhere.exe not found" }
  }

  const query = run(vswhere, [
    "-latest",
    "-products",
    "*",
    "-requires",
    "Microsoft.VisualStudio.Workload.VCTools",
    "-property",
    "installationPath"
  ])

  if (!query.ok) {
    return { ok: false, detail: "Desktop development with C++ workload not found" }
  }

  const installationPath = firstLine(query.output)
  return installationPath
    ? { ok: true, detail: installationPath }
    : { ok: false, detail: "Desktop development with C++ workload not found" }
}

function detectWebView2() {
  const command = [
    "$paths = @(",
    "'HKLM:\\SOFTWARE\\WOW6432Node\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',",
    "'HKCU:\\Software\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}',",
    "'HKLM:\\SOFTWARE\\Microsoft\\EdgeUpdate\\Clients\\{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'",
    ");",
    "$versions = foreach ($path in $paths) {",
    "  if (Test-Path $path) {",
    "    $value = (Get-ItemProperty -Path $path -Name pv -ErrorAction SilentlyContinue).pv;",
    "    if ($value -and $value -ne '0.0.0.0') { $value }",
    "  }",
    "};",
    "if ($versions) { $versions | Select-Object -First 1 }"
  ].join(" ")

  const result = powershell(command)
  if (!result.ok || !result.output) {
    return { ok: false, detail: "not installed" }
  }

  return { ok: true, detail: firstLine(result.output) }
}

versionCheck("node", "node")
versionCheck("npm", "npm")
versionCheck("rustc", "rustc")
versionCheck("cargo", "cargo")

if (isWindows) {
  const rustup = run("rustup", ["show", "active-toolchain"])
  addCheck("rustup active toolchain", {
    required: true,
    ok: rustup.ok && /msvc/i.test(rustup.output),
    detail: rustup.ok ? firstLine(rustup.output) : "not available",
    hint: "Run `rustup default stable-msvc`."
  })
}

pathCheck("node_modules", "node_modules", true, "Run `npm install`.")
pathCheck(
  "tauri config",
  "apps/desktop/src-tauri/tauri.conf.json",
  true,
  "The desktop app is missing its Tauri config."
)

if (process.platform === "darwin") {
  const xcode = run("xcode-select", ["-p"])
  addCheck("xcode command line tools", {
    required: true,
    ok: xcode.ok,
    detail: xcode.ok ? firstLine(xcode.output) : "not installed",
    hint: xcode.ok ? null : "Run `xcode-select --install`."
  })

  addCheck("homebrew", {
    required: false,
    ok:
      commandExists("brew") ||
      existsSync("/opt/homebrew/bin/brew") ||
      existsSync("/usr/local/bin/brew"),
    detail: "optional but recommended for bootstrap flows",
    hint: "Install Homebrew to simplify local setup."
  })
}

if (isWindows) {
  addCheck("winget", {
    required: true,
    ok: commandExists("winget"),
    detail: commandExists("winget") ? "detected" : "not detected",
    hint:
      "Sign in once and let App Installer register WinGet, or run Add-AppxPackage registration."
  })

  const vcTools = detectWindowsVcTools()
  addCheck("msvc build tools", {
    required: true,
    ok: vcTools.ok,
    detail: vcTools.detail,
    hint:
      "Install Visual Studio Build Tools with the Desktop development with C++ workload."
  })

  const webview2 = detectWebView2()
  addCheck("webview2 runtime", {
    required: true,
    ok: webview2.ok,
    detail: webview2.detail,
    hint: "Install the Evergreen WebView2 Runtime."
  })
}

for (const provider of ["gh", "claude", "codex"]) {
  const ok = commandExists(provider)
  addCheck(`${provider} cli`, {
    required: false,
    ok,
    detail: ok ? "detected" : "not installed",
    hint:
      "Optional at repo setup time. The app bootstrap can install it later if the platform supports it."
  })
}

const failingRequired = results.filter((item) => item.required && !item.ok)
const failingOptional = results.filter((item) => !item.required && !item.ok)

if (wantsJson) {
  console.log(
    JSON.stringify(
      {
        platform: process.platform,
        ok: failingRequired.length === 0,
        checks: results,
        failingRequired: failingRequired.map((item) => item.name),
        failingOptional: failingOptional.map((item) => item.name)
      },
      null,
      2
    )
  )
  process.exit(failingRequired.length === 0 ? 0 : 1)
}

console.log("")
console.log("AI Usage Dashboard doctor")
console.log(`platform: ${process.platform}`)
console.log("")

for (const item of results) {
  const status = item.ok ? "OK " : item.required ? "ERR" : "WARN"
  console.log(`${status}  ${item.name}`)
  console.log(`     ${item.detail}`)
  if (!item.ok && item.hint) {
    console.log(`     hint: ${item.hint}`)
  }
}

console.log("")
console.log(
  `summary: ${results.length - failingRequired.length - failingOptional.length}/${results.length} checks passed`
)

if (failingOptional.length > 0) {
  console.log(`optional warnings: ${failingOptional.length}`)
}

if (failingRequired.length > 0) {
  console.error(`required failures: ${failingRequired.length}`)
  process.exit(1)
}
