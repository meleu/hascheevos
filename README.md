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

`hascheevos` is a single binary for Linux and needs no other tools to run. For now you build it from source, which takes a [Rust toolchain](https://rustup.rs/), `git` and a C compiler:

```
git clone --recurse-submodules https://github.com/meleu/hascheevos
cd hascheevos
cargo install --path .
```

This puts the binary in `~/.cargo/bin`, so be sure that directory is on your `PATH`.

## Setting your API key

Everything but `hash` can need to download data, which takes a free RetroAchievements API key:

1. Sign in at [retroachievements.org](https://retroachievements.org) and copy your **Web API Key** from your [settings page](https://retroachievements.org/settings).
2. Set `RA_API_KEY` to it. You probably want to put this line at the end of your `~/.bashrc`:

```
export RA_API_KEY="your-web-api-key"
```

The key is only read from this variable. There's no option or config file for it, and `.env` files aren't loaded.

## Commands

Results go to stdout, so you can pipe them to other tools. Progress, game titles, notes and warnings go to stderr.

Wherever a command takes `--console`, you can give a console's name or its ID. Run `hascheevos check --help` for the list of names.

### `check`: say which ROMs have achievements

```
hascheevos check [OPTIONS] [FILE]...
```

Prints the path of each ROM with achievements. Zipped and `.7z` ROMs work without extracting them first. Unless you name a console, files with an unrecognised extension, like `gamelist.xml`, are skipped with a note, so `hascheevos check roms/*` is fine.

A ROM doesn't count as having achievements if its hash is marked as anything other than `compatible` or `untested`, for example when the game needs a patch applied to the original ROM. You get a warning saying why.

| Option | What it does |
| --- | --- |
| `--console <NAME\|ID>` | Says which console the listed files are for. Without it, each console the file extension could belong to is tried until one has the hash. |
| `--refresh` | Downloads each console's list even if the cached one would do. |
| `--tab-output` | Prints `name<TAB>gameID<TAB>x` for each ROM it has an answer for, for pasting into a spreadsheet. The name is the path up to the first ` (` in the file name, and the `x` is blank when there are no achievements. |
| `--copy-roms-to <DIR>` | Copies ROMs with achievements to `DIR/<console>/`, creating the directories and overwriting existing files. |
| `--system <NAME,...>` | Scans these systems under `~/RetroPie/roms`, subfolders included. |
| `--collection` | Adds ROMs with achievements to an EmulationStation collection per system. |
| `--single-collection` | Adds ROMs with achievements to one EmulationStation collection. |

Arcade hashes are calculated from the file name, so arcade ROMs need `--console arcade`:

```
hascheevos check --console arcade roms/arcade/*.zip
```

With `--console`, every listed file is treated as a ROM of that console, so list only the ROMs.

`.bin`, `.rom`, `.tap` and `.nds` files can belong to several consoles, but only the first is tried. If such a file shows no achievements you get a note suggesting `--console`.

#### Copying ROMs that have achievements

The console's subdirectory is worked out for each ROM, so this creates directories like `cheevos_roms/megadrive`:

```
hascheevos check --copy-roms-to cheevos_roms /path/to/megadrive/roms/*
```

#### RetroPie: checking whole systems

```
hascheevos check --system nes,snes
hascheevos check --system all
```

A directory's name sets its console, and `all` scans every directory under `~/RetroPie/roms` named after a console. You can list files as well, but `--console` only applies to those files.

#### RetroPie: EmulationStation collections

*Custom collections need EmulationStation 2.6.0 or newer. [Here's how to use them](https://github.com/retropie/retropie-setup/wiki/EmulationStation#custom-collections).*

```
hascheevos check --collection --system all
```

`--collection` writes one file per system, like `custom-achievements nes.cfg`, and `--single-collection` writes `custom-achievements.cfg`. Both go in `~/.emulationstation/collections`, and you can only use one of them per run.

Repeat runs don't grow these files, which are kept sorted and without duplicates. A ROM that no longer has achievements isn't removed from them. A ROM outside `~/RetroPie/roms` can't go in a collection, which is reported as an error.

Depending on how many ROMs you have, this can take a few minutes. When it's finished, restart EmulationStation, press `Start` to access the **MAIN MENU**, go to **GAME COLLECTIONS SETTINGS** -> **CUSTOM GAME COLLECTIONS** and enable the achievements collections you see there.

### `game`: say whether games have achievements

```
hascheevos game [--refresh] <ID>...
```

Prints each game ID that has achievements.

### `md5`: say whether a hash has achievements

```
hascheevos md5 --console <NAME|ID> [--refresh] <MD5>
```

Prints the hash if it has achievements. `--console` is required, because the API can't search by hash.

### `hash`: print a file's hashes

```
hascheevos hash [--console <NAME|ID>] [-v] <FILE>
```

Prints the RetroAchievements hash for the named console or, without one, the hashes for the consoles the file could belong to, each distinct hash once. `-v` reports rcheevos progress on stderr. This command needs no API key and no network.

## Exit codes

`check`, `game` and `md5` work like `grep`:

| Code | Meaning |
| --- | --- |
| `0` | At least one item has achievements. |
| `1` | None do. This includes a hash that isn't found, or that's marked as anything other than `compatible` or `untested`. |
| `2` | Something failed, even if other items have achievements. |

One failed ROM doesn't stop the rest of the run, but a missing or rejected API key does, as every other download would fail the same way.

`hash` exits with `0` when it prints a hash and `1` when it fails. Every command exits with `2` for invalid arguments.

## How caching works

The API can't search by hash, so `hascheevos` downloads a console's whole list of hashes the first time a lookup needs it, and only for the consoles it needs. Lists are kept in `$XDG_CACHE_HOME/hascheevos`, or `~/.cache/hascheevos` if that isn't set. `game` keeps each game the same way.

Entries are kept indefinitely, and one is downloaded again only when:

- a lookup finds no achievements and the entry is at least 24 hours old, or
- you pass `--refresh`.

So a positive answer needs no network, and a negative one is never based on data more than a day old. Each list is downloaded at most once per run, and every successful download resets its age, even if nothing changed.

Some things that follow from this:

- A game that loses its achievements keeps showing as having them until its list is next downloaded. Use `--refresh` if you suspect that happened.
- If an update fails, the cached list stays as it was and its answer stands, with a warning and exit code `2`. With no cached list, the ROM is reported as failed.
- The server allows 60 requests a minute and a large console's list takes about 30, so a first run over many consoles pauses whenever it reaches that limit.
- Moving the binary makes it download its lists again, as does an upgrade that changes the caching library's version. You can delete the cache directory at any time for the same effect.

## Moving from `hascheevos.sh`

`hascheevos` replaces the `hascheevos.sh` script, its data files and its separately compiled hash calculator. The script's flags are not accepted anymore, and neither are its short options like `-t`.

If you have a clone of this repository, run `git pull && git submodule update --init` and install as described above. You can delete the `bin/cheevoshash` left over from `make`, and you no longer need `jq`, `curl`, `unzip`, `gzip` or `p7zip-full` for this tool.

| Script | Now |
| --- | --- |
| `hascheevos.sh ROM...` | `hascheevos check ROM...` |
| `-t`, `--tab-output` | `check --tab-output`, which prints `name<TAB>gameID<TAB>x` |
| `-d`, `--copy-roms-to DIR` | `check --copy-roms-to DIR` |
| `-s`, `--system SYSTEMS` | `check --system SYSTEMS` |
| `--collection` | `check --collection` |
| `--single-collection` | `check --single-collection` |
| `-a`, `--arcade` | `check --console arcade` |
| `-g`, `--game-id 1,2,3` | `game 1 2 3` |
| `--hash CHECKSUM` | `md5 --console NAME CHECKSUM` |
| `-f`, `--check-false` | `--refresh` |
| `-r`, `--check-ra-server` | `--refresh` |
| `--get-hashlib SYSTEM` | `--refresh` |

These were dropped:

| Script | Why |
| --- | --- |
| `--update`, `-c`, `--check-hascheevos` | They only maintained the data files, which are gone. |
| `-u`, `--user`, `-p`, `--password`, `--token`, `--print-token` | Set `RA_API_KEY` instead. |
| `--scrape` | Achievement data isn't written to `gamelist.xml` anymore. |

The script's interactive dependency install and its refusal to run as root are gone too.

Two more changes to watch for if you call it from your own scripts: `game` now exits with `0` if any game has achievements, where `--game-id` returned `1` if any game lacked them. And failures have their own exit code, `2`.

---

**What's the point of creating this tool?!**

Links to the answer:

- https://retropie.org.uk/forum/topic/11859/what-about-adding-a-cheevos-flag-in-gamelist-xml

- http://retroachievements.org/viewtopic.php?t=5025
