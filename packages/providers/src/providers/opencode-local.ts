import type { MetricLine, ProviderId } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"
import { classifyProviderError, invokeTauri } from "../lib/tauri-bridge"

type LocalProviderId = "alibaba-token-plan" | "opencode-go" | "grok"

interface UsagePayload {
  providerId: LocalProviderId
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

interface LocalProviderConfig {
  id: LocalProviderId
  displayName: string
  brandColor: string
  command: string
}

function createOpenCodeLocalAdapter(config: LocalProviderConfig): ProviderAdapter {
  return {
    definition: {
      id: config.id,
      displayName: config.displayName,
      brandColor: config.brandColor,
      health: "ready"
    },
    async probe(_platform, options: ProbeOptions) {
      try {
        const payload = await invokeTauri<UsagePayload>(config.command, options)
        if (!payload) {
          const reason = "Tauri runtime not available."
          const failure = classifyProviderError(reason)
          return {
            ok: false as const,
            providerId: config.id as ProviderId,
            reason,
            retryable: false,
            errorKind: failure.errorKind
          }
        }

        return {
          ok: true as const,
          snapshot: {
            providerId: config.id,
            fetchedAt: new Date().toISOString(),
            plan: payload.plan ?? config.displayName,
            lines: payload.lines,
            source: payload.source
          }
        }
      } catch (error) {
        const reason = error instanceof Error ? error.message : String(error)
        const failure = classifyProviderError(reason)
        return {
          ok: false as const,
          providerId: config.id as ProviderId,
          reason,
          retryable: failure.retryable,
          errorKind: failure.errorKind
        }
      }
    }
  }
}

export const alibabaTokenPlanAdapter = createOpenCodeLocalAdapter({
  id: "alibaba-token-plan",
  displayName: "Alibaba Token Plan",
  brandColor: "#ff6a00",
  command: "get_alibaba_token_plan_usage"
})

export const openCodeGoAdapter = createOpenCodeLocalAdapter({
  id: "opencode-go",
  displayName: "OpenCode Go",
  brandColor: "#22c55e",
  command: "get_opencode_go_usage"
})

export const grokAdapter = createOpenCodeLocalAdapter({
  id: "grok",
  displayName: "Grok",
  brandColor: "#f4f4f5",
  command: "get_grok_usage"
})
