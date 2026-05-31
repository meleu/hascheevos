# `rah` v1 — Implementation Plan

CLI that hashes NES ROMs via the rcheevos `rc_hash` C library (Cgo).
Invocation: `rah nes <file>` → prints `<md5hash> <basename>`.

Acceptance: `rah nes testdata/Zooming-Secretary.zip` →
`bed0f7b12673dd762eed665c5c61927b Zooming-Secretary.zip`, exit 0.

## Locked decisions

| Topic | Decision |
|---|---|
| C consumption | Prebuilt static lib `librchash.a`, cgo links it |
| rcheevos | git submodule, moved to `internal/rchash/rcheevos/` |
| Build flags | `-DRC_HASH_NO_DISC -DRC_HASH_NO_ENCRYPTED -DRC_HASH_NO_ZIP` (no zlib/aes) |
| Zip | unzip in Go (`archive/zip`), first entry; zero-entry → generic failure (exit 1) |
| Hash call | uniform buffer path → `rc_hash_generate_from_buffer`; accepts `.zip` + raw |
| System arg | names-only Go map `{"nes": C.RC_CONSOLE_NINTENDO}`; unknown → error |
| CLI | stdlib only (`os.Args`) |
| Output | `<hash> <basename>\n` stdout; errors stderr; exit 0/1 (usage → 2) |
| Diagnostics | generic error, no rc_hash callbacks (additive upgrade later) |
| Module | `github.com/meleu/hascheevos`, binary `cmd/rah` |
| Build trigger | manual `make lib` then `go build` (no go:generate, `.a` gitignored) |
| Testdata | `testdata/` (project root; shared by unit + e2e tests) |
| Distribution | CI: lint+test job (ubuntu only) + separate build matrix (ubuntu/macos/windows-msys2) |

## Target layout

```
hascheevos/
  go.mod                       # module github.com/meleu/hascheevos
  PLAN.md
  README.md
  .gitignore                   # + internal/rchash/librchash.a, *.o
  .gitmodules                  # rcheevos path -> internal/rchash/rcheevos
  cmd/rah/
    rah.go                     # main + hashFile(system, path) (hash, name string, err error)
    rah_test.go                # TestNESHash
  testdata/Zooming-Secretary.zip
  internal/rchash/
    rchash.go                  # cgo wrapper: Hash(id uint32, data []byte) (string, error)
    Makefile                   # builds librchash.a
    rcheevos/                  # submodule
    librchash.a                # built artifact (gitignored)
  .github/workflows/ci.yml
```

---

## Phase 0 — Scaffolding & submodule relocation

Goal: repo skeleton compiles as a pure-Go no-op; submodule lives in its new home.

1. `go mod init github.com/meleu/hascheevos`.
2. Move submodule: `git mv rcheevos internal/rchash/rcheevos`; edit `.gitmodules`
   `path = internal/rchash/rcheevos`; `git submodule sync`.
3. Testdata lives at project root: `testdata/` (shared by unit + e2e tests).
4. `.gitignore`: add `internal/rchash/librchash.a`, `internal/rchash/**/*.o`.
5. Stub `cmd/rah/rah.go` with empty `main()`.

Verify: `go build ./cmd/rah` succeeds; `git submodule status` clean.

---

## Phase 1 — Build the static lib (`librchash.a`)

Goal: `make lib` produces a ROM-only `librchash.a` from the submodule.

1. Write `internal/rchash/Makefile`:
   - `CFLAGS = -O2 -DRC_HASH_NO_DISC -DRC_HASH_NO_ENCRYPTED -DRC_HASH_NO_ZIP`
   - `INCLUDES = -Ircheevos/include -Ircheevos/src`
   - Start source set: `rcheevos/src/rhash/hash.c`, `hash_rom.c`, `md5.c`,
     `rcheevos/src/rc_compat.c`, `rcheevos/src/rc_util.c`.
   - `ar rcs librchash.a $(OBJ)`.
2. `make lib`; resolve missing symbols reported by the next phase's link by adding
   the referenced `.c` (likely candidates: `rcheevos/src/rcheevos/consoleinfo.c`,
   `rcheevos/src/rc_version.c`). Keep the set minimal.

Verify: `librchash.a` exists; `nm librchash.a | grep rc_hash_generate_from_buffer`
shows the symbol defined (T).

---

## Phase 2 — Cgo wrapper (tracer bullet: raw bytes → hash)

Goal: Go can hash a raw NES byte buffer through the C lib.

1. `internal/rchash/rchash.go`:

   ```go
   package rchash

   /*
   #cgo CFLAGS: -Ircheevos/include
   #cgo LDFLAGS: -L${SRCDIR} -lrchash
   #include "rc_hash.h"
   */
   import "C"

   // exported console ids (extend as systems are added)
   const ConsoleNES = uint32(C.RC_CONSOLE_NINTENDO)

   func Hash(consoleID uint32, data []byte) (string, error) { ... }
   ```

   - Pass `&data[0]` + `C.size_t(len(data))` to `C.rc_hash_generate_from_buffer`
     into a `[33]C.char`; guard empty `data`.
   - Return `C.GoString` on non-zero; error on zero.
2. Throwaway check: a temp test that unzips the testdata in-test, calls
   `Hash(ConsoleNES, bytes)`, asserts `bed0f7b...`.

Verify: `go test ./internal/rchash` passes (after `make lib`).
Note: cgo `${SRCDIR}` resolves to the package dir, so the `.a` and `-Ircheevos/include` are relative to `internal/rchash/`.

---

## Phase 3 — CLI glue + end-to-end test

Goal: real `rah nes <file>` works; acceptance test green.

1. `cmd/rah/rah.go`:
   - `systems = map[string]uint32{"nes": rchash.ConsoleNES}`.
   - `hashFile(system, path) (hash, name string, err error)`:
     - lookup system → id (unknown → error).
     - if `.zip` (ext, case-insensitive): `archive/zip` open, take `File[0]`
       (zero entries → error); else `os.ReadFile`.
     - `rchash.Hash(id, data)`.
     - `name = filepath.Base(path)`.
   - `main()`: `len(os.Args) != 3` → usage + exit 2; call `hashFile`;
     error → `fmt.Fprintf(os.Stderr, "error: could not hash %s\n", file)` + exit 1;
     success → `fmt.Printf("%s %s\n", hash, name)`.
2. `cmd/rah/rah_test.go`: `TestNESHash` → `hashFile("nes", "../../testdata/Zooming-Secretary.zip")`
   asserts hash `== bed0f7b12673dd762eed665c5c61927b`.

Verify: `make -C internal/rchash lib && go test ./...` green;
manual `go run ./cmd/rah nes testdata/Zooming-Secretary.zip` prints expected line.

---

## Phase 4 — CI & docs

Goal: automated verification + buildable binaries.

1. `.github/workflows/ci.yml`:
   - **Job `lint-test`** (ubuntu only): checkout w/ submodules, setup-go,
     `make -C internal/rchash lib`, `gofmt -l .` (fail if output),
     `go vet ./...`, `go test ./...`.
   - **Job `build`** (matrix ubuntu/macos/windows): windows installs msys2
     (gcc+make); `make lib`; `go build -o dist/ ./cmd/rah`; upload artifact.
2. `README.md`: build steps (`git clone --recursive`, `make -C internal/rchash lib`,
   `go build ./cmd/rah`), usage, supported systems (nes).

Verify: CI green on a PR; artifacts present per platform.

---

## Risks / watch-items

- **Exact source set** is linker-discovered in Phase 1–2; keep additions minimal,
  stay within `RC_HASH_NO_*` boundaries (avoid pulling disc/zip/encrypted deps).
- **Windows cgo**: needs msys2 `make`+`gcc`; `${SRCDIR}` + `-lrchash` path must
  resolve on all platforms (static `.a`, so no runtime lib path issues).
- **`go install` not supported** (needs prebuilt `.a`); distribution is via CI binaries.
