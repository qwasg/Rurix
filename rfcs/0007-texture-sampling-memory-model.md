# RFC-0007 — 纹理采样内存模型(语义本体条款化)

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0007（4 位制，编号永不复用，10 §9.5） |
| 标题 | 纹理采样内存模型:采样 opcode / 坐标空间 / LOD·导数 / 寻址·过滤 / 越界 / 缓存可见性的语义本体 |
| 档位 | **Full RFC**（10 §3:内存模型映射 06 §4.2 🔒 禁区 + 新语法 + 新 codegen 面;AGENTS 硬规则 5） |
| 状态 | Agent Approved（2026-06-30）。agent 完全自主批准（AGENTS v3.0 硬规则 1），批准后推进下游实现 PR |
| 承接里程碑 | G2.4（D-G2-4 / G-G2-4 严格面:lighting pass 真采样 G-buffer）;关闭 RD-021 |
| 关联条款 | 拟落 spec **RXS-0174~RXS-0176**（见 §5）:`spec/shader_stages.md`（采样表达式类型面）/ `spec/dxil_backend.md`（资源采样 rvalue 降级） |
| 依据决策 | 06 §4.2（内存模型禁区,纹理路径引入时扩展映射条款）· 06 §8.2（PSO·资源状态·纹理采样器类型化）· D-131 v1.4（图形=B 链）· RFC-0002（着色阶段 + `Texture2D<F>`/`Sampler` 类型面 RXS-0156）· RFC-0004（B 转译链）· RFC-0005（绑定布局推导 RTS0）· RFC-0006（UC-04 deferred 渲染器） |
| Provenance | `Assisted-by: claude-code:claude-opus-4.8`。agent 自主决策,批准后推进下游实现 |
| Agent 批准 | Approved — 2026-06-30;批准范围含 §4 全部 🔒 禁区子节(采样 opcode / 坐标 / LOD·导数 / 寻址·过滤 / 越界 / 缓存·memory-order);记录于 §9 裁决 + 本表 + §修订记录 |

---

## 1. 摘要

本 RFC 把**纹理采样内存模型**作为语义本体条款化,关闭 RD-021、废止 G2_CONTRACT §8.5 选项 B「不采样」折中,使 UC-04 deferred 渲染器的 lighting pass **真采样 G-buffer**(真延迟着色)。

通路(承 RFC-0004 图形=B 链):

```
Rurix 源 tex.sample(samp, uv)
  └→ AST MethodCall → typeck(RXS-0174 采样表达式类型面)
       └→ MIR Rvalue::ResourceSample(over ResourceBinding)
            └→ SPIR-V OpSampledImage + OpImageSampleExplicitLod(Lod 0.0)  [RXS-0175]
                 └→ SPIRV-Cross → HLSL tex.SampleLevel(samp, uv, 0.0)
                      └→ dxc → DXIL sample 指令  [RXS-0176 白名单扩展]
                           └→ D3D12 PSO(SRV t0 / Sampler s0 descriptor table,RFC-0005)
                                └→ hardware lighting pass 采样 G-buffer albedo SRV
```

**首期收敛子集**(§4.2):显式 LOD 0 采样(`OpImageSampleExplicitLod` + `Lod` 操作数 = `0.0`),规避 fragment 隐式导数复杂度;`Texture2D<f32>` + `Sampler` + `vec2<f32>` 坐标 → `vec4<f32>` 结果。隐式 LOD/导数、mip/LOD bias、gather、比较采样器、其余纹理维度、可写 image 登记为新 deferred **RD-022**(§8),不偷偷略过。

## 2. 动机

- **用户痛点 / 已锁路线**:UC-04 deferred 渲染器(11 §5,G-G2-4)的本质 = 几何 pass 写 G-buffer(MRT)→ **lighting pass 采样 G-buffer(SRV)** 做延迟着色。G2_CONTRACT §8.5 已按选项 B(lighting pass 不采样、走自身插值输入)签 G-G2-4,但那是诚实折中:final 像素不真依赖采样到的 G-buffer 值,**未达「真 deferred shading」严格面**。本 RFC 升级为严格面并经 §8.6 supersede §8.5。
- **结构缺口**(承 `evidence/g2.4-uc04-deferred/rd021_scoping_20260629.md`):RXS-0171 图形=B body 降级白名单不含资源/纹理/采样访问 → `RX6013`;SPIR-V opcode 表无 `OpImageSample*`;`emit_resource` 只 emit opaque 绑定声明(`OpTypeImage`/`OpTypeSampler` + 装饰);Rurix 源无采样语法。四处缺口由本 RFC 一次补齐(语法 + 类型面 + MIR rvalue + SPIR-V 采样 opcode)。

**为何需要 Full RFC(而非 Direct/Mini)**:本变更同时触及 (1) **内存模型映射**(06 §4.2 🔒:纹理路径采样 opcode 语义 / 坐标空间 / LOD·导数 / 寻址·过滤 / 越界后果 / 缓存可见性与 memory-order)、(2) **新语法**(采样表达式)、(3) **新 codegen 面**(MIR→SPIR-V 采样 opcode)。AGENTS 硬规则 5:这些由 agent 自主经 Full RFC 落笔作留档与可追溯;判档争议向上取严。

## 3. 指导级解释(用户视角)

着色阶段函数把 `Texture2D<F>` 与 `Sampler` 作签名形参(RXS-0156 已有),在 body 内用**方法式采样表达式**读取纹理内容:

```rurix
struct LpVary { #[interpolate(perspective)] uv: vec2<f32> }
struct LpOut { color: vec4<f32> }

// lighting pass:真采样 G-buffer albedo(SRV t0)+ sampler(s0),输出 = f(采样值)
fragment fn uc04_lighting_fs(inp: LpVary, albedo: Texture2D<f32>, samp: Sampler) -> LpOut {
    let c: vec4<f32> = albedo.sample(samp, inp.uv);   // ← 采样表达式
    LpOut { color: c }
}
```

- **语法**:`<texture>.sample(<sampler>, <coord>)`——`<texture>` 须 `Texture2D<F>` 句柄,`<sampler>` 须 `Sampler` 句柄,`<coord>` 须 `vec2<f32>`(归一化纹理坐标,见 §4.3)。
- **结果类型**:`vec4<F>`(2D 采样恒产 4 分量向量,§4.5)。
- **可用阶段**:首期仅 `fragment`(采样在 fragment 阶段可用;`vertex` 采样需显式 LOD 语义,首期亦经显式 LOD 0 支持但 UC-04 不用,见 §4.2 / §9 Q3)。
- **首期语义**:`sample` = **在 LOD 0 上以 sampler 的寻址/过滤模式采样单层纹理**(显式 LOD,无隐式导数)。完整 mip/隐式导数/可配置 sampler 状态、整型 texel fetch、比较采样/gather/多分量纹理/可写 image 为新 deferred **RD-022 ~ RD-024**(§8)。

## 4. 参考级设计(采样内存模型本体,06 §4.2 纹理路径)

> 本节为 **06 §4.2 内存模型映射禁区(纹理路径)** 的语义本体落笔。06 §4.2 把 `Atomic<T, Scope>`
> 的 generic proxy 映射写入 spec,并显式声明「proxy(tex/generic)差异 MVP 不暴露;纹理路径(G2)
> 引入时再扩展映射条款」——本节即该扩展点的兑现。标 🔒 的子节为禁区语义本体,经本 Full RFC
> 由 agent 自主落笔(AGENTS 硬规则 5),**判档争议向上取严**。已落 spec `RXS-0176`(DS1~DS6 /
> IR1~IR3)为本节的可提取条款投影,二者**一字对齐**;本节为治理本体,spec 为机器核对锚点。

### 4.1 采样通路与四处结构缺口补齐(承 `rd021_scoping_20260629.md`)

停手分支 scoping(evidence)判定:UC-04 lighting pass 真采样 G-buffer 在升档前**结构不可达**,缺口四处——
(1) `RXS-0171` 图形=B body 降级白名单不含资源/纹理/采样访问 → `RX6013`;(2) SPIR-V opcode 表无
`OpImageSample*`;(3) `emit_resource` 只 emit opaque `OpTypeImage`/`OpTypeSampler` 绑定声明;(4) Rurix 源无采样语法。
本 RFC 一次补齐:**采样语法**(§3,复用 `MethodCall`)+ **类型面**(RXS-0174)+ **MIR rvalue**(`Rvalue::ResourceSample`)+
**SPIR-V 采样 opcode**(§4.5/§4.8,RXS-0175)+ **内存模型本体**(§4.3~§4.7,RXS-0176)。

### 4.2 首期收敛子集(显式 LOD 0,规避隐式导数)

首期**只**收敛:`Texture2D<f32>` 经 `Sampler` 的**显式 LOD 0** 采样、坐标 `vec2<f32>` 归一化 UV、结果
`vec4<f32>`、阶段 `fragment`。映射 `OpImageSampleExplicitLod`(ImageOperands `Lod` = 0x2,LOD 常量 `0.0`),
**规避 fragment 隐式导数(quad 派生 + 非均匀控制流)复杂度**。子集外构造经 `RX6023`(`codegen.dxil_sample_unsupported`,
strict-only)拒并登记 deferred(§8),**不偷偷略过、不占位式过度承诺**(P-01 strict-only / 14 §4)。

### 4.3 🔒 坐标空间与归一化

- `coord ∈ [0,1]²` 归一化 UV;`(0,0)` = 纹素左上、`(1,1)` = 右下(D3D 纹理坐标约定)。
- 采样在归一化坐标处取值,与纹理物理分辨率解耦(可移植)。
- **非归一化整型取址(texel fetch,`OpImageFetch`)不在本期** → **RD-023**。

### 4.4 🔒 寻址 / 过滤模式

- 寻址/过滤由绑定 `Sampler` 决定;**本期 `Sampler` = 静态默认**:min/mag/mip 线性过滤、UVW `clamp-to-edge`。
- **可配置 sampler 状态**(point / anisotropic / wrap / mirror / border、LOD bias、mip 选择)不在本期 → **RD-022**。

### 4.5 🔒 采样 opcode 语义 / LOD / 导数(结果类型)

- **DS(采样 opcode)**:`tex.sample(samp, coord)` = 在 `coord` 处、按绑定 sampler 过滤模式、对 `Texture2D<F>`
  **基础 mip 层(LOD 0)** 做过滤读取,产 `vec4<F>`(2D 采样恒产 4 分量;`F = f32` 首期)。
- **LOD / 导数**:本期 LOD 恒 `0.0`(**显式**),**无隐式导数依赖**——采样可出现于 fragment straight-line body
  (RXS-0171 直线切片)任意位置,无 quad 导数 / 非均匀控制流后果条款义务。
- **隐式 LOD**(`OpImageSampleImplicitLod` / `dx.op.sample`,依赖 quad 导数)+ LOD 选择 + 派生链一致性 → **RD-022**。

### 4.6 🔒 越界采样后果(well-defined,**严禁 UB 节**)

- 归一化坐标越界(`coord ∉ [0,1]²`)由 sampler `clamp-to-edge` 寻址吸收 → 取最近边缘纹素,**well-defined**。
- **无运行期未定义行为、无 UB 节**(P-01:strict-only,采样越界不是 UB,是 sampler 状态定义的确定性行为)。
  这是与 PTX 公理「mixed-size race 无约束」类 UB 边界的关键区别:纹理只读采样无竞争、无 UB。

### 4.7 🔒 缓存可见性与 memory-order

- SRV 采样为**只读**访问,无 store → **无 inter-thread memory-order 约束**(无可见性竞争问题),与 06 §4.2
  `Atomic` 的 scope/order 映射正交(采样不参与 morally-strong 指令对)。
- **跨 pass 写后读可见性**(几何 pass 写 G-buffer RT → lighting pass 采样 SRV)由 D3D12 `ResourceBarrier`
  (`RENDER_TARGET → PIXEL_SHADER_RESOURCE`)保证;**采样语义假定该 barrier 已就位**。缺失 → 渲染未定义,由
  `RXS-0169` deferred 编排校验(`RX6021`)在**编排层**拦截,**非采样语义层**(语义层不承担 barrier 缺失后果)。
- **可写 image(UAV)的 memory-order 不在本条**(跨线程写可见性 = 后续里程碑) → **RD-024**。

### 4.8 SPIR-V → B 链 → DXIL 降级映射(承 RFC-0004 图形=B 链)

```
Rvalue::ResourceSample { texture, sampler, coord }
  └→ OpLoad(纹理变量) + OpLoad(采样器变量)
       └→ OpSampledImage(组合 OpTypeSampledImage)
            └→ OpImageSampleExplicitLod(result vec4<F>, ImageOperands Lod=0x2, LOD 常量 0.0)
                 └→ spirv-cross → HLSL  tex.SampleLevel(samp, uv, 0.0)
                      └→ dxc → DXIL  dx.op.sampleLevel.f32
                           └→ dxv validator 接受
```

新增 SPIR-V opcode 常量:`OpTypeSampledImage`(27)/ `OpSampledImage`(86)/ `OpImageSampleExplicitLod`(88)。
产物保持 `spirv-val` 干净;`RXS-0159` 强制签名一致性校验门仍在 B 链末尾运行,**采样不旁路 `signature_gate::check`**。

### 4.9 绑定布局(SRV t0 / Sampler s0,承 RFC-0005,不重造)

`Texture2D<F>` → SRV、`Sampler` → Sampler,register/space/descriptor table 推导归 `RXS-0163~0166`
(binding_layout per-class 轴:SRV 自 `t0`、Sampler 自 `s0`)。本 RFC **不重造**绑定推导,只消费其产物:
device 侧 lighting pass root signature 由 `infer_root_signature` → `serialize_rts0` 推导的 RTS0 经
`CreateRootSignature` 真机解析(SRV t0 + Sampler s0 descriptor table)。**descriptor 编码 / 采样 opcode 二进制
布局 / register 数值不冻结为 stable**(承 RFC-0004 §4.6 / RFC-0005 §4.5 🔒)。

## 5. 下游 spec 条款计划表(已落条款的本体投影)

| 条款 | 文件 | 标题 | 测试锚定(每条 ≥1,`//@ spec`) |
|---|---|---|---|
| RXS-0174 | `spec/shader_stages.md` | 采样表达式类型面(`Texture2D<F>.sample(Sampler, vec2<f32>) → vec4<F>`,fragment 阶段;违例 RX3014) | accept `uc04_lighting_fs.rx` + reject(非 Texture2D / 非 Sampler / 非 fragment)+ UI golden |
| RXS-0175 | `spec/dxil_backend.md` | DXIL 图形=B 资源采样 rvalue 降级(`Rvalue::ResourceSample → OpImageSampleExplicitLod`;子集外 RX6023) | accept(降为 OpSampledImage+OpImageSampleExplicitLod)+ reject(子集外 → RX6023) |
| RXS-0176 | `spec/dxil_backend.md` | 🔒 纹理采样内存模型映射(§4.3~§4.7;DS1~DS6 / IR1~IR3) | device 数据流红绿(采样值依赖证明)+ clamp 越界 well-defined 断言 |

> 三条款已随本轮实现 PR 落条款体(条款先于 device 实现);本 RFC 为其规范上游本体。trace_matrix 全锚定。

## 6. feature gate / tracking / 实现序 + 真实红绿 + device 见证(10 §3 要件)

- **feature gate**:采样**类型面/codegen 面**承 `shader-stages` + `dxil-backend`(无独立新 gate;采样是着色 body
  能力的扩展,非新运行时面);device 真采样承 `uc04-demo` 的 `real-shim`(RFC-0006 Q-Gate 已立)。
- **实现序**(条款先于实现,硬规则 7):本 RFC §4 本体 + RXS-0174/0175/0176 条款体 → 前端(typeck/MIR)→
  codegen(SPIR-V 采样 opcode + B 链)→ device(shim RT→SRV + descriptor table + 真采样)→ golden bless + 签字。
- **真实红绿**:(a) host——子集外构造(隐式 LOD / 非 fragment / coord 非 vec2f)→ RX6023 strict-only 拒;
  (b) **device 数据流严格判据**(RXS-0176 IR2,本 RFC 核心):lighting `final = f(几何 pass 写入并被采样的
  G-buffer 值)`——篡改几何 pass FS 写入常量 → final 像素随之改变(红),复原绿(`ci/dxil_uc04_device_smoke.py`)。
  **仅「多 pass + 写 G-buffer」不充分;final 不依赖采样值即视为未达严格面。**
- **device 见证**:原生 D3D12 hardware 多 pass deferred draw(几何 pass MRT → RT→SRV barrier → lighting pass
  **真采样** G-buffer → offscreen readback),adapter 名 + 采样像素 + 数据流红绿对照(`DXIL_UC04: ok ...`)。
  CI step 48 `RURIX_REQUIRE_REAL=1`;本机 measured_local,CI run URL 已回填 self-hosted `rurix-dev-4070ti`(RTX 4070 Ti)pr-smoke step 48 全绿 run https://github.com/qwasg/Rurix/actions/runs/28442661542(不伪造)。

## 7. 备选方案

- **隐式 LOD 采样首期即上**(`OpImageSampleImplicitLod`):否决——引入 quad 派生 + 非均匀控制流后果的内存模型
  复杂度,与首期收敛子集冲突;留 RD-022,首期显式 LOD 0 已足以兑现真延迟着色。
- **采样语义本体留 spec 条款、不立 Full RFC**:否决——纹理路径内存模型映射是 06 §4.2 🔒 禁区(硬规则 5),
  须经 Full RFC 由 agent 自主落笔作留档与可追溯;裸条款不构成本体来源。
- **维持选项 B(不采样)**:否决(本 RFC 的废止对象)——final 不依赖采样值,未达真延迟着色严格面。
- **texel fetch(整型取址)替代归一化采样**:否决——首期 G-buffer 采样需过滤 + 归一化坐标可移植性;texel fetch 留 RD-023。

## 8. 不做(范围红线)+ 新 deferred

首期收敛子集(§4.2)外的采样子能力**登记新 deferred,不偷偷略过**(约束 2 / 14 §4)。下一未用 RD = **RD-022**
(RD-016 已跳号永不复用,10 §9.5):

| 编号 | 内容 | backfill 条件 |
|---|---|---|
| **RD-022** | 隐式 LOD / quad 导数 / LOD 选择 + 派生链一致性;可配置 sampler 状态(point/aniso/wrap/mirror/border、LOD bias、mip) | 需 mip/隐式导数/可配置过滤的渲染场景;经后续 Full RFC 增补 §4.5/§4.4 本体 |
| **RD-023** | 非归一化整型 texel fetch(`OpImageFetch` / `dx.op.textureLoad`) | 需精确纹素取址(无过滤)的场景;经后续 Full RFC 增补 §4.3 本体 |
| **RD-024** | 比较采样(shadow)/ gather / 多分量纹理元素 / 可写 image(UAV)写 + memory-order | 需阴影/gather/UAV 写的场景;UAV memory-order 触 06 §4.2 跨线程可见性,经后续 Full RFC |

- **🔒 不冻结 stable**:descriptor 编码 / 采样 opcode 二进制布局 / SPIR-V `Location` / register 数值(承 §4.9)。
- **🔒 不建立独立 UB 契约**:采样越界 well-defined(§4.6),无 UB 节(P-01)。

## 9. §9 agent 裁决清单(Agent Approved 2026-06-30)

> agent(完全自主,AGENTS v3.0 硬规则 1)于 2026-06-30 工作会话裁决下表全部项并批准 RFC-0007 全文(含 §4 全部
> 🔒 禁区子节);agent 自主记录裁决输入与机器事实。Provenance:`Assisted-by: claude-code:claude-opus-4.8`。

| 项 | 抉择 | 裁决 |
|---|---|---|
| Q-Syntax | 采样语法:方法式 `tex.sample(samp,uv)` / 内建 `sample(tex,samp,uv)` | **方法式** `tex.sample(samp, coord)`,复用既有 `MethodCall` 产生式(无新 parser/AST 节点),与 Rust 风格一致 |
| Q-LOD | 首期 LOD:显式 LOD 0 / 隐式 LOD | **显式 LOD 0**(`OpImageSampleExplicitLod`,Lod 0.0)规避隐式导数;隐式 → RD-022 |
| Q-Subset | 首期类型:仅 `Texture2D<f32>`+`vec2<f32>`→`vec4<f32>` / 更宽 | **仅 `Texture2D<f32>`** 收敛子集;子集外 RX6023 + RD-022~024 |
| Q-Stage | 可用阶段:仅 fragment / fragment+vertex | **首期 fragment**(UC-04 lighting);vertex 采样首期亦显式 LOD 支持但 UC-04 不用 |
| Q-ErrCode | 诊断码:复用 RX6013 / 新码 | **类型面 RX3014**(typeck,RXS-0174)+ **codegen 子集外 RX6023**(RXS-0175,区别于 RX6013 通用不可映射) |
| Q-Defer | 规避子能力处置 | 登记 **RD-022 / RD-023 / RD-024**(§8),不偷偷略过;与 spec RXS-0175 L2 / RXS-0176 DS 引用号一致 |
| Q-MemModel | §4.3~§4.7 🔒 禁区本体落点 | **本 RFC §4 落笔**(06 §4.2 扩展点);spec RXS-0176 为一字对齐的可核对投影 |

## 10. 稳定化与 provenance

- **稳定面**:采样语法 / 类型面 / 内存模型映射的**语义**经本 RFC + RXS-0174~0176 定型;**二进制面**(descriptor
  编码、采样 opcode 布局、register 数值、SPIR-V Location)**非 stable**,随 RD-008(G2.5 语言 1.0 候选触发点)评估。
- **provenance**:本 RFC 实质内容 `Assisted-by: claude-code:claude-opus-4.8`;agent 自主决策、批准、推进下游实现 PR;
  所有 device 数字来自真实命令输出(硬规则 3),无 GPU 即标 blocked,不伪造。

## 11. 规范与实现依据

- **决策/规范**:06 §4.2(内存模型映射禁区,纹理路径扩展点,🔒)· 06 §8.2 第 6 点(`Texture2D<F>` + sampler
  类型;tex proxy 内存模型条款扩展)· 13 §D-131 v1.4(图形=B 链)· 04 P-01(strict-only)/ P-11(单一事实源)/
  P-13(防 AI 幻觉治理)· 11 §5(G2 = UC-04 deferred 渲染器)· 10 §3(变更三档,Full RFC)/ AGENTS 硬规则 5/7。
- **关联 RFC**:RFC-0002(着色阶段 + `Texture2D<F>`/`Sampler` 类型面 RXS-0156)· RFC-0004(SPIR-V→DXIL 图形=B 链)·
  RFC-0005(绑定布局推导 RTS0)· RFC-0006(UC-04 deferred 渲染器,§4.5(a)/§9 Q-Texture 标 RD-021 禁区,本 RFC 兑现)。
- **实现锚点**:`src/rurixc/src/dxil_spirv.rs`(`lower_resource_sample` / 采样 opcode)· `dxil_codegen.rs`(B 链)·
  `binding_layout.rs`(per-class SRV/Sampler)· `src/uc04-demo/shim/uc04_offscreen.cpp`(RT→SRV + descriptor table + 真采样)·
  `conformance/dxil/graphics/accept/uc04_lighting_fs.rx`(fragment 真采样)· `ci/dxil_uc04_device_smoke.py`(数据流红绿)。

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-30 | 初版 Full RFC:纹理采样内存模型语义本体(§4.3~§4.7 06 §4.2 🔒 禁区落笔)+ 首期收敛子集(显式 LOD 0)+ §5 RXS-0174~0176 条款投影 + §8 RD-022~RD-024 登记 + §9 agent 自主裁决 Q-Syntax/Q-LOD/Q-Subset/Q-Stage/Q-ErrCode/Q-Defer/Q-MemModel。废止 G2_CONTRACT §8.5 选项 B「不采样」折中、关闭 RD-021。Agent Approved 2026-06-30(完全自主,硬规则 1);`Assisted-by: claude-code:claude-opus-4.8`。 | Full RFC |
| v1.1 | 2026-06-30 | §6 device 见证 CI run URL 回填:self-hosted `rurix-dev-4070ti`(RTX 4070 Ti)pr-smoke step 48 全绿 run https://github.com/qwasg/Rurix/actions/runs/28442661542(PR #115);随附修复 driver 着色阶段类型面前移至 typeck 前(恢复句柄返回位 RX3013,RXS-0156;commit c0e8730)——采样类型面回归致 `-> Texture2D<F>` 误触 RX2001 掩盖 RX3013,前移裁决次序后 CI 步 28 着色阶段类型面 + 步 48 device smoke 全绿。`Assisted-by: claude-opus-4.8`。 | Full RFC（provenance 回填） |
