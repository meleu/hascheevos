//! Short names for the consoles known to rcheevos.

use crate::rc_hash as ffi;

struct Console {
    id: u32,
    name: &'static str,
    aliases: &'static [&'static str],
}

const fn console(id: u32, name: &'static str, aliases: &'static [&'static str]) -> Console {
    Console { id, name, aliases }
}

const CONSOLES: &[Console] = &[
    console(ffi::RC_CONSOLE_MEGA_DRIVE, "megadrive", &["genesis"]),
    console(ffi::RC_CONSOLE_NINTENDO_64, "n64", &[]),
    console(ffi::RC_CONSOLE_SUPER_NINTENDO, "snes", &[]),
    console(ffi::RC_CONSOLE_GAMEBOY, "gb", &[]),
    console(ffi::RC_CONSOLE_GAMEBOY_ADVANCE, "gba", &[]),
    console(ffi::RC_CONSOLE_GAMEBOY_COLOR, "gbc", &[]),
    console(ffi::RC_CONSOLE_NINTENDO, "nes", &[]),
    console(ffi::RC_CONSOLE_PC_ENGINE, "pcengine", &[]),
    console(ffi::RC_CONSOLE_SEGA_CD, "segacd", &[]),
    console(ffi::RC_CONSOLE_SEGA_32X, "sega32x", &[]),
    console(ffi::RC_CONSOLE_MASTER_SYSTEM, "mastersystem", &[]),
    console(ffi::RC_CONSOLE_PLAYSTATION, "psx", &[]),
    console(ffi::RC_CONSOLE_ATARI_LYNX, "atarilynx", &[]),
    console(ffi::RC_CONSOLE_NEOGEO_POCKET, "ngp", &["ngpc"]),
    console(ffi::RC_CONSOLE_GAME_GEAR, "gamegear", &[]),
    console(ffi::RC_CONSOLE_GAMECUBE, "gamecube", &[]),
    console(ffi::RC_CONSOLE_ATARI_JAGUAR, "atarijaguar", &[]),
    console(ffi::RC_CONSOLE_NINTENDO_DS, "nds", &[]),
    console(ffi::RC_CONSOLE_WII, "wii", &[]),
    console(ffi::RC_CONSOLE_WII_U, "wiiu", &[]),
    console(ffi::RC_CONSOLE_PLAYSTATION_2, "ps2", &[]),
    console(ffi::RC_CONSOLE_XBOX, "xbox", &[]),
    console(ffi::RC_CONSOLE_MAGNAVOX_ODYSSEY2, "odyssey2", &[]),
    console(ffi::RC_CONSOLE_POKEMON_MINI, "pokemini", &[]),
    console(ffi::RC_CONSOLE_ATARI_2600, "atari2600", &[]),
    console(ffi::RC_CONSOLE_MS_DOS, "dos", &[]),
    console(ffi::RC_CONSOLE_ARCADE, "arcade", &["fbneo", "fba"]),
    console(ffi::RC_CONSOLE_VIRTUAL_BOY, "virtualboy", &[]),
    console(ffi::RC_CONSOLE_MSX, "msx", &[]),
    console(ffi::RC_CONSOLE_COMMODORE_64, "c64", &[]),
    console(ffi::RC_CONSOLE_ZX81, "zx81", &[]),
    console(ffi::RC_CONSOLE_ORIC, "oric", &[]),
    console(ffi::RC_CONSOLE_SG1000, "sg-1000", &[]),
    console(ffi::RC_CONSOLE_VIC20, "vic20", &[]),
    console(ffi::RC_CONSOLE_AMIGA, "amiga", &[]),
    console(ffi::RC_CONSOLE_ATARI_ST, "atarist", &[]),
    console(ffi::RC_CONSOLE_AMSTRAD_PC, "amstradcpc", &[]),
    console(ffi::RC_CONSOLE_APPLE_II, "apple2", &[]),
    console(ffi::RC_CONSOLE_SATURN, "saturn", &[]),
    console(ffi::RC_CONSOLE_DREAMCAST, "dreamcast", &[]),
    console(ffi::RC_CONSOLE_PSP, "psp", &[]),
    console(ffi::RC_CONSOLE_CDI, "cdi", &[]),
    console(ffi::RC_CONSOLE_3DO, "3do", &[]),
    console(ffi::RC_CONSOLE_COLECOVISION, "coleco", &[]),
    console(ffi::RC_CONSOLE_INTELLIVISION, "intellivision", &[]),
    console(ffi::RC_CONSOLE_VECTREX, "vectrex", &[]),
    console(ffi::RC_CONSOLE_PC8800, "pc88", &[]),
    console(ffi::RC_CONSOLE_PC9800, "pc98", &[]),
    console(ffi::RC_CONSOLE_PCFX, "pcfx", &[]),
    console(ffi::RC_CONSOLE_ATARI_5200, "atari5200", &[]),
    console(ffi::RC_CONSOLE_ATARI_7800, "atari7800", &[]),
    console(ffi::RC_CONSOLE_X68K, "x68000", &[]),
    console(
        ffi::RC_CONSOLE_WONDERSWAN,
        "wonderswan",
        &["wonderswancolor"],
    ),
    console(ffi::RC_CONSOLE_CASSETTEVISION, "cassettevision", &[]),
    console(
        ffi::RC_CONSOLE_SUPER_CASSETTEVISION,
        "supercassettevision",
        &[],
    ),
    console(ffi::RC_CONSOLE_NEO_GEO_CD, "neogeocd", &[]),
    console(ffi::RC_CONSOLE_FAIRCHILD_CHANNEL_F, "channelf", &[]),
    console(ffi::RC_CONSOLE_FM_TOWNS, "fmtowns", &[]),
    console(ffi::RC_CONSOLE_ZX_SPECTRUM, "zxspectrum", &[]),
    console(ffi::RC_CONSOLE_GAME_AND_WATCH, "gameandwatch", &[]),
    console(ffi::RC_CONSOLE_NOKIA_NGAGE, "ngage", &[]),
    console(ffi::RC_CONSOLE_NINTENDO_3DS, "3ds", &[]),
    console(ffi::RC_CONSOLE_SUPERVISION, "supervision", &[]),
    console(ffi::RC_CONSOLE_SHARPX1, "x1", &[]),
    console(ffi::RC_CONSOLE_TIC80, "tic80", &[]),
    console(ffi::RC_CONSOLE_THOMSONTO8, "thomsonto8", &[]),
    console(ffi::RC_CONSOLE_PC6000, "pc6000", &[]),
    console(ffi::RC_CONSOLE_PICO, "pico", &[]),
    console(ffi::RC_CONSOLE_MEGADUCK, "megaduck", &[]),
    console(ffi::RC_CONSOLE_ZEEBO, "zeebo", &[]),
    console(ffi::RC_CONSOLE_ARDUBOY, "arduboy", &[]),
    console(ffi::RC_CONSOLE_WASM4, "wasm4", &[]),
    console(ffi::RC_CONSOLE_ARCADIA_2001, "arcadia2001", &[]),
    console(ffi::RC_CONSOLE_INTERTON_VC_4000, "vc4000", &[]),
    console(
        ffi::RC_CONSOLE_ELEKTOR_TV_GAMES_COMPUTER,
        "tvgamescomputer",
        &[],
    ),
    console(ffi::RC_CONSOLE_PC_ENGINE_CD, "pcenginecd", &[]),
    console(ffi::RC_CONSOLE_ATARI_JAGUAR_CD, "atarijaguarcd", &[]),
    console(ffi::RC_CONSOLE_NINTENDO_DSI, "dsi", &[]),
    console(ffi::RC_CONSOLE_TI83, "ti83", &[]),
    console(ffi::RC_CONSOLE_UZEBOX, "uzebox", &[]),
    console(ffi::RC_CONSOLE_FAMICOM_DISK_SYSTEM, "fds", &[]),
    console(ffi::RC_CONSOLE_PLAYSTATION_3, "ps3", &[]),
];

/// What to say about a value that isn't a console.
pub const UNKNOWN: &str = "unknown console; see `--help` for the list of names";

/// Finds a console by name, alias or numeric ID, ignoring case.
pub fn parse(value: &str) -> Option<u32> {
    let value = value.to_lowercase();
    CONSOLES
        .iter()
        .find(|console| {
            console.name == value
                || console.aliases.contains(&value.as_str())
                || value.parse() == Ok(console.id)
        })
        .map(|console| console.id)
}

/// Finds a console by name or alias, ignoring case, for a value where a number
/// can't mean an ID.
pub fn parse_name(value: &str) -> Option<u32> {
    parse(value).filter(|_| value.parse::<u32>().is_err())
}

/// The short name of a console, or "unknown" for an ID outside the table.
pub fn name(id: u32) -> &'static str {
    CONSOLES
        .iter()
        .find(|console| console.id == id)
        .map_or("unknown", |console| console.name)
}

/// Lists the valid names in alphabetical order, wrapped for `--help`.
pub fn help_list() -> String {
    let mut names: Vec<String> = CONSOLES
        .iter()
        .map(|console| match console.aliases {
            [] => console.name.to_owned(),
            aliases => format!("{} ({})", console.name, aliases.join(", ")),
        })
        .collect();
    names.sort_unstable();

    let mut help = String::new();
    let mut line = String::new();
    for name in names {
        if line.len() + name.len() + 2 > 76 {
            help += &format!("  {}\n", line.trim_end());
            line.clear();
        }
        line += &format!("{name}, ");
    }
    help += &format!("  {}", line.trim_end().trim_end_matches(','));
    help
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn header_consoles() -> BTreeMap<u32, String> {
        include_str!("../vendor/rcheevos/include/rc_consoles.h")
            .lines()
            .filter_map(|line| {
                let (name, id) = line.trim().trim_end_matches(',').split_once(" = ")?;
                Some((
                    id.parse().ok()?,
                    name.strip_prefix("RC_CONSOLE_")?.to_owned(),
                ))
            })
            .filter(|(_, name)| {
                !["UNKNOWN", "HUBS", "EVENTS", "STANDALONE"].contains(&name.as_str())
            })
            .collect()
    }

    #[test]
    fn every_real_console_in_the_header_has_exactly_one_entry() {
        let header: Vec<u32> = header_consoles().into_keys().collect();
        assert_eq!(header, (1..=82).collect::<Vec<_>>());

        let mut table: Vec<u32> = CONSOLES.iter().map(|console| console.id).collect();
        table.sort_unstable();
        assert_eq!(table, header);
    }

    #[test]
    fn names_and_aliases_are_unique_and_lowercase() {
        let mut seen = std::collections::HashSet::new();
        for console in CONSOLES {
            for name in std::iter::once(&console.name).chain(console.aliases) {
                assert_eq!(*name, name.to_lowercase());
                assert!(seen.insert(*name), "duplicate name {name}");
            }
        }
    }

    #[test]
    fn script_system_names_keep_their_ids() {
        // From SYSTEMS_INFO in bin/hascheevos.sh.
        let systems = [
            ("megadrive", 1),
            ("n64", 2),
            ("snes", 3),
            ("gb", 4),
            ("gba", 5),
            ("gbc", 6),
            ("nes", 7),
            ("pcengine", 8),
            ("segacd", 9),
            ("sega32x", 10),
            ("mastersystem", 11),
            ("psx", 12),
            ("atarilynx", 13),
            ("ngp", 14),
            ("gamegear", 15),
            ("atarijaguar", 17),
            ("nds", 18),
            ("pokemini", 24),
            ("atari2600", 25),
            ("arcade", 27),
            ("virtualboy", 28),
            ("sg-1000", 33),
            ("coleco", 44),
            ("atari7800", 51),
            ("wonderswan", 53),
        ];
        for (name, id) in systems {
            assert_eq!(parse(name), Some(id), "{name}");
        }
    }

    #[test]
    fn aliases_resolve_to_their_console() {
        let aliases = [
            ("genesis", 1),
            ("ngpc", 14),
            ("wonderswancolor", 53),
            ("fbneo", 27),
            ("fba", 27),
            ("fds", 81),
            ("pcenginecd", 76),
        ];
        for (name, id) in aliases {
            assert_eq!(parse(name), Some(id), "{name}");
        }
    }

    #[test]
    fn matching_ignores_case() {
        assert_eq!(parse("SNES"), Some(3));
        assert_eq!(parse("Genesis"), Some(1));
    }

    #[test]
    fn numeric_ids_are_accepted_only_for_real_consoles() {
        assert_eq!(parse("7"), Some(7));
        assert_eq!(parse("82"), Some(82));
        for value in ["0", "83", "100", "-1"] {
            assert_eq!(parse(value), None, "{value}");
        }
    }

    #[test]
    fn unknown_names_are_rejected() {
        assert_eq!(parse("notaconsole"), None);
        assert_eq!(parse(""), None);
    }
}
