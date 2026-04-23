use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsagePayload {
    pub provider_id: &'static str,
    pub plan: Option<String>,
    pub lines: Vec<MetricLinePayload>,
    pub source: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum MetricLinePayload {
    #[serde(rename = "progress")]
    Progress {
        label: String,
        used: f64,
        limit: f64,
        format: MetricFormat,
        #[serde(skip_serializing_if = "Option::is_none")]
        resets_at: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        color: Option<String>,
    },
    #[serde(rename = "text")]
    Text {
        label: String,
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        subtitle: Option<String>,
    },
    #[serde(rename = "badge")]
    Badge {
        label: String,
        value: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tone: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MetricFormat {
    #[serde(rename = "percent")]
    Percent,
    #[allow(dead_code)]
    #[serde(rename = "count")]
    Count { suffix: String },
    #[serde(rename = "currency")]
    Currency { currency: String },
}

pub fn to_rfc3339(timestamp: OffsetDateTime) -> Option<String> {
    timestamp.format(&Rfc3339).ok()
}

pub fn unix_seconds_to_rfc3339(seconds: i64) -> Option<String> {
    OffsetDateTime::from_unix_timestamp(seconds)
        .ok()
        .and_then(to_rfc3339)
}

pub fn title_case(value: &str) -> String {
    value
        .split([' ', '-', '_'])
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut out = String::new();
                    out.extend(first.to_uppercase());
                    out.push_str(&chars.as_str().to_lowercase());
                    out
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn format_usd(value: f64) -> String {
    format!("${value:.2}")
}

pub fn format_token_count(value: u64) -> String {
    if value == 0 {
        return "0".to_string();
    }

    if value >= 1_000_000 {
        let millions = value as f64 / 1_000_000.0;
        return format!("{millions:.1}M").replace(".0M", "M");
    }

    if value >= 1_000 {
        let thousands = value as f64 / 1_000.0;
        return format!("{thousands:.1}K").replace(".0K", "K");
    }

    value.to_string()
}
