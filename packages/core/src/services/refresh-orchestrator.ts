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
  | { ok: false; providerId: ProviderId; reason: string; retryable: boolean }

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
    const results: RefreshResult[] = []

    for (const provider of this.providers) {
      results.push(await provider.refresh(options))
    }

    return results
  }

  async refreshOne(providerId: ProviderId, options: ProbeOptions): Promise<RefreshResult | null> {
    const provider = this.providers.find((candidate) => candidate.definition.id === providerId)
    if (!provider) return null
    return provider.refresh(options)
  }
}
