import { useId, useRef } from "react"
import type { ReactNode } from "react"
import { cn } from "../../lib/cn"

interface TabItem {
  id: string
  label: string
}

interface TabsProps {
  items: TabItem[]
  active: string
  onChange: (id: string) => void
  children: ReactNode
  className?: string
}

export function Tabs({ items, active, onChange, children, className }: TabsProps) {
  const tabsId = useId()
  const refs = useRef<Array<HTMLButtonElement | null>>([])
  const activeIndex = Math.max(
    0,
    items.findIndex((item) => item.id === active)
  )

  function moveFocus(index: number) {
    refs.current[index]?.focus()
  }

  function handleKeyDown(index: number, event: React.KeyboardEvent<HTMLButtonElement>) {
    if (event.key === "ArrowRight") {
      event.preventDefault()
      moveFocus((index + 1) % items.length)
      return
    }
    if (event.key === "ArrowLeft") {
      event.preventDefault()
      moveFocus((index - 1 + items.length) % items.length)
      return
    }
    if (event.key === "Home") {
      event.preventDefault()
      moveFocus(0)
      return
    }
    if (event.key === "End") {
      event.preventDefault()
      moveFocus(items.length - 1)
    }
  }

  return (
    <div className={cn("flex flex-col gap-3", className)}>
      <div
        className="inline-flex gap-1 rounded-lg bg-surface-2 p-1"
        role="tablist"
        aria-orientation="horizontal"
      >
        {items.map((item, index) => (
          <button
            key={item.id}
            id={`${tabsId}-tab-${item.id}`}
            ref={(node) => {
              refs.current[index] = node
            }}
            type="button"
            role="tab"
            aria-selected={active === item.id}
            aria-controls={`${tabsId}-panel-${item.id}`}
            tabIndex={active === item.id ? 0 : -1}
            onClick={() => onChange(item.id)}
            onKeyDown={(event) => handleKeyDown(index, event)}
            className={cn(
              "cursor-pointer rounded-md px-3 py-2 text-xs transition-colors",
              active === item.id
                ? "bg-surface-1 text-fg-primary"
                : "text-fg-muted hover:text-fg-primary"
            )}
          >
            {item.label}
          </button>
        ))}
      </div>
      <div
        id={`${tabsId}-panel-${items[activeIndex]?.id ?? active}`}
        role="tabpanel"
        aria-labelledby={`${tabsId}-tab-${items[activeIndex]?.id ?? active}`}
      >
        {children}
      </div>
    </div>
  )
}
