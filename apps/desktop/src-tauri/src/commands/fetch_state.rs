use super::shared::{MetricLinePayload, UsagePayload};
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RetryKind {
    #[default]
    None,
    Backoff,
    RateLimited,
}

#[derive(Default)]
pub struct ProviderFetchState {
    /// Most recent successful payload; returned for cache hits and during backoff.
    pub cached_payload: Option<UsagePayload>,
    /// Refresh interval used to compute the current cache TTL.
    pub cache_refresh_interval_minutes: Option<u32>,
    /// Epoch-ms until which the cache is considered fresh.
    pub cache_fresh_until_ms: i64,
    /// Consecutive network/5xx failure count, used to compute next attempt.
    pub consecutive_failures: u32,
    /// Epoch-ms until which we should not attempt a new request unless forced.
    pub retry_after_ms: i64,
    /// Distinguishes generic backoff from explicit server rate limiting.
    pub retry_kind: RetryKind,
}

pub fn backoff_ms(consecutive_failures: u32) -> i64 {
    let exponent = consecutive_failures.saturating_sub(1).min(5);
    1_000_i64.saturating_mul(1_i64 << exponent).min(30_000)
}

pub fn current_time_ms() -> i64 {
    (time::OffsetDateTime::now_utc().unix_timestamp_nanos() / 1_000_000) as i64
}

pub fn ttl_ms(refresh_interval_minutes: u32) -> i64 {
    (i64::from(refresh_interval_minutes).max(1) * 60_000) / 2
}

pub fn read_cached_or_stalled_payload(
    state: &'static Mutex<ProviderFetchState>,
    now_ms: i64,
    refresh_interval_minutes: u32,
    force: bool,
) -> Result<Option<UsagePayload>, String> {
    if force {
        return Ok(None);
    }

    let state = state.lock().map_err(|error| error.to_string())?;
    let cache_matches = state.cache_refresh_interval_minutes == Some(refresh_interval_minutes);
    let Some(cached_payload) = state.cached_payload.as_ref() else {
        return Ok(None);
    };

    if !cache_matches {
        return Ok(None);
    }

    if now_ms < state.cache_fresh_until_ms {
        return Ok(Some(as_cached_payload(cached_payload)));
    }

    if now_ms < state.retry_after_ms {
        return Ok(Some(match state.retry_kind {
            RetryKind::Backoff => with_status_badge(
                cached_payload,
                format_retrying_in_ms(state.retry_after_ms - now_ms),
                "warn",
            ),
            RetryKind::RateLimited => with_status_badge(
                cached_payload,
                format_rate_limited_in_seconds(((state.retry_after_ms - now_ms) / 1000).max(0)),
                "warn",
            ),
            RetryKind::None => as_cached_payload(cached_payload),
        }));
    }

    Ok(None)
}

pub fn record_success(
    state: &'static Mutex<ProviderFetchState>,
    payload: &UsagePayload,
    now_ms: i64,
    refresh_interval_minutes: u32,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    state.cached_payload = Some(payload.clone());
    state.cache_refresh_interval_minutes = Some(refresh_interval_minutes);
    state.cache_fresh_until_ms = now_ms + ttl_ms(refresh_interval_minutes);
    state.consecutive_failures = 0;
    state.retry_after_ms = 0;
    state.retry_kind = RetryKind::None;
    Ok(())
}

pub fn record_backoff_failure(
    state: &'static Mutex<ProviderFetchState>,
    now_ms: i64,
) -> Result<Option<UsagePayload>, String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    state.consecutive_failures = state.consecutive_failures.saturating_add(1);
    state.retry_after_ms = now_ms + backoff_ms(state.consecutive_failures);
    state.retry_kind = RetryKind::Backoff;

    Ok(state.cached_payload.as_ref().map(|cached_payload| {
        with_status_badge(
            cached_payload,
            format_retrying_in_ms(state.retry_after_ms - now_ms),
            "warn",
        )
    }))
}

pub fn record_rate_limit(
    state: &'static Mutex<ProviderFetchState>,
    now_ms: i64,
    retry_after_seconds: Option<i64>,
    fallback_ms: i64,
) -> Result<(), String> {
    let mut state = state.lock().map_err(|error| error.to_string())?;
    let backoff_ms = retry_after_seconds
        .map(|seconds| seconds.max(0) * 1_000)
        .unwrap_or(fallback_ms.max(0));
    state.retry_after_ms = now_ms + backoff_ms;
    state.retry_kind = RetryKind::RateLimited;
    Ok(())
}

pub fn cached_payload(
    state: &'static Mutex<ProviderFetchState>,
) -> Result<Option<UsagePayload>, String> {
    let state = state.lock().map_err(|error| error.to_string())?;
    Ok(state.cached_payload.clone())
}

pub fn as_cached_payload(payload: &UsagePayload) -> UsagePayload {
    UsagePayload {
        provider_id: payload.provider_id,
        plan: payload.plan.clone(),
        lines: payload.lines.clone(),
        source: "cache",
    }
}

pub fn with_status_badge(payload: &UsagePayload, value: String, tone: &str) -> UsagePayload {
    let mut lines = Vec::with_capacity(payload.lines.len() + 1);
    lines.push(MetricLinePayload::Badge {
        label: "Status".to_string(),
        value,
        tone: Some(tone.to_string()),
    });
    lines.extend(payload.lines.clone());

    UsagePayload {
        provider_id: payload.provider_id,
        plan: payload.plan.clone(),
        lines,
        source: "cache",
    }
}

fn format_retrying_in_ms(remaining_ms: i64) -> String {
    let seconds = ((remaining_ms.max(0) + 999) / 1_000).max(1);
    format!("Retrying in ~{seconds}s")
}

fn format_rate_limited_in_seconds(seconds: i64) -> String {
    if seconds <= 0 {
        return "Rate limited, retry now".to_string();
    }

    let minutes = ((seconds + 59) / 60).max(1);
    format!("Rate limited, retry in ~{minutes}m")
}
