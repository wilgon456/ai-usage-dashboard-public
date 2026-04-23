import type {
  ProviderDefinition,
  ProviderId,
  UsageSnapshot
} from "@ai-usage-dashboard/core"
import type { PlatformRuntime } from "@ai-usage-dashboard/platform"

export type ProviderProbeResult =
  | { ok: true; snapshot: UsageSnapshot }
  | { ok: false; providerId: ProviderId; reason: string; retryable: boolean }

export interface ProbeOptions {
  refreshIntervalMinutes: number
  force: boolean
}

export interface ProviderAdapter {
  definition: ProviderDefinition
  probe(platform: PlatformRuntime, options: ProbeOptions): Promise<ProviderProbeResult>
}
