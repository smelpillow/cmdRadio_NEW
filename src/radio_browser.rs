use serde::Deserialize;
use std::time::Duration;

use crate::m3u::parser::Station;

const SERVERS: [&str; 4] = [
    "https://de1.api.radio-browser.info",
    "https://de2.api.radio-browser.info",
    "https://fi1.api.radio-browser.info",
    "https://at1.api.radio-browser.info",
];
const RESULT_LIMIT: usize = 100;
const REQUEST_TIMEOUT_SECS: u64 = 10;
const MAX_RESPONSE_BYTES: u64 = 8 * 1024 * 1024;

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    pub name: String,
    pub country: String,
    pub tag: String,
    pub language: String,
}

impl SearchQuery {
    /// Parses free text where `tag:`, `country:` and `lang:` tokens are filters
    /// and the remaining words form the station name.
    pub fn parse(input: &str) -> Self {
        let mut query = Self::default();
        let mut name_words: Vec<&str> = Vec::new();

        for word in input.split_whitespace() {
            let lower = word.to_ascii_lowercase();
            if lower.starts_with("tag:") {
                query.tag = word[4..].to_string();
            } else if lower.starts_with("country:") {
                query.country = word[8..].to_string();
            } else if lower.starts_with("lang:") {
                query.language = word[5..].to_string();
            } else {
                name_words.push(word);
            }
        }

        query.name = name_words.join(" ");
        query
    }

    pub fn is_empty(&self) -> bool {
        self.name.is_empty()
            && self.country.is_empty()
            && self.tag.is_empty()
            && self.language.is_empty()
    }

    fn to_query_string(&self) -> String {
        let mut parts = Vec::new();
        for (key, value) in [
            ("name", &self.name),
            ("country", &self.country),
            ("tag", &self.tag),
            ("language", &self.language),
        ] {
            if !value.is_empty() {
                parts.push(format!("{key}={}", percent_encode(value)));
            }
        }
        parts.push(format!("limit={RESULT_LIMIT}"));
        parts.push(String::from("order=votes"));
        parts.push(String::from("reverse=true"));
        parts.push(String::from("hidebroken=true"));
        parts.join("&")
    }
}

#[derive(Debug, Deserialize)]
struct RawStation {
    #[serde(default)]
    name: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    url_resolved: String,
    #[serde(default)]
    lastcheckok: u8,
}

fn percent_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

pub fn parse_stations(json: &str) -> Result<Vec<Station>, String> {
    let raw: Vec<RawStation> =
        serde_json::from_str(json).map_err(|e| format!("invalid Radio Browser response: {e}"))?;

    let mut seen = std::collections::HashSet::new();
    let stations = raw
        .into_iter()
        .filter(|s| s.lastcheckok == 1)
        .filter_map(|s| {
            let url = if s.url_resolved.trim().is_empty() {
                s.url.trim().to_string()
            } else {
                s.url_resolved.trim().to_string()
            };
            let name = s.name.trim().to_string();
            let is_http = url.starts_with("http://") || url.starts_with("https://");
            if name.is_empty() || !is_http || !seen.insert(url.clone()) {
                return None;
            }
            Some(Station { name, url })
        })
        .collect();

    Ok(stations)
}

pub fn search(query: &SearchQuery) -> Result<Vec<Station>, String> {
    let user_agent = format!("cmdradio/{}", env!("CARGO_PKG_VERSION"));
    let query_string = query.to_query_string();
    let mut last_error = String::from("no Radio Browser server available");

    for server in SERVERS {
        let url = format!("{server}/json/stations/search?{query_string}");
        let response = ureq::get(&url)
            .set("User-Agent", &user_agent)
            .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
            .call();

        match response {
            Ok(response) => {
                let mut body = String::new();
                std::io::Read::read_to_string(
                    &mut std::io::Read::take(response.into_reader(), MAX_RESPONSE_BYTES),
                    &mut body,
                )
                .map_err(|e| format!("failed to read Radio Browser response: {e}"))?;
                return parse_stations(&body);
            }
            Err(err) => last_error = format!("{server}: {err}"),
        }
    }

    Err(last_error)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_filters_and_name() {
        let q = SearchQuery::parse("jazz radio tag:smooth country:Spain lang:spanish");
        assert_eq!(q.name, "jazz radio");
        assert_eq!(q.tag, "smooth");
        assert_eq!(q.country, "Spain");
        assert_eq!(q.language, "spanish");
        assert!(!q.is_empty());
        assert!(SearchQuery::parse("   ").is_empty());
    }

    #[test]
    fn encodes_query_string() {
        let q = SearchQuery::parse("rock & roll");
        assert!(q.to_query_string().starts_with("name=rock%20%26%20roll&"));
    }

    #[test]
    fn maps_and_filters_stations() {
        let json = r#"[
            {"name":"A","url":"http://a/pl.pls","url_resolved":"http://a/stream","lastcheckok":1},
            {"name":"Dup","url":"http://a/x","url_resolved":"http://a/stream","lastcheckok":1},
            {"name":"Broken","url":"http://b","url_resolved":"http://b","lastcheckok":0},
            {"name":"","url":"http://c","url_resolved":"http://c","lastcheckok":1},
            {"name":"NoResolved","url":"https://d/s","url_resolved":"","lastcheckok":1},
            {"name":"Ftp","url":"ftp://e","url_resolved":"ftp://e","lastcheckok":1}
        ]"#;
        let stations = parse_stations(json).unwrap();
        let urls: Vec<_> = stations.iter().map(|s| s.url.as_str()).collect();
        assert_eq!(urls, vec!["http://a/stream", "https://d/s"]);
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_stations("not json").is_err());
    }
}
