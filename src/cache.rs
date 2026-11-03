//! Keeps each console's list on disk, using `bkt` to run the hidden
//! `__fetch-system` subcommand of this same binary.

use crate::api::{EXIT_REJECTED, FetchError, PAYLOAD_FORMAT};
use bkt::{Bkt, CacheStatus, CommandDesc, Invocation};
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::fmt::Display;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The only TTL ever given to `bkt`. It deletes an entry when a lookup finds it
/// older than the requested TTL, so a shorter one would lose lists we keep.
const TTL: Duration = Duration::from_secs(100 * 365 * 24 * 60 * 60);

/// How old a list must be before a "no achievements" result downloads it again.
const UPDATE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// A file no key points to may be another run's list, written just before it
/// links it to its key. Writing and linking are far quicker than this.
const PRUNE_AFTER: Duration = Duration::from_secs(60);

/// A console's list, and when it was downloaded.
pub struct ConsoleList {
    system_id: u32,
    pub games: Map<String, Value>,
    downloaded: SystemTime,
}

/// Loads and updates lists for one run, which updates each console only once.
#[derive(Default)]
pub struct Lists {
    /// Consoles this run tried to download, whether or not that worked.
    tried: HashSet<u32>,
}

impl Lists {
    /// Returns a console's list, downloading it only if it isn't cached. With
    /// nothing cached, each call after a failed download tries again.
    pub fn load(&mut self, system_id: u32) -> Result<ConsoleList, FetchError> {
        let (invocation, status) = bkt()?
            .retrieve_streaming(
                &command(system_id)?,
                TTL,
                io::sink(),
                LiveProgress::default(),
            )
            .map_err(cache_failure)?;
        let downloaded = match status {
            CacheStatus::Hit(time) => time,
            CacheStatus::Miss(_) => {
                self.tried.insert(system_id);
                SystemTime::now()
            }
        };
        Ok(ConsoleList {
            system_id,
            games: read_list(&invocation, system_id)?,
            downloaded,
        })
    }

    /// Downloads a list again, unless this run already tried. On failure, both
    /// the list and its age stay as they were.
    pub fn update(&mut self, list: &mut ConsoleList) -> Result<(), FetchError> {
        if !self.tried.insert(list.system_id) {
            return Ok(());
        }
        let (invocation, _) = bkt()?
            .refresh_streaming(
                &command(list.system_id)?,
                TTL,
                io::sink(),
                LiveProgress::default(),
            )
            .map_err(cache_failure)?;
        list.games = read_list(&invocation, list.system_id)?;
        list.downloaded = SystemTime::now();
        // Only housekeeping, so a failure shouldn't fail the update.
        if let Ok(root) = cache_dir() {
            let _ = prune_replaced_lists(&root);
        }
        Ok(())
    }

    /// For a "no achievements" result: as [`Lists::update`], if the list is old
    /// enough for that result to be out of date.
    pub fn update_if_due(&mut self, list: &mut ConsoleList) -> Result<(), FetchError> {
        if list
            .downloaded
            .elapsed()
            .is_ok_and(|age| age >= UPDATE_AFTER)
        {
            return self.update(list);
        }
        Ok(())
    }
}

fn cache_failure(error: impl Display) -> FetchError {
    FetchError::Failed(format!("{error:#}"))
}

fn bkt() -> Result<Bkt, FetchError> {
    Bkt::create(cache_dir().map_err(FetchError::Failed)?).map_err(cache_failure)
}

fn command(system_id: u32) -> Result<CommandDesc, FetchError> {
    let exe = std::env::current_exe().map_err(|error| FetchError::Failed(error.to_string()))?;
    // RA_API_KEY reaches the subcommand by inheritance. `with_env` would make
    // it part of the cache key, and write it to disk.
    Ok(CommandDesc::new([
        exe.into_os_string(),
        "__fetch-system".into(),
        "--format".into(),
        PAYLOAD_FORMAT.to_string().into(),
        system_id.to_string().into(),
    ])
    .with_discard_failures(true))
}

fn read_list(invocation: &Invocation, system_id: u32) -> Result<Map<String, Value>, FetchError> {
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

/// Deletes the lists that updates have replaced. `bkt` only points a key at the
/// new list, and with our TTL its own cleanup would keep the old one for 100
/// years. It lays out each cache as `keys/<symlink>` and `data/<ttl>/<file>`.
fn prune_replaced_lists(root: &Path) -> io::Result<()> {
    for cache in fs::read_dir(root)? {
        let cache = cache?.path();
        let mut linked = HashSet::new();
        for key in fs::read_dir(cache.join("keys"))? {
            linked.insert(fs::canonicalize(key?.path())?);
        }
        for ttl_dir in fs::read_dir(cache.join("data"))? {
            for file in fs::read_dir(ttl_dir?.path())? {
                let file = fs::canonicalize(file?.path())?;
                let age = fs::metadata(&file)?.modified()?.elapsed();
                if !linked.contains(&file) && age.is_ok_and(|age| age >= PRUNE_AFTER) {
                    fs::remove_file(file)?;
                }
            }
        }
    }
    Ok(())
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
