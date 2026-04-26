use super::cache::read_keychain_cached;
use super::{Credential, CredentialError, CredentialSource, trim_to_configured};
use async_trait::async_trait;
use base64::Engine;
use dirs::{config_dir, home_dir};
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

const COPILOT_KEYCHAIN_SERVICE: &str = "github-copilot";
const GH_KEYCHAIN_SERVICES: [&str; 2] = ["github-cli:github.com", "gh:github.com"];

#[derive(Default)]
pub struct CopilotSource;

#[async_trait]
impl CredentialSource for CopilotSource {
    fn provider_id(&self) -> &'static str {
        "copilot"
    }

    async fn load(&self) -> Result<Credential, CredentialError> {
        let token = load_token().ok_or(CredentialError::NotConfigured)?;
        Ok(Credential::OAuth {
            access_token: token,
            refresh_token: None,
            expires_at: None,
        })
    }
}

fn load_token() -> Option<String> {
    load_gh_auth_token_command()
        .or_else(load_gh_cli_token)
        .or_else(load_copilot_keychain_token)
        .or_else(load_token_from_config_files)
}

fn load_copilot_keychain_token() -> Option<String> {
    let raw = read_keychain_cached(COPILOT_KEYCHAIN_SERVICE)?;
    parse_token_object(&raw)
}

fn load_gh_cli_token() -> Option<String> {
    GH_KEYCHAIN_SERVICES
        .iter()
        .find_map(|service| read_keychain_cached(service))
        .and_then(|raw| {
            if let Some(decoded) = maybe_decode_base64(&raw)
                && let Some(token) = parse_token_text(&decoded)
            {
                return Some(token);
            }
            parse_token_text(&raw)
        })
}

fn load_token_from_config_files() -> Option<String> {
    copilot_config_paths().into_iter().find_map(|path| {
        let raw = fs::read_to_string(path).ok()?;
        if let Ok(json) = serde_json::from_str::<Value>(&raw)
            && let Some(token) = find_token_in_json(&json)
        {
            return Some(token);
        }
        parse_token_text(&raw)
    })
}

fn load_gh_auth_token_command() -> Option<String> {
    let output = Command::new("gh").args(["auth", "token"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8(output.stdout).ok()?;
    parse_token_text(&stdout)
}

fn copilot_config_paths() -> Vec<PathBuf> {
    let root = config_dir()
        .or_else(|| home_dir().map(|home| home.join(".config")))
        .map(|path| path.join("github-copilot"));

    let Some(root) = root else {
        return Vec::new();
    };

    vec![root.join("apps.json"), root.join("hosts.json")]
}

fn find_token_in_json(value: &Value) -> Option<String> {
    match value {
        Value::Object(map) => {
            for key in [
                "oauth_token",
                "oauthToken",
                "token",
                "access_token",
                "accessToken",
            ] {
                if let Some(token) = map
                    .get(key)
                    .and_then(Value::as_str)
                    .and_then(parse_token_text)
                {
                    return Some(token);
                }
            }

            map.values().find_map(find_token_in_json)
        }
        Value::Array(items) => items.iter().find_map(find_token_in_json),
        Value::String(value) => parse_token_text(value),
        _ => None,
    }
}

fn parse_token_object(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
        if let Some(token) = json.get("token").and_then(Value::as_str) {
            return trim_to_configured(Some(token.to_string()));
        }
        if let Some(token) = json.get("oauth_token").and_then(Value::as_str) {
            return trim_to_configured(Some(token.to_string()));
        }
    }

    parse_token_text(trimmed)
}

fn maybe_decode_base64(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let candidate = trimmed
        .strip_prefix("go-keyring-base64:")
        .unwrap_or(trimmed);
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(candidate)
        .ok()?;
    String::from_utf8(decoded).ok()
}

fn parse_token_text(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.is_empty() {
        return None;
    }

    if let Ok(json) = serde_json::from_str::<Value>(trimmed) {
        if let Some(token) = json.get("oauth_token").and_then(Value::as_str) {
            return trim_to_configured(Some(token.to_string()));
        }
        if let Some(token) = json.get("token").and_then(Value::as_str) {
            return trim_to_configured(Some(token.to_string()));
        }
    }

    for line in trimmed.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("oauth_token:") {
            return trim_to_configured(Some(
                value.trim().trim_matches('"').trim_matches('\'').into(),
            ));
        }
    }

    if is_github_token(trimmed) {
        return Some(trimmed.to_string());
    }

    None
}

fn is_github_token(token: &str) -> bool {
    ["ghu_", "gho_", "ghp_", "ghs_", "github_pat_"]
        .iter()
        .any(|prefix| token.starts_with(prefix))
}
