/* cheevoshash.c */

#include <stdio.h>
#include <stdlib.h>
#include <stdint.h>
#include <unistd.h>
#include <string.h>
#include <sys/stat.h>

/*****************************************************************************
 * start of MD5 stuff
 ****************************************************************************/
typedef unsigned int MD5_u32plus;

typedef struct {
    MD5_u32plus lo, hi;
    MD5_u32plus a, b, c, d;
    unsigned char buffer[64];
    MD5_u32plus block[16];
} MD5_CTX;

/*
 * The basic MD5 functions.
 *
 * F and G are optimized compared to their RFC 1321 definitions for
 * architectures that lack an AND-NOT instruction, just like in Colin Plumb's
 * implementation.
 */
#define MD5_F(x, y, z)   ((z) ^ ((x) & ((y) ^ (z))))
#define MD5_G(x, y, z)   ((y) ^ ((z) & ((x) ^ (y))))
#define MD5_H(x, y, z)   (((x) ^ (y)) ^ (z))
#define MD5_H2(x, y, z)  ((x) ^ ((y) ^ (z)))
#define MD5_I(x, y, z)   ((y) ^ ((x) | ~(z)))

/*
 * The MD5 transformation for all four rounds.
 */
#define MD5_STEP(f, a, b, c, d, x, t, s) \
    (a) += f((b), (c), (d)) + (x) + (t); \
    (a) = (((a) << (s)) | (((a) & 0xffffffff) >> (32 - (s)))); \
    (a) += (b);

/*
 * MD5_SET reads 4 input bytes in little-endian byte order and stores them
 * in a properly aligned word in host byte order.
 */
#define MD5_SET(n) \
    (*(MD5_u32plus *)&ptr[(n) * 4])
#define MD5_GET(n) \
    MD5_SET(n)


static const void *MD5_body(MD5_CTX *ctx, const void *data, unsigned long size) {
    const unsigned char *ptr;
    MD5_u32plus a, b, c, d;
    MD5_u32plus saved_a, saved_b, saved_c, saved_d;

    ptr = (const unsigned char *)data;

    a = ctx->a;
    b = ctx->b;
    c = ctx->c;
    d = ctx->d;

    do {
        saved_a = a;
        saved_b = b;
        saved_c = c;
        saved_d = d;

/* Round 1 */
        MD5_STEP(MD5_F, a, b, c, d, MD5_SET(0), 0xd76aa478, 7)
        MD5_STEP(MD5_F, d, a, b, c, MD5_SET(1), 0xe8c7b756, 12)
        MD5_STEP(MD5_F, c, d, a, b, MD5_SET(2), 0x242070db, 17)
        MD5_STEP(MD5_F, b, c, d, a, MD5_SET(3), 0xc1bdceee, 22)
        MD5_STEP(MD5_F, a, b, c, d, MD5_SET(4), 0xf57c0faf, 7)
        MD5_STEP(MD5_F, d, a, b, c, MD5_SET(5), 0x4787c62a, 12)
        MD5_STEP(MD5_F, c, d, a, b, MD5_SET(6), 0xa8304613, 17)
        MD5_STEP(MD5_F, b, c, d, a, MD5_SET(7), 0xfd469501, 22)
        MD5_STEP(MD5_F, a, b, c, d, MD5_SET(8), 0x698098d8, 7)
        MD5_STEP(MD5_F, d, a, b, c, MD5_SET(9), 0x8b44f7af, 12)
        MD5_STEP(MD5_F, c, d, a, b, MD5_SET(10), 0xffff5bb1, 17)
        MD5_STEP(MD5_F, b, c, d, a, MD5_SET(11), 0x895cd7be, 22)
        MD5_STEP(MD5_F, a, b, c, d, MD5_SET(12), 0x6b901122, 7)
        MD5_STEP(MD5_F, d, a, b, c, MD5_SET(13), 0xfd987193, 12)
        MD5_STEP(MD5_F, c, d, a, b, MD5_SET(14), 0xa679438e, 17)
        MD5_STEP(MD5_F, b, c, d, a, MD5_SET(15), 0x49b40821, 22)

/* Round 2 */
        MD5_STEP(MD5_G, a, b, c, d, MD5_GET(1), 0xf61e2562, 5)
        MD5_STEP(MD5_G, d, a, b, c, MD5_GET(6), 0xc040b340, 9)
        MD5_STEP(MD5_G, c, d, a, b, MD5_GET(11), 0x265e5a51, 14)
        MD5_STEP(MD5_G, b, c, d, a, MD5_GET(0), 0xe9b6c7aa, 20)
        MD5_STEP(MD5_G, a, b, c, d, MD5_GET(5), 0xd62f105d, 5)
        MD5_STEP(MD5_G, d, a, b, c, MD5_GET(10), 0x02441453, 9)
        MD5_STEP(MD5_G, c, d, a, b, MD5_GET(15), 0xd8a1e681, 14)
        MD5_STEP(MD5_G, b, c, d, a, MD5_GET(4), 0xe7d3fbc8, 20)
        MD5_STEP(MD5_G, a, b, c, d, MD5_GET(9), 0x21e1cde6, 5)
        MD5_STEP(MD5_G, d, a, b, c, MD5_GET(14), 0xc33707d6, 9)
        MD5_STEP(MD5_G, c, d, a, b, MD5_GET(3), 0xf4d50d87, 14)
        MD5_STEP(MD5_G, b, c, d, a, MD5_GET(8), 0x455a14ed, 20)
        MD5_STEP(MD5_G, a, b, c, d, MD5_GET(13), 0xa9e3e905, 5)
        MD5_STEP(MD5_G, d, a, b, c, MD5_GET(2), 0xfcefa3f8, 9)
        MD5_STEP(MD5_G, c, d, a, b, MD5_GET(7), 0x676f02d9, 14)
        MD5_STEP(MD5_G, b, c, d, a, MD5_GET(12), 0x8d2a4c8a, 20)

/* Round 3 */
        MD5_STEP(MD5_H, a, b, c, d, MD5_GET(5), 0xfffa3942, 4)
        MD5_STEP(MD5_H2, d, a, b, c, MD5_GET(8), 0x8771f681, 11)
        MD5_STEP(MD5_H, c, d, a, b, MD5_GET(11), 0x6d9d6122, 16)
        MD5_STEP(MD5_H2, b, c, d, a, MD5_GET(14), 0xfde5380c, 23)
        MD5_STEP(MD5_H, a, b, c, d, MD5_GET(1), 0xa4beea44, 4)
        MD5_STEP(MD5_H2, d, a, b, c, MD5_GET(4), 0x4bdecfa9, 11)
        MD5_STEP(MD5_H, c, d, a, b, MD5_GET(7), 0xf6bb4b60, 16)
        MD5_STEP(MD5_H2, b, c, d, a, MD5_GET(10), 0xbebfbc70, 23)
        MD5_STEP(MD5_H, a, b, c, d, MD5_GET(13), 0x289b7ec6, 4)
        MD5_STEP(MD5_H2, d, a, b, c, MD5_GET(0), 0xeaa127fa, 11)
        MD5_STEP(MD5_H, c, d, a, b, MD5_GET(3), 0xd4ef3085, 16)
        MD5_STEP(MD5_H2, b, c, d, a, MD5_GET(6), 0x04881d05, 23)
        MD5_STEP(MD5_H, a, b, c, d, MD5_GET(9), 0xd9d4d039, 4)
        MD5_STEP(MD5_H2, d, a, b, c, MD5_GET(12), 0xe6db99e5, 11)
        MD5_STEP(MD5_H, c, d, a, b, MD5_GET(15), 0x1fa27cf8, 16)
        MD5_STEP(MD5_H2, b, c, d, a, MD5_GET(2), 0xc4ac5665, 23)

/* Round 4 */
        MD5_STEP(MD5_I, a, b, c, d, MD5_GET(0), 0xf4292244, 6)
        MD5_STEP(MD5_I, d, a, b, c, MD5_GET(7), 0x432aff97, 10)
        MD5_STEP(MD5_I, c, d, a, b, MD5_GET(14), 0xab9423a7, 15)
        MD5_STEP(MD5_I, b, c, d, a, MD5_GET(5), 0xfc93a039, 21)
        MD5_STEP(MD5_I, a, b, c, d, MD5_GET(12), 0x655b59c3, 6)
        MD5_STEP(MD5_I, d, a, b, c, MD5_GET(3), 0x8f0ccc92, 10)
        MD5_STEP(MD5_I, c, d, a, b, MD5_GET(10), 0xffeff47d, 15)
        MD5_STEP(MD5_I, b, c, d, a, MD5_GET(1), 0x85845dd1, 21)
        MD5_STEP(MD5_I, a, b, c, d, MD5_GET(8), 0x6fa87e4f, 6)
        MD5_STEP(MD5_I, d, a, b, c, MD5_GET(15), 0xfe2ce6e0, 10)
        MD5_STEP(MD5_I, c, d, a, b, MD5_GET(6), 0xa3014314, 15)
        MD5_STEP(MD5_I, b, c, d, a, MD5_GET(13), 0x4e0811a1, 21)
        MD5_STEP(MD5_I, a, b, c, d, MD5_GET(4), 0xf7537e82, 6)
        MD5_STEP(MD5_I, d, a, b, c, MD5_GET(11), 0xbd3af235, 10)
        MD5_STEP(MD5_I, c, d, a, b, MD5_GET(2), 0x2ad7d2bb, 15)
        MD5_STEP(MD5_I, b, c, d, a, MD5_GET(9), 0xeb86d391, 21)

        a += saved_a;
        b += saved_b;
        c += saved_c;
        d += saved_d;

        ptr += 64;
    } while (size -= 64);

    ctx->a = a;
    ctx->b = b;
    ctx->c = c;
    ctx->d = d;

    return ptr;
}


void MD5_Init(MD5_CTX *ctx) {
    ctx->a = 0x67452301;
    ctx->b = 0xefcdab89;
    ctx->c = 0x98badcfe;
    ctx->d = 0x10325476;

    ctx->lo = 0;
    ctx->hi = 0;
}


void MD5_Update(MD5_CTX *ctx, const void *data, unsigned long size) {
    MD5_u32plus saved_lo;
    unsigned long used, available;

    saved_lo = ctx->lo;
    if ((ctx->lo = (saved_lo + size) & 0x1fffffff) < saved_lo)
        ctx->hi++;
    ctx->hi += size >> 29;

    used = saved_lo & 0x3f;

    if (used) {
        available = 64 - used;

        if (size < available) {
            memcpy(&ctx->buffer[used], data, size);
            return;
        }

        memcpy(&ctx->buffer[used], data, available);
        data = (const unsigned char *)data + available;
        size -= available;
        MD5_body(ctx, ctx->buffer, 64);
    }

    if (size >= 64) {
        data = MD5_body(ctx, data, size & ~(unsigned long)0x3f);
        size &= 0x3f;
    }

    memcpy(ctx->buffer, data, size);
}


void MD5_Final(unsigned char *result, MD5_CTX *ctx) {
    unsigned long used, available;

    used = ctx->lo & 0x3f;

    ctx->buffer[used++] = 0x80;

    available = 64 - used;

    if (available < 8) {
        memset(&ctx->buffer[used], 0, available);
        MD5_body(ctx, ctx->buffer, 64);
        used = 0;
        available = 64;
    }

    memset(&ctx->buffer[used], 0, available - 8);

    ctx->lo <<= 3;
    ctx->buffer[56] = ctx->lo;
    ctx->buffer[57] = ctx->lo >> 8;
    ctx->buffer[58] = ctx->lo >> 16;
    ctx->buffer[59] = ctx->lo >> 24;
    ctx->buffer[60] = ctx->hi;
    ctx->buffer[61] = ctx->hi >> 8;
    ctx->buffer[62] = ctx->hi >> 16;
    ctx->buffer[63] = ctx->hi >> 24;

    MD5_body(ctx, ctx->buffer, 64);

    result[0] = ctx->a;
    result[1] = ctx->a >> 8;
    result[2] = ctx->a >> 16;
    result[3] = ctx->a >> 24;
    result[4] = ctx->b;
    result[5] = ctx->b >> 8;
    result[6] = ctx->b >> 16;
    result[7] = ctx->b >> 24;
    result[8] = ctx->c;
    result[9] = ctx->c >> 8;
    result[10] = ctx->c >> 16;
    result[11] = ctx->c >> 24;
    result[12] = ctx->d;
    result[13] = ctx->d >> 8;
    result[14] = ctx->d >> 16;
    result[15] = ctx->d >> 24;

    memset(ctx, 0, sizeof(*ctx));
}
/*****************************************************************************
 * end of MD5 stuff
 ****************************************************************************/


/*****************************************************************************
 * start of RetroArch/cheevos stuff
 ****************************************************************************/
#define CHEEVOS_SIX_MB     ( 6 * 1024 * 1024)
#define CHEEVOS_EIGHT_MB   ( 8 * 1024 * 1024)
#define ARRAY_SIZE(array)  (sizeof(array) / sizeof(array[0]))
#define HASH_STR_SIZE      33


/* inspired on cheevos_finder_t */
typedef struct {
    char *(*finder)(const char *);
    const char *name;
    const uint32_t *ext_hashes;
} cheevos_hash_calculator_t;


static unsigned cheevos_next_power_of_2(unsigned n) {
    n--;

    n |= n >> 1;
    n |= n >> 2;
    n |= n >> 4;
    n |= n >> 8;
    n |= n >> 16;

    return n + 1;
}


static uint32_t cheevos_djb2(const char* str, size_t length) {
    const unsigned char *aux = (const unsigned char*)str;
    const unsigned char *end = aux + length;
    uint32_t            hash = 5381;

    while (aux < end)
        hash = (hash << 5) + hash + *aux++;

    return hash;
}


static size_t cheevos_eval_md5(
    const char *path,
    size_t offset,
    size_t max_size,
    MD5_CTX *ctx)
{
    struct stat st;
    size_t size = 0;
    FILE *file = fopen(path, "r");

    MD5_Init(ctx);

    if (!file)
        return 0;

    if(stat(path, &st) == 0)
        size = st.st_size;
    else {
        perror("cheevos_eval_md5()");
        exit(1);
    }
  
    if (max_size == 0)
        max_size = size;
  
    if (size - offset < max_size)
        max_size = size - offset;

    fseek(file, offset, SEEK_SET);
    size = 0;

    for (;;) {
        uint8_t buffer[4096];
        ssize_t num_read;
        size_t to_read = sizeof(buffer);

        if (to_read > max_size)
            to_read = max_size;

        num_read = fread((void*)buffer, 1, to_read, file);

        if (num_read <= 0)
            break;

        MD5_Update(ctx, (void*)buffer, num_read);
        size += num_read;
     
        if (max_size != 0) {
            max_size -= num_read;

        if (max_size == 0)
                break;
        }
    }

    fclose(file);
    return size;
}


static void cheevos_fill_md5(size_t size, char fill, MD5_CTX *ctx) {
    char buffer[4096];

    memset((void*)buffer, fill, sizeof(buffer));

    while (size > 0) {
        size_t len = sizeof(buffer);

        if (len > size)
            len = size;

        MD5_Update(ctx, (void*)buffer, len);
        size -= len;
    }
}


char *hash_str(uint8_t *hash) {
    static char hash_str[HASH_STR_SIZE] = "\0";

    snprintf(hash_str, HASH_STR_SIZE,
        "%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x%02x",
        hash[ 0], hash[ 1], hash[ 2], hash[ 3], hash[ 4], hash[ 5], hash[ 6], hash[ 7],
        hash[ 8], hash[ 9], hash[10], hash[11], hash[12], hash[13], hash[14], hash[15]
    );
    return hash_str;
}


char *cheevos_hash_generic(const char *path) {
    MD5_CTX ctx;
    uint8_t hash[16];
    size_t size      = cheevos_eval_md5(path, 0, 0, &ctx);

    hash[0] = '\0';

    MD5_Final(hash, &ctx);

    if (!size)
        return 0;

    return hash_str(hash);
}


char *cheevos_hash_snes(const char *path) {
    MD5_CTX ctx;
    uint8_t hash[16];
    size_t count = cheevos_eval_md5(path, 0, 0, &ctx);

    if (count == 0) {
        MD5_Final(hash, &ctx);
        return 0;
    }

    if (count < CHEEVOS_EIGHT_MB)
        cheevos_fill_md5(CHEEVOS_EIGHT_MB - count, 0, &ctx);
   
    MD5_Final(hash, &ctx);
    return hash_str(hash);
}


char *cheevos_hash_genesis(const char *path) {
    MD5_CTX ctx;
    uint8_t hash[16];
    size_t count = cheevos_eval_md5(path, 0, 0, &ctx);

    if (count == 0) {
        MD5_Final(hash, &ctx);
        return 0;
    }

    if (count < CHEEVOS_SIX_MB)
        cheevos_fill_md5(CHEEVOS_SIX_MB - count, 0, &ctx);
   
    MD5_Final(hash, &ctx);
    return hash_str(hash);
}


char *cheevos_hash_nes(const char *path) {
   /* Note about the references to the FCEU emulator below. There is no
    * core-specific code in this function, it's rather Retro Achievements
    * specific code that must be followed to the letter so we compute
    * the correct ROM hash. Retro Achievements does indeed use some
    * FCEU related method to compute the hash, since its NES emulator
    * is based on it. */
    struct {
        uint8_t id[4]; /* NES^Z */
        uint8_t rom_size;
        uint8_t vrom_size;
        uint8_t rom_type;
        uint8_t rom_type2;
        uint8_t reserve[8];
    } header;

    size_t rom_size, offset, count;
    MD5_CTX ctx;
    uint8_t hash[16];

    size_t bytes;
    FILE *file;
    ssize_t num_read;
    int mapper_no;
    int round;

    file = fopen(path, "r");

    if (!file)
        return 0;

    num_read = fread((void*)&header, 1, sizeof(header), file);
    fclose(file);

    if (num_read < (ssize_t)sizeof(header))
        return 0;

    if (   header.id[0] != 'N'
        || header.id[1] != 'E'
        || header.id[2] != 'S'
        || header.id[3] != 0x1a)
        return 0;

    if (header.rom_size)
        rom_size = cheevos_next_power_of_2(header.rom_size);
    else
        rom_size = 256;

    /* from FCEU core - compute size using the cart mapper */
    mapper_no = (header.rom_type >> 4) | (header.rom_type2 & 0xF0);

    /* for games not to the power of 2, so we just read enough
     * PRG rom from it, but we have to keep ROM_size to the power of 2
     * since PRGCartMapping wants ROM_size to be to the power of 2
     * so instead if not to power of 2, we just use head.ROM_size when
     * we use FCEU_read. */
    round = mapper_no != 53 && mapper_no != 198 && mapper_no != 228;
    bytes = (round) ? rom_size : header.rom_size;

    /* from FCEU core - check if Trainer included in ROM data */
    offset = sizeof(header) + (header.rom_type & 4 ? sizeof(header) : 0);

    MD5_Init(&ctx);
    count = cheevos_eval_md5(path, offset, 0x4000 * bytes, &ctx);
    count = 0x4000 * bytes - count;
    cheevos_fill_md5(count, (char)0xff, &ctx);
    MD5_Final(hash, &ctx);

    return hash_str(hash);
}


int cheevos_print_rom_hashes(const char *path) {
    static const uint32_t genesis_exts[] =
    {
        0x0b888feeU, /* mdx */
        0x005978b6U, /* md  */
        0x0b88aa89U, /* smd */
        0x0b88767fU, /* gen */
        0x0b8861beU, /* bin */
        0x0b886782U, /* cue */
        0x0b8880d0U, /* iso */
        0x0b88aa98U, /* sms */
        0x005977f3U, /* gg  */
        0x0059797fU, /* sg  */
        0
    };

    static const uint32_t snes_exts[] =
    {
        0x0b88aa88U, /* smc */
        0x0b8872bbU, /* fig */
        0x0b88a9a1U, /* sfc */
        0x0b887623U, /* gd3 */
        0x0b887627U, /* gd7 */
        0x0b886bf3U, /* dx2 */
        0x0b886312U, /* bsx */
        0x0b88abd2U, /* swc */
        0
    };

    static cheevos_hash_calculator_t finders[] =
    {
        {cheevos_hash_snes,    "SNES",      snes_exts},
        {cheevos_hash_genesis, "Genesis",   genesis_exts},
        {cheevos_hash_nes,     "NES",       NULL},
        {cheevos_hash_generic, "plain MD5", NULL},
    };

    unsigned i;
    char *hash_str;

    for (i = 0; i < ARRAY_SIZE(finders); i++) {
        if (finders[i].ext_hashes) {
            /* get the file extension */
            const char *ext = strrchr(path, '.') + 1;

            while (ext) {
                int j;
                unsigned hash;
                const char *end = strchr(ext, '|');

                if (end) {
                    hash = cheevos_djb2(ext, end - ext);
                    ext = end + 1;
                } else {
                   hash = cheevos_djb2(ext, strlen(ext));
                   ext = NULL;
                }

                for (j = 0; finders[i].ext_hashes[j]; j++) {
                    if (finders[i].ext_hashes[j] == hash) {
                        if(hash_str = finders[i].finder(path))
                            printf("%s: %s\n", finders[i].name, hash_str);
                        ext = NULL; /* force next finder */
                        break;
                    }
                } /* end of for j */
            } /* end of while(ext) */
        } /* end of if(finders...) */
    }

    for (i = 0; i < ARRAY_SIZE(finders); i++) {
        if (finders[i].ext_hashes)
            continue;
        if(hash_str = finders[i].finder(path))
            printf("%s: %s\n", finders[i].name, hash_str);
    }
}
/*****************************************************************************
 * end of RetroArch/cheevos stuff
 ****************************************************************************/

int main(int argc, char **argv) {
    if(argc < 2) {
        fputs("ERROR: missing argument.\n", stderr);
        fprintf(stderr, "USAGE: %s file\n", argv[0]);
        exit(1);
    }

    if(argc > 2) {
        fputs("WARNING: ignoring extra arguments.\n", stderr);
        fprintf(stderr, "USAGE: %s file\n", argv[0]);
    }

    if(access(argv[1], R_OK) == -1) {
        perror(argv[1]);
        exit(1);
    }

    cheevos_print_rom_hashes(argv[1]);
}
