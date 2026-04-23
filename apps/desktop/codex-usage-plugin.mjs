import { getCodexUsageState } from "./codex-usage-service.mjs"

function writeJson(response, status, payload) {
  response.statusCode = status
  response.setHeader("Content-Type", "application/json; charset=utf-8")
  response.end(JSON.stringify(payload))
}

async function handleCodexUsage(_request, response) {
  try {
    const result = await getCodexUsageState()
    if (!result.ok) {
      writeJson(response, result.status, { error: result.error })
      return
    }

    writeJson(response, 200, result.payload)
  } catch (error) {
    writeJson(response, 500, {
      error: error instanceof Error ? error.message : "Unknown Codex usage error"
    })
  }
}

function createMiddleware() {
  return async function codexUsageMiddleware(request, response, next) {
    if (!request.url?.startsWith("/api/codex/usage")) {
      next()
      return
    }

    await handleCodexUsage(request, response)
  }
}

export function codexUsagePlugin() {
  return {
    name: "codex-usage-plugin",
    configureServer(server) {
      server.middlewares.use(createMiddleware())
    },
    configurePreviewServer(server) {
      server.middlewares.use(createMiddleware())
    }
  }
}
