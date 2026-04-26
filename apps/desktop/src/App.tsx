import { useCallback, useEffect, useRef, useState } from "react"
import type { MutableRefObject } from "react"
import type {
  AppSettings,
  DisplayMode,
  Locale,
  ProbeOptions,
  ProviderDefinition,
  ProviderId,
  ProviderSnapshotState,
  RefreshIntervalMinutes,
  ThemeMode
} from "@ai-usage-dashboard/core"
import { bootDesktopShell, type DesktopShell } from "./app-shell"
import { ConnectionModal } from "./components/ConnectionModal"
import { SideNav } from "./components/SideNav"
import { PanelFooter } from "./components/PanelFooter"
import { createT } from "./i18n"
import { formatRelative } from "./lib/format"
import {
  computeProviderMax,
  findProviderMaxProgressLine,
  fireThresholdNotification
} from "./lib/notifications"
import { syncTrayIcon } from "./lib/tray-renderer"
import {
  buildRelaySnapshotUrl,
  buildWidgetSyncPayload,
  createWidgetSyncPairId,
  createWidgetSyncToken,
  uploadWidgetSnapshot
} from "./lib/widget-sync"
import { Overview } from "./pages/Overview"
import { ProviderDetail } from "./pages/ProviderDetail"
import { Settings } from "./pages/Settings"
import { usePreferencesStore } from "./stores/preferences-store"
import { useProviderStore } from "./stores/provider-store"
import { useUiStore } from "./stores/ui-store"

const APP_VERSION = "0.2.0"

function useShell() {
  const [shell, setShell] = useState<DesktopShell | null>(null)
  useEffect(() => {
    let cancelled = false
    bootDesktopShell().then((s) => {
      if (!cancelled) setShell(s)
    })
    return () => {
      cancelled = true
    }
  }, [])
  return shell
}

export default function App() {
  const shell = useShell()
  const [booted, setBooted] = useState(false)
  const [widgetSyncUrls, setWidgetSyncUrls] = useState<string[]>([])
  const activeView = useUiStore((s) => s.activeView)
  const lastViewedProviderId = useUiStore((s) => s.lastViewedProviderId)
  const connectionModalFor = useUiStore((s) => s.connectionModalFor)
  const setConnectionModalFor = useUiStore((s) => s.setConnectionModalFor)
  const setRefreshing = useUiStore((s) => s.setRefreshing)
  const markRefreshed = useUiStore((s) => s.markRefreshed)
  const preferences = usePreferencesStore((s) => s.settings)
  const hydratePreferences = usePreferencesStore((s) => s.hydrate)
  const providers = useProviderStore((s) => s.providers)
  const snapshots = useProviderStore((s) => s.snapshots)
  const setProviders = useProviderStore((s) => s.setProviders)
  const setSnapshots = useProviderStore((s) => s.setSnapshots)
  const thresholdMemory = useRef<Record<string, number[]>>({})
  const seededThresholds = useRef(false)
  const refreshInFlight = useRef(false)
  const t = createT(preferences.locale)

  useEffect(() => {
    if (!shell) return
    let cancelled = false
    void (async () => {
      try {
        const [settings, defs] = await Promise.all([
          shell.getSettings(),
          shell.listProviders()
        ])
        if (cancelled) return
        hydratePreferences(settings)
        setProviders(defs)
        await refresh(
          shell,
          setSnapshots,
          setRefreshing,
          markRefreshed,
          refreshInFlight,
          {
            refreshIntervalMinutes: settings.refreshIntervalMinutes,
            force: false
          }
        )
      } finally {
        if (!cancelled) {
          setBooted(true)
        }
      }
    })()
    return () => {
      cancelled = true
    }
  }, [hydratePreferences, markRefreshed, setProviders, setRefreshing, setSnapshots, shell])

  const refreshAuto = useCallback(async (refreshIntervalMinutes: number) => {
    if (!shell) return
    await refresh(shell, setSnapshots, setRefreshing, markRefreshed, refreshInFlight, {
      refreshIntervalMinutes,
      force: false
    })
  }, [markRefreshed, setRefreshing, setSnapshots, shell])

  const refreshNow = useCallback(async () => {
    if (!shell) return
    await refresh(shell, setSnapshots, setRefreshing, markRefreshed, refreshInFlight, {
      refreshIntervalMinutes: preferences.refreshIntervalMinutes,
      force: true
    })
  }, [
    markRefreshed,
    preferences.refreshIntervalMinutes,
    setRefreshing,
    setSnapshots,
    shell
  ])

  const persistAndRefresh = useCallback(async (mutate: () => Promise<AppSettings>) => {
    if (!shell) return
    const updated = await mutate()
    hydratePreferences(updated)
    await refresh(shell, setSnapshots, setRefreshing, markRefreshed, refreshInFlight, {
      refreshIntervalMinutes: updated.refreshIntervalMinutes,
      force: false
    })
  }, [hydratePreferences, markRefreshed, setRefreshing, setSnapshots, shell])

  const persistOnly = useCallback(async (mutate: () => Promise<AppSettings>) => {
    if (!shell) return
    const updated = await mutate()
    hydratePreferences(updated)
  }, [hydratePreferences, shell])

  useEffect(() => {
    const root = document.documentElement
    const apply = (dark: boolean) => {
      root.classList.toggle("dark", dark)
      root.classList.toggle("light", !dark)
    }

    if (preferences.themeMode === "system") {
      const media = window.matchMedia("(prefers-color-scheme: dark)")
      const sync = () => apply(media.matches)
      sync()
      media.addEventListener("change", sync)
      return () => media.removeEventListener("change", sync)
    }

    apply(preferences.themeMode === "dark")
  }, [preferences.themeMode])

  useEffect(() => {
    if (!booted || !shell) return
    const id = window.setInterval(() => {
      void refreshAuto(preferences.refreshIntervalMinutes)
    }, preferences.refreshIntervalMinutes * 60_000)
    return () => window.clearInterval(id)
  }, [booted, preferences.refreshIntervalMinutes, refreshAuto, shell])

  useEffect(() => {
    if (!booted || !shell) return

    let cancelled = false
    void (async () => {
      const autostart = await import("@tauri-apps/plugin-autostart")
      if (cancelled) return

      try {
        const enabled = await autostart.isEnabled()
        if (preferences.startOnLogin && !enabled) {
          await autostart.enable()
        }
        if (!preferences.startOnLogin && enabled) {
          await autostart.disable()
        }
      } catch (error) {
        console.warn("autostart toggle failed", error)
      }
    })()

    return () => {
      cancelled = true
    }
  }, [booted, preferences.startOnLogin, shell])

  useEffect(() => {
    if (!booted) return
    const notifyT = createT(preferences.locale)

    if (!seededThresholds.current) {
      for (const state of snapshots) {
        const max = computeProviderMax(state)
        if (max == null) continue
        thresholdMemory.current[state.provider.id] = preferences.notificationThresholds.filter(
          (threshold) => max * 100 >= threshold
        )
      }
      seededThresholds.current = true
      void syncTrayIcon(snapshots, preferences.trayTarget, lastViewedProviderId)
      return
    }

    if (preferences.notificationsEnabled) {
      for (const state of snapshots) {
        const max = computeProviderMax(state)
        if (max == null) continue

        const maxPct = max * 100
        const memory = thresholdMemory.current[state.provider.id] ?? []
        const updated = [...memory]
        const maxLine = findProviderMaxProgressLine(state)
        const resetSuffix = maxLine?.resetsAt
          ? notifyT("notifications.limitResetSuffix", {
              time: formatRelative(maxLine.resetsAt, notifyT)
            })
          : ""

        for (const threshold of preferences.notificationThresholds) {
          if (maxPct >= threshold && !memory.includes(threshold)) {
            updated.push(threshold)
            void fireThresholdNotification(
              notifyT("notifications.limitTitle", {
                provider: state.provider.displayName,
                percent: Math.round(maxPct)
              }),
              notifyT("notifications.limitBody", {
                threshold,
                resetSuffix
              })
            )
          } else if (maxPct < threshold - 5 && memory.includes(threshold)) {
            const index = updated.indexOf(threshold)
            if (index >= 0) updated.splice(index, 1)
          }
        }

        thresholdMemory.current[state.provider.id] = updated.sort((left, right) => left - right)
      }
    }

    void syncTrayIcon(snapshots, preferences.trayTarget, lastViewedProviderId)
  }, [
    booted,
    lastViewedProviderId,
    preferences.notificationThresholds,
    preferences.notificationsEnabled,
    preferences.locale,
    preferences.trayTarget,
    snapshots
  ])

  useEffect(() => {
    if (!booted || !shell) return

    let cancelled = false
    void (async () => {
      let settings = preferences
      if (
        preferences.featureFlags.localApiEnabled &&
        (!preferences.widgetSyncToken || !preferences.widgetSyncPairId)
      ) {
        settings = await shell.updatePreferences({
          widgetSyncPairId: preferences.widgetSyncPairId || createWidgetSyncPairId(),
          widgetSyncToken: createWidgetSyncToken()
        })
        if (cancelled) return
        hydratePreferences(settings)
      }

      const snapshot = buildWidgetSyncPayload(snapshots, settings)
      await shell.configureWidgetSync({
        enabled: settings.featureFlags.localApiEnabled,
        token: settings.widgetSyncToken
      })
      await shell.updateWidgetSnapshot(snapshot)
      if (settings.featureFlags.localApiEnabled && settings.widgetSyncRelayUrl) {
        try {
          await uploadWidgetSnapshot(settings, snapshot)
        } catch (error) {
          console.warn("widget relay upload failed", error)
        }
      }
      if (settings.featureFlags.localApiEnabled && settings.widgetSyncToken) {
        setWidgetSyncUrls([
          buildRelaySnapshotUrl(settings),
          ...(await shell.getWidgetSyncUrls(settings.widgetSyncToken))
        ].filter(Boolean))
      } else {
        setWidgetSyncUrls([])
      }
    })()

    return () => {
      cancelled = true
    }
  }, [booted, hydratePreferences, preferences, shell, snapshots])

  const handlers = {
    onToggleProvider: (id: ProviderId) =>
      shell ? void persistAndRefresh(() => shell.toggleProvider(id)) : undefined,
    onThemeChange: (themeMode: ThemeMode) =>
      shell ? void persistOnly(() => shell.updatePreferences({ themeMode })) : undefined,
    onLocaleChange: (locale: Locale) =>
      shell ? void persistOnly(() => shell.updatePreferences({ locale })) : undefined,
    onDisplayModeChange: (displayMode: DisplayMode) =>
      shell ? void persistOnly(() => shell.updatePreferences({ displayMode })) : undefined,
    onRefreshIntervalChange: (refreshIntervalMinutes: RefreshIntervalMinutes) =>
      shell
        ? void persistAndRefresh(() => shell.updatePreferences({ refreshIntervalMinutes }))
        : undefined,
    onNotificationsEnabledChange: (notificationsEnabled: boolean) =>
      shell
        ? void persistOnly(() => shell.updatePreferences({ notificationsEnabled }))
        : undefined,
    onNotificationThresholdsChange: (notificationThresholds: number[]) =>
      shell
        ? void persistOnly(() => shell.updatePreferences({ notificationThresholds }))
        : undefined,
    onTrayTargetChange: (trayTarget: ProviderId | "max" | "last-viewed") =>
      shell ? void persistOnly(() => shell.updatePreferences({ trayTarget })) : undefined,
    onStartOnLoginChange: (startOnLogin: boolean) =>
      shell ? void persistOnly(() => shell.updatePreferences({ startOnLogin })) : undefined,
    onWidgetSyncEnabledChange: (localApiEnabled: boolean) =>
      shell
        ? void persistOnly(() =>
            shell.updatePreferences({
              featureFlags: {
                ...preferences.featureFlags,
                localApiEnabled
              },
              widgetSyncToken:
                localApiEnabled && !preferences.widgetSyncToken
                  ? createWidgetSyncToken()
                  : preferences.widgetSyncToken,
              widgetSyncPairId:
                localApiEnabled && !preferences.widgetSyncPairId
                  ? createWidgetSyncPairId()
                  : preferences.widgetSyncPairId
            })
          )
        : undefined,
    onWidgetSyncRelayUrlChange: (widgetSyncRelayUrl: string) =>
      shell ? void persistOnly(() => shell.updatePreferences({ widgetSyncRelayUrl })) : undefined
  }

  const enabledProviders: ProviderDefinition[] = providers.filter(
    (p) => !preferences.disabledProviders.includes(p.id)
  )
  const modalProvider = connectionModalFor
    ? providers.find((provider) => provider.id === connectionModalFor) ?? null
    : null

  const selectedProvider = typeof activeView === "string" && activeView !== "home" && activeView !== "settings"
    ? snapshots.find((s) => s.provider.id === activeView)
    : null

  return (
    <div className="relative mx-auto flex h-screen max-h-screen w-full max-w-[470px] flex-col overflow-hidden bg-bg shadow-panel">
      <div className="flex flex-1 overflow-hidden">
        <SideNav providers={enabledProviders} t={t} />
        <main className="flex-1 overflow-y-auto px-3 py-3">
          {activeView === "home" ? (
            <Overview
              states={snapshots}
              settings={preferences}
              onRefresh={refreshNow}
              onToggleProvider={handlers.onToggleProvider!}
              t={t}
            />
          ) : activeView === "settings" ? (
            <Settings
              settings={preferences}
              providerStates={snapshots}
              widgetSyncUrls={widgetSyncUrls}
              {...handlers}
              onNotificationThresholdsChange={handlers.onNotificationThresholdsChange!}
              onNotificationsEnabledChange={handlers.onNotificationsEnabledChange!}
              onToggleProvider={handlers.onToggleProvider!}
              onTrayTargetChange={handlers.onTrayTargetChange!}
              onLocaleChange={handlers.onLocaleChange!}
              onWidgetSyncEnabledChange={handlers.onWidgetSyncEnabledChange!}
              onWidgetSyncRelayUrlChange={handlers.onWidgetSyncRelayUrlChange!}
              t={t}
            />
          ) : selectedProvider ? (
            <ProviderDetail
              state={selectedProvider}
              settings={preferences}
              onRefresh={refreshNow}
              onToggleProvider={handlers.onToggleProvider!}
              t={t}
            />
          ) : (
            <div className="flex h-full items-center justify-center text-xs text-fg-muted">
              {t("app.providerUnavailable")}
            </div>
          )}
        </main>
      </div>
      <PanelFooter version={APP_VERSION} onRefresh={refreshNow} t={t} />
      {modalProvider?.connectionGuide ? (
        <ConnectionModal
          providerId={modalProvider.id}
          provider={modalProvider}
          guide={modalProvider.connectionGuide}
          onClose={() => setConnectionModalFor(null)}
          onRefresh={refreshNow}
          t={t}
        />
      ) : null}
    </div>
  )
}

async function refresh(
  shell: DesktopShell,
  setSnapshots: (states: ProviderSnapshotState[]) => void,
  setRefreshing: (value: boolean) => void,
  markRefreshed: (at: string) => void,
  inFlight: MutableRefObject<boolean>,
  options: ProbeOptions
) {
  if (inFlight.current) return
  inFlight.current = true
  setRefreshing(true)
  try {
    const states = await shell.refreshAll(options)
    setSnapshots(states)
    markRefreshed(new Date().toISOString())
  } finally {
    inFlight.current = false
    setRefreshing(false)
  }
}
