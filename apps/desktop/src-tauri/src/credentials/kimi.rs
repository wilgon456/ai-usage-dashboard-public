use super::{
    Credential, CredentialError, CredentialSource, from_unix_seconds, io_error, is_expired,
    trim_to_configured,
};
use async_trait::async_trait;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use time::{Duration, OffsetDateTime};

const KIMI_REFRESH_URL: &str = "https://auth.kimi.com/api/oauth/token";
const KIMI_CLIENT_ID: &str = "17e5f671-d194-4dfb-9706-5516cb48c098";
const KIMI_CLI_PLATFORM: &str = "kimi_cli";
const KIMI_CLI_VERSION: &str = "1.38.0";

#[derive(Debug, Clone, Deserialize, Serialize)]
struct KimiCredentialsFile {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_at: Option<StoredExpiry>,
    scope: Option<String>,
    token_type: Option<String>,
    expires_in: Option<StoredExpiry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
enum StoredExpiry {
    Integer(i64),
    Float(f64),
}

impl StoredExpiry {
    fn to_offset_datetime(&self) -> Option<OffsetDateTime> {
        match self {
            StoredExpiry::Integer(value) => from_unix_seconds(*value),
            StoredExpiry::Float(value) => from_unix_seconds(*value as i64),
        }
    }
}

#[derive(Default)]
pub struct KimiSource {
    inner: tokio::sync::Mutex<()>,
}

#[derive(Debug, Deserialize)]
struct KimiRefreshResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
    token_type: Option<String>,
}

#[async_trait]
impl CredentialSource for KimiSource {
    fn provider_id(&self) -> &'static str {
        "kimi"
    }

    async fn load(&self) -> Result<Credential, CredentialError> {
        let _guard = self.inner.lock().await;
        let path = kimi_credentials_path().ok_or(CredentialError::NotConfigured)?;
        let raw = fs::read_to_string(&path).map_err(io_error)?;
        let mut parsed: KimiCredentialsFile = serde_json::from_str(&raw).map_err(io_error)?;
        let mut access_token =
            trim_to_configured(parsed.access_token).ok_or(CredentialError::NotConfigured)?;
        let mut refresh_token = trim_to_configured(parsed.refresh_token);
        let mut expires_at = parsed
            .expires_at
            .as_ref()
            .and_then(StoredExpiry::to_offset_datetime);

        if expires_at.as_ref().is_none_or(is_expired) {
            let token = refresh_token
                .clone()
                .ok_or_else(|| CredentialError::RefreshFailed("no refresh token".into()))?;
            let refreshed = match refresh_access_token(&token).await {
                Ok(refreshed) => refreshed,
                Err(_) => {
                    return Ok(Credential::OAuth {
                        access_token,
                        refresh_token,
                        expires_at,
                    });
                }
            };
            let expires_in = refreshed.expires_in.unwrap_or(900);
            let refreshed_expires_at = OffsetDateTime::now_utc() + Duration::seconds(expires_in);

            access_token = trim_to_configured(refreshed.access_token)
                .ok_or_else(|| CredentialError::RefreshFailed("missing access token".into()))?;
            if let Some(new_refresh_token) = trim_to_configured(refreshed.refresh_token) {
                refresh_token = Some(new_refresh_token);
            }
            expires_at = Some(refreshed_expires_at);

            parsed = KimiCredentialsFile {
                access_token: Some(access_token.clone()),
                refresh_token: refresh_token.clone(),
                expires_at: Some(StoredExpiry::Float(
                    refreshed_expires_at.unix_timestamp() as f64
                )),
                scope: trim_to_configured(refreshed.scope),
                token_type: trim_to_configured(refreshed.token_type)
                    .or_else(|| Some("Bearer".to_string())),
                expires_in: Some(StoredExpiry::Integer(expires_in)),
            };
            persist_credentials(&path, &parsed)?;
        }

        Ok(Credential::OAuth {
            access_token,
            refresh_token,
            expires_at,
        })
    }
}

async fn refresh_access_token(refresh_token: &str) -> Result<KimiRefreshResponse, CredentialError> {
    let response = reqwest::Client::new()
        .post(KIMI_REFRESH_URL)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .header("X-Msh-Platform", KIMI_CLI_PLATFORM)
        .header("X-Msh-Version", KIMI_CLI_VERSION)
        .header("X-Msh-Device-Name", kimi_device_name())
        .header("X-Msh-Device-Model", kimi_device_model())
        .header("X-Msh-Os-Version", std::env::consts::OS)
        .header("X-Msh-Device-Id", read_device_id().unwrap_or_default())
        .form(&[
            ("client_id", KIMI_CLIENT_ID),
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await
        .map_err(|error| CredentialError::RefreshFailed(error.to_string()))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED
        || response.status() == reqwest::StatusCode::FORBIDDEN
    {
        return Err(CredentialError::RefreshFailed("unauthorized".into()));
    }

    if !response.status().is_success() {
        return Err(CredentialError::RefreshFailed(format!(
            "HTTP {}",
            response.status().as_u16()
        )));
    }

    response
        .json()
        .await
        .map_err(|error| CredentialError::RefreshFailed(error.to_string()))
}

fn persist_credentials(
    path: &Path,
    credentials: &KimiCredentialsFile,
) -> Result<(), CredentialError> {
    let content = serde_json::to_string_pretty(credentials).map_err(io_error)?;
    write_atomic(path, &content)
}

fn write_atomic(path: &Path, content: &str) -> Result<(), CredentialError> {
    let parent = path.parent().ok_or_else(|| {
        CredentialError::Io("Kimi credential path missing parent directory".into())
    })?;
    fs::create_dir_all(parent).map_err(io_error)?;

    let mut temp = NamedTempFile::new_in(parent).map_err(io_error)?;
    temp.write_all(content.as_bytes()).map_err(io_error)?;
    temp.flush().map_err(io_error)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        temp.as_file()
            .set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(io_error)?;
    }
    temp.persist(path).map_err(|error| io_error(error.error))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(io_error)?;
    }
    Ok(())
}

fn read_device_id() -> Option<String> {
    let path = kimi_config_dir()?.join("device_id");
    trim_to_configured(fs::read_to_string(path).ok())
}

fn kimi_device_model() -> String {
    std::env::consts::OS.to_string()
}

fn kimi_device_name() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .and_then(|value| trim_to_configured(Some(value)))
        .or_else(|| {
            std::env::var("COMPUTERNAME")
                .ok()
                .and_then(|value| trim_to_configured(Some(value)))
        })
        .unwrap_or_default()
}

fn kimi_credentials_path() -> Option<PathBuf> {
    Some(
        kimi_config_dir()?
            .join("credentials")
            .join("kimi-code.json"),
    )
}

fn kimi_config_dir() -> Option<PathBuf> {
    if let Ok(override_dir) = std::env::var("KIMI_CONFIG_DIR") {
        let trimmed = override_dir.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }

    home_dir().map(|home| home.join(".kimi"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_kimi_cli_credentials_with_float_expires_in() {
        let raw = r#"{
            "access_token": "access",
            "refresh_token": "refresh",
            "expires_at": 1777018585.8894231,
            "scope": "kimi-code",
            "token_type": "Bearer",
            "expires_in": 900.0
        }"#;

        let parsed: KimiCredentialsFile = serde_json::from_str(raw).expect("credentials parse");
        assert_eq!(parsed.access_token.as_deref(), Some("access"));
        assert!(parsed
            .expires_at
            .as_ref()
            .and_then(StoredExpiry::to_offset_datetime)
            .is_some());
        assert!(matches!(parsed.expires_in, Some(StoredExpiry::Float(value)) if value == 900.0));
    }
}
