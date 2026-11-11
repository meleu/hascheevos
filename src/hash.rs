//! Console hashes of a ROM, computed in-process by the vendored rcheevos
//! library (ADR-0001). Everything about rcheevos is hidden behind
//! [`console_hashes`] and [`ConsoleHash::console_name`].

use std::ffi::{c_char, CStr, CString};
use std::fs;
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

/// Returns the console hashes of `rom`, in rcheevos' order.
///
/// With a console, the list has exactly one element: the hash produced by
/// that console's official method. Without, it has one element per console
/// for which rcheevos produced a hash, in the order rcheevos tried them; the
/// same hash under several consoles is kept once per console. Producing no
/// hash at all is an error, never an empty list.
pub fn console_hashes(rom: &Path, console: Option<u32>) -> Result<Vec<ConsoleHash>> {
    let metadata = fs::metadata(rom).with_context(|| format!("cannot read '{}'", rom.display()))?;
    if !metadata.is_file() {
        bail!("'{}' is not a file", rom.display());
    }

    let iterator = HashIterator::new(rom)?;
    let Some(console) = console else {
        let console_hashes: Vec<ConsoleHash> = iterator.collect();
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

/// An initialised `rc_hash_iterator_t`, destroyed on drop.
struct HashIterator(rcheevos::rc_hash_iterator_t);

impl HashIterator {
    fn new(rom: &Path) -> Result<Self> {
        let path = CString::new(rom.as_os_str().as_encoded_bytes())
            .with_context(|| format!("path '{}' contains a NUL byte", rom.display()))?;

        let mut iterator = MaybeUninit::<rcheevos::rc_hash_iterator_t>::uninit();
        // SAFETY: `rc_hash_initialize_iterator` fully initialises the struct
        // (it zeroes it first) and copies `path`, which outlives the call.
        let mut iterator = unsafe {
            rcheevos::rc_hash_initialize_iterator(
                iterator.as_mut_ptr(),
                path.as_ptr(),
                std::ptr::null(),
                0,
            );
            iterator.assume_init()
        };
        iterator.callbacks.error_message = Some(report_error);
        Ok(Self(iterator))
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
impl Iterator for HashIterator {
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

impl Drop for HashIterator {
    fn drop(&mut self) {
        // SAFETY: the iterator was initialised in `new` and is not used again.
        unsafe { rcheevos::rc_hash_destroy_iterator(&mut self.0) }
    }
}

/// Sends an rcheevos error message to stderr.
unsafe extern "C" fn report_error(
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
