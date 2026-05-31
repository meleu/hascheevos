package main

import "testing"

func TestNESHash(t *testing.T) {
	hash, name, err := hashFile("nes", "../../testdata/Zooming-Secretary.zip")
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

func TestUnknownSystem(t *testing.T) {
	if _, _, err := hashFile("bogus", "../../testdata/Zooming-Secretary.zip"); err == nil {
		t.Fatal("expected error for unknown system, got nil")
	}
}
