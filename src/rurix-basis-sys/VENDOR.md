# VENDOR — rurix-basis-sys(M83 texture_transcode)

> G8.3 M83 前置(RFC-0020 §4.8 / 设计案 §3.6):纹理 cook 真实 codec FFI。
> 审计与骨架落盘日期:2026-08-06。

## 1. pin 与许可(过渡态)

| 组件 | 角色 | pin / 版本串 | 许可 |
|---|---|---|---|
| `vendor/rurix_basis_shim` | **过渡真实 codec**(手写确定性 BC1/BC7 + ASTC 4×4 void-extent;经 `cc` 编 .cpp) | 版本串 **`rurix-basis-transitional/0.1.0`**(与 `rurix_basis_version()` 字面全等) | MIT OR Apache-2.0(与仓根双许可对齐;见 `NOTICE`) |
| `basis_universal`(BinomialLLC) | **待合入**完整 UASTC/ETC1S/KTX2 encoder+transcoder | **未 vendor**(体量大;本机未拉完整树) | Apache-2.0(合入时登记 upstream URL/tag/source digest/LICENSE digest) |

- **诚实边界**:本切片 **不**声称已接入完整 `basis_universal`。四腿中 **BCn** 与 **ASTC(void-extent 实块)** 与 **KTX2 容器(无 supercompression)** 为过渡真实路径;**.basis / ETC1S** 腿未实现 → smoke 对应 check 必须为 `false`(禁充绿)。
- 合入完整 vendor 时:本文件改为上游 URL + 精确 tag/commit + source/LICENSE digest + 编译 flags/补丁 digest;`build.rs` 改为显式 .cpp 清单(仍 **禁止 cmake**、**禁止 zstd supercompression**)。

## 2. 构建策略(cc,非 cmake)

- `build.rs` 经 `cc` 编译 `vendor/rurix_basis_shim/rurix_basis_shim.cpp` → 静态库 `rurix_basis_shim`。
- flags 显式:`-std=c++17`、`RURIX_BASIS_THREADS=1`、`RURIX_BASIS_NO_ZSTD`。
- 确定性钳制(AP-TEX / RXS-0334):编码线程恒 1、固定算法序、禁 zstd/RDO 多线程、禁非确定浮点路径(本 shim 整数算术)。
- C++ 工具链画像:与仓内其它 `cc` crate 一致(Windows = MSVC;非 Windows = 宿主 c++)。

## 3. FFI 面(薄 C API)

| 符号 | 语义 |
|---|---|
| `rurix_basis_version` | 返回静态版本 C 字符串(== 上表 pin) |
| `rurix_basis_encode_bc7_rgba8` | RGBA8 → BC7_UNORM 块字节(4×4 对齐;不足边复制钳制) |
| `rurix_basis_encode_bc1_rgba8` | RGBA8 → BC1_RGB 块字节(过渡备用腿) |
| `rurix_basis_encode_astc4x4_rgba8` | RGBA8 → ASTC 4×4(每块 LDR unbounded void-extent,常色=块均值) |
| `rurix_basis_buf_free` | 释放 encoder 堆缓冲 |

Rust 侧 safe 包装见 `src/lib.rs`;unsafe 登记 U44~U46。

## 4. SBOM / NOTICE

- `NOTICE` — 过渡 shim 与待合入 basis_universal 许可提示。
- `SBOM.md` — 组件条目 + NOTICE digest 复核位(smoke `license_sbom_entries_present`)。
