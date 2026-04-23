import { cn } from "../../lib/cn"

interface ProgressProps {
  value: number
  max?: number
  color?: string
  className?: string
  label?: string
}

export function Progress({ value, max = 100, color, className, label }: ProgressProps) {
  const pct = Math.max(0, Math.min(100, (value / max) * 100))
  return (
    <div
      className={cn("h-1.5 w-full overflow-hidden rounded-full bg-surface-2", className)}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(pct)}
    >
      <div
        className="h-full rounded-full transition-[width] duration-300"
        style={{ width: `${pct}%`, background: color ?? "var(--color-page-accent)" }}
      />
    </div>
  )
}
