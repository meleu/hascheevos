//! Keeps each console's list on disk, using `bkt` to run the hidden
//! `__fetch-system` subcommand of this same binary.

use crate::api::{EXIT_REJECTED, FetchError, PAYLOAD_FORMAT};
use bkt::{Bkt, CommandDesc};
use serde_json::{Map, Value};
use std::io::{self, Write};
use std::path::PathBuf;
use std::time::Duration;

/// The only TTL ever given to `bkt`. It deletes an entry when a lookup finds it
/// older than the requested TTL, so a shorter one would lose lists we keep.
const TTL: Duration = Duration::from_secs(100 * 365 * 24 * 60 * 60);

/// Returns a console's list, downloading it only if it isn't cached.
pub fn system_list(system_id: u32) -> Result<Map<String, Value>, FetchError> {
    let exe = std::env::current_exe().map_err(|error| FetchError::Failed(error.to_string()))?;
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

    let bkt = Bkt::create(cache_dir().map_err(FetchError::Failed)?)
        .map_err(|error| FetchError::Failed(format!("{error:#}")))?;
    let (invocation, _) = bkt
        .retrieve_streaming(&command, TTL, io::sink(), LiveProgress::default())
        .map_err(|error| FetchError::Failed(format!("{error:#}")))?;
    let code = invocation.exit_code();
    if code != 0 {
        // On stdout, because stderr has already been shown as progress.
        let message = match invocation.stdout_utf8().trim() {
            "" => format!("console {system_id}: download exited with code {code}"),
            message => message.to_owned(),
        };
        return Err(if code == i32::from(EXIT_REJECTED) {
            FetchError::Rejected(message)
        } else {
            FetchError::Failed(message)
        });
    }
    serde_json::from_slice(invocation.stdout()).map_err(|error| {
        FetchError::Failed(format!("console {system_id}: invalid cached list: {error}"))
    })
}

/// Shows a download's progress on stderr as it happens. `bkt` also replays a
/// cached entry's stderr into its sink, which would repeat the progress of an
/// old download on every lookup. The two can only be told apart by `bkt`
/// flushing after each chunk of a running command but not after a replay, so
/// only what gets flushed is shown.
#[derive(Default)]
struct LiveProgress(Vec<u8>);

impl Write for LiveProgress {
    fn write(&mut self, chunk: &[u8]) -> io::Result<usize> {
        self.0.extend_from_slice(chunk);
        Ok(chunk.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        // A closed stderr shouldn't fail the download it was only reporting on.
        let _ = io::stderr().write_all(&self.0);
        self.0.clear();
        Ok(())
    }
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
