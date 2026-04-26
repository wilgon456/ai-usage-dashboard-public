import assert from "node:assert/strict"
import { test } from "node:test"
import worker from "./worker.js"

class MemoryKv {
  constructor() {
    this.map = new Map()
  }

  async get(key) {
    return this.map.get(key) ?? null
  }

  async put(key, value) {
    this.map.set(key, value)
  }

  async delete(key) {
    this.map.delete(key)
  }

  async list(options = {}) {
    const prefix = options.prefix ?? ""
    return {
      keys: Array.from(this.map.keys())
        .filter((name) => name.startsWith(prefix))
        .sort()
        .map((name) => ({ name }))
    }
  }
}

function env(extra = {}) {
  return {
    WIDGET_SNAPSHOTS: new MemoryKv(),
    ...extra
  }
}

const snapshot = {
  schemaVersion: 1,
  fetchedAt: "2026-04-26T00:00:00.000Z",
  providers: [
    {
      id: "codex",
      name: "Codex",
      percentUsed: 42,
      usageLabel: "42%",
      summary: "Today: 42%",
      accentColor: "#000000",
      state: "fresh"
    }
  ]
}

async function putSnapshot(testEnv, pairId = "pair123", token = "secret-token") {
  const response = await worker.fetch(
    new Request(`https://relay.test/v1/snapshots/${pairId}`, {
      method: "PUT",
      headers: {
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json"
      },
      body: JSON.stringify(snapshot)
    }),
    testEnv
  )
  assert.equal(response.status, 200)
  return response.json()
}

test("register stores a push device only when the snapshot token matches", async () => {
  const testEnv = env()
  await putSnapshot(testEnv)

  const missingAuth = await worker.fetch(
    new Request("https://relay.test/v1/push/pair123/register", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ platform: "android", provider: "fcm", pushToken: "fcm-token" })
    }),
    testEnv
  )
  assert.equal(missingAuth.status, 401)

  const response = await worker.fetch(
    new Request("https://relay.test/v1/push/pair123/register", {
      method: "POST",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({
        platform: "android",
        provider: "fcm",
        pushToken: "fcm-token",
        appVersion: "0.1.0",
        deviceId: "phone-1"
      })
    }),
    testEnv
  )

  assert.equal(response.status, 200)
  assert.deepEqual(await response.json(), { ok: true, registered: true })

  const keys = await testEnv.WIDGET_SNAPSHOTS.list({ prefix: "push:pair123:" })
  assert.equal(keys.keys.length, 1)
  const stored = JSON.parse(await testEnv.WIDGET_SNAPSHOTS.get(keys.keys[0].name))
  assert.equal(stored.platform, "android")
  assert.equal(stored.provider, "fcm")
  assert.equal(stored.pushToken, "fcm-token")
  assert.notEqual(stored.deviceIdHash, "phone-1")
})

test("unregister removes a registered push device", async () => {
  const testEnv = env()
  await putSnapshot(testEnv)

  await worker.fetch(
    new Request("https://relay.test/v1/push/pair123/register", {
      method: "POST",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ platform: "android", provider: "fcm", pushToken: "fcm-token" })
    }),
    testEnv
  )

  const response = await worker.fetch(
    new Request("https://relay.test/v1/push/pair123/unregister", {
      method: "POST",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ platform: "android", provider: "fcm", pushToken: "fcm-token" })
    }),
    testEnv
  )

  assert.equal(response.status, 200)
  assert.deepEqual(await response.json(), { ok: true, unregistered: true })
  const keys = await testEnv.WIDGET_SNAPSHOTS.list({ prefix: "push:pair123:" })
  assert.equal(keys.keys.length, 0)
})

test("snapshot PUT keeps succeeding when best-effort push transport throws", async () => {
  const testEnv = env({
    FCM_PROJECT_ID: "project-id",
    FCM_ACCESS_TOKEN: "test-access-token",
    FCM_SEND_URL: "https://fcm.test/send",
    fetch: async () => {
      throw new Error("network down")
    }
  })

  await putSnapshot(testEnv)
  await worker.fetch(
    new Request("https://relay.test/v1/push/pair123/register", {
      method: "POST",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ platform: "android", provider: "fcm", pushToken: "fcm-token" })
    }),
    testEnv
  )

  const response = await worker.fetch(
    new Request("https://relay.test/v1/snapshots/pair123", {
      method: "PUT",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ ...snapshot, providers: [{ ...snapshot.providers[0], percentUsed: 55 }] })
    }),
    testEnv
  )

  assert.equal(response.status, 200)
  const body = await response.json()
  assert.equal(body.ok, true)
  assert.equal(body.push.failed, 1)
})

test("snapshot PUT sends only wake-signal push metadata and skips identical etags", async () => {
  const sentMessages = []
  const testEnv = env({
    FCM_PROJECT_ID: "project-id",
    FCM_ACCESS_TOKEN: "test-access-token",
    FCM_SEND_URL: "https://fcm.test/send",
    fetch: async (_url, init) => {
      sentMessages.push(JSON.parse(init.body))
      return new Response(JSON.stringify({ name: "messages/1" }), { status: 200 })
    }
  })

  await putSnapshot(testEnv)
  await worker.fetch(
    new Request("https://relay.test/v1/push/pair123/register", {
      method: "POST",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ platform: "android", provider: "fcm", pushToken: "fcm-token" })
    }),
    testEnv
  )

  const duplicate = await putSnapshot(testEnv)
  assert.equal(duplicate.push.skipped, true)
  assert.equal(sentMessages.length, 0)

  const changed = await worker.fetch(
    new Request("https://relay.test/v1/snapshots/pair123", {
      method: "PUT",
      headers: {
        Authorization: "Bearer secret-token",
        "Content-Type": "application/json"
      },
      body: JSON.stringify({ ...snapshot, providers: [{ ...snapshot.providers[0], percentUsed: 51 }] })
    }),
    testEnv
  )

  assert.equal(changed.status, 200)
  const body = await changed.json()
  assert.equal(body.push.attempted, 1)
  assert.equal(body.push.sent, 1)
  assert.equal(sentMessages.length, 1)
  assert.equal(sentMessages[0].message.token, "fcm-token")
  assert.equal(sentMessages[0].message.data.type, "snapshot.updated")
  assert.equal(sentMessages[0].message.data.pairId, "pair123")
  assert.ok(sentMessages[0].message.data.snapshotEtag)
  assert.equal(JSON.stringify(sentMessages[0]).includes("secret-token"), false)
  assert.equal(Object.hasOwn(sentMessages[0].message.data, "providers"), false)
})
