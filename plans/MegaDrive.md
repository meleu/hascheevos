# `rah` — Sega Mega Drive (Genesis) support

Add `rah megadrive <file>` → `<md5hash> <basename>`.

Acceptance: `rah megadrive testdata/megadrive/Abbaye-des-Morts.zip` →
`bc2e5590cc0e7b6e863b4275343839d4 Abbaye-des-Morts.zip`, exit 0.

## Key finding

Mega Drive is **already fully supported by the compiled lib** — no
C/Makefile/lib changes needed:

- `RC_CONSOLE_MEGA_DRIVE = 1` (`rc_consoles.h`).
- In `rc_hash_generate_from_buffer` (the function our Go wrapper calls),
  `hash.c:525` puts `RC_CONSOLE_MEGA_DRIVE` in the group that falls through to
  `rc_hash_buffer` (`hash.c:543`) — a **plain whole-file MD5** of the buffer.
  No header parsing, no SMD de-interleave (the m3u/whole-file branch at
  `hash.c:827` only applies to the file-path API, not the buffer API we use).
- Same path as Atari 2600 → MD5 of the unzipped payload.

Work is pure Go wiring + tests, mirroring the Atari 2600 path.

## Changes

1. `internal/rchash/rchash.go`: add
   `const ConsoleMegaDrive = uint32(C.RC_CONSOLE_MEGA_DRIVE)`.
2. `cmd/rah/rah.go`: add `"megadrive": rchash.ConsoleMegaDrive` to the
   `systems` map.
3. `cmd/rah/rah_test.go`: add `TestMegaDriveHash` →
   `hashFile("megadrive", "../../testdata/megadrive/Abbaye-des-Morts.zip")`
   asserts hash `bc2e5590cc0e7b6e863b4275343839d4` and name
   `Abbaye-des-Morts.zip`.
4. `internal/rchash/rchash_test.go`: add `TestHashMegaDriveFromZip` mirroring
   `TestHashAtari2600FromZip` — unzip first entry, assert `Hash(ConsoleMegaDrive,
   data)` equals `md5.Sum(data)` (proves the whole-file path).

## Verify

```
make -C internal/rchash lib
go test ./...
go run ./cmd/rah megadrive testdata/megadrive/Abbaye-des-Morts.zip
```

Expect the acceptance line above.
