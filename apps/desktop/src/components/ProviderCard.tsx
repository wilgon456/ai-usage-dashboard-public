import type {
  AppSettings,
  ProviderCardState,
  ProviderSnapshotState
} from "@ai-usage-dashboard/core"
import { AlertTriangle, Link2Off } from "lucide-react"
import type { TFunction } from "../i18n"
import { toCardState } from "../lib/card-state"
import { computeProviderMax } from "../lib/notifications"
import { providerLogo } from "../lib/provider-logos"
import { EmptyStateInline } from "./EmptyStateInline"
import { MetricLineView } from "./MetricLineView"
import { Badge } from "./ui/Badge"

function stateBadge(state: ProviderCardState, t: TFunction) {
  if (state.kind === "error") return <Badge tone="danger">{t("card.issue")}</Badge>
  if (state.kind === "unconfigured") return <Badge tone="neutral">{t("card.notConnected")}</Badge>
  if (state.kind === "disabled") return <Badge tone="neutral">{t("card.disabled")}</Badge>
  if (state.kind === "idle") return <Badge tone="neutral">{t("card.idle")}</Badge>
  if (state.kind === "live") return <Badge tone="live">{t("card.live")}</Badge>
  return <Badge tone="neutral">{t("card.cached")}</Badge>
}

function looksCredentialRelated(message: string) {
  return /credential|auth|401/i.test(message)
}

function ProviderLogo({
  providerId,
  accent,
  compact = false
}: {
  providerId: ProviderSnapshotState["provider"]["id"]
  accent: string
  compact?: boolean
}) {
  const outerSize = compact ? "h-7 w-7 rounded-md" : "h-8 w-8 rounded-lg"
  const innerSize = compact ? "h-4 w-4" : "h-5 w-5"

  return (
    <div
      className={`flex shrink-0 items-center justify-center ${outerSize}`}
      style={{ background: `${accent}22` }}
    >
      <span
        aria-hidden="true"
        className={innerSize}
        style={{
          backgroundColor: accent,
          WebkitMaskImage: `url("${providerLogo[providerId]}")`,
          WebkitMaskPosition: "center",
          WebkitMaskRepeat: "no-repeat",
          WebkitMaskSize: "contain",
          maskImage: `url("${providerLogo[providerId]}")`,
          maskPosition: "center",
          maskRepeat: "no-repeat",
          maskSize: "contain"
        }}
      />
    </div>
  )
}

function CompactSummary({
  state,
  cardState,
  accent,
  displayMode,
  t
}: {
  state: ProviderSnapshotState
  cardState: Extract<ProviderCardState, { kind: "live" | "cached" }>
  accent: string
  displayMode: AppSettings["displayMode"]
  t: TFunction
}) {
  const max = computeProviderMax(state)
  const usedRatio = max == null ? 0 : Math.max(0, Math.min(max, 1))
  const shownRatio = displayMode === "left" ? 1 - usedRatio : usedRatio
  const label = displayMode === "left" ? t("card.displayLeft") : t("card.displayUsed")
  const percentLabel = max == null ? t("common.notAvailable") : `${Math.round(shownRatio * 100)}%`

  return (
    <div className="flex items-center gap-3">
      <ProviderLogo providerId={state.provider.id} accent={accent} compact />
      <span className="min-w-0 shrink-0 truncate text-sm font-semibold text-fg">
        {state.provider.displayName}
      </span>
      <div className="min-w-0 flex-1">
        <div className="flex items-center justify-between gap-2 text-[11px] text-muted">
          <span>{label}</span>
          <span className="text-fg">{percentLabel}</span>
        </div>
        <div className="mt-1 h-1.5 overflow-hidden rounded-full bg-surface-2">
          <div
            className="h-full rounded-full transition-[width]"
            style={{ width: `${shownRatio * 100}%`, backgroundColor: accent }}
          />
        </div>
      </div>
      {stateBadge(cardState, t)}
    </div>
  )
}

function CompactStatusSummary({
  state,
  cardState,
  accent,
  t
}: {
  state: ProviderSnapshotState
  cardState: Extract<ProviderCardState, { kind: "live" | "cached" }>
  accent: string
  t: TFunction
}) {
  return (
    <div className="flex flex-col gap-2">
      <div className="flex items-center gap-3">
        <ProviderLogo providerId={state.provider.id} accent={accent} compact />
        <span className="min-w-0 flex-1 truncate text-sm font-semibold text-fg">
          {state.provider.displayName}
        </span>
        {stateBadge(cardState, t)}
      </div>
      <div className="flex flex-col gap-1.5">
        {cardState.snapshot.lines.slice(0, 2).map((line, index) => (
          <MetricLineView
            key={`${line.label}-${index}`}
            line={line}
            accent={accent}
            t={t}
          />
        ))}
      </div>
    </div>
  )
}

export function ProviderCard({
  state,
  settings,
  compact = false,
  onClick,
  onRefresh,
  onOpenConnectionGuide,
  onToggleProvider,
  t
}: {
  state: ProviderSnapshotState
  settings: AppSettings
  compact?: boolean
  onClick?: () => void
  onRefresh?: () => void
  onOpenConnectionGuide?: () => void
  onToggleProvider?: () => void
  t: TFunction
}) {
  const { provider } = state
  const accent = provider.brandColor
  const cardState = toCardState(state, settings)
  const onCardKeyDown = onClick
    ? (event: React.KeyboardEvent<HTMLElement>) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault()
          onClick()
        }
      }
    : undefined
  const actionAriaLabel = (actionLabel: string) =>
    t("aria.providerAction", {
      provider: provider.displayName,
      action: actionLabel
    })
  const hasCompactProgress =
    (cardState.kind === "live" || cardState.kind === "cached") &&
    state.snapshot?.lines.some((line) => line.type === "progress")

  if (
    compact &&
    (cardState.kind === "live" || cardState.kind === "cached") &&
    hasCompactProgress
  ) {
    return (
      <article
        className={
          "group relative rounded-xl border border-border bg-card-muted/60 p-3 transition-colors hover:bg-card-muted " +
          (onClick ? "cursor-pointer" : "")
        }
        onClick={onClick}
        onKeyDown={onCardKeyDown}
        role={onClick ? "button" : undefined}
        tabIndex={onClick ? 0 : undefined}
      >
        <CompactSummary
          state={state}
          cardState={cardState}
          accent={accent}
          displayMode={settings.displayMode}
          t={t}
        />
      </article>
    )
  }

  if (
    compact &&
    (cardState.kind === "live" || cardState.kind === "cached") &&
    !hasCompactProgress
  ) {
    return (
      <article
        className={
          "group relative rounded-xl border border-border bg-card-muted/60 p-3 transition-colors hover:bg-card-muted " +
          (onClick ? "cursor-pointer" : "")
        }
        onClick={onClick}
        onKeyDown={onCardKeyDown}
        role={onClick ? "button" : undefined}
        tabIndex={onClick ? 0 : undefined}
      >
        <CompactStatusSummary
          state={state}
          cardState={cardState}
          accent={accent}
          t={t}
        />
      </article>
    )
  }

  if (compact && (cardState.kind === "unconfigured" || cardState.kind === "disabled")) {
    return (
      <article
        className={
          "group relative rounded-xl border border-border bg-card-muted/60 p-3 transition-colors hover:bg-card-muted " +
          (onClick ? "cursor-pointer" : "")
        }
        onClick={onClick}
        onKeyDown={onCardKeyDown}
        role={onClick ? "button" : undefined}
        tabIndex={onClick ? 0 : undefined}
      >
        <div className="flex items-center gap-3">
          <ProviderLogo providerId={state.provider.id} accent={accent} compact />
          <span className="min-w-0 flex-1 truncate text-sm font-semibold text-fg">
            {state.provider.displayName}
          </span>
          {stateBadge(cardState, t)}
        </div>
      </article>
    )
  }

  return (
    <article
      className={
        "group relative flex flex-col gap-3 rounded-xl border border-border bg-card-muted/60 p-3 transition-colors hover:bg-card-muted " +
        (onClick ? "cursor-pointer" : "")
      }
      onClick={onClick}
      onKeyDown={onCardKeyDown}
      role={onClick ? "button" : undefined}
      tabIndex={onClick ? 0 : undefined}
    >
      <header className="flex items-center gap-3">
        <ProviderLogo providerId={provider.id} accent={accent} />
        <div className="flex min-w-0 flex-1 items-center gap-2">
          <span className="truncate text-sm font-semibold text-fg">
            {provider.displayName}
          </span>
          {"snapshot" in cardState && cardState.snapshot.plan ? (
            <Badge
              tone="outline"
              title={cardState.kind === "idle" ? undefined : cardState.snapshot.plan}
              className="max-w-[40%] truncate"
            >
              {cardState.kind === "idle" ? t("card.refreshing") : cardState.snapshot.plan}
            </Badge>
          ) : null}
        </div>
        {stateBadge(cardState, t)}
      </header>

      {cardState.kind === "live" || cardState.kind === "cached" ? (
        <div className="flex flex-col gap-2.5">
          {cardState.snapshot.lines.map((line, i) => (
            <MetricLineView key={`${line.label}-${i}`} line={line} accent={accent} t={t} />
          ))}
        </div>
      ) : null}

      {cardState.kind === "idle" ? (
        <div className="flex items-center gap-2 text-[11px] text-muted">
          <span className="h-3 w-3 animate-spin rounded-full border border-border-strong border-t-fg-secondary" />
          {t("card.refreshing")}
        </div>
      ) : null}

      {cardState.kind === "unconfigured" ? (
        <EmptyStateInline
          Icon={Link2Off}
          title={t("card.notConnectedTitle")}
          description={
            provider.connectionGuide?.title ?? t("card.notConnectedDescription")
          }
          primaryAction={
            onOpenConnectionGuide
              ? {
                  label: t("card.connect"),
                  onClick: onOpenConnectionGuide,
                  ariaLabel: actionAriaLabel(t("card.connect"))
                }
              : undefined
          }
        />
      ) : null}

      {cardState.kind === "disabled" ? (
        <EmptyStateInline
          Icon={Link2Off}
          title={t("card.disabledTitle")}
          primaryAction={
            onToggleProvider
              ? {
                  label: t("card.enable"),
                  onClick: onToggleProvider,
                  ariaLabel: actionAriaLabel(t("card.enable"))
                }
              : undefined
          }
        />
      ) : null}

      {cardState.kind === "error" ? (
        <EmptyStateInline
          Icon={AlertTriangle}
          title={t("card.errorTitle")}
          description={cardState.message}
          primaryAction={
            cardState.retryable
              ? {
                  label: t("common.retry"),
                  onClick: () => onRefresh?.(),
                  ariaLabel: actionAriaLabel(t("common.retry"))
                }
              : looksCredentialRelated(cardState.message) && onOpenConnectionGuide
                ? {
                    label: t("connection.openDocs"),
                    onClick: onOpenConnectionGuide,
                    ariaLabel: actionAriaLabel(t("connection.openDocs"))
                  }
                : undefined
          }
          secondaryAction={
            cardState.retryable &&
            looksCredentialRelated(cardState.message) &&
            onOpenConnectionGuide
              ? {
                  label: t("connection.openDocs"),
                  onClick: onOpenConnectionGuide,
                  ariaLabel: actionAriaLabel(t("connection.openDocs"))
                }
              : undefined
          }
        />
      ) : null}
    </article>
  )
}
