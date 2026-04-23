import { cn } from "../../lib/cn"

interface SwitchProps {
  checked: boolean
  onChange: (value: boolean) => void
  disabled?: boolean
  ariaLabel?: string
}

export function Switch({ checked, onChange, disabled, ariaLabel }: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={ariaLabel}
      aria-disabled={disabled || undefined}
      disabled={disabled}
      onClick={() => onChange(!checked)}
      className={cn(
        "inline-flex h-5 w-9 shrink-0 cursor-pointer items-center rounded-full border border-border transition-colors",
        checked ? "bg-page-accent" : "border-border-strong bg-surface-2",
        disabled && "opacity-50 cursor-not-allowed"
      )}
    >
      <span
        className={cn(
          "ml-0.5 h-4 w-4 rounded-full bg-[color:var(--color-switch-handle)] shadow-sm transition-transform",
          checked ? "translate-x-4" : "translate-x-0"
        )}
      />
    </button>
  )
}
