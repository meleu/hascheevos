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
	ConsoleNES              = uint32(C.RC_CONSOLE_NINTENDO)
	ConsoleSNES             = uint32(C.RC_CONSOLE_SUPER_NINTENDO)
	ConsoleAtari2600        = uint32(C.RC_CONSOLE_ATARI_2600)
	ConsoleMegaDrive        = uint32(C.RC_CONSOLE_MEGA_DRIVE)
	ConsoleAmstradPC        = uint32(C.RC_CONSOLE_AMSTRAD_PC)
	ConsoleAppleII          = uint32(C.RC_CONSOLE_APPLE_II)
	ConsoleArcadia2001      = uint32(C.RC_CONSOLE_ARCADIA_2001)
	ConsoleAtariJaguar      = uint32(C.RC_CONSOLE_ATARI_JAGUAR)
	ConsoleColecoVision     = uint32(C.RC_CONSOLE_COLECOVISION)
	ConsoleGameBoyAdvance   = uint32(C.RC_CONSOLE_GAMEBOY_ADVANCE)
	ConsoleIntertonVC4000   = uint32(C.RC_CONSOLE_INTERTON_VC_4000)
	ConsoleMagnavoxOdyssey2 = uint32(C.RC_CONSOLE_MAGNAVOX_ODYSSEY2)
	ConsoleZXSpectrum       = uint32(C.RC_CONSOLE_ZX_SPECTRUM)
	ConsoleWonderSwan       = uint32(C.RC_CONSOLE_WONDERSWAN)
	ConsoleWASM4            = uint32(C.RC_CONSOLE_WASM4)
	ConsoleVirtualBoy       = uint32(C.RC_CONSOLE_VIRTUAL_BOY)
	ConsoleVectrex          = uint32(C.RC_CONSOLE_VECTREX)
	ConsoleUzebox           = uint32(C.RC_CONSOLE_UZEBOX)
	ConsoleTIC80            = uint32(C.RC_CONSOLE_TIC80)
	ConsoleTI83             = uint32(C.RC_CONSOLE_TI83)
	ConsoleSupervision      = uint32(C.RC_CONSOLE_SUPERVISION)
	ConsoleSG1000           = uint32(C.RC_CONSOLE_SG1000)
	ConsoleSega32X          = uint32(C.RC_CONSOLE_SEGA_32X)
	ConsolePokemonMini      = uint32(C.RC_CONSOLE_POKEMON_MINI)
	ConsolePC8800           = uint32(C.RC_CONSOLE_PC8800)
	ConsoleOric             = uint32(C.RC_CONSOLE_ORIC)
	ConsoleNeoGeoPocket     = uint32(C.RC_CONSOLE_NEOGEO_POCKET)
	ConsoleMSX              = uint32(C.RC_CONSOLE_MSX)
	ConsoleMegaDuck         = uint32(C.RC_CONSOLE_MEGADUCK)
	ConsoleMasterSystem     = uint32(C.RC_CONSOLE_MASTER_SYSTEM)
	ConsoleIntellivision    = uint32(C.RC_CONSOLE_INTELLIVISION)
	ConsoleGameGear         = uint32(C.RC_CONSOLE_GAME_GEAR)
	ConsoleGameBoyColor     = uint32(C.RC_CONSOLE_GAMEBOY_COLOR)
	ConsoleGameBoy          = uint32(C.RC_CONSOLE_GAMEBOY)
	ConsoleChannelF         = uint32(C.RC_CONSOLE_FAIRCHILD_CHANNEL_F)
	ConsoleElektor          = uint32(C.RC_CONSOLE_ELEKTOR_TV_GAMES_COMPUTER)
	ConsoleCommodore64      = uint32(C.RC_CONSOLE_COMMODORE_64)
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
