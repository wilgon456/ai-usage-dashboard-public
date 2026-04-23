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
        return {
          ok: false,
          providerId: "openrouter",
          reason: "Tauri runtime not available.",
          retryable: false
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
      return {
        ok: false,
        providerId: "openrouter",
        reason,
        retryable: !/not logged in|not set|invalid|expired/i.test(reason)
      }
    }
  }
}
