//! Adds ROMs to EmulationStation's custom collections.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use crate::retropie;

/// Which collection a ROM goes in.
#[derive(Clone, Copy)]
pub enum Layout {
    /// One collection per system.
    PerSystem,
    /// One collection covering all systems.
    Single,
}

/// A ROM's line in a collection file.
pub struct Entry {
    collection_file: PathBuf,
    rom: PathBuf,
}

impl Layout {
    /// Where a ROM goes in a collection, or why it can't go in one.
    pub fn entry(self, rom: &Path) -> Result<Entry, String> {
        let home = retropie::home()?;
        let roms_dir = retropie::roms_dir(&home);
        let in_roms_dir = in_directory(rom, &roms_dir).map_err(|error| error.to_string())?;
        let not_in_a_system = || format!("not in a system directory under {}", roms_dir.display());
        let in_roms_dir = in_roms_dir.ok_or_else(not_in_a_system)?;
        let mut components = in_roms_dir.components();
        let system = components.next().ok_or_else(not_in_a_system)?.as_os_str();
        let in_system = components.as_path();
        // A file directly in the ROMs directory belongs to no system.
        if in_system.as_os_str().is_empty() {
            return Err(not_in_a_system());
        }
        let system = link_target_in(&roms_dir, system).unwrap_or_else(|| system.to_owned());
        let name = match self {
            Self::PerSystem => {
                let mut name = OsString::from("custom-achievements ");
                name.push(&system);
                name.push(".cfg");
                name
            }
            Self::Single => OsString::from("custom-achievements.cfg"),
        };
        Ok(Entry {
            collection_file: home.join(".emulationstation/collections").join(name),
            rom: roms_dir.join(system).join(in_system),
        })
    }
}

impl Entry {
    pub fn collection_file(&self) -> &Path {
        &self.collection_file
    }

    /// Adds the ROM to its collection, keeping what the file already has, sorted
    /// and without duplicates.
    pub fn add(&self) -> io::Result<()> {
        let existing = match fs::read(&self.collection_file) {
            Ok(existing) => existing,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(error),
        };
        let mut lines: BTreeSet<&[u8]> = existing
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .collect();
        lines.insert(self.rom.as_os_str().as_encoded_bytes());
        let mut contents = Vec::new();
        for line in lines {
            contents.extend_from_slice(line);
            contents.push(b'\n');
        }
        if let Some(directory) = self.collection_file.parent() {
            fs::create_dir_all(directory)?;
        }
        // Through a rename, as being cut off halfway would lose the entries.
        let mut temporary = self.collection_file.clone().into_os_string();
        temporary.push(".tmp");
        fs::write(&temporary, contents)?;
        fs::rename(&temporary, &self.collection_file)
    }
}

/// The system a linked system's directory leads to, like `megadrive` for
/// RetroPie's `genesis`, so a ROM has one entry whichever way it's reached.
fn link_target_in(roms_dir: &Path, system: &OsStr) -> Option<OsString> {
    let target = resolved_inside(&roms_dir.join(system), roms_dir)?;
    (target.components().count() == 1).then(|| target.into_os_string())
}

/// A path's part inside `directory`, if it's in there. Going by the names keeps
/// a link to somewhere else, like a ROM folder on a USB drive, inside the directory.
/// The working directory has no links in it, though, so a relative path may need
/// them resolved to be recognised.
fn in_directory(path: &Path, directory: &Path) -> io::Result<Option<PathBuf>> {
    let normal = normal_absolute(path)?;
    Ok(normal
        .strip_prefix(directory)
        .ok()
        .map(Path::to_path_buf)
        .or_else(|| resolved_inside(path, directory)))
}

/// A path's part inside `directory` once links are resolved in both.
fn resolved_inside(path: &Path, directory: &Path) -> Option<PathBuf> {
    let path = path.canonicalize().ok()?;
    let directory = directory.canonicalize().ok()?;
    path.strip_prefix(directory).ok().map(Path::to_path_buf)
}

/// A path from the root with no `.` or `..`, worked out without following links.
fn normal_absolute(path: &Path) -> io::Result<PathBuf> {
    let mut normal = PathBuf::new();
    for component in std::path::absolute(path)?.components() {
        if component == Component::ParentDir {
            normal.pop();
        } else {
            normal.push(component);
        }
    }
    Ok(normal)
}
