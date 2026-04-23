import type { MetricLine, ProviderSnapshotState } from "@ai-usage-dashboard/core"

let permissionDenied = false

export function computeProviderMax(state: ProviderSnapshotState): number | null {
  if (!state.snapshot) return null
  const ratios: number[] = []
  for (const line of state.snapshot.lines) {
    if (line.type !== "progress") continue
    if (line.limit <= 0) continue
    ratios.push(line.used / line.limit)
  }
  if (ratios.length === 0) return null
  return Math.max(...ratios)
}

export function findProviderMaxProgressLine(
  state: ProviderSnapshotState
): Extract<MetricLine, { type: "progress" }> | null {
  if (!state.snapshot) return null

  let maxLine: Extract<MetricLine, { type: "progress" }> | null = null
  let maxRatio = -1

  for (const line of state.snapshot.lines) {
    if (line.type !== "progress") continue
    if (line.limit <= 0) continue

    const ratio = line.used / line.limit
    if (ratio > maxRatio) {
      maxRatio = ratio
      maxLine = line
    }
  }

  return maxLine
}

export async function fireThresholdNotification(title: string, body: string) {
  if (!(globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return
  if (permissionDenied) return

  const mod = await import("@tauri-apps/plugin-notification")
  const { isPermissionGranted, requestPermission, sendNotification } = mod

  let granted = await isPermissionGranted()
  if (!granted) {
    const perm = await requestPermission()
    granted = perm === "granted"
    if (perm === "denied") {
      permissionDenied = true
      console.warn("notification permission denied")
    }
  }

  if (!granted) return

  sendNotification({ title, body })
}
