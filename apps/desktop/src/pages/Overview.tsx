import type { AppSettings, ProviderId, ProviderSnapshotState } from "@ai-usage-dashboard/core"
import type { TFunction } from "../i18n"
import { ProviderCard } from "../components/ProviderCard"
import { useUiStore } from "../stores/ui-store"

export function Overview({
  states,
  settings,
  onRefresh,
  onToggleProvider,
  t
}: {
  states: ProviderSnapshotState[]
  settings: AppSettings
  onRefresh: () => void
  onToggleProvider: (providerId: ProviderId) => void
  t: TFunction
}) {
  const setActive = useUiStore((s) => s.setActiveView)

  return (
    <section className="flex flex-col gap-3">
      <header className="flex flex-col gap-0.5 px-1 pt-1">
        <h1 className="text-sm font-semibold text-fg">{t("home.providersTitle")}</h1>
        <p className="text-[11px] text-fg-muted">
          {t("home.providersSubtitle")}
        </p>
      </header>

      <div className="flex flex-col gap-2.5">
        {states.map((state) => (
          <ProviderCard
            key={state.provider.id}
            state={state}
            settings={settings}
            compact={true}
            onClick={() => setActive(state.provider.id)}
            onRefresh={onRefresh}
            onToggleProvider={() => onToggleProvider(state.provider.id)}
            t={t}
          />
        ))}
      </div>
    </section>
  )
}
