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

const CLIENT_ID: &str = "app_EMoamEEZ73f0CkXaXp7hrann";
const REFRESH_URL: &str = "https://auth.openai.com/oauth/token";
const LEGACY_REFRESH_WINDOW: Duration = Duration::days(8);

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AuthState {
    auth_mode: Option<String>,
    #[serde(rename = "OPENAI_API_KEY")]
    openai_api_key: Option<String>,
    tokens: Option<AuthTokens>,
    last_refresh: Option<String>,
    #[serde(default)]
    expires_at: Option<StoredExpiry>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct AuthTokens {
    id_token: Option<String>,
    access_token: Option<String>,
    refresh_token: Option<String>,
    account_id: Option<String>,
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

#[derive(Debug, Deserialize)]
struct RefreshResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    id_token: Option<String>,
    expires_in: Option<i64>,
}

#[derive(Default)]
pub struct CodexSource {
    inner: tokio::sync::Mutex<()>,
}

#[async_trait]
impl CredentialSource for CodexSource {
    fn provider_id(&self) -> &'static str {
        "codex"
    }

    async fn load(&self) -> Result<Credential, CredentialError> {
        let _guard = self.inner.lock().await;
        let auth_path = find_auth_path().ok_or(CredentialError::NotConfigured)?;
        let mut auth_state = load_auth_state(&auth_path)?;
        let access_token = ensure_fresh_access_token(&auth_path, &mut auth_state).await?;
        let refresh_token = trim_to_configured(
            auth_state
                .tokens
                .as_ref()
                .and_then(|tokens| tokens.refresh_token.clone()),
        );
        let expires_at = resolve_expires_at(&auth_state);

        Ok(Credential::OAuth {
            access_token,
            refresh_token,
            expires_at,
        })
    }
}

impl CodexSource {
    pub async fn load_account_id(&self) -> Result<Option<String>, CredentialError> {
        let _guard = self.inner.lock().await;
        let auth_path = find_auth_path().ok_or(CredentialError::NotConfigured)?;
        let auth_state = load_auth_state(&auth_path)?;
        Ok(trim_to_configured(
            auth_state
                .tokens
                .as_ref()
                .and_then(|tokens| tokens.account_id.clone()),
        ))
    }
}

fn find_auth_path() -> Option<PathBuf> {
    let home = home_dir()?;
    let candidates = [
        home.join(".codex").join("auth.json"),
        home.join(".config").join("codex").join("auth.json"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn load_auth_state(path: &Path) -> Result<AuthState, CredentialError> {
    let raw = fs::read_to_string(path).map_err(io_error)?;
    serde_json::from_str(&raw).map_err(io_error)
}

fn save_auth_state(path: &Path, auth_state: &AuthState) -> Result<(), CredentialError> {
    let content = serde_json::to_string_pretty(auth_state).map_err(io_error)?;
    let parent = path
        .parent()
        .ok_or_else(|| CredentialError::Io("Codex auth path missing parent directory".into()))?;

    let mut temp = NamedTempFile::new_in(parent).map_err(io_error)?;
    temp.write_all(format!("{content}\n").as_bytes())
        .map_err(io_error)?;
    temp.flush().map_err(io_error)?;
    temp.persist(path).map_err(|error| io_error(error.error))?;
    Ok(())
}

fn resolve_expires_at(auth_state: &AuthState) -> Option<OffsetDateTime> {
    auth_state
        .expires_at
        .as_ref()
        .and_then(StoredExpiry::to_offset_datetime)
        .or_else(|| {
            auth_state
                .last_refresh
                .as_deref()
                .and_then(parse_rfc3339)
                .map(|timestamp| timestamp + LEGACY_REFRESH_WINDOW)
        })
}

async fn ensure_fresh_access_token(
    auth_path: &Path,
    auth_state: &mut AuthState,
) -> Result<String, CredentialError> {
    let mut access_token = trim_to_configured(
        auth_state
            .tokens
            .as_ref()
            .and_then(|tokens| tokens.access_token.clone()),
    )
    .ok_or(CredentialError::NotConfigured)?;

    if resolve_expires_at(auth_state)
        .as_ref()
        .is_some_and(is_expired)
    {
        access_token = refresh_access_token(auth_path, auth_state).await?;
    }

    Ok(access_token)
}

async fn refresh_access_token(
    auth_path: &Path,
    auth_state: &mut AuthState,
) -> Result<String, CredentialError> {
    let refresh_token = trim_to_configured(
        auth_state
            .tokens
            .as_ref()
            .and_then(|tokens| tokens.refresh_token.clone()),
    )
    .ok_or_else(|| CredentialError::RefreshFailed("no refresh token".into()))?;

    let response = reqwest::Client::new()
        .post(REFRESH_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token.as_str()),
        ])
        .send()
        .await
        .map_err(|error| CredentialError::RefreshFailed(error.to_string()))?;

    if !response.status().is_success() {
        return Err(CredentialError::RefreshFailed(format!(
            "HTTP {}",
            response.status().as_u16()
        )));
    }

    let payload: RefreshResponse = response
        .json()
        .await
        .map_err(|error| CredentialError::RefreshFailed(error.to_string()))?;
    let access_token = trim_to_configured(payload.access_token)
        .ok_or_else(|| CredentialError::RefreshFailed("missing access token".into()))?;
    let now = OffsetDateTime::now_utc();
    let expires_at = now
        + payload
            .expires_in
            .map(Duration::seconds)
            .unwrap_or(LEGACY_REFRESH_WINDOW);

    let tokens = auth_state.tokens.get_or_insert_with(AuthTokens::default);
    tokens.access_token = Some(access_token.clone());
    if let Some(refresh_token) = trim_to_configured(payload.refresh_token) {
        tokens.refresh_token = Some(refresh_token);
    }
    if let Some(id_token) = trim_to_configured(payload.id_token) {
        tokens.id_token = Some(id_token);
    }
    auth_state.last_refresh = Some(format_rfc3339(now)?);
    auth_state.expires_at = Some(StoredExpiry::Iso(format_rfc3339(expires_at)?));

    save_auth_state(auth_path, auth_state)?;
    Ok(access_token)
}
