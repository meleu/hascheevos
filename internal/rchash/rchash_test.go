package rchash

import (
	"archive/zip"
	"crypto/md5"
	"encoding/hex"
	"io"
	"os"
	"testing"
)

func TestHashNESFromZip(t *testing.T) {
	const want = "bed0f7b12673dd762eed665c5c61927b"

	zr, err := zip.OpenReader("../../testdata/nes/Zooming-Secretary.zip")
	if err != nil {
		t.Fatalf("open zip: %v", err)
	}
	defer zr.Close()

	if len(zr.File) == 0 {
		t.Fatal("zip has no entries")
	}

	rc, err := zr.File[0].Open()
	if err != nil {
		t.Fatalf("open entry: %v", err)
	}
	defer rc.Close()

	data, err := io.ReadAll(rc)
	if err != nil {
		t.Fatalf("read entry: %v", err)
	}

	got, err := Hash(ConsoleNES, data)
	if err != nil {
		t.Fatalf("Hash: %v", err)
	}
	if got != want {
		t.Errorf("Hash = %q, want %q", got, want)
	}
}

// TestHashAtari2600FromZip checks the generic full-file MD5 path: Atari 2600
// ROMs are hashed as the MD5 of the entire payload (no header, no parsing).
func TestHashAtari2600FromZip(t *testing.T) {
	zr, err := zip.OpenReader("../../testdata/atari2600/Wall-Jump-Ninja.zip")
	if err != nil {
		t.Fatalf("open zip: %v", err)
	}
	defer zr.Close()

	if len(zr.File) == 0 {
		t.Fatal("zip has no entries")
	}

	rc, err := zr.File[0].Open()
	if err != nil {
		t.Fatalf("open entry: %v", err)
	}
	defer rc.Close()

	data, err := io.ReadAll(rc)
	if err != nil {
		t.Fatalf("read entry: %v", err)
	}

	// Atari 2600 hash == MD5 of the whole payload.
	wantSum := md5.Sum(data)
	want := hex.EncodeToString(wantSum[:])

	got, err := Hash(ConsoleAtari2600, data)
	if err != nil {
		t.Fatalf("Hash: %v", err)
	}
	if got != want {
		t.Errorf("Hash = %q, want %q", got, want)
	}
}

// TestHashMegaDriveFromZip checks the generic full-file MD5 path: Mega Drive
// ROMs hashed via the buffer API are the MD5 of the entire payload (no header
// parsing, no SMD de-interleave).
func TestHashMegaDriveFromZip(t *testing.T) {
	zr, err := zip.OpenReader("../../testdata/megadrive/Abbaye-des-Morts.zip")
	if err != nil {
		t.Fatalf("open zip: %v", err)
	}
	defer zr.Close()

	if len(zr.File) == 0 {
		t.Fatal("zip has no entries")
	}

	rc, err := zr.File[0].Open()
	if err != nil {
		t.Fatalf("open entry: %v", err)
	}
	defer rc.Close()

	data, err := io.ReadAll(rc)
	if err != nil {
		t.Fatalf("read entry: %v", err)
	}

	// Mega Drive hash == MD5 of the whole payload.
	wantSum := md5.Sum(data)
	want := hex.EncodeToString(wantSum[:])

	got, err := Hash(ConsoleMegaDrive, data)
	if err != nil {
		t.Fatalf("Hash: %v", err)
	}
	if got != want {
		t.Errorf("Hash = %q, want %q", got, want)
	}
}

// TestHashSNESHeadered checks the path where a 512-byte SNES (SMC/SFC) copier
// header is detected and stripped before hashing. The ROM is 8704 bytes
// (0x2000 + 512), so rc_hash_snes ignores the header and hashes only the
// remaining 8192 bytes.
func TestHashSNESHeadered(t *testing.T) {
	data, err := os.ReadFile("../../testdata/snes/FakeHeaderedROM.sfc")
	if err != nil {
		t.Fatalf("read ROM: %v", err)
	}

	const header = 512
	if (len(data)-header)%0x2000 != 0 || len(data) <= header {
		t.Fatalf("ROM size %d does not exercise the header path", len(data))
	}

	// With the header stripped, the hash is the MD5 of the payload.
	wantSum := md5.Sum(data[header:])
	want := hex.EncodeToString(wantSum[:])

	got, err := Hash(ConsoleSNES, data)
	if err != nil {
		t.Fatalf("Hash: %v", err)
	}
	if got != want {
		t.Errorf("Hash = %q, want %q (header not stripped?)", got, want)
	}

	// Sanity: hashing the whole file (header included) must differ, proving
	// the header was actually ignored rather than hashed.
	fullSum := md5.Sum(data)
	if full := hex.EncodeToString(fullSum[:]); got == full {
		t.Errorf("Hash = full-file MD5 %q, header was not stripped", full)
	}
}
