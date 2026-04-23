use super::shared::{self, MetricFormat, MetricLinePayload, UsagePayload};
use crate::credentials::{Credential, CredentialError, CredentialRegistry};
use serde::Deserialize;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;

const USAGE_URL: &str = "https://openrouter.ai/api/v1/key";
const CREDITS_URL: &str = "https://openrouter.ai/api/v1/credits";

static OPENROUTER_FETCH: OnceLock<Mutex<super::fetch_state::ProviderFetchState>> = OnceLock::new();

#[derive(Debug, Deserialize)]
struct OpenRouterResponse {
    data: OpenRouterKeyData,
}

#[derive(Debug, Deserialize)]
struct OpenRouterCreditsResponse {
    data: OpenRouterCreditsData,
}

#[derive(Debug, Deserialize)]
struct OpenRouterKeyData {
    limit: Option<f64>,
    usage: f64,
    limit_remaining: Option<f64>,
    is_free_tier: bool,
    #[allow(dead_code)]
    is_provisioning_key: bool,
}

#[derive(Debug, Deserialize)]
struct OpenRouterCreditsData {
    total_credits: f64,
    total_usage: f64,
}

fn fetch_state() -> &'static Mutex<super::fetch_state::ProviderFetchState> {
    OPENROUTER_FETCH.get_or_init(|| Mutex::new(Default::default()))
}

#[tauri::command]
pub async fn get_openrouter_usage(
    app_handle: tauri::AppHandle,
    refresh_interval_minutes: u32,
    force: bool,
) -> Result<UsagePayload, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get("openrouter")
        .ok_or_else(|| "OpenRouter credential source missing".to_string())?;
    let credential = source
        .load()
        .await
        .map_err(map_openrouter_credential_error)?;
    let api_key = match credential {
        Credential::ApiKey(key) => key,
        Credential::OAuth { .. } => return Err("OpenRouter credential kind mismatch".to_string()),
    };

    let now_ms = super::fetch_state::current_time_ms();
    if let Some(payload) = super::fetch_state::read_cached_or_stalled_payload(
        fetch_state(),
        now_ms,
        refresh_interval_minutes,
        force,
    )? {
        return Ok(payload);
    }

    let response = match reqwest::Client::new()
        .get(USAGE_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return handle_backoff_failure(fetch_state(), now_ms, error.to_string()),
    };

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("OpenRouter API key invalid.".to_string());
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

    let key_payload: OpenRouterResponse = response.json().await.map_err(|error| error.to_string())?;
    let credits_payload = fetch_openrouter_credits(&api_key).await.ok();
    let payload = build_usage_payload(key_payload.data, credits_payload.as_ref().map(|value| &value.data));

    super::fetch_state::record_success(fetch_state(), &payload, now_ms, refresh_interval_minutes)?;

    Ok(payload)
}

async fn fetch_openrouter_credits(api_key: &str) -> Result<OpenRouterCreditsResponse, String> {
    let response = reqwest::Client::new()
        .get(CREDITS_URL)
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| error.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status().as_u16()));
    }

    response.json().await.map_err(|error| error.to_string())
}

fn build_usage_payload(
    data: OpenRouterKeyData,
    credits: Option<&OpenRouterCreditsData>,
) -> UsagePayload {
    let mut lines = Vec::new();
    let computed_limit = credits
        .map(|credits| credits.total_credits)
        .filter(|limit| *limit > 0.0)
        .or_else(|| {
            data.limit
                .filter(|limit| *limit > 0.0)
                .or_else(|| data.limit_remaining.map(|remaining| remaining + data.usage))
                .filter(|limit| *limit > 0.0)
        });
    let computed_usage = credits.map(|credits| credits.total_usage).unwrap_or(data.usage);
    let computed_remaining = computed_limit.map(|limit| (limit - computed_usage).max(0.0));

    if let Some(limit) = computed_limit {
        let used_percent = ((computed_usage / limit) * 100.0).clamp(0.0, 100.0);
        lines.push(MetricLinePayload::Progress {
            label: "Credits Used".to_string(),
            used: used_percent,
            limit: 100.0,
            format: MetricFormat::Percent,
            resets_at: None,
            color: None,
        });
    }

    lines.push(MetricLinePayload::Text {
        label: "Used".to_string(),
        value: shared::format_usd(computed_usage),
        subtitle: None,
    });

    if let Some(limit_remaining) = computed_remaining.or(data.limit_remaining) {
        lines.push(MetricLinePayload::Text {
            label: "Remaining".to_string(),
            value: shared::format_usd(limit_remaining),
            subtitle: None,
        });
    }

    if let Some(limit) = computed_limit {
        lines.push(MetricLinePayload::Text {
            label: "Total".to_string(),
            value: shared::format_usd(limit),
            subtitle: None,
        });
    }

    lines.push(MetricLinePayload::Badge {
        label: "Tier".to_string(),
        value: if data.is_free_tier {
            "Free".to_string()
        } else {
            "Paid".to_string()
        },
        tone: Some(if data.is_free_tier {
            "neutral".to_string()
        } else {
            "good".to_string()
        }),
    });

    UsagePayload {
        provider_id: "openrouter",
        plan: Some(if data.is_free_tier {
            "Free".to_string()
        } else {
            "Paid".to_string()
        }),
        lines,
        source: "remote",
    }
}

#[tauri::command]
pub async fn save_openrouter_key(app_handle: tauri::AppHandle, key: String) -> Result<(), String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get_writable("openrouter")
        .ok_or_else(|| "OpenRouter writable credential source missing".to_string())?;
    source
        .save(key)
        .await
        .map_err(map_openrouter_credential_error)
}

#[tauri::command]
pub async fn clear_openrouter_key(app_handle: tauri::AppHandle) -> Result<(), String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get_writable("openrouter")
        .ok_or_else(|| "OpenRouter writable credential source missing".to_string())?;
    source
        .clear()
        .await
        .map_err(map_openrouter_credential_error)
}

#[tauri::command]
pub async fn has_openrouter_key(app_handle: tauri::AppHandle) -> Result<bool, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get("openrouter")
        .ok_or_else(|| "OpenRouter credential source missing".to_string())?;

    match source.load().await {
        Ok(_) => Ok(true),
        Err(CredentialError::NotConfigured) => Ok(false),
        Err(error) => Err(map_openrouter_credential_error(error)),
    }
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

fn map_openrouter_credential_error(error: CredentialError) -> String {
    match error {
        CredentialError::NotConfigured => {
            "OpenRouter credentials not configured. Add an API key in Settings → Providers → OpenRouter.".to_string()
        }
        CredentialError::RefreshFailed(reason) | CredentialError::Io(reason) => reason,
    }
}
