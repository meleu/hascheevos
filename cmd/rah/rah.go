// Command rah hashes a console ROM using the rcheevos hashing library.
//
// Usage:
//
//	rah <system> <file>
//
// It prints "<hash> <basename>" to stdout on success.
package main

import (
	"archive/zip"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"

	"github.com/meleu/hascheevos/internal/rchash"
)

// systems maps user-facing system names to rcheevos console ids.
var systems = map[string]uint32{
	"nes":          rchash.ConsoleNES,
	"snes":         rchash.ConsoleSNES,
	"atari2600":    rchash.ConsoleAtari2600,
	"megadrive":    rchash.ConsoleMegaDrive,
	"amstradpc":    rchash.ConsoleAmstradPC,
	"appleii":      rchash.ConsoleAppleII,
	"arcadia2001":  rchash.ConsoleArcadia2001,
	"atarijaguar":  rchash.ConsoleAtariJaguar,
	"colecovision": rchash.ConsoleColecoVision,
	"c64":          rchash.ConsoleCommodore64,
	"elektor":      rchash.ConsoleElektor,
}

func main() {
	if len(os.Args) != 3 {
		fmt.Fprintf(os.Stderr, "usage: %s <system> <file>\n", filepath.Base(os.Args[0]))
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
