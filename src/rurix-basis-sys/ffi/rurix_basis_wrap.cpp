/* G8.3 M83 — 薄 C API 实现,包真实 `basis_universal`(BinomialLLC 1.16.4)。
 * 设计案 §3.6。本文件 = Rurix 自有代码(MIT OR Apache-2.0)。
 *
 * 确定性钳制(RXS-0334):
 *   1. job_pool(1) —— 上游要求 m_pJob_pool 非空;1 = 仅调用线程,零额外线程;
 *      并 m_multithreading=false,使 RDO/frontend 不走并行路径。
 *   2. 固定档位:ETC1S m_compression_level 与 m_quality_level 硬编码;
 *      UASTC m_pack_uastc_flags = cPackUASTCLevelDefault;禁 RDO。
 *   3. 禁 zstd / 任意 supercompression:KTX2_SS_NONE + 编译期 BASISD_SUPPORT_KTX2_ZSTD=0。
 *   4. 禁 OpenCL(BASISU_SUPPORT_OPENCL=0 + m_use_opencl=false)。
 *   5. 禁 mip 生成(单 mip)、禁 y_flip、禁 stats/status 输出(不写 stdout)。
 */
#include "rurix_basis_wrap.h"

#include <cstring>
#include <mutex>
#include <string>

#include "../vendor/basis_universal/encoder/basisu_comp.h"
#include "../vendor/basis_universal/transcoder/basisu_transcoder.h"

namespace {

/* 版本串 = "basis_universal/<tag>+g<commit12>",与 VENDOR.md / SBOM.md 字面全等。
 * 由 vendor_manifest.json 的 tag/commit 派生,写死在此以便 FFI 零分配返回。 */
const char kRurixBasisVersion[] = "basis_universal/1.16.4+g900e40fb5d25";

std::once_flag g_init_once;

void init_library() {
  std::call_once(g_init_once, [] {
    /* use_opencl=false, opencl_force_serialization=false */
    basisu::basisu_encoder_init(false, false);
    basist::basisu_transcoder_init();
  });
}

int publish(const basisu::uint8_vec &src, RurixBasisBuf *out) {
  if (src.empty()) {
    return 20;
  }
  uint8_t *p = new (std::nothrow) uint8_t[src.size()];
  if (!p) {
    return 21;
  }
  std::memcpy(p, src.data(), src.size());
  out->data = p;
  out->len = src.size();
  return 0;
}

/* 确定性 encoder 参数基线。swizzle_rg != 0 时把 XY normal 流铺成
 * R→RGB / G→A,使 ETC1S/UASTC 的 alpha slice 携带 Y,BC5 腿 transcode 后
 * X=R、Y=G 语义成立(上游 cTFBC5_RG 取 color.r 与 alpha)。 */
void apply_determinism_clamps(basisu::basis_compressor_params &p,
                              basisu::job_pool *pool, int swizzle_rg) {
  p.m_pJob_pool = pool;
  p.m_multithreading = false;
  p.m_use_opencl = false;

  /* 不读磁盘、不写磁盘、不打印 */
  p.m_read_source_images = false;
  p.m_write_output_basis_files = false;
  p.m_status_output = false;
  p.m_compute_stats = false;
  p.m_print_stats = false;
  p.m_debug = false;
  p.m_debug_images = false;

  /* 单 mip、无翻转 */
  p.m_mip_gen = false;
  p.m_y_flip = false;

  /* 禁 UASTC RDO(引入额外启发式与并行路径) */
  p.m_rdo_uastc = false;

  /* 禁任何 KTX2 supercompression */
  p.m_ktx2_uastc_supercompression = basist::KTX2_SS_NONE;

  if (swizzle_rg) {
    /* R,R,R,G —— alpha 通道承载 Y */
    p.m_swizzle[0] = 0;
    p.m_swizzle[1] = 0;
    p.m_swizzle[2] = 0;
    p.m_swizzle[3] = 1;
    p.m_force_alpha = true;
  } else {
    p.m_swizzle[0] = 0;
    p.m_swizzle[1] = 1;
    p.m_swizzle[2] = 2;
    p.m_swizzle[3] = 3;
  }
}

}  // namespace

extern "C" {

const char *rurix_basis_version(void) { return kRurixBasisVersion; }

void rurix_basis_buf_free(RurixBasisBuf *buf) {
  if (!buf) {
    return;
  }
  delete[] buf->data;
  buf->data = nullptr;
  buf->len = 0;
}

int rurix_basis_encode_container(const uint8_t *rgba, uint32_t width,
                                 uint32_t height, int mode, int swizzle_rg,
                                 RurixBasisBuf *out) {
  if (!out) {
    return 1;
  }
  out->data = nullptr;
  out->len = 0;
  if (!rgba || width == 0 || height == 0) {
    return 2;
  }
  if (mode != RURIX_BASIS_MODE_UASTC_KTX2 && mode != RURIX_BASIS_MODE_ETC1S_BASIS) {
    return 3;
  }

  init_library();

  basisu::job_pool pool(1);
  basisu::basis_compressor_params p;
  apply_determinism_clamps(p, &pool, swizzle_rg);

  if (mode == RURIX_BASIS_MODE_UASTC_KTX2) {
    p.m_uastc = true;
    p.m_create_ktx2_file = true;
    p.m_pack_uastc_flags = basisu::cPackUASTCLevelDefault;
  } else {
    p.m_uastc = false;
    p.m_create_ktx2_file = false;
    /* 固定 ETC1S 档位(禁 -1 自适应,保证跨机同参) */
    p.m_compression_level = 2;
    p.m_quality_level = 128;
  }

  {
    basisu::image img;
    img.init(rgba, width, height, 4);
    p.m_source_images.push_back(img);
  }

  basisu::basis_compressor comp;
  if (!comp.init(p)) {
    return 10;
  }
  basisu::basis_compressor::error_code ec = comp.process();
  if (ec != basisu::basis_compressor::cECSuccess) {
    return 100 + static_cast<int>(ec);
  }

  const basisu::uint8_vec &result = (mode == RURIX_BASIS_MODE_UASTC_KTX2)
                                        ? comp.get_output_ktx2_file()
                                        : comp.get_output_basis_file();
  return publish(result, out);
}

int rurix_basis_transcode(const uint8_t *data, size_t len, int src_kind,
                          int target, RurixBasisBuf *out, uint32_t *out_width,
                          uint32_t *out_height) {
  return rurix_basis_transcode_level(data, len, src_kind, target, 0, out,
                                     out_width, out_height);
}

int rurix_basis_transcode_level(const uint8_t *data, size_t len, int src_kind,
                                int target, uint32_t level, RurixBasisBuf *out,
                                uint32_t *out_width, uint32_t *out_height) {
  if (!out) {
    return 1;
  }
  out->data = nullptr;
  out->len = 0;
  if (!data || len == 0 || len > 0xFFFFFFFFu) {
    return 2;
  }

  switch (target) {
    case RURIX_BASIS_TF_BC4_R:
    case RURIX_BASIS_TF_BC5_RG:
    case RURIX_BASIS_TF_BC7_RGBA:
    case RURIX_BASIS_TF_ASTC_4x4:
      break;
    default:
      return 3;
  }

  init_library();

  const basist::transcoder_texture_format fmt =
      static_cast<basist::transcoder_texture_format>(target);
  const uint32_t bytes_per_block = basist::basis_get_bytes_per_block_or_pixel(fmt);
  const uint32_t data_size = static_cast<uint32_t>(len);

  uint32_t w = 0, h = 0, blocks_x = 0, blocks_y = 0, total_blocks = 0;

  if (src_kind == RURIX_BASIS_SRC_KTX2) {
    basist::ktx2_transcoder t;
    if (!t.init(data, data_size)) {
      return 11;
    }
    if (t.get_levels() == 0) {
      return 12;
    }
    if (level >= t.get_levels()) {
      return 17;
    }
    if (!t.start_transcoding()) {
      return 13;
    }
    basist::ktx2_image_level_info info;
    if (!t.get_image_level_info(info, level, 0, 0)) {
      return 14;
    }
    w = info.m_orig_width;
    h = info.m_orig_height;
    blocks_x = info.m_num_blocks_x;
    blocks_y = info.m_num_blocks_y;
    total_blocks = blocks_x * blocks_y;
    if (total_blocks == 0) {
      return 15;
    }
    basisu::uint8_vec buf(static_cast<size_t>(total_blocks) * bytes_per_block);
    if (!t.transcode_image_level(level, 0, 0, buf.data(), total_blocks, fmt, 0, 0, 0)) {
      return 16;
    }
    if (out_width) *out_width = w;
    if (out_height) *out_height = h;
    return publish(buf, out);
  }

  if (src_kind != RURIX_BASIS_SRC_BASIS) {
    return 4;
  }
  if (level != 0) {
    /* 在树 .basis 产物恒单级(m_mip_gen=false 钳制);level 参数面仅 KTX2。 */
    return 5;
  }

  basist::basisu_transcoder t;
  if (!t.validate_header(data, data_size)) {
    return 21;
  }
  if (t.get_total_images(data, data_size) == 0) {
    return 22;
  }
  if (!t.start_transcoding(data, data_size)) {
    return 23;
  }
  basist::basisu_image_level_info info;
  if (!t.get_image_level_info(data, data_size, info, 0, 0)) {
    return 24;
  }
  w = info.m_orig_width;
  h = info.m_orig_height;
  total_blocks = info.m_total_blocks;
  if (total_blocks == 0) {
    return 25;
  }
  basisu::uint8_vec buf(static_cast<size_t>(total_blocks) * bytes_per_block);
  if (!t.transcode_image_level(data, data_size, 0, 0, buf.data(), total_blocks, fmt,
                               0, 0, nullptr, 0)) {
    return 26;
  }
  if (out_width) *out_width = w;
  if (out_height) *out_height = h;
  return publish(buf, out);
}

}  /* extern "C" */
