import fs from "node:fs"
import path from "node:path"
import os from "node:os"

const AUTH_PATHS = [
  path.join(os.homedir(), ".codex", "auth.json"),
  path.join(os.homedir(), ".config", "codex", "auth.json")
]
const SESSIONS_DIR = path.join(os.homedir(), ".codex", "sessions")
const REFRESH_URL = "https://auth.openai.com/oauth/token"
const USAGE_URL = "https://chatgpt.com/backend-api/wham/usage"
const CLIENT_ID = "app_EMoamEEZ73f0CkXaXp7hrann"
const REFRESH_AGE_MS = 8 * 24 * 60 * 60 * 1000

function findAuthPath() {
  return AUTH_PATHS.find((candidate) => fs.existsSync(candidate)) ?? null
}

function loadAuthState() {
  const authPath = findAuthPath()
  if (!authPath) {
    return null
  }

  const raw = fs.readFileSync(authPath, "utf8")
  const auth = JSON.parse(raw)

  if (!auth?.tokens?.access_token) {
    return null
  }

  return { authPath, auth }
}

function saveAuthState(authPath, auth) {
  fs.writeFileSync(authPath, `${JSON.stringify(auth, null, 2)}\n`)
}

function needsRefresh(auth) {
  if (!auth.last_refresh) return true
  const lastRefresh = Date.parse(auth.last_refresh)
  if (!Number.isFinite(lastRefresh)) return true
  return Date.now() - lastRefresh > REFRESH_AGE_MS
}

async function refreshAccessToken(authState) {
  const refreshToken = authState.auth?.tokens?.refresh_token
  if (!refreshToken) return null

  const body = new URLSearchParams({
    grant_type: "refresh_token",
    client_id: CLIENT_ID,
    refresh_token: refreshToken
  })

  const response = await fetch(REFRESH_URL, {
    method: "POST",
    headers: {
      "Content-Type": "application/x-www-form-urlencoded"
    },
    body
  })

  if (!response.ok) {
    const text = await response.text()
    throw new Error(`Failed to refresh Codex token (${response.status}): ${text.slice(0, 240)}`)
  }

  const payload = await response.json()
  const accessToken = payload.access_token
  if (!accessToken) {
    throw new Error("Refresh response missing access_token")
  }

  authState.auth.tokens.access_token = accessToken
  if (payload.refresh_token) authState.auth.tokens.refresh_token = payload.refresh_token
  if (payload.id_token) authState.auth.tokens.id_token = payload.id_token
  authState.auth.last_refresh = new Date().toISOString()
  saveAuthState(authState.authPath, authState.auth)

  return accessToken
}

async function fetchUsageResponse(authState) {
  let accessToken = authState.auth.tokens.access_token
  if (needsRefresh(authState.auth)) {
    const refreshed = await refreshAccessToken(authState)
    if (refreshed) {
      accessToken = refreshed
    }
  }

  const headers = {
    Authorization: `Bearer ${accessToken}`,
    Accept: "application/json",
    "User-Agent": "AI-Usage-Dashboard"
  }

  const accountId = authState.auth?.tokens?.account_id
  if (accountId) {
    headers["ChatGPT-Account-Id"] = accountId
  }

  let response = await fetch(USAGE_URL, { headers })
  if (response.status === 401 || response.status === 403) {
    const refreshed = await refreshAccessToken(authState)
    if (!refreshed) {
      throw new Error("Codex session expired. Run `codex` to log in again.")
    }

    headers.Authorization = `Bearer ${refreshed}`
    response = await fetch(USAGE_URL, { headers })
  }

  if (!response.ok) {
    const text = await response.text()
    throw new Error(`Usage request failed (${response.status}): ${text.slice(0, 240)}`)
  }

  const body = await response.json()
  return { response, body }
}

function collectSessionFiles(rootDir) {
  if (!fs.existsSync(rootDir)) return []

  const files = []
  const queue = [rootDir]

  while (queue.length > 0) {
    const current = queue.pop()
    const entries = fs.readdirSync(current, { withFileTypes: true })
    for (const entry of entries) {
      const fullPath = path.join(current, entry.name)
      if (entry.isDirectory()) {
        queue.push(fullPath)
        continue
      }
      if (entry.isFile() && fullPath.endsWith(".jsonl")) {
        files.push(fullPath)
      }
    }
  }

  return files
}

function readTokenUsageEvent(line) {
  try {
    const event = JSON.parse(line)
    if (event?.type !== "event_msg") return null
    if (event?.payload?.type !== "token_count") return null

    const lastUsage = event.payload?.info?.last_token_usage
    const totalTokens = Number(lastUsage?.total_tokens)
    if (!Number.isFinite(totalTokens) || totalTokens <= 0) return null

    const timestamp = Date.parse(event.timestamp)
    if (!Number.isFinite(timestamp)) return null

    return {
      timestamp,
      totalTokens,
      inputTokens: Number(lastUsage?.input_tokens) || 0,
      outputTokens: Number(lastUsage?.output_tokens) || 0
    }
  } catch {
    return null
  }
}

function summarizeLocalTokenUsage() {
  const files = collectSessionFiles(SESSIONS_DIR)
  const now = Date.now()
  const todayStart = new Date()
  todayStart.setHours(0, 0, 0, 0)
  const todayStartMs = todayStart.getTime()
  const monthStartMs = now - 30 * 24 * 60 * 60 * 1000

  const summary = {
    todayTokens: 0,
    todayInputTokens: 0,
    todayOutputTokens: 0,
    monthTokens: 0,
    latestTokens: 0,
    latestAt: null
  }

  for (const file of files) {
    const content = fs.readFileSync(file, "utf8")
    const lines = content.split("\n")

    for (const line of lines) {
      if (!line.trim()) continue
      const usage = readTokenUsageEvent(line)
      if (!usage) continue

      if (usage.timestamp >= monthStartMs) {
        summary.monthTokens += usage.totalTokens
      }

      if (usage.timestamp >= todayStartMs) {
        summary.todayTokens += usage.totalTokens
        summary.todayInputTokens += usage.inputTokens
        summary.todayOutputTokens += usage.outputTokens
      }

      if (!summary.latestAt || usage.timestamp > summary.latestAt) {
        summary.latestAt = usage.timestamp
        summary.latestTokens = usage.totalTokens
      }
    }
  }

  return summary
}

function formatPlan(planType) {
  if (!planType || typeof planType !== "string") return null
  const normalized = planType.trim().toLowerCase()
  if (normalized === "prolite") return "Pro 5x"
  if (normalized === "pro") return "Pro 10x"
  return planType
}

function toIsoReset(window) {
  if (!window) return null
  if (typeof window.reset_at === "number") {
    return new Date(window.reset_at * 1000).toISOString()
  }
  if (typeof window.reset_after_seconds === "number") {
    return new Date(Date.now() + window.reset_after_seconds * 1000).toISOString()
  }
  return null
}

export async function getCodexUsageState() {
  const authState = loadAuthState()
  if (!authState) {
    return {
      ok: false,
      status: 401,
      error: "Codex auth not found. Run `codex` to authenticate first."
    }
  }

  const [{ body }, tokenUsage] = await Promise.all([
    fetchUsageResponse(authState),
    Promise.resolve(summarizeLocalTokenUsage())
  ])

  return {
    ok: true,
    status: 200,
    payload: {
      providerId: "codex",
      plan: formatPlan(body.plan_type),
      rateLimits: {
        session: body?.rate_limit?.primary_window
          ? {
              usedPercent: Number(body.rate_limit.primary_window.used_percent) || 0,
              resetsAt: toIsoReset(body.rate_limit.primary_window)
            }
          : null,
        weekly: body?.rate_limit?.secondary_window
          ? {
              usedPercent: Number(body.rate_limit.secondary_window.used_percent) || 0,
              resetsAt: toIsoReset(body.rate_limit.secondary_window)
            }
          : null
      },
      tokens: {
        today: tokenUsage.todayTokens,
        todayInput: tokenUsage.todayInputTokens,
        todayOutput: tokenUsage.todayOutputTokens,
        last30Days: tokenUsage.monthTokens,
        lastEvent: tokenUsage.latestTokens,
        lastEventAt: tokenUsage.latestAt ? new Date(tokenUsage.latestAt).toISOString() : null
      },
      source: "local-codex-auth"
    }
  }
}
