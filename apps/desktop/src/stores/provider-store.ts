import { create } from "zustand"
import type {
  ProviderDefinition,
  ProviderId,
  ProviderSnapshotState
} from "@ai-usage-dashboard/core"

interface ProviderState {
  providers: ProviderDefinition[]
  snapshots: ProviderSnapshotState[]
  setProviders(defs: ProviderDefinition[]): void
  setSnapshots(states: ProviderSnapshotState[]): void
  getState(id: ProviderId): ProviderSnapshotState | undefined
}

export const useProviderStore = create<ProviderState>((set, get) => ({
  providers: [],
  snapshots: [],
  setProviders: (defs) => set({ providers: defs }),
  setSnapshots: (states) => set({ snapshots: states }),
  getState: (id) => get().snapshots.find((s) => s.provider.id === id)
}))
