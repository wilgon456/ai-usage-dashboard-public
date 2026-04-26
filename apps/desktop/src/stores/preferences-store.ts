import { create } from "zustand"
import { defaultProviderOrder, defaultSettings } from "@ai-usage-dashboard/core"
import type { AppSettings } from "@ai-usage-dashboard/core"

const DROPPED_PREFERENCE_KEYS = new Set([
  "homeCompactView",
  "targetPlatform",
  "platformOverride",
  "globalShortcut",
  "resetTimerDisplay"
])

function stripDroppedPreferenceKeys(payload: Record<string, unknown>) {
  return Object.fromEntries(
    Object.entries(payload).filter(([key]) => !DROPPED_PREFERENCE_KEYS.has(key))
  )
}

export function normalizePreferencesPayload(
  payload: Partial<AppSettings> | Record<string, unknown> | null | undefined
): Partial<AppSettings> {
  if (!payload) return {}
  const normalized = stripDroppedPreferenceKeys(
    payload as Record<string, unknown>
  ) as Partial<AppSettings>

  if (Array.isArray(normalized.providerOrder)) {
    normalized.providerOrder = [
      ...normalized.providerOrder.filter((id) => defaultProviderOrder.includes(id)),
      ...defaultProviderOrder.filter((id) => !normalized.providerOrder?.includes(id))
    ]
  }

  if (Array.isArray(normalized.disabledProviders)) {
    normalized.disabledProviders = normalized.disabledProviders.filter((id) =>
      defaultProviderOrder.includes(id)
    )
  }

  return normalized
}

export function toPersistedPreferences(settings: AppSettings): Partial<AppSettings> {
  return stripDroppedPreferenceKeys(settings as unknown as Record<string, unknown>) as Partial<AppSettings>
}

interface PreferencesState {
  settings: AppSettings
  hydrate(next: Partial<AppSettings>): void
}

export const usePreferencesStore = create<PreferencesState>((set) => ({
  settings: defaultSettings,
  hydrate: (next) =>
    set((state) => ({
      // Drop persisted legacy keys as soon as settings are rehydrated locally.
      settings: {
        ...state.settings,
        ...normalizePreferencesPayload(next),
        locale: next.locale ?? state.settings.locale ?? defaultSettings.locale
      }
    }))
}))
