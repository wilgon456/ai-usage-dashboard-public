import type { MouseEvent } from "react"
import type { LucideIcon } from "lucide-react"
import { Button } from "./ui/Button"

function actionHandler(action?: { label: string; onClick: () => void }) {
  if (!action) return undefined

  return (event: MouseEvent<HTMLButtonElement>) => {
    event.stopPropagation()
    action.onClick()
  }
}

export function EmptyStateInline({
  Icon,
  title,
  description,
  primaryAction,
  secondaryAction
}: {
  Icon: LucideIcon
  title: string
  description?: string
  primaryAction?: { label: string; onClick: () => void; ariaLabel?: string }
  secondaryAction?: { label: string; onClick: () => void; ariaLabel?: string }
}) {
  return (
    <div className="flex items-start gap-3 rounded-lg bg-bg/40 p-3">
      <Icon className="mt-0.5 h-4 w-4 shrink-0 text-muted" />
      <div className="min-w-0 flex-1">
        <p className="text-xs font-medium text-fg">{title}</p>
        {description ? <p className="mt-0.5 text-[11px] text-muted">{description}</p> : null}
        {primaryAction || secondaryAction ? (
          <div className="mt-2 flex gap-2">
            {primaryAction ? (
              <Button
                size="xs"
                onClick={actionHandler(primaryAction)}
                aria-label={primaryAction.ariaLabel}
              >
                {primaryAction.label}
              </Button>
            ) : null}
            {secondaryAction ? (
              <Button
                size="xs"
                variant="ghost"
                onClick={actionHandler(secondaryAction)}
                aria-label={secondaryAction.ariaLabel}
              >
                {secondaryAction.label}
              </Button>
            ) : null}
          </div>
        ) : null}
      </div>
    </div>
  )
}
