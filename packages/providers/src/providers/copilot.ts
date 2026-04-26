import type { MetricLine } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"
import { classifyProviderError, invokeTauri } from "../lib/tauri-bridge"

interface UsagePayload {
  providerId: "copilot"
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

export const copilotAdapter: ProviderAdapter = {
  definition: {
    id: "copilot",
    displayName: "Copilot",
    brandColor: "#3b82f6",
    health: "needs-auth",
    connectionGuide: {
      kind: "cli",
      title: "Sign in with GitHub Copilot",
      steps: [
        "Install the GitHub CLI: https://cli.github.com",
        "Run: gh auth login --web",
        "Ensure your account has Copilot access.",
        "Return to this app, then click Done - Refresh."
      ],
      docsUrl: "https://docs.github.com/en/copilot"
    }
  },
  async probe(_platform, options: ProbeOptions) {
    try {
      const payload = await invokeTauri<UsagePayload>("get_copilot_usage", options)
      if (!payload) {
        const failure = classifyProviderError("Tauri runtime not available.")
        return {
          ok: false,
          providerId: "copilot",
          reason: "Tauri runtime not available.",
          retryable: false,
          errorKind: failure.errorKind
        }
      }

      return {
        ok: true,
        snapshot: {
          providerId: "copilot",
          fetchedAt: new Date().toISOString(),
          plan: payload.plan ?? "Copilot",
          lines: payload.lines,
          source: payload.source
        }
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      const failure = classifyProviderError(reason)
      return {
        ok: false,
        providerId: "copilot",
        reason,
        retryable: failure.retryable,
        errorKind: failure.errorKind
      }
    }
  }
}
