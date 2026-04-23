import type { MetricLine } from "@ai-usage-dashboard/core"
import type { ProbeOptions, ProviderAdapter } from "../contracts"

async function invokeTauri<T>(command: string, args?: unknown): Promise<T | null> {
  if (!(globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return null
  }

  const module = await import("@tauri-apps/api/core")
  return module.invoke<T>(command, args as Record<string, unknown> | undefined)
}

interface UsagePayload {
  providerId: "codex"
  plan: string | null
  lines: MetricLine[]
  source: "remote" | "cache"
}

interface LegacyCodexPayload {
  providerId: "codex"
  plan: string | null
  rateLimits: {
    session: { usedPercent: number; resetsAt: string | null } | null
    weekly: { usedPercent: number; resetsAt: string | null } | null
  }
  tokens: {
    today: number
    todayInput: number
    todayOutput: number
    last30Days: number
    lastEvent: number
    lastEventAt: string | null
  }
  source: string
}

function mapLegacyPayload(payload: LegacyCodexPayload): UsagePayload {
  const lines: MetricLine[] = []

  if (payload.rateLimits.session) {
    lines.push({
      type: "progress",
      label: "Session",
      used: payload.rateLimits.session.usedPercent,
      limit: 100,
      format: { kind: "percent" },
      resetsAt: payload.rateLimits.session.resetsAt ?? undefined
    })
  }

  if (payload.rateLimits.weekly) {
    lines.push({
      type: "progress",
      label: "Weekly",
      used: payload.rateLimits.weekly.usedPercent,
      limit: 100,
      format: { kind: "percent" },
      resetsAt: payload.rateLimits.weekly.resetsAt ?? undefined
    })
  }

  lines.push({
    type: "text",
    label: "Today",
    value: `${payload.tokens.today} tokens`
  })
  lines.push({
    type: "text",
    label: "Last 30 Days",
    value: `${payload.tokens.last30Days} tokens`
  })
  lines.push({
    type: "text",
    label: "Today I/O",
    value: `${payload.tokens.todayInput} in · ${payload.tokens.todayOutput} out`
  })

  if (payload.tokens.lastEvent > 0) {
    lines.push({
      type: "text",
      label: "Last Event",
      value: `${payload.tokens.lastEvent} tokens`,
      subtitle: payload.tokens.lastEventAt ?? undefined
    })
  }

  return {
    providerId: "codex",
    plan: payload.plan,
    lines,
    source: payload.source === "cache" ? "cache" : "remote"
  }
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

      const response = await fetch("/api/codex/usage")
      const result = (await response.json()) as { error: string } | LegacyCodexPayload

      if (!response.ok || "error" in result) {
        const reason = "error" in result ? result.error : "Codex usage could not be loaded."
        return {
          ok: false,
          providerId: "codex",
          reason,
          retryable: !/not logged in|not set|invalid|expired/i.test(reason)
        }
      }

      const payload = mapLegacyPayload(result)
      return {
        ok: true,
        snapshot: {
          providerId: "codex",
          fetchedAt: new Date().toISOString(),
          plan: payload.plan ?? "Codex",
          lines: payload.lines,
          source: payload.source
        }
      }
    } catch (error) {
      const reason = error instanceof Error ? error.message : String(error)
      return {
        ok: false,
        providerId: "codex",
        reason,
        retryable: !/not logged in|not set|invalid|expired/i.test(reason)
      }
    }
  }
}
