# [DONE!] `rah` — Super Nintendo (SNES) support

Add `rah snes <file>` → `<md5hash> <basename>`.

Acceptance: `rah snes testdata/snes/Christmas-Craze.zip` →
`0829ebfde80e7a9673fe9966a8d47b0f Christmas-Craze.zip`, exit 0.

## Key finding

SNES is **already fully supported by the compiled lib** — no C/Makefile/lib
changes needed:
- `RC_CONSOLE_SUPER_NINTENDO = 3` (`rc_consoles.h`).
- `hash.c:565` dispatches it to `rc_hash_snes` (buffer hash w/ optional
  512-byte header skip); `hash_rom.c` is already in Makefile `SRC`.
- Existing buffer path (`rc_hash_generate_from_buffer`) handles it as-is.

Work is pure Go wiring + a test, mirroring the NES path.

## Changes

1. `internal/rchash/rchash.go`: add
   `const ConsoleSNES = uint32(C.RC_CONSOLE_SUPER_NINTENDO)`.
2. `cmd/rah/rah.go`: add `"snes": rchash.ConsoleSNES` to the `systems` map.
3. `cmd/rah/rah_test.go`: add `TestSNESHash` →
   `hashFile("snes", "../../testdata/snes/Christmas-Craze.zip")` asserts
   `0829ebfde80e7a9673fe9966a8d47b0f`.

## Verify

```
make -C internal/rchash lib
go test ./...
go run ./cmd/rah snes testdata/snes/Christmas-Craze.zip
```

Expect the acceptance line above.

## Unresolved questions

- None.
