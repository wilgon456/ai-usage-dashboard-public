use super::cache::{
    invalidate_keychain_cache, keychain_entry, prime_keychain_cache, read_keychain_cached,
};
use super::{Credential, CredentialError, CredentialSource, WritableCredentialSource, io_error};
use async_trait::async_trait;

const KEYCHAIN_SERVICE: &str = "ai-usage-dashboard-openrouter";

#[derive(Default)]
pub struct OpenRouterSource;

#[async_trait]
impl CredentialSource for OpenRouterSource {
    fn provider_id(&self) -> &'static str {
        "openrouter"
    }

    async fn load(&self) -> Result<Credential, CredentialError> {
        let value = std::env::var("OPENROUTER_API_KEY")
            .ok()
            .or_else(|| read_keychain_cached(KEYCHAIN_SERVICE))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .ok_or(CredentialError::NotConfigured)?;

        Ok(Credential::ApiKey(value))
    }
}

#[async_trait]
impl WritableCredentialSource for OpenRouterSource {
    async fn save(&self, raw: String) -> Result<(), CredentialError> {
        let value = raw.trim().to_string();
        if value.is_empty() {
            return Err(CredentialError::NotConfigured);
        }

        keychain_entry(KEYCHAIN_SERVICE)
            .map_err(io_error)?
            .set_password(&value)
            .map_err(io_error)?;
        prime_keychain_cache(KEYCHAIN_SERVICE, value);
        Ok(())
    }

    async fn clear(&self) -> Result<(), CredentialError> {
        let _ = keychain_entry(KEYCHAIN_SERVICE)
            .map_err(io_error)?
            .delete_credential();
        invalidate_keychain_cache(KEYCHAIN_SERVICE);
        Ok(())
    }
}
