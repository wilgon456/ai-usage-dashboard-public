import type { MetricLine } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"
import { classifyProviderError, invokeTauri } from "../lib/tauri-bridge"

interface UsagePayload {
  providerId: "codex"
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

export const codexAdapter: ProviderAdapter = {
  definition: {
    id: "codex",
    displayName: "Codex",
    brandColor: "#bfff00",
    health: "needs-auth",
    connectionGuide: {
      kind: "cli",
      title: "Install the Codex CLI",
      steps: [
        "Run: brew install codex  (macOS)  or npm install -g @openai/codex",
        "Run: codex login",
        "Return to this app, then click Done - Refresh."
      ],
      docsUrl: "https://github.com/openai/codex"
    }
  },
  async probe(_platform, options: ProbeOptions) {
    try {
      const tauriPayload = await invokeTauri<UsagePayload>("get_codex_usage", options)
      if (tauriPayload) {
        return {
          ok: true,
          snapshot: {
            providerId: "codex",
            fetchedAt: new Date().toISOString(),
            plan: tauriPayload.plan ?? "Codex",
            lines: tauriPayload.lines,
            source: tauriPayload.source
          }
        }
      }

      const reason = "Tauri runtime not available."
      const failure = classifyProviderError(reason)
      return {
        ok: false,
        providerId: "codex",
        reason,
        retryable: false,
        errorKind: failure.errorKind
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      const failure = classifyProviderError(reason)
      return {
        ok: false,
        providerId: "codex",
        reason,
        retryable: failure.retryable,
        errorKind: failure.errorKind
      }
    }
  }
}
