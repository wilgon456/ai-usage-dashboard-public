use std::process::Command;

fn connect_cli(provider: &str) -> Result<&'static str, String> {
    match provider {
        "claude" => Ok("claude auth login"),
        "codex" => Ok("codex login"),
        "copilot" => Ok("gh auth login --web"),
        _ => Err(format!("No connect command for provider '{provider}'")),
    }
}

#[cfg(target_os = "macos")]
fn launch_in_terminal(cli: &str) -> Result<(), String> {
    let script = format!(
        "tell application \"Terminal\" to activate\n\
         tell application \"Terminal\" to do script \"{}\"",
        cli.replace('\\', "\\\\").replace('"', "\\\"")
    );

    let status = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .status()
        .map_err(|error| error.to_string())?;

    if !status.success() {
        return Err(format!(
            "Terminal launch failed (exit {})",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

#[cfg(target_os = "windows")]
fn launch_in_terminal(cli: &str) -> Result<(), String> {
    let status = Command::new("cmd")
        .args(["/c", "start", "cmd", "/k", cli])
        .status()
        .map_err(|error| error.to_string())?;

    if !status.success() {
        return Err(format!(
            "cmd launch failed (exit {})",
            status.code().unwrap_or(-1)
        ));
    }

    Ok(())
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
fn launch_in_terminal(_cli: &str) -> Result<(), String> {
    Err("Unsupported OS for shell-based connect".to_string())
}

#[tauri::command]
pub async fn run_connect_command(provider: String) -> Result<(), String> {
    let cli = connect_cli(provider.as_str())?;
    launch_in_terminal(cli)
}
