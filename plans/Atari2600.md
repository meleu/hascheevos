# Plan: Atari 2600 ROM hashing

## Summary

Atari 2600 hashing in rcheevos is a plain **full-file MD5** (`rc_hash_buffer`
path, `RC_CONSOLE_ATARI_2600 = 25`). No special ROM parsing, no header
stripping. The needed C symbol is already in `hash.c`, which is already
compiled into `librchash.a`.

=> **No Makefile / C-source changes. No new build deps.** Pure Go wiring +
tests, following the CLAUDE.md "Adding a system" steps.

## Test data

`testdata/atari2600/Wall-Jump-Ninja.zip`
- single entry: `Wall Jump Ninja (World) (2015-01-15) (NTSC) (Aftermarket).a26`, 4096 bytes
- full-file MD5 (= the expected hash): `3c56c0c5f6f97850ed0aa7bcc2a4e30e`

## Changes

1. **`internal/rchash/rchash.go`** — add to the console-id const block:
   ```go
   ConsoleAtari2600 = uint32(C.RC_CONSOLE_ATARI_2600)
   ```

2. **`cmd/rah/rah.go`** — add to `systems` map:
   ```go
   "atari2600": rchash.ConsoleAtari2600,
   ```

3. **`internal/rchash/rchash_test.go`** — add `TestHashAtari2600FromZip`.
   Mirror `TestHashNESFromZip`: unzip first entry, `Hash(ConsoleAtari2600, data)`.
   Since the algorithm is full-file MD5, assert against `md5.Sum(data)`
   (computed in-test, like the SNES test does) rather than only a hardcoded
   literal — proves the hash == MD5 of the whole payload.

4. **`cmd/rah/rah_test.go`** — add `TestAtari2600Hash`.
   Mirror `TestSNESHash`: `hashFile("atari2600", ".../Wall-Jump-Ninja.zip")`,
   assert hash == `3c56c0c5f6f97850ed0aa7bcc2a4e30e`, name == `Wall-Jump-Ninja.zip`.

## Verify

```
make -C internal/rchash lib   # already built; no-op unless clean
go test ./...
go build ./cmd/rah
```

## Unresolved questions

- System key name: plan uses `"atari2600"` (matches the testdata dir). Acceptable, or prefer `"atari"` / `"2600"`?
