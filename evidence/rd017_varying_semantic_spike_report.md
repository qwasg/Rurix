# RD-017 保名机制 spike 取证报告(选项① HLSL 边界改写)

> SPIKE(RD-017) · measured-first / blocked-honest(AGENTS 硬规则 3/4)。
> 隔离探针 `spike/rd017-varying-semantic/probe_rewrite.py`,不入 `src/` 生产路径。
> 证据机:`evidence/rd017_varying_semantic_spike_20260629.json`。
> owner ruling:选项① HLSL 文本边界保名改写;**否决③**(不放宽 `signature_gate`)。

## 1. 命题

验证 owner 裁定的选项①是否兑现四个验收点(对**输出 varying** 与 **fragment 输入
varying** 两个 RD-017 缺口面):

- (a) 改写后 `dxc` 接受(B 链不破);
- (b) 用户语义名端到端存活进 DXIL 签名 → `signature_gate`(`semantic_name_matches`)
  **不放宽**也能过;
- (c) ABI 中立:改写只动 HLSL struct field 的 semantic token,不触 register / mask /
  packing / byte layout / Location 数值(owner 边界 B);
- (d) 确定性:同输入 ×N 改写字节一致。

## 2. 方法(链路)

命名 HLSL(`corpus/vs_named.hlsl` 输出 NORMAL/WORLDPOS/UV;`corpus/ps_named.hlsl`
片元输入同名)→ `dxc -spirv` → `spirv-cross --hlsl`(语义退化 `TEXCOORD#`)
→ **候选改写**:按 location provenance 把目标 struct(VS=`SPIRV_Cross_Output` /
PS=`SPIRV_Cross_Input`)的 `TEXCOORD<loc>` 改回用户名 → `dxc` → DXContainer
ISG1/OSG1 签名 part 解析(复用 `spike/dxil-path-probe/dxil_container.py`)。
gate 判定用 `signature_gate::semantic_name_matches` 的**逐字复刻**(大小写无关 +
剥尾随 index 数字),不放宽。

工具(本机,非 owner pin):
- `dxc` = dxcompiler.dll 1.8.0.4739(d9a5e97d0)。
- `spirv-cross` = vulkan-sdk-1.3.290.0-44-g65d73934。

## 3. 结果(measured_local,2/2 sample)

| 判据 | 结果 |
|---|---|
| 改写前 gate(退化 TEXCOORD#) | **拒**(both samples;复现 RX6011 缺口) |
| 改写后 `dxc` 接受 | **pass**(stderr 0) |
| 改写后 gate(不放宽) | **过**(NORMAL/WORLDPOS/UV 等价名命中) |
| HLSL 文本 ABI 中立 | **true**(行数不变、仅 semantic token 变、前后缀逐字节不变) |
| 物理 ABI 不变(register/mask/comp_type/system_value) | **true** |
| 确定性(改写文本 + 改写后 DXIL ×4) | **一致** |

退化实测:`spirv-cross` 把 VS 输出三个 varying 与 PS 输入三个 varying 一律写为
`TEXCOORD0/1/2`(共享基名 `TEXCOORD`),用户名 `NORMAL`/`WORLDPOS`/`UV` 丢失 →
gate `semantic_name_matches` 不命中 → 拒。改写后 DXIL 签名名恢复为 `NORMAL`/
`WORLDPOS`/`UV` → 命中 → 过。

## 4. 一个必须如实记录的副作用:`semantic_index`

改写**唯一**的二进制层副作用:`semantic_index` 由 `1/2` 归 `0`。成因:三个共享基名
`TEXCOORD` 的元素 index 为 0/1/2;恢复为三个**不同**用户基名后,各自 index 自然归 0。

判定:**不构成** owner 边界 B 的物理 ABI 触碰,理由——
- `semantic_index` 是**语义名的 index 后缀**(`TEXCOORD0` 的 `0`),属语义命名维度;
- `signature_gate` 明确**不比对** index 数字本身(剥尾随数字,"属 ABI 维度");
- 边界 B 列举的冻结面是 register / mask / packing / byte layout / Location 数值,
  这些**实测全不变**(`physical_abi_invariant=true`);
- 故 index 归零是改名的**确定性自然后果**,非独立 ABI 操纵 → **不触 Full RFC**。

## 5. pin 与范围诚实声明(blocked-honest)

- 本机 `spirv-cross` 为 1.3.290,owner pin 环境留痕为 1.3.296。改写机制依赖
  `spirv-cross` 对 varying 默认发 `TEXCOORD<location>`(稳定约定);location→用户名
  provenance 来自 `io_sig`(非工具),对工具次版本鲁棒。若未来 `spirv-cross` 改默认
  varying 语义命名,改写的**探测侧**(识别 `TEXCOORD#`)须复验——登记为实现侧 pin 复验点。
- **签名 validator(`dxv.exe`/`dxil.dll`)未参与**:本机无签名 validator,只测 `dxc`
  结构接受 + 签名名存活。完整签名验证、golden bless、device 真跑归 **owner pin 环境
  (G-G2-4)**,本 spike 不代签、不翻 RD-017 状态、不收口。

## 6. 结论与下一步

选项①在两个缺口面(VS 输出 varying / PS 片元输入 varying)经端到端实测兑现 (a)~(d),
且**不放宽 signature_gate**(Property 5 保持)、**不触物理 ABI**(边界 B 保持)。
据此推进:spec-first 落 **RXS-0172**(`spec/dxil_backend.md` §2)→ 实现 RD-017
(`vertex_input_semantic_flags` 同源 provenance 扩出 HLSL 边界改写)→ RD-017 维持
**open** 至 owner golden/device bless。
