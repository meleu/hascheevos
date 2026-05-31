# rah

`rah` hashes console ROMs using RetroAchievements' [rcheevos](https://github.com/RetroAchievements/rcheevos)
`rc_hash` C library, called from Go via cgo.

```console
$ rah nes Zooming-Secretary.zip
bed0f7b12673dd762eed665c5c61927b Zooming-Secretary.zip
```

The hash is what RetroAchievements uses to identify a game, so it can be matched
against their database.

## Supported systems

| Name  | System                |
|-------|-----------------------|
| `nes` | Nintendo / Famicom    |

## Building

`rah` links a small static library (`librchash.a`) built from the rcheevos
submodule, so a C toolchain (`gcc`/`clang`) and `make` are required.

```sh
git clone --recursive https://github.com/meleu/hascheevos.git
cd hascheevos
make -C internal/rchash lib   # builds internal/rchash/librchash.a
go build ./cmd/rah
```

If you cloned without `--recursive`, fetch the submodule first:

```sh
git submodule update --init --recursive
```

> `go install` is not supported: the build needs the prebuilt `librchash.a`.
> Use the steps above, or grab a binary from CI artifacts.

## Usage

```sh
rah <system> <file>
```

- `<system>` is one of the names in the table above (e.g. `nes`).
- `<file>` is a ROM, either raw or a `.zip` (the first entry is hashed).

On success it prints `<md5hash> <basename>` to stdout and exits 0.
On error it prints a message to stderr and exits 1; bad usage exits 2.

## Development

```sh
make -C internal/rchash lib
go test ./...
go vet ./...
```
