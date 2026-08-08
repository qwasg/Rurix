/* G8.3 M83 — 薄 C API,包真实 `basis_universal`(BinomialLLC, 1.16.4) encoder/transcoder。
 * 设计案 §3.6。本文件 = Rurix 自有代码(MIT OR Apache-2.0);vendor/ 下为上游 Apache-2.0 快照。
 * 确定性钳制:线程恒 1、禁 zstd supercompression、禁 OpenCL、禁 RDO 多线程。
 */
#ifndef RURIX_BASIS_WRAP_H
#define RURIX_BASIS_WRAP_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct RurixBasisBuf {
  uint8_t *data;
  size_t len;
} RurixBasisBuf;

/* 真实 vendor 版本串:"basis_universal/<tag>+g<commit12>"(与 VENDOR.md 字面全等)。 */
const char *rurix_basis_version(void);

/* 释放 encoder/transcoder 堆缓冲(配对 new[]/delete[])。 */
void rurix_basis_buf_free(RurixBasisBuf *buf);

/* 容器编码模式 */
#define RURIX_BASIS_MODE_UASTC_KTX2 0 /* UASTC 4x4 → .ktx2(无 supercompression) */
#define RURIX_BASIS_MODE_ETC1S_BASIS 1 /* ETC1S → 真实 .basis */

/* transcode 源容器种类 */
#define RURIX_BASIS_SRC_BASIS 0
#define RURIX_BASIS_SRC_KTX2 1

/* transcode 目标(数值 == basist::transcoder_texture_format 字面) */
#define RURIX_BASIS_TF_BC4_R 4
#define RURIX_BASIS_TF_BC5_RG 5
#define RURIX_BASIS_TF_BC7_RGBA 6
#define RURIX_BASIS_TF_ASTC_4x4 10

/* RGBA8(w*h*4,行主序) → 容器字节。
 * swizzle_rg != 0:上游 XY normal map 流(R→RGB、G→A),使 BC5 腿 X=R / Y=G。
 * 返回 0 = 成功;非 0 = 失败码(不写 out->data)。 */
int rurix_basis_encode_container(const uint8_t *rgba, uint32_t width, uint32_t height,
                                 int mode, int swizzle_rg, RurixBasisBuf *out);

/* 容器 → GPU 块字节(真 transcode)。out_width/out_height 可为 NULL。 */
int rurix_basis_transcode(const uint8_t *data, size_t len, int src_kind, int target,
                          RurixBasisBuf *out, uint32_t *out_width, uint32_t *out_height);

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* RURIX_BASIS_WRAP_H */
