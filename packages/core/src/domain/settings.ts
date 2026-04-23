import type { ProviderId } from "./provider"

export type Locale = "ko" | "en"
export type ThemeMode = "system" | "light" | "dark"
export type DisplayMode = "used" | "left"
export type MenubarIconStyle = "provider" | "bars" | "donut"
export type RefreshIntervalMinutes = 5 | 15 | 30 | 60

export interface FeatureFlags {
  telemetryEnabled: boolean
  localApiEnabled: boolean
  updaterEnabled: boolean
}

export interface AppSettings {
  providerOrder: ProviderId[]
  disabledProviders: ProviderId[]
  /** @deprecated Phase 16: always treated as true; field retained only for settings migration. */
  homeCompactView?: boolean
  themeMode: ThemeMode
  locale: Locale
  displayMode: DisplayMode
  menubarIconStyle: MenubarIconStyle
  refreshIntervalMinutes: RefreshIntervalMinutes
  notificationsEnabled: boolean
  notificationThresholds: number[]
  trayTarget: "max" | "last-viewed" | ProviderId
  startOnLogin: boolean
  featureFlags: FeatureFlags
}

export const defaultProviderOrder: ProviderId[] = [
  "codex",
  "claude",
  "copilot",
  "openrouter"
]

export const defaultSettings: AppSettings = {
  providerOrder: defaultProviderOrder,
  disabledProviders: [],
  themeMode: "system",
  locale: "ko",
  displayMode: "used",
  menubarIconStyle: "provider",
  refreshIntervalMinutes: 15,
  notificationsEnabled: true,
  notificationThresholds: [80, 95],
  trayTarget: "last-viewed",
  startOnLogin: false,
  featureFlags: {
    telemetryEnabled: false,
    localApiEnabled: false,
    updaterEnabled: false
  }
}
