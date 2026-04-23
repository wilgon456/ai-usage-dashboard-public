import type {
  AppSettings,
  ProviderCardState,
  ProviderError,
  ProviderSnapshotState
} from "@ai-usage-dashboard/core"

function isIdleSnapshot(state: ProviderSnapshotState) {
  return (
    state.snapshot?.source === "cache" &&
    state.snapshot.plan === "Waiting for refresh"
  )
}

function toErrorState(error: ProviderError): ProviderCardState {
  if ("retryable" in error) {
    return {
      kind: "error",
      message: error.message,
      retryable: error.retryable
    }
  }

  return {
    kind: "error",
    message: error.message,
    retryable: false
  }
}

export function toCardState(
  state: ProviderSnapshotState,
  settings: AppSettings
): ProviderCardState {
  if (settings.disabledProviders.includes(state.provider.id)) {
    return { kind: "disabled" }
  }

  if (state.error?.code === "missing_credentials") {
    return { kind: "unconfigured" }
  }

  if (state.error) {
    return toErrorState(state.error)
  }

  if (state.snapshot?.source === "remote") {
    return { kind: "live", snapshot: state.snapshot }
  }

  if (state.snapshot && isIdleSnapshot(state)) {
    return { kind: "idle", snapshot: state.snapshot }
  }

  if (state.snapshot) {
    return { kind: "cached", snapshot: state.snapshot }
  }

  return {
    kind: "error",
    message: "Provider state is unavailable.",
    retryable: true
  }
}
