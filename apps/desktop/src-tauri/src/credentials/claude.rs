use super::cache::{keychain_entry, prime_keychain_cache, read_keychain_cached};
use super::{
    Credential, CredentialError, CredentialSource, format_rfc3339, io_error, is_expired,
    parse_rfc3339, trim_to_configured,
};
use async_trait::async_trait;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use time::{Duration, OffsetDateTime};

const KEYCHAIN_SERVICE_PREFIX: &str = "Claude Code";
const PROD_BASE_API_URL: &str = "https://api.anthropic.com";
const PROD_REFRESH_URL: &str = "https://platform.claude.com/v1/oauth/token";
const PROD_CLIENT_ID: &str = "9d1c250a-e61b-44d9-88ed-5944d1962f5e";
const NON_PROD_CLIENT_ID: &str = "22422756-60c9-4084-8eb7-27705fd5cf9a";
const SCOPES: &str =
    "user:profile user:inference user:sessions:claude_code user:mcp_servers user:file_upload";

#[derive(Debug, Clone)]
pub(crate) struct ClaudeOauthConfig {
    pub(crate) usage_url: String,
    pub(crate) refresh_url: String,
    pub(crate) client_id: String,
    pub(crate) keychain_service: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ClaudeCredentialsFile {
    #[serde(default, alias = "claude_ai_oauth")]
    claude_ai_oauth: ClaudeOauth,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct ClaudeOauth {
    #[serde(alias = "access_token")]
    access_token: Option<String>,
    #[serde(alias = "refresh_token")]
    refresh_token: Option<String>,
    #[serde(alias = "expires_at")]
    expires_at: Option<StoredExpiry>,
    #[serde(alias = "scopes")]
    scopes: Option<Vec<String>>,
    #[serde(alias = "subscription_type")]
    subscription_type: Option<String>,
    #[serde(alias = "rate_limit_tier")]
    rate_limit_tier: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum StoredExpiry {
    Iso(String),
    Integer(i64),
    Float(f64),
}

impl StoredExpiry {
    fn to_offset_datetime(&self) -> Option<OffsetDateTime> {
        match self {
            StoredExpiry::Iso(value) => parse_rfc3339(value),
            StoredExpiry::Integer(value) => {
                if *value >= 1_000_000_000_000 {
                    super::from_unix_millis(*value)
                } else {
                    super::from_unix_seconds(*value)
                }
            }
            StoredExpiry::Float(value) => {
                let rounded = value.round() as i64;
                if rounded >= 1_000_000_000_000 {
                    super::from_unix_millis(rounded)
                } else {
                    super::from_unix_seconds(rounded)
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
enum ClaudeCredentialLocation {
    File(PathBuf),
    Keychain,
}

#[derive(Debug, Clone)]
struct ClaudeStoredCredentials {
    oauth: ClaudeOauth,
    source: ClaudeCredentialLocation,
    full_data: ClaudeCredentialsFile,
}

#[derive(Debug, Clone)]
struct ClaudeResolvedCredentials {
    oauth: ClaudeOauth,
    source: Option<ClaudeCredentialLocation>,
    full_data: Option<ClaudeCredentialsFile>,
    inference_only: bool,
}

#[derive(Debug, Deserialize)]
struct ClaudeRefreshResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ClaudeRefreshError {
    error: Option<String>,
}

#[derive(Default)]
pub struct ClaudeSource {
    inner: tokio::sync::Mutex<()>,
}

#[async_trait]
impl CredentialSource for ClaudeSource {
    fn provider_id(&self) -> &'static str {
        "claude"
    }

    async fn load(&self) -> Result<Credential, CredentialError> {
        let _guard = self.inner.lock().await;
        let mut credentials = resolve_credentials()?;
        let mut access_token = trim_to_configured(credentials.oauth.access_token.clone())
            .ok_or(CredentialError::NotConfigured)?;
        let mut expires_at = credentials
            .oauth
            .expires_at
            .as_ref()
            .and_then(StoredExpiry::to_offset_datetime);

        if !credentials.inference_only && expires_at.as_ref().is_some_and(is_expired) {
            access_token = refresh_access_token(&mut credentials).await?;
            expires_at = credentials
                .oauth
                .expires_at
                .as_ref()
                .and_then(StoredExpiry::to_offset_datetime);
        }

        Ok(Credential::OAuth {
            access_token,
            refresh_token: trim_to_configured(credentials.oauth.refresh_token.clone()),
            expires_at,
        })
    }
}

impl ClaudeSource {
    pub async fn load_plan_label(&self) -> Result<Option<String>, CredentialError> {
        let _guard = self.inner.lock().await;
        Ok(plan_label(&resolve_credentials()?.oauth))
    }
}

pub(crate) fn oauth_config() -> ClaudeOauthConfig {
    let mut base_api_url = PROD_BASE_API_URL.to_string();
    let mut refresh_url = PROD_REFRESH_URL.to_string();
    let mut client_id = PROD_CLIENT_ID.to_string();
    let mut oauth_file_suffix = String::new();

    let is_ant_user = read_env_text("USER_TYPE").as_deref() == Some("ant");
    if is_ant_user && read_env_flag("USE_LOCAL_OAUTH") {
        let local_api_base = read_env_text("CLAUDE_LOCAL_OAUTH_API_BASE")
            .unwrap_or_else(|| "http://localhost:8000".to_string());
        base_api_url = trim_trailing_slashes(&local_api_base).to_string();
        refresh_url = format!("{base_api_url}/v1/oauth/token");
        client_id = NON_PROD_CLIENT_ID.to_string();
        oauth_file_suffix = "-local-oauth".to_string();
    } else if is_ant_user && read_env_flag("USE_STAGING_OAUTH") {
        base_api_url = "https://api-staging.anthropic.com".to_string();
        refresh_url = "https://platform.staging.ant.dev/v1/oauth/token".to_string();
        client_id = NON_PROD_CLIENT_ID.to_string();
        oauth_file_suffix = "-staging-oauth".to_string();
    }

    if let Some(custom_oauth_base) = read_env_text("CLAUDE_CODE_CUSTOM_OAUTH_URL") {
        let base = trim_trailing_slashes(&custom_oauth_base);
        base_api_url = base.to_string();
        refresh_url = format!("{base}/v1/oauth/token");
        oauth_file_suffix = "-custom-oauth".to_string();
    }

    if let Some(client_id_override) = read_env_text("CLAUDE_CODE_OAUTH_CLIENT_ID") {
        client_id = client_id_override;
    }

    ClaudeOauthConfig {
        usage_url: format!("{base_api_url}/api/oauth/usage"),
        refresh_url,
        client_id,
        keychain_service: format!("{KEYCHAIN_SERVICE_PREFIX}{oauth_file_suffix}-credentials"),
    }
}

fn resolve_credentials() -> Result<ClaudeResolvedCredentials, CredentialError> {
    let env_token = trim_to_configured(std::env::var("CLAUDE_CODE_OAUTH_TOKEN").ok());
    if let Some(env_token) = env_token {
        let oauth = ClaudeOauth {
            access_token: Some(env_token),
            ..Default::default()
        };
        return Ok(ClaudeResolvedCredentials {
            oauth,
            source: None,
            full_data: None,
            inference_only: true,
        });
    }

    let stored = load_stored_credentials()?;
    if let Some(stored) = stored {
        return Ok(ClaudeResolvedCredentials {
            oauth: stored.oauth,
            source: Some(stored.source),
            full_data: Some(stored.full_data),
            inference_only: false,
        });
    }

    Err(CredentialError::NotConfigured)
}

fn load_stored_credentials() -> Result<Option<ClaudeStoredCredentials>, CredentialError> {
    let oauth_config = oauth_config();

    if let Some(credentials_path) = claude_credentials_path()
        && credentials_path.exists()
    {
        let raw = fs::read_to_string(&credentials_path).map_err(io_error)?;
        let parsed = parse_claude_credentials_json(&raw)
            .ok_or_else(|| CredentialError::Io("failed to parse Claude credential file".into()))?;

        if trim_to_configured(parsed.claude_ai_oauth.access_token.clone()).is_some() {
            return Ok(Some(ClaudeStoredCredentials {
                oauth: parsed.claude_ai_oauth.clone(),
                source: ClaudeCredentialLocation::File(credentials_path),
                full_data: parsed,
            }));
        }
    }

    let Some(raw) = read_keychain_cached(&oauth_config.keychain_service) else {
        return Ok(None);
    };
    let parsed = parse_claude_credentials_json(&raw).ok_or_else(|| {
        CredentialError::Io("failed to parse Claude credential keychain entry".into())
    })?;

    if trim_to_configured(parsed.claude_ai_oauth.access_token.clone()).is_some() {
        return Ok(Some(ClaudeStoredCredentials {
            oauth: parsed.claude_ai_oauth.clone(),
            source: ClaudeCredentialLocation::Keychain,
            full_data: parsed,
        }));
    }

    Ok(None)
}

fn claude_credentials_path() -> Option<PathBuf> {
    if let Ok(override_dir) = std::env::var("CLAUDE_CONFIG_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed).join(".credentials.json"));
        }
    }

    home_dir().map(|home| home.join(".claude").join(".credentials.json"))
}

fn parse_claude_credentials_json(raw: &str) -> Option<ClaudeCredentialsFile> {
    serde_json::from_str::<ClaudeCredentialsFile>(raw)
        .ok()
        .or_else(|| decode_hex_json(raw))
}

fn decode_hex_json(raw: &str) -> Option<ClaudeCredentialsFile> {
    let hex = raw
        .trim()
        .strip_prefix("0x")
        .or_else(|| raw.trim().strip_prefix("0X"))
        .unwrap_or(raw.trim());

    if hex.is_empty()
        || !hex.len().is_multiple_of(2)
        || !hex.chars().all(|char| char.is_ascii_hexdigit())
    {
        return None;
    }

    let bytes = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).ok())
        .collect::<Option<Vec<_>>>()?;
    let decoded = String::from_utf8(bytes).ok()?;
    serde_json::from_str::<ClaudeCredentialsFile>(&decoded).ok()
}

async fn refresh_access_token(
    credentials: &mut ClaudeResolvedCredentials,
) -> Result<String, CredentialError> {
    let oauth_config = oauth_config();
    let refresh_token = trim_to_configured(credentials.oauth.refresh_token.clone())
        .ok_or_else(|| CredentialError::RefreshFailed("no refresh token".into()))?;

    let response = reqwest::Client::new()
        .post(&oauth_config.refresh_url)
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "grant_type": "refresh_token",
            "refresh_token": refresh_token,
            "client_id": oauth_config.client_id,
            "scope": SCOPES,
        }))
        .send()
        .await
        .map_err(|error| CredentialError::RefreshFailed(error.to_string()))?;

    if response.status() == reqwest::StatusCode::BAD_REQUEST
        || response.status() == reqwest::StatusCode::UNAUTHORIZED
    {
        let body = response.json::<ClaudeRefreshError>().await.ok();
        let reason =
            if body.as_ref().and_then(|value| value.error.as_deref()) == Some("invalid_grant") {
                "invalid grant"
            } else {
                "unauthorized"
            };
        return Err(CredentialError::RefreshFailed(reason.into()));
    }

    if !response.status().is_success() {
        return Err(CredentialError::RefreshFailed(format!(
            "HTTP {}",
            response.status().as_u16()
        )));
    }

    let payload: ClaudeRefreshResponse = response
        .json()
        .await
        .map_err(|error| CredentialError::RefreshFailed(error.to_string()))?;
    let access_token = trim_to_configured(payload.access_token)
        .ok_or_else(|| CredentialError::RefreshFailed("missing access token".into()))?;
    let expires_at =
        OffsetDateTime::now_utc() + Duration::seconds(payload.expires_in.unwrap_or(3600));

    credentials.oauth.access_token = Some(access_token.clone());
    if let Some(refresh_token) = trim_to_configured(payload.refresh_token) {
        credentials.oauth.refresh_token = Some(refresh_token);
    }
    credentials.oauth.expires_at = Some(StoredExpiry::Iso(format_rfc3339(expires_at)?));

    persist_credentials(credentials)?;
    Ok(access_token)
}

fn persist_credentials(credentials: &ClaudeResolvedCredentials) -> Result<(), CredentialError> {
    let Some(source) = credentials.source.as_ref() else {
        return Ok(());
    };

    let mut full_data = credentials.full_data.clone().unwrap_or_default();
    full_data.claude_ai_oauth = credentials.oauth.clone();
    let encoded = serde_json::to_string(&full_data).map_err(io_error)?;
    let oauth_config = oauth_config();

    match source {
        ClaudeCredentialLocation::File(path) => write_atomic(path, &encoded),
        ClaudeCredentialLocation::Keychain => {
            keychain_entry(&oauth_config.keychain_service)
                .map_err(io_error)?
                .set_password(&encoded)
                .map_err(io_error)?;
            prime_keychain_cache(&oauth_config.keychain_service, encoded);
            Ok(())
        }
    }
}

fn write_atomic(path: &Path, content: &str) -> Result<(), CredentialError> {
    let parent = path.parent().ok_or_else(|| {
        CredentialError::Io("Claude credential path missing parent directory".into())
    })?;
    fs::create_dir_all(parent).map_err(io_error)?;

    let mut temp = NamedTempFile::new_in(parent).map_err(io_error)?;
    temp.write_all(content.as_bytes()).map_err(io_error)?;
    temp.flush().map_err(io_error)?;
    temp.persist(path).map_err(|error| io_error(error.error))?;
    Ok(())
}

fn plan_label(oauth: &ClaudeOauth) -> Option<String> {
    let subscription_type = oauth.subscription_type.as_deref()?.trim();
    if subscription_type.is_empty() {
        return None;
    }

    let mut plan = super::super::commands::shared::title_case(subscription_type);
    if subscription_type.eq_ignore_ascii_case("max")
        && let Some(rate_limit_tier) = oauth.rate_limit_tier.as_deref()
    {
        let trimmed = rate_limit_tier.trim();
        if !trimmed.is_empty() {
            plan.push(' ');
            plan.push_str(trimmed);
        }
    }

    Some(plan)
}

fn read_env_text(name: &str) -> Option<String> {
    let value = std::env::var(name).ok()?;
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn read_env_flag(name: &str) -> bool {
    let Some(value) = read_env_text(name) else {
        return false;
    };
    let lower = value.to_ascii_lowercase();
    !matches!(lower.as_str(), "0" | "false" | "no" | "off")
}

fn trim_trailing_slashes(value: &str) -> &str {
    value.trim_end_matches('/')
}
