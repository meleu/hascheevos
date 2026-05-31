package rchash

import (
	"archive/zip"
	"io"
	"testing"
)

func TestHashNESFromZip(t *testing.T) {
	const want = "bed0f7b12673dd762eed665c5c61927b"

	zr, err := zip.OpenReader("../../testdata/Zooming-Secretary.zip")
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
