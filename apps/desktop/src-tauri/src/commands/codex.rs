use super::shared::{self, MetricFormat, MetricLinePayload, UsagePayload};
use crate::credentials::{Credential, CredentialError, CredentialRegistry};
use dirs::home_dir;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

const USAGE_URL: &str = "https://chatgpt.com/backend-api/wham/usage";

static CODEX_FETCH: OnceLock<Mutex<super::fetch_state::ProviderFetchState>> = OnceLock::new();

#[derive(Debug, Clone)]
struct RateLimitWindow {
    used_percent: f64,
    resets_at: Option<String>,
}

#[derive(Debug, Clone)]
struct TokenSummary {
    today: u64,
    today_input: u64,
    today_output: u64,
    last30_days: u64,
    last_event: u64,
    last_event_at: Option<String>,
}

fn fetch_state() -> &'static Mutex<super::fetch_state::ProviderFetchState> {
    CODEX_FETCH.get_or_init(|| Mutex::new(Default::default()))
}

#[tauri::command]
pub async fn get_codex_usage(
    app_handle: tauri::AppHandle,
    refresh_interval_minutes: u32,
    force: bool,
) -> Result<UsagePayload, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get("codex")
        .ok_or_else(|| "Codex credential source missing".to_string())?;
    let credential = source.load().await.map_err(map_codex_credential_error)?;
    let access_token = match credential {
        Credential::OAuth { access_token, .. } => access_token,
        Credential::ApiKey(_) => return Err("Codex credential kind mismatch".to_string()),
    };
    let account_id = registry
        .codex()
        .load_account_id()
        .await
        .map_err(map_codex_credential_error)?;

    let now_ms = super::fetch_state::current_time_ms();
    if let Some(payload) = super::fetch_state::read_cached_or_stalled_payload(
        fetch_state(),
        now_ms,
        refresh_interval_minutes,
        force,
    )? {
        return Ok(payload);
    }

    let response = match fetch_usage_response(&access_token, account_id.as_deref()).await {
        Ok(response) => response,
        Err(error) => return handle_backoff_failure(fetch_state(), now_ms, error),
    };

    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err("Codex auth refresh failed. Run `codex login` and Retry.".to_string());
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

    let raw: Value = response.json().await.map_err(|error| error.to_string())?;
    let plan = raw
        .get("plan_type")
        .and_then(Value::as_str)
        .map(format_plan);
    let session = raw
        .get("rate_limit")
        .and_then(|value| value.get("primary_window"))
        .and_then(map_window);
    let weekly = raw
        .get("rate_limit")
        .and_then(|value| value.get("secondary_window"))
        .and_then(map_window);

    if plan.is_none() && session.is_none() && weekly.is_none() {
        return Err(
            "Codex returned no usage fields. Endpoint schema may have changed — run `codex login` to refresh, or file an issue."
                .to_string(),
        );
    }

    let token_summary = summarize_token_usage()?;
    let payload = UsagePayload {
        provider_id: "codex",
        plan,
        lines: build_usage_lines(session, weekly, &token_summary),
        source: "remote",
    };

    super::fetch_state::record_success(fetch_state(), &payload, now_ms, refresh_interval_minutes)?;

    Ok(payload)
}

async fn fetch_usage_response(
    access_token: &str,
    account_id: Option<&str>,
) -> Result<reqwest::Response, String> {
    let client = reqwest::Client::new();
    let mut request = client
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {access_token}"))
        .header("Accept", "application/json")
        .header("User-Agent", "AI-Usage-Dashboard");

    if let Some(account_id) = account_id {
        request = request.header("ChatGPT-Account-Id", account_id);
    }

    request.send().await.map_err(|error| error.to_string())
}

fn map_window(value: &Value) -> Option<RateLimitWindow> {
    let used_percent = value.get("used_percent").and_then(Value::as_f64)?;
    let resets_at = if let Some(reset_at) = value.get("reset_at").and_then(Value::as_i64) {
        OffsetDateTime::from_unix_timestamp(reset_at)
            .ok()
            .and_then(shared::to_rfc3339)
    } else if let Some(reset_after_seconds) =
        value.get("reset_after_seconds").and_then(Value::as_i64)
    {
        let future = OffsetDateTime::now_utc() + time::Duration::seconds(reset_after_seconds);
        shared::to_rfc3339(future)
    } else {
        None
    };

    Some(RateLimitWindow {
        used_percent,
        resets_at,
    })
}

fn build_usage_lines(
    session: Option<RateLimitWindow>,
    weekly: Option<RateLimitWindow>,
    tokens: &TokenSummary,
) -> Vec<MetricLinePayload> {
    let mut lines = Vec::new();

    if let Some(session) = session {
        lines.push(MetricLinePayload::Progress {
            label: "Session".to_string(),
            used: session.used_percent,
            limit: 100.0,
            format: MetricFormat::Percent,
            resets_at: session.resets_at,
            color: None,
        });
    }

    if let Some(weekly) = weekly {
        lines.push(MetricLinePayload::Progress {
            label: "Weekly".to_string(),
            used: weekly.used_percent,
            limit: 100.0,
            format: MetricFormat::Percent,
            resets_at: weekly.resets_at,
            color: None,
        });
    }

    lines.push(MetricLinePayload::Text {
        label: "Today".to_string(),
        value: format!("{} tokens", shared::format_token_count(tokens.today)),
        subtitle: None,
    });
    lines.push(MetricLinePayload::Text {
        label: "Last 30 Days".to_string(),
        value: format!("{} tokens", shared::format_token_count(tokens.last30_days)),
        subtitle: None,
    });
    lines.push(MetricLinePayload::Text {
        label: "Today I/O".to_string(),
        value: format!(
            "{} in · {} out",
            shared::format_token_count(tokens.today_input),
            shared::format_token_count(tokens.today_output)
        ),
        subtitle: None,
    });

    if tokens.last_event > 0 {
        lines.push(MetricLinePayload::Text {
            label: "Last Event".to_string(),
            value: format!("{} tokens", shared::format_token_count(tokens.last_event)),
            subtitle: tokens.last_event_at.clone(),
        });
    }

    lines
}

fn format_plan(value: &str) -> String {
    match value.trim().to_lowercase().as_str() {
        "prolite" => "Pro 5x".to_string(),
        "pro" => "Pro 10x".to_string(),
        other => other.to_string(),
    }
}

fn summarize_token_usage() -> Result<TokenSummary, String> {
    let root = home_dir()
        .ok_or_else(|| "No home directory available".to_string())?
        .join(".codex")
        .join("sessions");

    if !root.exists() {
        return Ok(TokenSummary {
            today: 0,
            today_input: 0,
            today_output: 0,
            last30_days: 0,
            last_event: 0,
            last_event_at: None,
        });
    }

    let today_start = OffsetDateTime::now_utc().replace_time(time::Time::MIDNIGHT);
    let last30_start = OffsetDateTime::now_utc() - time::Duration::days(30);

    let mut summary = TokenSummary {
        today: 0,
        today_input: 0,
        today_output: 0,
        last30_days: 0,
        last_event: 0,
        last_event_at: None,
    };
    let mut latest_timestamp: Option<OffsetDateTime> = None;

    for file in walk_jsonl_files(&root)? {
        let content = fs::read_to_string(&file).map_err(|error| error.to_string())?;
        for line in content.lines() {
            let Ok(json) = serde_json::from_str::<Value>(line) else {
                continue;
            };

            if json.get("type").and_then(Value::as_str) != Some("event_msg") {
                continue;
            }
            if json
                .get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str)
                != Some("token_count")
            {
                continue;
            }

            let timestamp = json
                .get("timestamp")
                .and_then(Value::as_str)
                .and_then(|value| OffsetDateTime::parse(value, &Rfc3339).ok());
            let Some(timestamp) = timestamp else {
                continue;
            };

            let last_usage = &json["payload"]["info"]["last_token_usage"];
            let total_tokens = last_usage
                .get("total_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            if total_tokens == 0 {
                continue;
            }

            let input_tokens = last_usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);
            let output_tokens = last_usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0);

            if timestamp >= last30_start {
                summary.last30_days += total_tokens;
            }
            if timestamp >= today_start {
                summary.today += total_tokens;
                summary.today_input += input_tokens;
                summary.today_output += output_tokens;
            }

            if latest_timestamp
                .map(|value| timestamp > value)
                .unwrap_or(true)
            {
                latest_timestamp = Some(timestamp);
                summary.last_event = total_tokens;
                summary.last_event_at = timestamp.format(&Rfc3339).ok();
            }
        }
    }

    Ok(summary)
}

fn walk_jsonl_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut stack = vec![root.to_path_buf()];
    let mut files = Vec::new();

    while let Some(path) = stack.pop() {
        let entries = fs::read_dir(&path).map_err(|error| error.to_string())?;
        for entry in entries {
            let entry = entry.map_err(|error| error.to_string())?;
            let child = entry.path();
            if child.is_dir() {
                stack.push(child);
                continue;
            }
            if child.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                files.push(child);
            }
        }
    }

    Ok(files)
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

fn map_codex_credential_error(error: CredentialError) -> String {
    match error {
        CredentialError::NotConfigured => {
            "Codex credentials not configured. Run `codex login` first.".to_string()
        }
        CredentialError::RefreshFailed(_) => {
            "Codex auth refresh failed. Run `codex login` and Retry.".to_string()
        }
        CredentialError::Io(reason) => reason,
    }
}
