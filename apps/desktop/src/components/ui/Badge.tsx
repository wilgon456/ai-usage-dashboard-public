import type { CSSProperties, ComponentPropsWithoutRef, ReactNode } from "react"
import { cn } from "../../lib/cn"

type Tone = "neutral" | "good" | "warn" | "danger" | "live" | "outline"

interface BadgeProps extends ComponentPropsWithoutRef<"span"> {
  tone?: Tone
  children: ReactNode
}

const toneClasses: Record<Tone, string> = {
  neutral: "bg-surface-2 text-fg-secondary border-border",
  good: "text-good",
  warn: "text-warn",
  danger: "text-danger",
  live: "text-page-accent",
  outline: "bg-transparent text-fg-secondary border-border-strong"
}

const toneStyles: Partial<Record<Tone, CSSProperties>> = {
  good: {
    backgroundColor: "color-mix(in oklch, var(--color-good), transparent 86%)",
    borderColor: "color-mix(in oklch, var(--color-good), transparent 72%)"
  },
  warn: {
    backgroundColor: "color-mix(in oklch, var(--color-warn), transparent 84%)",
    borderColor: "color-mix(in oklch, var(--color-warn), transparent 70%)"
  },
  danger: {
    backgroundColor: "color-mix(in oklch, var(--color-danger), transparent 86%)",
    borderColor: "color-mix(in oklch, var(--color-danger), transparent 72%)"
  },
  live: {
    backgroundColor: "color-mix(in oklch, var(--color-page-accent), transparent 82%)",
    borderColor: "color-mix(in oklch, var(--color-page-accent), transparent 68%)"
  }
}

export function Badge({ tone = "neutral", children, className, style, ...props }: BadgeProps) {
  return (
    <span
      {...props}
      style={{ ...toneStyles[tone], ...style }}
      className={cn(
        "inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[11px] font-medium",
        toneClasses[tone],
        className
      )}
    >
      {children}
    </span>
  )
}
