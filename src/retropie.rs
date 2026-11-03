//! Finds the ROMs of whole systems under `~/RetroPie/roms`.

use crate::{console, input, rc_hash};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

/// ROM extensions rcheevos has no console for, from the script's system table.
const EXTRA_ROM_EXTENSIONS: [&str; 11] = [
    "32x", "ccd", "exe", "fba", "gen", "j64", "mgd", "ngp", "smd", "toc", "ws",
];

/// A value of `--system`.
#[derive(Clone)]
pub enum System {
    /// Every directory matching a known console.
    All,
    Named {
        directory: String,
        console: u32,
    },
}

pub fn parse_system(value: &str) -> Result<System, String> {
    if value == "all" {
        return Ok(System::All);
    }
    let console = console::parse_name(value).ok_or_else(|| console::UNKNOWN.to_owned())?;
    Ok(System::Named {
        directory: value.to_owned(),
        console,
    })
}

/// A file found in a system's directory, whose name sets its console.
pub struct Rom {
    pub path: PathBuf,
    pub console: u32,
}

/// What came of scanning the systems.
#[derive(Default)]
pub struct Scan {
    pub roms: Vec<Rom>,
    pub any_failed: bool,
    /// The directories scanned so far, by device and inode.
    visited: HashSet<(u64, u64)>,
}

impl Scan {
    fn add_all(&mut self, roms_dir: &Path) {
        let paths = match sorted_paths(roms_dir) {
            Ok(paths) => paths,
            Err(error) => return self.fail(roms_dir, &error),
        };
        for path in paths {
            let name = path.file_name().and_then(OsStr::to_str);
            if let Some(console) = name.and_then(console::parse_name)
                && path.is_dir()
            {
                self.add_directory(&path, console);
            }
        }
    }

    fn add_directory(&mut self, directory: &Path, console: u32) {
        let paths = fs::metadata(directory).and_then(|metadata| {
            // A link can lead back up the tree, or from one system to another,
            // like RetroPie's `genesis` to `megadrive`.
            if self.visited.insert((metadata.dev(), metadata.ino())) {
                sorted_paths(directory)
            } else {
                Ok(Vec::new())
            }
        });
        let paths = match paths {
            Ok(paths) => paths,
            Err(error) => return self.fail(directory, &error),
        };
        for path in paths {
            if path.is_dir() {
                self.add_directory(&path, console);
            } else if can_be_rom(&path) {
                self.roms.push(Rom { path, console });
            } else {
                input::note_unrecognised(&path);
            }
        }
    }

    fn fail(&mut self, directory: &Path, error: &io::Error) {
        eprintln!("error: {}: {error}", directory.display());
        self.any_failed = true;
    }
}

/// Rules out what else a system's directory holds, like `gamelist.xml`. Named as
/// the console's, those would be hashed, and fail for disc consoles.
fn can_be_rom(path: &Path) -> bool {
    rc_hash::knows_extension(path)
        || EXTRA_ROM_EXTENSIONS
            .iter()
            .any(|extension| input::has_extension(path, extension))
}

fn sorted_paths(directory: &Path) -> io::Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(directory)?
        .map(|entry| Ok(entry?.path()))
        .collect::<io::Result<Vec<_>>>()?;
    paths.sort_unstable();
    Ok(paths)
}

pub fn home() -> Result<PathBuf, String> {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .ok_or_else(|| "HOME is not an absolute path".to_owned())
}

pub fn roms_dir(home: &Path) -> PathBuf {
    home.join("RetroPie/roms")
}

/// Finds the ROMs of `systems`, reporting on stderr what it can't scan.
pub fn scan(systems: &[System]) -> Scan {
    let mut scan = Scan::default();
    if systems.is_empty() {
        return scan;
    }
    let roms_dir = match home() {
        Ok(home) => roms_dir(&home),
        Err(error) => {
            eprintln!("error: {error}");
            scan.any_failed = true;
            return scan;
        }
    };
    for system in systems {
        match system {
            System::All => scan.add_all(&roms_dir),
            System::Named { directory, console } => {
                scan.add_directory(&roms_dir.join(directory), *console);
            }
        }
    }
    scan
}
