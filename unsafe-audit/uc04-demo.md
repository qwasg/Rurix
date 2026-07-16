# unsafe-audit — `uc04-demo`（UC-04 deferred 渲染器 D3D12 离屏 shim 边界）

> 地位:`src/uc04-demo` crate 的 unsafe 原语注册表(AGENTS 硬规则 9 / 10 §7.6）。G2.4 UC-04
> deferred 渲染器 device 真出图（RFC-0006 / G-G2-4;选项 B 不采样 G-buffer 的最小多 pass
> deferred）。crate 维持 workspace lints（含 `undocumented_unsafe_blocks = deny`，每 unsafe 块
> 携 `// SAFETY:`）;`unsafe_code = deny` 由 `device.rs` 内 `#[cfg(feature = "real-shim")]` +
> `#[allow(unsafe_code)]` 在**最小范围**局部豁免，host/safe 装配/编排路径（pso/deferred/barrier/
> readback）仍 **零 unsafe**。

## 范围与豁免

- **默认（无 feature / 仅 `d3d12-runtime`）**:**零 unsafe**——`execute_offscreen` 在无 `real-shim`
  时显式返回 `Uc04Error::ShimUnavailable`（环境缺失 sentinel，非语言 RX），不触 FFI。无 MSVC/
  Windows SDK D3D12 环境亦编译（常驻回归网绿）。host 侧 RXS-0167~0170 装配/编排模型全程 safe。
- **feature `real-shim`**:经 `build.rs` + `cc` 编译 `shim/uc04_offscreen.cpp`（D3D12 离屏两 pass
  deferred draw + readback），Rust 侧仅经下列 `extern "C"` 扁平面 FFI 调用。D3D12/DXGI COM 复杂度
  全部留 C++,**不进语言**（D-130 先例，对齐 `rurix-d3d12`）。

## 原语清单与验证义务（RustBelt 式;real-shim 段）

| # | 原语 | 位置 | 验证义务 |
|---|---|---|---|
| U24 | `rx_uc04_offscreen_run` extern "C"（5 对只读 DXIL/RTS0 字节指针+长度 + 2 个 4 字节可写像素出参 + 256 字节可写 adapter 缓冲+cap） | `device.rs` `execute_offscreen` | 首参 `abi_version == RX_UC04_ABI_VERSION`（=1，shim 侧 `kAbiVersion` 核对）;每个 `*const u8` 指向有效只读字节切片且配对 `len` = 切片实际 `len()`（`req.pso.rts0_bytes` / `req.{geom,light}_{vs,fs}_dxil`）;`out_gbuffer_pixel` / `out_final_pixel` 为 4 字节可写数组、`out_adapter` 为 `out_adapter_cap`(256) 字节可写缓冲;shim 只读入字节、回填 out、**不持有指针越出调用**;返回 i32（0=成功，非 0=HRESULT 位码或哨兵失败码，经 `Uc04Error::DeviceRunFailed` 透出，不伪造 device 绿） |
| U24 | `rx_uc04_abi_version` extern "C"（无参） | `device.rs` `shim_abi_version` | 无参纯返回 C 侧编译期常量 `kAbiVersion`，无副作用、不解引用任何指针 |

## C++ shim 侧 D3D12 所有权纪律（RFC-0006 §4.2;G2.4 选项 B）

- DXGI factory·adapter / D3D12 device·queue·command allocator·command list / G-buffer MRT（albedo
  R8 / normal R16F / depth R32F）·final RT·readback 缓冲·fence·event·vertex buffer **全部由 shim
  拥有**，单函数调用内 `ComPtr` RAII 释放，不跨调用持有。
- 消费的 DXIL 着色器对象（VS/FS 各 pass）= **Rurix 源经 `rurixc::dxil_codegen::emit_dxil_b_container`
  图形=B 链产物**（非手写 HLSL/DXIL，G-G2-4 防降级硬门);RTS0 = RFC-0005 `serialize_rts0`（P-11，
  device `CreateRootSignature` 真机解析）。顶点缓冲（全屏三角形 pos/uv/normal）为 host 几何数据。
- 手动 barrier（RXS-0169 编排锚点）:pass 间 G-buffer albedo `RENDER_TARGET → COPY_SOURCE`（选项 B
  不采样 → 转 copy source 供 readback 见证，非 RT→SRV）、final `RENDER_TARGET → COPY_SOURCE`。
- C++ 异常不越过 C ABI;成功返回 0，失败返回 HRESULT 位模式或负哨兵码（adapter/PSO/draw/readback 各阶段）。

## 测试

- 默认 stub（无 real-shim）:`cargo test -p uc04-demo --features d3d12-runtime`
  (`device_path_shim_unavailable_without_real_shim` 断言 `ShimUnavailable` + `rx_code()==None`）。
- real-shim:device 真跑见 `ci/dxil_uc04_device_smoke.py`（步骤 48，`RURIX_REQUIRE_REAL=1` 缺
  validator/D3D12/MSVC/signed-DXC 即红）+ 像素对照（G-buffer albedo + final 中心像素），run URL 随回填。
