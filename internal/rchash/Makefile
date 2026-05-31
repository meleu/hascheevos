# Builds a ROM-only static lib (librchash.a) from the rcheevos submodule.
# No disc/encrypted/zip support -> no zlib/aes dependencies.

CFLAGS  = -O2 -DRC_HASH_NO_DISC -DRC_HASH_NO_ENCRYPTED -DRC_HASH_NO_ZIP
INCLUDES = -Ircheevos/include -Ircheevos/src

SRC = \
	rcheevos/src/rhash/hash.c \
	rcheevos/src/rhash/hash_rom.c \
	rcheevos/src/rhash/md5.c \
	rcheevos/src/rc_compat.c \
	rcheevos/src/rc_util.c \
	rcheevos/src/rcheevos/consoleinfo.c \
	rcheevos/src/rc_version.c

OBJ = $(SRC:.c=.o)

LIB = librchash.a

.PHONY: lib clean

lib: $(LIB)

$(LIB): $(OBJ)
	ar rcs $@ $(OBJ)

%.o: %.c
	$(CC) $(CFLAGS) $(INCLUDES) -c $< -o $@

clean:
	rm -f $(OBJ) $(LIB)
