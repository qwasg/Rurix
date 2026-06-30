# UC-04 deferred 真采样 G-buffer device 取证（G2.4 强化轮 / G-G2-4 严格面重签）

> 日期：2026-06-30。地位：agent 完全自主签署 G-G2-4 严格面（supersede G2_CONTRACT §8.5 选项 B
> 不采样折中）的 measured_local 取证源。Provenance：`Assisted-by: claude-code:claude-opus-4.8`。所有数字
> 来自真实命令输出（硬规则 3）；device 像素/run URL 不伪造（本机有 GPU，做真实 local 运行，
> CI run URL 待 self-hosted runner 上线回填）。

## 1. 目标与判据

废止 §8.5 选项 B（lighting pass 不采样 G-buffer、走自身插值输入）的折中，升级为 **RFC-0007 严格面**：
lighting pass **真采样 G-buffer**（真延迟着色），`final = f(几何 pass 写入并被采样的 G-buffer 值)`。
严格判据（RXS-0176 IR2）：篡改几何 pass FS 写入的 albedo 常量 → final 像素必须随之改变；仅
「多 pass + 写 G-buffer」不接受。

## 2. 链路（防降级硬门，全链来自 Rurix 源）

```
Rurix 源 albedo.sample(samp, inp.uv)           [conformance/dxil/graphics/accept/uc04_lighting_fs.rx]
  → typeck RX3014 fragment-only                [RXS-0174]
  → MIR Rvalue::ResourceSample                 [dxil_spirv lower_resource_sample]
  → SPIR-V OpSampledImage + OpImageSampleExplicitLod(Lod 0.0)   [RXS-0175 IR1]
  → spirv-cross HLSL tex.SampleLevel(samp, uv, 0.0)
  → dxc DXIL dx.op.sampleLevel.f32             [RXS-0175 IR2]
  → 每 pass RFC-0005 RTS0（lighting = SRV t0 + Sampler s0 descriptor table，infer_root_signature）
  → D3D12 CreateRootSignature 真机解析 → PSO
  → hardware 几何 pass 写 G-buffer MRT
  → albedo RT→SRV barrier（RENDER_TARGET → PIXEL_SHADER_RESOURCE）   [RXS-0176 IR1]
  → lighting pass 经 SRV/Sampler descriptor table 真采样 albedo
  → offscreen readback 中心像素
```

未触禁区/未降级：无手写 HLSL/DXIL、CPU 预填、单 pass、fullscreen copy、固定像素、host-only、
窗口截图、SKIP 充绿、复用 G-G2-2/G-G2-3 smoke。

## 3. device 真跑（本机 RTX 4070 Ti，measured_local）

命令：

```
RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64 \
RURIX_SPIRV_CROSS=C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe \
RURIX_REQUIRE_REAL=1 py -3 ci/dxil_uc04_device_smoke.py
```

green 见证行（采样到 albedo 0.75 → R8=191；final 真采样 → 追踪 gbuffer.R）：

```
DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=191,0,0,0 final=191,0,0,0 draw=ok
```

数据流红绿变体（几何 FS 源 albedo 0.75→0.5 经同一图形=B 链 → R8=127；final 随采样值改变）：

```
DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=127,0,0,0 final=127,0,0,0 draw=ok
```

smoke 结尾：

```
[dxil_uc04_device_smoke] PASS adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer.R=191 final.R=191; 写 target\dxil_uc04_device_smoke\result.json; run_url=local interactive runner
```

## 4. 数据流严格红绿（RXS-0176 IR2，本轮核心）

| 阶段 | 几何 FS albedo 常量 | gbuffer.R | final.R | 结论 |
|---|---|---|---|---|
| green（原始） | 0.75 | 191 | 191 | final 真追踪采样到的 albedo |
| 红（变体，同一图形=B 链重编译） | 0.5 | 127 | 127 | **final 随采样值改变（191→127）** = final 真依赖被采样的 G-buffer 值 |
| 复原绿 | 0.75 | 191 | 191 | 红绿闭合 |

变体经**同一编译器链**（`emit_uc04_dxil` 图形=B）从 Rurix 源重产 DXIL，**非手编 DXIL**——证真数据流
穿过编译器。另保留 DXIL 容器篡改红绿（篡改几何 FS DXIL 容器 fourcc → dxv 拒 + device
`CreateGraphicsPipelineState` 拒 → 复原绿），证 device 非 no-op/固定像素。

## 5. DXIL golden（含真采样指令，dxv 接受）

`tests/dxil/graphics/uc04_lighting_fs.dxil-disasm`（重 bless）含：

```
%5 = call %dx.types.ResRet.f32 @dx.op.sampleLevel.f32(i32 62, %dx.types.Handle %1, %dx.types.Handle %2, float %3, float %4, float undef, float undef, i32 0, i32 0, i32 undef, float 0.000000e+00)  ; SampleLevel(...,LOD)
```

（显式 LOD `0.000000e+00`，RXS-0176 首期收敛子集）+ SRV t0 / Sampler s0 `createHandle`。命令：

```
RURIX_BLESS=1 cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden \
  dxil_b_disasm_golden_matches_when_toolchain_present -- --exact --nocapture
```

入 golden 前 4 个 DXIL 各经签名 `dxv.exe` validator `Validation succeeded.` 接受；非 bless 复跑确定性匹配。
bless 留痕：`tests/dxil/bless_log.md`（2026-06-30 行）。

## 6. host 门 + 守卫（measured_local，逐条真实输出）

| 命令 | 结果 |
|---|---|
| `cargo test -p rurixc --features "dxil-backend shader-stages" --lib` | 404 passed; 0 failed |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus` | 7 passed; 0 failed |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden` | 5 passed; 0 failed; 1 ignored |
| `cargo test -p uc04-demo --features d3d12-runtime` | 21 passed; 0 failed |
| `cargo build -p uc04-demo --features real-shim` | Finished（D3D12 离屏 shim cc 编，ABI v2） |
| `cargo clippy --all-targets --features "dxil-backend shader-stages" -- -D warnings` | clean |
| `cargo clippy -p uc04-demo --all-targets --features real-shim -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| `py -3 ci/trace_matrix.py --check` | PASS（176/176 clauses anchored） |
| `py -3 ci/check_schemas.py` | PASS |
| `py -3 ci/budget_eval.py` | PASS（69 pass, 0 skip） |
| `py -3 ci/bilingual_coverage.py` | PASS（zh/en 87/87，含 RX3014/RX6023） |

## 7. RD 处置 + 契约

- **RD-021 `open→closed`**：纹理采样内存模型本体经 RFC-0007（§4.3~§4.7，06 §4.2 🔒 禁区）落笔 +
  device 真采样兑现。
- 新增 **RD-022**（隐式 LOD/导数 + 可配置 sampler 状态）/ **RD-023**（整型 texel fetch）/
  **RD-024**（比较采样/gather/多分量纹理/UAV 写 + memory-order）——RFC-0007 §8 首期收敛子集外，不偷偷略过。
- RD-019（窗口 present）/ RD-020（自动状态跟踪）维持 open；RD-013/RD-017 维持 closed（§8.5）。
- `registry/deferred.json` revision_log v1.41；`milestones/g2/G2_CONTRACT.md` §8.6 supersede §8.5。
- **G2 契约整体仍 `active`**：不执行 `g2-closed` tag / 基准切换（g1-closed→g2-closed）/ G2 整体 close-out。

## 8. 残留（待 self-hosted runner）

- **CI run URL**：本取证为本机 measured_local（run_url=`local interactive runner`）；真实 GitHub Actions
  device run URL（pr-smoke step 48）待 self-hosted runner 上线后回填本节，按 §8.5 先例（run 28383303273）范式。
  本机有 GPU、已做真实运行，**不以 SKIP/替代物伪造**；CI 回填为 provenance 补强，非验收前置。

## 9. CI run URL 回填（self-hosted runner 上线，§8 残留兑现）

self-hosted runner `rurix-dev-4070ti`（RTX 4070 Ti）上线，PR #115 pr-smoke 全 48 步绿，G-G2-4 device 见证 CI run URL 回填：

- **run**: https://github.com/qwasg/Rurix/actions/runs/28442661542 （`pull_request`，PR #115，sha `c0e8730`，conclusion `success`）
- **step 48（G-G2-4 UC-04 deferred device smoke）** CI 见证行（runner 自 `GITHUB_*` 派生 run_url，非伪造）：
  - `DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=191,0,0,0 final=191,0,0,0 draw=ok`
  - 数据流变体（albedo 0.75→0.5 经同一图形=B 链）：`DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=127,0,0,0 final=127,0,0,0 draw=ok`
  - `[dxil_uc04_device_smoke] PASS adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer.R=191 final.R=191; run_url=https://github.com/qwasg/Rurix/actions/runs/28442661542`
- **step 28（G-G2-1 着色阶段类型面拦截）** 绿：RX3001/3011/3012/3013 + green。
- 与本机 measured_local（§1~§6）见证一致（gbuffer=191 final=191，变体 127）；CI 与本机双见证闭合。

### 9.1 随附修复（commit `c0e8730`）

首次 pr-smoke 在 step 28（着色阶段类型面 `ci/shader_stages_smoke.py`）红，根因：采样面 commit `0c86647` 使 typeck 把 `-> Texture2D<F>` 当作具体返回类型，空 body 触类型不匹配 RX2001，先于 spec 强制的资源句柄违例 RX3013（RXS-0156）发出并 short-circuit `check_shader_stages`，导致 `-> Texture2D<F>` 误报 RX2001。该回归被 `0c86647`/`db667dd` 提交但漏跑 `shader_corpus` / `ui_golden` / `shader_stages_smoke`（§8.6 host 门枚举未覆盖三者），CI 全量 `cargo test --workspace` + shader_stages_smoke 才暴露。

修复（镜像 driver + 两处测试 harness 一致）：着色阶段 AST 层检查（RX3011~3013）前移至 resolve 后、typeck 前——非法句柄位置先于 body↔返回类型匹配裁决，RX3013 不再被 RX2001 掩盖。`src/rurixc/src/driver.rs` / `src/rurixc/tests/shader_corpus.rs` / `src/rurixc/tests/ui_golden.rs`（`ui/shader/handle_return.stderr` 既有期望即 RX3013，无需重 bless，仅 harness 对齐）。RXS-0156「句柄仅签名形参」语义不变。

修复后验证（measured_local，回填前置）：`cargo test -p rurixc --features "dxil-backend shader-stages"` 27/27 targets 绿（lib 404/0 + shader_corpus 4/0 + ui_golden 4/0）；`cargo test --workspace` 75/0；`cargo clippy --workspace --all-targets -- -D warnings` 干净；`cargo fmt --check` 干净；`py -3 ci/shader_stages_smoke.py` PASS。CI run 28442661542 全 48 步绿复证。
