package main

import "testing"

func TestNESHash(t *testing.T) {
	hash, name, err := hashFile("nes", "../../testdata/nes/Zooming-Secretary.zip")
	if err != nil {
		t.Fatalf("hashFile: %v", err)
	}

	const wantHash = "bed0f7b12673dd762eed665c5c61927b"
	if hash != wantHash {
		t.Errorf("hash = %q, want %q", hash, wantHash)
	}

	const wantName = "Zooming-Secretary.zip"
	if name != wantName {
		t.Errorf("name = %q, want %q", name, wantName)
	}
}

func TestSNESHash(t *testing.T) {
	hash, name, err := hashFile("snes", "../../testdata/snes/Christmas-Craze.zip")
	if err != nil {
		t.Fatalf("hashFile: %v", err)
	}

	const wantHash = "0829ebfde80e7a9673fe9966a8d47b0f"
	if hash != wantHash {
		t.Errorf("hash = %q, want %q", hash, wantHash)
	}

	const wantName = "Christmas-Craze.zip"
	if name != wantName {
		t.Errorf("name = %q, want %q", name, wantName)
	}
}

func TestAtari2600Hash(t *testing.T) {
	hash, name, err := hashFile("atari2600", "../../testdata/atari2600/Wall-Jump-Ninja.zip")
	if err != nil {
		t.Fatalf("hashFile: %v", err)
	}

	const wantHash = "3c56c0c5f6f97850ed0aa7bcc2a4e30e"
	if hash != wantHash {
		t.Errorf("hash = %q, want %q", hash, wantHash)
	}

	const wantName = "Wall-Jump-Ninja.zip"
	if name != wantName {
		t.Errorf("name = %q, want %q", name, wantName)
	}
}

func TestMegaDriveHash(t *testing.T) {
	hash, name, err := hashFile("megadrive", "../../testdata/megadrive/Abbaye-des-Morts.zip")
	if err != nil {
		t.Fatalf("hashFile: %v", err)
	}

	const wantHash = "bc2e5590cc0e7b6e863b4275343839d4"
	if hash != wantHash {
		t.Errorf("hash = %q, want %q", hash, wantHash)
	}

	const wantName = "Abbaye-des-Morts.zip"
	if name != wantName {
		t.Errorf("name = %q, want %q", name, wantName)
	}
}

func TestWholeFileSystems(t *testing.T) {
	const (
		wantHash = "6ed56530d641cd3cc1c9921fe3327b4a"
		wantName = "generic-file.rom"
	)
	cases := []string{
		"amstradpc",
		"appleii",
		"arcadia2001",
		"atarijaguar",
		"colecovision",
		"c64",
		"elektor",
		"channelf",
		"gb",
		"gba",
		"gbc",
		"gamegear",
		"intellivision",
		"intertonvc4000",
		"odyssey2",
		"mastersystem",
		// later phases append their key here
	}
	for _, key := range cases {
		t.Run(key, func(t *testing.T) {
			hash, name, err := hashFile(key, "../../testdata/generic-file.rom")
			if err != nil {
				t.Fatalf("hashFile: %v", err)
			}
			if hash != wantHash {
				t.Errorf("hash = %q, want %q", hash, wantHash)
			}
			if name != wantName {
				t.Errorf("name = %q, want %q", name, wantName)
			}
		})
	}
}

func TestUnknownSystem(t *testing.T) {
	if _, _, err := hashFile("bogus", "../../testdata/nes/Zooming-Secretary.zip"); err == nil {
		t.Fatal("expected error for unknown system, got nil")
	}
}
