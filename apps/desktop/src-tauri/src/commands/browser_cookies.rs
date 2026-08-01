use aes::Aes128;
use cbc::cipher::{BlockDecryptMut, KeyIvInit, block_padding::Pkcs7};
use pbkdf2::pbkdf2_hmac;
use rusqlite::{Connection, OpenFlags, params};
use sha1::Sha1;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

type Aes128CbcDec = cbc::Decryptor<Aes128>;
static SAFE_STORAGE_PASSWORDS: OnceLock<Mutex<HashMap<String, String>>> = OnceLock::new();

#[derive(Clone, Debug)]
pub struct BrowserCookie {
    pub host: String,
    pub name: String,
    pub value: String,
}

struct BrowserSpec {
    root: PathBuf,
    safe_storage_service: &'static str,
    safe_storage_account: &'static str,
}

pub fn load_matching(domain_fragments: &[&str]) -> Result<Vec<BrowserCookie>, String> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = domain_fragments;
        return Err("Browser session import is currently available on macOS only.".to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let specs = browser_specs()?;
        let mut errors = Vec::new();

        for spec in specs {
            let databases = cookie_databases(&spec.root);
            for database in databases {
                match query_encrypted_cookies(&database, domain_fragments) {
                    Ok(rows) if rows.is_empty() => continue,
                    Ok(rows) => match safe_storage_password(
                        spec.safe_storage_service,
                        spec.safe_storage_account,
                    ) {
                        Ok(password) => match decrypt_rows(rows, &password) {
                            Ok(cookies) if !cookies.is_empty() => return Ok(cookies),
                            Ok(_) => errors.push(format!(
                                "{} has matching cookies, but none could be decrypted",
                                database.display()
                            )),
                            Err(error) => errors.push(error),
                        },
                        Err(error) => errors.push(error),
                    },
                    Err(error) => errors.push(error),
                }
            }
        }

        if errors.is_empty() {
            Err("No matching browser login session was found.".to_string())
        } else {
            Err(errors.join("; "))
        }
    }
}

pub fn header_for_host(cookies: &[BrowserCookie], request_host: &str) -> Option<String> {
    let request_host = request_host.trim_start_matches('.').to_ascii_lowercase();
    let mut selected = BTreeMap::<String, (usize, String)>::new();

    for cookie in cookies {
        let cookie_host = cookie.host.trim_start_matches('.').to_ascii_lowercase();
        if request_host == cookie_host || request_host.ends_with(&format!(".{cookie_host}")) {
            let specificity = cookie_host.len();
            let replace = selected
                .get(&cookie.name)
                .is_none_or(|(current, _)| specificity >= *current);
            if replace {
                selected.insert(cookie.name.clone(), (specificity, cookie.value.clone()));
            }
        }
    }

    (!selected.is_empty()).then(|| {
        selected
            .into_iter()
            .map(|(name, (_, value))| format!("{name}={value}"))
            .collect::<Vec<_>>()
            .join("; ")
    })
}

pub fn latest_history_url_containing(needle: &str) -> Option<String> {
    let needle = format!("%{needle}%");
    for spec in browser_specs().ok()? {
        let Ok(entries) = std::fs::read_dir(&spec.root) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name != "Default" && !name.starts_with("Profile ") {
                continue;
            }
            let history = entry.path().join("History");
            if !history.is_file() {
                continue;
            }
            let Ok(connection) = Connection::open_with_flags(
                history,
                OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
            ) else {
                continue;
            };
            let result = connection.query_row(
                "SELECT url FROM urls WHERE url LIKE ?1 ORDER BY last_visit_time DESC LIMIT 1",
                params![needle],
                |row| row.get::<_, String>(0),
            );
            if let Ok(url) = result {
                return Some(url);
            }
        }
    }
    None
}

fn browser_specs() -> Result<Vec<BrowserSpec>, String> {
    let home = dirs::home_dir().ok_or_else(|| "No home directory available".to_string())?;
    let support = home.join("Library/Application Support");
    Ok(vec![
        BrowserSpec {
            root: support.join("Microsoft Edge"),
            safe_storage_service: "Microsoft Edge Safe Storage",
            safe_storage_account: "Microsoft Edge",
        },
        BrowserSpec {
            root: support.join("Google/Chrome"),
            safe_storage_service: "Chrome Safe Storage",
            safe_storage_account: "Chrome",
        },
        BrowserSpec {
            root: support.join("BraveSoftware/Brave-Browser"),
            safe_storage_service: "Brave Safe Storage",
            safe_storage_account: "Brave",
        },
        BrowserSpec {
            root: support.join("Arc/User Data"),
            safe_storage_service: "Arc Safe Storage",
            safe_storage_account: "Arc",
        },
    ])
}

fn cookie_databases(root: &Path) -> Vec<PathBuf> {
    let mut databases = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return databases;
    };

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name != "Default" && !name.starts_with("Profile ") {
            continue;
        }
        for relative in ["Network/Cookies", "Cookies"] {
            let candidate = entry.path().join(relative);
            if candidate.is_file() {
                databases.push(candidate);
            }
        }
    }
    databases
}

struct EncryptedCookie {
    host: String,
    name: String,
    plaintext_value: String,
    encrypted_value: Vec<u8>,
}

fn query_encrypted_cookies(
    database: &Path,
    domain_fragments: &[&str],
) -> Result<Vec<EncryptedCookie>, String> {
    let connection = Connection::open_with_flags(
        database,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|error| format!("Could not open browser cookies: {error}"))?;

    let mut statement = connection
        .prepare(
            "SELECT host_key, name, value, encrypted_value
             FROM cookies
             WHERE expires_utc = 0 OR expires_utc > ?1
             ORDER BY last_access_utc DESC",
        )
        .map_err(|error| format!("Could not read browser cookies: {error}"))?;
    let now_chrome_micros = (time::OffsetDateTime::now_utc().unix_timestamp() + 11_644_473_600)
        .saturating_mul(1_000_000);
    let rows = statement
        .query_map(params![now_chrome_micros], |row| {
            Ok(EncryptedCookie {
                host: row.get(0)?,
                name: row.get(1)?,
                plaintext_value: row.get(2)?,
                encrypted_value: row.get(3)?,
            })
        })
        .map_err(|error| format!("Could not query browser cookies: {error}"))?;

    let fragments = domain_fragments
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let mut output = Vec::new();
    for row in rows.flatten() {
        let host = row.host.to_ascii_lowercase();
        if fragments.iter().any(|fragment| host.contains(fragment)) {
            output.push(row);
        }
    }
    Ok(output)
}

fn safe_storage_password(service: &str, account: &str) -> Result<String, String> {
    let cache = SAFE_STORAGE_PASSWORDS.get_or_init(|| Mutex::new(HashMap::new()));
    if let Some(password) = cache
        .lock()
        .map_err(|_| "Browser key cache is unavailable.".to_string())?
        .get(service)
        .cloned()
    {
        return Ok(password);
    }

    let entry = keyring::Entry::new(service, account)
        .map_err(|error| format!("Could not open browser keychain entry: {error}"))?;
    let password = entry
        .get_password()
        .map_err(|error| format!("Browser keychain access was not granted: {error}"))?;
    cache
        .lock()
        .map_err(|_| "Browser key cache is unavailable.".to_string())?
        .insert(service.to_string(), password.clone());
    Ok(password)
}

fn decrypt_rows(rows: Vec<EncryptedCookie>, password: &str) -> Result<Vec<BrowserCookie>, String> {
    let mut key = [0_u8; 16];
    pbkdf2_hmac::<Sha1>(password.as_bytes(), b"saltysalt", 1003, &mut key);
    let iv = [b' '; 16];
    let mut seen = HashSet::new();
    let mut output = Vec::new();

    for row in rows {
        let value = if !row.plaintext_value.is_empty() {
            Some(row.plaintext_value)
        } else {
            decrypt_cookie(&row.host, &row.encrypted_value, &key, &iv)
        };
        let Some(value) = value.filter(|value| !value.is_empty()) else {
            continue;
        };
        if seen.insert((row.host.clone(), row.name.clone())) {
            output.push(BrowserCookie {
                host: row.host,
                name: row.name,
                value,
            });
        }
    }

    Ok(output)
}

fn decrypt_cookie(host: &str, encrypted: &[u8], key: &[u8; 16], iv: &[u8; 16]) -> Option<String> {
    let ciphertext = encrypted
        .strip_prefix(b"v10")
        .or_else(|| encrypted.strip_prefix(b"v11"))?;
    let mut buffer = ciphertext.to_vec();
    let decrypted = Aes128CbcDec::new(key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buffer)
        .ok()?;

    let plaintext = if decrypted.len() >= 32 {
        let expected = Sha256::digest(host.as_bytes());
        if decrypted[..32] == expected[..] {
            &decrypted[32..]
        } else {
            decrypted
        }
    } else {
        decrypted
    };
    String::from_utf8(plaintext.to_vec()).ok()
}
