use super::browser_cookies;
use super::shared::{self, MetricFormat, MetricLinePayload, UsagePayload};
use regex::Regex;
use reqwest::Client;
use serde_json::{Map, Value, json};
use std::time::Duration;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/143.0.0.0 Safari/537.36";
const OPENCODE_WORKSPACES_SERVER_ID: &str =
    "def39973159c7f0483d8793a822b8dbb10d067e12c65455fcb4608459ba0234f";

#[derive(Debug)]
struct UsageWindow {
    percent: f64,
    resets_at: Option<String>,
}

#[tauri::command]
pub async fn get_opencode_go_usage(
    _refresh_interval_minutes: u32,
    _force: bool,
) -> Result<UsagePayload, String> {
    let cookies = browser_cookies::load_matching(&["opencode.ai"])?;
    let cookie_header =
        browser_cookies::header_for_host(&cookies, "opencode.ai").ok_or_else(|| {
            "OpenCode browser login was not found. Log in at opencode.ai.".to_string()
        })?;
    if !cookies
        .iter()
        .any(|cookie| cookie.name == "auth" || cookie.name == "__Host-auth")
    {
        return Err("OpenCode browser login was not found. Log in at opencode.ai.".to_string());
    }

    let client = http_client()?;
    let workspace_id = fetch_opencode_workspace(&client, &cookie_header).await?;
    let url = format!("https://opencode.ai/workspace/{workspace_id}/go");
    let response = client
        .get(url)
        .header("Cookie", &cookie_header)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        .map_err(|error| format!("OpenCode Go dashboard request failed: {error}"))?;
    let status = response.status();
    let final_url = response.url().clone();
    let text = response
        .text()
        .await
        .map_err(|error| format!("OpenCode Go response could not be read: {error}"))?;
    if !status.is_success() || final_url.path().contains("auth") || looks_signed_out(&text) {
        return Err("OpenCode browser session expired. Log in again at opencode.ai.".to_string());
    }

    let mut rolling = parse_named_window(&text, "rollingUsage")
        .or_else(|| parse_html_window(&text, &["5h", "5 hour", "rolling"]))
        .ok_or_else(|| "OpenCode Go did not return a 5-hour usage window.".to_string())?;
    let weekly = parse_named_window(&text, "weeklyUsage")
        .or_else(|| parse_html_window(&text, &["week", "7 day"]));
    let monthly = parse_named_window(&text, "monthlyUsage")
        .or_else(|| parse_html_window(&text, &["month", "30 day"]));
    if let Some(monthly) = monthly.as_ref() {
        apply_monthly_cap(&mut rolling, monthly);
    }
    let mut lines = vec![progress_line("5h", rolling)];
    if let Some(window) = weekly {
        lines.push(progress_line("Weekly", window));
    }
    if let Some(window) = monthly {
        lines.push(progress_line("Monthly", window));
    }
    lines.push(MetricLinePayload::Badge {
        label: "Source".to_string(),
        value: "OpenCode Go dashboard".to_string(),
        tone: Some("good".to_string()),
    });

    Ok(UsagePayload {
        provider_id: "opencode-go",
        plan: Some("OpenCode Go".to_string()),
        lines,
        source: "remote",
    })
}

#[tauri::command]
pub async fn get_alibaba_token_plan_usage(
    _refresh_interval_minutes: u32,
    _force: bool,
) -> Result<UsagePayload, String> {
    let cookies = browser_cookies::load_matching(&["aliyun", "alibabacloud"])?;
    let client = http_client()?;
    let account_trace_id = browser_cookies::latest_history_url_containing(
        "modelstudio.console.alibabacloud.com%accounttraceid=",
    )
    .and_then(|url| extract_query_value(&url, "accounttraceid"));

    let variants = [AlibabaRegion::International, AlibabaRegion::China];
    let mut errors = Vec::new();
    for region in variants {
        let Some(cookie_header) = browser_cookies::header_for_host(&cookies, region.quota_host())
        else {
            errors.push(format!("{} browser login was not found", region.label()));
            continue;
        };
        match fetch_alibaba_personal(&client, &cookie_header, region, account_trace_id.as_deref())
            .await
        {
            Ok((five_hour, weekly)) => {
                let mut lines = Vec::new();
                if let Some(window) = five_hour {
                    lines.push(progress_line("5h", window));
                }
                if let Some(window) = weekly {
                    lines.push(progress_line("Weekly", window));
                }
                if lines.is_empty() {
                    errors.push(format!("{} returned no usage windows", region.label()));
                    continue;
                }
                lines.push(MetricLinePayload::Badge {
                    label: "Source".to_string(),
                    value: format!("Alibaba Personal · {}", region.label()),
                    tone: Some("good".to_string()),
                });
                return Ok(UsagePayload {
                    provider_id: "alibaba-token-plan",
                    plan: Some("Alibaba Token Plan Personal".to_string()),
                    lines,
                    source: "remote",
                });
            }
            Err(error) => errors.push(format!("{} Personal: {error}", region.label())),
        }
    }

    for region in variants {
        let Some(cookie_header) = browser_cookies::header_for_host(&cookies, region.quota_host())
        else {
            continue;
        };
        match fetch_alibaba_coding_plan(&client, &cookie_header, region).await {
            Ok((five_hour, weekly)) => {
                let mut lines = Vec::new();
                if let Some(window) = five_hour {
                    lines.push(progress_line("5h", window));
                }
                if let Some(window) = weekly {
                    lines.push(progress_line("Weekly", window));
                }
                if !lines.is_empty() {
                    lines.push(MetricLinePayload::Badge {
                        label: "Source".to_string(),
                        value: format!("Alibaba Coding Plan · {}", region.label()),
                        tone: Some("good".to_string()),
                    });
                    return Ok(UsagePayload {
                        provider_id: "alibaba-token-plan",
                        plan: Some("Alibaba Coding Plan".to_string()),
                        lines,
                        source: "remote",
                    });
                }
            }
            Err(error) => errors.push(format!("{} Coding Plan: {error}", region.label())),
        }
    }

    let message = format!(
        "Alibaba Token Plan usage could not be loaded. {}",
        errors.join("; ")
    );
    Err(message)
}

fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(12))
        .user_agent(USER_AGENT)
        .build()
        .map_err(|error| format!("Could not create usage client: {error}"))
}

async fn fetch_opencode_workspace(client: &Client, cookie_header: &str) -> Result<String, String> {
    let url = format!("https://opencode.ai/_server?id={OPENCODE_WORKSPACES_SERVER_ID}");
    for method in ["GET", "POST"] {
        let mut request = if method == "GET" {
            client.get(&url)
        } else {
            client.post("https://opencode.ai/_server").body("[]")
        };
        request = request
            .header("Cookie", cookie_header)
            .header("X-Server-Id", OPENCODE_WORKSPACES_SERVER_ID)
            .header("X-Server-Instance", format!("server-fn:{}", Uuid::new_v4()))
            .header("Origin", "https://opencode.ai")
            .header("Referer", "https://opencode.ai/")
            .header(
                "Accept",
                "text/javascript, application/json;q=0.9, */*;q=0.8",
            );
        if method == "POST" {
            request = request.header("Content-Type", "application/json");
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("OpenCode workspace request failed: {error}"))?;
        let status = response.status();
        let text = response
            .text()
            .await
            .map_err(|error| format!("OpenCode workspace response failed: {error}"))?;
        if status.is_success() {
            if looks_signed_out(&text) {
                return Err(
                    "OpenCode browser session expired. Log in again at opencode.ai.".to_string(),
                );
            }
            if let Some(id) = Regex::new(r"wrk_[A-Za-z0-9]+")
                .ok()
                .and_then(|regex| regex.find(&text))
                .map(|found| found.as_str().to_string())
            {
                return Ok(id);
            }
        }
    }
    Err("OpenCode workspace could not be discovered from the logged-in session.".to_string())
}

fn looks_signed_out(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("auth/authorize")
        || lower.contains("not associated with an account")
        || lower.contains("actor of type \"public\"")
}

fn parse_named_window(text: &str, name: &str) -> Option<UsageWindow> {
    let name = regex::escape(name);
    let object_pattern = format!(
        r#"(?s){name}["']?\s*[:=]\s*(?:\$R\[\d+\]\s*=\s*)?\{{(?P<body>[^{{}}]{{0,900}})\}}"#
    );
    let object_regex = Regex::new(&object_pattern).ok()?;
    let captures = object_regex.captures(text)?;
    let block = captures.name("body")?.as_str();
    let percent = capture_number(
        block,
        r#"(?:usagePercent|usedPercent|percentUsed|percentage)[\"']?\s*[:=]\s*[\"']?([0-9]+(?:\.[0-9]+)?)"#,
    )?;
    let reset_seconds = capture_number(
        block,
        r#"(?:resetInSec|resetInSeconds|resetSeconds)[\"']?\s*[:=]\s*[\"']?([0-9]+(?:\.[0-9]+)?)"#,
    );
    let reset_epoch = capture_number(
        block,
        r#"(?:resetsAt|resetAt|reset_at)[\"']?\s*[:=]\s*[\"']?([0-9]{10,16})"#,
    );
    Some(UsageWindow {
        percent: normalize_percent(percent),
        resets_at: reset_seconds
            .and_then(reset_after_seconds)
            .or_else(|| reset_epoch.and_then(epoch_to_rfc3339)),
    })
}

fn apply_monthly_cap(rolling: &mut UsageWindow, monthly: &UsageWindow) {
    if monthly.percent >= 100.0 {
        rolling.percent = 100.0;
        rolling.resets_at.clone_from(&monthly.resets_at);
    }
}

fn parse_html_window(text: &str, labels: &[&str]) -> Option<UsageWindow> {
    let plain = Regex::new(r"<[^>]+>").ok()?.replace_all(text, " ");
    let plain = plain.replace("&nbsp;", " ").replace("&percnt;", "%");
    for label in labels {
        let pattern = format!(
            r"(?is){}.{{0,220}}?([0-9]+(?:\.[0-9]+)?)\s*%",
            regex::escape(label)
        );
        if let Some(percent) = capture_number(&plain, &pattern) {
            return Some(UsageWindow {
                percent: normalize_percent(percent),
                resets_at: None,
            });
        }
    }
    None
}

fn capture_number(text: &str, pattern: &str) -> Option<f64> {
    let captures = Regex::new(pattern).ok()?.captures(text)?;
    captures.get(1)?.as_str().parse().ok()
}

fn normalize_percent(value: f64) -> f64 {
    let points = if value <= 1.0 { value * 100.0 } else { value };
    points.clamp(0.0, 100.0)
}

fn reset_after_seconds(seconds: f64) -> Option<String> {
    let now = OffsetDateTime::now_utc();
    shared::to_rfc3339(now + time::Duration::seconds(seconds.max(0.0) as i64))
}

fn epoch_to_rfc3339(value: f64) -> Option<String> {
    let seconds = if value > 1_000_000_000_000.0 {
        value / 1000.0
    } else {
        value
    };
    shared::unix_seconds_to_rfc3339(seconds as i64)
}

fn progress_line(label: &str, window: UsageWindow) -> MetricLinePayload {
    MetricLinePayload::Progress {
        label: label.to_string(),
        used: window.percent,
        limit: 100.0,
        format: MetricFormat::Percent,
        resets_at: window.resets_at,
        color: None,
    }
}

#[derive(Clone, Copy)]
enum AlibabaRegion {
    International,
    China,
}

impl AlibabaRegion {
    fn label(self) -> &'static str {
        match self {
            Self::International => "International",
            Self::China => "China",
        }
    }

    fn quota_host(self) -> &'static str {
        match self {
            Self::International => "bailian-singapore-cs.alibabacloud.com",
            Self::China => "bailian-cs.console.aliyun.com",
        }
    }

    fn dashboard(self) -> &'static str {
        match self {
            Self::International => {
                "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=plan#/efm/subscription/token-plan/personal"
            }
            Self::China => {
                "https://bailian.console.aliyun.com/cn-beijing?tab=plan#/efm/subscription/token-plan/personal"
            }
        }
    }

    fn coding_dashboard(self) -> &'static str {
        match self {
            Self::International => {
                "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=coding-plan#/efm/coding_plan"
            }
            Self::China => {
                "https://bailian.console.aliyun.com/cn-beijing/?tab=model#/efm/coding_plan"
            }
        }
    }

    fn coding_referer(self) -> &'static str {
        match self {
            Self::International => {
                "https://modelstudio.console.alibabacloud.com/ap-southeast-1/?tab=coding-plan"
            }
            Self::China => "https://bailian.console.aliyun.com/cn-beijing/?tab=model",
        }
    }

    fn origin(self) -> &'static str {
        match self {
            Self::International => "https://modelstudio.console.alibabacloud.com",
            Self::China => "https://bailian.console.aliyun.com",
        }
    }

    fn action(self) -> &'static str {
        match self {
            Self::International => "IntlBroadScopeAspnGateway",
            Self::China => "BroadScopeAspnGateway",
        }
    }

    fn region_id(self) -> &'static str {
        match self {
            Self::International => "ap-southeast-1",
            Self::China => "cn-beijing",
        }
    }

    fn console_site(self) -> &'static str {
        match self {
            Self::International => "MODELSTUDIO_ALBABACLOUD",
            Self::China => "BAILIAN_ALIYUN",
        }
    }

    fn coding_commodity(self) -> &'static str {
        match self {
            Self::International => "sfm_codingplan_public_intl",
            Self::China => "sfm_codingplan_public_cn",
        }
    }
}

async fn fetch_alibaba_coding_plan(
    client: &Client,
    cookie_header: &str,
    region: AlibabaRegion,
) -> Result<(Option<UsageWindow>, Option<UsageWindow>), String> {
    const QUOTA_API: &str = "zeldaEasy.broadscope-bailian.codingPlan.queryCodingPlanInstanceInfoV2";
    let sec_token =
        resolve_alibaba_sec_token(client, cookie_header, region, region.coding_dashboard()).await?;
    let mut url = reqwest::Url::parse(&format!("https://{}/data/api.json", region.quota_host()))
        .map_err(|error| format!("invalid Coding Plan URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("action", region.action())
        .append_pair("product", "sfm_bailian")
        .append_pair("api", QUOTA_API)
        .append_pair("_v", "undefined");

    let mut cornerstone = json!({
        "feTraceId": Uuid::new_v4().to_string(),
        "feURL": region.coding_dashboard(),
        "protocol": "V2",
        "console": "ONE_CONSOLE",
        "productCode": "p_efm",
        "domain": reqwest::Url::parse(region.coding_dashboard()).ok().and_then(|url| url.host_str().map(str::to_string)).unwrap_or_default(),
        "consoleSite": region.console_site(),
        "userNickName": "",
        "userPrincipalName": "",
        "xsp_lang": "en-US"
    });
    if let Some(cna) = cookie_value(cookie_header, "cna") {
        cornerstone["X-Anonymous-Id"] = Value::String(cna.to_string());
    }
    let params_json = json!({
        "Api": QUOTA_API,
        "V": "1.0",
        "Data": {
            "queryCodingPlanInstanceInfoRequest": {
                "commodityCode": region.coding_commodity(),
                "onlyLatestOne": true
            },
            "cornerstoneParam": cornerstone
        }
    })
    .to_string();
    let form = [
        ("params", params_json),
        ("region", region.region_id().to_string()),
        ("sec_token", sec_token),
    ];
    let mut request = client
        .post(url)
        .header("Cookie", cookie_header)
        .header("Accept", "*/*")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Origin", region.origin())
        .header("Referer", region.coding_referer())
        .form(&form);
    if let Some(csrf) = cookie_value(cookie_header, "login_aliyunid_csrf")
        .or_else(|| cookie_value(cookie_header, "csrf"))
    {
        request = request
            .header("x-xsrf-token", csrf)
            .header("x-csrf-token", csrf);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("request failed: {error}"))?;
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|error| format!("response could not be read: {error}"))?;
    if !status.is_success() {
        return Err(format!("HTTP {}", status.as_u16()));
    }
    parse_alibaba_coding_windows(&body)
}

async fn resolve_alibaba_sec_token(
    client: &Client,
    cookie_header: &str,
    region: AlibabaRegion,
    dashboard_url: &str,
) -> Result<String, String> {
    if let Some(value) = cookie_value(cookie_header, "sec_token") {
        return Ok(value.to_string());
    }

    if let Ok(response) = client
        .get(dashboard_url)
        .header("Cookie", cookie_header)
        .header(
            "Accept",
            "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        )
        .send()
        .await
        && response.status().is_success()
        && let Ok(html) = response.text().await
        && let Some(token) = extract_sec_token(&html)
    {
        return Ok(token);
    }

    let user_info_url = format!("{}/tool/user/info.json", region.origin());
    if let Ok(response) = client
        .get(user_info_url)
        .header("Cookie", cookie_header)
        .header("Accept", "application/json, text/plain, */*")
        .header("Referer", format!("{}/", region.origin()))
        .send()
        .await
        && response.status().is_success()
        && let Ok(raw) = response.json::<Value>().await
    {
        let expanded = expand_embedded_json(raw);
        if let Some(token) = find_first_scalar(&expanded, "secToken")
            .or_else(|| find_first_scalar(&expanded, "sec_token"))
        {
            return Ok(token);
        }
    }

    Err("console security token is unavailable; reopen the Coding Plan page".to_string())
}

fn extract_sec_token(html: &str) -> Option<String> {
    let patterns = [
        r#"SEC_TOKEN\s*:\s*[\"']([^\"']+)[\"']"#,
        r#"secToken\s*:\s*[\"']([^\"']+)[\"']"#,
        r#"sec_token\s*:\s*[\"']([^\"']+)[\"']"#,
        r#"[\"']SEC_TOKEN[\"']\s*:\s*[\"']([^\"']+)[\"']"#,
        r#"[\"']sec_token[\"']\s*:\s*[\"']([^\"']+)[\"']"#,
    ];
    patterns.iter().find_map(|pattern| {
        Regex::new(pattern)
            .ok()?
            .captures(html)?
            .get(1)
            .map(|value| value.as_str().to_string())
    })
}

fn parse_alibaba_coding_windows(
    body: &str,
) -> Result<(Option<UsageWindow>, Option<UsageWindow>), String> {
    let raw: Value = serde_json::from_str(body).map_err(|_| "invalid JSON response".to_string())?;
    let expanded = expand_embedded_json(raw);
    if let Some(code) = find_first_scalar(&expanded, "code") {
        let lower = code.to_ascii_lowercase();
        if lower.contains("needlogin") || lower.contains("notlogined") {
            return Err("browser login expired".to_string());
        }
    }
    let quota = find_object_with_any_key(
        &expanded,
        &[
            "per5HourUsedQuota",
            "per5HourTotalQuota",
            "perWeekUsedQuota",
            "perWeekTotalQuota",
        ],
    )
    .ok_or_else(|| "Coding Plan quota windows are missing".to_string())?;

    let five_hour = quota_window(
        quota,
        &["per5HourUsedQuota", "perFiveHourUsedQuota"],
        &["per5HourTotalQuota", "perFiveHourTotalQuota"],
        &[
            "per5HourQuotaNextRefreshTime",
            "perFiveHourQuotaNextRefreshTime",
        ],
    );
    let weekly = quota_window(
        quota,
        &["perWeekUsedQuota"],
        &["perWeekTotalQuota"],
        &["perWeekQuotaNextRefreshTime"],
    );
    if five_hour.is_none() && weekly.is_none() {
        return Err("Coding Plan returned no usable quota counters".to_string());
    }
    Ok((five_hour, weekly))
}

fn quota_window(
    object: &Map<String, Value>,
    used_keys: &[&str],
    total_keys: &[&str],
    reset_keys: &[&str],
) -> Option<UsageWindow> {
    let used = used_keys
        .iter()
        .find_map(|key| value_number(object.get(*key)))?;
    let total = total_keys
        .iter()
        .find_map(|key| value_number(object.get(*key)))?;
    if total <= 0.0 {
        return None;
    }
    Some(UsageWindow {
        percent: (used / total * 100.0).clamp(0.0, 100.0),
        resets_at: reset_keys
            .iter()
            .find_map(|key| value_date(object.get(*key))),
    })
}

async fn fetch_alibaba_personal(
    client: &Client,
    cookie_header: &str,
    region: AlibabaRegion,
    account_trace_id: Option<&str>,
) -> Result<(Option<UsageWindow>, Option<UsageWindow>), String> {
    const USAGE_API: &str = "zeldaHttp.apikeyMgr./tokenplan/personal/api/v2/usage";
    let mut url = reqwest::Url::parse(&format!("https://{}/data/api.json", region.quota_host()))
        .map_err(|error| format!("invalid quota URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("action", region.action())
        .append_pair("product", "sfm_bailian")
        .append_pair("api", USAGE_API)
        .append_pair("_v", "undefined");

    let dashboard_url = if matches!(region, AlibabaRegion::International) {
        account_trace_id
            .map(|trace_id| add_query_value(region.dashboard(), "accounttraceid", trace_id))
            .unwrap_or_else(|| region.dashboard().to_string())
    } else {
        region.dashboard().to_string()
    };
    let sec_token =
        resolve_alibaba_sec_token(client, cookie_header, region, &dashboard_url).await?;

    let mut cornerstone = json!({
        "feTraceId": Uuid::new_v4().to_string(),
        "feURL": dashboard_url.clone(),
        "protocol": "V2",
        "console": "ONE_CONSOLE",
        "productCode": "p_efm",
        "switchAgent": 1222756,
        "switchUserType": 3,
        "domain": reqwest::Url::parse(region.dashboard()).ok().and_then(|url| url.host_str().map(str::to_string)).unwrap_or_default(),
        "consoleSite": region.console_site(),
        "userNickName": "",
        "userPrincipalName": "",
        "xsp_lang": "en-US"
    });
    if let Some(cna) = cookie_value(cookie_header, "cna") {
        cornerstone["X-Anonymous-Id"] = Value::String(cna.to_string());
    }
    let params_json = json!({
        "Api": USAGE_API,
        "V": "1.0",
        "Data": { "cornerstoneParam": cornerstone }
    })
    .to_string();
    let form = [
        ("params", params_json),
        ("region", region.region_id().to_string()),
        ("sec_token", sec_token),
    ];
    let mut request = client
        .post(url)
        .header("Cookie", cookie_header)
        .header("Accept", "application/json, text/plain, */*")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Origin", region.origin())
        .header("Referer", &dashboard_url)
        .form(&form);
    if let Some(csrf) = cookie_value(cookie_header, "login_aliyunid_csrf")
        .or_else(|| cookie_value(cookie_header, "csrf"))
    {
        request = request
            .header("x-xsrf-token", csrf)
            .header("x-csrf-token", csrf);
    }
    let response = request
        .send()
        .await
        .map_err(|error| format!("request failed: {error}"))?;
    let status = response.status();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    let body = response
        .text()
        .await
        .map_err(|error| format!("response could not be read: {error}"))?;
    if !status.is_success() {
        return Err(if status.as_u16() == 401 || status.as_u16() == 403 {
            "browser login expired".to_string()
        } else {
            format!("HTTP {}", status.as_u16())
        });
    }
    if content_type.contains("text/html") || body.trim_start().starts_with('<') {
        return Err("browser login required".to_string());
    }

    parse_alibaba_windows(&body)
}

fn parse_alibaba_windows(body: &str) -> Result<(Option<UsageWindow>, Option<UsageWindow>), String> {
    let raw: Value = serde_json::from_str(body).map_err(|_| "invalid JSON response".to_string())?;
    let expanded = expand_embedded_json(raw);
    if let Some(error_code) = find_first_scalar(&expanded, "errorCode")
        && error_code.to_ascii_lowercase().contains("notlogined")
    {
        return Err("browser login expired".to_string());
    }
    let usage = find_object_with_any_key(&expanded, &["per5HourPercentage", "per1WeekPercentage"])
        .ok_or_else(|| {
            let mut keys = Vec::new();
            collect_json_keys(&expanded, 0, &mut keys);
            keys.sort();
            keys.dedup();
            let diagnostic = ["errorCode", "errorMsg", "code", "httpStatusCode"]
                .iter()
                .filter_map(|key| {
                    find_first_scalar(&expanded, key).map(|value| format!("{key}={value}"))
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Personal usage windows are missing ({diagnostic}; response keys: {})",
                keys.into_iter().take(24).collect::<Vec<_>>().join(", ")
            )
        })?;

    let five_hour = value_number(usage.get("per5HourPercentage")).map(|percent| UsageWindow {
        percent: normalize_percent(percent),
        resets_at: value_date(usage.get("per5HourResetTime")),
    });
    let weekly = value_number(usage.get("per1WeekPercentage")).map(|percent| UsageWindow {
        percent: normalize_percent(percent),
        resets_at: value_date(usage.get("per1WeekResetTime")),
    });
    Ok((five_hour, weekly))
}

fn find_first_scalar(value: &Value, wanted: &str) -> Option<String> {
    match value {
        Value::Object(object) => {
            if let Some(value) = object.get(wanted) {
                match value {
                    Value::String(text) => return Some(text.chars().take(160).collect()),
                    Value::Number(number) => return Some(number.to_string()),
                    Value::Bool(value) => return Some(value.to_string()),
                    _ => {}
                }
            }
            object
                .values()
                .find_map(|child| find_first_scalar(child, wanted))
        }
        Value::Array(values) => values
            .iter()
            .find_map(|child| find_first_scalar(child, wanted)),
        _ => None,
    }
}

fn collect_json_keys(value: &Value, depth: usize, output: &mut Vec<String>) {
    if depth > 5 {
        return;
    }
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                output.push(key.clone());
                collect_json_keys(child, depth + 1, output);
            }
        }
        Value::Array(values) => {
            for child in values.iter().take(8) {
                collect_json_keys(child, depth + 1, output);
            }
        }
        _ => {}
    }
}

fn expand_embedded_json(value: Value) -> Value {
    match value {
        Value::String(text) => {
            let trimmed = text.trim();
            if (trimmed.starts_with('{') || trimmed.starts_with('['))
                && let Ok(parsed) = serde_json::from_str::<Value>(trimmed)
            {
                return expand_embedded_json(parsed);
            }
            Value::String(text)
        }
        Value::Array(values) => {
            Value::Array(values.into_iter().map(expand_embedded_json).collect())
        }
        Value::Object(values) => Value::Object(
            values
                .into_iter()
                .map(|(key, value)| (key, expand_embedded_json(value)))
                .collect(),
        ),
        other => other,
    }
}

fn find_object_with_any_key<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Map<String, Value>> {
    match value {
        Value::Object(object) => {
            if keys.iter().any(|key| object.contains_key(*key)) {
                return Some(object);
            }
            object
                .values()
                .find_map(|child| find_object_with_any_key(child, keys))
        }
        Value::Array(values) => values
            .iter()
            .find_map(|child| find_object_with_any_key(child, keys)),
        _ => None,
    }
}

fn value_number(value: Option<&Value>) -> Option<f64> {
    match value? {
        Value::Number(number) => number.as_f64(),
        Value::String(text) => text.trim().trim_end_matches('%').parse().ok(),
        _ => None,
    }
}

fn value_date(value: Option<&Value>) -> Option<String> {
    let value = value?;
    if let Some(number) = value_number(Some(value)) {
        return epoch_to_rfc3339(number);
    }
    let text = value.as_str()?.trim();
    OffsetDateTime::parse(text, &Rfc3339)
        .ok()
        .and_then(shared::to_rfc3339)
}

fn cookie_value<'a>(header: &'a str, name: &str) -> Option<&'a str> {
    header.split(';').find_map(|part| {
        let (candidate, value) = part.trim().split_once('=')?;
        (candidate == name).then_some(value)
    })
}

fn extract_query_value(url: &str, name: &str) -> Option<String> {
    reqwest::Url::parse(url)
        .ok()?
        .query_pairs()
        .find_map(|(key, value)| (key == name).then(|| value.into_owned()))
}

fn add_query_value(url: &str, name: &str, value: &str) -> String {
    let Ok(mut parsed) = reqwest::Url::parse(url) else {
        return url.to_string();
    };
    parsed.query_pairs_mut().append_pair(name, value);
    parsed.to_string()
}

#[cfg(test)]
mod tests {
    use super::{UsageWindow, apply_monthly_cap, parse_named_window};

    const USAGE_DATA: &str = r#"
        rollingUsage:$R[33]={status:"ok",resetInSec:18000,usagePercent:0},
        weeklyUsage:$R[34]={status:"ok",resetInSec:124240,usagePercent:72},
        monthlyUsage:$R[35]={status:"rate-limited",resetInSec:646300,usagePercent:100}
    "#;

    #[test]
    fn named_windows_do_not_borrow_a_sibling_percentage() {
        assert_eq!(
            parse_named_window(USAGE_DATA, "rollingUsage")
                .unwrap()
                .percent,
            0.0
        );
        assert_eq!(
            parse_named_window(USAGE_DATA, "weeklyUsage")
                .unwrap()
                .percent,
            72.0
        );
        assert_eq!(
            parse_named_window(USAGE_DATA, "monthlyUsage")
                .unwrap()
                .percent,
            100.0
        );
    }

    #[test]
    fn exhausted_monthly_limit_blocks_the_rolling_window() {
        let mut rolling = UsageWindow {
            percent: 0.0,
            resets_at: Some("rolling-reset".to_string()),
        };
        let monthly = UsageWindow {
            percent: 100.0,
            resets_at: Some("monthly-reset".to_string()),
        };

        apply_monthly_cap(&mut rolling, &monthly);

        assert_eq!(rolling.percent, 100.0);
        assert_eq!(rolling.resets_at.as_deref(), Some("monthly-reset"));
    }
}
