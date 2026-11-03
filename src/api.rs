//! Downloads a console's list from the RetroAchievements v2 API.

use serde_json::{Map, Value, json};

const DEFAULT_URL: &str = "https://api.retroachievements.org/v2";

/// Part of the hidden subcommand's arguments, and so of the cache key. Change
/// it whenever the output of [`fetch_system`] changes, or an upgraded binary
/// would keep reading lists written in the old format.
pub const PAYLOAD_FORMAT: u32 = 1;

/// Returns compact JSON mapping each hash to its game, for the hidden
/// subcommand to print.
pub fn fetch_system(system_id: u32) -> Result<String, String> {
    let key = std::env::var("RA_API_KEY")
        .ok()
        .filter(|key| !key.is_empty())
        .ok_or(
            "RA_API_KEY is not set; copy your free Web API Key from \
             https://retroachievements.org/settings and export it as RA_API_KEY",
        )?;
    let url = std::env::var("RA_API_URL").unwrap_or_else(|_| DEFAULT_URL.to_owned());

    let body = ureq::get(format!("{url}/games"))
        .header("X-API-Key", key)
        .query("filter[systemId]", system_id.to_string())
        .query("include", "hashes")
        // Without `hashes`, JSON:API would leave out the relationship that
        // links a game to the hashes in `included`.
        .query("fields[games]", "title,achievementsPublished,hashes")
        .query("fields[game-hashes]", "raMd5,compatibility")
        .query("page[size]", "100")
        .call()
        .and_then(|mut response| response.body_mut().read_to_string())
        .map_err(|error| format!("console {system_id}: {error}"))?;

    let page: Value = serde_json::from_str(&body)
        .map_err(|error| format!("console {system_id}: invalid response: {error}"))?;
    let list = console_list(&page)
        .ok_or_else(|| format!("console {system_id}: response has an unexpected shape"))?;
    // Keeping only the first page would cache a partial list indefinitely.
    if page["meta"]["page"]["lastPage"].as_u64() != Some(1) {
        return Err(format!(
            "console {system_id}: lists of several pages aren't supported yet"
        ));
    }
    Ok(Value::Object(list).to_string())
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
