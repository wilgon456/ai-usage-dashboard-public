use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

pub fn keychain_entry(service: &str) -> Result<keyring::Entry, String> {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "default".into());
    // keyring v3 routes to Security.framework (macOS) or Credential Manager (Windows) via target-native features.
    keyring::Entry::new(service, &user).map_err(|error| error.to_string())
}

// Cached keychain reads. Unsigned macOS binaries prompt the user on every
// Security.framework access, so we only hit the keychain the first time per
// service and re-use the result until save/clear invalidates it.
#[derive(Clone)]
enum KeychainCacheEntry {
    Hit(String),
    Miss,
}

static KEYCHAIN_CACHE: LazyLock<Mutex<HashMap<String, KeychainCacheEntry>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn read_keychain_cached(service: &str) -> Option<String> {
    if let Ok(guard) = KEYCHAIN_CACHE.lock()
        && let Some(entry) = guard.get(service)
    {
        return match entry {
            KeychainCacheEntry::Hit(value) => Some(value.clone()),
            KeychainCacheEntry::Miss => None,
        };
    }

    let resolved = keychain_entry(service)
        .and_then(|entry| entry.get_password().map_err(|error| error.to_string()))
        .ok();

    if let Ok(mut guard) = KEYCHAIN_CACHE.lock() {
        guard.insert(
            service.to_string(),
            match resolved.clone() {
                Some(value) => KeychainCacheEntry::Hit(value),
                None => KeychainCacheEntry::Miss,
            },
        );
    }
    resolved
}

pub fn prime_keychain_cache(service: &str, value: String) {
    if let Ok(mut guard) = KEYCHAIN_CACHE.lock() {
        guard.insert(service.to_string(), KeychainCacheEntry::Hit(value));
    }
}

pub fn invalidate_keychain_cache(service: &str) {
    if let Ok(mut guard) = KEYCHAIN_CACHE.lock() {
        guard.remove(service);
    }
}
