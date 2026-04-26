import type { ProviderDefinition, ProviderId } from "../domain/provider"
import type { UsageSnapshot } from "../domain/snapshot"

export interface ProbeOptions {
  refreshIntervalMinutes: number
  force: boolean
}

export interface RefreshRequest {
  providerId?: ProviderId
  requestedAt: string
}

export type RefreshResult =
  | { ok: true; snapshot: UsageSnapshot }
  | {
      ok: false
      providerId: ProviderId
      reason: string
      retryable: boolean
      errorKind?: "auth" | "network" | "rate_limited" | "parse" | "unexpected"
    }

export interface ProviderRuntime {
  definition: ProviderDefinition
  refresh(options: ProbeOptions): Promise<RefreshResult>
}

export class RefreshOrchestrator {
  constructor(private readonly providers: ProviderRuntime[]) {}

  listProviders(): ProviderDefinition[] {
    return this.providers.map((provider) => provider.definition)
  }

  async refreshAll(options: ProbeOptions): Promise<RefreshResult[]> {
    return Promise.all(this.providers.map((provider) => provider.refresh(options)))
  }

  async refreshOne(providerId: ProviderId, options: ProbeOptions): Promise<RefreshResult | null> {
    const provider = this.providers.find((candidate) => candidate.definition.id === providerId)
    if (!provider) return null
    return provider.refresh(options)
  }
}
