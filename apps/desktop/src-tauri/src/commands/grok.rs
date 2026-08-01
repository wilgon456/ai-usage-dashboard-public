use super::shared::{self, MetricFormat, MetricLinePayload, UsagePayload};
use dirs::home_dir;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use time::{Duration, OffsetDateTime};

const DEFAULT_PROXY_BASE: &str = "https://cli-chat-proxy.grok.com/v1";

#[derive(Debug, Deserialize)]
struct OidcMetadata {
    token_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

struct StoredAuth {
    path: PathBuf,
    root: Value,
    entry_key: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<OffsetDateTime>,
    issuer: Option<String>,
    client_id: Option<String>,
}

#[tauri::command]
pub async fn get_grok_usage(
    _refresh_interval_minutes: u32,
    _force: bool,
) -> Result<UsagePayload, String> {
    let client = Client::new();
    let mut auth = load_auth()?;

    if auth
        .expires_at
        .is_some_and(|expiry| expiry <= OffsetDateTime::now_utc() + Duration::minutes(5))
    {
        refresh_auth(&client, &mut auth).await?;
    }

    let base = std::env::var("GROK_CLI_CHAT_PROXY_BASE_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PROXY_BASE.to_string());
    let url = format!("{}/billing?format=credits", base.trim_end_matches('/'));
    let response = client
        .get(url)
        .bearer_auth(auth.access_token.trim())
        .header("x-grok-client-mode", "cli")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| format!("Grok usage request failed: {error}"))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        refresh_auth(&client, &mut auth).await?;
        return fetch_billing(&client, &base, &auth.access_token).await;
    }

    parse_billing_response(response).await
}

async fn fetch_billing(
    client: &Client,
    base: &str,
    access_token: &str,
) -> Result<UsagePayload, String> {
    let response = client
        .get(format!(
            "{}/billing?format=credits",
            base.trim_end_matches('/')
        ))
        .bearer_auth(access_token.trim())
        .header("x-grok-client-mode", "cli")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|error| format!("Grok usage request failed: {error}"))?;
    parse_billing_response(response).await
}

async fn parse_billing_response(response: reqwest::Response) -> Result<UsagePayload, String> {
    let status = response.status();
    if !status.is_success() {
        return Err(if status == reqwest::StatusCode::UNAUTHORIZED {
            "Grok login expired. Run `grok login` and refresh.".to_string()
        } else {
            format!("Grok billing endpoint returned HTTP {}.", status.as_u16())
        });
    }

    let json: Value = response
        .json()
        .await
        .map_err(|error| format!("Failed to parse Grok billing data: {error}"))?;
    let used_percent = number_at(
        &json,
        &[
            "/config/creditUsagePercent",
            "/config/credit_usage_percent",
            "/creditUsagePercent",
            "/credit_usage_percent",
        ],
    )
    .or_else(|| {
        let used = number_at(
            &json,
            &[
                "/config/includedUsed",
                "/config/included_used",
                "/includedUsed",
                "/included_used",
            ],
        )?;
        let limit = number_at(
            &json,
            &[
                "/config/monthlyLimit",
                "/config/monthly_limit",
                "/monthlyLimit",
                "/monthly_limit",
            ],
        )?;
        (limit > 0.0).then_some((used / limit) * 100.0)
    })
    .ok_or_else(|| "Grok billing data did not include a usage percentage.".to_string())?
    .clamp(0.0, 100.0);

    let resets_at = string_at(
        &json,
        &[
            "/config/currentPeriod/end",
            "/config/currentPeriod/endsAt",
            "/config/current_period/end",
            "/config/current_period/ends_at",
            "/config/billingPeriodEnd",
            "/currentPeriod/end",
            "/currentPeriod/endsAt",
            "/current_period/end",
            "/current_period/ends_at",
            "/end",
        ],
    );
    let plan = string_at(
        &json,
        &[
            "/config/subscription_tier",
            "/config/subscriptionTier",
            "/subscription_tier",
            "/subscriptionTier",
        ],
    )
    .map(|value| shared::title_case(&value))
    .unwrap_or_else(|| "Grok weekly credits".to_string());

    Ok(UsagePayload {
        provider_id: "grok",
        plan: Some(plan),
        lines: vec![MetricLinePayload::Progress {
            label: "Weekly credits".to_string(),
            used: used_percent,
            limit: 100.0,
            format: MetricFormat::Percent,
            resets_at,
            color: None,
        }],
        source: "remote",
    })
}

fn load_auth() -> Result<StoredAuth, String> {
    let path = grok_auth_path()?;
    let raw = fs::read_to_string(&path)
        .map_err(|_| "Grok login not found. Run `grok login`.".to_string())?;
    let root: Value = serde_json::from_str(&raw)
        .map_err(|error| format!("Failed to parse Grok auth file: {error}"))?;
    let entries = root
        .as_object()
        .ok_or_else(|| "Grok auth file has an unexpected format.".to_string())?;
    let (entry_key, entry) = entries
        .iter()
        .find(|(_, value)| value.get("key").and_then(Value::as_str).is_some())
        .ok_or_else(|| "Grok login not found. Run `grok login`.".to_string())?;

    Ok(StoredAuth {
        path,
        root: root.clone(),
        entry_key: entry_key.clone(),
        access_token: required_string(entry, "key")?,
        refresh_token: optional_string(entry, "refresh_token"),
        expires_at: optional_string(entry, "expires_at").and_then(|value| {
            OffsetDateTime::parse(&value, &time::format_description::well_known::Rfc3339).ok()
        }),
        issuer: optional_string(entry, "oidc_issuer"),
        client_id: optional_string(entry, "oidc_client_id"),
    })
}

async fn refresh_auth(client: &Client, auth: &mut StoredAuth) -> Result<(), String> {
    let refresh_token = auth
        .refresh_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Grok login expired. Run `grok login`.".to_string())?;
    let issuer = auth
        .issuer
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Grok login issuer is missing. Run `grok login`.".to_string())?;
    let client_id = auth
        .client_id
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Grok login client ID is missing. Run `grok login`.".to_string())?;

    let metadata: OidcMetadata = client
        .get(format!(
            "{}/.well-known/openid-configuration",
            issuer.trim_end_matches('/')
        ))
        .send()
        .await
        .map_err(|error| format!("Grok login refresh discovery failed: {error}"))?
        .error_for_status()
        .map_err(|error| format!("Grok login refresh discovery failed: {error}"))?
        .json()
        .await
        .map_err(|error| format!("Grok login refresh discovery failed: {error}"))?;

    let refreshed: RefreshResponse = client
        .post(metadata.token_endpoint)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ])
        .send()
        .await
        .map_err(|error| format!("Grok login refresh failed: {error}"))?
        .error_for_status()
        .map_err(|_| "Grok login expired. Run `grok login`.".to_string())?
        .json()
        .await
        .map_err(|error| format!("Grok login refresh failed: {error}"))?;

    auth.access_token = refreshed.access_token;
    if let Some(refresh_token) = refreshed.refresh_token {
        auth.refresh_token = Some(refresh_token);
    }
    auth.expires_at =
        Some(OffsetDateTime::now_utc() + Duration::seconds(refreshed.expires_in.unwrap_or(3600)));
    persist_auth(auth)
}

fn persist_auth(auth: &mut StoredAuth) -> Result<(), String> {
    let entry = auth
        .root
        .as_object_mut()
        .and_then(|entries| entries.get_mut(&auth.entry_key))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "Grok auth file changed while refreshing.".to_string())?;
    entry.insert("key".to_string(), Value::String(auth.access_token.clone()));
    if let Some(refresh_token) = auth.refresh_token.as_ref() {
        entry.insert(
            "refresh_token".to_string(),
            Value::String(refresh_token.clone()),
        );
    }
    if let Some(expires_at) = auth.expires_at.and_then(shared::to_rfc3339) {
        entry.insert("expires_at".to_string(), Value::String(expires_at));
    }

    let encoded = serde_json::to_vec_pretty(&auth.root)
        .map_err(|error| format!("Failed to encode Grok auth: {error}"))?;
    fs::write(&auth.path, encoded)
        .map_err(|error| format!("Failed to save refreshed Grok login: {error}"))
}

fn grok_auth_path() -> Result<PathBuf, String> {
    home_dir()
        .map(|home| home.join(".grok").join("auth.json"))
        .ok_or_else(|| "No home directory available.".to_string())
}

fn required_string(value: &Value, key: &str) -> Result<String, String> {
    optional_string(value, key).ok_or_else(|| format!("Grok auth field `{key}` is missing."))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn number_at(value: &Value, pointers: &[&str]) -> Option<f64> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_f64))
}

fn string_at(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers.iter().find_map(|pointer| {
        value
            .pointer(pointer)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_credit_percent_from_included_usage() {
        let json = serde_json::json!({"includedUsed": 25.0, "monthlyLimit": 100.0});
        let used = number_at(&json, &["/includedUsed"]).unwrap();
        let limit = number_at(&json, &["/monthlyLimit"]).unwrap();
        assert_eq!((used / limit) * 100.0, 25.0);
    }
}
