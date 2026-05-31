// Package rchash wraps the rcheevos rc_hash C library to hash console ROMs.
package rchash

/*
#cgo CFLAGS: -Ircheevos/include
#cgo LDFLAGS: -L${SRCDIR} -lrchash
#include "rc_hash.h"
*/
import "C"

import (
	"errors"
	"unsafe"
)

// Exported console ids (extend as systems are added).
const (
	ConsoleNES  = uint32(C.RC_CONSOLE_NINTENDO)
	ConsoleSNES = uint32(C.RC_CONSOLE_SUPER_NINTENDO)
)

// hashBufSize is the buffer rc_hash_generate_from_buffer writes into:
// 32 hex MD5 chars + NUL terminator.
const hashBufSize = 33

// Hash returns the rcheevos hash for the given console's ROM bytes.
func Hash(consoleID uint32, data []byte) (string, error) {
	if len(data) == 0 {
		return "", errors.New("rchash: empty data")
	}

	var hash [hashBufSize]C.char
	ok := C.rc_hash_generate_from_buffer(
		&hash[0],
		C.uint32_t(consoleID),
		(*C.uint8_t)(unsafe.Pointer(&data[0])),
		C.size_t(len(data)),
	)
	if ok == 0 {
		return "", errors.New("rchash: hash generation failed")
	}

	return C.GoString(&hash[0]), nil
}
