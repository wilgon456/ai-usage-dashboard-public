import type { MetricLine } from "@ai-usage-dashboard/core"
import type { TFunction } from "../i18n"
import { Badge } from "./ui/Badge"
import { Progress } from "./ui/Progress"
import { formatLineValue, formatRelative } from "../lib/format"
import { usePreferencesStore } from "../stores/preferences-store"

export function MetricLineView({
  line,
  accent,
  t
}: {
  line: MetricLine
  accent?: string
  t: TFunction
}) {
  const displayMode = usePreferencesStore((s) => s.settings.displayMode)

  if (line.type === "progress") {
    const shown = displayMode === "left" ? Math.max(0, line.limit - line.used) : line.used
    const pct = Math.max(0, Math.min(100, (shown / line.limit) * 100))
    const suffix = displayMode === "left" ? t("metric.leftSuffix") : ""
    return (
      <div className="flex flex-col gap-1.5">
        <div className="flex items-baseline justify-between gap-2">
          <span className="text-xs text-fg-secondary">{line.label}</span>
          <div className="flex items-baseline gap-2">
            <span className="text-sm font-semibold text-fg tabular-nums">
              {line.format.kind === "currency"
                ? new Intl.NumberFormat("en-US", {
                    style: "currency",
                    currency: "USD"
                  }).format(shown)
                : `${Math.round(pct)}%${suffix}`}
            </span>
            {line.resetsAt ? (
              <span className="text-[11px] tabular-nums text-fg-muted">
                {formatRelative(line.resetsAt, t)} {t("common.resets")}
              </span>
            ) : null}
          </div>
        </div>
        <Progress
          value={shown}
          max={line.limit}
          color={line.color ?? accent}
          label={line.label}
        />
      </div>
    )
  }

  if (line.type === "badge") {
    const tone = (line.tone ?? "neutral") as "neutral" | "good" | "warn" | "danger"
    return (
      <div className="flex items-center justify-between">
        <span className="text-xs text-fg-secondary">{line.label}</span>
        <Badge tone={tone}>{line.value}</Badge>
      </div>
    )
  }

  return (
    <div className="flex items-baseline justify-between">
      <span className="text-xs text-fg-secondary">{line.label}</span>
      <span className="text-sm font-medium text-fg tabular-nums">{formatLineValue(line)}</span>
    </div>
  )
}
