//! Safe wrapper around the rcheevos `rc_hash` iterator API.

#[allow(non_camel_case_types, non_upper_case_globals, dead_code)]
#[rustfmt::skip]
mod bindings;

pub use bindings::*;

use std::ffi::{CStr, CString, c_char, c_void};
use std::io::Write;
use std::mem::MaybeUninit;
use std::path::Path;

/// What rcheevos printed when the extension has no console mapping.
const NO_CONSOLE_MAPPING: &str = "No console mapping specified";

/// The result of hashing one file.
pub struct Report {
    /// Distinct hashes, in the order rcheevos produced them.
    pub hashes: Vec<String>,
    /// Errors reported by rcheevos, including those of candidate consoles
    /// that didn't lead to a hash.
    pub errors: Vec<String>,
}

impl Report {
    /// Whether rcheevos fell back to a whole-file hash.
    pub fn used_fallback(&self) -> bool {
        self.errors
            .iter()
            .any(|error| error.contains(NO_CONSOLE_MAPPING))
    }
}

struct Messages {
    verbose: bool,
    errors: Vec<String>,
}

unsafe fn message_text(message: *const c_char) -> String {
    // SAFETY: rcheevos passes a NUL-terminated string that outlives the call.
    unsafe { CStr::from_ptr(message) }
        .to_string_lossy()
        .into_owned()
}

// The callbacks must not unwind into C, so they ignore stderr write failures.
unsafe extern "C" fn on_verbose(message: *const c_char, _: *const rc_hash_iterator) {
    let text = unsafe { message_text(message) };
    let _ = writeln!(std::io::stderr(), "{text}");
}

unsafe extern "C" fn on_error(message: *const c_char, iterator: *const rc_hash_iterator) {
    let text = unsafe { message_text(message) };
    // SAFETY: `hash_file` points userdata at a `Messages` that outlives the iterator.
    let messages = unsafe { &mut *((*iterator).userdata as *mut Messages) };
    if messages.verbose {
        let _ = writeln!(std::io::stderr(), "{text}");
    }
    messages.errors.push(text);
}

/// Hashes `path` as `console`, or as every candidate console for its extension.
pub fn hash_file(path: &Path, console: Option<u32>, verbose: bool) -> Report {
    let c_path = CString::new(path.as_os_str().as_encoded_bytes()).expect("paths have no NUL");
    let mut messages = Messages {
        verbose,
        errors: Vec::new(),
    };
    let mut hashes: Vec<String> = Vec::new();
    let mut hash = [0 as c_char; 33];

    // SAFETY: rc_hash_initialize_iterator clears the iterator before anything
    // reads it, and `messages` and `c_path` outlive every use of the iterator.
    unsafe {
        let mut uninit = MaybeUninit::<rc_hash_iterator>::uninit();
        rc_hash_initialize_iterator(uninit.as_mut_ptr(), c_path.as_ptr(), std::ptr::null(), 0);
        let iterator = uninit.assume_init_mut();

        iterator.userdata = (&raw mut messages).cast::<c_void>();
        iterator.callbacks.error_message = Some(on_error);
        if verbose {
            iterator.callbacks.verbose_message = Some(on_verbose);
        }
        rc_hash_get_default_cdreader(&mut iterator.callbacks.cdreader);

        let mut record = |hash: &[c_char; 33]| {
            let text = CStr::from_ptr(hash.as_ptr()).to_string_lossy().into_owned();
            if !hashes.contains(&text) {
                hashes.push(text);
            }
        };
        match console {
            Some(id) => {
                if rc_hash_generate(hash.as_mut_ptr(), id, iterator) != 0 {
                    record(&hash);
                }
            }
            None => {
                while rc_hash_iterate(hash.as_mut_ptr(), iterator) != 0 {
                    record(&hash);
                }
            }
        }

        rc_hash_destroy_iterator(iterator);
    }

    Report {
        hashes,
        errors: messages.errors,
    }
}
