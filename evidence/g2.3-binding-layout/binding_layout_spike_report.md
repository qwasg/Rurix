# G2.3 绑定布局推导可行性 spike 报告（round-1）

> 分支：`spike/g2.3-binding-layout`（隔离取证，不入 `src/` 生产路径，spike 结束可弃）。
> 探针：`spike/g2.3-binding-layout/probe_binding_layout.py` + `corpus/*.hlsl`。
> 证据：`evidence/g2.3-binding-layout/binding_layout_spike_20260627.json`（schema `g2.3-binding-layout-spike/v1`，命令真实输出）。
> 承接：RFC-0005「绑定布局推导」的 Design 与 spike 留痕（Draft / Awaiting Agent）。
> 诚实纪律（对齐 RD-010 dxil-path spike）：**measured 与 assumed 严格分栏**；工具缺失为 SKIP 不伪造；**不声称未验证的推导路径「已打通」**。

---

## 0. 一句话结论

在现有图形=B 链（MIR→SPIR-V→spirv-cross→HLSL→dxc→DXIL，RFC-0004）上，**从 SPIR-V `DescriptorSet`/`Binding` 装饰到 HLSL `register(x#, spaceN)` 的映射是确定性、规则可预测的（measured）**；但 **D3D12 root signature（容器 `RTS0` part）不由工具链从资源使用自动合成（measured）**——dxc 只在显式 `[RootSignature]` 时才把 root signature 序列化进容器，否则容器仅含 `PSV0` 资源绑定反射。**故「绑定布局推导」（资源使用 → root signature 形态）本质是 Rurix 编译器侧的推导职责，工具链只提供反射与序列化承接，不替代推导**。同时一个**结构前提**为 assumed 未验证：Rurix 自有 MIR→SPIR-V 编码器当前**结构上无法表达资源绑定**（`io_sig`/`MirIoType` 仅标量/向量，无资源种类），故「按 io_sig 顺序确定性 emit 资源绑定装饰」是 RFC-0005 实现侧待建面，本 spike **不实测**。

---

## 1. 工具链版本（measured，隔离不入库）

| 工具 | 角色 | 版本（命令真实输出） |
|---|---|---|
| dxc（signed pin） | HLSL→DXIL + `-dumpbin` | `dxcompiler.dll: 1.9(5191-d355aa83)(1.9.2602.24) - 1.9.2602.24 (d355aa836)`（`H:\dxc-round7\extracted\bin\x64\dxc.exe`） |
| spirv-cross | SPIR-V→HLSL | `vulkan-sdk-1.3.290.0-44-g65d73934`（2024-10-04） |
| dxc（Vulkan SDK） | HLSL→SPIR-V producer | `1.8 - 1.8.0.4739 (d9a5e97d0)` |
| spirv-dis | SPIR-V 反汇编 | Vulkan SDK 1.3.296.0 Bin |

> producer 用 Vulkan SDK dxc（`-spirv`）仅为取证产代表性 SPIR-V；生产链中该段由 Rurix 自有 `dxil_spirv::emit_spirv` 承担（见 §4 assumed）。

---

## 2. 语料（RXS-0156 资源面 + cbuffer/structured buffer）

| 语料 | profile | 资源面 | 用途 |
|---|---|---|---|
| `ps_textured` | ps_6_0 | `Texture2D<float4>` + `SamplerState` | RXS-0156 核心最小绑定面 |
| `ps_mixed` | ps_6_0 | cbuffer + 2×Texture + 2×Sampler | Q-Space 混合种类打包实测 |
| `cs_structured` | cs_6_0 | cbuffer + `StructuredBuffer`(SRV) + `RWStructuredBuffer`(UAV) | Q-RootShape SRV/UAV/CBV 三类 |
| `ps_rootsig` | ps_6_0 | 同 `ps_mixed` + 显式 `[RootSignature]` | Q-RootShape root signature 序列化对照 |

语料用 `[[vk::binding(b, set)]]` 显式控制 SPIR-V `(binding, set)`，**模拟** Rurix 按 io_sig 顺序确定性分配（该模拟本身属 assumed，见 §4）。

---

## 3. Measured 事实（命令真实输出，4/4 语料 chain 全通）

### 3.1 Q-Space：SPIR-V (set,binding) → HLSL register(x#, spaceN) 映射规则（measured）

spirv-cross 默认（无 `--hlsl-auto-binding`）从 SPIR-V 装饰派生 HLSL 寄存器，**实测规则**：

- **descriptor set → space**：SPIR-V `DescriptorSet N` → HLSL `spaceN`（4/4 语料 set 0 → space0）。
- **binding number → register index**：SPIR-V `Binding K` → HLSL 寄存器**索引 = K 本身**（**非按种类各自从 0 计数**）。
- **register class 由资源种类定**：CBV→`b`、SRV(`Texture2D`/`StructuredBuffer`)→`t`、Sampler→`s`、UAV(`RWStructuredBuffer`)→`u`。

`ps_mixed`（SPIR-V binding 全局连号 0..4）→ HLSL：

| 资源 | 种类 | SPIR-V (set,binding) | HLSL register（measured） |
|---|---|---|---|
| Globals(cbuffer) | CBV | (0,0) | `b0, space0` |
| albedo | SRV | (0,1) | `t1, space0` |
| normal | SRV | (0,2) | `t2, space0` |
| samp_linear | Sampler | (0,3) | `s3, space0` |
| samp_point | Sampler | (0,4) | `s4, space0` |

**关键观测**：因 binding 全局连号，texture 落 `t1/t2`（非 `t0/t1`）、sampler 落 `s3/s4`（非 `s0/s1`）——即 **register 索引直接复用 binding 号，跨种类共享同一计数轴**。对照 `ps_rootsig`（手写 `register()` 按种类计数）→ `b0,t0,t1,s0,s1`。

→ **推论（measured 支撑，裁决留 agent）**：register/space 分配可由编译器**确定性导出**，但「索引轴」有两种自洽策略：(A) 单一全局 binding 轴（spirv-cross 默认，t/s 索引跨种类连号）；(B) 按种类各自从 0 计数（需 Rurix 在 emit SPIR-V binding 时按 (种类, 种类内序) 分配，或译后用 register 覆盖）。两者都确定性、都能产合规 DXIL。**策略选择 = Q-Space，落 §9，AI 不自填默认**。

`cs_structured` 实测：cbuffer→`b0`、StructuredBuffer(SRV)→`t1`、RWStructuredBuffer(UAV)→`u2`（同一全局 binding 轴，class 按种类分；measured）。

### 3.2 Q-RootShape：root signature 是否由工具链自动合成（measured）

二进制扫描 DXIL 容器 fourcc part（`probe_binding_layout.py::container_parts`）：

| 语料 | 容器 parts（measured） | `RTS0`(root sig) | `PSV0`(绑定反射) |
|---|---|---|---|
| `ps_textured` | DXIL,PSV0,ISG1,OSG1,STAT,SFI0,HASH | **无** | 有 |
| `ps_mixed` | DXIL,PSV0,ISG1,OSG1,STAT,SFI0,HASH | **无** | 有 |
| `cs_structured` | DXIL,PSV0,ISG1,OSG1,STAT,SFI0,HASH | **无** | 有 |
| `ps_rootsig` | DXIL,PSV0,**RTS0**,ISG1,OSG1,STAT,SFI0,HASH | **有** | 有 |

**关键观测**：

- **默认编译不产 `RTS0`**——dxc **不会**从资源使用自动合成 root signature 进容器（measured，3/3 默认语料）。
- 仅显式 `[RootSignature(...)]` 时 dxc 把 root signature 序列化为 `RTS0` part（measured，`ps_rootsig`）。
- 所有语料都产 `PSV0`，其 `PSVRuntimeInfo` 含资源绑定反射（register/space/种类，dumpbin 可读）——即**资源绑定信息工具链恒可反射**，但**root signature 形态（descriptor table vs root descriptor vs root constant / static vs dynamic sampler）须由上游给定**。

→ **推论（measured 支撑）**：「绑定布局推导」= 从资源使用推导 **root signature 形态** 并序列化为 `RTS0`，**是 Rurix 编译器侧职责**；工具链提供 (a) `PSV0` 资源绑定反射作交叉校验输入、(b) `[RootSignature]`/序列化 API 作 `RTS0` 承接，但**不替代推导决策**。root signature 形态策略 = Q-RootShape，落 §9。

### 3.3 确定性（measured）

4/4 语料：同输入二次编译，DXIL 容器 SHA256 **全等**（`deterministic: true`）。→ B 链对给定输入确定，可纳入 golden（对齐 RXS-0162 Property 3）。

---

## 4. Assumed / 未验证（严格分栏，不冒充 measured）

以下属 RFC-0005 实现侧待建面或待 agent 裁决，本 spike **明确不实测、不声称已打通**：

1. **Rurix 自有 MIR→SPIR-V emit 资源绑定装饰 = 结构上不可达（assumed，未实测）**：
   `dxil_spirv::emit_spirv` 当前只 emit `Location`/`BuiltIn`/`UserSemantic` 装饰；`IoSigKind`（Builtin/Interpolate/Varying）与 `MirIoType`（Scalar/Vector）**无资源种类**，无法表达 `Texture2D`/`Sampler`/cbuffer/structured buffer 句柄。本 spike 用 Vulkan dxc producer + `[[vk::binding]]` **模拟** Rurix 应 emit 的 SPIR-V，**不等于** Rurix 已能产此 SPIR-V。「按 io_sig 顺序确定性分配 (set,binding)」的 Rurix 侧实现属 RFC-0005 实现 PR（条款先于实现）。
2. **RXS-0156 句柄类型 → SPIR-V opaque 资源类型 + DescriptorSet/Binding 装饰的降级面**：RXS-0161 当前仅承诺 opaque 类型形态、纹理访问语义结构上不可达 → RX6013 升档（RFC-0004 §4.6(b)）。资源绑定降级面是 RFC-0005 待建，**未实测**。
3. **root signature 形态推导规则的正确性 / device 真跑**：本 spike 只证「工具链能序列化给定 root signature」与「不自动合成」；**Rurix 推导出的 root signature 是否与资源使用语义一致、能否被 D3D12 PSO 接受并 device 真跑出图，未验证**（属 G-G2-3 + UC-04/G2.4）。
4. **descriptor heap / bindless / unbounded array**：本 spike 语料不含 bindless，**未触及**（Q-Bindless 落 §9：本期 defer 还是 out_of_scope）。
5. **`register`/`space` 物理布局、descriptor table 偏移、root parameter DWORD 计数的二进制 ABI**：属 RFC-0004 §4.6(a) 同级 🔒 禁区（签名/绑定二进制 ABI 布局），本 spike **只观测工具链产出、不裁定 ABI**。

---

## 5. 可推导路径（measured 锚定的结论，供 RFC-0005 Design 引用）

```
RXS-0156 资源使用             [assumed: Rurix 侧待建，结构上当前不可达]
  (Texture2D<F>/Sampler/
   cbuffer/structured buffer)
        │  ① Rurix MIR→SPIR-V：按 io_sig 顺序 emit opaque 资源类型 + DescriptorSet/Binding 装饰
        ▼                       [Q-Space：(set,binding) 分配轴策略待 agent]
  SPIR-V (set,binding) 装饰     [measured：spirv-dis 可解析]
        │  ② spirv-cross：确定性映射 set→spaceN / binding→register index / 种类→t·s·b·u
        ▼                       [measured：规则可预测、确定性]
  HLSL register(x#, spaceN)     [measured：4/4 语料]
        │  ③ Rurix 推导 root signature 形态（descriptor table / root descriptor / root constant；
        │     static vs dynamic sampler）→ 经 dxc [RootSignature]/序列化 API 注入
        ▼                       [Q-RootShape / Q-Sampler：形态策略待 agent；推导=Rurix 职责（measured：工具链不自动合成）]
  dxc → DXIL 容器
    ├─ PSV0 资源绑定反射         [measured：恒产，作交叉校验输入]
    └─ RTS0 root signature       [measured：仅显式注入时产；默认不产]
```

---

## 6. 本轮边界（纯取证，未越界）

- 不落 codegen、不接线 `src/`、不创建 spec 条款体、不造错误码、不入 golden、不登 registry。
- 不裁 §9 任何路径抉择（Q-Space / Q-RootShape / Q-Sampler / Q-Bindless / Q-Gate / Q-Err / Q-File / Q-Inference-vs-Explicit）——均留 agent（RFC-0005 §9）。
- 触及 🔒 签名/绑定二进制 ABI 布局（RFC-0004 §4.6(a) 同级）、纹理路径内存模型映射（06 §4.2）只引边界声明，不落禁区语义本体。
- 常驻回归网未触动（纯文档/spike）。

## 7. 复现命令

```
set RURIX_DXC_DIR=H:\dxc-round7\extracted\bin\x64
set RURIX_SPIRV_CROSS=...\vulkan-1.3.296.0\Bin\spirv-cross.exe
set RURIX_SPIRV_DXC=...\vulkan-1.3.296.0\Bin\dxc.exe
set RURIX_SPIRV_DIS=...\vulkan-1.3.296.0\Bin\spirv-dis.exe
py -3 spike/g2.3-binding-layout/probe_binding_layout.py
```

工具缺失 → evidence `status: SKIP`（不伪造）。当前轮 `status: MEASURED`，4/4 语料 chain 全通、确定性全等。
