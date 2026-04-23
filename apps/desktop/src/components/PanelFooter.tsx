import { RefreshCw } from "lucide-react"
import type { TFunction } from "../i18n"
import { cn } from "../lib/cn"
import { useUiStore } from "../stores/ui-store"
import { formatRelative } from "../lib/format"

export function PanelFooter({
  version,
  onRefresh,
  t
}: {
  version: string
  onRefresh: () => void
  t: TFunction
}) {
  const refreshing = useUiStore((s) => s.refreshing)
  const lastRefreshedAt = useUiStore((s) => s.lastRefreshedAt)
  const label = lastRefreshedAt
    ? t("footer.updated", { time: formatRelative(lastRefreshedAt, t) })
    : t("footer.neverRefreshed")

  return (
    <footer className="flex items-center justify-between border-t border-border bg-surface-0 px-3 py-2 text-[11px] text-fg-muted">
      <span className="truncate">
        <span className="text-fg-primary">AI_Usage_Dashboard</span>
        <span className="ml-1">v{version}</span>
      </span>
      <button
        type="button"
        onClick={onRefresh}
        disabled={refreshing}
        aria-label={t("footer.refreshAction")}
        className={cn(
          "inline-flex items-center gap-1.5 rounded-md px-2 py-1 text-[11px] transition-colors",
          refreshing
            ? "cursor-not-allowed text-fg-muted"
            : "text-fg-secondary hover:bg-surface-2 hover:text-fg-primary"
        )}
      >
        <RefreshCw className={cn("h-3 w-3", refreshing && "animate-spin")} />
        <span>{label}</span>
      </button>
    </footer>
  )
}
