import type { AppSettings, ProviderId, ProviderSnapshotState } from "@ai-usage-dashboard/core"
import type { TFunction } from "../i18n"
import { ProviderCard } from "../components/ProviderCard"
import { useUiStore } from "../stores/ui-store"

export function ProviderDetail({
  state,
  settings,
  onRefresh,
  onToggleProvider,
  t
}: {
  state: ProviderSnapshotState
  settings: AppSettings
  onRefresh: () => void
  onToggleProvider: (providerId: ProviderId) => void
  t: TFunction
}) {
  const setConnectionModalFor = useUiStore((s) => s.setConnectionModalFor)

  return (
    <section className="flex flex-col gap-3">
      <header className="flex flex-col gap-0.5 px-1 pt-1">
        <h1 className="text-sm font-semibold text-fg">{state.provider.displayName}</h1>
        <p className="text-[11px] text-fg-muted">{t("provider.detailSubtitle")}</p>
      </header>
      <ProviderCard
        state={state}
        settings={settings}
        onRefresh={onRefresh}
        onToggleProvider={() => onToggleProvider(state.provider.id)}
        onOpenConnectionGuide={() => setConnectionModalFor(state.provider.id)}
        t={t}
      />
    </section>
  )
}
