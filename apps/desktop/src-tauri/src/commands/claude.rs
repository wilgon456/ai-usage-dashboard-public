use super::shared::{self, MetricFormat, MetricLinePayload, UsagePayload};
use crate::credentials::claude::{oauth_config, subscription_token_available};
use crate::credentials::{Credential, CredentialError, CredentialRegistry};
use dirs::home_dir;
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;

const DEFAULT_RATE_LIMIT_BACKOFF_MS: i64 = 5 * 60 * 1000;

static CLAUDE_FETCH: OnceLock<Mutex<super::fetch_state::ProviderFetchState>> = OnceLock::new();

#[derive(Debug, Clone)]
struct ClaudeUsagePayload {
    windows: BTreeMap<String, ClaudeWindow>,
    extra: Option<ClaudeExtraUsage>,
}

#[derive(Debug, Clone)]
struct ClaudeWindow {
    utilization: f64,
    resets_at: Option<ClaudeResetAt>,
}

#[derive(Debug, Clone)]
struct ClaudeLocalUsageSummary {
    today: u64,
    yesterday: u64,
    last30_days: u64,
}

#[derive(Debug, Clone, Deserialize)]
struct ClaudeExtraUsage {
    #[serde(alias = "isEnabled")]
    is_enabled: bool,
    #[serde(alias = "usedCredits")]
    used_credits: Option<f64>,
    #[serde(alias = "monthlyLimit")]
    monthly_limit: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum ClaudeResetAt {
    Unix(i64),
    Iso(String),
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeStatsCache {
    #[serde(default)]
    daily_model_tokens: Vec<ClaudeDailyModelTokens>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClaudeDailyModelTokens {
    date: String,
    #[serde(default)]
    tokens_by_model: BTreeMap<String, u64>,
}

fn debug_log_enabled() -> bool {
    std::env::var("AI_USAGE_DEBUG_CLAUDE")
        .map(|value| !value.is_empty() && value != "0" && !value.eq_ignore_ascii_case("false"))
        .unwrap_or(false)
}

fn env_oauth_token_active() -> bool {
    subscription_token_available()
}

fn preview_text(value: &str, max_chars: usize) -> &str {
    value
        .char_indices()
        .nth(max_chars)
        .map(|(index, _)| &value[..index])
        .unwrap_or(value)
}

fn fetch_state() -> &'static Mutex<super::fetch_state::ProviderFetchState> {
    CLAUDE_FETCH.get_or_init(|| Mutex::new(Default::default()))
}

#[tauri::command]
pub async fn get_claude_usage(
    app_handle: tauri::AppHandle,
    refresh_interval_minutes: u32,
    force: bool,
) -> Result<UsagePayload, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get("claude")
        .ok_or_else(|| "Claude credential source missing".to_string())?;
    let credential = match source.load().await {
        Ok(credential) => credential,
        Err(CredentialError::RefreshFailed(message)) if message.starts_with("rate_limited:") => {
            let retry_after = parse_rate_limited_seconds(&message);
            let local_usage = load_local_usage_summary().ok();
            let quota_usage = load_quota_cache_usage().ok();
            let plan = registry
                .claude()
                .load_plan_label()
                .await
                .map_err(map_claude_credential_error)?;
            return Ok(build_local_usage_payload(
                plan.or_else(|| env_oauth_token_active().then(|| "API".to_string())),
                quota_usage.as_ref(),
                local_usage.as_ref(),
                env_oauth_token_active()
                    .then(|| ("Mode".to_string(), "API".to_string(), "good".to_string())),
                Some((
                    "Status".to_string(),
                    format_refresh_rate_limited_value(retry_after),
                    "warn".to_string(),
                )),
            ));
        }
        Err(error) => return Err(map_claude_credential_error(error)),
    };
    let access_token = match credential {
        Credential::OAuth { access_token, .. } => access_token,
        Credential::ApiKey(_) => return Err("Claude credential kind mismatch".to_string()),
    };
    let plan = registry
        .claude()
        .load_plan_label()
        .await
        .map_err(map_claude_credential_error)?;
    let credential_fingerprint = super::fetch_state::credential_fingerprint(&access_token);
    let quota_usage = load_quota_cache_usage().ok();

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

    let client = reqwest::Client::new();
    let response = match fetch_claude_usage(&client, &access_token).await {
        Ok(response) => response,
        Err(error) => return handle_backoff_failure(fetch_state(), now_ms, error),
    };

    let status = response.status();
    let headers = response.headers().clone();
    let body_text = response.text().await.map_err(|error| error.to_string())?;
    let local_usage = load_local_usage_summary().ok();

    if debug_log_enabled() {
        eprintln!("[claude-debug] status={status}");
        eprintln!(
            "[claude-debug] content-type={:?}",
            headers.get("content-type")
        );
        eprintln!("[claude-debug] body-bytes={}", body_text.len());
        eprintln!(
            "[claude-debug] body-preview={}",
            preview_text(&body_text, 2048)
        );
    }

    if is_auth_status(status) {
        if env_oauth_token_active() && body_text.contains("scope requirement user:profile") {
            return Ok(build_local_usage_payload(
                Some("API".to_string()),
                quota_usage.as_ref(),
                local_usage.as_ref(),
                Some(("Mode".to_string(), "API".to_string(), "good".to_string())),
                Some((
                    "Status".to_string(),
                    "Usage derived from local API stats".to_string(),
                    "neutral".to_string(),
                )),
            ));
        }

        return Err("Claude auth refresh failed. Run `claude auth login` and Retry.".to_string());
    }

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry_after = parse_retry_after_seconds(&headers);
        super::fetch_state::record_rate_limit(
            fetch_state(),
            now_ms,
            retry_after,
            DEFAULT_RATE_LIMIT_BACKOFF_MS,
        )?;

        if let Some(cached) = super::fetch_state::cached_payload(fetch_state())? {
            return Ok(super::fetch_state::with_status_badge(
                &cached,
                format_rate_limited_value(retry_after),
                "warn",
            ));
        }

        return Ok(build_local_usage_payload(
            plan.or_else(|| env_oauth_token_active().then(|| "API".to_string())),
            quota_usage.as_ref(),
            local_usage.as_ref(),
            env_oauth_token_active()
                .then(|| ("Mode".to_string(), "API".to_string(), "good".to_string())),
            Some((
                "Status".to_string(),
                format_rate_limited_value(retry_after),
                "warn".to_string(),
            )),
        ));
    }

    if status.is_client_error() {
        return Err(format!(
            "Claude usage endpoint returned HTTP {}. The API may have moved. Set AI_USAGE_DEBUG_CLAUDE=1 and refresh to capture the response body.",
            status.as_u16()
        ));
    }

    if status.is_server_error() {
        return handle_backoff_failure(
            fetch_state(),
            now_ms,
            format!(
                "Usage request failed (HTTP {}). Try again later.",
                status.as_u16()
            ),
        );
    }

    if !status.is_success() {
        return Err(format!(
            "Usage request failed (HTTP {}). Try again later.",
            status.as_u16()
        ));
    }

    let raw: Value = serde_json::from_str(&body_text)
        .map_err(|error| format!("Claude usage: invalid JSON ({error})"))?;
    persist_quota_cache(&raw).ok();
    let usage = parse_claude_usage(raw);
    if usage.windows.is_empty() && usage.extra.is_none() {
        return Err(
            "Claude returned no usage fields. Endpoint schema may have changed — run `claude auth login` to refresh, or file an issue."
                .to_string(),
        );
    }

    let payload = UsagePayload {
        provider_id: "claude",
        plan: plan.or_else(|| env_oauth_token_active().then(|| "API".to_string())),
        lines: build_usage_lines(&usage, local_usage.as_ref()),
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

async fn fetch_claude_usage(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<reqwest::Response, String> {
    let oauth = oauth_config();
    client
        .get(&oauth.usage_url)
        .header("Authorization", format!("Bearer {}", access_token.trim()))
        .header("anthropic-beta", "oauth-2025-04-20")
        .header("Accept", "application/json")
        .header("Content-Type", "application/json")
        .header(
            "User-Agent",
            std::env::var("AI_USAGE_CLAUDE_UA")
                .unwrap_or_else(|_| "claude-code/2.1.118".to_string()),
        )
        .send()
        .await
        .map_err(|error| error.to_string())
}

fn parse_claude_usage(raw: Value) -> ClaudeUsagePayload {
    let mut windows = BTreeMap::new();
    let mut extra = None;

    let Some(obj) = raw.as_object() else {
        return ClaudeUsagePayload { windows, extra };
    };

    for (key, value) in obj {
        if key == "extra_usage" || key == "extraUsage" {
            extra = serde_json::from_value::<ClaudeExtraUsage>(value.clone()).ok();
            continue;
        }

        let Some(inner) = value.as_object() else {
            continue;
        };
        let Some(utilization) = inner.get("utilization").and_then(Value::as_f64) else {
            continue;
        };
        let resets_at = inner
            .get("resets_at")
            .or_else(|| inner.get("reset_at"))
            .or_else(|| inner.get("resetAt"))
            .and_then(|reset| serde_json::from_value::<ClaudeResetAt>(reset.clone()).ok());

        windows.insert(
            key.clone(),
            ClaudeWindow {
                utilization,
                resets_at,
            },
        );
    }

    ClaudeUsagePayload { windows, extra }
}

fn build_usage_lines(
    usage: &ClaudeUsagePayload,
    local_usage: Option<&ClaudeLocalUsageSummary>,
) -> Vec<MetricLinePayload> {
    let mut lines = Vec::new();
    append_local_usage_lines(&mut lines, local_usage);

    for (key, window) in &usage.windows {
        lines.push(MetricLinePayload::Progress {
            label: humanize_window_key(key),
            used: window.utilization,
            limit: 100.0,
            format: MetricFormat::Percent,
            resets_at: window.resets_at.as_ref().and_then(reset_at_to_iso),
            color: None,
        });
    }

    if let Some(extra_usage) = usage.extra.as_ref()
        && extra_usage.is_enabled
        && let (Some(used), Some(limit)) = (extra_usage.used_credits, extra_usage.monthly_limit)
        && limit > 0.0
    {
        lines.push(MetricLinePayload::Progress {
            label: "Extra Usage".to_string(),
            used,
            limit,
            format: MetricFormat::Currency {
                currency: "USD".to_string(),
            },
            resets_at: None,
            color: None,
        });
    }

    lines
}

fn build_local_usage_payload(
    plan: Option<String>,
    quota_usage: Option<&ClaudeUsagePayload>,
    local_usage: Option<&ClaudeLocalUsageSummary>,
    leading_badge: Option<(String, String, String)>,
    status_badge: Option<(String, String, String)>,
) -> UsagePayload {
    let mut lines = Vec::new();
    if let Some(quota_usage) = quota_usage {
        lines.extend(build_usage_lines(quota_usage, local_usage));
    } else {
        append_local_usage_lines(&mut lines, local_usage);
    }

    if let Some((label, value, tone)) = leading_badge {
        lines.push(MetricLinePayload::Badge {
            label,
            value,
            tone: Some(tone),
        });
    }

    if let Some((label, value, tone)) = status_badge {
        lines.push(MetricLinePayload::Badge {
            label,
            value,
            tone: Some(tone),
        });
    }

    if lines.is_empty() {
        lines.push(MetricLinePayload::Badge {
            label: "Status".to_string(),
            value: "No usage data".to_string(),
            tone: Some("neutral".to_string()),
        });
    }

    UsagePayload {
        provider_id: "claude",
        plan,
        lines,
        source: "cache",
    }
}

fn append_local_usage_lines(
    lines: &mut Vec<MetricLinePayload>,
    local_usage: Option<&ClaudeLocalUsageSummary>,
) {
    let Some(local_usage) = local_usage else {
        return;
    };

    lines.push(MetricLinePayload::Text {
        label: "Today".to_string(),
        value: format!("{} tokens", shared::format_token_count(local_usage.today)),
        subtitle: None,
    });
    lines.push(MetricLinePayload::Text {
        label: "Yesterday".to_string(),
        value: format!(
            "{} tokens",
            shared::format_token_count(local_usage.yesterday)
        ),
        subtitle: None,
    });
    lines.push(MetricLinePayload::Text {
        label: "Last 30 Days".to_string(),
        value: format!(
            "{} tokens",
            shared::format_token_count(local_usage.last30_days)
        ),
        subtitle: None,
    });
}

fn humanize_window_key(key: &str) -> String {
    match key {
        "five_hour" => "Session".to_string(),
        "current_session" => "Session".to_string(),
        "seven_day" => "Weekly".to_string(),
        "current_week" | "current_week_all_models" => "Weekly".to_string(),
        "seven_day_sonnet" | "current_week_sonnet" | "current_week_sonnet_only" => {
            "Sonnet".to_string()
        }
        "seven_day_omelette" => "Claude Design".to_string(),
        key if key.starts_with("seven_day_") => {
            let suffix = key.trim_start_matches("seven_day_");
            format!("{} (7-day)", shared::title_case(suffix))
        }
        key if key.starts_with("current_week_") => {
            let suffix = key.trim_start_matches("current_week_");
            format!("{} (7-day)", shared::title_case(suffix))
        }
        key if key.starts_with("five_hour_") => {
            let suffix = key.trim_start_matches("five_hour_");
            format!("{} (5-hour)", shared::title_case(suffix))
        }
        _ => shared::title_case(key),
    }
}

fn reset_at_to_iso(value: &ClaudeResetAt) -> Option<String> {
    match value {
        ClaudeResetAt::Unix(seconds) => shared::unix_seconds_to_rfc3339(*seconds),
        ClaudeResetAt::Iso(value) => Some(value.clone()),
    }
}

fn parse_retry_after_seconds(headers: &reqwest::header::HeaderMap) -> Option<i64> {
    let raw = headers
        .get("retry-after")
        .or_else(|| headers.get("Retry-After"))?
        .to_str()
        .ok()?
        .trim()
        .to_string();

    if raw.is_empty() {
        return None;
    }

    if let Ok(seconds) = raw.parse::<i64>() {
        return Some(seconds.max(0));
    }

    let timestamp =
        time::OffsetDateTime::parse(&raw, &time::format_description::well_known::Rfc2822).ok()?;
    Some((timestamp.unix_timestamp() - time::OffsetDateTime::now_utc().unix_timestamp()).max(0))
}

fn format_rate_limited_value(retry_after_seconds: Option<i64>) -> String {
    let value = retry_after_seconds.unwrap_or(DEFAULT_RATE_LIMIT_BACKOFF_MS / 1000);
    if value <= 0 {
        return "Rate limited, retry now".to_string();
    }

    let minutes = ((value + 59) / 60).max(1);
    format!("Rate limited, retry in ~{minutes}m")
}

fn parse_rate_limited_seconds(message: &str) -> Option<i64> {
    message
        .strip_prefix("rate_limited:")
        .and_then(|value| value.trim().parse::<i64>().ok())
}

fn format_refresh_rate_limited_value(retry_after_seconds: Option<i64>) -> String {
    let value = retry_after_seconds.unwrap_or(DEFAULT_RATE_LIMIT_BACKOFF_MS / 1000);
    let minutes = ((value + 59) / 60).max(1);
    format!("Refresh rate-limited, retry in {minutes}m")
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

fn map_claude_credential_error(error: CredentialError) -> String {
    match error {
        CredentialError::NotConfigured => {
            "Claude credentials not configured. Run `claude auth login` first.".to_string()
        }
        CredentialError::RefreshFailed(_) => {
            "Claude auth refresh failed. Run `claude auth login` and Retry.".to_string()
        }
        CredentialError::Io(reason) => reason,
    }
}

fn load_local_usage_summary() -> Result<ClaudeLocalUsageSummary, String> {
    let mut daily_totals = BTreeMap::new();
    let mut stats_dates = BTreeSet::new();

    if let Some(path) = claude_stats_cache_path()
        && path.exists()
    {
        let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
        let stats: ClaudeStatsCache =
            serde_json::from_str(&raw).map_err(|error| error.to_string())?;

        for day in stats.daily_model_tokens {
            let day_total = day.tokens_by_model.values().copied().sum::<u64>();
            if day_total == 0 {
                continue;
            }

            stats_dates.insert(day.date.clone());
            daily_totals.insert(day.date, day_total);
        }
    }

    append_claude_jsonl_usage(&mut daily_totals, &stats_dates);
    if daily_totals.is_empty() {
        return Err("Claude local usage missing".to_string());
    }

    let today = current_day_key(0);
    let yesterday = current_day_key(1);
    let last30_start = current_day_key(30);

    let mut summary = ClaudeLocalUsageSummary {
        today: 0,
        yesterday: 0,
        last30_days: 0,
    };

    for (day, day_total) in daily_totals {
        if day == today {
            summary.today += day_total;
        }
        if day == yesterday {
            summary.yesterday += day_total;
        }
        if day >= last30_start {
            summary.last30_days += day_total;
        }
    }

    Ok(summary)
}

fn load_quota_cache_usage() -> Result<ClaudeUsagePayload, String> {
    let path = quota_cache_path().ok_or_else(|| "No Claude home directory available".to_string())?;
    if !path.exists() {
        return Err("Claude quota cache missing".to_string());
    }

    let raw = fs::read_to_string(path).map_err(|error| error.to_string())?;
    let json: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    let mut windows = BTreeMap::new();

    if let Some(window) = quota_cache_window(&json, "/current_session") {
        windows.insert("five_hour".to_string(), window);
    }
    if let Some(window) = quota_cache_window(&json, "/weekly_limits") {
        windows.insert("seven_day".to_string(), window);
    }

    if windows.is_empty() {
        return Err("Claude quota cache has no usage windows".to_string());
    }

    Ok(ClaudeUsagePayload {
        windows,
        extra: None,
    })
}

fn quota_cache_window(json: &Value, pointer: &str) -> Option<ClaudeWindow> {
    let value = json.pointer(pointer)?;
    let utilization = value
        .get("percent_used")
        .or_else(|| value.get("utilization"))
        .and_then(Value::as_f64)?;
    if !(0.0..=100.0).contains(&utilization) {
        return None;
    }

    let resets_at = value
        .get("resets_at")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(|value| ClaudeResetAt::Iso(value.to_string()));

    Some(ClaudeWindow {
        utilization,
        resets_at,
    })
}

fn persist_quota_cache(raw: &Value) -> Result<(), String> {
    let usage = parse_claude_usage(raw.clone());
    let Some(five_hour) = usage.windows.get("five_hour") else {
        return Err("Claude usage response has no five_hour window".to_string());
    };

    let now = time::OffsetDateTime::now_utc();
    let now_iso = shared::to_rfc3339(now).unwrap_or_else(|| now.to_string());
    let seven_day = usage.windows.get("seven_day");
    let cache = serde_json::json!({
        "schema_version": 2,
        "source_url": oauth_config().usage_url,
        "attempted_at_utc": now_iso,
        "fetched_at_utc": now_iso,
        "current_session": {
            "percent_used": normalize_percent(five_hour.utilization),
            "resets_at": reset_at_to_iso_for_cache(five_hour.resets_at.as_ref()),
        },
        "weekly_limits": {
            "percent_used": seven_day.map(|window| normalize_percent(window.utilization)),
            "resets_at": seven_day.and_then(|window| reset_at_to_iso_for_cache(window.resets_at.as_ref())),
        },
        "quota_used_pct": normalize_percent(five_hour.utilization),
        "weekly_used_pct": seven_day.map(|window| normalize_percent(window.utilization)),
        "updated": now_iso,
        "valid": true,
        "stale": false,
        "error": "",
        "api_status_code": 200,
        "consecutive_failures": 0
    });

    let path = quota_cache_path().ok_or_else(|| "No Claude home directory available".to_string())?;
    let parent = path
        .parent()
        .ok_or_else(|| "Claude quota cache path has no parent".to_string())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(
        path,
        serde_json::to_string_pretty(&cache).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn quota_cache_path() -> Option<PathBuf> {
    home_dir().map(|home| home.join(".claude").join("quota-data.json"))
}

fn normalize_percent(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn reset_at_to_iso_for_cache(value: Option<&ClaudeResetAt>) -> Option<String> {
    value.and_then(reset_at_to_iso)
}

fn append_claude_jsonl_usage(
    daily_totals: &mut BTreeMap<String, u64>,
    stats_dates: &BTreeSet<String>,
) {
    let Some(projects_dir) = home_dir().map(|home| home.join(".claude").join("projects")) else {
        return;
    };

    let mut files = Vec::new();
    collect_jsonl_files(&projects_dir, &mut files);
    for path in files {
        let Ok(file) = File::open(path) else {
            continue;
        };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(json) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(date) = claude_event_date(&json) else {
                continue;
            };
            if stats_dates.contains(&date) {
                continue;
            }
            let Some(total) = json
                .pointer("/message/usage")
                .and_then(token_total_from_usage)
            else {
                continue;
            };
            if total > 0 {
                *daily_totals.entry(date).or_default() += total;
            }
        }
    }
}

fn collect_jsonl_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, files);
        } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
            files.push(path);
        }
    }
}

fn claude_event_date(value: &Value) -> Option<String> {
    value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(|timestamp| timestamp.get(..10))
        .map(str::to_string)
}

fn token_total_from_usage(value: &Value) -> Option<u64> {
    let mut total = 0_u64;
    for key in [
        "input_tokens",
        "output_tokens",
        "cache_creation_input_tokens",
        "cache_read_input_tokens",
    ] {
        total += value.get(key).and_then(Value::as_u64).unwrap_or(0);
    }
    Some(total)
}

fn claude_stats_cache_path() -> Option<PathBuf> {
    if let Ok(override_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed).join("stats-cache.json"));
        }
    }

    home_dir().map(|home| home.join(".claude").join("stats-cache.json"))
}

fn current_day_key(days_ago: i64) -> String {
    let date = (time::OffsetDateTime::now_utc() - time::Duration::days(days_ago)).date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}
