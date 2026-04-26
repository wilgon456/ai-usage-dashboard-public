import { useState } from "react"
import type {
  AppSettings,
  DisplayMode,
  Locale,
  ProviderId,
  ProviderSnapshotState,
  RefreshIntervalMinutes,
  ThemeMode
} from "@ai-usage-dashboard/core"
import type { TFunction } from "../i18n"
import { cn } from "../lib/cn"
import { Segmented } from "../components/ui/Segmented"
import { Switch } from "../components/ui/Switch"
import { Tabs } from "../components/ui/Tabs"

type TabId = "display" | "providers" | "system" | "notifications" | "about"

interface SettingsProps {
  settings: AppSettings
  providerStates: ProviderSnapshotState[]
  widgetSyncUrls: string[]
  onToggleProvider: (providerId: ProviderId) => void
  onThemeChange: (mode: ThemeMode) => void
  onLocaleChange: (locale: Locale) => void
  onDisplayModeChange: (mode: DisplayMode) => void
  onRefreshIntervalChange: (value: RefreshIntervalMinutes) => void
  onNotificationsEnabledChange: (value: boolean) => void
  onNotificationThresholdsChange: (value: number[]) => void
  onTrayTargetChange: (value: ProviderId | "max" | "last-viewed") => void
  onStartOnLoginChange: (value: boolean) => void
  onWidgetSyncEnabledChange: (value: boolean) => void
  onWidgetSyncRelayUrlChange: (value: string) => void
  t: TFunction
}

function Row({
  label,
  description,
  children
}: {
  label: string
  description?: string
  children: React.ReactNode
}) {
  return (
    <div className="flex items-start justify-between gap-3 border-b border-border py-2 last:border-b-0">
      <div className="flex min-w-0 flex-col gap-0.5">
        <span className="text-xs text-fg">{label}</span>
        {description ? <span className="text-[11px] text-fg-muted">{description}</span> : null}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
  )
}

export function Settings(props: SettingsProps) {
  const [tab, setTab] = useState<TabId>("display")
  const {
    settings,
    providerStates,
    widgetSyncUrls,
    onToggleProvider,
    onThemeChange,
    onLocaleChange,
    onDisplayModeChange,
    onRefreshIntervalChange,
    onNotificationsEnabledChange,
    onNotificationThresholdsChange,
    onTrayTargetChange,
    onStartOnLoginChange,
    onWidgetSyncEnabledChange,
    onWidgetSyncRelayUrlChange,
    t
  } = props
  const thresholdValues = [
    settings.notificationThresholds[0] ?? 80,
    settings.notificationThresholds[1] ?? 95
  ]
  const trayTargetOptions: Array<{ value: ProviderId | "max" | "last-viewed"; label: string }> = [
    { value: "last-viewed", label: t("settings.trayTarget.lastViewed") },
    { value: "max", label: t("settings.trayTarget.max") },
    ...settings.providerOrder
      .filter((id) => !settings.disabledProviders.includes(id))
      .map((id) => {
        const state = providerStates.find((providerState) => providerState.provider.id === id)
        return {
          value: id,
          label: state?.provider.displayName ?? id
        }
      })
  ]

  function updateThreshold(index: number, value: number) {
    const next = [...thresholdValues]
    next[index] = value
    onNotificationThresholdsChange(next.sort((left, right) => left - right))
  }

  return (
    <section className="flex flex-col gap-3">
      <header className="flex flex-col gap-0.5 px-1 pt-1">
        <h1 className="text-sm font-semibold text-fg">{t("settings.title")}</h1>
      </header>

      <Tabs
        items={[
          { id: "display", label: t("settings.tabs.display") },
          { id: "providers", label: t("settings.tabs.providers") },
          { id: "system", label: t("settings.tabs.system") },
          { id: "notifications", label: t("settings.tabs.notifications") },
          { id: "about", label: t("settings.tabs.about") }
        ]}
        active={tab}
        onChange={(id) => setTab(id as TabId)}
      >
        {tab === "display" ? (
          <div className="flex flex-col gap-1">
            <Row label={t("settings.display.theme")}>
              <Segmented
                value={settings.themeMode}
                options={[
                  { value: "system", label: t("settings.display.themeSystem") },
                  { value: "light", label: t("settings.display.themeLight") },
                  { value: "dark", label: t("settings.display.themeDark") }
                ]}
                onChange={onThemeChange}
              />
            </Row>
            <Row label={t("settings.display.locale")}>
              <Segmented
                value={settings.locale}
                options={[
                  { value: "ko", label: "한국어" },
                  { value: "en", label: "English" }
                ]}
                onChange={(value) => onLocaleChange(value as Locale)}
                size="sm"
              />
            </Row>
            <Row
              label={t("settings.display.displayMode")}
              description={t("settings.display.displayModeDescription")}
            >
              <Segmented
                value={settings.displayMode}
                options={[
                  { value: "used", label: t("settings.display.displayUsed") },
                  { value: "left", label: t("settings.display.displayLeft") }
                ]}
                onChange={onDisplayModeChange}
              />
            </Row>
          </div>
        ) : null}

        {tab === "providers" ? (
          <div className="flex flex-col gap-1">
            {providerStates.map((state) => {
              const enabled = !settings.disabledProviders.includes(state.provider.id)
              return (
                <Row key={state.provider.id} label={state.provider.displayName}>
                  <div className="flex items-center gap-2">
                    <span
                      className="inline-block h-2 w-2 rounded-full"
                      style={{ background: state.provider.brandColor }}
                    />
                    <Switch
                      checked={enabled}
                      onChange={() => onToggleProvider(state.provider.id)}
                      ariaLabel={t("settings.providers.toggle", {
                        provider: state.provider.displayName
                      })}
                    />
                  </div>
                </Row>
              )
            })}
          </div>
        ) : null}

        {tab === "system" ? (
          <div className="flex flex-col gap-1">
            <Row label={t("settings.system.refreshInterval")}>
              <Segmented
                value={settings.refreshIntervalMinutes}
                options={[
                  { value: 5, label: t("settings.system.interval5Minutes") },
                  { value: 15, label: t("settings.system.interval15Minutes") },
                  { value: 30, label: t("settings.system.interval30Minutes") },
                  { value: 60, label: t("settings.system.interval60Minutes") }
                ]}
                onChange={onRefreshIntervalChange}
              />
            </Row>
            <Row label={t("settings.system.startOnLogin")}>
              <Switch
                checked={settings.startOnLogin}
                onChange={onStartOnLoginChange}
                ariaLabel={t("settings.system.startOnLoginToggle")}
              />
            </Row>
            <Row
              label={t("settings.system.widgetSync")}
              description={t("settings.system.widgetSyncDescription")}
            >
              <Switch
                checked={settings.featureFlags.localApiEnabled}
                onChange={onWidgetSyncEnabledChange}
                ariaLabel={t("settings.system.widgetSyncToggle")}
              />
            </Row>
            {settings.featureFlags.localApiEnabled ? (
              <div className="flex flex-col gap-1 border-b border-border py-2 last:border-b-0">
                <span className="text-xs text-fg">{t("settings.system.widgetRelayUrl")}</span>
                <input
                  value={settings.widgetSyncRelayUrl}
                  onChange={(event) => onWidgetSyncRelayUrlChange(event.target.value)}
                  placeholder="https://your-relay.example.com"
                  className="min-w-0 rounded-md border border-border bg-surface-2 px-2 py-1 text-[11px] text-fg outline-none focus:border-page-accent"
                />
                <span className="text-xs text-fg">{t("settings.system.widgetSyncUrl")}</span>
                {(widgetSyncUrls.length
                  ? widgetSyncUrls
                  : [`http://PC_IP:18790/widget-snapshot?token=${settings.widgetSyncToken || "..."}`]
                ).map((url) => (
                  <code
                    key={url}
                    className="max-w-full overflow-x-auto rounded-md border border-border bg-surface-2 px-2 py-1 text-[10px] text-fg-secondary"
                  >
                    {url}
                  </code>
                ))}
              </div>
            ) : null}
          </div>
        ) : null}

        {tab === "notifications" ? (
          <div className="flex flex-col gap-1">
            <Row
              label={t("settings.notifications.enabled")}
              description={t("settings.notifications.enabledDescription")}
            >
              <Switch
                checked={settings.notificationsEnabled}
                onChange={onNotificationsEnabledChange}
                ariaLabel={t("settings.notifications.toggleAria")}
              />
            </Row>
            <Row
              label={t("settings.notifications.threshold1")}
              description={t("settings.notifications.threshold1Description")}
            >
              <Segmented
                value={thresholdValues[0]}
                size="sm"
                options={[
                  { value: 70, label: "70%" },
                  { value: 80, label: "80%" },
                  { value: 90, label: "90%" },
                  { value: 95, label: "95%" }
                ]}
                onChange={(value) => updateThreshold(0, value)}
              />
            </Row>
            <Row
              label={t("settings.notifications.threshold2")}
              description={t("settings.notifications.threshold2Description")}
            >
              <Segmented
                value={thresholdValues[1]}
                size="sm"
                options={[
                  { value: 85, label: "85%" },
                  { value: 90, label: "90%" },
                  { value: 95, label: "95%" },
                  { value: 98, label: "98%" }
                ]}
                onChange={(value) => updateThreshold(1, value)}
              />
            </Row>
            <div className="flex flex-col gap-1 border-b border-border py-2 last:border-b-0">
              <div className="flex flex-col gap-0.5">
                <span className="text-xs text-fg">{t("settings.notifications.trayTarget")}</span>
                <span className="text-[11px] text-fg-muted">
                  {t("settings.notifications.trayTargetDescription")}
                </span>
              </div>
              <div
                className="grid grid-cols-2 gap-1"
                role="radiogroup"
                aria-label={t("settings.notifications.trayTarget")}
              >
                {trayTargetOptions.map((option) => {
                  const selected = settings.trayTarget === option.value
                  return (
                    <button
                      key={option.value}
                      type="button"
                      role="radio"
                      aria-checked={selected}
                      onClick={() => onTrayTargetChange(option.value)}
                      className={cn(
                        "rounded-md border px-2 py-1 text-[11px] transition-colors",
                        selected
                          ? "border-page-accent bg-page-accent/15 text-fg"
                          : "border-border bg-surface-2 text-fg-secondary hover:bg-surface-1"
                      )}
                    >
                      {option.label}
                    </button>
                  )
                })}
              </div>
            </div>
          </div>
        ) : null}

        {tab === "about" ? (
          <div className="flex flex-col gap-2 text-xs text-fg-secondary">
            <p className="text-sm font-semibold text-fg">{t("settings.about.title")}</p>
            <p className="text-[11px] text-fg-muted">{t("settings.about.description")}</p>
            <p className="text-[11px] text-fg-muted">{t("settings.about.inspiredBy")}</p>
          </div>
        ) : null}
      </Tabs>
    </section>
  )
}
