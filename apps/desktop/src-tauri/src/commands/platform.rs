#[tauri::command]
pub fn detect_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    {
        "macos"
    }

    #[cfg(target_os = "windows")]
    {
        "windows"
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        "unknown"
    }
}
