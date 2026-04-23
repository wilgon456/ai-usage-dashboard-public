import { defaultSettings, RefreshOrchestrator } from "@ai-usage-dashboard/core"
import type {
  AppSettings,
  CredentialHandle,
  Locale,
  ProbeOptions,
  ProviderDefinition,
  ProviderId,
  ProviderSnapshotState,
  UsageSnapshot
} from "@ai-usage-dashboard/core"
import type {
  CredentialStore,
  PlatformRuntime,
  SettingsStore
} from "@ai-usage-dashboard/platform"
import {
  claudeAdapter,
  codexAdapter,
  copilotAdapter,
  openrouterAdapter
} from "@ai-usage-dashboard/providers"
import type { ProviderAdapter } from "@ai-usage-dashboard/providers"
import { translate } from "./i18n"
import {
  normalizePreferencesPayload,
  toPersistedPreferences
} from "./stores/preferences-store"

const SETTINGS_STORAGE_KEY = "ai-usage-dashboard.settings"
const CREDENTIAL_STORAGE_PREFIX = "ai-usage-dashboard.credential."

const providerAdapters: ProviderAdapter[] = [
  codexAdapter,
  claudeAdapter,
  copilotAdapter,
  openrouterAdapter
]

type ResolvedPlatform = "macos" | "windows"

let resolvedPlatform: ResolvedPlatform = "macos"

function readJson<T>(key: string): T | null {
  if (typeof localStorage === "undefined") return null

  const raw = localStorage.getItem(key)
  if (!raw) return null

  try {
    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

function writeJson(key: string, value: unknown) {
  if (typeof localStorage === "undefined") return
  localStorage.setItem(key, JSON.stringify(value))
}

function isResolvedPlatform(value: string): value is ResolvedPlatform {
  return value === "macos" || value === "windows"
}

function isLocale(value: string): value is Locale {
  return value === "ko" || value === "en"
}

async function invokeTauri<T>(command: string, args?: Record<string, unknown>): Promise<T | null> {
  if (!(globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
    return null
  }

  try {
    const module = await import("@tauri-apps/api/core")
    return module.invoke<T>(command, args)
  } catch {
    return null
  }
}

async function detectRuntimePlatform(): Promise<ResolvedPlatform> {
  const detected = await invokeTauri<string>("detect_platform")
  return detected && isResolvedPlatform(detected) ? detected : "macos"
}

async function syncTrayLabels(locale: Locale) {
  await invokeTauri<void>("set_tray_labels", {
    showDashboard: translate(locale, "tray.showDashboard"),
    goToSettings: translate(locale, "tray.goToSettings"),
    quit: translate(locale, "tray.quit")
  })
}

function purgeLegacyDemoCredentials() {
  if (typeof localStorage === "undefined") return

  for (const providerId of ["claude", "codex"] as const) {
    const storageKey = CREDENTIAL_STORAGE_PREFIX + providerId
    const raw = localStorage.getItem(storageKey)
    if (!raw) continue
    if (raw.includes("demo-claude-") || raw.includes("demo-codex-")) {
      localStorage.removeItem(storageKey)
    }
  }
}

class BrowserCredentialStore implements CredentialStore {
  async load(providerId: ProviderId) {
    return readJson<CredentialHandle>(CREDENTIAL_STORAGE_PREFIX + providerId)
  }

  async save(providerId: ProviderId, credential: CredentialHandle) {
    writeJson(CREDENTIAL_STORAGE_PREFIX + providerId, credential)
  }

  async clear(providerId: ProviderId) {
    if (typeof localStorage === "undefined") return
    localStorage.removeItem(CREDENTIAL_STORAGE_PREFIX + providerId)
  }
}

class BrowserSettingsStore implements SettingsStore {
  async load() {
    const saved = readJson<Record<string, unknown>>(SETTINGS_STORAGE_KEY)
    const merged = {
      ...defaultSettings,
      ...normalizePreferencesPayload(saved)
    } as AppSettings
    if (!isLocale(merged.locale)) {
      merged.locale = defaultSettings.locale
    }
    // Older installs stored trayTarget="max" as the default. The new default
    // is "last-viewed"; upgrade the prior default so users get the new
    // behavior without a manual toggle. Anyone who wants "max" again can set
    // it explicitly in Settings → Notifications.
    if (merged.trayTarget === "max") {
      merged.trayTarget = "last-viewed"
    }
    return merged
  }

  async save(settings: AppSettings) {
    writeJson(SETTINGS_STORAGE_KEY, toPersistedPreferences(settings))
  }
}

function createPlatformRuntime(target: ResolvedPlatform): PlatformRuntime {
  return {
    info: {
      target,
      supportsTray: true,
      supportsAutoStart: true,
      supportsSecureCredentialStore: true
    },
    credentials: new BrowserCredentialStore(),
    settings: new BrowserSettingsStore()
  }
}

function createProviderRuntime(platform: PlatformRuntime) {
  return new RefreshOrchestrator(
    providerAdapters.map((adapter) => ({
      definition: adapter.definition,
      refresh: (options) => adapter.probe(platform, options)
    }))
  )
}

function enabledProviderIds(settings: AppSettings): ProviderId[] {
  return settings.providerOrder.filter((id) => !settings.disabledProviders.includes(id))
}

function toProviderState(
  definition: ProviderDefinition,
  result:
    | { ok: true; snapshot: UsageSnapshot }
    | { ok: false; providerId: ProviderId; reason: string; retryable: boolean }
): ProviderSnapshotState {
  if (result.ok) {
    return {
      provider: definition,
      snapshot: result.snapshot
    }
  }

  return {
    provider: definition,
    error: result.reason.includes("credentials")
      ? {
          code: "missing_credentials",
          message: result.reason
        }
      : {
          code: "unexpected",
          message: result.reason,
          retryable: result.retryable
        }
  }
}

async function buildIdleState(
  definition: ProviderDefinition,
  enabled: ProviderId[],
  runtime: PlatformRuntime
): Promise<ProviderSnapshotState> {
  const credential = await runtime.credentials.load(definition.id)

  if (!enabled.includes(definition.id)) {
    return {
      provider: definition,
      error: {
        code: "missing_credentials",
        message: "Disabled in current settings."
      }
    }
  }

  if (!credential) {
    return {
      provider: definition,
      error: {
        code: "missing_credentials",
        message: "Credentials not configured yet."
      }
    }
  }

  return {
    provider: definition,
    snapshot: {
      providerId: definition.id,
      fetchedAt: new Date().toISOString(),
      plan: "Waiting for refresh",
      lines: [
        {
          type: "badge",
          label: "Status",
          value: "Configured",
          tone: "good"
        },
        {
          type: "text",
          label: "Credential",
          value: credential.kind
        }
      ],
      source: "cache"
    }
  }
}

export interface DesktopShell {
  getSettings(): Promise<AppSettings>
  listProviders(): ProviderDefinition[]
  refreshAll(options: ProbeOptions): Promise<ProviderSnapshotState[]>
  updatePreferences(partial: Partial<AppSettings>): Promise<AppSettings>
  toggleProvider(providerId: ProviderId): Promise<AppSettings>
}

export async function bootDesktopShell(): Promise<DesktopShell> {
  purgeLegacyDemoCredentials()

  const settingsStore = new BrowserSettingsStore()
  resolvedPlatform = await detectRuntimePlatform()
  let settings = await settingsStore.load()
  await syncTrayLabels(settings.locale)
  let platform = createPlatformRuntime(resolvedPlatform)

  async function refreshAll(options: ProbeOptions) {
    const orchestrator = createProviderRuntime(platform)
    const refreshed = await orchestrator.refreshAll(options)
    const refreshedById = new Map(
      refreshed.map((result) => [result.ok ? result.snapshot.providerId : result.providerId, result])
    )

    const enabled = enabledProviderIds(settings)
    const states: ProviderSnapshotState[] = []

    for (const definition of orchestrator.listProviders()) {
      if (!enabled.includes(definition.id)) {
        states.push(await buildIdleState(definition, enabled, platform))
        continue
      }

      const result = refreshedById.get(definition.id)
      if (!result) {
        states.push(await buildIdleState(definition, enabled, platform))
        continue
      }

      states.push(toProviderState(definition, result))
    }

    return states
  }

  async function updateSettings(next: AppSettings) {
    const previousLocale = settings.locale
    settings = next
    await settingsStore.save(next)
    if (previousLocale !== next.locale) {
      await syncTrayLabels(next.locale)
    }
    platform = createPlatformRuntime(resolvedPlatform)
    return settings
  }

  return {
    async getSettings() {
      return settings
    },
    listProviders() {
      return providerAdapters.map((adapter) => adapter.definition)
    },
    refreshAll,
    async updatePreferences(partial: Partial<AppSettings>) {
      return updateSettings({
        ...settings,
        ...partial
      })
    },
    async toggleProvider(providerId: ProviderId) {
      const isDisabled = settings.disabledProviders.includes(providerId)
      const disabledProviders = isDisabled
        ? settings.disabledProviders.filter((id) => id !== providerId)
        : [...settings.disabledProviders, providerId]
      const providerOrder = settings.providerOrder.includes(providerId)
        ? settings.providerOrder
        : [...settings.providerOrder, providerId]

      return updateSettings({
        ...settings,
        providerOrder,
        disabledProviders
      })
    }
  }
}
