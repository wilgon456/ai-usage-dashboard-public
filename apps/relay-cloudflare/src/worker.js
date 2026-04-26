const MAX_BODY_BYTES = 128 * 1024
const MAX_PUSH_TOKEN_BYTES = 4096
const DEVICE_KEY_ALGORITHM = "SHA-256"

export default {
  async fetch(request, env) {
    const cors = corsHeaders()

    if (request.method === "OPTIONS") {
      return new Response(null, { status: 204, headers: cors })
    }

    const url = new URL(request.url)
    const snapshotMatch = url.pathname.match(/^\/v1\/snapshots\/([a-zA-Z0-9_-]{6,80})$/)
    if (snapshotMatch) {
      const pairId = snapshotMatch[1]
      if (request.method === "PUT") {
        return handlePut(request, env, pairId, cors)
      }

      if (request.method === "GET") {
        return handleGet(url, env, pairId, cors)
      }

      return json({ error: "method_not_allowed" }, 405, cors)
    }

    const pushMatch = url.pathname.match(
      /^\/v1\/push\/([a-zA-Z0-9_-]{6,80})\/(register|unregister|test)$/
    )
    if (pushMatch) {
      if (request.method !== "POST") return json({ error: "method_not_allowed" }, 405, cors)
      return handlePushRoute(request, url, env, pushMatch[1], pushMatch[2], cors)
    }

    return json({ error: "not_found" }, 404, cors)
  }
}

async function handlePut(request, env, pairId, cors) {
  const token = bearerToken(request)
  if (!token) return json({ error: "missing_token" }, 401, cors)

  const body = await request.text()
  if (body.length > MAX_BODY_BYTES) {
    return json({ error: "request_too_large" }, 413, cors)
  }

  let snapshot
  try {
    snapshot = JSON.parse(body)
  } catch {
    return json({ error: "invalid_json" }, 400, cors)
  }

  if (!Array.isArray(snapshot.providers)) {
    return json({ error: "invalid_snapshot" }, 400, cors)
  }

  const previous = await getSnapshotEntry(env, pairId)
  const sanitized = sanitizeSnapshot(snapshot)
  const etag = await sha256Hex(JSON.stringify(sanitized))
  const entry = {
    token,
    updatedAt: new Date().toISOString(),
    etag,
    snapshot: sanitized
  }

  await env.WIDGET_SNAPSHOTS.put(snapshotKey(pairId), JSON.stringify(entry))
  const push = previous?.etag === etag
    ? { attempted: 0, sent: 0, failed: 0, skipped: true }
    : await notifySnapshotUpdated(env, pairId, entry)

  return json({ ok: true, updatedAt: entry.updatedAt, etag, push }, 200, cors)
}

async function handleGet(url, env, pairId, cors) {
  const token = url.searchParams.get("token") ?? ""
  const entry = await getSnapshotEntry(env, pairId)
  if (!entry) return json({ error: "snapshot_not_found" }, 404, cors)

  if (!token || entry.token !== token) {
    return json({ error: "snapshot_not_found" }, 404, cors)
  }

  return json(entry.snapshot, 200, {
    ...cors,
    "Cache-Control": "no-store"
  })
}

async function handlePushRoute(request, url, env, pairId, action, cors) {
  const entry = await getSnapshotEntry(env, pairId)
  if (!entry) return json({ error: "snapshot_not_found" }, 404, cors)

  const token = bearerToken(request) || url.searchParams.get("token") || ""
  if (!token || token !== entry.token) {
    return json({ error: "missing_or_invalid_token" }, 401, cors)
  }

  let payload
  try {
    payload = await parseJsonBody(request)
  } catch (error) {
    return json({ error: error.message }, error.message === "request_too_large" ? 413 : 400, cors)
  }

  const device = normalizePushDevice(payload)
  if (!device) return json({ error: "invalid_push_device" }, 400, cors)

  if (action === "register") {
    const key = await pushKey(pairId, device.platform, device.provider, device.pushToken)
    const now = new Date().toISOString()
    const existing = await env.WIDGET_SNAPSHOTS.get(key)
    const existingValue = existing ? JSON.parse(existing) : {}
    await env.WIDGET_SNAPSHOTS.put(
      key,
      JSON.stringify({
        platform: device.platform,
        provider: device.provider,
        pushToken: device.pushToken,
        appVersion: device.appVersion,
        deviceIdHash: device.deviceId ? await sha256Hex(device.deviceId) : existingValue.deviceIdHash ?? "",
        createdAt: existingValue.createdAt ?? now,
        lastSeenAt: now,
        lastPushAt: existingValue.lastPushAt ?? "",
        failCount: Number(existingValue.failCount ?? 0)
      })
    )
    return json({ ok: true, registered: true }, 200, cors)
  }

  if (action === "unregister") {
    await env.WIDGET_SNAPSHOTS.delete(
      await pushKey(pairId, device.platform, device.provider, device.pushToken)
    )
    return json({ ok: true, unregistered: true }, 200, cors)
  }

  const result = await sendPushMessage(env, device, pushData(pairId, entry))
  return json({ ok: result.ok, sent: result.ok }, result.ok ? 200 : 502, cors)
}

async function parseJsonBody(request) {
  const body = await request.text()
  if (body.length > MAX_BODY_BYTES) throw new Error("request_too_large")
  try {
    return JSON.parse(body || "{}")
  } catch {
    throw new Error("invalid_json")
  }
}

function normalizePushDevice(payload) {
  const platform = String(payload.platform ?? "").toLowerCase().trim()
  const provider = String(payload.provider ?? "").toLowerCase().trim()
  const pushToken = String(payload.pushToken ?? "").trim()
  if (!["android", "ios"].includes(platform)) return null
  if (!["fcm", "apns"].includes(provider)) return null
  if (platform === "android" && provider !== "fcm") return null
  if (platform === "ios" && provider !== "apns") return null
  if (!pushToken || new TextEncoder().encode(pushToken).length > MAX_PUSH_TOKEN_BYTES) return null
  return {
    platform,
    provider,
    pushToken,
    appVersion: String(payload.appVersion ?? "").slice(0, 32),
    deviceId: String(payload.deviceId ?? "").slice(0, 256)
  }
}

async function notifySnapshotUpdated(env, pairId, entry) {
  const devices = await listPushDevices(env, pairId)
  const result = { attempted: devices.length, sent: 0, failed: 0, skipped: devices.length === 0 }
  const data = pushData(pairId, entry)

  for (const device of devices) {
    try {
      const sent = await sendPushMessage(env, device.value, data)
      if (sent.ok) {
        result.sent += 1
        await env.WIDGET_SNAPSHOTS.put(
          device.key,
          JSON.stringify({
            ...device.value,
            lastPushAt: new Date().toISOString(),
            failCount: 0
          })
        )
      } else {
        result.failed += 1
        if (sent.remove) {
          await env.WIDGET_SNAPSHOTS.delete(device.key)
        } else {
          await env.WIDGET_SNAPSHOTS.put(
            device.key,
            JSON.stringify({
              ...device.value,
              failCount: Number(device.value.failCount ?? 0) + 1
            })
          )
        }
      }
    } catch {
      result.failed += 1
    }
  }

  return result
}

function pushData(pairId, entry) {
  return {
    type: "snapshot.updated",
    pairId,
    updatedAt: entry.updatedAt,
    snapshotEtag: entry.etag,
    schemaVersion: "1"
  }
}

async function listPushDevices(env, pairId) {
  const listed = await env.WIDGET_SNAPSHOTS.list({ prefix: `push:${pairId}:` })
  const devices = []
  for (const key of listed.keys ?? []) {
    const raw = await env.WIDGET_SNAPSHOTS.get(key.name)
    if (!raw) continue
    try {
      devices.push({ key: key.name, value: JSON.parse(raw) })
    } catch {
      await env.WIDGET_SNAPSHOTS.delete(key.name)
    }
  }
  return devices
}

async function sendPushMessage(env, device, data) {
  if (device.provider === "fcm") return sendFcmMessage(env, device.pushToken, data)
  return { ok: false, remove: false, reason: "provider_not_configured" }
}

async function sendFcmMessage(env, token, data) {
  const projectId = env.FCM_PROJECT_ID
  if (!projectId) return { ok: false, remove: false, reason: "fcm_not_configured" }

  let accessToken
  try {
    accessToken = env.FCM_ACCESS_TOKEN || await getFcmAccessToken(env)
  } catch {
    return { ok: false, remove: false, reason: "fcm_auth_error" }
  }
  if (!accessToken) return { ok: false, remove: false, reason: "fcm_auth_not_configured" }

  const fetchImpl = env.fetch ?? fetch
  const url = env.FCM_SEND_URL ?? `https://fcm.googleapis.com/v1/projects/${projectId}/messages:send`
  let response
  try {
    response = await fetchImpl(url, {
      method: "POST",
      headers: {
        Authorization: `Bearer ${accessToken}`,
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        message: {
          token,
          android: { priority: "HIGH" },
          data
        }
      })
    })
  } catch {
    return { ok: false, remove: false, reason: "fcm_transport_error" }
  }

  if (response.ok) return { ok: true }
  const text = await response.text().catch(() => "")
  return {
    ok: false,
    remove: /UNREGISTERED|NOT_FOUND|INVALID_ARGUMENT/i.test(text),
    reason: `fcm_http_${response.status}`
  }
}

async function getFcmAccessToken(env) {
  if (!env.FCM_CLIENT_EMAIL || !env.FCM_PRIVATE_KEY) return ""
  const now = Math.floor(Date.now() / 1000)
  const assertion = await createServiceAccountJwt(env.FCM_CLIENT_EMAIL, env.FCM_PRIVATE_KEY, now)
  const fetchImpl = env.fetch ?? fetch
  const response = await fetchImpl("https://oauth2.googleapis.com/token", {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: new URLSearchParams({
      grant_type: "urn:ietf:params:oauth:grant-type:jwt-bearer",
      assertion
    })
  })
  if (!response.ok) return ""
  const payload = await response.json()
  return typeof payload.access_token === "string" ? payload.access_token : ""
}

async function createServiceAccountJwt(clientEmail, privateKeyPem, now) {
  const header = { alg: "RS256", typ: "JWT" }
  const claim = {
    iss: clientEmail,
    scope: "https://www.googleapis.com/auth/firebase.messaging",
    aud: "https://oauth2.googleapis.com/token",
    iat: now,
    exp: now + 3600
  }
  const unsigned = `${base64UrlJson(header)}.${base64UrlJson(claim)}`
  const key = await importPrivateKey(privateKeyPem)
  const signature = await crypto.subtle.sign("RSASSA-PKCS1-v1_5", key, new TextEncoder().encode(unsigned))
  return `${unsigned}.${base64UrlBytes(new Uint8Array(signature))}`
}

async function importPrivateKey(privateKeyPem) {
  const normalized = privateKeyPem.replace(/\\n/g, "\n")
  const body = normalized
    .replace("-----BEGIN PRIVATE KEY-----", "")
    .replace("-----END PRIVATE KEY-----", "")
    .replace(/\s+/g, "")
  const binary = Uint8Array.from(atob(body), (char) => char.charCodeAt(0))
  return crypto.subtle.importKey(
    "pkcs8",
    binary,
    { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
    false,
    ["sign"]
  )
}

function sanitizeSnapshot(snapshot) {
  return {
    schemaVersion: 1,
    fetchedAt: String(snapshot.fetchedAt ?? new Date().toISOString()),
    providers: snapshot.providers.slice(0, 12).map((provider) => ({
      id: String(provider.id ?? ""),
      name: String(provider.name ?? "Provider"),
      percentUsed: clampPercent(provider.percentUsed),
      usageLabel: String(provider.usageLabel ?? "").slice(0, 24),
      summary: String(provider.summary ?? "").slice(0, 160),
      accentColor: String(provider.accentColor ?? "#667085").slice(0, 16),
      state: String(provider.state ?? "cached").slice(0, 32)
    }))
  }
}

function clampPercent(value) {
  const number = Number(value)
  if (!Number.isFinite(number)) return 0
  return Math.max(0, Math.min(100, Math.round(number)))
}

function bearerToken(request) {
  const header = request.headers.get("Authorization") ?? ""
  return header.startsWith("Bearer ") ? header.slice("Bearer ".length).trim() : ""
}

function json(payload, status, headers) {
  return new Response(JSON.stringify(payload), {
    status,
    headers: {
      ...headers,
      "Content-Type": "application/json"
    }
  })
}

function corsHeaders() {
  return {
    "Access-Control-Allow-Origin": "*",
    "Access-Control-Allow-Methods": "GET, PUT, POST, OPTIONS",
    "Access-Control-Allow-Headers": "Authorization, Content-Type"
  }
}

async function getSnapshotEntry(env, pairId) {
  const raw = await env.WIDGET_SNAPSHOTS.get(snapshotKey(pairId))
  if (raw) return JSON.parse(raw)

  // Backward compatibility with the first relay shape, which stored snapshots directly under pairId.
  const legacy = await env.WIDGET_SNAPSHOTS.get(pairId)
  return legacy ? JSON.parse(legacy) : null
}

function snapshotKey(pairId) {
  return `snapshot:${pairId}`
}

async function pushKey(pairId, platform, provider, pushToken) {
  return `push:${pairId}:${await sha256Hex(`${platform}:${provider}:${pushToken}`)}`
}

async function sha256Hex(value) {
  const bytes = new TextEncoder().encode(value)
  const digest = await crypto.subtle.digest(DEVICE_KEY_ALGORITHM, bytes)
  return Array.from(new Uint8Array(digest), (byte) => byte.toString(16).padStart(2, "0")).join("")
}

function base64UrlJson(value) {
  return base64UrlBytes(new TextEncoder().encode(JSON.stringify(value)))
}

function base64UrlBytes(bytes) {
  let binary = ""
  for (const byte of bytes) binary += String.fromCharCode(byte)
  return btoa(binary).replace(/=/g, "").replace(/\+/g, "-").replace(/\//g, "_")
}
