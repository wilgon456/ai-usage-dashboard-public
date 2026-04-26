pub mod cache;
pub mod claude;
pub mod codex;
pub mod copilot;
pub mod kimi;
pub mod openrouter;

use async_trait::async_trait;
use std::fmt::Display;
use std::sync::Arc;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum Credential {
    OAuth {
        access_token: String,
        refresh_token: Option<String>,
        expires_at: Option<OffsetDateTime>,
    },
    ApiKey(String),
}

#[derive(Debug, thiserror::Error)]
pub enum CredentialError {
    #[error("credential not configured")]
    NotConfigured,
    #[error("credential expired and refresh failed: {0}")]
    RefreshFailed(String),
    #[error("credential source error: {0}")]
    Io(String),
}

#[async_trait]
pub trait CredentialSource: Send + Sync {
    fn provider_id(&self) -> &'static str;

    async fn load(&self) -> Result<Credential, CredentialError>;
}

#[async_trait]
pub trait WritableCredentialSource: CredentialSource {
    async fn save(&self, raw: String) -> Result<(), CredentialError>;
    async fn clear(&self) -> Result<(), CredentialError>;
}

pub struct CredentialRegistry {
    codex: Arc<codex::CodexSource>,
    claude: Arc<claude::ClaudeSource>,
    sources: Vec<Arc<dyn CredentialSource>>,
    writable_sources: Vec<Arc<dyn WritableCredentialSource>>,
}

impl CredentialRegistry {
    pub fn new() -> Self {
        let codex = Arc::new(codex::CodexSource::default());
        let claude = Arc::new(claude::ClaudeSource::default());
        let copilot = Arc::new(copilot::CopilotSource);
        let kimi = Arc::new(kimi::KimiSource::default());
        let openrouter = Arc::new(openrouter::OpenRouterSource);

        Self {
            codex: Arc::clone(&codex),
            claude: Arc::clone(&claude),
            sources: vec![codex, claude, copilot, openrouter.clone(), kimi.clone()],
            writable_sources: vec![openrouter],
        }
    }

    pub fn get(&self, provider_id: &str) -> Option<Arc<dyn CredentialSource>> {
        self.sources
            .iter()
            .find(|source| source.provider_id() == provider_id)
            .cloned()
    }

    pub fn get_writable(&self, provider_id: &str) -> Option<Arc<dyn WritableCredentialSource>> {
        self.writable_sources
            .iter()
            .find(|source| source.provider_id() == provider_id)
            .cloned()
    }

    pub fn codex(&self) -> Arc<codex::CodexSource> {
        Arc::clone(&self.codex)
    }

    pub fn claude(&self) -> Arc<claude::ClaudeSource> {
        Arc::clone(&self.claude)
    }
}

impl Default for CredentialRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn is_expired(expires_at: &OffsetDateTime) -> bool {
    *expires_at <= OffsetDateTime::now_utc() + Duration::seconds(60)
}

pub(crate) fn trim_to_configured(raw: Option<String>) -> Option<String> {
    raw.map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(crate) fn parse_rfc3339(value: &str) -> Option<OffsetDateTime> {
    OffsetDateTime::parse(value, &Rfc3339).ok()
}

pub(crate) fn format_rfc3339(value: OffsetDateTime) -> Result<String, CredentialError> {
    value
        .format(&Rfc3339)
        .map_err(|error| CredentialError::Io(error.to_string()))
}

pub(crate) fn from_unix_seconds(seconds: i64) -> Option<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp(seconds).ok()
}

pub(crate) fn from_unix_millis(millis: i64) -> Option<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp_nanos((millis as i128) * 1_000_000).ok()
}

pub(crate) fn io_error(error: impl Display) -> CredentialError {
    CredentialError::Io(error.to_string())
}
