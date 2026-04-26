#!/usr/bin/env node

import { spawnSync } from "node:child_process"

const rootDir = process.cwd()

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    shell: false
  })

  if (typeof result.status === "number") {
    process.exit(result.status)
  }

  process.exit(1)
}

switch (process.platform) {
  case "darwin":
    run("bash", ["scripts/setup-macos.sh"])
    break
  case "win32":
    run("powershell", ["-ExecutionPolicy", "Bypass", "-File", "scripts/setup-windows.ps1"])
    break
  default:
    console.error(`Unsupported platform for automatic setup: ${process.platform}`)
    process.exit(1)
}
