use super::shared::{self, MetricFormat, MetricLinePayload, UsagePayload};
use crate::credentials::{Credential, CredentialError, CredentialRegistry};
use serde::Deserialize;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;

const USAGE_URL: &str = "https://api.github.com/copilot_internal/user";

static COPILOT_FETCH: OnceLock<Mutex<super::fetch_state::ProviderFetchState>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct CopilotUsageResponse {
    copilot_plan: Option<String>,
    quota_snapshots: Option<CopilotQuotaSnapshots>,
    quota_reset_date: Option<String>,
    limited_user_quotas: Option<CopilotQuotaCounts>,
    monthly_quotas: Option<CopilotQuotaCounts>,
    limited_user_reset_date: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CopilotQuotaSnapshots {
    premium_interactions: Option<CopilotQuotaSnapshot>,
    chat: Option<CopilotQuotaSnapshot>,
}

#[derive(Debug, Deserialize)]
struct CopilotQuotaSnapshot {
    percent_remaining: Option<f64>,
}

#[derive(Debug, Deserialize)]
struct CopilotQuotaCounts {
    chat: Option<f64>,
    completions: Option<f64>,
}

fn fetch_state() -> &'static Mutex<super::fetch_state::ProviderFetchState> {
    COPILOT_FETCH.get_or_init(|| Mutex::new(Default::default()))
}

#[tauri::command]
pub async fn get_copilot_usage(
    app_handle: tauri::AppHandle,
    refresh_interval_minutes: u32,
    force: bool,
) -> Result<UsagePayload, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get("copilot")
        .ok_or_else(|| "Copilot credential source missing".to_string())?;
    let credential = source.load().await.map_err(map_copilot_credential_error)?;
    let token = match credential {
        Credential::OAuth { access_token, .. } => access_token,
        Credential::ApiKey(_) => return Err("Copilot credential kind mismatch".to_string()),
    };
    let credential_fingerprint = super::fetch_state::credential_fingerprint(&token);

    let now_ms = super::fetch_state::current_time_ms();
    if let Some(payload) = super::fetch_state::read_cached_or_stale_payload(
        fetch_state(),
        now_ms,
        refresh_interval_minutes,
        Some(&credential_fingerprint),
        force,
    )? {
        return Ok(payload);
    }

    let response = match fetch_copilot_usage(&reqwest::Client::new(), &token).await {
        Ok(response) => response,
        Err(error) => return handle_backoff_failure(fetch_state(), now_ms, error),
    };

    if is_auth_status(response.status()) {
        return Err("Copilot auth expired. Run `gh auth login` and Retry.".to_string());
    }

    if response.status().is_server_error() {
        return handle_backoff_failure(
            fetch_state(),
            now_ms,
            format!(
                "Usage request failed (HTTP {}). Try again later.",
                response.status().as_u16()
            ),
        );
    }

    if !response.status().is_success() {
        return Err(format!(
            "Usage request failed (HTTP {}). Try again later.",
            response.status().as_u16()
        ));
    }

    let usage: CopilotUsageResponse = response.json().await.map_err(|error| error.to_string())?;
    let payload = UsagePayload {
        provider_id: "copilot",
        plan: usage.copilot_plan.as_deref().map(shared::title_case),
        lines: build_copilot_lines(&usage),
        source: "remote",
    };

    super::fetch_state::record_success(
        fetch_state(),
        &payload,
        now_ms,
        refresh_interval_minutes,
        Some(&credential_fingerprint),
    )?;

    Ok(payload)
}

async fn fetch_copilot_usage(
    client: &reqwest::Client,
    token: &str,
) -> Result<reqwest::Response, String> {
    client
        .get(USAGE_URL)
        .header("Authorization", format!("token {token}"))
        .header("Accept", "application/json")
        .header("Editor-Version", "vscode/1.96.2")
        .header("Editor-Plugin-Version", "copilot-chat/0.26.7")
        .header("User-Agent", "GitHubCopilotChat/0.26.7")
        .header("X-Github-Api-Version", "2025-04-01")
        .send()
        .await
        .map_err(|error| error.to_string())
}

fn build_copilot_lines(usage: &CopilotUsageResponse) -> Vec<MetricLinePayload> {
    let mut lines = Vec::new();

    if let Some(snapshots) = usage.quota_snapshots.as_ref() {
        push_percent_remaining_line(
            &mut lines,
            "Premium",
            snapshots.premium_interactions.as_ref(),
            usage.quota_reset_date.clone(),
        );
        push_percent_remaining_line(
            &mut lines,
            "Chat",
            snapshots.chat.as_ref(),
            usage.quota_reset_date.clone(),
        );
    }

    if let (Some(remaining), Some(total)) = (
        usage.limited_user_quotas.as_ref(),
        usage.monthly_quotas.as_ref(),
    ) {
        push_limited_progress_line(
            &mut lines,
            "Chat",
            remaining.chat,
            total.chat,
            usage.limited_user_reset_date.clone(),
        );
        push_limited_progress_line(
            &mut lines,
            "Completions",
            remaining.completions,
            total.completions,
            usage.limited_user_reset_date.clone(),
        );
    }

    lines
}

fn push_percent_remaining_line(
    lines: &mut Vec<MetricLinePayload>,
    label: &str,
    snapshot: Option<&CopilotQuotaSnapshot>,
    resets_at: Option<String>,
) {
    let Some(snapshot) = snapshot else {
        return;
    };
    let Some(percent_remaining) = snapshot.percent_remaining else {
        return;
    };

    lines.push(MetricLinePayload::Progress {
        label: label.to_string(),
        used: (100.0 - percent_remaining).clamp(0.0, 100.0),
        limit: 100.0,
        format: MetricFormat::Percent,
        resets_at,
        color: None,
    });
}

fn push_limited_progress_line(
    lines: &mut Vec<MetricLinePayload>,
    label: &str,
    remaining: Option<f64>,
    total: Option<f64>,
    resets_at: Option<String>,
) {
    let (Some(remaining), Some(total)) = (remaining, total) else {
        return;
    };
    if total <= 0.0 {
        return;
    }

    lines.push(MetricLinePayload::Progress {
        label: label.to_string(),
        used: (total - remaining).clamp(0.0, total),
        limit: total,
        format: MetricFormat::Percent,
        resets_at,
        color: None,
    });
}

fn handle_backoff_failure(
    state: &'static Mutex<super::fetch_state::ProviderFetchState>,
    now_ms: i64,
    message: String,
) -> Result<UsagePayload, String> {
    if let Some(payload) = super::fetch_state::record_backoff_failure(state, now_ms)? {
        return Ok(payload);
    }

    Err(message)
}

fn is_auth_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
}

fn map_copilot_credential_error(error: CredentialError) -> String {
    match error {
        CredentialError::NotConfigured => {
            "Copilot credentials not configured. Run `gh auth login` first.".to_string()
        }
        CredentialError::RefreshFailed(_) => {
            "Copilot auth refresh failed. Run `gh auth login` and Retry.".to_string()
        }
        CredentialError::Io(reason) => reason,
    }
}
