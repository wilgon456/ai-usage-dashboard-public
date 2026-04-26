import type { MetricLine } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"
import { classifyProviderError, invokeTauri } from "../lib/tauri-bridge"

interface UsagePayload {
  providerId: "kimi"
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

export const kimiAdapter: ProviderAdapter = {
  definition: {
    id: "kimi",
    displayName: "Kimi",
    brandColor: "#5b4bff",
    health: "needs-auth",
    connectionGuide: {
      kind: "cli",
      title: "Install Kimi CLI",
      steps: [
        "Run: uv tool install kimi-cli   (or: pipx install kimi-cli)",
        "Run: kimi login",
        "Return to this app, then click Done - Refresh."
      ],
      docsUrl: "https://moonshotai.github.io/kimi-cli/"
    }
  },
  async probe(_platform, options: ProbeOptions) {
    try {
      const payload = await invokeTauri<UsagePayload>("get_kimi_usage", options)
      if (!payload) {
        const failure = classifyProviderError("Tauri runtime not available.")
        return {
          ok: false,
          providerId: "kimi",
          reason: "Tauri runtime not available.",
          retryable: false,
          errorKind: failure.errorKind
        }
      }

      return {
        ok: true,
        snapshot: {
          providerId: "kimi",
          fetchedAt: new Date().toISOString(),
          plan: payload.plan ?? "Kimi",
          lines: payload.lines,
          source: payload.source
        }
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      const failure = classifyProviderError(reason)
      return {
        ok: false,
        providerId: "kimi",
        reason,
        retryable: failure.retryable,
        errorKind: failure.errorKind
      }
    }
  }
}
