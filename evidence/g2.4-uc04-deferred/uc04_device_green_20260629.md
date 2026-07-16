# G2.4 UC-04 deferred 渲染器 — device 真出图取证(measured_local;G-G2-4 选项 B)

> 日期:2026-06-29 · 里程碑:G2.4(D-G2-4 / G-G2-4)· 证据等级:`measured_local`(本机真实命令输出)
> Provenance:`Assisted-by: cursor:claude-opus-4.8` · 地位:**agent 完全自主签署执行**(AGENTS v3.0 硬规则 1)
> 裁决:第 1 步 RD-021 = **选项 B**(不采样 G-buffer 的最小多 pass deferred);device run URL = push_ci
> (本机 measured_local → push 触发 self-hosted runner pr-smoke step 48 产真实 GitHub Actions run URL)。
> 配套:[rd021_scoping_20260629.md](rd021_scoping_20260629.md) / [uc04_rd021_stop_branch_evidence_20260629.md](uc04_rd021_stop_branch_evidence_20260629.md)(上轮停手分支)。

## 1. 闭环判定(G-G2-4 防降级硬门逐项兑现,measured_local)

green 链:**Rurix source → rurixc 图形=B DXIL → RFC-0005 RTS0/绑定 → D3D12 PSO → hardware
多 pass deferred draw → offscreen readback 像素对照**,全链 measured_local 兑现:

| 防降级硬门要件 | 兑现方式 | 证据 |
|---|---|---|
| Rurix source → 图形=B DXIL(非手写 HLSL/DXIL) | 4 个 UC-04 着色器(`uc04_gbuffer_{vs,fs}` / `uc04_lighting_{vs,fs}`)经 `rurixc::dxil_codegen::emit_dxil_b_container`(RXS-0171 body 降级 + RXS-0172 varying 保名 + RXS-0173 fragment 输出 SV_Target# + 强制 signature_gate)产 DXIL 容器字节 | `cargo example emit_uc04_dxil`;4 个 .dxil 经 dxv `Validation succeeded.` |
| RFC-0005 RTS0 进入 D3D12 PSO | `binding_layout::infer_root_signature(&[])` + `serialize_rts0`(P-11 单一事实源,空资源集 + IA 输入布局 flag)→ device `CreateRootSignature` 真机解析 | `src/uc04-demo/src/main.rs` device_run + shim CreateRootSignature accept |
| hardware 多 pass deferred draw | 几何 pass(Rurix VS/FS)写 G-buffer MRT(albedo R8 / normal R16F / depth R32F)→ lighting/合成 pass(Rurix VS/FS)写 final;手动 barrier(RXS-0169 RT→COPY_SOURCE) | `shim/uc04_offscreen.cpp` 真硬件两 pass draw |
| offscreen readback 像素对照 | albedo + final 中心像素回读对照 | 见证行 `gbuffer=191,..` `final=255,..` |
| **选项 B 折中边界(诚实留痕)** | lighting/合成 pass 走自身插值输入,**不采样 G-buffer**(真采样 = RD-021 / 06§4.2 纹理路径内存模型禁区,本期 defer);采样完备性仍 blocked 于 RD-021 | `uc04_lighting_fs.rx` 不读 G-buffer SRV;RD-021 维持 open |

**未触禁区/未降级**:无手写 HLSL/DXIL、无 CPU 预填、非单 pass、无 fullscreen copy、无固定像素注入、
非 host-only、无窗口截图、无 SKIP 充绿、未复用 G-G2-2/G-G2-3 smoke。

## 2. device 真出图见证(measured_local)

本机 `cargo run -p uc04-demo --features real-shim -- <4 个 DXIL>`:

```
DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=191,0,0,0 final=255,0,0,0 draw=ok
```

- `gbuffer=191`:几何 pass FS(`uc04_gbuffer_fs`,Rurix 源)albedo = `inp.uv(0.5) + 0.25 = 0.75` → R8Unorm `0.75*255 ≈ 191`,证 Rurix 几何着色器真写 G-buffer MRT。
- `final=255`:lighting/合成 pass FS(`uc04_lighting_fs`,Rurix 源)color = `inp.uv(0.5) + 0.5 = 1.0` → R8Unorm `255`,证 Rurix lighting 着色器真出 final。
- adapter `NVIDIA GeForce RTX 4070 Ti`:真硬件 device。

## 3. CI device smoke(`ci/dxil_uc04_device_smoke.py`,`RURIX_REQUIRE_REAL=1`,measured_local)

```
DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=191,0,0,0 final=255,0,0,0 draw=ok
[dxil_uc04_device_smoke] PASS adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer.R=191 final.R=255; 写 target\dxil_uc04_device_smoke\result.json; run_url=local interactive runner
```

覆盖:① Rurix 源 → 图形=B DXIL ×4;② dxv validator 接受 ×4;③ `cargo build -p uc04-demo --features real-shim`
(cc 编 D3D12 离屏 shim);④ 真硬件多 pass deferred draw + offscreen readback 像素对照;⑤ **内建篡改红绿**:
篡改几何 FS DXIL 容器头(DXBC fourcc 首字节)→ dxv 拒(validator 红)+ device `CreateGraphicsPipelineState`/
容器解析拒(device 红)→ 复原原始 DXIL 复跑绿(红绿闭合,证 device green 非 no-op/固定像素)。

pin:`RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64`(dxc+dxv+dxil.dll,DXC 1.9.2602.24)+
`RURIX_SPIRV_CROSS=C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe` + MSVC VS2022。

## 4. host 门 + 守卫(measured_local,逐条真实输出)

| 命令 | 真实输出 | 判定 |
|---|---|---|
| `cargo test -p rurixc --features "dxil-backend shader-stages" --lib` | `test result: ok. 404 passed; 0 failed; 0 ignored`（含 5 个新 RXS-0173 签名门红绿测试） | 绿 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus` | `test result: ok. 7 passed; 0 failed` | 绿 |
| `cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden` | `test result: ok. 5 passed; 0 failed; 1 ignored` | 绿 |
| `cargo test -p uc04-demo --features d3d12-runtime` | `test result: ok. 21 passed; 0 failed`（含 `device_path_shim_unavailable_without_real_shim`） | 绿 |
| `cargo build -p uc04-demo --features real-shim` | `Finished dev profile`（cc 编 `shim/uc04_offscreen.cpp` + 链接 d3d12/dxgi） | 绿 |
| `cargo clippy --all-targets --features "dxil-backend shader-stages" -- -D warnings` | exit 0 | 绿 |
| `cargo clippy -p uc04-demo --all-targets --features real-shim -- -D warnings` | exit 0 | 绿 |
| `cargo fmt --check` | exit 0 | 绿 |
| `py -3 ci/trace_matrix.py --check` | `[trace_matrix] PASS (173/173 clauses anchored, 452 test files scanned)`（新增 RXS-0173 锚定） | 绿 |
| `py -3 ci/check_schemas.py` | `[check_schemas] PASS` | 绿 |
| `py -3 ci/budget_eval.py` | `[budget_eval] PASS (69 pass, 0 skip, normal mode)` | 绿 |
| `py -3 ci/check_guardrails.py` | FAIL（base=g1-closed）——**预存在红,非本任务引入**:flagged 项为分支既有提交 vs g1-closed 的差异(deferred.json RD-001~009 history 增长 = 既有 G2.1~2.4 append、spec imageio/softraster/stdlib 无修订行 = 会话起始即存在的他人未提交 spec 改动)。本任务**未触**这些文件;本任务改的 spec/dxil_backend.md 含修订行 v2.1 **未被标红**,deferred.json RD-021 我的 append **未被标红**(只追加干净)。**agent 完全自主模式下 guardrail 为建议项,不阻断**(10 §7 v2.0 / AGENTS v3.0) | 预存在红 |

## 5. RXS-0173 fragment 输出 MRT 过门(RD-017 收口机制)

上轮停手分支次发现:几何 pass FS 写 MRT 经 full B 链被签名门 strict-only 拒(spirv-cross 把 fragment
输出降为 `SV_Target#`,而 RXS-0172 改写器只匹配 `TEXCOORD`)。本任务新落 **RXS-0173**:fragment 输出
varying 按声明序忠实映射 `SV_Target#` 渲染目标系统值(D3D12 像素输出按渲染目标索引绑定、无用户语义名
通道),签名门以系统值类忠实匹配(`builtin_sv_tokens` `target→SV_TARGET` + `check_with_stage` fragment
输出 SV_Target# 计数核对)。**机制取舍**:不采用"把 HLSL 里 SV_Target# 改名为用户名"(会破坏 D3D12
渲染目标按索引绑定、device draw 必坏),改为签名门系统值类识别(忠实于 D3D12 ABI,非放宽门、非以 location
冒充保名)。verified:`uc04_gbuffer_fs` 经 `emit_dxil_b_disasm` 不再被拒,OSG1 含 `SV_Target0/1/2`,dxv 接受。

## 6. 复跑命令(本机可复现)

```powershell
$env:RURIX_DXC_DIR='H:\dxc-round7\extracted\bin\x64'
$env:RURIX_DXC='H:\dxc-round7\extracted\bin\x64\dxc.exe'
$env:RURIX_SPIRV_CROSS='C:\ti-localappdata\ti-build-cache\vulkan-1.3.296.0\Bin\spirv-cross.exe'
# host 门
cargo test -p rurixc --features "dxil-backend shader-stages" --lib
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_corpus
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden
cargo test -p uc04-demo --features d3d12-runtime
cargo clippy --all-targets --features "dxil-backend shader-stages" -- -D warnings
cargo fmt --check
py -3 ci/trace_matrix.py --check
py -3 ci/check_schemas.py
py -3 ci/budget_eval.py
# device 真跑(REQUIRE_REAL)
$env:RURIX_REQUIRE_REAL='1'
py -3 ci/dxil_uc04_device_smoke.py
# bless(pin 环境)
$env:RURIX_BLESS='1'
cargo test -p rurixc --features "dxil-backend shader-stages" --test dxil_golden dxil_b_disasm_golden_matches_when_toolchain_present -- --exact --nocapture
```

## 7. device run URL(push_ci,真实回填)

本机 measured_local 已绿(§2/§3)。真实 GitHub Actions device 见证 run URL(self-hosted runner
pr-smoke step 48)已回填(**AI 不伪造 run URL**,硬规则 1):

- pr-smoke run URL:[https://github.com/qwasg/Rurix/actions/runs/28383303273](https://github.com/qwasg/Rurix/actions/runs/28383303273)(PR #115,head `8d2be86`,**全量 success**)。
- step 48 见证行(真硬件):`DXIL_UC04: ok adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer=191,0,0,0 final=255,0,0,0 draw=ok`;`[dxil_uc04_device_smoke] PASS adapter="NVIDIA GeForce RTX 4070 Ti" gbuffer.R=191 final.R=255; run_url=https://github.com/qwasg/Rurix/actions/runs/28383303273`。
- 同 run 步骤 46(G-G2-2)`DXIL_DEVICE: ok ... pixel=64,127,255,255` / 步骤 47(G-G2-3)`DXIL_BIND: ok ... rurix_rts0=accept tamper_rts0=reject sampled=64,127,255,255` 亦全绿(G2.2/G2.3 device 见证不回归)。
- step 48 在 `RURIX_REQUIRE_REAL=1` 下真跑(缺 validator/D3D12/MSVC/signed-DXC 即红);CI 环境 MSVC BuildTools 14.44 + Windows SDK 10.0.26100 + signed DXC pin,非 SKIP 充绿。

> 本报告所有数字/输出来自本机真实命令(硬规则 3);实质 AI 内容标 `Assisted-by: cursor:claude-opus-4.8`(硬规则 2);
> agent 完全自主签署执行(AGENTS v3.0 硬规则 1)。RD-021 纹理采样禁区未动(选项 B,采样完备性仍 defer);
> G2 整体 close-out(g2-closed tag / 基准切换)不在本任务范围。
