use serde::Deserialize;
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::thread;

pub const WIDGET_SYNC_PORT: u16 = 18790;

#[derive(Default)]
pub struct WidgetSyncState {
    enabled: bool,
    token: String,
    snapshot: Option<Value>,
}

pub type WidgetSyncStore = Arc<Mutex<WidgetSyncState>>;

#[derive(Deserialize)]
pub struct WidgetSyncConfig {
    enabled: bool,
    token: String,
}

pub fn new_store() -> WidgetSyncStore {
    Arc::new(Mutex::new(WidgetSyncState {
        enabled: std::env::var("VITE_WIDGET_SYNC_TOKEN")
            .map(|token| !token.trim().is_empty())
            .unwrap_or(false),
        token: std::env::var("VITE_WIDGET_SYNC_TOKEN").unwrap_or_default(),
        snapshot: None,
    }))
}

pub fn start_server(store: WidgetSyncStore) {
    thread::spawn(move || {
        let Ok(listener) = TcpListener::bind(("0.0.0.0", WIDGET_SYNC_PORT)) else {
            return;
        };

        for stream in listener.incoming().flatten() {
            handle_client(stream, Arc::clone(&store));
        }
    });
}

#[tauri::command]
pub fn set_widget_sync_config(
    config: WidgetSyncConfig,
    store: tauri::State<'_, WidgetSyncStore>,
) -> Result<(), String> {
    let mut state = store.lock().map_err(|_| "Widget sync state lock failed.")?;
    state.enabled = config.enabled;
    state.token = config.token;
    Ok(())
}

#[tauri::command]
pub fn update_widget_snapshot(
    snapshot: Value,
    store: tauri::State<'_, WidgetSyncStore>,
) -> Result<(), String> {
    let mut state = store.lock().map_err(|_| "Widget sync state lock failed.")?;
    state.snapshot = Some(snapshot);
    Ok(())
}

#[tauri::command]
pub fn get_widget_sync_urls(token: String) -> Vec<String> {
    let token = url_encode_component(token.trim());
    if token.is_empty() {
        return Vec::new();
    }

    let mut urls = Vec::new();
    if let Some(ip) = primary_lan_ip() {
        urls.push(format!(
            "http://{ip}:{WIDGET_SYNC_PORT}/widget-snapshot?token={token}"
        ));
    }
    urls.push(format!(
        "http://127.0.0.1:{WIDGET_SYNC_PORT}/widget-snapshot?token={token}"
    ));
    urls.push(format!(
        "http://10.0.2.2:{WIDGET_SYNC_PORT}/widget-snapshot?token={token}"
    ));
    urls
}

fn url_encode_component(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

fn primary_lan_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let ip = socket.local_addr().ok()?.ip();
    if ip.is_loopback() {
        None
    } else {
        Some(ip.to_string())
    }
}

fn handle_client(mut stream: TcpStream, store: WidgetSyncStore) {
    let mut buffer = [0; 2048];
    let Ok(bytes_read) = stream.read(&mut buffer) else {
        return;
    };
    let request = String::from_utf8_lossy(&buffer[..bytes_read]);
    let first_line = request.lines().next().unwrap_or_default();

    if !first_line.starts_with("GET /widget-snapshot") {
        let _ = write_response(&mut stream, 404, "text/plain", "Not found");
        return;
    }

    let token = extract_token(first_line);
    let Ok(state) = store.lock() else {
        let _ = write_response(&mut stream, 500, "text/plain", "State unavailable");
        return;
    };

    if !state.enabled {
        let _ = write_response(&mut stream, 403, "text/plain", "Widget sync disabled");
        return;
    }

    if state.token.is_empty() || token.as_deref() != Some(state.token.as_str()) {
        let _ = write_response(&mut stream, 401, "text/plain", "Invalid token");
        return;
    }

    let body = state
        .snapshot
        .as_ref()
        .map(Value::to_string)
        .unwrap_or_else(|| "{\"schemaVersion\":1,\"providers\":[]}".to_string());
    let _ = write_response(&mut stream, 200, "application/json", &body);
}

fn extract_token(first_line: &str) -> Option<String> {
    let path = first_line.split_whitespace().nth(1)?;
    let query = path.split_once('?')?.1;
    query.split('&').find_map(|pair| {
        let (key, value) = pair.split_once('=')?;
        if key == "token" {
            url_decode_component(value).or_else(|| Some(value.to_string()))
        } else {
            None
        }
    })
}

fn url_decode_component(value: &str) -> Option<String> {
    let mut bytes = Vec::with_capacity(value.len());
    let raw = value.as_bytes();
    let mut index = 0;
    while index < raw.len() {
        match raw[index] {
            b'%' if index + 2 < raw.len() => {
                let high = hex_value(raw[index + 1])?;
                let low = hex_value(raw[index + 2])?;
                bytes.push((high << 4) | low);
                index += 3;
            }
            b'+' => {
                bytes.push(b' ');
                index += 1;
            }
            byte => {
                bytes.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(bytes).ok()
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn write_response(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &str,
) -> std::io::Result<()> {
    let reason = match status {
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widget_sync_urls_include_the_real_token() {
        let urls = get_widget_sync_urls("abc123".to_string());

        assert!(!urls.is_empty());
        assert!(urls.iter().all(|url| url.contains("token=abc123")));
        assert!(urls.iter().all(|url| !url.contains("token=***")));
    }

    #[test]
    fn widget_sync_urls_are_empty_without_a_token() {
        assert!(get_widget_sync_urls("   ".to_string()).is_empty());
    }

    #[test]
    fn widget_sync_urls_encode_token_for_query_string() {
        let urls = get_widget_sync_urls("abc 123".to_string());

        assert!(urls.iter().all(|url| url.contains("token=abc%20123")));
    }

    #[test]
    fn extract_token_decodes_url_encoded_values() {
        assert_eq!(
            extract_token("GET /widget-snapshot?token=abc%20123 HTTP/1.1").as_deref(),
            Some("abc 123")
        );
    }

    #[test]
    fn extract_token_falls_back_to_raw_invalid_encoding() {
        assert_eq!(
            extract_token("GET /widget-snapshot?token=abc%XX HTTP/1.1").as_deref(),
            Some("abc%XX")
        );
    }
}
