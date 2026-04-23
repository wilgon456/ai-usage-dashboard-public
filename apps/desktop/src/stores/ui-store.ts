import { create } from "zustand"
import type { ProviderId } from "@ai-usage-dashboard/core"

export type ActiveView = "home" | "settings" | ProviderId

interface UiState {
  activeView: ActiveView
  lastViewedProviderId: ProviderId | null
  connectionModalFor: ProviderId | null
  refreshing: boolean
  lastRefreshedAt: string | null
  setActiveView(view: ActiveView): void
  setConnectionModalFor(id: ProviderId | null): void
  setRefreshing(value: boolean): void
  markRefreshed(at: string): void
}

function isProviderView(view: ActiveView): view is ProviderId {
  return view !== "home" && view !== "settings"
}

export const useUiStore = create<UiState>((set) => ({
  activeView: "home",
  lastViewedProviderId: null,
  connectionModalFor: null,
  refreshing: false,
  lastRefreshedAt: null,
  setActiveView: (view) =>
    set((state) => ({
      activeView: view,
      lastViewedProviderId: isProviderView(view) ? view : state.lastViewedProviderId
    })),
  setConnectionModalFor: (id) => set({ connectionModalFor: id }),
  setRefreshing: (value) => set({ refreshing: value }),
  markRefreshed: (at) => set({ lastRefreshedAt: at, refreshing: false })
}))
