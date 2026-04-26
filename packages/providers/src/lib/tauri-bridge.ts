import type { ProviderErrorKind } from "../contracts"

export async function invokeTauri<T>(command: string, args?: unknown): Promise<T | null> {
  if (!(globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return null
  }

  const mod = await import("@tauri-apps/api/core")
  return mod.invoke<T>(command, args as Record<string, unknown> | undefined)
}

export function classifyProviderError(reason: string): {
  errorKind: ProviderErrorKind
  retryable: boolean
} {
  const errorKind = classifyErrorKind(reason)
  return {
    errorKind,
    retryable: errorKind !== "auth"
  }
}

function classifyErrorKind(reason: string): ProviderErrorKind {
  if (/credentials|logged in|API key|unauthorized/i.test(reason)) {
    return "auth"
  }
  if (/Rate limited|429/i.test(reason)) {
    return "rate_limited"
  }
  if (/HTTP|network|fetch/i.test(reason)) {
    return "network"
  }
  return "unexpected"
}
