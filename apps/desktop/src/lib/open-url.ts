export async function openExternal(url: string): Promise<void> {
  try {
    if ((globalThis as { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__) {
      const mod = await import("@tauri-apps/plugin-opener")
      await mod.openUrl(url)
      return
    }

    if (typeof window !== "undefined") {
      window.open(url, "_blank", "noopener,noreferrer")
    }
  } catch (error) {
    console.warn("Could not open browser", error)
  }
}
