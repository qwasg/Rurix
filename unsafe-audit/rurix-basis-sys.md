# unsafe-audit: rurix-basis-sys(M83 纹理 codec FFI 边界)

> 注册依据:AGENTS.md 硬规则 9 / 10 §7.6。G8.3 M83 激活(RFC-0020 §4.8;
> 设计案 §3.6)。编号:**U44 起**(ledger next_free=44;主 agent 校准 number_ledger,
> 本文件只登记条目不改台账)。
> vendor / 过渡边界:[`src/rurix-basis-sys/VENDOR.md`](../src/rurix-basis-sys/VENDOR.md)。

## 范围与豁免

- crate:`src/rurix-basis-sys`(`[lints.rust] unsafe_code = "allow"`;`undocumented_unsafe_blocks
  = "deny"`——每个 unsafe 块强制 `// SAFETY:`)。
- `rurix-asset` 维持 `#![forbid(unsafe_code)]`;全仓其余 crate 维持 workspace
  `unsafe_code = "deny"`。
- 全部 unsafe 集中于:`src/rurix-basis-sys/src/ffi.rs`(声明)+ `src/lib.rs`(调用/缓冲)。

## 原语清单与验证义务

| # | 原语 | 位置 | 验证义务(SAFETY 不变量) |
|---|---|---|---|
| U44 | encoder 入口(`rurix_basis_encode_bc7_rgba8` / `bc1` / `astc4x4`) | lib.rs `encode_with` | 调用前:`rgba` 指针在调用期内有效且长度 ≥ `w*h*4`,`w/h>0`;`out` 指向栈上 `RurixBasisBuf` POD;失败时 shim 不留下需调用方释放的非配对指针(仍统一 `buf_free` 兜底);成功后缓冲所有权经 U46 移交;编码线程=1、无可重入共享可变全局 |
| U45 | 版本串 / `extern "C"` 声明面(`rurix_basis_version` + 头文件镜像) | ffi.rs 全文;lib.rs `version_string` | `version` 返回静态只读 C 字符串,非 null,生命周期=进程,UTF-8 字面 == `VENDOR_VERSION` / VENDOR.md pin;`extern "C"` 签名与 `rurix_basis_shim.h` 字段序/类型一致;`RurixBasisBuf` 布局 = 指针+usize(单测锚定) |
| U46 | 跨 FFI 内存视图(encoder 堆缓冲 → Rust `Vec`) | lib.rs `take_buf` / 失败路径 `buf_free` | `data` 指向 C++ `new[]` 的 `len` 字节;在 `buf_free` 前只读拷贝进 Rust `Vec`(**禁止** `from_raw_parts` 跨分配器接管);随后恰好一次 `rurix_basis_buf_free`(`delete[]`);失败路径同样 free;无双释放/无 UAF |

## 销毁纪律

唯一释放出口 = `rurix_basis_buf_free` ↔ C++ `delete[]`。Rust 侧持有的是拷贝后的 `Vec<u8>`,
与 C++ 堆无共享。进程级静态版本串不卸载。

## 测试

- `cargo test -p rurix-basis-sys`
- `cargo test -p rurix-asset`
- 门:`ci/g8_texture_transcode_smoke.py --gate g8.p1.m83.texture_transcode`
