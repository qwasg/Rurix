/* SPDX-License-Identifier: MIT OR Apache-2.0 */
/* G8.3 M83 transitional texture codec C ABI(rurix-basis-sys). */
#ifndef RURIX_BASIS_SHIM_H
#define RURIX_BASIS_SHIM_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RurixBasisBuf {
    uint8_t *data;
    size_t len;
} RurixBasisBuf;

/* Static NUL-terminated version string; must match VENDOR.md pin. */
const char *rurix_basis_version(void);

void rurix_basis_buf_free(RurixBasisBuf *buf);

/* Encode RGBA8 (row-major, tightly packed) to BC7_UNORM blocks.
   width/height must be >0; edges are clamp-to-edge padded to 4x4.
   Returns 0 on success, nonzero on failure. out takes ownership of heap bytes. */
int rurix_basis_encode_bc7_rgba8(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    RurixBasisBuf *out);

int rurix_basis_encode_bc1_rgba8(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    RurixBasisBuf *out);

/* ASTC 4x4 LDR: each block is a real unbounded void-extent with block-mean color. */
int rurix_basis_encode_astc4x4_rgba8(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    RurixBasisBuf *out);

#ifdef __cplusplus
}
#endif

#endif /* RURIX_BASIS_SHIM_H */
