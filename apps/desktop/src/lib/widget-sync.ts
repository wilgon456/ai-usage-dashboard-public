import type { AppSettings, ProviderSnapshotState } from "@ai-usage-dashboard/core"
import { toCardState } from "./card-state"
import { formatLineValue } from "./format"
import { computeProviderMax } from "./notifications"

export interface WidgetSyncPayload {
  schemaVersion: 1
  fetchedAt: string
  providers: Array<{
    id: string
    name: string
    percentUsed: number
    usageLabel: string
    summary: string
    accentColor: string
    state: string
  }>
}

export function createWidgetSyncToken() {
  const bytes = new Uint8Array(18)
  globalThis.crypto?.getRandomValues(bytes)
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}

export function createWidgetSyncPairId() {
  const bytes = new Uint8Array(9)
  globalThis.crypto?.getRandomValues(bytes)
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("")
}

export function buildRelaySnapshotUrl(settings: AppSettings) {
  if (!settings.widgetSyncRelayUrl || !settings.widgetSyncPairId || !settings.widgetSyncToken) {
    return ""
  }
  const base = settings.widgetSyncRelayUrl.replace(/\/+$/, "")
  const pairId = encodeURIComponent(settings.widgetSyncPairId)
  const token = encodeURIComponent(settings.widgetSyncToken)
  return `${base}/v1/snapshots/${pairId}?token=${token}`
}

export async function uploadWidgetSnapshot(
  settings: AppSettings,
  snapshot: WidgetSyncPayload
) {
  if (!settings.widgetSyncRelayUrl || !settings.widgetSyncPairId || !settings.widgetSyncToken) {
    return
  }

  const base = settings.widgetSyncRelayUrl.replace(/\/+$/, "")
  const response = await fetch(`${base}/v1/snapshots/${settings.widgetSyncPairId}`, {
    method: "PUT",
    headers: {
      "Authorization": `Bearer ${settings.widgetSyncToken}`,
      "Content-Type": "application/json"
    },
    body: JSON.stringify(snapshot)
  })

  if (!response.ok) {
    throw new Error(`Relay upload failed: HTTP ${response.status}`)
  }
}

export function buildWidgetSyncPayload(
  states: ProviderSnapshotState[],
  settings: AppSettings
): WidgetSyncPayload {
  const byId = new Map(states.map((state) => [state.provider.id, state]))
  const providers = settings.providerOrder
    .filter((id) => !settings.disabledProviders.includes(id))
    .map((id) => byId.get(id))
    .filter((state): state is ProviderSnapshotState => Boolean(state))
    .map((state) => {
      const max = computeProviderMax(state)
      return {
        id: state.provider.id,
        name: state.provider.displayName,
        percentUsed: max == null ? 0 : Math.round(Math.max(0, Math.min(1, max)) * 100),
        usageLabel: usageLabelForState(state),
        summary: summaryForState(state),
        accentColor: state.provider.brandColor,
        state: toCardState(state, settings).kind
      }
    })

  return {
    schemaVersion: 1,
    fetchedAt: new Date().toISOString(),
    providers
  }
}

function usageLabelForState(state: ProviderSnapshotState) {
  if (state.error) return ""
  const textLine = state.snapshot?.lines.find(
    (candidate) =>
      candidate.type === "text" &&
      (candidate.label === "Today" ||
        candidate.label === "Yesterday" ||
        candidate.label === "Last 30 Days" ||
        candidate.label === "Used" ||
        candidate.label === "Cost" ||
        candidate.label === "Balance" ||
        /token|\$|credit/i.test(candidate.value))
  )
  if (textLine?.type === "text") return compactUsageValue(textLine.value)

  const progressLine = state.snapshot?.lines.find((candidate) => candidate.type === "progress")
  if (progressLine?.type === "progress") return compactUsageValue(formatLineValue(progressLine))

  const line = state.snapshot?.lines.find(
    (candidate) => candidate.type === "text" || candidate.type === "badge"
  )
  if (line && "value" in line) return compactUsageValue(line.value)
  return ""
}

function summaryForState(state: ProviderSnapshotState) {
  if (state.error) return state.error.message
  const line = state.snapshot?.lines.find((candidate) => candidate.type === "text")
  if (line?.type === "text") return `${line.label}: ${line.value}`
  const badge = state.snapshot?.lines.find((candidate) => candidate.type === "badge")
  if (badge?.type === "badge") return `${badge.label}: ${badge.value}`
  return state.snapshot?.plan ?? "No usage data"
}

function compactUsageValue(value: string) {
  return value
    .replace(/\s+tokens?/i, " tok")
    .replace(/\s+requests?/i, " req")
    .replace(/\s+credits?/i, " cr")
    .trim()
    .slice(0, 12)
}
