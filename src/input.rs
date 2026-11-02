//! Turns the file given on the command line into a path rcheevos can hash.
//!
//! rcheevos treats `.zip` and `.7z` files as arcade games and hashes their
//! name, so anything that isn't explicitly arcade must not reach it as an
//! archive.

use crate::rc_hash::RC_CONSOLE_ARCADE;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use zip::ZipArchive;

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
        return extract_first_file(file);
    }
    reject_unsupported(file)?;
    if has_extension(file, "m3u") {
        let playlist = fs::read(file).map_err(|error| error.to_string())?;
        reject_unsupported(Path::new(&first_playlist_item(&playlist)))?;
    }
    Ok(Input::at(file))
}

fn has_extension(path: &Path, extension: &str) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|ext| ext.eq_ignore_ascii_case(extension))
}

fn not_supported(format: &str) -> String {
    format!("{format} files are not supported yet")
}

fn reject_unsupported(path: &Path) -> Result<(), String> {
    for (extension, format) in [("chd", "CHD"), ("7z", "7z"), ("zip", "nested zip")] {
        if has_extension(path, extension) {
            return Err(not_supported(format));
        }
    }
    Ok(())
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
            .ok_or_else(|| format!("unsafe zip entry name: {}", entry.name()))?;
        reject_unsupported(&name)?;

        let directory = TempDir::new().map_err(|error| error.to_string())?;
        let path = directory.path().join(name);
        let mut output = File::create(&path).map_err(|error| error.to_string())?;
        io::copy(&mut entry, &mut output).map_err(|error| error.to_string())?;
        return Ok(Input {
            path,
            _extracted_to: Some(directory),
        });
    }
    Err("zip file has no files".to_owned())
}
