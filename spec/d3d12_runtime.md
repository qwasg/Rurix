# Rurix 语言规范 — UC-04 deferred 渲染器 / D3D12 运行时出图语义面（G2.4 起）

> 条款:**RXS-0167 ~ RXS-0170 计划区间**（G2.4 UC-04 deferred 渲染器 / 原生 D3D12 运行时出图语义面:DXIL + RTS0 → graphics PSO 装配一致性 / deferred 多 pass 编排 / 资源状态 + barrier 编排锚点 / offscreen readback + 像素对照）。体例见 [README.md](README.md)。
> 依据:**[RFC-0006](../rfcs/0006-uc04-deferred-renderer.md)**（UC-04 deferred 渲染器 / 原生 D3D12 运行时出图路径,owner Approved 2026-06-28,§9 全 11 项已裁）;06 §8.2 第 4/5 点（PSO 装配 / 资源状态 / barrier 运行时面 = G2 设计预留）;06 §4.2（纹理路径内存模型禁区,🔒）;04 P-01（strict-only）;04 P-11（host 绑定结构 ↔ shader 布局单一事实源）;[RFC-0002](../rfcs/0002-shader-stages.md)（着色阶段类型面）;[RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)（图形=B DXIL codegen 与禁区边界）;[RFC-0005](../rfcs/0005-binding-layout-inference.md) RXS-0163~0166（绑定布局推导 + RTS0 序列化）;[RFC-0001](../rfcs/0001-cuda-d3d12-interop.md)（D3D12 device/queue/swapchain 运行时先例）。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)（D-G2-4,G-G2-4）+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.4 子里程碑。
> 档位:**Full RFC**（RFC-0006;10 §3:本设计首次落 **D3D12 运行时执行面**——PSO 装配 / 资源状态机 / barrier 语义 / swapchain 呈现,并触 AGENTS 硬规则 5 禁区边界——**纹理路径内存模型映射（06 §4.2）** / **D3D12 运行时 stable ABI** / **host↔运行时·host↔DXIL FFI ABI 二进制布局** / **barrier·资源状态并发语义**）。RFC-0006 已由 owner（Language Lead）于 2026-06-28 批准并裁决 §9 全部路径项。**agent 自主判档**,判档以 RFC-0006 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **纹理路径内存模型映射（06 §4.2）** / **barrier 并发·可见性·内存序语义本体** / **D3D12 运行时 stable ABI / FFI ABI 二进制布局** / **DXIL·SPIR-V UB 边界** 的条款,必须停下标注「需升档」,不在本文件自行落笔。**严禁 UB 节**（10 §7.5）:PSO 装配不一致 / 资源状态非法转换 / barrier 缺失或冲突 / RTS0 与 PSO 不匹配以编译期/装配期可预测错误（6xxx 段,自 RX6018 起,落码归 PR-F2）或运行时显式失败（P-01 strict-only,无运行期 fallback）定义;D3D12 API 返回的纯运行期/环境失败不滥发语言 RX,作 smoke/evidence runtime failure 报告。
> 规范先行（AGENTS.md 硬规则第 7 条）:**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 >=1 测试锚定（`//@ spec: RXS-####`）。PR-F1 spec 脚手架仅登记文件名 + 计划区间 RXS-0167~0170（不落裸条款头）;**本轮 PR-F2（blocked-honest interim slice,§3.1）已落带编号条款体 RXS-0167~0170（§2）+ host 侧 safe 装配/编排模型 [`src/uc04-demo`](../src/uc04-demo) + 每条 ≥1 测试锚定（`ci/trace_matrix.py` 增扫 `src/uc04-demo`,trace 维持全锚定 166→170）+ 6xxx 装配期错误码 RX6018~RX6022**。**device 段（hardware 多 pass deferred draw + offscreen 像素对照 + CI step 48 + golden + G-G2-4 签字）阻塞于 RD-013,本轮标 blocked-honest 不达成、不以替代物伪造、不签 G-G2-4**（§3.1;G2_CONTRACT G-G2-4 防降级硬门）。

---

## 1. 范围与编号区间

本文件承载 **UC-04 deferred 渲染器 / 原生 D3D12 运行时出图** 的语义条款（G2.4+,D-G2-4）。UC-04 是 G2.1 着色阶段类型面 + G2.2 DXIL B 链 codegen + G2.3 绑定布局推导的**首个端到端集成验证点**:运行时把 RFC-0004 产的 DXIL 着色器对象 + RFC-0005 推导的 RTS0 root signature 装配成可执行的多 pass deferred 管线,以编译期推导的单一事实源（P-11）装配,不在运行时手维护第二份绑定布局。

覆盖语义面（RFC-0006 §4 / §5 / §9）:

- **DXIL + RTS0 → graphics PSO 装配一致性**:运行时把 RFC-0004 DXIL 着色器对象（VS/PS）+ RFC-0005 推导的 RTS0 + 渲染目标格式/深度状态组装成 graphics PSO;RTS0 与 PSO 一致性承 G-G2-3 `CreateRootSignature` accept 见证。装配不一致 → strict-only 显式错（无运行期 fallback,P-01）。当前 Rurix 运行时装配面仍为待建面,不得冒充已实测。
- **deferred 多 pass 编排**:§9 Q-DeferredPass 裁决为最小集 = 几何 pass（G-buffer:albedo + normal + depth MRT）→ 单光源 lighting pass（采样 G-buffer 作 shader resource）→ offscreen readback;pass 顺序/目标缺失 → strict-only 显式错。
- **资源状态 + barrier 编排锚点**:§9 Q-Barrier 裁决为首期手动 barrier 编排——pass 间 G-buffer 资源状态转换（`RENDER_TARGET` → `PIXEL_SHADER_RESOURCE` → 回 `RENDER_TARGET` / Copy / Readback）由运行时显式插入;不做编译器自动状态跟踪（自动状态推导 defer → RD-020）。本面只承诺**编排锚点（哪里需要状态转换）**;🔒 barrier 并发/可见性/内存序语义本体不在本文件。
- **offscreen readback + 像素对照**:§9 Q-Present 裁决为 offscreen-first——offscreen 渲染后回读像素做数值对照（对齐 G-G2-2/G-G2-3 readback 先例,CI device 可真跑,REQUIRE_REAL）;窗口 swapchain present 作后续可选阶段,不阻塞 G-G2-4（窗口 present defer → RD-019）。

明确不在本文件落语义本体的范围:

- **🔒 纹理路径内存模型映射（06 §4.2）**:G-buffer 写入（MRT render target）与 lighting pass 采样（SRV）的纹理访问语义 / 采样 opcode / 描述符编码 / 缓存一致性 / LOD·导数 / 越界采样后果 / memory-order 留独立 agent Full RFC（§9 Q-Texture defer → RD-021）。首期只消费 opaque `Texture2D`/`Sampler` 句柄 + D3D12 RT/SRV 视图绑定。
- **🔒 barrier / 资源状态并发语义本体**:barrier 的并发/可见性/内存序语义本体不在本文件;本面仅定义编排锚点,语义本体「需升档」（agent Full RFC）。
- **🔒 D3D12 运行时 stable ABI / host↔运行时·host↔DXIL FFI ABI 二进制布局**:运行时封装层（device/queue/PSO/command list 的 host↔D3D12 边界）不冻结为 stable 语言/运行时 ABI（与 RFC-0004 §4.6(a) / RFC-0005 RXS-0165 同级:实现确定、gate 后、非 stable;stable 面随 RD-008,G2.5 候选触发点）。
- **🔒 DXIL/SPIR-V UB 边界**:不建立独立于源码语义的 DXIL/SPIR-V UB 契约（承 RFC-0004 §4.6(c)）;依赖未建模行为的运行时 lowering 须显式拒绝。
- **运行时/库实现 + demo crate**:D3D12 运行时封装层、PSO 装配、资源状态/barrier 编排、`src/uc04-demo` demo crate、command list 录制、错误码落码、golden、device 真跑均归 PR-F2（agent 闸门,G-G2-4）。

**编号区间**:本文件计划条款为 **RXS-0167 ~ RXS-0170**（全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;当前最高现存 RXS-0166 @ [binding_layout.md](binding_layout.md);区间 §9 Q-Range 已裁锁定 4 条）。本轮 **仅登记区间预留**,**不落带编号裸条款头**;条款体与每条 >=1 测试锚定随 PR-F2 同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款

> 本节落带编号条款体（PR-F2 **blocked-honest interim slice**,owner 2026-06-29 裁定）。各条按需分 **Syntax / Legality / Dynamic Semantics / Implementation Requirements** 节,**严禁 UB 节**（10 §7.5:PSO 装配不一致 / 资源状态非法转换 / barrier 缺失或冲突 / RTS0 与 PSO 不匹配以装配期可预测错误 6xxx 段定义,P-01 strict-only,无运行期 fallback）。本轮 host 侧 safe 装配/编排模型落 [`src/uc04-demo`](../src/uc04-demo)（纯 host/safe,零新 unsafe,复用 [binding_layout.md](binding_layout.md) RFC-0005 RTS0 推导面 [`binding_layout::{RootSignature, serialize_rts0, check_binding_consistency}`](../src/rurixc/src/binding_layout.rs) 兑现 P-11 单一事实源,不在运行时手维护第二份绑定布局）。**device 段（hardware 多 pass deferred draw + offscreen 像素对照）阻塞于 RD-013**——图形=B 入口 body 数据流降级未实现（`dxil_spirv::emit_spirv` 仅产接口 + 平凡 `main`）→ 无 Rurix 自产可出图着色器;按 G-G2-4 防降级硬门,device 真绿**不得**以手写 HLSL/DXIL、CPU 预填、单 pass textured draw、fullscreen copy、固定像素注入、host-only 模拟、窗口截图或 SKIP 替代 → **本轮 device 段标 blocked-honest,不签 G-G2-4**（详见 §3 状态）。**本片不碰** 🔒 纹理路径内存模型映射（06 §4.2,RD-021）/ barrier 并发·可见性·内存序语义本体 / D3D12 运行时 stable ABI / host↔运行时·host↔DXIL FFI ABI 二进制布局 / DXIL·SPIR-V UB 边界——只引边界声明,触及即停手标「需升档」,不在本文件自落笔。

### RXS-0167 DXIL + RTS0 → graphics PSO 装配一致性

运行时把 RFC-0004 图形=B DXIL 着色器对象（VS/PS 的接口签名 + 资源绑定反射）与 RFC-0005 推导的 RTS0 root signature + 渲染目标/深度格式装配为 graphics PSO,以**编译期推导的单一事实源**（P-11）为准:RTS0 取编译期推导,不在运行时手维护第二份绑定布局。本条只承诺**装配输入间一致性的 host 侧可判定核验**（着色器资源绑定 ↔ RTS0 / PS 输出签名 ↔ 渲染目标格式集）;装配不一致 → strict-only 显式错（无运行期 fallback,P-01）。🔒 PSO / RTS0 的具体二进制物理布局、host↔运行时 / host↔DXIL FFI ABI 不冻结为 stable（实现确定、gate 后、非 stable;承 RFC-0004 §4.6(a) / RFC-0005 RXS-0165 同级）。

#### Syntax

PSO 装配为运行时库面,非语言文法面:VS/PS 着色器接口由着色阶段签名（RXS-0153~0156）与图形=B codegen（RXS-0157~0162）给定,RTS0 由绑定布局推导（RXS-0163~0166）给定,不因装配改写。

#### Legality

- L1（可装配）:VS/PS 接口签名 + 资源绑定反射 + RTS0（单一事实源）+ 渲染目标格式集（MRT）+ 深度格式互相一致 → 可装配为 graphics PSO 装配描述。
- L2（渲染目标失配,strict-only）:PS 输出签名元素数与渲染目标格式集基数不等 / 深度写入意图与深度格式缺失矛盾 → **RX6018** `runtime.uc04_pso_assembly_mismatch`。无 fallback。
- L3（RTS0 ↔ 着色器绑定失配,strict-only）:着色器资源绑定反射与 RTS0 推导意图不等价（资源数 / 种类轴 / register / space / count 失配）→ **RX6019** `runtime.uc04_rts0_pso_mismatch`（复用 RFC-0005 RXS-0166 [`check_binding_consistency`](../src/rurixc/src/binding_layout.rs) 一致性门,P-11 不另维护第二份）。无 fallback。
- 🔒（布局边界）:PSO / RTS0 具体二进制物理布局、host↔运行时 / host↔DXIL FFI ABI 二进制布局越出本条 = ABI 禁区,**需升档**;本条仅作装配输入间一致性边界声明。

#### Dynamic Semantics

PSO 装配一致性核验为 host 侧确定性变换,本条无运行期语言语义（着色器在 D3D12 管线的真实执行属 device,承 RXS-0170 device 段,blocked-on-RD-013）。给定相同装配输入,一致性结论确定（两次核验同结论）。

#### Implementation Requirements

- IR1（装配核验）:host 侧 [`uc04_demo::pso::assemble_graphics_pso`](../src/uc04-demo/src/pso.rs)`(&GraphicsPsoDesc)` 对 VS/PS 接口 + 资源绑定 + RTS0 + 渲染目标/深度格式做一致性核验,产 `AssembledPso` 装配描述（host 侧,不触 device）;纯 host/safe,零新 unsafe。
- IR2（P-11 单一事实源）:RTS0 取 [`binding_layout::RootSignature`](../src/rurixc/src/binding_layout.rs)（RFC-0005 编译期推导）,着色器绑定 ↔ RTS0 一致性复用 [`binding_layout::check_binding_consistency`](../src/rurixc/src/binding_layout.rs);运行时不手维护第二份绑定布局。
- IR3（strict-only）:渲染目标失配 → `Uc04Error::PsoTargetMismatch`（RX6018）;RTS0 ↔ 绑定失配 → `Uc04Error::Rts0PsoMismatch`（RX6019）。无运行期 fallback。
- IR4（测试锚定）:≥1 `//@ spec: RXS-0167`——accept（一致输入装配成 `AssembledPso`）+ reject（渲染目标数失配 → `PsoTargetMismatch` / RTS0 ↔ 绑定失配 → `Rts0PsoMismatch`）。

### RXS-0168 deferred 多 pass 编排

§9 Q-DeferredPass 裁决的最小 deferred 编排:几何 pass（G-buffer:albedo + normal + depth 多渲染目标 MRT）→ 单光源 lighting pass（采样 G-buffer 作 shader resource）→ offscreen readback。本条只承诺**编排结构的 host 侧可判定核验**（pass 顺序 / MRT 目标存在性 / lighting pass 的 SRV 输入引用几何 pass 输出 / readback 引用 lighting 输出）;顺序/目标缺失 → strict-only 显式错。**device 像素对照（几何 pass 真写 G-buffer + lighting 真采样 + 数值结果）须由 Rurix 自产 DXIL 出图兑现,阻塞于 RD-013（承 RXS-0170 device 段,blocked-honest,本轮不达成）。**

#### Syntax

deferred 多 pass 编排为运行时库面,非语言文法面。

#### Legality

- L1（可编排）:几何 pass 声明 albedo+normal+depth MRT + 深度目标;lighting pass 的 SRV 输入逐一引用几何 pass 已声明的 G-buffer 输出;readback 引用 lighting pass 输出;pass 序 = 几何 → lighting → readback → 可编排为 `DeferredPlan`。
- L2（pass 顺序/目标缺失,strict-only）:pass 乱序（lighting 先于几何 / readback 先于 lighting）/ G-buffer MRT 目标缺失 / lighting SRV 输入引用未声明的 G-buffer 目标 / readback 源缺失 → **RX6020** `runtime.uc04_pass_orchestration`。无 fallback。
- 🔒（边界）:G-buffer 写入 / lighting 采样的**纹理路径内存模型**（采样 opcode / LOD·导数 / 越界 / 缓存一致性 / memory-order,06 §4.2）不在本条;首期只编排 opaque `Texture2D`/`Sampler` 句柄 + RT/SRV 视图绑定,触及纹理内存模型语义即停手升档（RD-021,agent Full RFC）。

#### Dynamic Semantics

编排结构核验为 host 侧确定性变换,本条无运行期语言语义。**device 段:** 几何 pass 真写 + lighting 真采样 + offscreen readback 像素对照为 G-G2-4 device 必要面,须 Rurix source → 图形=B DXIL → RTS0 → D3D12 PSO → hardware 多 pass draw 兑现;当前 RD-013 阻塞（无 Rurix 自产可出图着色器）→ 本轮 **blocked-honest**,不以任何替代物伪造（防降级硬门,§3）。

#### Implementation Requirements

- IR1（编排核验）:host 侧 [`uc04_demo::deferred::plan_deferred_passes`](../src/uc04-demo/src/deferred.rs)`(&DeferredGraph)` 核验 pass 顺序 / MRT 目标存在性 / SRV 输入引用 / readback 源,产 `DeferredPlan`;纯 host/safe,零新 unsafe。
- IR2（strict-only）:乱序 / 目标缺失 / 引用未声明目标 → `Uc04Error::PassOrchestration`（RX6020）。无运行期 fallback。
- IR3（device 段 blocked-honest）:hardware 多 pass deferred draw + offscreen 像素对照承 RXS-0170 device 段,阻塞于 RD-013;本轮 device 执行入口 [`uc04_demo::device`](../src/uc04-demo/src/device.rs)（gate `d3d12-runtime`）显式返回 `Uc04Error::BlockedOnRd013`,**不**伪造 device 绿。
- IR4（测试锚定）:≥1 `//@ spec: RXS-0168`——accept（合法编排 → `DeferredPlan`）+ reject（乱序 / 目标缺失 / SRV 引用未声明目标 → `PassOrchestration`）。**device 像素对照不在本轮测试**（blocked-honest）。

### RXS-0169 资源状态 + barrier 编排锚点

§9 Q-Barrier 裁决的首期手动 barrier 编排:pass 间 G-buffer 资源状态转换（`RENDER_TARGET` → `PIXEL_SHADER_RESOURCE` → 回 `RENDER_TARGET` / Copy / Readback）由运行时显式插入;不做编译器/运行时自动状态跟踪（自动状态推导 defer → RD-020）。本条只承诺**编排锚点（哪里必须有状态转换）的 host 侧可判定核验**;缺 barrier / 非法状态转换 → strict-only 显式错。🔒 **barrier 的并发 / 可见性 / 内存序语义本体不在本条**——「需升档」（agent Full RFC）。

#### Syntax

barrier 编排锚点为运行时库面,非语言文法面。

#### Legality

- L1（可编排锚点）:几何 pass 后 G-buffer `RENDER_TARGET → PIXEL_SHADER_RESOURCE`（供 lighting 采样）;lighting pass 后输出 `RENDER_TARGET → COPY_SOURCE`,readback 目标 `→ COPY_DEST`;每个 pass 边界所需状态转换均有对应 barrier 锚点 → 可编排为 barrier 计划。
- L2（缺 barrier / 非法转换,strict-only）:某 pass 边界所需状态转换缺对应 barrier / 状态转换非法（源状态与目标状态不构成合法 D3D12 转换 / 资源在被采样前未离开 `RENDER_TARGET`）→ **RX6021** `runtime.uc04_barrier_plan`。无 fallback。
- 🔒（语义本体边界）:barrier 的并发 / 可见性 / 内存序语义本体（happens-before / 跨队列可见性 / 缓存刷新语义）**不在本条**,触及即停手标「需升档」（agent Full RFC）;本条仅核验编排锚点的**存在性与状态转换合法性**,不定义并发内存模型。

#### Dynamic Semantics

barrier 编排锚点核验为 host 侧确定性变换,本条无运行期语言语义（barrier 的运行期并发语义本体不在本条,见 🔒）。给定相同 pass 编排,所需 barrier 锚点集确定。

#### Implementation Requirements

- IR1（锚点核验）:host 侧 [`uc04_demo::barrier::plan_barriers`](../src/uc04-demo/src/barrier.rs)`(&DeferredPlan)` 按 pass 边界推出所需状态转换锚点集,核验每个所需转换均有对应 barrier 且转换合法,产 `Vec<BarrierAnchor>`;纯 host/safe,零新 unsafe。
- IR2（strict-only）:缺 barrier / 非法状态转换 → `Uc04Error::BarrierPlan`（RX6021）。无运行期 fallback（首期手动编排,不自动补 barrier,自动状态跟踪 defer RD-020）。
- IR3（测试锚定）:≥1 `//@ spec: RXS-0169`——accept（完整状态转换 → 合法 barrier 锚点集）+ reject（缺某转换 barrier / 非法转换 → `BarrierPlan`）。🔒 并发语义本体不测（不在本条）。

### RXS-0170 offscreen readback + 像素对照

§9 Q-Present=offscreen-first:offscreen 渲染后回读像素做数值对照为 G-G2-4 device 必要面;窗口 swapchain present 不进必要条款（defer → RD-019）。本条 host 面承诺**readback 缓冲布局/格式的可判定核验**（row pitch 对齐 / 格式 / 尺寸与源一致）;布局/格式失配 → strict-only 显式错。**device 段（hardware offscreen draw + 像素逐值对照,REQUIRE_REAL,Q-CIStep step 48）阻塞于 RD-013 → 本轮 blocked-honest,不达成、不以替代物伪造、不签 G-G2-4。**

#### Syntax

offscreen readback 为运行时库面,非语言文法面。

#### Legality

- L1（host 可核验:readback 布局）:readback 缓冲 row pitch 对齐 D3D12 `TEXTURE_DATA_PITCH_ALIGNMENT`、格式与 lighting 输出一致、尺寸 = 对齐 row pitch × 行数 → 可装配 readback 布局。
- L2（布局/格式失配,strict-only）:row pitch 未对齐 / 格式与源不一致 / 尺寸不足 → **RX6022** `runtime.uc04_readback_layout`。无 fallback。
- L3（device 像素对照,blocked-honest）:offscreen hardware draw + 像素逐值对照（REQUIRE_REAL）须 Rurix source → 图形=B DXIL → RTS0 → D3D12 PSO → hardware 多 pass deferred draw → offscreen readback 全链兑现;当前 RD-013 阻塞 → **本轮不达成**,按 G-G2-4 防降级硬门**不得**以手写 HLSL/DXIL、CPU 预填、单 pass、fullscreen copy、固定像素、host-only 模拟、窗口截图或 SKIP 替代。窗口 present 子路径无显示环境可 SKIP,但不替代 offscreen readback 真跑（RD-019）。
- 🔒（边界）:readback 缓冲的纹理内存模型 / 跨队列可见性语义本体不在本条（06 §4.2 / RD-021）;本条只核验 host 侧布局结构。

#### Dynamic Semantics

readback 布局核验为 host 侧确定性变换,本条无运行期语言语义。**device 段:** offscreen 像素对照为运行期 device 真跑面,承 G-G2-4 / CI step 48 REQUIRE_REAL;当前 blocked-on-RD-013（§3 状态),不以替代物伪造。

#### Implementation Requirements

- IR1（readback 布局核验）:host 侧 [`uc04_demo::readback::plan_readback`](../src/uc04-demo/src/readback.rs)`(&ReadbackRequest)` 核验 row pitch 对齐 / 格式一致 / 尺寸充足,产 `ReadbackLayout`;纯 host/safe,零新 unsafe。
- IR2（strict-only）:布局/格式失配 → `Uc04Error::ReadbackLayout`（RX6022）。无运行期 fallback。
- IR3（device 段 blocked-honest）:hardware offscreen draw + 像素对照阻塞于 RD-013;本轮 device 执行入口（gate `d3d12-runtime`）显式 `BlockedOnRd013`,**不**伪造;CI step 48 接线 + device run URL + G-G2-4 签字归 RD-013 解锁后的 device PR + owner（§3）。
- IR4（测试锚定）:≥1 `//@ spec: RXS-0170`——accept（合法 readback 请求 → `ReadbackLayout`）+ reject（row pitch 未对齐 / 格式失配 / 尺寸不足 → `ReadbackLayout` 错）。**device 像素对照不在本轮测试**（blocked-honest）。

## 2A. G3.2 present 面条款（RXS-0220 ~ RXS-0222;RFC-0013 §4.A;RD-019 兑现）

> 本节承 **[RFC-0013](../rfcs/0013-industrial-rendering.md) §4.A**（G3 工业渲染期 present 章,Agent Approved 2026-07-18,验收门 G-G3-2）把 §3 裁决地基 **Q-Present=offscreen-first**（窗口 present 登 RD-019）按其 backfill_condition **全量兑现**:UC-04 deferred 渲染器从「离屏出图 + 回读断言」升级为「可见窗口逐帧呈现 + 回读断言」+ 拖动 resize 重建 + Vulkan `OUT_OF_DATE` 重建收尾。**语言面零新语法**（D-130）:`.rx` 侧 present 面维持 RXS-0197/0198 typestate 0-byte 复用,全部增量在 C++ shim / rurix-rt 运行时层——**UC-04 窗口 present 是纯 D3D12 图形管线,独立走 C++ shim,不实例化 RXS-0197 的 CUDA↔D3D12 interop present typestate**（两种 present 机制不混,RFC-0013 SC-5）。条款体按 FLS 体例分节,**严禁 UB 节**（装配违例以装配期可预测 6xxx 定义,纯运行期/环境失败不占 RX 码;P-01 strict-only）。**offscreen 不被替代**:步骤 48（ci/dxil_uc04_device_smoke.py）硬门 0-byte 不动,present 不得替代 offscreen 真跑（RD-019 backfill_condition 原文）。

### RXS-0220 UC-04 可见窗口 flip-model swapchain present 装配与呈现循环

§4.A1 裁决:UC-04 deferred 渲染器把 lighting pass final 输出经 `IDXGIFactory2::CreateSwapChainForHwnd` + `DXGI_SWAP_EFFECT_FLIP_DISCARD`（flip-model 恒定）装配到**可见** win32 窗口（`WS_OVERLAPPEDWINDOW + ShowWindow`）,逐帧 record → backbuffer 状态迁移 → `Present(sync_interval, flags)`。本条只承诺**present 会话装配与呈现循环结构的 host 侧可判定核验**;装配违例 → strict-only 显式错。🔒 host↔shim 二进制布局 / present 会话 ABI 不冻结为 stable（承 RXS-0167 同级:实现确定、gate 后、非 stable;stable 面随 RD-008）。

#### Syntax

present 装配与呈现循环为运行时库面,非语言文法面。可见窗口 present 无 `.rx` 面（RXS-0197/0198 present typestate 维持不动,UC-04 窗口 present 独立走 C++ shim,SC-5）——「零新语法」因此成立,不新增 lang item / 方法 / RX3xxx 类别。

#### Legality

- L1（可装配 present 会话）:swapchain desc = `FLIP_DISCARD` + `BufferCount ∈ {2,3}`（默认 3）+ image format 与 lighting pass final RT 格式一致 + 可见窗口（`WS_OVERLAPPEDWINDOW`）+ `sync_interval ∈ {0,1}` + 逐帧迁移锚点集含 `RENDER_TARGET → COPY_SOURCE`（readback copy）`→ PRESENT` → 可装配为 present 会话描述（镜像 RXS-0167 PSO↔RT 一致性口径 + RXS-0169 手动状态迁移口径）。
- L2（present 装配违例,strict-only）:swapchain desc ↔ final RT 格式/缓冲数失配 / 请求 blt-model 或不支持的 swap effect / 缺 `PRESENT` 态迁移锚点 → **RX6027** `runtime.uc04_present_assembly`。无 fallback。
- L3（tearing 能力面）:`sync_interval = 0` 且请求 tearing 须 `CheckFeatureSupport(DXGI_FEATURE_PRESENT_ALLOW_TEARING)` 探测通过 + `ALLOW_TEARING` 建链/呈现旗标成对;能力缺失 = **纯运行期确定性拒**（不静默降级为 vsync,不占 RX 码;Q-P-TearingFail / 06 §8.2 环境口径）。
- 🔒（边界）:present 会话 host↔shim 二进制布局 / D3D12 swapchain COM 生命周期越出本条 = ABI 禁区,本条仅作 present 会话装配输入间一致性边界声明。

#### Dynamic Semantics

present 装配核验为 host 侧确定性变换。**device 段呈现循环**:每帧 record（deferred 三 pass 复用既有编排,RXS-0168 结构 0-byte）→ backbuffer `RENDER_TARGET → COPY_SOURCE`（readback）`→ PRESENT` → `Present(sync_interval, flags)` 逐帧 `S_OK`;状态迁移沿 RXS-0169 手动编排——**本条不引入任何自动状态推导**（自动推导 = RFC-0013 §4.D,不在本文件）。缺 `PRESENT` 态迁移 = 装配核验显式拒（L2）+ debug layer 真跑翻红（G-G3-2 RED 判据）。**可见性为语义承诺**（WS_OVERLAPPEDWINDOW+ShowWindow）,机器判据 = flip-model `Present` 逐帧 `S_OK` + readback 数值断言,**不断言「人眼可见 scanout」**（scanout 内容不可编程回读,诚实边界,Q-P-VisibleWindow / RFC-0013 §9.2 P-1）。

#### Implementation Requirements

- IR1（装配核验）:host 侧 [`uc04_demo::present::assemble_present`](../src/uc04-demo/src/present.rs)`(&PresentRequest)` 核验 swapchain desc ↔ final RT 格式/缓冲数一致性 + swap effect + 态迁移锚点集,产 `PresentSession` 装配描述;纯 host/safe。
- IR2（shim present 段,加性独立入口）:device 呈现循环经 `rx_uc04_present_run(...)` 独立入口（present 参数:宽高/帧数/sync_interval/tearing/resize 注入点）承载,携其**自有版本常量** `RX_UC04_PRESENT_ABI_VERSION`（恒 == 3、`>=3` 语义）;既有 `rx_uc04_offscreen_run` 入口版本常量恒 == 2、函数体字节不变（步骤 48 0-byte 守卫,RFC-0013 §4.A4/E-1;采样等新能力一律走新增独立入口,不扩 offscreen 参数面 SC-6）。
- IR3（strict-only）:present 装配违例 → `Uc04Error::PresentAssembly`（RX6027）。无运行期 fallback。
- IR4（测试锚定）:≥1 `//@ spec: RXS-0220`——accept（一致 present 请求 → `PresentSession`）+ reject（格式/缓冲数失配 / blt-model / 缺 PRESENT 锚点 → `PresentAssembly`）。**device N 帧逐帧 S_OK 由 ci/uc04_present_smoke.py 覆盖**（有显示环境;无则 SKIP,RXS-0222）。

### RXS-0221 swapchain 失效与重建（D3D12 ResizeBuffers / Vulkan OUT_OF_DATE·SUBOPTIMAL）

§4.A2 裁决（跨后端不变式）:**swapchain 失效是正常路径不是错误**;重建序 = 等待 GPU idle → 释放全部 backbuffer 引用/尺寸依赖视图 → 重建（重查 surface caps extent）→ 首帧重新校验。本条**单条承载跨后端重建不变式**;[vulkan_backend.md](vulkan_backend.md) RXS-0210 走加性修订行引用（Q-P-RebuildHome:重建序为后端无关不变式,单条防两文件语义漂移）。重建核验违例 → strict-only 显式错。

#### Syntax

swapchain 失效与重建为运行时库面,非语言文法面。

#### Legality

- L1（可重建）:**D3D12 载体** = `WM_SIZE` → `ResizeBuffers(0, w, h, DXGI_FORMAT_UNKNOWN, flags)`（缓冲数/格式恒定,尺寸取新客户区）;重建前 RTV/依赖资源全释放,重建后 RTV 重建。**Vulkan 载体** = `vkAcquireNextImageKHR`/`vkQueuePresentKHR` 返回 `VK_ERROR_OUT_OF_DATE_KHR`（与可选 `SUBOPTIMAL_KHR`）→ `vkDeviceWaitIdle` → 重建 swapchain/imageView/framebuffer（重查 surface caps extent）→ 重录后续帧。两载体重建后**首帧 readback 再断言**（G-G3-2「重建后再 readback 绿」判据）。
- L2（重建核验失败,strict-only）:重建后格式/缓冲数漂移 / 视图未重建即录制 = host 侧可判定装配违例 → **RX6028** `runtime.uc04_resize_rebuild`（§5.1 预测 RX6028,Q-P-CodeGranularity 装配核验 ×1 + 重建核验 ×1）。无 fallback。
- L3（重建协商,host 可判定）:present/acquire 返回码 → 三分类 `{Present, Rebuild, Fatal}` 协商为**纯 host 确定性判定**（`OUT_OF_DATE`/`ResizeBuffers` 触发 = Rebuild;`SUCCESS`/`SUBOPTIMAL` = Present;其余非预期码 = Fatal 终止）,纯 host 可单测。
- 🔒（边界）:swapchain COM/`VkSwapchainKHR` 句柄生命周期二进制布局不在本条;本条只核验重建序结构与协商判定。

#### Dynamic Semantics

重建协商为 host 侧确定性变换（相同返回码同结论）。**device 段**:`SetWindowPos` 合成 `WM_SIZE`（D3D12）/ ICD 触发 `OUT_OF_DATE`（Vulkan）驱动重建路径,重建后首帧 readback 再断言（合成 resize 不经用户拖拽 sizing loop,evidence 如实标注驱动方式,RFC-0013 §9.2 P-2）。Vulkan `OUT_OF_DATE` 在 NVIDIA/Windows 极难自然触发,重建路径依赖合成触发,AMD/Android 面照 G-MB1-6/7 尾门措辞不 claim（P-5）。

#### Implementation Requirements

- IR1（重建协商 helper）:host 侧纯函数（present/acquire 返回码 → `{Present, Rebuild, Fatal}`）纯 host 可单测;[`src/rurix-rt/src/vk.rs`](../src/rurix-rt/src/vk.rs) 协商 helper + resize 重建单测锚定。
- IR2（D3D12 载体）:[`uc04_demo::present`](../src/uc04-demo/src/present.rs) `WM_SIZE` → `ResizeBuffers` 重建 + 重建后核验（shim `rx_uc04_present_run` resize 注入点）。
- IR3（Vulkan 载体收尾）:`run_graphics_present` 对 `VK_ERROR_OUT_OF_DATE_KHR`/`SUBOPTIMAL_KHR` → `vkDeviceWaitIdle` + 重建 swapchain/imageView/framebuffer（重查 surface caps extent）+ 重建后首帧 readback 再断言;新 unsafe 沿 U27 扩注（graphics FFI 边界,0 新号,RFC-0013 §6.4）。
- IR4（strict-only + 测试锚定）:重建核验失败 → `Uc04Error::ResizeRebuild`（RX6028）;≥1 `//@ spec: RXS-0221`——accept（合法重建序 → 重建后 session）+ reject（格式/缓冲数漂移 / 视图未重建 → `ResizeRebuild`）+ vk.rs 重建协商 host 单测。

### RXS-0222 present headless readback 校验与 SKIP 纪律

§4.A3 裁决:**readback = present 面必要 device 证据**（MB1 W6 纪律,反「present 无 headless 数值校验」先例）——逐帧 present 前 `COPY_SOURCE` 态 copy 到 readback buffer;断言点 ≥3（首帧 / resize 重建后首帧 / 末帧,判据与步骤 48 offscreen 同族,布局复用 RXS-0170 / RX6022）。SKIP 纪律 = 无显示环境 device 段 SKIP（dev-env degrade 非 fake pass）,`RURIX_REQUIRE_REAL=1` 翻硬红。SKIP 不占 RX 码（工具/环境层口径,spec/release.md §3）。

#### Syntax

present readback 校验为运行时库/CI 工具面,非语言文法面。

#### Legality

- L1（readback 断言点）:逐帧 present 前 `COPY_SOURCE` copy 到 readback buffer;断言点 ≥3（首帧 / resize 重建后首帧 / 末帧）;readback 布局复用 RXS-0170（row pitch 对齐 / 格式一致 / 尺寸充足,失配 RX6022）。
- L2（SKIP 三态,不占码）:无显示环境/非交互桌面 → device 段 **SKIP = dev-env degrade 非 fake pass**（退出 0,打印 dev-env-degrade）;`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**;绿路 = device N 帧 readback 断言全过。三态均**不占 RX 码**（环境层口径）。
- L3（offscreen 不被替代）:步骤 48（ci/dxil_uc04_device_smoke.py）offscreen 硬门 **0-byte 不动**;RD-019 close 留痕明记「present 不得替代 offscreen 真跑」（RD-019 backfill_condition 原文）。
- 🔒（证据边界,P-1）:readback 在 present 前 copy,证明的是**渲染产物非 scanout 像素**;`Present` `S_OK` 仅证提交被 DWM 接受。本条只 claim「呈现链路数值可校验」,不 claim「readback == 呈现内容」。

#### Dynamic Semantics

readback 断言为 device 真跑面（承 RXS-0170 host 布局锚 + RXS-0220 呈现循环）。SKIP/REQUIRE_REAL/GREEN 三态确定;`OUT_OF_DATE`/遮挡 `DXGI_STATUS_OCCLUDED` 等纯运行期/环境失败确定性诊断 + 终止,不占 RX 码（P-3/P-4）。CI 内建 `red_self_test`:篡改 `PRESENT` 态迁移锚点 → present 装配核验拒（host 段）/ debug layer 真跑报错翻红（device 段）,证门非空过。

#### Implementation Requirements

- IR1（CI 三态）:[`ci/uc04_present_smoke.py`](../ci/uc04_present_smoke.py)（步骤 61）host 段恒跑（present 装配核验单测 + typestate 编译面）+ device 段（present N 帧 readback 断言 + resize 后再断言,gate `real-shim` + GPU）+ 内建 `red_self_test`（篡改 PRESENT 态迁移翻红）+ 无显示 SKIP + `RURIX_REQUIRE_REAL=1` 硬红。
- IR2（evidence + counter）:device 绿写 `evidence/uc04_present_*.json`（`present_ok=true` + 断言点像素 + adapter）;`g3.counter.uc04_present_frames`（RFC-0013 §6.5 PR-P2）登记 milestones/g3/g3_budget.json + ci/budget_eval.py evaluator 分支同 PR（未知 id 强制 FAIL）。
- IR3（测试锚定）:≥1 `//@ spec: RXS-0222`——readback 断言点核验单测 + SKIP/REQUIRE_REAL 三态。

## 3. 裁决摘要与实现门控

承 RFC-0006 §9 agent 裁决（Accepted / Approved 2026-06-28,AI agent 自主）:

- **Q-Present = offscreen-first**:offscreen 渲染 + 像素回读对照为 G-G2-4 必要面;窗口 swapchain present 作后续可选阶段,不阻塞 G-G2-4（窗口 present defer → **RD-019**）。
- **Q-DemoCrate = 独立 demo crate `src/uc04-demo`**:默认 `unsafe_code=deny`;D3D12 边界若必须 unsafe 集中到最小 runtime module,按硬规则 9 每 `unsafe` 块 `// SAFETY:` + unsafe-audit 注册（**U23** 续号,归 PR-F2）。
- **Q-RuntimeShape = safe wrapper**:最小 D3D12 device/queue/PSO/command list/resource/barrier 封装,复用 RFC-0001 device 基座;运行时 ABI 明确 **non-stable**,不进入语言 stable 面（🔒,stable 面随 RD-008）。
- **Q-DeferredPass = 最小 deferred**:G-buffer（albedo + normal + depth）→ 单光源 lighting → offscreen readback;窗口 present 不作 G-G2-4 必要条件。
- **Q-Barrier = 首期手动 barrier 编排**:实现层显式插入 RT → SRV → RT/Copy/Readback 状态转换;不做编译器自动状态跟踪（自动状态推导 defer → **RD-020**）;🔒 barrier 并发/可见性语义本体不在本期,触及即升档（agent Full RFC）。
- **Q-Texture = 不落纹理内存模型本体**:首期只消费 opaque `Texture2D`/`Sampler` 句柄 + D3D12 RT/SRV 视图绑定;🔒 采样 opcode / LOD·导数 / 越界 / 缓存一致性等 06 §4.2 语义触及即停手,另起 agent Full RFC（defer → **RD-021**）。
- **Q-Range = RXS-0167 ~ RXS-0170**:4 条锁定,对齐 §2 条款体。
- **Q-Err = 6xxx codegen/装配段,自 RX6018 起**:编译期/装配期可预测错误按真实可达类别只追加分配 + en/zh message-key（`ci/bilingual_coverage.py` 覆盖）;D3D12 API 返回的纯运行期/环境失败不滥发语言 RX,作 smoke/evidence runtime failure 报告。**PR-F1 不预留、不预造、不落码、不改 `registry/error_codes.json`**（当前 6xxx 段最高现存 RX6017,落码随 PR-F2）。
- **Q-Gate = 新增运行时/demo 专属 gate**（推荐 `d3d12-runtime` 或 `uc04-demo`,终名随 PR-F2）;**不**把 D3D12 runtime 面塞进 `dxil-backend`——`dxil-backend` 只作为 codegen 前置依赖。
- **Q-CIStep = step 48 offscreen readback REQUIRE_REAL**:对齐步骤 46/47,`RURIX_REQUIRE_REAL=1` 下缺 D3D12/MSVC/signed DXC pin/validator/GPU 即红;窗口 present 路径若存在可 SKIP,但不替代 offscreen 真跑。CI 步骤 48 落地归 agent / 实现 PR。

实现门控:

- **Q-File 人工定调（2026-06-28）**:owner（Language Lead）在本工作会话确认 PR-F1 的 spec 落点采用新建本文 `spec/d3d12_runtime.md`（镜像 RFC-0005 `binding_layout.md` 独立成文先例）,不延伸既有 spec 文件。Codex 仅代录该人工决定,非 AI 代签 / 代决。
- **Feature gate**:新增 `d3d12-runtime`/`uc04-demo` 专属 gate（Q-Gate）,不复用 `dxil-backend` 暴露面。
- **Registry**:§9 Q-RD 裁决 append-only 登记 **RD-019**（窗口 swapchain present defer）/ **RD-020**（自动资源状态跟踪推导 defer）/ **RD-021**（纹理内存模型映射 defer,须 agent Full RFC）——PR-F1 落 `registry/deferred.json`（下一个未用 RD = RD-019,RD-016 已跳号永不复用,10 §9.5）。错误码段位不预造（Q-Err,RX6018 起留 PR-F2）;包 registry（D-312,SG-007）维持 not_triggered,不开 SG。
- **PR 序**:**PR-F1 = spec 脚手架**——文件 + [README.md](README.md) 文件清单/修订记录 + registry RD-019/020/021;**PR-F2（agent 闸门）= 条款体 RXS-0167~0170 + `src/uc04-demo` demo crate + safe wrapper D3D12 封装 + 首期手动 barrier + offscreen readback + 6xxx 错误码自 RX6018 落码 + golden/bless + device 真跑/run URL**（G-G2-4 闭环;CI step 48 落地 + G-G2-4 签字归 agent）。

### 3.1 实现状态（PR-F2 blocked-honest interim slice,2026-06-29）

owner（Language Lead）2026-06-29 裁定 PR-F2 以 **blocked-honest interim slice** 落地（前置 RD-013 仍 open,无法达成 device 真绿）。本轮**已落 host 侧可交付面**:

- §2 带编号条款体 RXS-0167~0170 + 每条 ≥1 `//@ spec` 测试锚定（host accept/reject;trace_matrix 全锚定,166→170）。
- [`src/uc04-demo`](../src/uc04-demo) host 侧 safe 装配/编排模型（PSO 装配一致性 / deferred pass 编排 / barrier 锚点 / readback 布局）,纯 host/safe **零新 unsafe**（无 FFI 执行 → 不消费 U23;U23 + `unsafe-audit/uc04-demo.md` 归 RD-013 解锁后含真实 D3D12 执行的 device PR），复用 RFC-0005 RTS0（P-11）。新 feature gate `d3d12-runtime`（不复用 `dxil-backend`）。
- 6xxx 装配期错误码 **RX6018~RX6022**（`registry/error_codes.json` + en/zh message-key,`ci/bilingual_coverage.py` 全对齐）。

**本轮明确未达成（blocked-honest,归 RD-013 解锁后的 device PR + owner）**:device hardware 多 pass deferred draw + offscreen 像素对照真跑（Q-CIStep step 48 REQUIRE_REAL）/ DXIL·像素 golden bless / CI step 48 接线（owner 2026-06-29 裁定**本轮不接线 step 48 入 `pr-smoke.yml`**,避免常驻红门）/ device run URL / **G-G2-4 签字**。按 G-G2-4 防降级硬门,device 真绿须 Rurix source 经图形=B DXIL（RD-013）→ RTS0 → D3D12 PSO → hardware 多 pass draw → offscreen readback,**不得**以手写 HLSL/DXIL、CPU 预填、单 pass、fullscreen copy、固定像素、host-only 模拟、窗口截图或 SKIP 替代;RD-013 阻塞期间 **标 blocked,不签 G-G2-4**（G2_CONTRACT G-G2-4 / CI_GATES 步骤 48）。本状态 agent 代录机器事实,非 agent 签署。

---

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-28 | 新建 d3d12_runtime.md（PR-F1 spec 脚手架,承 [RFC-0006](../rfcs/0006-uc04-deferred-renderer.md),owner Approved 2026-06-28）:登记文件名 + G2.4 UC-04 deferred 渲染器 / D3D12 运行时出图语义面说明 + **RXS-0167~0170 计划区间**（DXIL+RTS0→graphics PSO 装配一致性 / deferred 多 pass 编排 / 资源状态+barrier 编排锚点 / offscreen readback+像素对照）。**仅登记计划映射,不落带编号裸条款头**——条款体与每条 >=1 测试锚定随 PR-F2 同落,trace_matrix 维持全锚定。同步 agent 裁决摘要:Q-Present=offscreen-first→RD-019 / Q-DemoCrate=src/uc04-demo（unsafe_code=deny,U23 续号）/ Q-RuntimeShape=safe wrapper（运行时 ABI non-stable）/ Q-DeferredPass=G-buffer(albedo+normal+depth)→单光源→offscreen readback / Q-Barrier=首期手动编排→RD-020 / Q-Texture=不落纹理内存模型本体→RD-021 / Q-Range=4 条 / Q-Err=6xxx 自 RX6018（不预造）/ Q-Gate=d3d12-runtime/uc04-demo 专属 / Q-CIStep=step 48 offscreen REQUIRE_REAL。落点（Q-File,owner §9 未单列）取新建本文（镜像 RFC-0005 binding_layout.md 独立成文先例,请 owner 确认）。禁区不动:纹理路径内存模型映射 / barrier 并发语义本体 / 运行时 stable ABI / FFI ABI 二进制布局 / DXIL·SPIR-V UB 边界只作边界声明,不落语义本体。registry/error_codes.json / spike_gating.json 不动,不开 SG;不碰 00–14、不改 CI、不动 src/。 | **Full RFC**（RFC-0006 / PR-F1） |
| v1.1 | 2026-06-28 | **Q-File 人工定调留痕**:owner（Language Lead）在本工作会话确认 PR-F1 的 spec 落点采用新建本文 `spec/d3d12_runtime.md`（镜像 RFC-0005 `binding_layout.md` 独立成文先例）,不延伸既有 spec 文件。Codex 仅代录该人工决定,非 AI 代签 / 代决。范围仍为 PR-F1 scaffold:不落 `### RXS-####` 条款体、不接线实现、不改 CI/golden/device/error_codes/spike_gating。 | **Full RFC**（RFC-0006 / PR-F1） |
| v1.2 | 2026-06-29 | **PR-F2 blocked-honest interim slice:§2 计划映射升格为带编号条款体 `### RXS-0167 ~ ### RXS-0170`**（FLS 体例,按需分 Syntax / Legality / Dynamic Semantics / Implementation Requirements 节,**严禁 UB 节**,Legality 引 6xxx 实码,镜像 [binding_layout.md](binding_layout.md) RXS-0163~0166 先例）——RXS-0167 DXIL+RTS0→graphics PSO 装配一致性（host 侧装配核验,渲染目标失配 RX6018 / RTS0↔绑定失配 RX6019 复用 RFC-0005 `check_binding_consistency`,P-11）/ RXS-0168 deferred 多 pass 编排（几何 MRT→lighting SRV 采样→readback 结构核验,pass 顺序/目标缺失 RX6020;device 像素对照 blocked-on-RD-013）/ RXS-0169 资源状态+barrier 编排锚点（RT→SRV→RT/Copy/Readback 锚点存在性/合法性,缺/非法 RX6021;🔒 并发语义本体不落）/ RXS-0170 offscreen readback+像素对照（host 锚=readback 布局 RX6022;device 像素对照 REQUIRE_REAL blocked-on-RD-013）。配套 **host 侧 safe 装配/编排模型**落 [`src/uc04-demo`](../src/uc04-demo)（纯 host/safe,**零新 unsafe → 不消费 U23**;复用 [`binding_layout`](../src/rurixc/src/binding_layout.rs) RTS0 推导面 P-11）+ 新 feature gate `d3d12-runtime`（不复用 `dxil-backend`）+ 每条 ≥1 `//@ spec` 单测锚定（`ci/trace_matrix.py` 增扫 `src/uc04-demo`,**166→170 全锚定**）+ 6xxx 装配期错误码 **RX6018~RX6022**（`registry/error_codes.json` append-only + en/zh message-key,`runtime.uc04_*` 前缀,`ci/bilingual_coverage.py` PASS）。**device 段 blocked-honest**（§3.1）:hardware 多 pass deferred draw + offscreen 像素对照 / DXIL·像素 golden bless / CI step 48 接线 / device run URL / **G-G2-4 签字**均**未达成**——前置 RD-013（图形=B 入口 body 数据流降级）open,无 Rurix 自产可出图着色器;按 G-G2-4 防降级硬门**不得**以任何替代物伪造 device 绿,标 blocked 不签。owner 2026-06-29 裁定本轮**不接线 CI step 48 入 `pr-smoke.yml`**（归 RD-013 解锁后的 device PR）。禁区不动:纹理路径内存模型（RD-021）/ barrier 并发语义本体 / 运行时 stable ABI / FFI ABI 二进制布局 / DXIL·SPIR-V UB 只作边界声明。deferred.json append-only 留痕（RD-013/019/020/021 history + revision_log）,无 status 翻转;spike_gating.json 不动,不开 SG;G2_CONTRACT/CI_GATES 语义不改;不碰 00–14。 | **Full RFC**（RFC-0006 / PR-F2） |
| v1.3 | 2026-07-18 | **§2A 新增 G3.2 present 面条款 `### RXS-0220 ~ ### RXS-0222`**（承 [RFC-0013](../rfcs/0013-industrial-rendering.md) §4.A,Agent Approved 2026-07-18,验收门 G-G3-2;把 §3 裁决地基 Q-Present=offscreen-first 登记的 **RD-019** 窗口 present 按其 backfill_condition 全量兑现）——RXS-0220 可见窗口 flip-model swapchain present 装配与呈现循环（`CreateSwapChainForHwnd` + `FLIP_DISCARD` + BufferCount∈{2,3} + WS_OVERLAPPEDWINDOW 可见窗 + Present(sync_interval∈{0,1}) + tearing 参数面;逐帧迁移锚点 RENDER_TARGET→COPY_SOURCE→PRESENT 沿 RXS-0169 手动口径;装配违例 strict-only **RX6027**;RXS-0197 typestate 维持不动,窗口 present 独立走 shim SC-5）/ RXS-0221 swapchain 失效与重建（D3D12 ResizeBuffers / Vulkan OUT_OF_DATE·SUBOPTIMAL,失效=正常路径,重建序 idle→释放→重建→首帧再校验,重建核验失败 **RX6028**;vulkan_backend.md RXS-0210 加性修订行引用,Q-P-RebuildHome 单条承载跨后端不变式）/ RXS-0222 present headless readback 校验与 SKIP 纪律（三断言点首/重建后/末帧,布局复用 RXS-0170/RX6022;SKIP=dev-env degrade + REQUIRE_REAL 硬红 + red_self_test 篡改 PRESENT 迁移,不占 RX 码;步骤 48 offscreen 硬门 0-byte 不替代）。**语言面零新语法**（D-130）:`.rx` present 面维持 RXS-0197/0198 typestate 0-byte,全部增量在 C++ shim（`rx_uc04_present_run` 加性独立入口 ABI v3,自有版本常量 == 3;既有 `rx_uc04_offscreen_run` 入口 == 2 字节不变,步骤 48 0-byte 守卫,E-1/SC-6）/ rurix-rt 运行时层。配套 `src/uc04-demo/src/present.rs`（host present 装配/重建核验 accept/reject）+ device.rs PresentRequest FFI + main.rs present 子命令 + vk.rs `run_graphics_present` OUT_OF_DATE/SUBOPTIMAL 重建收尾（U27 扩注,0 新 U 号）+ 6xxx 装配期错误码 **RX6027/RX6028**（append-only + en/zh message-key `runtime.uc04_present_assembly`/`runtime.uc04_resize_rebuild`,bilingual 96→98）+ CI 步骤 61 `ci/uc04_present_smoke.py`（host 恒跑 + device 三态 SKIP/REQUIRE_REAL/GREEN + red_self_test）+ `g3.counter.uc04_present_frames`（g3_budget.json + budget_eval evaluator 同 PR）。条款 commit 先于实现 commit（EA1 #158 先例）;stable 快照因条款计数增长同 PR 重 bless + `tests/stable/bless_log.md` 追加（RXS-0180 L2 加性演进）;trace_matrix 全锚定。禁区不动:纹理路径内存模型 / barrier 并发语义本体 / 运行时 stable ABI / FFI ABI 二进制布局只作边界声明。 | **Full RFC**（RFC-0013 / §4.A / PR-P1） |
