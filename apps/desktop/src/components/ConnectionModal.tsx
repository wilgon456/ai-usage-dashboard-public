import { useEffect, useId, useRef, useState } from "react"
import type { ConnectionGuide, ProviderDefinition, ProviderId } from "@ai-usage-dashboard/core"
import { ExternalLink, X } from "lucide-react"
import type { TFunction } from "../i18n"
import { openExternal } from "../lib/open-url"
import { Badge } from "./ui/Badge"
import { Button } from "./ui/Button"

interface ConnectionModalProps {
  providerId: ProviderId
  provider: ProviderDefinition
  guide: ConnectionGuide
  onClose: () => void
  onRefresh: () => Promise<void> | void
  t: TFunction
}

interface BootstrapStep {
  id: string
  status: "ready" | "action_required" | "unavailable"
  detail?: string | null
}

interface BootstrapStatus {
  canAutoInstall: boolean
  commandAvailable: boolean
  availableAgents: string[]
  recommendedMode: "agent" | "shell"
  steps: BootstrapStep[]
}

type DelegatingAgent = "codex" | "claude"

async function invokeCommand<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const module = await import("@tauri-apps/api/core")
  return module.invoke<T>(command, args)
}

function inTauri() {
  return Boolean((globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__)
}

function saveArgs(command: string, value: string) {
  if (command === "save_openrouter_key" || command === "save_kimi_key") {
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
  const [bootstrap, setBootstrap] = useState<BootstrapStatus | null>(null)
  const shouldInspectBootstrap =
    guide.kind === "cli" || providerId === "openrouter" || providerId === "kimi"
  const [loadingBootstrap, setLoadingBootstrap] = useState(shouldInspectBootstrap)

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
    setBootstrap(null)

    if (!apiKeyGuide) {
      setHasExistingKey(false)
      setLoadingExistingKey(false)
    }

    if (!shouldInspectBootstrap) {
      setLoadingBootstrap(false)
    } else {
      setLoadingBootstrap(true)
    }

    void (async () => {
      if (apiKeyGuide) {
        setLoadingExistingKey(true)
        if (!inTauri()) {
          if (!cancelled) {
            setHasExistingKey(false)
            setLoadingExistingKey(false)
          }
        } else {
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
        }
      }

      if (!shouldInspectBootstrap) {
        return
      }

      if (!inTauri()) {
        if (!cancelled) {
          setLoadingBootstrap(false)
        }
        return
      }

      try {
        const result = await invokeCommand<BootstrapStatus>("inspect_provider_bootstrap", {
          provider: providerId
        })
        if (!cancelled) {
          setBootstrap(result)
        }
      } catch (commandError) {
        if (!cancelled) {
          const message = commandError instanceof Error ? commandError.message : String(commandError)
          setError(message)
        }
      } finally {
        if (!cancelled) {
          setLoadingBootstrap(false)
        }
      }
    })()

    return () => {
      cancelled = true
    }
  }, [apiKeyGuide, guide.kind, providerId, shouldInspectBootstrap])

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

  async function handleAgentLaunch(agent: DelegatingAgent) {
    if (!inTauri()) {
      setError(t("connection.packagedOnly"))
      return
    }

    setBusy(true)
    setError(null)
    setLaunchHint(null)

    try {
      await invokeCommand<void>("run_agent_connect_command", { provider: providerId, agent })
      window.setTimeout(() => {
        setLaunchHint(t("connection.agentRunning", { agent: agentDisplayName(agent) }))
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
  const hasManualBlocker =
    bootstrap?.steps.some((step) => step.status === "unavailable") ?? false
  const allBootstrapReady =
    bootstrap?.steps.length ? bootstrap.steps.every((step) => step.status === "ready") : false
  const canLaunchBootstrap =
    guide.kind === "cli" ? Boolean(bootstrap?.commandAvailable) && !loadingBootstrap : true
  const availableAgents = (bootstrap?.availableAgents ?? []).filter(
    (agent): agent is DelegatingAgent =>
      (agent === "codex" || agent === "claude") && agent !== providerId
  )
  const showAgentBootstrap =
    bootstrap?.recommendedMode === "agent" && availableAgents.length > 0 && !loadingBootstrap
  const showShellFallback = guide.kind === "cli" && Boolean(bootstrap?.commandAvailable)
  const showInlineAgentLinks = showAgentBootstrap || showShellFallback
  const agentLinkClass =
    "text-[11px] text-fg-muted underline underline-offset-2 hover:text-fg transition-colors disabled:opacity-50"

  function agentDisplayName(agent: DelegatingAgent) {
    return agent === "codex" ? "Codex" : "Claude"
  }

  function stepLabel(stepId: string) {
    switch (stepId) {
      case "homebrew":
        return t("connection.bootstrapStep.homebrew")
      case "winget":
        return t("connection.bootstrapStep.winget")
      case "nodejs":
        return t("connection.bootstrapStep.nodejs")
      case "claude_cli":
        return t("connection.bootstrapStep.claudeCli")
      case "codex_cli":
        return t("connection.bootstrapStep.codexCli")
      case "gh_cli":
        return t("connection.bootstrapStep.ghCli")
      case "provider_auth":
        return t("connection.bootstrapStep.providerAuth")
      default:
        return stepId
    }
  }

  function stepBadge(stepStatus: BootstrapStep["status"]) {
    if (stepStatus === "ready") {
      return <Badge tone="good">{t("connection.bootstrapReady")}</Badge>
    }

    if (stepStatus === "action_required") {
      return <Badge tone="warn">{t("connection.bootstrapActionRequired")}</Badge>
    }

    return <Badge tone="danger">{t("connection.bootstrapManualRequired")}</Badge>
  }

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
          <>
            <ol className="mt-4 list-decimal space-y-2 pl-4 text-xs text-fg-secondary">
              {guide.steps.map((step) => (
                <li key={step}>{step}</li>
              ))}
            </ol>
            <p className="mt-3 text-xs text-fg-muted">{t("connection.autoInstallHint")}</p>
            {loadingBootstrap ? (
              <p className="mt-3 text-xs text-fg-muted">{t("connection.bootstrapChecking")}</p>
            ) : bootstrap ? (
              <div className="mt-3 space-y-2 rounded-lg border border-border bg-surface-0 p-3">
                {bootstrap.steps.map((step) => (
                  <div key={step.id} className="rounded-md border border-border/70 bg-surface-1 p-2">
                    <div className="flex items-center justify-between gap-3">
                      <span className="text-xs font-medium text-fg">{stepLabel(step.id)}</span>
                      {stepBadge(step.status)}
                    </div>
                    {step.detail ? (
                      <p className="mt-1 text-[11px] text-fg-secondary">{step.detail}</p>
                    ) : null}
                  </div>
                ))}
                {hasManualBlocker ? (
                  <p className="text-[11px] text-danger">{t("connection.bootstrapManualRequired")}</p>
                ) : null}
              </div>
            ) : null}
          </>
        ) : null}

        {guide.kind === "oauth" ? (
          <p className="mt-4 text-xs text-fg-secondary">
            {t("connection.completeOauth")}
          </p>
        ) : null}

        {showTerminalLaunch || showInlineAgentLinks ? (
          <div className="mt-3 flex flex-wrap items-center gap-3">
            {showTerminalLaunch ? (
              <button
                type="button"
                className="inline-flex items-center gap-1 text-xs text-page-accent transition-opacity hover:opacity-80"
                onClick={() => void openExternal(guide.docsUrl)}
              >
                {t("connection.openDocs")}
                <ExternalLink className="h-3.5 w-3.5" />
              </button>
            ) : null}
            {showAgentBootstrap
              ? availableAgents.map((agent) => (
                  <button
                    key={agent}
                    type="button"
                    className={agentLinkClass}
                    onClick={() => void handleAgentLaunch(agent)}
                    disabled={busy}
                  >
                    {t("connection.agentButton", { agent: agentDisplayName(agent) })}
                  </button>
                ))
              : null}
            {showShellFallback ? (
              <button
                type="button"
                className={agentLinkClass}
                onClick={() => void handleLaunch()}
                disabled={busy}
              >
                {t("connection.shellFallback")}
              </button>
            ) : null}
          </div>
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

        <div className="mt-4 space-y-3">
          {apiKeyGuide ? (
            <>
              {hasExistingKey ? (
                <div className="flex flex-wrap justify-end gap-2">
                  <Button size="xs" variant="ghost" onClick={() => void handleSave()} disabled={busy}>
                    {t("connection.replaceKey")}
                  </Button>
                  <Button size="xs" variant="ghost" onClick={() => void handleRemove()} disabled={busy}>
                    {t("connection.removeKey")}
                  </Button>
                </div>
              ) : null}
              <div className="flex justify-end">
                <Button
                  ref={primaryButtonRef}
                  variant="accent"
                  className="w-full justify-center"
                  onClick={() => void handleSave()}
                  disabled={busy}
                >
                  {t("common.save")}
                </Button>
              </div>
            </>
          ) : showAgentBootstrap ? (
            <>
              <div className="flex flex-wrap justify-end gap-2">
                <Button
                  ref={primaryButtonRef}
                  variant="ghost"
                  onClick={() => void handleRefresh()}
                  disabled={busy || loadingBootstrap}
                >
                  {t("connection.iveSetUp")}
                </Button>
              </div>
            </>
          ) : (
            <div className="flex flex-wrap justify-end gap-2">
              <Button
                ref={primaryButtonRef}
                variant="accent"
                onClick={() => void (allBootstrapReady ? handleRefresh() : handleLaunch())}
                disabled={busy || !canLaunchBootstrap}
              >
                {allBootstrapReady ? t("connection.iveSetUp") : t("connection.launchCli")}
              </Button>
              <Button variant="ghost" onClick={() => void handleRefresh()} disabled={busy || loadingBootstrap}>
                {t("connection.iveSetUp")}
              </Button>
            </div>
          )}
        </div>
      </div>
    </div>
  )
}
