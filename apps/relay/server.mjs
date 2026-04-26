import { createServer } from "node:http"
import { mkdir, readFile, writeFile } from "node:fs/promises"
import { dirname, resolve } from "node:path"

const port = Number(process.env.PORT ?? 8787)
const dataFile = resolve(process.env.RELAY_DATA_FILE ?? "apps/relay/.data/snapshots.json")
const maxBodyBytes = 128 * 1024

let snapshots = await loadSnapshots()

const server = createServer(async (req, res) => {
  try {
    setCors(res)

    if (req.method === "OPTIONS") {
      res.writeHead(204)
      res.end()
      return
    }

    const url = new URL(req.url ?? "/", `http://${req.headers.host ?? "localhost"}`)
    const match = url.pathname.match(/^\/v1\/snapshots\/([a-zA-Z0-9_-]{6,80})$/)
    if (!match) {
      sendJson(res, 404, { error: "not_found" })
      return
    }

    const pairId = match[1]
    if (req.method === "PUT") {
      await handlePut(req, res, pairId)
      return
    }

    if (req.method === "GET") {
      handleGet(url, res, pairId)
      return
    }

    sendJson(res, 405, { error: "method_not_allowed" })
  } catch (error) {
    sendJson(res, 500, {
      error: "internal_error",
      message: error instanceof Error ? error.message : String(error)
    })
  }
})

server.listen(port, () => {
  console.log(`AI Usage relay listening on :${port}`)
})

async function handlePut(req, res, pairId) {
  const token = bearerToken(req)
  if (!token) {
    sendJson(res, 401, { error: "missing_token" })
    return
  }

  const snapshot = JSON.parse(await readBody(req))
  if (!Array.isArray(snapshot.providers)) {
    sendJson(res, 400, { error: "invalid_snapshot" })
    return
  }

  snapshots[pairId] = {
    token,
    updatedAt: new Date().toISOString(),
    snapshot: sanitizeSnapshot(snapshot)
  }
  await saveSnapshots()
  sendJson(res, 200, { ok: true, updatedAt: snapshots[pairId].updatedAt })
}

function handleGet(url, res, pairId) {
  const token = url.searchParams.get("token") ?? ""
  const entry = snapshots[pairId]
  if (!entry || entry.token !== token) {
    sendJson(res, 404, { error: "snapshot_not_found" })
    return
  }

  sendJson(res, 200, entry.snapshot)
}

function sanitizeSnapshot(snapshot) {
  return {
    schemaVersion: 1,
    fetchedAt: String(snapshot.fetchedAt ?? new Date().toISOString()),
    providers: snapshot.providers.slice(0, 12).map((provider) => ({
      id: String(provider.id ?? ""),
      name: String(provider.name ?? "Provider"),
      percentUsed: clampPercent(provider.percentUsed),
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

function bearerToken(req) {
  const header = req.headers.authorization ?? ""
  return header.startsWith("Bearer ") ? header.slice("Bearer ".length).trim() : ""
}

async function readBody(req) {
  let body = ""
  for await (const chunk of req) {
    body += chunk
    if (body.length > maxBodyBytes) {
      throw new Error("Request body too large")
    }
  }
  return body
}

function sendJson(res, status, payload) {
  const body = JSON.stringify(payload)
  res.writeHead(status, {
    "Content-Type": "application/json",
    "Content-Length": Buffer.byteLength(body)
  })
  res.end(body)
}

function setCors(res) {
  res.setHeader("Access-Control-Allow-Origin", "*")
  res.setHeader("Access-Control-Allow-Methods", "GET, PUT, OPTIONS")
  res.setHeader("Access-Control-Allow-Headers", "Authorization, Content-Type")
}

async function loadSnapshots() {
  try {
    return JSON.parse(await readFile(dataFile, "utf8"))
  } catch {
    return {}
  }
}

async function saveSnapshots() {
  await mkdir(dirname(dataFile), { recursive: true })
  await writeFile(dataFile, JSON.stringify(snapshots, null, 2))
}
