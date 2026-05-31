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
const ConsoleNES = uint32(C.RC_CONSOLE_NINTENDO)

// Hash returns the rcheevos hash for the given console's ROM bytes.
func Hash(consoleID uint32, data []byte) (string, error) {
	if len(data) == 0 {
		return "", errors.New("rchash: empty data")
	}

	var hash [33]C.char
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
