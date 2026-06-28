# Rurix 语言规范 — UC-04 deferred 渲染器 / D3D12 运行时出图语义面（G2.4 起）

> 条款:**RXS-0167 ~ RXS-0170 计划区间**（G2.4 UC-04 deferred 渲染器 / 原生 D3D12 运行时出图语义面:DXIL + RTS0 → graphics PSO 装配一致性 / deferred 多 pass 编排 / 资源状态 + barrier 编排锚点 / offscreen readback + 像素对照）。体例见 [README.md](README.md)。
> 依据:**[RFC-0006](../rfcs/0006-uc04-deferred-renderer.md)**（UC-04 deferred 渲染器 / 原生 D3D12 运行时出图路径,owner Approved 2026-06-28,§9 全 11 项已裁）;06 §8.2 第 4/5 点（PSO 装配 / 资源状态 / barrier 运行时面 = G2 设计预留）;06 §4.2（纹理路径内存模型禁区,🔒）;04 P-01（strict-only）;04 P-11（host 绑定结构 ↔ shader 布局单一事实源）;[RFC-0002](../rfcs/0002-shader-stages.md)（着色阶段类型面）;[RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)（图形=B DXIL codegen 与禁区边界）;[RFC-0005](../rfcs/0005-binding-layout-inference.md) RXS-0163~0166（绑定布局推导 + RTS0 序列化）;[RFC-0001](../rfcs/0001-cuda-d3d12-interop.md)（D3D12 device/queue/swapchain 运行时先例）。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)（D-G2-4,G-G2-4）+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.4 子里程碑。
> 档位:**Full RFC**（RFC-0006;10 §3:本设计首次落 **D3D12 运行时执行面**——PSO 装配 / 资源状态机 / barrier 语义 / swapchain 呈现,并触 AGENTS 硬规则 5 禁区边界——**纹理路径内存模型映射（06 §4.2）** / **D3D12 运行时 stable ABI** / **host↔运行时·host↔DXIL FFI ABI 二进制布局** / **barrier·资源状态并发语义**）。RFC-0006 已由 owner（Language Lead）于 2026-06-28 批准并裁决 §9 全部路径项。**AI 无权自判 Direct**,判档以 RFC-0006 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **纹理路径内存模型映射（06 §4.2）** / **barrier 并发·可见性·内存序语义本体** / **D3D12 运行时 stable ABI / FFI ABI 二进制布局** / **DXIL·SPIR-V UB 边界** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**（10 §7.5）:PSO 装配不一致 / 资源状态非法转换 / barrier 缺失或冲突 / RTS0 与 PSO 不匹配以编译期/装配期可预测错误（6xxx 段,自 RX6018 起,落码归 PR-F2）或运行时显式失败（P-01 strict-only,无运行期 fallback）定义;D3D12 API 返回的纯运行期/环境失败不滥发语言 RX,作 smoke/evidence runtime failure 报告。
> 规范先行（AGENTS.md 硬规则第 7 条）:**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 >=1 测试锚定（`//@ spec: RXS-####`）。**本 PR-F1 spec 脚手架仅登记新文件名 + 计划区间 RXS-0167~0170,不落带编号裸条款头**——条款体与每条 >=1 测试锚定随 PR-F2 实现 PR 同落（条款 PR 先于实现 PR,trace_matrix 维持全锚定）。

---

## 1. 范围与编号区间

本文件承载 **UC-04 deferred 渲染器 / 原生 D3D12 运行时出图** 的语义条款（G2.4+,D-G2-4）。UC-04 是 G2.1 着色阶段类型面 + G2.2 DXIL B 链 codegen + G2.3 绑定布局推导的**首个端到端集成验证点**:运行时把 RFC-0004 产的 DXIL 着色器对象 + RFC-0005 推导的 RTS0 root signature 装配成可执行的多 pass deferred 管线,以编译期推导的单一事实源（P-11）装配,不在运行时手维护第二份绑定布局。

覆盖语义面（RFC-0006 §4 / §5 / §9）:

- **DXIL + RTS0 → graphics PSO 装配一致性**:运行时把 RFC-0004 DXIL 着色器对象（VS/PS）+ RFC-0005 推导的 RTS0 + 渲染目标格式/深度状态组装成 graphics PSO;RTS0 与 PSO 一致性承 G-G2-3 `CreateRootSignature` accept 见证。装配不一致 → strict-only 显式错（无运行期 fallback,P-01）。当前 Rurix 运行时装配面仍为待建面,不得冒充已实测。
- **deferred 多 pass 编排**:§9 Q-DeferredPass 裁决为最小集 = 几何 pass（G-buffer:albedo + normal + depth MRT）→ 单光源 lighting pass（采样 G-buffer 作 shader resource）→ offscreen readback;pass 顺序/目标缺失 → strict-only 显式错。
- **资源状态 + barrier 编排锚点**:§9 Q-Barrier 裁决为首期手动 barrier 编排——pass 间 G-buffer 资源状态转换（`RENDER_TARGET` → `PIXEL_SHADER_RESOURCE` → 回 `RENDER_TARGET` / Copy / Readback）由运行时显式插入;不做编译器自动状态跟踪（自动状态推导 defer → RD-020）。本面只承诺**编排锚点（哪里需要状态转换）**;🔒 barrier 并发/可见性/内存序语义本体不在本文件。
- **offscreen readback + 像素对照**:§9 Q-Present 裁决为 offscreen-first——offscreen 渲染后回读像素做数值对照（对齐 G-G2-2/G-G2-3 readback 先例,CI device 可真跑,REQUIRE_REAL）;窗口 swapchain present 作后续可选阶段,不阻塞 G-G2-4（窗口 present defer → RD-019）。

明确不在本文件落语义本体的范围:

- **🔒 纹理路径内存模型映射（06 §4.2）**:G-buffer 写入（MRT render target）与 lighting pass 采样（SRV）的纹理访问语义 / 采样 opcode / 描述符编码 / 缓存一致性 / LOD·导数 / 越界采样后果 / memory-order 留独立 owner Full RFC（§9 Q-Texture defer → RD-021）。首期只消费 opaque `Texture2D`/`Sampler` 句柄 + D3D12 RT/SRV 视图绑定。
- **🔒 barrier / 资源状态并发语义本体**:barrier 的并发/可见性/内存序语义本体不在本文件;本面仅定义编排锚点,语义本体「需人工升档」（owner Full RFC）。
- **🔒 D3D12 运行时 stable ABI / host↔运行时·host↔DXIL FFI ABI 二进制布局**:运行时封装层（device/queue/PSO/command list 的 host↔D3D12 边界）不冻结为 stable 语言/运行时 ABI（与 RFC-0004 §4.6(a) / RFC-0005 RXS-0165 同级:实现确定、gate 后、非 stable;stable 面随 RD-008,G2.5 候选触发点）。
- **🔒 DXIL/SPIR-V UB 边界**:不建立独立于源码语义的 DXIL/SPIR-V UB 契约（承 RFC-0004 §4.6(c)）;依赖未建模行为的运行时 lowering 须显式拒绝。
- **运行时/库实现 + demo crate**:D3D12 运行时封装层、PSO 装配、资源状态/barrier 编排、`src/uc04-demo` demo crate、command list 录制、错误码落码、golden、device 真跑均归 PR-F2（owner 闸门,G-G2-4）。

**编号区间**:本文件计划条款为 **RXS-0167 ~ RXS-0170**（全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;当前最高现存 RXS-0166 @ [binding_layout.md](binding_layout.md);区间 §9 Q-Range 已裁锁定 4 条）。本轮 **仅登记区间预留**,**不落带编号裸条款头**;条款体与每条 >=1 测试锚定随 PR-F2 同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款计划映射（无条款体）

> 本节仅为 PR-F1 计划映射,零 `### RXS-####` 三级标题,trace_matrix 不计本节。带编号条款体随 PR-F2 实现 PR 同落（对齐 RFC-0006 §5 下游条款计划表）。

| 条款（计划,§9 Q-Range 已裁锁定） | 标题 | 测试锚定计划（每条 >=1,`//@ spec`） |
|---|---|---|
| RXS-0167 | DXIL + RTS0 → graphics PSO 装配一致性 | accept（RTS0/着色器/RT 格式一致 → PSO 装配）+ reject（不一致 → strict-only 显式错）+ host 侧装配核验 |
| RXS-0168 | deferred 多 pass 编排（几何 pass MRT → lighting pass 采样 G-buffer → offscreen readback） | accept（多 pass 编排）+ reject（pass 顺序/目标缺失）+ device 像素对照 |
| RXS-0169 | 资源状态 + barrier 编排锚点（pass 间 RT → SRV → RT/Copy/Readback 状态转换;首期手动编排;🔒 并发语义本体不在本条） | accept（状态转换编排）+ reject（缺 barrier/非法转换 → strict-only）;🔒 并发语义本体「需人工升档」 |
| RXS-0170 | offscreen readback + 像素对照（Q-Present=offscreen-first;窗口 present 不进必要条款,登 RD-019） | offscreen readback accept + device 像素对照（REQUIRE_REAL,Q-CIStep）;窗口 present 路径作 RD 后续 |

## 3. 裁决摘要与实现门控

承 RFC-0006 §9 owner 裁决（Accepted / Owner Approved 2026-06-28,AI 代录非代签）:

- **Q-Present = offscreen-first**:offscreen 渲染 + 像素回读对照为 G-G2-4 必要面;窗口 swapchain present 作后续可选阶段,不阻塞 G-G2-4（窗口 present defer → **RD-019**）。
- **Q-DemoCrate = 独立 demo crate `src/uc04-demo`**:默认 `unsafe_code=deny`;D3D12 边界若必须 unsafe 集中到最小 runtime module,按硬规则 9 每 `unsafe` 块 `// SAFETY:` + unsafe-audit 注册（**U23** 续号,归 PR-F2）。
- **Q-RuntimeShape = safe wrapper**:最小 D3D12 device/queue/PSO/command list/resource/barrier 封装,复用 RFC-0001 device 基座;运行时 ABI 明确 **non-stable**,不进入语言 stable 面（🔒,stable 面随 RD-008）。
- **Q-DeferredPass = 最小 deferred**:G-buffer（albedo + normal + depth）→ 单光源 lighting → offscreen readback;窗口 present 不作 G-G2-4 必要条件。
- **Q-Barrier = 首期手动 barrier 编排**:实现层显式插入 RT → SRV → RT/Copy/Readback 状态转换;不做编译器自动状态跟踪（自动状态推导 defer → **RD-020**）;🔒 barrier 并发/可见性语义本体不在本期,触及即升档（owner Full RFC）。
- **Q-Texture = 不落纹理内存模型本体**:首期只消费 opaque `Texture2D`/`Sampler` 句柄 + D3D12 RT/SRV 视图绑定;🔒 采样 opcode / LOD·导数 / 越界 / 缓存一致性等 06 §4.2 语义触及即停手,另起 owner Full RFC（defer → **RD-021**）。
- **Q-Range = RXS-0167 ~ RXS-0170**:4 条锁定,对齐 §2 计划映射。
- **Q-Err = 6xxx codegen/装配段,自 RX6018 起**:编译期/装配期可预测错误按真实可达类别只追加分配 + en/zh message-key（`ci/bilingual_coverage.py` 覆盖）;D3D12 API 返回的纯运行期/环境失败不滥发语言 RX,作 smoke/evidence runtime failure 报告。**PR-F1 不预留、不预造、不落码、不改 `registry/error_codes.json`**（当前 6xxx 段最高现存 RX6017,落码随 PR-F2）。
- **Q-Gate = 新增运行时/demo 专属 gate**（推荐 `d3d12-runtime` 或 `uc04-demo`,终名随 PR-F2）;**不**把 D3D12 runtime 面塞进 `dxil-backend`——`dxil-backend` 只作为 codegen 前置依赖。
- **Q-CIStep = step 48 offscreen readback REQUIRE_REAL**:对齐步骤 46/47,`RURIX_REQUIRE_REAL=1` 下缺 D3D12/MSVC/signed DXC pin/validator/GPU 即红;窗口 present 路径若存在可 SKIP,但不替代 offscreen 真跑。CI 步骤 48 落地归 owner / 实现 PR。

实现门控:

- **Q-File 人工定调（2026-06-28）**:owner（Language Lead）在本工作会话确认 PR-F1 的 spec 落点采用新建本文 `spec/d3d12_runtime.md`（镜像 RFC-0005 `binding_layout.md` 独立成文先例）,不延伸既有 spec 文件。Codex 仅代录该人工决定,非 AI 代签 / 代决。
- **Feature gate**:新增 `d3d12-runtime`/`uc04-demo` 专属 gate（Q-Gate）,不复用 `dxil-backend` 暴露面。
- **Registry**:§9 Q-RD 裁决 append-only 登记 **RD-019**（窗口 swapchain present defer）/ **RD-020**（自动资源状态跟踪推导 defer）/ **RD-021**（纹理内存模型映射 defer,须 owner Full RFC）——PR-F1 落 `registry/deferred.json`（下一个未用 RD = RD-019,RD-016 已跳号永不复用,10 §9.5）。错误码段位不预造（Q-Err,RX6018 起留 PR-F2）;包 registry（D-312,SG-007）维持 not_triggered,不开 SG。
- **PR 序**:**PR-F1（本文）= spec 脚手架**——本文件 + [README.md](README.md) 文件清单/修订记录 + registry RD-019/020/021;**PR-F2（owner 闸门）= 条款体 RXS-0167~0170 + `src/uc04-demo` demo crate + safe wrapper D3D12 封装 + 首期手动 barrier + offscreen readback + 6xxx 错误码自 RX6018 落码 + golden/bless + device 真跑/run URL**（G-G2-4 闭环;CI step 48 落地 + G-G2-4 签字归 owner）。

---

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-28 | 新建 d3d12_runtime.md（PR-F1 spec 脚手架,承 [RFC-0006](../rfcs/0006-uc04-deferred-renderer.md),owner Approved 2026-06-28）:登记文件名 + G2.4 UC-04 deferred 渲染器 / D3D12 运行时出图语义面说明 + **RXS-0167~0170 计划区间**（DXIL+RTS0→graphics PSO 装配一致性 / deferred 多 pass 编排 / 资源状态+barrier 编排锚点 / offscreen readback+像素对照）。**仅登记计划映射,不落带编号裸条款头**——条款体与每条 >=1 测试锚定随 PR-F2 同落,trace_matrix 维持全锚定。同步 owner 裁决摘要:Q-Present=offscreen-first→RD-019 / Q-DemoCrate=src/uc04-demo（unsafe_code=deny,U23 续号）/ Q-RuntimeShape=safe wrapper（运行时 ABI non-stable）/ Q-DeferredPass=G-buffer(albedo+normal+depth)→单光源→offscreen readback / Q-Barrier=首期手动编排→RD-020 / Q-Texture=不落纹理内存模型本体→RD-021 / Q-Range=4 条 / Q-Err=6xxx 自 RX6018（不预造）/ Q-Gate=d3d12-runtime/uc04-demo 专属 / Q-CIStep=step 48 offscreen REQUIRE_REAL。落点（Q-File,owner §9 未单列）取新建本文（镜像 RFC-0005 binding_layout.md 独立成文先例,请 owner 确认）。禁区不动:纹理路径内存模型映射 / barrier 并发语义本体 / 运行时 stable ABI / FFI ABI 二进制布局 / DXIL·SPIR-V UB 边界只作边界声明,不落语义本体。registry/error_codes.json / spike_gating.json 不动,不开 SG;不碰 00–14、不改 CI、不动 src/。 | **Full RFC**（RFC-0006 / PR-F1） |
| v1.1 | 2026-06-28 | **Q-File 人工定调留痕**:owner（Language Lead）在本工作会话确认 PR-F1 的 spec 落点采用新建本文 `spec/d3d12_runtime.md`（镜像 RFC-0005 `binding_layout.md` 独立成文先例）,不延伸既有 spec 文件。Codex 仅代录该人工决定,非 AI 代签 / 代决。范围仍为 PR-F1 scaffold:不落 `### RXS-####` 条款体、不接线实现、不改 CI/golden/device/error_codes/spike_gating。 | **Full RFC**（RFC-0006 / PR-F1） |
