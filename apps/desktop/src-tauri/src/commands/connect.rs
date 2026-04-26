use crate::credentials::CredentialRegistry;
use serde::Serialize;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use tauri::Manager;

const HOMEBREW_INSTALL: &str = r#"NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)""#;
const MACOS_BREW_SHELLENV: &str = r#"eval "$(/opt/homebrew/bin/brew shellenv 2>/dev/null || /usr/local/bin/brew shellenv 2>/dev/null)""#;
#[cfg(target_os = "windows")]
const WINDOWS_WINGET_FLAGS: &str = "--accept-package-agreements --accept-source-agreements";
#[cfg(target_os = "windows")]
const WINDOWS_NODE_INSTALL: &str =
    "winget install --id OpenJS.NodeJS.LTS -e --disable-interactivity";
#[cfg(target_os = "windows")]
const WINDOWS_GH_INSTALL: &str = "winget install --id GitHub.cli -e --disable-interactivity";
#[cfg(target_os = "windows")]
const WINDOWS_CLAUDE_INSTALL: &str = r#"powershell -NoProfile -ExecutionPolicy Bypass -Command "irm https://claude.ai/install.ps1 | iex""#;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStatus {
    can_auto_install: bool,
    command_available: bool,
    available_agents: Vec<String>,
    recommended_mode: &'static str,
    steps: Vec<BootstrapStep>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapStep {
    id: &'static str,
    status: BootstrapStepState,
    detail: Option<String>,
}

#[allow(dead_code)]
#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BootstrapStepState {
    Ready,
    ActionRequired,
    Unavailable,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DelegatingAgent {
    Codex,
    Claude,
}

impl DelegatingAgent {
    fn command(self) -> &'static str {
        match self {
            Self::Codex => "codex",
            Self::Claude => "claude",
        }
    }

    fn from_str(agent: &str) -> Option<Self> {
        match agent {
            "codex" => Some(Self::Codex),
            "claude" => Some(Self::Claude),
            _ => None,
        }
    }
}

struct PlannedBootstrap {
    can_auto_install: bool,
    command: Option<String>,
    steps: Vec<BootstrapStep>,
}

struct PlannedBootstrapBuilder {
    can_auto_install: bool,
    commands: Vec<String>,
    steps: Vec<BootstrapStep>,
}

impl PlannedBootstrapBuilder {
    fn new() -> Self {
        Self {
            can_auto_install: true,
            commands: Vec::new(),
            steps: Vec::new(),
        }
    }

    fn ready(&mut self, id: &'static str, detail: impl Into<Option<String>>) {
        self.steps.push(step_ready(id, detail));
    }

    fn action(&mut self, id: &'static str, detail: impl Into<Option<String>>) {
        self.steps.push(step_action(id, detail));
    }

    #[cfg(not(target_os = "macos"))]
    fn unavailable(&mut self, id: &'static str, detail: impl Into<Option<String>>) {
        self.steps.push(step_unavailable(id, detail));
    }

    #[cfg(not(target_os = "macos"))]
    fn disable_auto_install(&mut self) {
        self.can_auto_install = false;
    }

    fn command(&mut self, command: impl Into<String>) {
        self.commands.push(command.into());
    }

    fn build(self) -> PlannedBootstrap {
        PlannedBootstrap {
            can_auto_install: self.can_auto_install,
            command: join_commands(&self.commands),
            steps: self.steps,
        }
    }
}

fn command_exists(command: &str) -> bool {
    #[cfg(target_os = "windows")]
    let output = Command::new("where").arg(command).output();

    #[cfg(not(target_os = "windows"))]
    let output = Command::new("which").arg(command).output();

    output
        .map(|result| result.status.success())
        .unwrap_or(false)
}

fn detect_available_agents() -> Vec<DelegatingAgent> {
    let mut agents = Vec::new();

    if command_exists("codex") {
        agents.push(DelegatingAgent::Codex);
    }

    if command_exists("claude") {
        agents.push(DelegatingAgent::Claude);
    }

    agents
}

fn provider_matches_agent(provider: &str, agent: DelegatingAgent) -> bool {
    provider == agent.command()
}

fn provider_specific_hints(provider: &str) -> Result<&'static str, String> {
    match provider {
        "claude" => Ok("npm install -g @anthropic-ai/claude-code; then claude auth login"),
        "codex" => {
            Ok("brew install codex (macOS) OR npm install -g @openai/codex; then codex login")
        }
        "copilot" => Ok(
            "Install GitHub CLI (gh) via brew/winget; then gh auth login --web; ensure Copilot subscription",
        ),
        "openrouter" => Ok(
            "OpenRouter uses an API key - open https://openrouter.ai/keys, copy a key, and print instructions for the user to paste it into the dashboard",
        ),
        "kimi" => Ok("Install Kimi CLI with uv or pipx; then run kimi login"),
        _ => Err(format!("No agent bootstrap flow for provider '{provider}'")),
    }
}

fn os_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "macOS"
    } else if cfg!(target_os = "windows") {
        "Windows"
    } else {
        "current OS"
    }
}

fn agent_prompt(provider: &str) -> Result<String, String> {
    Ok(format!(
        "You are on the user's {} machine. Please install the {provider} CLI and sign in so it is ready to use. Specifically: {}. Run shell commands as needed. When complete, print 'SETUP COMPLETE' and exit.",
        os_name(),
        provider_specific_hints(provider)?
    ))
}

fn shell_double_quote_arg(input: &str) -> String {
    format!(
        "\"{}\"",
        input
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\r', " ")
            .replace('\n', " ")
    )
}

fn agent_connect_command(agent: DelegatingAgent, prompt: &str) -> String {
    let prompt_arg = shell_double_quote_arg(prompt);
    match agent {
        DelegatingAgent::Codex => format!("codex exec {prompt_arg}"),
        DelegatingAgent::Claude => format!("claude -p {prompt_arg}"),
    }
}

fn homebrew_exists() -> bool {
    command_exists("brew")
        || Path::new("/opt/homebrew/bin/brew").exists()
        || Path::new("/usr/local/bin/brew").exists()
}

fn step(
    id: &'static str,
    status: BootstrapStepState,
    detail: impl Into<Option<String>>,
) -> BootstrapStep {
    BootstrapStep {
        id,
        status,
        detail: detail.into(),
    }
}

fn step_ready(id: &'static str, detail: impl Into<Option<String>>) -> BootstrapStep {
    step(id, BootstrapStepState::Ready, detail)
}

fn step_action(id: &'static str, detail: impl Into<Option<String>>) -> BootstrapStep {
    step(id, BootstrapStepState::ActionRequired, detail)
}

#[cfg(not(target_os = "macos"))]
fn step_unavailable(id: &'static str, detail: impl Into<Option<String>>) -> BootstrapStep {
    step(id, BootstrapStepState::Unavailable, detail)
}

fn join_commands(commands: &[String]) -> Option<String> {
    if commands.is_empty() {
        None
    } else {
        Some(commands.join(" && "))
    }
}

#[cfg(target_os = "windows")]
fn windows_node_runtime_hint() -> String {
    r#"set "PATH=%PATH%;%ProgramFiles%\nodejs;%LOCALAPPDATA%\Programs\nodejs;%APPDATA%\npm""#
        .to_string()
}

#[cfg(target_os = "windows")]
fn windows_npm_command() -> String {
    r#"if exist "%ProgramFiles%\nodejs\npm.cmd" (set "AI_USAGE_NPM=%ProgramFiles%\nodejs\npm.cmd") else set "AI_USAGE_NPM=%LOCALAPPDATA%\Programs\nodejs\npm.cmd""#
        .to_string()
}

#[cfg(target_os = "windows")]
fn windows_codex_login_command() -> String {
    r#"if exist "%APPDATA%\npm\codex.cmd" ("%APPDATA%\npm\codex.cmd" login) else codex login"#
        .to_string()
}

async fn auth_configured(registry: &CredentialRegistry, provider: &str) -> bool {
    let Some(source) = registry.get(provider) else {
        return false;
    };

    source.load().await.is_ok()
}

async fn append_auth_step(
    builder: &mut PlannedBootstrapBuilder,
    registry: &CredentialRegistry,
    provider: &str,
    ready_detail: &'static str,
    action_detail: &'static str,
    login_command: &'static str,
) {
    if auth_configured(registry, provider).await {
        builder.ready("provider_auth", Some(ready_detail.to_string()));
    } else {
        builder.action("provider_auth", Some(action_detail.to_string()));
        builder.command(login_command);
    }
}

fn plan_api_key_bootstrap(provider_name: &'static str) -> PlannedBootstrap {
    PlannedBootstrap {
        can_auto_install: false,
        command: None,
        steps: vec![step_action(
            "provider_auth",
            Some(format!(
                "Paste your {provider_name} API key into the dashboard."
            )),
        )],
    }
}

#[cfg(target_os = "macos")]
async fn plan_claude_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let cli_ready = command_exists("claude");
    let mut builder = PlannedBootstrapBuilder::new();

    if cli_ready {
        builder.ready("claude_cli", Some("Claude Code CLI detected.".to_string()));
    } else {
        builder.action(
            "claude_cli",
            Some(
                "Will install Claude Code with Homebrew cask or the official installer."
                    .to_string(),
            ),
        );
        if homebrew_exists() {
            builder.command(format!(
                "{MACOS_BREW_SHELLENV} && brew install --cask claude-code"
            ));
        } else {
            builder.command(r#"curl -fsSL https://claude.ai/install.sh | bash"#);
        }
    }

    append_auth_step(
        &mut builder,
        registry,
        "claude",
        "Claude account already connected.",
        "Will open Claude login in the terminal.",
        "claude auth login",
    )
    .await;

    builder.build()
}

#[cfg(target_os = "macos")]
async fn plan_copilot_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let gh_ready = command_exists("gh");
    let mut builder = PlannedBootstrapBuilder::new();

    if gh_ready {
        builder.ready("gh_cli", Some("GitHub CLI detected.".to_string()));
    } else if homebrew_exists() {
        builder.action(
            "gh_cli",
            Some("Will install GitHub CLI with Homebrew.".to_string()),
        );
        builder.command(format!("{MACOS_BREW_SHELLENV} && brew install gh"));
    } else {
        builder.action(
            "homebrew",
            Some("Will install Homebrew first so GitHub CLI can be installed.".to_string()),
        );
        builder.action(
            "gh_cli",
            Some("Will install GitHub CLI after Homebrew is ready.".to_string()),
        );
        builder.command(HOMEBREW_INSTALL);
        builder.command(MACOS_BREW_SHELLENV);
        builder.command("brew install gh");
    }

    append_auth_step(
        &mut builder,
        registry,
        "copilot",
        "GitHub authentication is already configured.",
        "Will open GitHub CLI web login.",
        "gh auth login --web",
    )
    .await;

    builder.build()
}

#[cfg(target_os = "macos")]
async fn plan_codex_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let npm_ready = command_exists("npm");
    let codex_ready = command_exists("codex");
    let mut builder = PlannedBootstrapBuilder::new();

    if !npm_ready && !homebrew_exists() {
        builder.action(
            "homebrew",
            Some("Will install Homebrew so Node.js and Codex can be installed.".to_string()),
        );
        builder.command(HOMEBREW_INSTALL);
        builder.command(MACOS_BREW_SHELLENV);
    } else if homebrew_exists() {
        builder.command(MACOS_BREW_SHELLENV);
    }

    if npm_ready {
        builder.ready("nodejs", Some("Node.js/npm detected.".to_string()));
    } else {
        builder.action(
            "nodejs",
            Some("Will install Node.js with Homebrew.".to_string()),
        );
        builder.command("brew install node");
    }

    if codex_ready {
        builder.ready("codex_cli", Some("Codex CLI detected.".to_string()));
    } else {
        builder.action(
            "codex_cli",
            Some("Will install Codex with npm.".to_string()),
        );
        if npm_ready {
            builder.command("npm install -g @openai/codex");
        } else {
            builder.command(r#""$(brew --prefix node)/bin/npm" install -g @openai/codex"#);
        }
    }

    append_auth_step(
        &mut builder,
        registry,
        "codex",
        "OpenAI authentication is already configured.",
        "Will open Codex login in the terminal.",
        "codex login",
    )
    .await;

    builder.build()
}

#[cfg(target_os = "macos")]
async fn plan_kimi_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let kimi_ready = command_exists("kimi");
    let uv_ready = command_exists("uv");
    let pipx_ready = command_exists("pipx");
    let mut builder = PlannedBootstrapBuilder::new();

    if kimi_ready {
        builder.ready("kimi_cli", Some("Kimi CLI detected.".to_string()));
    } else {
        builder.action("kimi_cli", Some("Will install Kimi CLI.".to_string()));
        if uv_ready {
            builder.command("uv tool install kimi-cli");
        } else if pipx_ready {
            builder.command("pipx install kimi-cli");
        } else if homebrew_exists() {
            builder.command(format!("{MACOS_BREW_SHELLENV} && brew install uv"));
            builder.command("uv tool install kimi-cli");
        } else {
            builder.action(
                "homebrew",
                Some(
                    "Will install Homebrew first so uv and Kimi CLI can be installed.".to_string(),
                ),
            );
            builder.command(HOMEBREW_INSTALL);
            builder.command(MACOS_BREW_SHELLENV);
            builder.command("brew install uv");
            builder.command("uv tool install kimi-cli");
        }
    }

    append_auth_step(
        &mut builder,
        registry,
        "kimi",
        "Kimi account already connected.",
        "Will open Kimi login in the terminal.",
        "kimi login",
    )
    .await;

    builder.build()
}

#[cfg(target_os = "windows")]
async fn plan_claude_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let cli_ready = command_exists("claude");
    let mut builder = PlannedBootstrapBuilder::new();

    if cli_ready {
        builder.ready("claude_cli", Some("Claude Code CLI detected.".to_string()));
    } else {
        builder.action(
            "claude_cli",
            Some("Will run the official Claude Code PowerShell installer.".to_string()),
        );
        builder.command(WINDOWS_CLAUDE_INSTALL);
    }

    append_auth_step(
        &mut builder,
        registry,
        "claude",
        "Claude account already connected.",
        "Will open Claude login in the terminal.",
        "claude auth login",
    )
    .await;

    builder.build()
}

#[cfg(target_os = "windows")]
async fn plan_copilot_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let gh_ready = command_exists("gh");
    let winget_ready = command_exists("winget");
    let mut builder = PlannedBootstrapBuilder::new();

    if gh_ready {
        builder.ready("gh_cli", Some("GitHub CLI detected.".to_string()));
    } else if winget_ready {
        builder.action(
            "gh_cli",
            Some("Will install GitHub CLI with winget.".to_string()),
        );
        builder.command(format!("{WINDOWS_GH_INSTALL} {WINDOWS_WINGET_FLAGS}"));
    } else {
        builder.disable_auto_install();
        builder.unavailable(
            "winget",
            Some(
                "winget is not available, so GitHub CLI cannot be installed automatically."
                    .to_string(),
            ),
        );
        builder.unavailable(
            "gh_cli",
            Some("Install GitHub CLI manually, then retry.".to_string()),
        );
    }

    if auth_configured(registry, "copilot").await {
        builder.ready(
            "provider_auth",
            Some("GitHub authentication is already configured.".to_string()),
        );
    } else if builder.can_auto_install {
        builder.action(
            "provider_auth",
            Some("Will open GitHub CLI web login.".to_string()),
        );
        builder.command("gh auth login --web");
    } else {
        builder.unavailable(
            "provider_auth",
            Some("GitHub login can continue after GitHub CLI is installed.".to_string()),
        );
    }

    builder.build()
}

#[cfg(target_os = "windows")]
async fn plan_codex_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let npm_ready = command_exists("npm");
    let codex_ready = command_exists("codex");
    let winget_ready = command_exists("winget");
    let mut builder = PlannedBootstrapBuilder::new();

    if npm_ready {
        builder.ready("nodejs", Some("Node.js/npm detected.".to_string()));
    } else if winget_ready {
        builder.action(
            "nodejs",
            Some("Will install Node.js LTS with winget.".to_string()),
        );
        builder.command(format!("{WINDOWS_NODE_INSTALL} {WINDOWS_WINGET_FLAGS}"));
        builder.command(windows_node_runtime_hint());
        builder.command(windows_npm_command());
    } else {
        builder.disable_auto_install();
        builder.unavailable(
            "winget",
            Some(
                "winget is not available, so Node.js cannot be installed automatically."
                    .to_string(),
            ),
        );
        builder.unavailable(
            "nodejs",
            Some("Install Node.js LTS manually, then retry.".to_string()),
        );
    }

    if codex_ready {
        builder.ready("codex_cli", Some("Codex CLI detected.".to_string()));
    } else if builder.can_auto_install {
        builder.action(
            "codex_cli",
            Some("Will install Codex with npm.".to_string()),
        );
        if npm_ready {
            builder.command("npm install -g @openai/codex");
        } else {
            builder.command(r#"%AI_USAGE_NPM% install -g @openai/codex"#);
            builder.command(windows_node_runtime_hint());
        }
    } else {
        builder.unavailable(
            "codex_cli",
            Some("Codex can be installed after Node.js is available.".to_string()),
        );
    }

    if auth_configured(registry, "codex").await {
        builder.ready(
            "provider_auth",
            Some("OpenAI authentication is already configured.".to_string()),
        );
    } else if builder.can_auto_install {
        builder.action(
            "provider_auth",
            Some("Will open Codex login in the terminal.".to_string()),
        );
        builder.command(windows_codex_login_command());
    } else {
        builder.unavailable(
            "provider_auth",
            Some("Codex login can continue after the CLI is installed.".to_string()),
        );
    }

    builder.build()
}

#[cfg(target_os = "windows")]
async fn plan_kimi_bootstrap(registry: &CredentialRegistry) -> PlannedBootstrap {
    let kimi_ready = command_exists("kimi");
    let pipx_ready = command_exists("pipx");
    let mut builder = PlannedBootstrapBuilder::new();

    if kimi_ready {
        builder.ready("kimi_cli", Some("Kimi CLI detected.".to_string()));
    } else if pipx_ready {
        builder.action(
            "kimi_cli",
            Some("Will install Kimi CLI with pipx.".to_string()),
        );
        builder.command("pipx install kimi-cli");
    } else {
        builder.disable_auto_install();
        builder.unavailable(
            "kimi_cli",
            Some("Install Python, pipx, then run `pipx install kimi-cli` manually.".to_string()),
        );
    }

    if auth_configured(registry, "kimi").await {
        builder.ready(
            "provider_auth",
            Some("Kimi account already connected.".to_string()),
        );
    } else if builder.can_auto_install {
        builder.action(
            "provider_auth",
            Some("Will open Kimi login in the terminal.".to_string()),
        );
        builder.command("kimi login");
    } else {
        builder.unavailable(
            "provider_auth",
            Some("Kimi login can continue after Kimi CLI is installed.".to_string()),
        );
    }

    builder.build()
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
async fn plan_provider_bootstrap(
    provider: &str,
    _registry: &CredentialRegistry,
) -> Result<PlannedBootstrap, String> {
    match provider {
        "openrouter" => Ok(plan_api_key_bootstrap("OpenRouter")),
        _ => Ok(PlannedBootstrap {
            can_auto_install: false,
            command: None,
            steps: vec![step_unavailable(
                "provider_auth",
                Some("This OS is not supported by the shell bootstrap flow.".to_string()),
            )],
        }),
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
async fn plan_provider_bootstrap(
    provider: &str,
    registry: &CredentialRegistry,
) -> Result<PlannedBootstrap, String> {
    match provider {
        "claude" => Ok(plan_claude_bootstrap(registry).await),
        "codex" => Ok(plan_codex_bootstrap(registry).await),
        "copilot" => Ok(plan_copilot_bootstrap(registry).await),
        "openrouter" => Ok(plan_api_key_bootstrap("OpenRouter")),
        "kimi" => Ok(plan_kimi_bootstrap(registry).await),
        _ => Err(format!("No bootstrap flow for provider '{provider}'")),
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
pub async fn inspect_provider_bootstrap(
    app_handle: tauri::AppHandle,
    provider: String,
) -> Result<BootstrapStatus, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let planned = plan_provider_bootstrap(provider.as_str(), registry.inner()).await?;
    let available_agents = detect_available_agents();
    let recommended_mode = if available_agents
        .iter()
        .any(|agent| !provider_matches_agent(provider.as_str(), *agent))
    {
        "agent"
    } else {
        "shell"
    };

    Ok(BootstrapStatus {
        can_auto_install: planned.can_auto_install,
        command_available: planned.command.is_some(),
        available_agents: available_agents
            .iter()
            .map(|agent| agent.command().to_string())
            .collect(),
        recommended_mode,
        steps: planned.steps,
    })
}

#[tauri::command]
pub async fn run_connect_command(
    app_handle: tauri::AppHandle,
    provider: String,
) -> Result<(), String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let planned = plan_provider_bootstrap(provider.as_str(), registry.inner()).await?;
    let cli = planned.command.ok_or_else(|| {
        "No automatic bootstrap command is available for this provider.".to_string()
    })?;
    launch_in_terminal(&cli)
}

#[tauri::command]
pub async fn run_agent_connect_command(
    _app_handle: tauri::AppHandle,
    provider: String,
    agent: String,
) -> Result<(), String> {
    let agent = DelegatingAgent::from_str(agent.as_str())
        .ok_or_else(|| "Agent must be 'codex' or 'claude'.".to_string())?;

    if provider_matches_agent(provider.as_str(), agent) {
        return Err(format!(
            "Cannot ask {agent} to install or configure itself.",
            agent = agent.command()
        ));
    }

    if !command_exists(agent.command()) {
        return Err(format!(
            "{agent} is not available on PATH.",
            agent = agent.command()
        ));
    }

    let prompt = agent_prompt(provider.as_str())?;
    let cli = agent_connect_command(agent, &prompt);
    launch_in_terminal(&cli)
}
