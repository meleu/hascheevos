//! Adds ROMs to EmulationStation's custom collections.

use std::collections::BTreeSet;
use std::ffi::OsString;
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
        // A file directly in the ROMs directory belongs to no system.
        components.next().ok_or_else(not_in_a_system)?;
        let name = match self {
            Self::PerSystem => {
                let mut name = OsString::from("custom-achievements ");
                name.push(system);
                name.push(".cfg");
                name
            }
            Self::Single => OsString::from("custom-achievements.cfg"),
        };
        Ok(Entry {
            collection_file: home.join(".emulationstation/collections").join(name),
            rom: roms_dir.join(&in_roms_dir),
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

/// A path's part inside `directory`, if it's in there. Going by the names keeps
/// a linked directory like RetroPie's `genesis` as it is, but the working directory
/// has no links in it, so a relative path needs them resolved to be recognised.
fn in_directory(path: &Path, directory: &Path) -> io::Result<Option<PathBuf>> {
    let inside =
        |path: &Path, directory: &Path| path.strip_prefix(directory).ok().map(Path::to_path_buf);
    if let Some(found) = inside(&normal_absolute(path)?, directory) {
        return Ok(Some(found));
    }
    Ok(path
        .canonicalize()
        .and_then(|path| Ok((path, directory.canonicalize()?)))
        .ok()
        .and_then(|(path, directory)| inside(&path, &directory)))
}

/// A path from the root with no `.` or `..`, worked out without following links
/// so a linked directory like RetroPie's `genesis` keeps its name.
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
