import { forwardRef } from "react"
import type { ButtonHTMLAttributes, ReactNode } from "react"
import { cn } from "../../lib/cn"

type Variant = "default" | "ghost" | "accent" | "danger"
type Size = "xs" | "sm" | "md" | "icon"

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant
  size?: Size
  children?: ReactNode
}

const variantClasses: Record<Variant, string> = {
  default:
    "border border-border bg-surface-1 text-fg-primary hover:bg-surface-2",
  ghost: "bg-transparent text-fg-primary hover:bg-surface-2",
  accent: "bg-page-accent text-black hover:bg-page-accent/90 font-medium",
  danger: "bg-danger/15 text-danger hover:bg-danger/25 border border-danger/30"
}

const sizeClasses: Record<Size, string> = {
  xs: "text-[11px] px-2 py-1 rounded-md",
  sm: "text-xs px-2.5 py-1.5 rounded-md",
  md: "text-sm px-3 py-2 rounded-md",
  icon: "h-8 w-8 rounded-md p-0 inline-flex items-center justify-center"
}

export const Button = forwardRef<HTMLButtonElement, ButtonProps>(function Button(
  { variant = "default", size = "md", className, children, ...rest },
  ref
) {
  return (
    <button
      ref={ref}
      aria-disabled={rest.disabled || undefined}
      className={cn(
        "inline-flex cursor-pointer items-center gap-1.5 transition-colors select-none disabled:cursor-not-allowed disabled:opacity-50",
        variantClasses[variant],
        sizeClasses[size],
        className
      )}
      {...rest}
    >
      {children}
    </button>
  )
})
