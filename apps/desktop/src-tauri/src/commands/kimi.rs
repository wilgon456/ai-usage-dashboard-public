use super::shared::{MetricFormat, MetricLinePayload, UsagePayload};
use crate::credentials::{Credential, CredentialError, CredentialRegistry};
use base64::Engine;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};
use tauri::Manager;
use time::OffsetDateTime;

const USAGES_URL: &str = "https://api.kimi.com/coding/v1/usages";
const ME_URL: &str = "https://api.kimi.com/coding/v1/me";
const DEFAULT_RATE_LIMIT_BACKOFF_MS: i64 = 60_000;

static KIMI_FETCH: OnceLock<Mutex<super::fetch_state::ProviderFetchState>> = OnceLock::new();

#[derive(Debug, Clone)]
struct KimiLocalUsageSummary {
    today: u64,
    yesterday: u64,
    last30_days: u64,
}

fn fetch_state() -> &'static Mutex<super::fetch_state::ProviderFetchState> {
    KIMI_FETCH.get_or_init(|| Mutex::new(Default::default()))
}

#[tauri::command]
pub async fn get_kimi_usage(
    app_handle: tauri::AppHandle,
    refresh_interval_minutes: u32,
    force: bool,
) -> Result<UsagePayload, String> {
    let registry = app_handle.state::<Arc<CredentialRegistry>>();
    let source = registry
        .get("kimi")
        .ok_or_else(|| "Kimi credential source missing".to_string())?;
    let credential = match source.load().await {
        Ok(credential) => credential,
        Err(error) => return Err(map_kimi_credential_error(error)),
    };
    let mut access_token = match credential {
        Credential::OAuth { access_token, .. } => access_token,
        Credential::ApiKey(_) => return Err("Kimi credential kind mismatch".to_string()),
    };
    let mut credential_fingerprint = super::fetch_state::credential_fingerprint(&access_token);

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
    let mut response = match fetch_kimi_json(&client, USAGES_URL, &access_token).await {
        Ok(response) => response,
        Err(error) => {
            return handle_backoff_failure(fetch_state(), now_ms, &access_token, error);
        }
    };

    if is_auth_status(response.status()) {
        let refreshed = source.load().await.map_err(map_kimi_credential_error)?;
        let refreshed_token = match refreshed {
            Credential::OAuth { access_token, .. } => access_token,
            Credential::ApiKey(_) => return Err("Kimi credential kind mismatch".to_string()),
        };
        response = match fetch_kimi_json(&client, USAGES_URL, &refreshed_token).await {
            Ok(response) => response,
            Err(error) => {
                return handle_backoff_failure(fetch_state(), now_ms, &refreshed_token, error);
            }
        };

        if is_auth_status(response.status()) {
            return Err("Kimi credentials rejected. Run `kimi login` and Retry.".to_string());
        }
        access_token = refreshed_token;
        credential_fingerprint = super::fetch_state::credential_fingerprint(&access_token);
    }

    let status = response.status();
    let headers = response.headers().clone();
    let body_text = response.text().await.map_err(|error| error.to_string())?;

    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return handle_rate_limit(fetch_state(), now_ms, &headers);
    }

    if status.is_server_error() {
        return handle_backoff_failure(
            fetch_state(),
            now_ms,
            &access_token,
            format!(
                "Usage request failed (HTTP {}). Try again later.",
                status.as_u16()
            ),
        );
    }

    let payload = if status.is_success() {
        let raw: Value = serde_json::from_str(&body_text)
            .map_err(|error| format!("Kimi usage: invalid JSON ({error})"))?;
        build_usage_payload(&access_token, Some(&raw), "remote", "OAuth token loaded", "good")
    } else if status == reqwest::StatusCode::NOT_FOUND {
        fetch_kimi_identity_payload(&client, &access_token).await?
    } else if status.is_client_error() {
        return Err(format!(
            "Kimi usage endpoint returned HTTP {}. Run `kimi login` and Retry.",
            status.as_u16()
        ));
    } else {
        return Err(format!(
            "Usage request failed (HTTP {}). Try again later.",
            status.as_u16()
        ));
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

async fn fetch_kimi_identity_payload(
    client: &reqwest::Client,
    access_token: &str,
) -> Result<UsagePayload, String> {
    let response = match fetch_kimi_json(client, ME_URL, access_token).await {
        Ok(response) => response,
        Err(_) => {
            return local_fallback_payload(
                Some(access_token),
                "Local logs only",
                "warn",
                None,
            );
        }
    };

    if !response.status().is_success() {
        return local_fallback_payload(Some(access_token), "Local logs only", "warn", None);
    }

    let body_text = response.text().await.map_err(|error| error.to_string())?;
    let raw: Value = serde_json::from_str(&body_text)
        .map_err(|error| format!("Kimi usage: invalid JSON ({error})"))?;
    Ok(build_usage_payload(
        access_token,
        Some(&raw),
        "remote",
        "OAuth token loaded",
        "good",
    ))
}

async fn fetch_kimi_json(
    client: &reqwest::Client,
    url: &str,
    access_token: &str,
) -> Result<reqwest::Response, String> {
    client
        .get(url)
        .header("Authorization", format!("Bearer {}", access_token.trim()))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| error.to_string())
}

fn build_usage_payload(
    access_token: &str,
    me: Option<&Value>,
    source: &'static str,
    status: &str,
    status_tone: &str,
) -> UsagePayload {
    let jwt = (!access_token.trim().is_empty()).then(|| decode_jwt_payload(access_token)).flatten();
    let plan = infer_plan(me).unwrap_or_else(|| "Kimi Code".to_string());
    let local_usage = load_local_usage_summary().ok();
    let mut lines = Vec::new();

    let has_usage_progress = if let Some(progress) = usage_progress(me) {
        lines.extend(progress);
        true
    } else {
        false
    };

    if let Some(token_validity) = token_validity_text(jwt.as_ref()) {
        lines.push(token_validity);
    }

    lines.push(MetricLinePayload::Badge {
        label: "Plan".to_string(),
        value: plan.clone(),
        tone: Some("good".to_string()),
    });
    lines.push(MetricLinePayload::Badge {
        label: "Status".to_string(),
        value: status.to_string(),
        tone: Some(status_tone.to_string()),
    });

    if !has_usage_progress {
        append_local_usage_lines(&mut lines, local_usage.as_ref());
    }

    if let Some(region) = me.and_then(find_region) {
        lines.push(MetricLinePayload::Text {
            label: "Region".to_string(),
            value: region,
            subtitle: None,
        });
    }

    if let Some(parallel) = me.and_then(find_parallel_limit) {
        lines.push(MetricLinePayload::Text {
            label: "Parallel".to_string(),
            value: parallel,
            subtitle: None,
        });
    }

    if let Some(user_id) = me
        .and_then(find_user_id)
        .or_else(|| jwt.as_ref().and_then(find_user_id))
    {
        lines.push(MetricLinePayload::Text {
            label: "User ID".to_string(),
            value: user_id,
            subtitle: None,
        });
    }

    if let Some(scope) = jwt.as_ref().and_then(find_scope) {
        lines.push(MetricLinePayload::Text {
            label: "Scope".to_string(),
            value: scope,
            subtitle: None,
        });
    }

    UsagePayload {
        provider_id: "kimi",
        plan: Some(plan),
        lines,
        source,
    }
}

fn local_fallback_payload(
    access_token: Option<&str>,
    status: &str,
    tone: &str,
    error: Option<String>,
) -> Result<UsagePayload, String> {
    if load_local_usage_summary().is_err() {
        return Err(error.unwrap_or_else(|| status.to_string()));
    }

    Ok(build_usage_payload(
        access_token.unwrap_or_default(),
        None,
        "cache",
        status,
        tone,
    ))
}

fn append_local_usage_lines(
    lines: &mut Vec<MetricLinePayload>,
    local_usage: Option<&KimiLocalUsageSummary>,
) {
    let Some(local_usage) = local_usage else {
        return;
    };

    lines.push(MetricLinePayload::Text {
        label: "Today".to_string(),
        value: format!(
            "{} tokens",
            super::shared::format_token_count(local_usage.today)
        ),
        subtitle: None,
    });
    lines.push(MetricLinePayload::Text {
        label: "Yesterday".to_string(),
        value: format!(
            "{} tokens",
            super::shared::format_token_count(local_usage.yesterday)
        ),
        subtitle: None,
    });
    lines.push(MetricLinePayload::Text {
        label: "Last 30 Days".to_string(),
        value: format!(
            "{} tokens",
            super::shared::format_token_count(local_usage.last30_days)
        ),
        subtitle: None,
    });
}

fn load_local_usage_summary() -> Result<KimiLocalUsageSummary, String> {
    let mut daily_totals: BTreeMap<String, u64> = BTreeMap::new();
    let Some(sessions_dir) = dirs::home_dir().map(|home| home.join(".kimi").join("sessions"))
    else {
        return Err("Kimi home directory missing".to_string());
    };

    let mut files = Vec::new();
    collect_wire_files(&sessions_dir, &mut files);
    for path in files {
        let Ok(file) = File::open(path) else {
            continue;
        };
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let Ok(json) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(date) = kimi_event_date(&json) else {
                continue;
            };
            let Some(total) = json
                .pointer("/message/payload/token_usage")
                .and_then(kimi_token_total)
            else {
                continue;
            };
            if total > 0 {
                *daily_totals.entry(date).or_default() += total;
            }
        }
    }

    if daily_totals.is_empty() {
        return Err("Kimi local usage missing".to_string());
    }

    let today = current_day_key(0);
    let yesterday = current_day_key(1);
    let last30_start = current_day_key(30);
    let mut summary = KimiLocalUsageSummary {
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

fn collect_wire_files(dir: &PathBuf, files: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_wire_files(&path, files);
        } else if path.file_name().and_then(|value| value.to_str()) == Some("wire.jsonl") {
            files.push(path);
        }
    }
}

fn kimi_event_date(value: &Value) -> Option<String> {
    let timestamp = value.get("timestamp")?.as_f64()? as i64;
    let date = OffsetDateTime::from_unix_timestamp(timestamp).ok()?.date();
    Some(format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    ))
}

fn kimi_token_total(value: &Value) -> Option<u64> {
    let mut total = 0_u64;
    for key in [
        "input_other",
        "input_cache_read",
        "input_cache_creation",
        "output",
    ] {
        total += value.get(key).and_then(Value::as_u64).unwrap_or(0);
    }
    Some(total)
}

fn current_day_key(days_ago: i64) -> String {
    let date = (OffsetDateTime::now_utc() - time::Duration::days(days_ago)).date();
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn token_validity_text(jwt: Option<&Value>) -> Option<MetricLinePayload> {
    let exp = jwt
        .and_then(|value| value.get("exp"))
        .and_then(Value::as_i64)?;

    Some(MetricLinePayload::Text {
        label: "Token Expires".to_string(),
        value: super::shared::unix_seconds_to_rfc3339(exp).unwrap_or_else(|| exp.to_string()),
        subtitle: None,
    })
}

fn usage_progress(value: Option<&Value>) -> Option<Vec<MetricLinePayload>> {
    let value = value?;
    let mut lines = Vec::new();

    if let Some(total_quota) = value
        .get("totalQuota")
        .or_else(|| value.get("total_quota"))
        .or_else(|| value.pointer("/data/totalQuota"))
        .or_else(|| value.pointer("/data/total_quota"))
    {
        if let Some(progress) = quota_progress(
            "Subscription Usage",
            total_quota,
            first_limit_reset_time(value).or_else(|| reset_time(total_quota)),
        ) {
            lines.push(progress);
        }
    }

    if let Some(usage) = value
        .get("usage")
        .or_else(|| value.pointer("/data/usage"))
        .or_else(|| value.pointer("/detail"))
    {
        if let Some(progress) = quota_progress("5-min Window", usage, reset_time(usage)) {
            lines.push(progress);
        }
    }

    if let Some(limits) = value
        .get("limits")
        .or_else(|| value.pointer("/data/limits"))
        .and_then(Value::as_array)
    {
        for limit_entry in limits {
            let Some(detail) = limit_entry.get("detail") else {
                continue;
            };
            let label = limit_entry
                .get("window")
                .map(humanize_limit_window)
                .unwrap_or_else(|| "Window".to_string());
            if let Some(progress) = quota_progress(&label, detail, reset_time(detail)) {
                lines.push(progress);
            }
        }
    }

    if lines.is_empty() { None } else { Some(lines) }
}

fn quota_progress(
    label: &str,
    value: &Value,
    resets_at: Option<String>,
) -> Option<MetricLinePayload> {
    let percent = quota_used_percent(value)?;

    Some(MetricLinePayload::Progress {
        label: label.to_string(),
        used: percent,
        limit: 100.0,
        format: MetricFormat::Percent,
        resets_at,
        color: None,
    })
}

fn quota_used_percent(value: &Value) -> Option<f64> {
    if let Some(percent) = first_numeric_field(
        value,
        &[
            "usedPercent",
            "used_percent",
            "percentUsed",
            "percent_used",
            "usagePercent",
            "usage_percent",
        ],
    ) {
        return Some(normalize_percent(percent));
    }

    if let Some(remaining_percent) = first_numeric_field(
        value,
        &[
            "remainingPercent",
            "remaining_percent",
            "percentRemaining",
            "percent_remaining",
        ],
    ) {
        return Some((100.0 - normalize_percent(remaining_percent)).clamp(0.0, 100.0));
    }

    let limit = first_numeric_field(value, &["limit", "total", "quota"])?;
    if limit <= 0.0 {
        return None;
    }

    let used = first_numeric_field(value, &["used", "usage", "consumed"])
        .or_else(|| first_numeric_field(value, &["remaining", "left"]).map(|remaining| limit - remaining))
        .unwrap_or(0.0)
        .max(0.0);

    Some((used / limit * 100.0).clamp(0.0, 100.0))
}

fn normalize_percent(value: f64) -> f64 {
    if value <= 1.0 {
        (value * 100.0).clamp(0.0, 100.0)
    } else {
        value.clamp(0.0, 100.0)
    }
}

fn first_numeric_field(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| field_as_f64(value, key))
}

fn reset_time(value: &Value) -> Option<String> {
    value
        .get("resetTime")
        .or_else(|| value.get("reset_time"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn first_limit_reset_time(value: &Value) -> Option<String> {
    value
        .get("limits")
        .or_else(|| value.pointer("/data/limits"))
        .and_then(Value::as_array)?
        .iter()
        .find_map(|limit| limit.get("detail").and_then(reset_time))
}

fn humanize_limit_window(value: &Value) -> String {
    let duration = value
        .get("duration")
        .and_then(Value::as_i64)
        .unwrap_or_default();
    let unit = value
        .get("timeUnit")
        .or_else(|| value.get("time_unit"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim_start_matches("TIME_UNIT_");

    match unit {
        "SECOND" | "SECONDS" => format!("{duration}s"),
        "MINUTE" | "MINUTES" if duration >= 60 && duration % 60 == 0 => {
            format!("{}-min", duration / 60)
        }
        "MINUTE" | "MINUTES" => format!("{duration}-min"),
        "HOUR" | "HOURS" => format!("{duration}h"),
        _ if duration > 0 => duration.to_string(),
        _ => "Window".to_string(),
    }
}

fn field_as_f64(value: &Value, key: &str) -> Option<f64> {
    let field = value.get(key)?;
    field
        .as_f64()
        .or_else(|| field.as_str()?.trim().parse::<f64>().ok())
}

fn decode_jwt_payload(access_token: &str) -> Option<Value> {
    let payload = access_token.split('.').nth(1)?;
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    serde_json::from_slice(&decoded).ok()
}

fn infer_plan(me: Option<&Value>) -> Option<String> {
    let me = me?;
    let subscription = me
        .pointer("/subscription/name")
        .or_else(|| me.pointer("/subscription/plan"))
        .or_else(|| me.pointer("/subscription/type"))
        .or_else(|| me.pointer("/user/membership/level"))
        .or_else(|| me.pointer("/plan"))
        .or_else(|| me.pointer("/data/subscription/name"))
        .or_else(|| me.pointer("/data/subscription/plan"))
        .or_else(|| me.pointer("/data/subscription/type"))
        .or_else(|| me.pointer("/data/user/membership/level"))?;

    value_to_non_empty_string(subscription).map(|value| {
        value
            .strip_prefix("LEVEL_")
            .map(super::shared::title_case)
            .unwrap_or(value)
    })
}

fn find_user_id(value: &Value) -> Option<String> {
    value_to_non_empty_string(
        value
            .get("user_id")
            .or_else(|| value.get("userId"))
            .or_else(|| value.pointer("/user/id"))
            .or_else(|| value.pointer("/data/user_id"))
            .or_else(|| value.pointer("/data/userId"))
            .or_else(|| value.pointer("/data/user/id"))?,
    )
}

fn find_scope(value: &Value) -> Option<String> {
    let scope = value.get("scope")?;
    if let Some(text) = scope.as_str() {
        let trimmed = text.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }

    let joined = scope.as_array().map(|items| {
        items
            .iter()
            .filter_map(Value::as_str)
            .filter(|item| !item.trim().is_empty())
            .collect::<Vec<_>>()
            .join(" ")
    })?;
    (!joined.is_empty()).then_some(joined)
}

fn find_region(value: &Value) -> Option<String> {
    value
        .pointer("/user/region")
        .or_else(|| value.pointer("/data/user/region"))
        .and_then(value_to_non_empty_string)
        .map(|value| humanize_enum_value(&value, "REGION_"))
}

fn find_parallel_limit(value: &Value) -> Option<String> {
    value
        .pointer("/parallel/limit")
        .or_else(|| value.pointer("/data/parallel/limit"))
        .and_then(value_to_non_empty_string)
}

fn humanize_enum_value(value: &str, prefix: &str) -> String {
    value
        .strip_prefix(prefix)
        .map(super::shared::title_case)
        .unwrap_or_else(|| value.to_string())
}

fn value_to_non_empty_string(value: &Value) -> Option<String> {
    if let Some(text) = value.as_str() {
        let trimmed = text.trim();
        return (!trimmed.is_empty()).then(|| trimmed.to_string());
    }

    value
        .as_i64()
        .map(|number| number.to_string())
        .or_else(|| value.as_u64().map(|number| number.to_string()))
}

fn handle_rate_limit(
    state: &'static Mutex<super::fetch_state::ProviderFetchState>,
    now_ms: i64,
    headers: &reqwest::header::HeaderMap,
) -> Result<UsagePayload, String> {
    let retry_after = parse_retry_after_seconds(headers);
    super::fetch_state::record_rate_limit(
        state,
        now_ms,
        retry_after,
        DEFAULT_RATE_LIMIT_BACKOFF_MS,
    )?;

    if let Some(cached) = super::fetch_state::cached_payload(state)? {
        return Ok(super::fetch_state::with_status_badge(
            &cached,
            format_rate_limited_value(retry_after),
            "warn",
        ));
    }

    Err(format_rate_limited_value(retry_after))
}

fn handle_backoff_failure(
    state: &'static Mutex<super::fetch_state::ProviderFetchState>,
    now_ms: i64,
    access_token: &str,
    message: String,
) -> Result<UsagePayload, String> {
    if let Some(payload) = super::fetch_state::record_backoff_failure(state, now_ms)? {
        return Ok(payload);
    }

    local_fallback_payload(
        Some(access_token),
        "Local logs only - Kimi API unavailable",
        "warn",
        Some(message),
    )
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

fn is_auth_status(status: reqwest::StatusCode) -> bool {
    status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN
}

fn map_kimi_credential_error(error: CredentialError) -> String {
    match error {
        CredentialError::NotConfigured => {
            "Kimi credentials not configured. Run `kimi login` and Retry.".to_string()
        }
        CredentialError::RefreshFailed(reason) | CredentialError::Io(reason) => reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn usage_progress_reads_kimi_subscription_quota_response() {
        let payload = json!({
            "usage": {
                "limit": "100",
                "remaining": "75",
                "resetTime": "2026-04-30T16:59:35.398070Z"
            },
            "limits": [{
                "window": { "duration": 300, "timeUnit": "TIME_UNIT_MINUTE" },
                "detail": {
                    "limit": "100",
                    "remaining": "80",
                    "resetTime": "2026-04-24T07:59:35.398070Z"
                }
            }],
            "totalQuota": {
                "limit": "100",
                "remaining": "99"
            }
        });

        let lines = usage_progress(Some(&payload)).expect("expected progress lines");
        let progress = lines
            .iter()
            .map(|line| match line {
                MetricLinePayload::Progress { label, used, .. } => (label.as_str(), *used),
                _ => unreachable!("usage_progress should only return progress lines"),
            })
            .collect::<Vec<_>>();

        assert!(progress.contains(&("Subscription Usage", 1.0)));
        assert!(progress.contains(&("5-min Window", 25.0)));
        assert!(progress.contains(&("5-min", 20.0)));
    }

    #[test]
    fn quota_used_percent_accepts_percent_and_remaining_shapes() {
        assert_eq!(
            quota_used_percent(&json!({ "usedPercent": 0.42 })).unwrap(),
            42.0
        );
        assert_eq!(
            quota_used_percent(&json!({ "percentRemaining": 80 })).unwrap(),
            20.0
        );
        assert_eq!(
            quota_used_percent(&json!({ "limit": "200", "used": "50" })).unwrap(),
            25.0
        );
        assert_eq!(
            quota_used_percent(&json!({ "limit": "200", "remaining": "50" })).unwrap(),
            75.0
        );
    }
}
