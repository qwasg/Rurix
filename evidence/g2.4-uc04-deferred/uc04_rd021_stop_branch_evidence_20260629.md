# G2.4 UC-04 deferred — RD-021 停手分支取证报告(measured_local)

> 日期:2026-06-29 · 里程碑:G2.4(D-G2-4 / G-G2-4)· 证据等级:`measured_local`(本机真实命令输出)
> Provenance:`Assisted-by: cursor:glm-5.2` · 地位:**agent 代录机器事实,非 agent 签署**(硬规则 1)
> 配套 scoping 判定:[rd021_scoping_20260629.md](rd021_scoping_20260629.md)

## 1. 交付物清单(本任务实际产出)

| # | 产物 | 路径 | 状态 |
|---|---|---|---|
| 1 | RD-021 scoping 判定文档 | [evidence/g2.4-uc04-deferred/rd021_scoping_20260629.md](rd021_scoping_20260629.md) | 落 |
| 2 | UC-04 几何 pass VS Rurix 语料 | conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx | 落(RXS-0171 子集内) |
| 3 | UC-04 几何 pass FS MRT Rurix 语料 | conformance/dxil/graphics/accept/uc04_gbuffer_fs.rx | 落(RXS-0171 子集内,host SPIR-V 绿;B 链 bless 受 RD-017 阻,见 §5) |
| 4 | DXIL disasm dump(NOT BLESSED,agent 审阅) | src/rurixc/tests/dxil_golden.rs `uc04_gbuffer_disasm_dump_not_blessed`(`#[ignore]`) | 落 |
| 5 | traceability 矩阵重生成 | conformance/traceability_matrix.{json,md} | 172/172 全锚定 |
| 6 | 本取证报告 | evidence/g2.4-uc04-deferred/uc04_rd021_stop_branch_evidence_20260629.md | 落 |

**未达成(blocked-honest,交接 agent)**:device hardware 多 pass deferred draw + offscreen 像素对照 / `ci/dxil_uc04_device_smoke.py` / `pr-smoke.yml` step 48 接线 / device run URL / G-G2-4 签字 / DXIL·像素 golden bless / RD-013·RD-017·RD-019·RD-020·RD-021 status 翻转。`src/uc04-demo/src/device.rs::execute_offscreen` 维持 `Uc04Error::BlockedOnRd013` 不解。

## 2. 关键发现(真发现,非预想)

### 2.1 RD-021 第 0 步判定 = 停手分支(主判定)

UC-04 lighting pass 读 G-buffer(SRV 采样)须纹理访问 opcode;`src/rurixc/src/dxil_spirv.rs` `BodyLowerer` 白名单(RXS-0171 L4)不含资源/纹理/采样访问 → `RX6013`,且 SPIR-V opcode 表无 `OpImageSample*`/`Fetch`/`Read`;Rurix 源无采样/取数语法(资源句柄仅止于 opaque 绑定声明)。故 lighting 采样半**结构不可达**,触 RD-021 / 06§4.2 禁区。详见 scoping 文档 §3。**第一分支不成立 → 不出 device 绿。**

### 2.2 几何 pass FS MRT 输出命中 RD-017 fragment 输出用户名保名边界(次发现,非 RD-021)

UC-04 **几何 pass FS 写 MRT**(不采样,在 RXS-0171 白名单内)经生产忠实 B 链 `emit_dxil_b_disasm` 时,**签名门 strict-only 拒**:

```
uc04_gbuffer_fs: 生产 B 链 strict-only 拒(RD-017 fragment 输出 MRT 用户名保名边界,非采样/RD-021):
  SigGate(SigMismatch { detail: "用户语义名 `albedo`(dir Out)未在译后 输出(OSG1) 签名以等价名出现(疑退化为通用名)" })
```

根因(实读 `src/rurixc/src/dxil_codegen.rs:498-534` `rewrite_field_semantic`):spirv-cross 把 fragment **输出** varying 降为 `SV_Target#`(render target 语义,非 `TEXCOORD#`),而 RXS-0172 当前改写器只匹配 `TEXCOORD`/`texcoord` 前缀 → fragment 输出用户名(`albedo`/`normal`/`depth`)未恢复 → 签名门 RX6011 strict-only 拒。**VS 输出**(inter-stage varying,spirv-cross 降 `TEXCOORD#`)则成功恢复 `uv`/`normal`(见 §4 真实 DXIL)。

**此为 RD-017(open)fragment 输出 MRT 用户名保名边界,早于且独立于 RD-021(采样)**。几何 pass FS 的 host 侧 SPIR-V lowering(`emit_spirv_body`)仍产合法 `OpLoad`/`OpStore`(dxil_corpus accept 测试绿);仅 full B 链 blessed DXIL 受 RD-017 阻。agent 收口 RD-017 fragment 输出 MRT 名保名后,几何 pass FS 方可重 bless 入 golden。

## 3. host 门 + 守卫真实输出(measured_local,逐条)

| 命令 | 真实输出 | 判定 |
|---|---|---|
| `cargo test -p rurixc --features "dxil-backend shader-stages" --lib` | `test result: ok. 399 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out` | 绿 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus` | `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`(含 `accept_graphics_body_corpus_lowers_io_dataflow`/`accept_graphics_corpus_lowers_to_spirv`/`accept_graphics_link_consistent`/`reject_graphics_corpus_intercepted`/`corpus_files_carry_spec_anchor`) | 绿 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden` | `test result: ok. 5 passed; 0 failed; 1 ignored`(ignored = `uc04_gbuffer_disasm_dump_not_blessed`;`dxil_b_disasm_golden_matches_when_toolchain_present` 绿,既有 gfx_vs_min golden 不漂移) | 绿 |
| `cargo build -p uc04-demo` | `Finished `dev` profile` | 绿 |
| `cargo build -p uc04-demo --features d3d12-runtime` | `Finished `dev` profile`(device gate 编译,`execute_offscreen` 维持 `BlockedOnRd013`) | 绿 |
| `cargo test -p uc04-demo` | `test result: ok. 20 passed; 0 failed`(host RXS-0167~0170 装配/编排模型绿) | 绿 |
| `cargo clippy --all-targets --features "dxil-backend shader-stages" -- -D warnings` | `Finished `dev` profile` | 绿 |
| `cargo fmt --check` | exit 0 | 绿 |
| `py -3 ci/trace_matrix.py --check` | `[trace_matrix] PASS (172/172 clauses anchored, 448 test files scanned)` | 绿 |
| `py -3 ci/check_schemas.py` | `[check_schemas] PASS` | 绿 |
| `py -3 ci/budget_eval.py` | `[budget_eval] PASS (69 pass, 0 skip, normal mode)` | 绿 |
| `py -3 ci/check_guardrails.py` | `[check_guardrails] FAIL (base=g1-closed)` — 10 spec 文件(async_buffer/cublas/engine_integration/imageio/interop/interop_d3d12/pipeline/release/softraster/stdlib)"spec 变更未新增修订行" | **预存在红,非本任务引入**(见 §6) |

### 3.1 DXIL disasm dump(NOT BLESSED,`--ignored` 按需)

```
$env:RURIX_DXC='H:\dxc-round7\extracted\bin\x64\dxc.exe'
$env:RURIX_SPIRV_CROSS='C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe'
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden uc04_gbuffer_disasm_dump_not_blessed -- --ignored --exact --nocapture
→ test result: ok. 1 passed; 0 failed; 1 ignored(... filtered out)
```

- `uc04_gbuffer_vs`:成功产 DXIL 文本(见 §4)。
- `uc04_gbuffer_fs`:strict-only 拒(RD-017 fragment 输出 MRT 用户名保名边界,§2.2)。
- pin B 工具(`RURIX_DXC` / `RURIX_SPIRV_CROSS`)缺则 SKIP(对齐 RXS-0162)。

## 4. UC-04 几何 pass VS 真实 DXIL(NOT BLESSED 摘录)

`uc04_gbuffer_vs.rx`(`vertex fn uc04_gbuffer_vs() -> GBufVary { GBufVary { uv: 0.5, normal: 0.25 } }`)经生产忠实 B 链产 DXIL 反汇编(版本噪声行已规范化):

```
; Output signature:
; Name                 Index   Mask Register SysValue  Format   Used
; uv                       0   x           0     NONE   float   x
; normal                   0    y          0     NONE   float    y
...
define void @main() {
  call void @dx.op.storeOutput.f32(i32 5, i32 0, i32 0, i8 0, float 5.000000e-01)  ; StoreOutput(outputSigId,rowIndex,colIndex,value)
  call void @dx.op.storeOutput.f32(i32 5, i32 1, i32 0, i8 0, float 2.500000e-01)
  ret void
}
...
!3 = !{!"vs", i32 6, i32 0}            ; vs_6_0
!7 = !{!8, !11}
!8 = !{i32 0, !"uv", ...}             ; 输出 signature 元素 uv(用户名保真,RXS-0172)
!11 = !{i32 1, !"normal", ...}        ; 输出 signature 元素 normal(用户名保真)
```

证明:Rurix 源 → `emit_dxil_b_body` → B 链 → DXIL `vs_6_0`,输出 signature 用户名 `uv`/`normal` 端到端保真(RXS-0172),`dx.op.storeOutput.f32` 写出常量 0.5/0.25(RXS-0171 body 降级 + 输出聚合机械分解)。**此为 G-G2-4 防降级硬门要求的"至少一个 pass 来自 Rurix 源"的几何半证据**(VS 半)。完整 dump 输出存 `target/uc04_disasm_dump.txt`(本机产物,不入库)。

## 5. 几何 pass FS MRT 受 RD-017 阻的诚实边界

`uc04_gbuffer_fs.rx` host 侧 `emit_spirv_body` 产合法 SPIR-V(`OpLoad` ×2 输入 `uv`/`normal` + `OpFAdd`/`OpFMul` 白名单算术 + `OpStore` ×3 输出 `albedo`/`normal`/`depth`,由 `accept_graphics_body_corpus_lowers_io_dataflow` 测试断言绿)。但 full B 链 blessed DXIL 受阻于 RD-017(fragment 输出 → `SV_Target#`,RXS-0172 改写器只匹配 `TEXCOORD`)。**不在本任务强行绕过**(不放宽签名门 / 不发明 SV_Target 改写 / 不手写 HLSL);归 agent 收口 RD-017 后重 bless。

## 6. check_guardrails 预存在红(非本任务引入)

`check_guardrails.py` FAIL 列出的 10 个 spec 文件(async_buffer / cublas / engine_integration / imageio / interop / interop_d3d12 / pipeline / release / softraster / stdlib)均为**本任务开工前已存在的未提交 spec 修改**(见会话起始 `git status`:`M spec/async_buffer.md` … `M spec/stdlib.md`),与本任务无关。本任务**未触碰任何 spec 文件**:

```
git status --short -- conformance/ evidence/ src/rurixc/tests/dxil_golden.rs
 M conformance/traceability_matrix.json      (重生成,新增 2 语料锚定)
 M conformance/traceability_matrix.md        (同上)
 M src/rurixc/tests/dxil_golden.rs            (新增 #[ignore] disasm dump)
?? conformance/dxil/graphics/accept/uc04_gbuffer_fs.rx
?? conformance/dxil/graphics/accept/uc04_gbuffer_vs.rx
?? evidence/g2.4-uc04-deferred/
```

本任务的 spec 面(`spec/dxil_backend.md` / `spec/d3d12_runtime.md`)**未修改**,不在 guardrail 失败列表内。预存在 spec 红由对应 spec 编辑者补修订行(档位标记)解决,不属本任务范围;本任务**不代他人补 spec 修订行**(避免越界 + 污染他人进行中工作)。

## 7. 复跑命令(全部本机可复现)

```powershell
# host 门
cargo test -p rurixc --features "dxil-backend shader-stages" --lib
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden
cargo build -p uc04-demo
cargo build -p uc04-demo --features d3d12-runtime
cargo test -p uc04-demo
cargo clippy --all-targets --features "dxil-backend shader-stages" -- -D warnings
cargo fmt --check
# 守卫
py -3 ci/trace_matrix.py --check
py -3 ci/check_schemas.py
py -3 ci/budget_eval.py
py -3 ci/check_guardrails.py     # 预存在 spec 红,非本任务(§6)
# DXIL disasm dump(NOT BLESSED,需 pin B 工具)
$env:RURIX_DXC='H:\dxc-round7\extracted\bin\x64\dxc.exe'
$env:RURIX_SPIRV_CROSS='C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe'
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden uc04_gbuffer_disasm_dump_not_blessed -- --ignored --exact --nocapture
```

## 8. 不在本任务范围(交接 agent,详见 scoping §8)

device 绿 / G-G2-4 签字 / DXIL·像素 golden bless / step 48 接线 / device run URL / RD-013·RD-017·RD-019·RD-020·RD-021 status 翻转 / RD-021 纹理采样语义 Full RFC / g2-closed 切换。

---

> 本报告所有数字/输出来自本机真实命令(硬规则 3);实质 AI 内容标 `Assisted-by: cursor:glm-5.2`(硬规则 2);agent 代录机器事实,非 agent 签署(硬规则 1)。
