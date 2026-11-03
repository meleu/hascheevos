//! Keeps each console's list and each game on disk, using `bkt` to run the
//! hidden subcommands of this same binary.

use crate::api::{EXIT_REJECTED, FetchError, PAYLOAD_FORMAT};
use bkt::{Bkt, CacheStatus, CommandDesc, Invocation};
use serde_json::Value;
use std::collections::HashSet;
use std::fmt::{self, Display};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// The only TTL ever given to `bkt`. It deletes an entry when a lookup finds it
/// older than the requested TTL, so a shorter one would lose entries we keep.
const TTL: Duration = Duration::from_secs(100 * 365 * 24 * 60 * 60);

/// How old an entry must be before a "no achievements" result downloads it
/// again.
const UPDATE_AFTER: Duration = Duration::from_secs(24 * 60 * 60);

/// A file no key points to may be another run's entry, written just before it
/// links it to its key. Writing and linking are far quicker than this.
const PRUNE_AFTER: Duration = Duration::from_secs(60);

/// What a cache entry holds, and so where it's downloaded from.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Source {
    /// A list mapping each of a console's hashes to its game.
    Console(u32),
    /// One game's title and achievement count.
    Game(u32),
}

impl Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::Console(id) => write!(f, "console {id}"),
            Self::Game(id) => write!(f, "game {id}"),
        }
    }
}

/// What was downloaded from a source, and when.
pub struct Entry {
    source: Source,
    pub data: Value,
    downloaded: SystemTime,
}

/// Loads and updates entries for one run, which updates each source only once.
#[derive(Default)]
pub struct Entries {
    /// Sources this run tried to download, whether or not that worked.
    tried: HashSet<Source>,
}

impl Entries {
    /// Returns a source's entry, downloading it only if it isn't cached. With
    /// nothing cached, each call after a failed download tries again.
    pub fn load(&mut self, source: Source) -> Result<Entry, FetchError> {
        let (invocation, status) = bkt()?
            .retrieve_streaming(&command(source)?, TTL, io::sink(), LiveProgress::default())
            .map_err(cache_failure)?;
        let downloaded = match status {
            CacheStatus::Hit(time) => time,
            CacheStatus::Miss(_) => {
                self.tried.insert(source);
                SystemTime::now()
            }
        };
        Ok(Entry {
            source,
            data: read_data(&invocation, source)?,
            downloaded,
        })
    }

    /// Downloads an entry again, unless this run already tried. On failure,
    /// both the entry and its age stay as they were.
    pub fn update(&mut self, entry: &mut Entry) -> Result<(), FetchError> {
        if !self.tried.insert(entry.source) {
            return Ok(());
        }
        let (invocation, _) = bkt()?
            .refresh_streaming(
                &command(entry.source)?,
                TTL,
                io::sink(),
                LiveProgress::default(),
            )
            .map_err(cache_failure)?;
        entry.data = read_data(&invocation, entry.source)?;
        entry.downloaded = SystemTime::now();
        // Only housekeeping, so a failure shouldn't fail the update.
        if let Ok(root) = cache_dir() {
            let _ = prune_replaced_entries(&root);
        }
        Ok(())
    }

    /// For a "no achievements" result: as [`Entries::update`], if the entry is
    /// old enough for that result to be out of date.
    pub fn update_if_due(&mut self, entry: &mut Entry) -> Result<(), FetchError> {
        if entry
            .downloaded
            .elapsed()
            .is_ok_and(|age| age >= UPDATE_AFTER)
        {
            return self.update(entry);
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

fn command(source: Source) -> Result<CommandDesc, FetchError> {
    let exe = std::env::current_exe().map_err(|error| FetchError::Failed(error.to_string()))?;
    let (subcommand, id) = match source {
        Source::Console(id) => ("__fetch-system", id),
        Source::Game(id) => ("__fetch-game", id),
    };
    // RA_API_KEY reaches the subcommand by inheritance. `with_env` would make
    // it part of the cache key, and write it to disk.
    Ok(CommandDesc::new([
        exe.into_os_string(),
        subcommand.into(),
        "--format".into(),
        PAYLOAD_FORMAT.to_string().into(),
        id.to_string().into(),
    ])
    .with_discard_failures(true))
}

fn read_data(invocation: &Invocation, source: Source) -> Result<Value, FetchError> {
    let code = invocation.exit_code();
    if code != 0 {
        // On stdout, because stderr has already been shown as progress.
        let message = match invocation.stdout_utf8().trim() {
            "" => format!("{source}: download exited with code {code}"),
            message => message.to_owned(),
        };
        return Err(if code == i32::from(EXIT_REJECTED) {
            FetchError::Rejected(message)
        } else {
            FetchError::Failed(message)
        });
    }
    let invalid = |error: &dyn Display| {
        FetchError::Failed(format!("{source}: invalid cached entry: {error}"))
    };
    let data: Value = serde_json::from_slice(invocation.stdout()).map_err(|e| invalid(&e))?;
    if !data.is_object() {
        return Err(invalid(&"not an object"));
    }
    Ok(data)
}

/// Deletes the entries that updates have replaced. `bkt` only points a key at
/// the new entry, and with our TTL its own cleanup would keep the old one for
/// 100 years. It lays out each cache as `keys/<symlink>` and `data/<ttl>/<file>`.
fn prune_replaced_entries(root: &Path) -> io::Result<()> {
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
