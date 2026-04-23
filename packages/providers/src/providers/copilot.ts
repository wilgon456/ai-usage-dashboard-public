import type { MetricLine } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"

async function invokeTauri<T>(command: string, args?: unknown): Promise<T | null> {
  if (!(globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return null
  }

  const mod = await import("@tauri-apps/api/core")
  return mod.invoke<T>(command, args as Record<string, unknown> | undefined)
}

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
        return {
          ok: false,
          providerId: "copilot",
          reason: "Tauri runtime not available.",
          retryable: false
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
      return {
        ok: false,
        providerId: "copilot",
        reason,
        retryable: !/not logged in|not set|invalid|expired/i.test(reason)
      }
    }
  }
}
