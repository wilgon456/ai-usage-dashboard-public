import type { AppSettings, CredentialHandle, ProviderId } from "@ai-usage-dashboard/core"

export interface CredentialStore {
  load(providerId: ProviderId): Promise<CredentialHandle | null>
  save(providerId: ProviderId, credential: CredentialHandle): Promise<void>
  clear(providerId: ProviderId): Promise<void>
}

export interface SettingsStore {
  load(): Promise<AppSettings>
  save(settings: AppSettings): Promise<void>
}

export interface PlatformInfo {
  target: "macos" | "windows"
  supportsTray: boolean
  supportsAutoStart: boolean
  supportsSecureCredentialStore: boolean
}

export interface PlatformRuntime {
  info: PlatformInfo
  credentials: CredentialStore
  settings: SettingsStore
}
