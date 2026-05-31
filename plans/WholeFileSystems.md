# `rah` — whole-file-MD5 systems

Wire up every console that the compiled lib hashes as a **plain whole-file
MD5** of the buffer (cases `hash.c:508–542` → `rc_hash_buffer` at `hash.c:543`,
via the `rc_hash_from_buffer` path our Go wrapper uses).

Atari 2600 and Mega Drive are already done. The remaining **33** systems are
each a separate phase below.

IMPORTANT: each phase should be finished with a `Add <system> support` commit.

## Why this is pure Go wiring

- All these console ids fall through to a plain MD5 of the whole buffer — no
  header parsing, no transform. Same code path already proven by
  `TestHashAtari2600FromZip` / `TestHashMegaDriveFromZip`.
- The `case` group sits **outside** the `#ifndef RC_HASH_NO_ROM` block, so it is
  always compiled under our build flags. `RC_CONSOLE_*` are plain enum values in
  `rc_consoles.h`, always available.
- **No C / Makefile / lib changes for any phase.**

## Test fixture

All phases share one fixture: `testdata/generic-file.rom` (96 bytes).
Expected hash for every whole-file system:

```
6ed56530d641cd3cc1c9921fe3327b4a generic-file.rom
```

It is a plain file (not a zip), so `readROM` reads it as-is.

## Table-driven test (set up once, in Phase 1)

Phase 1 adds a single table-driven test to `cmd/rah/rah_test.go` covering every
whole-file system. Later phases just append one line to its `cases` slice.

```go
func TestWholeFileSystems(t *testing.T) {
 const (
  wantHash = "6ed56530d641cd3cc1c9921fe3327b4a"
  wantName = "generic-file.rom"
 )
 cases := []string{
  "amstradpc",
  // later phases append their key here
 }
 for _, key := range cases {
  t.Run(key, func(t *testing.T) {
   hash, name, err := hashFile(key, "../../testdata/generic-file.rom")
   if err != nil {
    t.Fatalf("hashFile: %v", err)
   }
   if hash != wantHash {
    t.Errorf("hash = %q, want %q", hash, wantHash)
   }
   if name != wantName {
    t.Errorf("name = %q, want %q", name, wantName)
   }
  })
 }
}
```

## Per-phase recipe (identical for every phase)

For system with key `KEY`, const `ConsoleX`, macro `RC_CONSOLE_MACRO`:

1. `internal/rchash/rchash.go`: add
   `ConsoleX = uint32(C.RC_CONSOLE_MACRO)` to the const block.
2. `cmd/rah/rah.go`: add `"KEY": rchash.ConsoleX,` to the `systems` map.
3. `cmd/rah/rah_test.go`: append `"KEY",` to the `cases` slice in
   `TestWholeFileSystems` (Phase 1 creates the test itself).

The whole-file code path itself is already proven by the existing
Atari2600/MegaDrive `rchash` tests; no new `internal/rchash` test per phase.

### Per-phase verify

```
go test ./...
go run ./cmd/rah KEY testdata/generic-file.rom
```

Expect: `6ed56530d641cd3cc1c9921fe3327b4a generic-file.rom`, exit 0.
(`make -C internal/rchash lib` only needed once — already built.)

## Phases

| # | System | key | const | C macro |
|---|--------|-----|-------|---------|
| 1 | Amstrad PC | `amstradpc` | `ConsoleAmstradPC` | `RC_CONSOLE_AMSTRAD_PC` |
| 2 | Apple II | `appleii` | `ConsoleAppleII` | `RC_CONSOLE_APPLE_II` |
| 3 | Arcadia 2001 | `arcadia2001` | `ConsoleArcadia2001` | `RC_CONSOLE_ARCADIA_2001` |
| 4 | Atari Jaguar | `atarijaguar` | `ConsoleAtariJaguar` | `RC_CONSOLE_ATARI_JAGUAR` |
| 5 | ColecoVision | `colecovision` | `ConsoleColecoVision` | `RC_CONSOLE_COLECOVISION` |
| 6 | Commodore 64 | `c64` | `ConsoleCommodore64` | `RC_CONSOLE_COMMODORE_64` |
| 7 | Elektor TV Games Computer | `elektor` | `ConsoleElektor` | `RC_CONSOLE_ELEKTOR_TV_GAMES_COMPUTER` |
| 8 | Fairchild Channel F | `channelf` | `ConsoleChannelF` | `RC_CONSOLE_FAIRCHILD_CHANNEL_F` |
| 9 | Game Boy | `gb` | `ConsoleGameBoy` | `RC_CONSOLE_GAMEBOY` |
| 10 | Game Boy Advance | `gba` | `ConsoleGameBoyAdvance` | `RC_CONSOLE_GAMEBOY_ADVANCE` |
| 11 | Game Boy Color | `gbc` | `ConsoleGameBoyColor` | `RC_CONSOLE_GAMEBOY_COLOR` |
| 12 | Game Gear | `gamegear` | `ConsoleGameGear` | `RC_CONSOLE_GAME_GEAR` |
| 13 | Intellivision | `intellivision` | `ConsoleIntellivision` | `RC_CONSOLE_INTELLIVISION` |
| 14 | Interton VC 4000 | `intertonvc4000` | `ConsoleIntertonVC4000` | `RC_CONSOLE_INTERTON_VC_4000` |
| 15 | Magnavox Odyssey 2 | `odyssey2` | `ConsoleMagnavoxOdyssey2` | `RC_CONSOLE_MAGNAVOX_ODYSSEY2` |
| 16 | Master System | `mastersystem` | `ConsoleMasterSystem` | `RC_CONSOLE_MASTER_SYSTEM` |
| 17 | Mega Duck | `megaduck` | `ConsoleMegaDuck` | `RC_CONSOLE_MEGADUCK` |
| 18 | MSX | `msx` | `ConsoleMSX` | `RC_CONSOLE_MSX` |
| 19 | Neo Geo Pocket | `neogeopocket` | `ConsoleNeoGeoPocket` | `RC_CONSOLE_NEOGEO_POCKET` |
| 20 | Oric | `oric` | `ConsoleOric` | `RC_CONSOLE_ORIC` |
| 21 | PC-8800 | `pc8800` | `ConsolePC8800` | `RC_CONSOLE_PC8800` |
| 22 | Pokémon Mini | `pokemonmini` | `ConsolePokemonMini` | `RC_CONSOLE_POKEMON_MINI` |
| 23 | Sega 32X | `sega32x` | `ConsoleSega32X` | `RC_CONSOLE_SEGA_32X` |
| 24 | SG-1000 | `sg1000` | `ConsoleSG1000` | `RC_CONSOLE_SG1000` |
| 25 | Supervision | `supervision` | `ConsoleSupervision` | `RC_CONSOLE_SUPERVISION` |
| 26 | TI-83 | `ti83` | `ConsoleTI83` | `RC_CONSOLE_TI83` |
| 27 | TIC-80 | `tic80` | `ConsoleTIC80` | `RC_CONSOLE_TIC80` |
| 28 | Uzebox | `uzebox` | `ConsoleUzebox` | `RC_CONSOLE_UZEBOX` |
| 29 | Vectrex | `vectrex` | `ConsoleVectrex` | `RC_CONSOLE_VECTREX` |
| 30 | Virtual Boy | `virtualboy` | `ConsoleVirtualBoy` | `RC_CONSOLE_VIRTUAL_BOY` |
| 31 | WASM-4 | `wasm4` | `ConsoleWASM4` | `RC_CONSOLE_WASM4` |
| 32 | WonderSwan | `wonderswan` | `ConsoleWonderSwan` | `RC_CONSOLE_WONDERSWAN` |
| 33 | ZX Spectrum | `zxspectrum` | `ConsoleZXSpectrum` | `RC_CONSOLE_ZX_SPECTRUM` |

## Phase 34 — `-list` option

Add `rah -list` to print every supported system key, one per line, sorted, to
stdout; exit 0. Lets users discover keys as the `systems` map grows large.

1. `cmd/rah/rah.go`:
   - In `main`, before the `len(os.Args) != 3` check, handle the list case:
     if `len(os.Args) == 2 && os.Args[1] == "-list"`, print sorted keys and
     `os.Exit(0)`.
   - Add a helper `listSystems() []string` returning the sorted map keys
     (`sort.Strings`), so it is unit-testable. Add `"sort"` import.
2. `cmd/rah/rah_test.go`: add `TestListSystems` asserting `listSystems()` is
   sorted and contains a representative sample (e.g. `nes`, `megadrive`,
   `zxspectrum`).
3. Update the usage string / package doc comment to mention `-list`.

### Verify

```
go test ./...
go run ./cmd/rah -list
```

Expect a sorted list of all keys, exit 0.

## Final verify (after all phases)

```
go test ./...
```

## Open questions

None — keys are short, tests are table-driven, `-list` is the final phase.
