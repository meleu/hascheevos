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

/// A hash and a console whose list it can be looked up in.
pub struct ConsoleHash {
    // Nothing reads this until `check` looks hashes up per console.
    #[allow(dead_code)]
    pub console: u32,
    pub hash: String,
}

/// The result of hashing one file.
pub struct Report {
    /// Every hash, in the order rcheevos produced them. A hash that several
    /// consoles share appears once per console. After a fallback, the console
    /// is only the one rcheevos borrowed to hash the whole file.
    pub hashes: Vec<ConsoleHash>,
    /// Errors reported by rcheevos, including those of candidate consoles
    /// that didn't lead to a hash.
    pub errors: Vec<String>,
}

impl Report {
    /// The distinct hashes, in the order rcheevos produced them.
    pub fn distinct_hashes(&self) -> Vec<&str> {
        let mut hashes = Vec::new();
        for entry in &self.hashes {
            if !hashes.contains(&entry.hash.as_str()) {
                hashes.push(&entry.hash);
            }
        }
        hashes
    }

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
    let mut hashes = Vec::new();
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

        let mut record = |console: u32, hash: &[c_char; 33]| {
            let hash = CStr::from_ptr(hash.as_ptr()).to_string_lossy().into_owned();
            hashes.push(ConsoleHash { console, hash });
        };
        match console {
            Some(id) => {
                if rc_hash_generate(hash.as_mut_ptr(), id, iterator) != 0 {
                    record(id, &hash);
                }
            }
            None => {
                while rc_hash_iterate(hash.as_mut_ptr(), iterator) != 0 {
                    // rcheevos moves `index` past a console before hashing
                    // for it, and past any that fail, so a hash always
                    // belongs to the entry just before `index`.
                    let console = u32::from(iterator.consoles[iterator.index as usize - 1]);
                    record(console, &hash);
                    // rcheevos only lists Sega CD as a candidate, and uses it
                    // for Saturn discs too.
                    if console == RC_CONSOLE_SEGA_CD {
                        record(RC_CONSOLE_SATURN, &hash);
                    }
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

#[cfg(test)]
mod tests {
    use super::*;

    // From RAHasher 1.8.4, like the expected hashes in the CLI tests.
    const SNES_SFC: &str = "6556112372898c69e1de0bf689d8db26";
    const PSX: &str = "090500155aa193ff78586fa532a36023";
    const SEGA_CD: &str = "206d293cdd4e514fffdd4057d7f7c9fa";
    // From md5sum.
    const SMALL_FILE: &str = "351f58718c9ef422df35ec121ac53c3f";

    fn console_hashes(report: &Report) -> Vec<(u32, &str)> {
        report
            .hashes
            .iter()
            .map(|entry| (entry.console, entry.hash.as_str()))
            .collect()
    }

    fn hash_testdata(path: &str, console: Option<u32>) -> Report {
        hash_file(Path::new(path), console, false)
    }

    fn hash_small_file(name: &str) -> Report {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(name);
        std::fs::write(&path, "hascheevos").unwrap();
        hash_file(&path, None, false)
    }

    #[test]
    fn a_named_console_owns_its_hash() {
        let report = hash_testdata(
            "testdata/snes/FakeHeaderedROM.sfc",
            Some(RC_CONSOLE_SUPER_NINTENDO),
        );
        assert_eq!(
            console_hashes(&report),
            [(RC_CONSOLE_SUPER_NINTENDO, SNES_SFC)]
        );
    }

    #[test]
    fn a_console_found_from_the_extension_owns_its_hash() {
        let report = hash_testdata("testdata/snes/FakeHeaderedROM.sfc", None);
        assert_eq!(
            console_hashes(&report),
            [(RC_CONSOLE_SUPER_NINTENDO, SNES_SFC)]
        );
    }

    #[test]
    fn each_hash_stays_with_its_console_when_candidates_fail_in_between() {
        let report = hash_testdata("testdata/disc/both.cue", None);
        assert_eq!(
            console_hashes(&report),
            [
                (RC_CONSOLE_PLAYSTATION, PSX),
                (RC_CONSOLE_SEGA_CD, SEGA_CD),
                (RC_CONSOLE_SATURN, SEGA_CD),
            ]
        );
    }

    #[test]
    fn a_named_sega_cd_console_is_not_also_saturn() {
        let report = hash_testdata("testdata/disc/segacd.iso", Some(RC_CONSOLE_SEGA_CD));
        assert_eq!(console_hashes(&report), [(RC_CONSOLE_SEGA_CD, SEGA_CD)]);
    }

    #[test]
    fn a_hash_shared_by_candidates_is_kept_for_each_console() {
        let report = hash_small_file("disk.dsk");
        assert_eq!(
            console_hashes(&report),
            [
                (RC_CONSOLE_MSX, SMALL_FILE),
                (RC_CONSOLE_AMSTRAD_PC, SMALL_FILE),
                (RC_CONSOLE_ZX_SPECTRUM, SMALL_FILE),
                (RC_CONSOLE_APPLE_II, SMALL_FILE),
            ]
        );
    }

    #[test]
    fn a_hash_shared_by_candidates_is_distinct_only_once() {
        assert_eq!(hash_small_file("disk.dsk").distinct_hashes(), [SMALL_FILE]);
    }

    #[test]
    fn a_fallback_is_still_detectable() {
        assert!(hash_small_file("notes.unknown").used_fallback());
    }
}
