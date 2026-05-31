# CLAUDE.md

## Build / test — REQUIRED order

cgo links a prebuilt static lib that is gitignored. Build it first:

```
make -C internal/rchash lib   # produces internal/rchash/librchash.a
go test ./...
go build ./cmd/rah
```

## Constraints

- Lib is ROM-only: built with `-DRC_HASH_NO_DISC -DRC_HASH_NO_ENCRYPTED -DRC_HASH_NO_ZIP` (no zlib/aes). Zip handled in Go (`archive/zip`, first entry). Don't add C sources that pull disc/zip/encrypted deps.
- Makefile `SRC` is the minimal hand-curated source set; missing symbols at link time → add the specific `.c`, keep minimal.
- cgo `${SRCDIR}` / `-Ircheevos/...` paths are relative to `internal/rchash/`, not repo root.

## Adding a system

1. Export id in `internal/rchash/rchash.go` (`const ConsoleX = uint32(C.RC_CONSOLE_...)`).
2. Add name → id in `systems` map in `cmd/rah/rah.go`.
