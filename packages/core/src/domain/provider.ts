export type ProviderId = "claude" | "codex" | "copilot" | "openrouter" | "kimi"

export type CredentialHandle =
  | {
      kind: "oauth"
      accessToken: string
      refreshToken?: string
      expiresAt?: string
    }
  | {
      kind: "apiKey"
      apiKey: string
    }

export type ProviderHealth = "ready" | "needs-auth" | "degraded"

export type ConnectionGuide =
  | {
      kind: "cli"
      title: string
      steps: string[]
      docsUrl: string
    }
  | {
      kind: "oauth"
      title: string
      docsUrl: string
    }
  | {
      kind: "apikey"
      title: string
      docsUrl: string
      saveCommand: string
      clearCommand: string
      hasCommand: string
      placeholder?: string
    }

export type ProviderAuthErrorCode =
  | "missing_credentials"
  | "expired_credentials"
  | "unsupported_platform"

export type ProviderFetchErrorCode =
  | "auth"
  | "network"
  | "parse"
  | "rate-limited"
  | "unexpected"

export interface ProviderAuthError {
  code: ProviderAuthErrorCode
  message: string
}

export interface ProviderFetchError {
  code: ProviderFetchErrorCode
  message: string
  retryable: boolean
}

export type ProviderError = ProviderAuthError | ProviderFetchError

export interface ProviderDefinition {
  id: ProviderId
  displayName: string
  brandColor: string
  health: ProviderHealth
  connectionGuide?: ConnectionGuide
}
