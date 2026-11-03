//! Turns the file given on the command line into a path rcheevos can hash.
//!
//! rcheevos treats `.zip` and `.7z` files as arcade games and hashes their
//! name, so anything that isn't explicitly arcade must not reach it as an
//! archive.

use crate::rc_hash::{RC_CONSOLE_ARCADE, RC_CONSOLE_MS_DOS};
use sevenz_rust2::{ArchiveReader, Password};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Component, Path, PathBuf};
use tempfile::TempDir;
use zip::ZipArchive;

const MAX_PLAYLIST_DEPTH: usize = 8;

/// A path ready for hashing. An extracted file lives as long as this value.
pub struct Input {
    pub path: PathBuf,
    _extracted_to: Option<TempDir>,
}

impl Input {
    fn at(path: &Path) -> Self {
        Input {
            path: path.to_owned(),
            _extracted_to: None,
        }
    }
}

pub fn prepare(file: &Path, console: Option<u32>) -> Result<Input, String> {
    let is_arcade = console == Some(RC_CONSOLE_ARCADE);

    if !file.exists() {
        return Err("no such file".to_owned());
    }
    if !file.is_file() && !is_arcade {
        return Err("not a file".to_owned());
    }
    if has_extension(file, "chd") {
        return Err(not_supported("CHD"));
    }
    if is_arcade {
        return Ok(Input::at(file));
    }
    if has_extension(file, "zip") {
        // The DOS hash covers the contents of the zip itself.
        if console == Some(RC_CONSOLE_MS_DOS) {
            return Ok(Input::at(file));
        }
        return extract_first_zip_file(file);
    }
    if has_extension(file, "7z") {
        return extract_first_7z_file(file);
    }
    reject_unsupported(file)?;
    Ok(Input::at(file))
}

// Matches how rcheevos finds an extension: a file named `.zip` has one.
pub fn has_extension(path: &Path, extension: &str) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case(extension))
}

pub fn note_unrecognised(file: &Path) {
    eprintln!(
        "note: {}: unrecognised file extension; skipped",
        file.display()
    );
}

fn not_supported(format: &str) -> String {
    format!("{format} files are not supported yet")
}

/// Rejects files rcheevos can't hash correctly, following playlists the way
/// it does: through the first item of each one.
fn reject_unsupported(file: &Path) -> Result<(), String> {
    let mut current = file.to_owned();
    for _ in 0..MAX_PLAYLIST_DEPTH {
        if !has_extension(&current, "m3u") {
            for (extension, format) in [("chd", "CHD"), ("7z", "nested 7z"), ("zip", "nested zip")]
            {
                if has_extension(&current, extension) {
                    return Err(not_supported(format));
                }
            }
            return Ok(());
        }
        // rcheevos reports playlists it can't read.
        let Ok(playlist) = fs::read(&current) else {
            return Ok(());
        };
        let item = first_playlist_item(&playlist);
        current = current.parent().unwrap_or(Path::new("")).join(item);
    }
    Err("playlists are nested too deeply".to_owned())
}

fn first_playlist_item(playlist: &[u8]) -> String {
    String::from_utf8_lossy(playlist)
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with('#'))
        .unwrap_or_default()
        .to_owned()
}

fn extract_first_zip_file(file: &Path) -> Result<Input, String> {
    let describe = |error: zip::result::ZipError| error.to_string();
    let source = File::open(file).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(source).map_err(describe)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(describe)?;
        // An empty file can't be the ROM, but it would hash as if it were.
        if entry.is_dir() || entry.size() == 0 {
            continue;
        }
        let name = entry
            .enclosed_name()
            .and_then(|path| path.file_name().map(PathBuf::from))
            .ok_or_else(|| {
                let raw_name = String::from_utf8_lossy(entry.name_raw());
                format!("unsafe zip entry name: {raw_name}")
            })?;
        return prepare_extracted(&name, &mut entry);
    }
    Err("zip file has no files".to_owned())
}

fn extract_first_7z_file(file: &Path) -> Result<Input, String> {
    let describe = |error: sevenz_rust2::Error| match error {
        sevenz_rust2::Error::Io(error, _) => format!("can't read 7z file: {error}"),
        error => format!("can't read 7z file: {error}"),
    };
    let mut archive = ArchiveReader::open(file, Password::empty()).map_err(describe)?;

    let mut extracted = None;
    archive
        .for_each_entries(|entry, contents| {
            // The returned value says whether to go on to the next entry.
            // An empty file can't be the ROM, but it would hash as if it were.
            if entry.is_directory() || entry.size() == 0 {
                return Ok(true);
            }
            extracted = Some(match safe_7z_file_name(entry.name()) {
                Some(name) => prepare_extracted(name, contents),
                None => Err(format!("unsafe 7z entry name: {}", entry.name())),
            });
            Ok(false)
        })
        .map_err(describe)?;
    extracted.unwrap_or_else(|| Err("7z file has no files".to_owned()))
}

fn safe_7z_file_name(name: &str) -> Option<&Path> {
    let path = Path::new(name);
    let is_enclosed = path
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if !is_enclosed {
        return None;
    }
    path.file_name().map(Path::new)
}

/// `name` must be a base name, so entries can't escape the directory.
fn prepare_extracted(name: &Path, contents: &mut dyn Read) -> Result<Input, String> {
    let directory = TempDir::new().map_err(|error| error.to_string())?;
    let path = directory.path().join(name);
    let mut output = File::create(&path).map_err(|error| error.to_string())?;
    io::copy(contents, &mut output).map_err(|error| error.to_string())?;
    reject_unsupported(&path)?;
    Ok(Input {
        path,
        _extracted_to: Some(directory),
    })
}
