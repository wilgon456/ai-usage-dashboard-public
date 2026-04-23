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
  providerId: "claude"
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

export const claudeAdapter: ProviderAdapter = {
  definition: {
    id: "claude",
    displayName: "Claude",
    brandColor: "#d97757",
    health: "needs-auth",
    connectionGuide: {
      kind: "cli",
      title: "Install the Claude Code CLI",
      steps: [
        "Run: npm install -g @anthropic-ai/claude-code",
        "Run: claude auth login",
        "Return to this app, then click Done - Refresh."
      ],
      docsUrl: "https://docs.claude.com/en/docs/claude-code/quickstart"
    }
  },
  async probe(_platform, options: ProbeOptions) {
    try {
      const payload = await invokeTauri<UsagePayload>("get_claude_usage", options)
      if (!payload) {
        return {
          ok: false,
          providerId: "claude",
          reason: "Tauri runtime not available.",
          retryable: false
        }
      }

      return {
        ok: true,
        snapshot: {
          providerId: "claude",
          fetchedAt: new Date().toISOString(),
          plan: payload.plan ?? "Claude",
          lines: payload.lines,
          source: payload.source
        }
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      return {
        ok: false,
        providerId: "claude",
        reason,
        retryable: !/not logged in|not set|invalid|expired/i.test(reason)
      }
    }
  }
}
