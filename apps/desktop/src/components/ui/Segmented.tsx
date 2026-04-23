import { useId, useRef } from "react"
import { cn } from "../../lib/cn"

type SegmentedSize = "sm" | "md"

const sizeClasses: Record<SegmentedSize, string> = {
  sm: "px-2 py-1 text-[11px]",
  md: "px-2.5 py-1.5 text-xs"
}

export function Segmented<T extends string | number>({
  value,
  options,
  onChange,
  disabled = false,
  size = "md"
}: {
  value: T
  options: readonly { value: T; label: string }[]
  onChange: (next: T) => void
  disabled?: boolean
  size?: SegmentedSize
}) {
  const listId = useId()
  const refs = useRef<Array<HTMLButtonElement | null>>([])

  function moveFocus(index: number) {
    refs.current[index]?.focus()
  }

  function handleKeyDown(index: number, event: React.KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "ArrowRight" || event.key === "ArrowDown") {
      event.preventDefault()
      moveFocus((index + 1) % options.length)
      return
    }
    if (event.key === "ArrowLeft" || event.key === "ArrowUp") {
      event.preventDefault()
      moveFocus((index - 1 + options.length) % options.length)
      return
    }
    if (event.key === "Home") {
      event.preventDefault()
      moveFocus(0)
      return
    }
    if (event.key === "End") {
      event.preventDefault()
      moveFocus(options.length - 1)
    }
  }

  return (
    <div
      className="inline-flex rounded-md border border-border bg-surface-2 p-0.5"
      role="tablist"
      aria-orientation="horizontal"
    >
      {options.map((opt, index) => (
        <button
          key={String(opt.value)}
          id={`${listId}-${String(opt.value)}`}
          ref={(node) => {
            refs.current[index] = node
          }}
          type="button"
          role="tab"
          aria-selected={value === opt.value}
          tabIndex={value === opt.value ? 0 : -1}
          disabled={disabled}
          onClick={() => onChange(opt.value)}
          onKeyDown={(event) => handleKeyDown(index, event)}
          className={cn(
            "cursor-pointer rounded transition-colors",
            sizeClasses[size],
            disabled && "cursor-not-allowed opacity-50",
            value === opt.value
              ? "bg-surface-popover text-fg-primary shadow-sm"
              : "text-fg-muted hover:text-fg-primary"
          )}
        >
          {opt.label}
        </button>
      ))}
    </div>
  )
}
