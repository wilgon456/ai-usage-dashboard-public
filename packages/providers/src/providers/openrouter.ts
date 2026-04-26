import type { MetricLine } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"
import { classifyProviderError, invokeTauri } from "../lib/tauri-bridge"

interface UsagePayload {
  providerId: "openrouter"
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

export const openrouterAdapter: ProviderAdapter = {
  definition: {
    id: "openrouter",
    displayName: "OpenRouter",
    brandColor: "#6366f1",
    health: "needs-auth",
    connectionGuide: {
      kind: "apikey",
      title: "Paste your OpenRouter API key",
      docsUrl: "https://openrouter.ai/keys",
      saveCommand: "save_openrouter_key",
      clearCommand: "clear_openrouter_key",
      hasCommand: "has_openrouter_key",
      placeholder: "sk-or-v1-..."
    }
  },
  async probe(_platform, options: ProbeOptions) {
    try {
      const payload = await invokeTauri<UsagePayload>("get_openrouter_usage", options)
      if (!payload) {
        const failure = classifyProviderError("Tauri runtime not available.")
        return {
          ok: false,
          providerId: "openrouter",
          reason: "Tauri runtime not available.",
          retryable: false,
          errorKind: failure.errorKind
        }
      }

      return {
        ok: true,
        snapshot: {
          providerId: "openrouter",
          fetchedAt: new Date().toISOString(),
          plan: payload.plan ?? "OpenRouter",
          lines: payload.lines,
          source: payload.source
        }
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      const failure = classifyProviderError(reason)
      return {
        ok: false,
        providerId: "openrouter",
        reason,
        retryable: failure.retryable,
        errorKind: failure.errorKind
      }
    }
  }
}
