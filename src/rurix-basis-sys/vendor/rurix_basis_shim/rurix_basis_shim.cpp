/* SPDX-License-Identifier: MIT OR Apache-2.0 */
/* G8.3 M83 transitional deterministic BC1/BC7 + ASTC void-extent encoder.
   Integer-only; threads=1; no zstd. Full basis_universal pending(VENDOR.md). */

#include "rurix_basis_shim.h"

#include <algorithm>
#include <cstdint>
#include <cstring>
#include <new>
#include <vector>

namespace {

constexpr const char *kVersion = "rurix-basis-transitional/0.1.0";

struct BitWriter {
    uint8_t bytes[16]{};
    int bit_pos = 0;

    void put(uint32_t value, int nbits) {
        for (int i = 0; i < nbits; ++i) {
            const int byte_i = bit_pos >> 3;
            const int bit_i = bit_pos & 7;
            if ((value >> i) & 1u) {
                bytes[byte_i] = static_cast<uint8_t>(bytes[byte_i] | (1u << bit_i));
            }
            ++bit_pos;
        }
    }
};

void sample_rgba(
    const uint8_t *rgba,
    uint32_t w,
    uint32_t h,
    uint32_t x,
    uint32_t y,
    uint8_t out[4]) {
    const uint32_t cx = std::min(x, w - 1);
    const uint32_t cy = std::min(y, h - 1);
    const size_t i = (static_cast<size_t>(cy) * w + cx) * 4;
    out[0] = rgba[i];
    out[1] = rgba[i + 1];
    out[2] = rgba[i + 2];
    out[3] = rgba[i + 3];
}

void load_block(
    const uint8_t *rgba,
    uint32_t w,
    uint32_t h,
    uint32_t bx,
    uint32_t by,
    uint8_t block[16][4]) {
    for (uint32_t ty = 0; ty < 4; ++ty) {
        for (uint32_t tx = 0; tx < 4; ++tx) {
            sample_rgba(rgba, w, h, bx + tx, by + ty, block[ty * 4 + tx]);
        }
    }
}

uint8_t quant7(uint8_t v) {
    /* Expand-aware: store top 7 bits such that (q<<1)|p ≈ v for p chosen later. */
    return static_cast<uint8_t>(v >> 1);
}

int color_dist2(const uint8_t a[4], const uint8_t b[4]) {
    int d = 0;
    for (int c = 0; c < 4; ++c) {
        const int t = static_cast<int>(a[c]) - static_cast<int>(b[c]);
        d += t * t;
    }
    return d;
}

void interpolate_mode6(
    const uint8_t e0[4],
    const uint8_t e1[4],
    int index,
    uint8_t out[4]) {
    /* BC7 mode 6 uses 4-bit weight table (same as mode 1/3/7 color weights). */
    static const int kW[16] = {
        0, 9, 18, 27, 37, 46, 55, 64, 74, 83, 92, 101, 111, 120, 129, 138};
    const int w = kW[index];
    for (int c = 0; c < 4; ++c) {
        const int v = (e0[c] * (64 - w) + e1[c] * w + 32) >> 6;
        out[c] = static_cast<uint8_t>(std::min(255, std::max(0, v)));
    }
}

void encode_bc7_block(const uint8_t block[16][4], uint8_t out[16]) {
    /* Pick endpoints = min/max per-channel extremes by luminance proxy. */
    int best_lo = 0;
    int best_hi = 0;
    int best_span = -1;
    for (int i = 0; i < 16; ++i) {
        for (int j = i; j < 16; ++j) {
            const int span = color_dist2(block[i], block[j]);
            if (span > best_span) {
                best_span = span;
                best_lo = i;
                best_hi = j;
            }
        }
    }
    uint8_t e0[4];
    uint8_t e1[4];
    for (int c = 0; c < 4; ++c) {
        e0[c] = block[best_lo][c];
        e1[c] = block[best_hi][c];
    }

    uint8_t q0[4];
    uint8_t q1[4];
    for (int c = 0; c < 4; ++c) {
        q0[c] = quant7(e0[c]);
        q1[c] = quant7(e1[c]);
    }
    /* Mode 6: one p-bit per endpoint applied to all channels. Majority vote. */
    int vote0 = 0;
    int vote1 = 0;
    for (int c = 0; c < 4; ++c) {
        const int r0a = static_cast<int>((q0[c] << 1) | 0);
        const int r0b = static_cast<int>((q0[c] << 1) | 1);
        vote0 += ((r0b - e0[c]) * (r0b - e0[c]) < (r0a - e0[c]) * (r0a - e0[c])) ? 1 : 0;
        const int r1a = static_cast<int>((q1[c] << 1) | 0);
        const int r1b = static_cast<int>((q1[c] << 1) | 1);
        vote1 += ((r1b - e1[c]) * (r1b - e1[c]) < (r1a - e1[c]) * (r1a - e1[c])) ? 1 : 0;
    }
    uint8_t p0 = vote0 >= 2 ? 1 : 0;
    uint8_t p1 = vote1 >= 2 ? 1 : 0;

    uint8_t ep0[4];
    uint8_t ep1[4];
    for (int c = 0; c < 4; ++c) {
        ep0[c] = static_cast<uint8_t>((q0[c] << 1) | p0);
        ep1[c] = static_cast<uint8_t>((q1[c] << 1) | p1);
    }

    uint8_t indices[16];
    for (int i = 0; i < 16; ++i) {
        int best_i = 0;
        int best_d = 1 << 30;
        for (int idx = 0; idx < 16; ++idx) {
            uint8_t pred[4];
            interpolate_mode6(ep0, ep1, idx, pred);
            const int d = color_dist2(block[i], pred);
            if (d < best_d) {
                best_d = d;
                best_i = idx;
            }
        }
        indices[i] = static_cast<uint8_t>(best_i);
    }
    /* Fix-up index MSB must be 0 for texel 0 in mode 6. */
    if (indices[0] & 0x8) {
        /* Swap endpoints and invert indices. */
        for (int c = 0; c < 4; ++c) {
            std::swap(q0[c], q1[c]);
        }
        std::swap(p0, p1);
        for (int i = 0; i < 16; ++i) {
            indices[i] = static_cast<uint8_t>(15 - indices[i]);
        }
    }

    BitWriter bw;
    /* Mode 6 = 7-bit pattern 0000001 stored in the lowest bits (LSB-first → value 1). */
    bw.put(1u, 7);
    /* Endpoints: R0 R1 G0 G1 B0 B1 A0 A1 each 7 bits. */
    bw.put(q0[0], 7);
    bw.put(q1[0], 7);
    bw.put(q0[1], 7);
    bw.put(q1[1], 7);
    bw.put(q0[2], 7);
    bw.put(q1[2], 7);
    bw.put(q0[3], 7);
    bw.put(q1[3], 7);
    bw.put(p0, 1);
    bw.put(p1, 1);
    /* Indices: texel0 is 3 bits (MSB omitted), others 4 bits. */
    bw.put(indices[0] & 0x7u, 3);
    for (int i = 1; i < 16; ++i) {
        bw.put(indices[i], 4);
    }
    std::memcpy(out, bw.bytes, 16);
}

uint16_t rgb565(uint8_t r, uint8_t g, uint8_t b) {
    return static_cast<uint16_t>(((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3));
}

void encode_bc1_block(const uint8_t block[16][4], uint8_t out[8]) {
    int lo = 0;
    int hi = 0;
    int best = -1;
    for (int i = 0; i < 16; ++i) {
        for (int j = i; j < 16; ++j) {
            const int d = color_dist2(block[i], block[j]);
            if (d > best) {
                best = d;
                lo = i;
                hi = j;
            }
        }
    }
    uint16_t c0 = rgb565(block[lo][0], block[lo][1], block[lo][2]);
    uint16_t c1 = rgb565(block[hi][0], block[hi][1], block[hi][2]);
    if (c0 < c1) {
        std::swap(c0, c1);
        std::swap(lo, hi);
    }
    uint8_t palette[4][3];
    palette[0][0] = block[lo][0];
    palette[0][1] = block[lo][1];
    palette[0][2] = block[lo][2];
    palette[1][0] = block[hi][0];
    palette[1][1] = block[hi][1];
    palette[1][2] = block[hi][2];
    for (int c = 0; c < 3; ++c) {
        palette[2][c] = static_cast<uint8_t>((2 * palette[0][c] + palette[1][c]) / 3);
        palette[3][c] = static_cast<uint8_t>((palette[0][c] + 2 * palette[1][c]) / 3);
    }
    uint32_t indices = 0;
    for (int i = 0; i < 16; ++i) {
        int best_i = 0;
        int best_d = 1 << 30;
        for (int p = 0; p < 4; ++p) {
            uint8_t tmp[4] = {palette[p][0], palette[p][1], palette[p][2], 255};
            const int d = color_dist2(block[i], tmp);
            if (d < best_d) {
                best_d = d;
                best_i = p;
            }
        }
        indices |= static_cast<uint32_t>(best_i) << (2 * i);
    }
    out[0] = static_cast<uint8_t>(c0 & 0xff);
    out[1] = static_cast<uint8_t>(c0 >> 8);
    out[2] = static_cast<uint8_t>(c1 & 0xff);
    out[3] = static_cast<uint8_t>(c1 >> 8);
    out[4] = static_cast<uint8_t>(indices & 0xff);
    out[5] = static_cast<uint8_t>((indices >> 8) & 0xff);
    out[6] = static_cast<uint8_t>((indices >> 16) & 0xff);
    out[7] = static_cast<uint8_t>((indices >> 24) & 0xff);
}

void encode_astc_void_extent(const uint8_t block[16][4], uint8_t out[16]) {
    /* LDR unbounded void-extent: bits[10:0]=11111111100, coords all-1, color UNORM16. */
    uint32_t sum[4] = {0, 0, 0, 0};
    for (int i = 0; i < 16; ++i) {
        for (int c = 0; c < 4; ++c) {
            sum[c] += block[i][c];
        }
    }
    const uint16_t cr = static_cast<uint16_t>(((sum[0] / 16) << 8) | (sum[0] / 16));
    const uint16_t cg = static_cast<uint16_t>(((sum[1] / 16) << 8) | (sum[1] / 16));
    const uint16_t cb = static_cast<uint16_t>(((sum[2] / 16) << 8) | (sum[2] / 16));
    const uint16_t ca = static_cast<uint16_t>(((sum[3] / 16) << 8) | (sum[3] / 16));

    std::memset(out, 0, 16);
    /* Pack low 64 bits: mode + unbounded coords. */
    uint64_t low = 0;
    low |= 0x7FCu;                 /* bits 0..10 */
    /* bits 11..12 = 00 (LDR) already */
    low |= (0xFFFull << 13);       /* min S */
    low |= (0xFFFull << 25);       /* max S */
    low |= (0xFFFull << 37);       /* min T */
    low |= (0xFFFull << 49);       /* max T (bits 49..60) */
    std::memcpy(out, &low, 8);
    out[8] = static_cast<uint8_t>(cr & 0xff);
    out[9] = static_cast<uint8_t>(cr >> 8);
    out[10] = static_cast<uint8_t>(cg & 0xff);
    out[11] = static_cast<uint8_t>(cg >> 8);
    out[12] = static_cast<uint8_t>(cb & 0xff);
    out[13] = static_cast<uint8_t>(cb >> 8);
    out[14] = static_cast<uint8_t>(ca & 0xff);
    out[15] = static_cast<uint8_t>(ca >> 8);
}

int encode_blocks(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    size_t block_bytes,
    void (*encode_one)(const uint8_t[16][4], uint8_t *),
    RurixBasisBuf *out) {
    if (!rgba || !out || width == 0 || height == 0) {
        return 1;
    }
    out->data = nullptr;
    out->len = 0;
    const uint32_t bw = (width + 3u) / 4u;
    const uint32_t bh = (height + 3u) / 4u;
    const size_t total = static_cast<size_t>(bw) * bh * block_bytes;
    auto *buf = new (std::nothrow) uint8_t[total];
    if (!buf) {
        return 2;
    }
    size_t off = 0;
    uint8_t block[16][4];
    std::vector<uint8_t> tmp(block_bytes);
    for (uint32_t by = 0; by < bh; ++by) {
        for (uint32_t bx = 0; bx < bw; ++bx) {
            load_block(rgba, width, height, bx * 4u, by * 4u, block);
            encode_one(block, tmp.data());
            std::memcpy(buf + off, tmp.data(), block_bytes);
            off += block_bytes;
        }
    }
    out->data = buf;
    out->len = total;
    return 0;
}

void encode_bc7_adapt(const uint8_t block[16][4], uint8_t *out) {
    encode_bc7_block(block, out);
}
void encode_bc1_adapt(const uint8_t block[16][4], uint8_t *out) {
    encode_bc1_block(block, out);
}
void encode_astc_adapt(const uint8_t block[16][4], uint8_t *out) {
    encode_astc_void_extent(block, out);
}

} // namespace

extern "C" const char *rurix_basis_version(void) {
    return kVersion;
}

extern "C" void rurix_basis_buf_free(RurixBasisBuf *buf) {
    if (!buf) {
        return;
    }
    delete[] buf->data;
    buf->data = nullptr;
    buf->len = 0;
}

extern "C" int rurix_basis_encode_bc7_rgba8(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    RurixBasisBuf *out) {
    return encode_blocks(rgba, width, height, 16, encode_bc7_adapt, out);
}

extern "C" int rurix_basis_encode_bc1_rgba8(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    RurixBasisBuf *out) {
    return encode_blocks(rgba, width, height, 8, encode_bc1_adapt, out);
}

extern "C" int rurix_basis_encode_astc4x4_rgba8(
    const uint8_t *rgba,
    uint32_t width,
    uint32_t height,
    RurixBasisBuf *out) {
    return encode_blocks(rgba, width, height, 16, encode_astc_adapt, out);
}
