//! Downloads a console's list from the RetroAchievements v2 API.

use serde_json::{Map, Value, json};
use std::fmt;
use std::time::Duration;
use ureq::Agent;

const DEFAULT_URL: &str = "https://api.retroachievements.org/v2";

/// Part of the hidden subcommand's arguments, and so of the cache key. Change
/// it whenever the output of [`fetch_system`] changes, or an upgraded binary
/// would keep reading lists written in the old format.
pub const PAYLOAD_FORMAT: u32 = 1;

/// How the hidden subcommand signals [`FetchError::Rejected`] to its parent.
pub const EXIT_REJECTED: u8 = 3;

/// Retries of one request after a 429, on top of the first try.
const MAX_RETRIES: u32 = 3;
/// For a 429 whose `Retry-After` isn't a number of seconds.
const DEFAULT_RETRY_WAIT_SECS: u64 = 1;
/// A longer limit isn't brief, and waiting it out would leave the run stuck.
const MAX_RETRY_WAIT_SECS: u64 = 60;

#[derive(Debug)]
pub enum FetchError {
    /// The server refused the key, so every other download would fail the same
    /// way and the run should stop.
    Rejected(String),
    Failed(String),
}

impl fmt::Display for FetchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Rejected(message) | Self::Failed(message) => f.write_str(message),
        }
    }
}

/// Returns compact JSON mapping each hash to its game, for the hidden
/// subcommand to print. Reports each page on stderr as it arrives.
pub fn fetch_system(system_id: u32) -> Result<String, FetchError> {
    let key = std::env::var("RA_API_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .ok_or_else(|| {
            FetchError::Failed(
                "RA_API_KEY is not set; copy your free Web API Key from \
                 https://retroachievements.org/settings and export it as RA_API_KEY"
                    .to_owned(),
            )
        })?;
    let url = std::env::var("RA_API_URL").unwrap_or_else(|_| DEFAULT_URL.to_owned());
    // The status decides between retrying, stopping the run and failing.
    let agent: Agent = Agent::config_builder()
        .http_status_as_error(false)
        .build()
        .into();

    let mut list = Map::new();
    let mut page_number = 1;
    loop {
        let page = fetch_page(&agent, &url, &key, system_id, page_number)?;
        list.extend(
            console_list(&page)
                .ok_or_else(|| failed(system_id, "response has an unexpected shape"))?,
        );
        // Read from every page rather than worked out from the page size,
        // because the server may give fewer games per page than requested.
        let last = page["meta"]["page"]["lastPage"]
            .as_u64()
            .ok_or_else(|| failed(system_id, "response has no last page"))?;
        eprintln!("console {system_id}: page {page_number}/{last}");
        if page_number >= last {
            return Ok(Value::Object(list).to_string());
        }
        page_number += 1;
    }
}

fn fetch_page(
    agent: &Agent,
    url: &str,
    key: &str,
    system_id: u32,
    page_number: u64,
) -> Result<Value, FetchError> {
    let mut retries = 0;
    loop {
        let mut response = agent
            .get(format!("{url}/games"))
            .header("X-API-Key", key)
            .query("filter[systemId]", system_id.to_string())
            .query("include", "hashes")
            // Without `hashes`, JSON:API would leave out the relationship that
            // links a game to the hashes in `included`.
            .query("fields[games]", "title,achievementsPublished,hashes")
            .query("fields[game-hashes]", "raMd5,compatibility")
            .query("page[size]", "100")
            .query("page[number]", page_number.to_string())
            .call()
            .map_err(|error| failed(system_id, error))?;

        let status = response.status();
        if matches!(status.as_u16(), 401 | 403) {
            return Err(FetchError::Rejected(format!(
                "console {system_id}: the server rejected RA_API_KEY (HTTP status {}); \
                 check your Web API Key at https://retroachievements.org/settings",
                status.as_u16()
            )));
        }
        if status.as_u16() == 429 {
            if retries == MAX_RETRIES {
                return Err(failed(
                    system_id,
                    format!("still rate limited after {MAX_RETRIES} retries"),
                ));
            }
            let wait = retry_wait_secs(&response);
            if wait > MAX_RETRY_WAIT_SECS {
                return Err(failed(
                    system_id,
                    format!("rate limited for {wait} seconds"),
                ));
            }
            eprintln!("console {system_id}: rate limited; retrying in {wait}s");
            std::thread::sleep(Duration::from_secs(wait));
            retries += 1;
            continue;
        }
        if !status.is_success() {
            return Err(failed(
                system_id,
                format!("HTTP status {}", status.as_u16()),
            ));
        }

        let body = response
            .body_mut()
            .read_to_string()
            .map_err(|error| failed(system_id, error))?;
        return serde_json::from_str(&body)
            .map_err(|error| failed(system_id, format!("invalid response: {error}")));
    }
}

fn failed(system_id: u32, message: impl fmt::Display) -> FetchError {
    FetchError::Failed(format!("console {system_id}: {message}"))
}

fn retry_wait_secs<B>(response: &ureq::http::Response<B>) -> u64 {
    response
        .headers()
        .get("Retry-After")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(DEFAULT_RETRY_WAIT_SECS)
}

fn console_list(page: &Value) -> Option<Map<String, Value>> {
    let hashes: Map<String, Value> = page
        .get("included")
        .map_or(Some(&Vec::new()), Value::as_array)?
        .iter()
        .filter(|resource| resource["type"] == "game-hashes")
        .map(|hash| Some((hash["id"].as_str()?.to_owned(), hash["attributes"].clone())))
        .collect::<Option<_>>()?;

    let mut list = Map::new();
    for game in page["data"].as_array()? {
        let attributes = &game["attributes"];
        for hash in game["relationships"]["hashes"]["data"].as_array()? {
            let hash = hashes.get(hash["id"].as_str()?)?;
            list.insert(
                hash["raMd5"].as_str()?.to_ascii_lowercase(),
                json!({
                    "id": game["id"].as_str()?,
                    "title": attributes["title"].as_str()?,
                    "achievements": attributes["achievementsPublished"].as_u64()?,
                    "compatibility": hash["compatibility"].as_str()?,
                }),
            );
        }
    }
    Some(list)
}
