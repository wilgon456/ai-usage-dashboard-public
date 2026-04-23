import { HelpCircle, House, Settings } from "lucide-react"
import type { ProviderDefinition } from "@ai-usage-dashboard/core"
import type { TFunction } from "../i18n"
import { cn } from "../lib/cn"
import { openExternal } from "../lib/open-url"
import { providerLogo } from "../lib/provider-logos"
import { useUiStore, type ActiveView } from "../stores/ui-store"

const ISSUES_URL = "https://github.com/ai-usage-dashboard/ai-usage-dashboard/issues"

export function SideNav({
  providers,
  t
}: {
  providers: ProviderDefinition[]
  t: TFunction
}) {
  const active = useUiStore((s) => s.activeView)
  const setActive = useUiStore((s) => s.setActiveView)

  const NavButton = ({
    view,
    onClick,
    children,
    accent,
    title
  }: {
    view?: ActiveView
    onClick?: () => void
    children: React.ReactNode
    accent?: string
    title: string
  }) => {
    const isActive = view !== undefined && active === view

    return (
      <button
        type="button"
        title={title}
        aria-label={title}
        onClick={onClick ?? (() => view && setActive(view))}
        className={cn(
          "relative flex h-9 w-9 cursor-pointer items-center justify-center rounded-lg text-xs font-semibold transition-colors",
          isActive
            ? "bg-surface-2 text-fg-primary"
            : "text-fg-secondary hover:bg-surface-2 hover:text-fg-primary"
        )}
        style={accent && isActive ? { boxShadow: `inset 0 0 0 1px ${accent}55` } : undefined}
      >
        {isActive ? (
          <span className="absolute left-0 h-5 w-0.5 -translate-x-2 rounded-r bg-page-accent" />
        ) : null}
        {children}
      </button>
    )
  }

  return (
    <nav className="flex w-12 flex-col items-center gap-1.5 border-r border-border bg-surface-0 py-3">
      <NavButton view="home" title={t("nav.home")}>
        <House className="h-4 w-4" />
      </NavButton>

      <div className="my-1 h-px w-6 bg-border" />

      {providers.map((p) => (
        <NavButton
          key={p.id}
          view={p.id}
          accent={p.brandColor}
          title={p.displayName}
        >
          <span
            aria-hidden="true"
            className="h-4 w-4"
            style={{
              backgroundColor: p.brandColor,
              WebkitMaskImage: `url("${providerLogo[p.id]}")`,
              WebkitMaskPosition: "center",
              WebkitMaskRepeat: "no-repeat",
              WebkitMaskSize: "contain",
              maskImage: `url("${providerLogo[p.id]}")`,
              maskPosition: "center",
              maskRepeat: "no-repeat",
              maskSize: "contain"
            }}
          />
        </NavButton>
      ))}

      <div className="flex-1" />

      <NavButton onClick={() => void openExternal(ISSUES_URL)} title={t("nav.help")}>
        <HelpCircle className="h-4 w-4" />
      </NavButton>
      <NavButton view="settings" title={t("nav.settings")}>
        <Settings className="h-4 w-4" />
      </NavButton>
    </nav>
  )
}
