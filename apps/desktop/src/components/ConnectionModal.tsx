import { useEffect, useId, useRef, useState } from "react"
import type { ConnectionGuide, ProviderDefinition, ProviderId } from "@ai-usage-dashboard/core"
import { ExternalLink, X } from "lucide-react"
import type { TFunction } from "../i18n"
import { openExternal } from "../lib/open-url"
import { Button } from "./ui/Button"

interface ConnectionModalProps {
  providerId: ProviderId
  provider: ProviderDefinition
  guide: ConnectionGuide
  onClose: () => void
  onRefresh: () => Promise<void> | void
  t: TFunction
}

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const module = await import("@tauri-apps/api/core")
  return module.invoke<T>(command, args)
}

function inTauri() {
  return Boolean((globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
}

function saveArgs(command: string, value: string) {
  if (command === "save_openrouter_key") {
    return { key: value }
  }

  return { apiKey: value }
}

export function ConnectionModal({
  providerId,
  provider,
  guide,
  onClose,
  onRefresh,
  t
}: ConnectionModalProps) {
  const titleId = useId()
  const primaryButtonRef = useRef<HTMLButtonElement>(null)
  const apiKeyGuide = guide.kind === "apikey" ? guide : null
  const [value, setValue] = useState("")
  const [hasExistingKey, setHasExistingKey] = useState(false)
  const [loadingExistingKey, setLoadingExistingKey] = useState(guide.kind === "apikey")
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState<string | null>(null)
  const [launchHint, setLaunchHint] = useState<string | null>(null)

  useEffect(() => {
    primaryButtonRef.current?.focus()
  }, [guide.kind, providerId])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault()
        onClose()
      }
    }

    window.addEventListener("keydown", onKeyDown)
    return () => window.removeEventListener("keydown", onKeyDown)
  }, [onClose])

  useEffect(() => {
    let cancelled = false

    setValue("")
    setError(null)
    setLaunchHint(null)

    if (!apiKeyGuide) {
      setHasExistingKey(false)
      setLoadingExistingKey(false)
      return () => {
        cancelled = true
      }
    }

    setLoadingExistingKey(true)
    void (async () => {
      if (!inTauri()) {
        if (!cancelled) {
          setHasExistingKey(false)
          setLoadingExistingKey(false)
        }
        return
      }

      try {
        const result = await invokeCommand<boolean>(apiKeyGuide.hasCommand)
        if (!cancelled) {
          setHasExistingKey(result)
        }
      } catch (commandError) {
        console.warn("openrouter key check failed", commandError)
      } finally {
        if (!cancelled) {
          setLoadingExistingKey(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [apiKeyGuide, guide, providerId])

  async function handleRefresh() {
    setBusy(true)
    setError(null)

    try {
      await onRefresh()
      onClose()
    } catch (refreshError) {
      const message = refreshError instanceof Error ? refreshError.message : String(refreshError)
      setError(message)
    } finally {
      setBusy(false)
    }
  }

  async function handleLaunch() {
    if (!inTauri()) {
      setError(t("connection.packagedOnly"))
      return
    }

    setBusy(true)
    setError(null)
    setLaunchHint(null)

    try {
      await invokeCommand<void>("run_connect_command", { provider: providerId })
      window.setTimeout(() => {
        setLaunchHint(t("connection.afterLaunchHint"))
      }, 250)
    } catch (launchError) {
      const reason = launchError instanceof Error ? launchError.message : String(launchError)
      setError(t("connection.launchFailed", { reason }))
    } finally {
      setBusy(false)
    }
  }

  async function handleSave() {
    if (!apiKeyGuide) return

    const trimmed = value.trim()
    if (!trimmed) {
      setError(t("connection.enterApiKey"))
      return
    }

    if (!inTauri()) {
      window.alert(t("connection.packagedOnly"))
      return
    }

    setBusy(true)
    setError(null)

    try {
      await invokeCommand<void>(
        apiKeyGuide.saveCommand,
        saveArgs(apiKeyGuide.saveCommand, trimmed)
      )
      await onRefresh()
      onClose()
    } catch (saveError) {
      const message = saveError instanceof Error ? saveError.message : String(saveError)
      setError(message)
    } finally {
      setBusy(false)
    }
  }

  async function handleRemove() {
    if (!apiKeyGuide) return

    if (!inTauri()) {
      window.alert(t("connection.packagedOnly"))
      return
    }

    setBusy(true)
    setError(null)

    try {
      await invokeCommand<void>(apiKeyGuide.clearCommand)
      await onRefresh()
      onClose()
    } catch (removeError) {
      const message = removeError instanceof Error ? removeError.message : String(removeError)
      setError(message)
    } finally {
      setBusy(false)
    }
  }

  const showTerminalLaunch = guide.kind === "cli" || guide.kind === "oauth"

  return (
    <div
      className="absolute inset-0 z-40 flex items-center justify-center bg-overlay p-4"
      onClick={onClose}
    >
      <div
        className="w-full max-w-md rounded-xl border border-border-strong bg-surface-popover p-4 shadow-panel"
        onClick={(event) => event.stopPropagation()}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
      >
        <div className="flex items-start justify-between gap-3">
          <div className="min-w-0">
            <p className="text-[11px] uppercase tracking-wide text-muted">
              {provider.displayName}
            </p>
            <h2 id={titleId} className="mt-1 text-sm font-semibold text-fg">
              {guide.title}
            </h2>
          </div>
          <Button
            size="icon"
            variant="ghost"
            onClick={onClose}
            aria-label={t("connection.closeGuide")}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>

        {guide.kind === "cli" ? (
          <ol className="mt-4 list-decimal space-y-2 pl-4 text-xs text-fg-secondary">
            {guide.steps.map((step) => (
              <li key={step}>{step}</li>
            ))}
          </ol>
        ) : null}

        {guide.kind === "oauth" ? (
          <p className="mt-4 text-xs text-fg-secondary">
            {t("connection.completeOauth")}
          </p>
        ) : null}

        {showTerminalLaunch ? (
          <button
            type="button"
            className="mt-3 inline-flex items-center gap-1 text-xs text-page-accent transition-opacity hover:opacity-80"
            onClick={() => void openExternal(guide.docsUrl)}
          >
            {t("connection.openDocs")}
            <ExternalLink className="h-3.5 w-3.5" />
          </button>
        ) : null}

        {apiKeyGuide ? (
          <div className="mt-4 flex flex-col gap-3">
            <label className="flex flex-col gap-1">
              <span className="text-[11px] text-muted">{t("connection.apiKey")}</span>
              <input
                type="password"
                value={value}
                onChange={(event) => setValue(event.target.value)}
                placeholder={apiKeyGuide.placeholder}
                className="rounded-md border border-border bg-surface-0 px-3 py-2 text-sm text-fg outline-none transition-colors focus-visible:border-border-strong focus-visible:ring-1 focus-visible:ring-border-strong"
              />
            </label>
            <div className="flex items-center justify-between gap-3">
              <button
                type="button"
                className="text-xs text-page-accent transition-opacity hover:opacity-80"
                onClick={() => void openExternal(apiKeyGuide.docsUrl)}
              >
                {t("connection.createKey")}
              </button>
              {loadingExistingKey ? (
                <span className="text-[11px] text-muted">{t("connection.keyCheckInProgress")}</span>
              ) : hasExistingKey ? (
                <span className="text-[11px] text-muted">{t("connection.existingKeySaved")}</span>
              ) : null}
            </div>
          </div>
        ) : null}

        {launchHint ? <p className="mt-3 text-xs text-fg-muted">{launchHint}</p> : null}
        {error ? <p className="mt-3 text-xs text-danger">{error}</p> : null}

        <div className="mt-4 flex flex-wrap gap-2">
          {apiKeyGuide ? (
            <>
              <Button
                ref={primaryButtonRef}
                variant="accent"
                onClick={() => void handleSave()}
                disabled={busy}
              >
                {hasExistingKey ? t("connection.replaceKey") : t("connection.saveKey")}
              </Button>
              {hasExistingKey ? (
                <Button variant="ghost" onClick={() => void handleRemove()} disabled={busy}>
                  {t("connection.removeKey")}
                </Button>
              ) : null}
            </>
          ) : (
            <>
              <Button
                ref={primaryButtonRef}
                variant="accent"
                onClick={() => void handleLaunch()}
                disabled={busy}
              >
                {t("connection.launchCli")}
              </Button>
              <Button variant="ghost" onClick={() => void handleRefresh()} disabled={busy}>
                {t("connection.iveSetUp")}
              </Button>
            </>
          )}
        </div>
      </div>
    </div>
  )
}
