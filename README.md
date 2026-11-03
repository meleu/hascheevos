# hascheevos

A way to check if your ROM is OK for RetroAchievements.

`hascheevos` calculates a ROM's hash with [rcheevos](https://github.com/RetroAchievements/rcheevos), looks it up in data from the RetroAchievements API, and tells you whether the game has published achievements.

```
$ hascheevos check "roms/Sonic the Hedgehog (USA, Europe).zip" "roms/Qix (USA).zip"
roms/Sonic the Hedgehog (USA, Europe).zip: Sonic the Hedgehog: 23 achievements
roms/Sonic the Hedgehog (USA, Europe).zip
roms/Qix (USA).zip: hash not found
```

**`.chd` files are unsupported.** They're reported as an error, not as having no achievements.

## Installing

`hascheevos` is a single binary for Linux. Building it takes a [Rust toolchain](https://rustup.rs/), `git` and a C compiler:

```
git clone --recurse-submodules https://github.com/meleu/hascheevos
cd hascheevos
cargo install --path .
```

## Setting your API key

Everything but `hash` needs a free API key. Copy your **Web API Key** from your [RetroAchievements settings page](https://retroachievements.org/settings) and put this in your `~/.bashrc`:

```
export RA_API_KEY="your-web-api-key"
```

## Commands

Results go to stdout. Progress, game titles, notes and warnings go to stderr. `--console` takes a console's name or ID; run `hascheevos check --help` for the names.

### `check`: say which ROMs have achievements

```
hascheevos check [OPTIONS] [FILE]...
```

Prints the path of each ROM with achievements. Zipped and `.7z` ROMs work without extracting them. A hash marked as anything other than `compatible` or `untested` doesn't count, and you get a warning saying why.

| Option | What it does |
| --- | --- |
| `--console <NAME\|ID>` | Treats every listed file as a ROM of this console. Without it, each console the extension could belong to is tried, and files with an unrecognised extension are skipped. |
| `--refresh` | Downloads each console's list even if the cached one would do. |
| `--tab-output` | Prints `name<TAB>gameID<TAB>x` for pasting into a spreadsheet. The `x` is blank when there are no achievements. |
| `--copy-roms-to <DIR>` | Copies ROMs with achievements to `DIR/<console>/`, overwriting existing files. |
| `--system <NAME,...>` | Scans these systems under `~/RetroPie/roms`, or every one with `all`. |
| `--collection` | Adds ROMs with achievements to an EmulationStation collection per system. |
| `--single-collection` | Adds ROMs with achievements to one EmulationStation collection. |

Arcade hashes are calculated from the file name, so arcade ROMs need `--console arcade`.

`.bin`, `.rom`, `.tap` and `.nds` files can belong to several consoles, but only the first is tried. If such a file shows no achievements you get a note suggesting `--console`.

#### EmulationStation collections

```
hascheevos check --collection --system all
```

The files go in `~/.emulationstation/collections`, and a ROM outside `~/RetroPie/roms` can't go in one. A ROM that no longer has achievements isn't removed from them.

To see them, restart EmulationStation and enable them under **MAIN MENU** -> **GAME COLLECTIONS SETTINGS** -> **CUSTOM GAME COLLECTIONS**. [Custom collections](https://github.com/retropie/retropie-setup/wiki/EmulationStation#custom-collections) need EmulationStation 2.6.0 or newer.

### `game`: say whether games have achievements

```
hascheevos game [--refresh] <ID>...
```

Prints each game ID that has achievements.

### `md5`: say whether a hash has achievements

```
hascheevos md5 --console <NAME|ID> [--refresh] <MD5>
```

Prints the hash if it has achievements.

### `hash`: print a file's hashes

```
hascheevos hash [--console <NAME|ID>] [-v] <FILE>
```

Prints the hash for the named console or, without one, the hashes for the consoles the file could belong to. `-v` reports rcheevos progress on stderr. This command needs no API key and no network.

## Exit codes

`check`, `game` and `md5` work like `grep`:

| Code | Meaning |
| --- | --- |
| `0` | At least one item has achievements. |
| `1` | None do. |
| `2` | Something failed, even if other items have achievements. |

`hash` exits with `0` when it prints a hash and `1` when it fails. Every command exits with `2` for invalid arguments.

---

**What's the point of creating this tool?!**

- <https://retropie.org.uk/forum/topic/11859/what-about-adding-a-cheevos-flag-in-gamelist-xml>
- <http://retroachievements.org/viewtopic.php?t=5025>
