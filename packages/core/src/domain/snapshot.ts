import type { ProviderDefinition, ProviderError, ProviderId } from "./provider"

export type MetricFormat =
  | { kind: "percent" }
  | { kind: "count"; suffix: string }
  | { kind: "currency"; currency: "USD" }

export type MetricLineScope = "overview" | "detail"

export interface MetricLineBase {
  label: string
  scope?: MetricLineScope
  primaryOrder?: number
}

export type MetricLine =
  | (MetricLineBase & {
      type: "progress"
      used: number
      limit: number
      format: MetricFormat
      resetsAt?: string
      color?: string
    })
  | (MetricLineBase & {
      type: "text"
      value: string
      subtitle?: string
      color?: string
    })
  | (MetricLineBase & {
      type: "badge"
      value: string
      tone?: "neutral" | "good" | "warn" | "danger"
    })

export interface UsageSnapshot {
  providerId: ProviderId
  fetchedAt: string
  plan?: string
  lines: MetricLine[]
  source: "remote" | "cache"
}

export type ProviderCardState =
  | { kind: "live"; snapshot: UsageSnapshot }
  | { kind: "cached"; snapshot: UsageSnapshot }
  | { kind: "idle"; snapshot: UsageSnapshot }
  | { kind: "unconfigured" }
  | { kind: "disabled" }
  | { kind: "error"; message: string; retryable: boolean }

export interface ProviderSnapshotState {
  provider: ProviderDefinition
  snapshot?: UsageSnapshot
  error?: ProviderError
}
