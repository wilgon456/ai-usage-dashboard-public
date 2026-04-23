import type { ProviderDefinition, ProviderId, ProviderSnapshotState } from "@ai-usage-dashboard/core"
import { computeProviderMax } from "./notifications"
import { providerLogo } from "./provider-logos"

type TrayTarget = "max" | "last-viewed" | ProviderId

let lastKey: string | null = null
const imageCache = new Map<string, HTMLImageElement>()

async function loadLogo(id: ProviderDefinition["id"]): Promise<HTMLImageElement> {
  const cached = imageCache.get(id)
  if (cached) return cached

  const img = new Image()
  img.src = providerLogo[id]
  await img.decode()
  imageCache.set(id, img)
  return img
}

function resolveTarget(
  states: ProviderSnapshotState[],
  trayTarget: TrayTarget,
  lastViewedProviderId: ProviderId | null
): { state: ProviderSnapshotState; pct: number } | null {
  const pickMax = () => {
    let best: { state: ProviderSnapshotState; pct: number } | null = null
    for (const state of states) {
      const max = computeProviderMax(state)
      if (max == null) continue
      const pct = Math.round(max * 100)
      if (!best || pct > best.pct) best = { state, pct }
    }
    return best
  }

  const pickById = (id: ProviderId) => {
    const state = states.find((candidate) => candidate.provider.id === id)
    if (!state) return null
    const max = computeProviderMax(state)
    return { state, pct: max == null ? 0 : Math.round(max * 100) }
  }

  if (trayTarget === "max") return pickMax()
  if (trayTarget === "last-viewed") {
    return (lastViewedProviderId ? pickById(lastViewedProviderId) : null) ?? pickMax()
  }
  return pickById(trayTarget) ?? pickMax()
}

export async function syncTrayIcon(
  states: ProviderSnapshotState[],
  trayTarget: TrayTarget,
  lastViewedProviderId: ProviderId | null = null
): Promise<void> {
  if (!(globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) return
  if (states.length === 0) return

  const resolved = resolveTarget(states, trayTarget, lastViewedProviderId)
  if (!resolved) return
  const { state: target, pct } = resolved

  const key = `${target.provider.id}:${pct}`
  if (key === lastKey) return

  const logo = await loadLogo(target.provider.id)
  const canvas = new OffscreenCanvas(64, 64)
  const ctx = canvas.getContext("2d")
  if (!ctx) return

  ctx.clearRect(0, 0, 64, 64)
  ctx.drawImage(logo, 8, 8, 48, 48)
  ctx.globalCompositeOperation = "source-in"
  ctx.fillStyle = "#ffffff"
  ctx.fillRect(0, 0, 64, 64)
  ctx.globalCompositeOperation = "source-over"

  const blob = await canvas.convertToBlob({ type: "image/png" })
  const buf = new Uint8Array(await blob.arrayBuffer())

  try {
    const core = await import("@tauri-apps/api/core")
    await core.invoke("set_tray_icon", {
      bytes: Array.from(buf),
      label: `${pct}%`
    })
    lastKey = key
  } catch {
    // The tray can race app setup on boot; the next refresh retries.
  }
}
