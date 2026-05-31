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

func TestUnknownSystem(t *testing.T) {
	if _, _, err := hashFile("bogus", "../../testdata/nes/Zooming-Secretary.zip"); err == nil {
		t.Fatal("expected error for unknown system, got nil")
	}
}
