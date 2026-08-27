# VENDOR — rurix-basis-sys(M83 texture_transcode)

> G8.3 M83(RFC-0020 §4.8 / 设计案 §3.6):纹理 cook 真实 codec FFI。
> 真实 `basis_universal`(BinomialLLC)已 vendor 落盘:2026-08-07。

## 1. pin 与许可

| 组件 | 角色 | pin / 版本串 | 许可 |
|---|---|---|---|
| `basis_universal`(BinomialLLC) | **真实** UASTC/ETC1S/KTX2 encoder + transcoder | tag **1.16.4** @ commit **900e40fb5d2502927360fe2f31762bdbb624455f** → 版本串 **`basis_universal/1.16.4+g900e40fb5d25`**(== `rurix_basis_version()` 字面全等) | Apache-2.0(见 `vendor/basis_universal/LICENSE` + `LICENSES/`) |
| `vendor/rurix_basis_shim` | 旧过渡 shim(已停用;保留仅供历史参考,不参与编译) | — | MIT OR Apache-2.0 |

**upstream URL**: `https://github.com/BinomialLLC/basis_universal`

**source digest**(全树 SHA-256 聚合,见 `vendor_manifest.json`): `a9a6ac43374801b74b2e73de781f1776992a274c6fdbb199e7c15534b455ab87`

**LICENSE digest**(vendor/basis_universal/LICENSE SHA-256): `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`

**compile flags/patch digest**: 无 patch(vendor 快照原版);编译 flags 锁定 = `-std=c++17 BASISU_SUPPORT_SSE=0 BASISU_SUPPORT_OPENCL=0 BASISD_SUPPORT_KTX2=1 BASISD_SUPPORT_KTX2_ZSTD=0 RURIX_BASIS_THREADS=1`(见 `build.rs`)。

**诚实边界**:本切片已接入完整 `basis_universal` 真实路径。四腿全部真实产出:KTX2=真实 UASTC 容器、`.basis`=真实 ETC1S 码流(签名 `sB`)、RXBC=真 transcode BCn、RXAS=真 transcode ASTC 4×4。禁 zstd supercompression、禁 cmake、线程恒 1。

旧过渡 shim 版本串已**废除**;任何含过渡串的 evidence 均为旧快照。

## 2. 构建策略(cc,非 cmake)

- `build.rs` 经 `cc` 编译 `vendor/basis_universal/encoder/*.cpp`(15 件)+ `transcoder/basisu_transcoder.cpp` + `ffi/rurix_basis_wrap.cpp` → 静态库 `rurix_basis_wrap`。
- 编译单元显式清单 = `vendor/basis_universal/vendor_manifest.json` 的 `compile_units` 字段;剔除上游 CLI main(`basisu_tool.cpp`)、`zstd/`(禁 supercompression)、OpenCL kernel。
- flags 显式：`-std=c++17`、`BASISU_SUPPORT_SSE=0`、`BASISU_SUPPORT_OPENCL=0`、`BASISD_SUPPORT_KTX2=1`、`BASISD_SUPPORT_KTX2_ZSTD=0`、`RURIX_BASIS_THREADS=1`。
- 确定性钳制(AP-TEX / RXS-0334):job_pool(1)=零额外线程、m_multithreading=false、禁 RDO、禁 zstd/supercompression、固定档位(ETC1S m_compression_level=2 / m_quality_level=128;UASTC cPackUASTCLevelDefault)。

## 3. FFI 面(薄 C API)

| 符号 | 语义 |
|---|---|
| `rurix_basis_version` | 返回静态版本 C 字符串(== 上表 pin 字面) |
| `rurix_basis_encode_container` | RGBA8 → KTX2(UASTC)或真实 `.basis`(ETC1S);mode 与 swizzle_rg 参数 |
| `rurix_basis_transcode` | 容器 → GPU 块字节(BC4/BC5/BC7/ASTC 4×4,真 transcode);level 0 |
| `rurix_basis_transcode_level` | 同上但 mip level 参数化(G31+ 波 C Task C14):KTX2 路径 `level ∈ [0, levels)` 越界 rc=17;`.basis` 路径 `level != 0` rc=5(fail-closed) |
| `rurix_basis_buf_free` | 释放 encoder/transcoder 堆缓冲(配对 new[]/delete[]) |

Rust 侧 safe 包装见 `src/lib.rs`;unsafe 登记 U44~U46 + U60。

## 4. SBOM / NOTICE

- `NOTICE` — basis_universal 版权行与许可提示(合入后须更新)。
- `SBOM.md` — 组件条目 + source/LICENSE digest 复核位。

