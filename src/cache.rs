//! Keeps each console's list on disk, using `bkt` to run the hidden
//! `__fetch-system` subcommand of this same binary.

use crate::api::PAYLOAD_FORMAT;
use bkt::{Bkt, CommandDesc};
use serde_json::{Map, Value};
use std::path::PathBuf;
use std::time::Duration;

/// The only TTL ever given to `bkt`. It deletes an entry when a lookup finds it
/// older than the requested TTL, so a shorter one would lose lists we keep.
const TTL: Duration = Duration::from_secs(100 * 365 * 24 * 60 * 60);

/// Returns a console's list, downloading it only if it isn't cached.
pub fn system_list(system_id: u32) -> Result<Map<String, Value>, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    // RA_API_KEY reaches the subcommand by inheritance. `with_env` would make
    // it part of the cache key, and write it to disk.
    let command = CommandDesc::new([
        exe.into_os_string(),
        "__fetch-system".into(),
        "--format".into(),
        PAYLOAD_FORMAT.to_string().into(),
        system_id.to_string().into(),
    ])
    .with_discard_failures(true);

    let bkt = Bkt::create(cache_dir()?).map_err(|error| format!("{error:#}"))?;
    let (invocation, _) = bkt
        .retrieve(&command, TTL)
        .map_err(|error| format!("{error:#}"))?;
    if invocation.exit_code() != 0 {
        return Err(invocation.stderr_utf8().trim().to_owned());
    }
    serde_json::from_slice(invocation.stdout())
        .map_err(|error| format!("console {system_id}: invalid cached list: {error}"))
}

fn cache_dir() -> Result<PathBuf, String> {
    // The XDG spec says to ignore anything but an absolute path.
    let absolute = |name| {
        std::env::var_os(name)
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
    };
    let root = absolute("XDG_CACHE_HOME")
        .or_else(|| Some(absolute("HOME")?.join(".cache")))
        .ok_or("neither XDG_CACHE_HOME nor HOME is an absolute path")?;
    Ok(root.join("hascheevos"))
}
