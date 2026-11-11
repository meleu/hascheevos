//! Console hashes of a ROM, computed in-process by the vendored rcheevos
//! library (ADR-0001). Everything about rcheevos is hidden behind
//! [`console_hashes`].

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

/// Returns the console hashes of `rom`, in rcheevos' order.
///
/// With a console, the list has exactly one element: the hash produced by
/// that console's official method. Producing no hash at all is an error,
/// never an empty list.
pub fn console_hashes(rom: &Path, console: Option<u32>) -> Result<Vec<ConsoleHash>> {
    let Some(console) = console else {
        bail!("hashing a ROM whose console is unknown is not supported yet: pass a console ID");
    };

    let metadata =
        fs::metadata(rom).with_context(|| format!("cannot read ROM {}", rom.display()))?;
    if !metadata.is_file() {
        bail!("ROM {} is not a file", rom.display());
    }

    let iterator = HashIterator::new(rom)?;
    match iterator.generate(console) {
        Some(hash) => Ok(vec![ConsoleHash { console, hash }]),
        None => bail!(
            "console {console} produced no hash for ROM {}",
            rom.display()
        ),
    }
}

/// An initialised `rc_hash_iterator_t`, destroyed on drop.
struct HashIterator(rcheevos::rc_hash_iterator_t);

impl HashIterator {
    fn new(rom: &Path) -> Result<Self> {
        let path = CString::new(rom.as_os_str().as_encoded_bytes())
            .with_context(|| format!("ROM path {} contains a NUL byte", rom.display()))?;

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
        if !generated {
            return None;
        }
        // SAFETY: on success rcheevos wrote a NUL-terminated string to `hash`.
        let hash = unsafe { CStr::from_ptr(hash.as_ptr()) };
        Some(hash.to_string_lossy().into_owned())
    }
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
