import { create } from "zustand"
import { defaultSettings } from "@ai-usage-dashboard/core"
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
  return stripDroppedPreferenceKeys(payload as Record<string, unknown>) as Partial<AppSettings>
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
