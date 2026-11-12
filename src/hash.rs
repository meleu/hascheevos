//! Console hashes of a ROM, computed in-process by the vendored rcheevos
//! library (ADR-0001). Everything about rcheevos is hidden behind
//! [`console_hashes`] and [`ConsoleHash::console_name`].

use std::cell::Cell;
use std::ffi::{c_char, c_void, CStr, CString};
use std::fs::{self, File};
use std::io::Read;
use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::path::Path;

use anyhow::{bail, Context, Result};

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code
)]
mod rcheevos {
    include!(concat!(env!("OUT_DIR"), "/rcheevos.rs"));
}

/// A hash together with the console whose official method produced it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConsoleHash {
    /// Numeric console ID, as RetroAchievements defines it.
    pub console: u32,
    /// The 32-hex-character hash.
    pub hash: String,
}

impl ConsoleHash {
    /// The console's name, as rcheevos spells it.
    pub fn console_name(&self) -> String {
        // SAFETY: `rc_console_name` accepts any ID and returns a pointer to a
        // static NUL-terminated string.
        let name = unsafe { CStr::from_ptr(rcheevos::rc_console_name(self.console)) };
        name.to_string_lossy().into_owned()
    }
}

/// Consoles whose official method reads a `.zip` itself rather than the ROM
/// inside it, per the pinned rcheevos (v12.5.0): arcade hashes the zip's
/// filename without its extension, prefixed by its folder when that folder
/// names an FBNeo subsystem (see `rc_hash_arcade` in `hash_rom.c`), and
/// MS-DOS hashes the files in the zip. A zip given with one of these consoles
/// reaches rcheevos untouched. (RAHasher also lists Zeebo, but rcheevos has no
/// hash method for Zeebo at all.)
const CONSOLES_HASHING_THE_ZIP_ITSELF: &[u32] = &[
    26, // RC_CONSOLE_MS_DOS
    27, // RC_CONSOLE_ARCADE
];

/// Consoles whose official method reads a `.7z` itself rather than the ROM
/// inside it, per the pinned rcheevos (v12.5.0): only arcade, which hashes
/// the 7z's filename as it does a zip's. MS-DOS parses zips only.
const CONSOLES_HASHING_THE_7Z_ITSELF: &[u32] = &[
    27, // RC_CONSOLE_ARCADE
];

/// Returns the console hashes of `rom`, in rcheevos' order.
///
/// With a console, the list has exactly one element: the hash produced by
/// that console's official method. Without, it has one element per console
/// for which rcheevos produced a hash, in the order rcheevos tried them; the
/// same hash under several consoles is kept once per console. Producing no
/// hash at all is an error, never an empty list.
///
/// A `.zip` or `.7z` is unpacked in memory and its first entry hashed under
/// that entry's name, unless the console's official method reads the archive
/// itself.
///
/// Without a console, a ROM whose extension rcheevos maps to no console is
/// refused, rather than hashed by rcheevos' full-file fallback; for an
/// archive, the extension is that of the entry hashed.
///
/// rcheevos' error messages always go to stderr. With `verbose`, so does its
/// trace of which consoles it tried and what it read.
pub fn console_hashes(rom: &Path, console: Option<u32>, verbose: bool) -> Result<Vec<ConsoleHash>> {
    let metadata = fs::metadata(rom).with_context(|| format!("cannot read '{}'", rom.display()))?;
    if !metadata.is_file() {
        bail!("'{}' is not a file", rom.display());
    }

    let content = match Archive::of(rom) {
        Some(archive) if !console.is_some_and(|console| archive.is_hashed_whole_by(console)) => {
            archive.first_entry(rom)?
        }
        _ => Content::File(rom),
    };

    let mut iterator = HashIterator::new(&content, verbose)?;
    let Some(console) = console else {
        let console_hashes: Vec<ConsoleHash> = iterator.by_ref().collect();
        if iterator.found_no_console_mapping() {
            bail!(
                "rcheevos maps no console to the extension of {}; \
                 give --console to hash it anyway",
                content.describe(rom)
            );
        }
        if console_hashes.is_empty() {
            bail!("no console produced a hash for '{}'", rom.display());
        }
        return Ok(console_hashes);
    };

    match iterator.generate(console) {
        Some(hash) => Ok(vec![ConsoleHash { console, hash }]),
        None => bail!("console {console} produced no hash for '{}'", rom.display()),
    }
}

/// An archive format `hc` unpacks, judged by the ROM's extension in any
/// letter case.
#[derive(Debug, Clone, Copy)]
enum Archive {
    /// A `.zip`.
    Zip,
    /// A `.7z`.
    SevenZip,
}

impl Archive {
    /// The archive format of `rom`, if it is an archive.
    fn of(rom: &Path) -> Option<Self> {
        let extension = rom.extension()?;
        [Archive::Zip, Archive::SevenZip]
            .into_iter()
            .find(|archive| extension.eq_ignore_ascii_case(archive.extension()))
    }

    /// The format's extension, which also names it in messages.
    fn extension(self) -> &'static str {
        match self {
            Archive::Zip => "zip",
            Archive::SevenZip => "7z",
        }
    }

    /// Whether `console`'s official method reads an archive of this format
    /// itself, so it must reach rcheevos untouched.
    fn is_hashed_whole_by(self, console: u32) -> bool {
        match self {
            Archive::Zip => CONSOLES_HASHING_THE_ZIP_ITSELF,
            Archive::SevenZip => CONSOLES_HASHING_THE_7Z_ITSELF,
        }
        .contains(&console)
    }

    /// The first entry of the archive at `path`, unpacked in memory.
    ///
    /// When the archive holds several files, taking the first is a guess, so
    /// a warning naming it goes to stderr.
    fn first_entry(self, path: &Path) -> Result<Content<'static>> {
        let format = self.extension();
        let entry = match self {
            Archive::Zip => first_zip_entry(path),
            Archive::SevenZip => first_7z_entry(path),
        }
        .with_context(|| format!("cannot read {format} '{}'", path.display()))?;
        let (entry, files) =
            entry.with_context(|| format!("{format} '{}' is empty", path.display()))?;
        if files > 1 {
            eprintln!(
                "hc: warning: {format} '{}' holds {files} files; guessing the first, '{}', is the ROM",
                path.display(),
                entry.name()
            );
        }
        Ok(entry)
    }
}

/// The first entry of the zip at `zip` and how many files the zip holds, or
/// `None` if it has no entries.
fn first_zip_entry(zip: &Path) -> Result<Option<(Content<'static>, usize)>> {
    let mut archive = zip::ZipArchive::new(File::open(zip)?)?;
    if archive.is_empty() {
        return Ok(None);
    }
    let files = archive
        .file_names_raw()
        .filter(|name| !name.ends_with(b"/"))
        .count();
    let mut entry = archive.by_index(0)?;
    let name = entry.name()?.into_owned();
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    Ok(Some((Content::InMemory { name, bytes }, files)))
}

/// The first entry of the 7z at `seven_zip` and how many files the 7z holds,
/// or `None` if it has no entries.
fn first_7z_entry(seven_zip: &Path) -> Result<Option<(Content<'static>, usize)>> {
    let mut archive =
        sevenz_rust2::ArchiveReader::open(seven_zip, sevenz_rust2::Password::empty())?;
    let entries = &archive.archive().files;
    let Some(entry) = entries.first() else {
        return Ok(None);
    };
    let name = entry.name().to_owned();
    let files = entries.iter().filter(|entry| !entry.is_directory()).count();
    let bytes = archive.read_file(&name)?;
    Ok(Some((Content::InMemory { name, bytes }, files)))
}

/// What rcheevos hashes. Its name's extension drives which consoles rcheevos
/// considers plausible.
enum Content<'rom> {
    /// A file rcheevos reads itself.
    File(&'rom Path),
    /// Bytes already in memory, named by the file they came from.
    InMemory { name: String, bytes: Vec<u8> },
}

impl Content<'_> {
    /// The name rcheevos sees: a path, or an archive entry's name.
    fn name(&self) -> String {
        match self {
            Content::File(path) => path.display().to_string(),
            Content::InMemory { name, .. } => name.clone(),
        }
    }

    /// The content as named in messages: quoted, and for an archive entry,
    /// followed by the archive it came from, `rom`.
    fn describe(&self, rom: &Path) -> String {
        match self {
            Content::File(path) => format!("'{}'", path.display()),
            Content::InMemory { name, .. } => format!("'{name}' in '{}'", rom.display()),
        }
    }
}

/// An initialised `rc_hash_iterator_t`, destroyed on drop. It points into the
/// bytes of the [`Content`] it was made from, so it must not outlive them.
///
/// Its `userdata` points to the boxed flag, which rcheevos' error callback
/// raises when rcheevos maps no console to the ROM's extension. The box keeps
/// the flag's address fixed while the iterator moves.
struct HashIterator<'content>(
    rcheevos::rc_hash_iterator_t,
    Box<Cell<bool>>,
    PhantomData<&'content [u8]>,
);

impl<'content> HashIterator<'content> {
    fn new(content: &'content Content, verbose: bool) -> Result<Self> {
        let name = match content {
            Content::File(path) => CString::new(path.as_os_str().as_encoded_bytes()),
            Content::InMemory { name, .. } => CString::new(name.as_str()),
        }
        .with_context(|| format!("name '{}' contains a NUL byte", content.name()))?;
        let (buffer, buffer_size) = match content {
            Content::File(_) => (std::ptr::null(), 0),
            Content::InMemory { bytes, .. } => (bytes.as_ptr(), bytes.len()),
        };

        let mut iterator = MaybeUninit::<rcheevos::rc_hash_iterator_t>::uninit();
        // SAFETY: `rc_hash_initialize_iterator` fully initialises the struct
        // (it zeroes it first) and copies `name`, which outlives the call. It
        // keeps `buffer` without copying it; `buffer` points into `content`,
        // which the `'content` lifetime keeps alive as long as the iterator.
        let mut iterator = unsafe {
            rcheevos::rc_hash_initialize_iterator(
                iterator.as_mut_ptr(),
                name.as_ptr(),
                buffer,
                buffer_size,
            );
            iterator.assume_init()
        };
        // rcheevos sends no message while initialising, so setting the
        // callbacks afterwards misses none.
        let no_console_mapping = Box::new(Cell::new(false));
        iterator.userdata = no_console_mapping.as_ref() as *const Cell<bool> as *mut c_void;
        iterator.callbacks.error_message = Some(report_error);
        if verbose {
            iterator.callbacks.verbose_message = Some(report_message);
        }
        Ok(Self(iterator, no_console_mapping, PhantomData))
    }

    /// Whether rcheevos, while iterating, found no console mapped to the ROM's
    /// extension and fell back to a full-file hash.
    fn found_no_console_mapping(&self) -> bool {
        self.1.get()
    }

    /// The hash of the ROM by one console's official method, if it has one.
    fn generate(&self, console: u32) -> Option<String> {
        let mut hash: [c_char; 33] = [0; 33];
        // SAFETY: `hash` has room for 32 characters and the terminating NUL,
        // and the iterator is initialised and alive.
        let generated =
            unsafe { rcheevos::rc_hash_generate(hash.as_mut_ptr(), console, &self.0) } != 0;
        generated.then(|| hash_string(&hash))
    }
}

/// Walks the plausible consoles rcheevos chose for the ROM, yielding a console
/// hash for each console that produced one.
impl Iterator for HashIterator<'_> {
    type Item = ConsoleHash;

    fn next(&mut self) -> Option<ConsoleHash> {
        let mut hash: [c_char; 33] = [0; 33];
        // SAFETY: `hash` has room for 32 characters and the terminating NUL,
        // and the iterator is initialised and alive.
        let generated = unsafe { rcheevos::rc_hash_iterate(hash.as_mut_ptr(), &mut self.0) } != 0;
        if !generated {
            return None;
        }
        // On success rcheevos has just advanced `index` past the console that
        // produced the hash.
        let index = usize::try_from(self.0.index - 1)
            .expect("rcheevos advances the index before every hash it produces");
        let console = self.0.consoles[index];
        Some(ConsoleHash {
            console: console.into(),
            hash: hash_string(&hash),
        })
    }
}

/// The hash rcheevos wrote to `hash`.
fn hash_string(hash: &[c_char; 33]) -> String {
    // SAFETY: `hash` starts zeroed and rcheevos writes it with
    // `snprintf(hash, 33, ...)`, so it always holds a NUL within its 33 bytes.
    let hash = unsafe { CStr::from_ptr(hash.as_ptr()) };
    hash.to_string_lossy().into_owned()
}

impl Drop for HashIterator<'_> {
    fn drop(&mut self) {
        // SAFETY: the iterator was initialised in `new` and is not used again.
        unsafe { rcheevos::rc_hash_destroy_iterator(&mut self.0) }
    }
}

/// How rcheevos' error message begins when it maps no console to a ROM's
/// extension, just before falling back to a full-file hash (see
/// `rc_hash_initialize_iterator_from_path` in `hash.c`). It is rcheevos' only
/// signal of that case.
const NO_CONSOLE_MAPPING: &[u8] = b"No console mapping specified for ";

/// Sends an rcheevos error message to stderr, except the one saying it maps no
/// console to the ROM's extension: that one raises the iterator's flag
/// instead, since `hc` refuses the ROM rather than take the fallback.
unsafe extern "C" fn report_error(
    message: *const c_char,
    iterator: *const rcheevos::rc_hash_iterator_t,
) {
    // SAFETY: rcheevos passes a NUL-terminated string valid for this call,
    // and an iterator whose `userdata` it copied from the one `HashIterator`
    // set up, pointing to that iterator's live flag.
    unsafe {
        if !message.is_null()
            && !iterator.is_null()
            && CStr::from_ptr(message)
                .to_bytes()
                .starts_with(NO_CONSOLE_MAPPING)
        {
            let flag = (*iterator).userdata as *const Cell<bool>;
            if !flag.is_null() {
                (*flag).set(true);
                return;
            }
        }
        report_message(message, iterator);
    }
}

/// Sends an rcheevos error or verbose message to stderr.
unsafe extern "C" fn report_message(
    message: *const c_char,
    _iterator: *const rcheevos::rc_hash_iterator_t,
) {
    if message.is_null() {
        return;
    }
    // SAFETY: rcheevos passes a NUL-terminated string valid for this call.
    let message = unsafe { CStr::from_ptr(message) };
    eprintln!("rcheevos: {}", message.to_string_lossy());
}
