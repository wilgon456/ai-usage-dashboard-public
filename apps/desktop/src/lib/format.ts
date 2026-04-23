import type { MetricFormat, MetricLine } from "@ai-usage-dashboard/core"
import type { Locale } from "@ai-usage-dashboard/core"
import type { TFunction } from "../i18n"

export function formatMetricValue(value: number, format: MetricFormat): string {
  switch (format.kind) {
    case "percent":
      return `${Math.round(value)}%`
    case "count":
      return `${Math.round(value)} ${format.suffix}`
    case "currency":
      return new Intl.NumberFormat("en-US", {
        style: "currency",
        currency: format.currency
      }).format(value)
  }
}

export function formatLineValue(line: MetricLine): string {
  if (line.type === "progress") {
    return formatMetricValue(line.used, line.format)
  }
  return line.value
}

export function formatRelative(isoString: string | undefined, t: TFunction): string {
  if (!isoString) return t("common.none")
  const now = Date.now()
  const target = new Date(isoString).getTime()
  const diff = target - now
  const abs = Math.abs(diff)
  const minutes = Math.round(abs / 60_000)
  const hours = Math.round(abs / 3_600_000)
  const days = Math.round(abs / 86_400_000)

  if (abs < 60_000) return t("common.justNow")
  if (minutes < 60) {
    return t(diff > 0 ? "common.minutesFromNow" : "common.minutesAgo", { count: minutes })
  }
  if (hours < 48) {
    return t(diff > 0 ? "common.hoursFromNow" : "common.hoursAgo", { count: hours })
  }
  return t(diff > 0 ? "common.daysFromNow" : "common.daysAgo", { count: days })
}

export function formatAbsolute(
  isoString: string | undefined,
  locale: Locale,
  t: TFunction
): string {
  if (!isoString) return t("common.none")
  try {
    return new Date(isoString).toLocaleString(locale === "ko" ? "ko-KR" : "en-US", {
      month: "short",
      day: "numeric",
      hour: "2-digit",
      minute: "2-digit"
    })
  } catch {
    return isoString
  }
}
