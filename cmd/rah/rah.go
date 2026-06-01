// Command rah hashes a console ROM using the rcheevos hashing library.
//
// Usage:
//
//	rah <system> <file>
//	rah -list
//
// It prints "<hash> <basename>" to stdout on success. With -list it prints
// every supported system key, one per line, sorted.
package main

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"sort"
	"strings"

	"github.com/meleu/hascheevos/internal/rchash"
)

// systems maps user-facing system names to rcheevos console ids.
var systems = map[string]uint32{
	"nes":            rchash.ConsoleNES,
	"n64":            rchash.ConsoleNintendo64,
	"snes":           rchash.ConsoleSNES,
	"atari2600":      rchash.ConsoleAtari2600,
	"megadrive":      rchash.ConsoleMegaDrive,
	"amstradpc":      rchash.ConsoleAmstradPC,
	"appleii":        rchash.ConsoleAppleII,
	"arcadia2001":    rchash.ConsoleArcadia2001,
	"atarijaguar":    rchash.ConsoleAtariJaguar,
	"colecovision":   rchash.ConsoleColecoVision,
	"c64":            rchash.ConsoleCommodore64,
	"elektor":        rchash.ConsoleElektor,
	"channelf":       rchash.ConsoleChannelF,
	"gb":             rchash.ConsoleGameBoy,
	"gba":            rchash.ConsoleGameBoyAdvance,
	"gbc":            rchash.ConsoleGameBoyColor,
	"gamegear":       rchash.ConsoleGameGear,
	"intellivision":  rchash.ConsoleIntellivision,
	"intertonvc4000": rchash.ConsoleIntertonVC4000,
	"odyssey2":       rchash.ConsoleMagnavoxOdyssey2,
	"mastersystem":   rchash.ConsoleMasterSystem,
	"megaduck":       rchash.ConsoleMegaDuck,
	"msx":            rchash.ConsoleMSX,
	"neogeopocket":   rchash.ConsoleNeoGeoPocket,
	"oric":           rchash.ConsoleOric,
	"pc8800":         rchash.ConsolePC8800,
	"pokemonmini":    rchash.ConsolePokemonMini,
	"sega32x":        rchash.ConsoleSega32X,
	"sg1000":         rchash.ConsoleSG1000,
	"supervision":    rchash.ConsoleSupervision,
	"ti83":           rchash.ConsoleTI83,
	"tic80":          rchash.ConsoleTIC80,
	"uzebox":         rchash.ConsoleUzebox,
	"vectrex":        rchash.ConsoleVectrex,
	"virtualboy":     rchash.ConsoleVirtualBoy,
	"wasm4":          rchash.ConsoleWASM4,
	"wonderswan":     rchash.ConsoleWonderSwan,
	"zxspectrum":     rchash.ConsoleZXSpectrum,
}

func main() {
	if len(os.Args) == 2 && os.Args[1] == "-list" {
		for _, key := range listSystems() {
			fmt.Println(key)
		}
		os.Exit(0)
	}

	if len(os.Args) != 3 {
		fmt.Fprintf(os.Stderr, "usage: %s <system> <file>\n       %s -list\n", filepath.Base(os.Args[0]), filepath.Base(os.Args[0]))
		os.Exit(2)
	}

	system, file := os.Args[1], os.Args[2]

	hash, name, err := hashFile(system, file)
	if err != nil {
		fmt.Fprintf(os.Stderr, "error: could not hash %s\n", file)
		os.Exit(1)
	}

	fmt.Printf("%s %s\n", hash, name)
}

// listSystems returns all supported system keys, sorted.
func listSystems() []string {
	keys := make([]string, 0, len(systems))
	for key := range systems {
		keys = append(keys, key)
	}
	sort.Strings(keys)
	return keys
}

// hashFile hashes the ROM at path for the given system, returning the hash and
// the file's basename.
func hashFile(system, path string) (hash, name string, err error) {
	id, ok := systems[system]
	if !ok {
		return "", "", fmt.Errorf("unknown system %q", system)
	}

	data, err := readROM(path)
	if err != nil {
		return "", "", err
	}

	hash, err = rchash.Hash(id, data)
	if err != nil {
		return "", "", err
	}

	return hash, filepath.Base(path), nil
}

// readROM returns the ROM bytes for path. Zip archives are unzipped in Go and
// the first entry is used; any other file is read as-is.
func readROM(path string) ([]byte, error) {
	if strings.EqualFold(filepath.Ext(path), ".zip") {
		return readZipFirstEntry(path)
	}
	return os.ReadFile(path)
}

// readZipFirstEntry reads the first file entry from a zip archive.
func readZipFirstEntry(path string) ([]byte, error) {
	r, err := zip.OpenReader(path)
	if err != nil {
		return nil, err
	}
	defer r.Close()

	if len(r.File) == 0 {
		return nil, fmt.Errorf("zip %q has no entries", path)
	}

	rc, err := r.File[0].Open()
	if err != nil {
		return nil, err
	}
	defer rc.Close()

	return io.ReadAll(rc)
}
