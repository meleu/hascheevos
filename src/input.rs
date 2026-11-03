//! Turns the file given on the command line into a path rcheevos can hash.
//!
//! rcheevos treats `.zip` and `.7z` files as arcade games and hashes their
//! name, so anything that isn't explicitly arcade must not reach it as an
//! archive.

use crate::rc_hash::{RC_CONSOLE_ARCADE, RC_CONSOLE_MS_DOS};
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
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
        return extract_first_file(file);
    }
    reject_unsupported(file)?;
    Ok(Input::at(file))
}

// Matches how rcheevos finds an extension: a file named `.zip` has one.
fn has_extension(path: &Path, extension: &str) -> bool {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    name.rsplit_once('.')
        .is_some_and(|(_, ext)| ext.eq_ignore_ascii_case(extension))
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
            for (extension, format) in [("chd", "CHD"), ("7z", "7z"), ("zip", "nested zip")] {
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

fn extract_first_file(file: &Path) -> Result<Input, String> {
    let describe = |error: zip::result::ZipError| error.to_string();
    let source = File::open(file).map_err(|error| error.to_string())?;
    let mut archive = ZipArchive::new(source).map_err(describe)?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(describe)?;
        if entry.is_dir() {
            continue;
        }
        // Only the base name is used, so entries can't escape the directory.
        let name = entry
            .enclosed_name()
            .and_then(|path| path.file_name().map(PathBuf::from))
            .ok_or_else(|| {
                let raw_name = String::from_utf8_lossy(entry.name_raw());
                format!("unsafe zip entry name: {raw_name}")
            })?;

        let directory = TempDir::new().map_err(|error| error.to_string())?;
        let path = directory.path().join(name);
        let mut output = File::create(&path).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        reject_unsupported(&path)?;
        return Ok(Input {
            path,
            _extracted_to: Some(directory),
        });
    }
    Err("zip file has no files".to_owned())
}
