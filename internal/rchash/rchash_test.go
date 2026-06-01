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

	data := readZipFirstEntry(t, "../../testdata/nes/Zooming-Secretary.zip")

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
	data := readZipFirstEntry(t, "../../testdata/atari2600/Wall-Jump-Ninja.zip")

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
	data := readZipFirstEntry(t, "../../testdata/megadrive/Abbaye-des-Morts.zip")

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

// TestHashN64FromZip checks the z64 (big-endian, native) path: the test ROM
// begins with 0x80, so rc_hash_n64 performs no byteswap and the hash is the
// MD5 of the whole payload.
func TestHashN64FromZip(t *testing.T) {
	data := readZipFirstEntry(t, "../../testdata/n64/pyoro64-ntsc.zip")

	if data[0] != 0x80 {
		t.Fatalf("ROM first byte = %#x, want 0x80 (z64); test pins the no-swap path", data[0])
	}

	// z64 hash == MD5 of the whole payload (no conversion).
	wantSum := md5.Sum(data)
	want := hex.EncodeToString(wantSum[:])

	got, err := Hash(ConsoleNintendo64, data)
	if err != nil {
		t.Fatalf("Hash: %v", err)
	}
	if got != want {
		t.Errorf("Hash = %q, want %q", got, want)
	}
}

// TestHashN64FormatsMatch proves format normalization: the same ROM in z64,
// v64 (byteswapped), and n64 (little-endian) formats all hash identically,
// because rc_hash_n64 converts v64/n64 back to z64 before hashing.
func TestHashN64FormatsMatch(t *testing.T) {
	z64 := readZipFirstEntry(t, "../../testdata/n64/pyoro64-ntsc.zip")
	if len(z64)%4 != 0 {
		t.Fatalf("ROM size %d not divisible by 4; cannot synthesize n64", len(z64))
	}

	// v64: byteswap every 16-bit word (first byte 0x80 -> 0x37).
	v64 := make([]byte, len(z64))
	copy(v64, z64)
	for i := 0; i+1 < len(v64); i += 2 {
		v64[i], v64[i+1] = v64[i+1], v64[i]
	}
	if v64[0] != 0x37 {
		t.Fatalf("synthesized v64 first byte = %#x, want 0x37", v64[0])
	}

	// n64: reverse every 32-bit word (first byte 0x80 -> 0x40).
	n64 := make([]byte, len(z64))
	copy(n64, z64)
	for i := 0; i+3 < len(n64); i += 4 {
		n64[i], n64[i+1], n64[i+2], n64[i+3] = n64[i+3], n64[i+2], n64[i+1], n64[i]
	}
	if n64[0] != 0x40 {
		t.Fatalf("synthesized n64 first byte = %#x, want 0x40", n64[0])
	}

	want, err := Hash(ConsoleNintendo64, z64)
	if err != nil {
		t.Fatalf("Hash z64: %v", err)
	}

	for _, tc := range []struct {
		name string
		data []byte
	}{
		{"v64", v64},
		{"n64", n64},
	} {
		got, err := Hash(ConsoleNintendo64, tc.data)
		if err != nil {
			t.Fatalf("Hash %s: %v", tc.name, err)
		}
		if got != want {
			t.Errorf("%s hash = %q, want %q (z64)", tc.name, got, want)
		}
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

// readZipFirstEntry is a test helper that reads the first entry of a zip.
func readZipFirstEntry(t *testing.T, path string) []byte {
	t.Helper()

	zr, err := zip.OpenReader(path)
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
	return data
}
