# Architecture

## Objective

Ship a desktop AI usage dashboard that reaches Windows without depending on macOS-only internals.

## Why Not Build Directly on a Fork

Forking a macOS-first project is useful for reference and fast experiments, but not as the long-term base:

- provider code often has broad filesystem, SQLite, keychain, and network access
- macOS-specific panel and system behavior create Windows migration cost later
- telemetry, updater, and local API defaults may not match this product's risk profile
- reverse-engineered provider flows should be isolated behind explicit adapters

The final product should reuse ideas, not inherit all trust boundaries unchanged.

## Target Structure

```text
ai_usage_dashboard/
  README.md
  docs/
    architecture.md
    roadmap.md
  apps/
    desktop/
      src/
      src-tauri/
  packages/
    core/
    providers/
    platform/
```

## Layer Model

### 1. Core

Responsibilities:

- shared entities
- usage snapshot model
- refresh scheduler
- provider registry
- cache policy
- settings schema
- redaction policy

Key rules:

- no direct OS credential access
- no tray or window code
- deterministic output for provider inputs

### 2. Providers

Responsibilities:

- resolve credentials through approved platform adapter
- fetch remote usage data
- normalize provider responses into shared snapshot model
- report auth, network, and parsing errors in consistent form

Key rules:

- one adapter per provider
- no arbitrary script loading
- no unrestricted filesystem traversal
- write access disabled unless a provider explicitly requires token refresh persistence

### 3. Platform

Responsibilities:

- credential storage bridge
- file path resolution
- secure local cache path handling
- startup integration
- OS-specific policy toggles

Platform adapters:

- `macos`
- `windows`

### 4. Desktop Shell

Responsibilities:

- tray and window UX
- settings UI
- provider refresh actions
- log access and diagnostics

The shell consumes `core` and `providers` through typed interfaces only.

## Credential Boundary

Credential handling is the main design constraint.

Rules:

- UI never reads raw tokens directly.
- Provider adapters request credentials through a platform credential interface.
- Credentials are returned only for the provider currently being refreshed.
- Logs must never include raw credential values.
- Refresh token writes must go through explicit persistence methods, not direct file writes from UI code.

Proposed interfaces:

```ts
type CredentialHandle =
  | { kind: "oauth"; accessToken: string; refreshToken?: string }
  | { kind: "apiKey"; apiKey: string }

interface CredentialStore {
  load(providerId: string): Promise<CredentialHandle | null>
  save(providerId: string, credential: CredentialHandle): Promise<void>
  clear(providerId: string): Promise<void>
}
```

## OS Strategy

### macOS

Use:

- tray icon
- compact dashboard window
- Keychain-backed credential store

Avoid locking app behavior to:

- NSPanel-only assumptions
- private API unless clearly required

### Windows

Use:

- system tray
- compact dashboard window
- Credential Manager or DPAPI-backed storage
- WebView2-supported Tauri shell

Design constraint:

- every macOS-only behavior must have a shell-level fallback before the app is considered stable

## Local API Strategy

Default:

- disabled

If later enabled:

- bind to loopback only
- explicit user opt-in
- minimal response shape
- no permissive wildcard access without a clear reason
- tokenless read path only

## Telemetry Strategy

Default:

- disabled

If later enabled:

- opt-in only
- no provider identifiers that expose sensitive account data
- no usage content, token payloads, file paths, or auth metadata

## Updater Strategy

Default for early development:

- off in local builds

Release requirement:

- signed artifacts only
- platform-specific update verification
- rollback path documented before enablement

## Testing Strategy

### Core

- unit tests for snapshot normalization
- redaction tests
- settings schema tests

### Providers

- fixture-based response parsing
- credential resolution tests
- stale token and refresh tests

### Desktop

- smoke tests for tray, settings, refresh
- packaging checks on macOS and Windows

## First Build Decision

Start on the final layered architecture immediately, even if the first runnable app is macOS-only.

That keeps Windows as an additive shell task instead of a rewrite.
