# Rurix 语言规范 — 绑定布局推导语义面（descriptor / root signature；G2.3 起）

> 条款:**RXS-0163 ~ RXS-0166 计划区间**(G2.3 绑定布局推导语义面:资源句柄 → SPIR-V 资源绑定降级 / register-space 分配推导 / root signature 形态推导 + RTS0 序列化 / 绑定布局一致性校验门 + strict-only 推导失败)。体例见 [README.md](README.md)。
> 依据:**[RFC-0005](../rfcs/0005-binding-layout-inference.md)**(绑定布局推导,owner Approved 2026-06-28);06 §8.2(descriptor / root signature 编译器推导,P-11 单一事实源);04 P-01(strict-only);04 P-11(host 绑定结构 ↔ shader 布局单一事实源);[RFC-0002](../rfcs/0002-shader-stages.md) RXS-0156(资源句柄类型面);[RFC-0004](../rfcs/0004-spirv-dxil-graphics-backend.md)(图形=B codegen 与禁区边界);[dxil_backend.md](dxil_backend.md) RXS-0157~0162(DXIL B 链)。授权:[../milestones/g2/G2_CONTRACT.md](../milestones/g2/G2_CONTRACT.md)(D-G2-3,G-G2-3)+ [../milestones/g2/G2_PLAN.md](../milestones/g2/G2_PLAN.md) G2.3 子里程碑。
> 档位:**Full RFC**(RFC-0005;10 §3:本设计触新 codegen 推导面,并触及签名/绑定二进制 ABI 布局、纹理路径内存模型映射、DXIL/SPIR-V UB 边界等硬规则 5 禁区边界)。RFC-0005 已由 owner(Language Lead)于 2026-06-28 批准并裁决 §9 全部路径项。**AI 无权自判 Direct**,判档以 RFC-0005 与 G2_CONTRACT 授权为据,判档争议向上取严。任何触及 **register/space/mask/packing/descriptor table 偏移/root parameter DWORD 物理布局** / **纹理路径内存模型映射(06 §4.2)** / **DXIL-SPIR-V UB 边界** 的条款,必须停下标注「需人工升档」,不在本文件自行落笔。**严禁 UB 节**(10 §7.5):不可推导 / 超上限 / register-layout 冲突 / PSV0 mismatch 以编译期 6xxx codegen 诊断(P-01 strict-only,无运行期 fallback)定义。
> 规范先行(AGENTS.md 硬规则第 7 条):**条款 PR 先于实现 PR**;缺条款的语义 PR 必须先补 spec。`ci/trace_matrix.py --check` 要求每条 `### RXS-####` 条款 >=1 测试锚定(`//@ spec: RXS-####`)。**本 PR-E1 spec 脚手架仅登记新文件名 + 计划区间 RXS-0163~0166,不落带编号裸条款头**——条款体与每条 >=1 测试锚定随 PR-E2 实现 PR 同落(条款 PR 先于实现 PR,trace_matrix 维持全锚定)。

---

## 1. 范围与编号区间

本文件承载 **绑定布局推导** 的语义条款(G2.3+,D-G2-3)。绑定布局推导把 RXS-0156 资源句柄类型面与 RFC-0004 图形=B codegen 链连接起来,由编译器从 shader 资源使用推导 D3D12 descriptor / root signature,兑现 P-11 单一事实源:host 绑定结构与 shader 布局不手维护两份、不静默漂移。

覆盖语义面(RFC-0005 §4 / §9):

- **资源句柄 → SPIR-V 资源绑定降级面**:RXS-0156 的 `Texture2D<F>` / `Sampler` / constant buffer / structured buffer 等资源使用降级为 SPIR-V opaque 资源类型与 `DescriptorSet`/`Binding` 装饰。当前 Rurix MIR→SPIR-V 资源绑定结构仍为待建面,不得冒充已实测。
- **register/space 分配推导**:§9 Q-Space 裁决为按资源种类分轴;首期默认单 set/`space0`,CBV/SRV/UAV/Sampler 分别走 `b/t/u/s` 轴并按声明序各自从 0 递增。多 space 与 `#[binding(...)]` 显式覆盖不进本期。
- **root signature 形态推导 + RTS0 序列化**:§9 Q-RootShape 裁决为 CBV root descriptor + SRV/UAV descriptor table + Sampler descriptor table;§9 Q-Sampler 裁决为 `Sampler` 默认 dynamic sampler。root constant 与 static sampler 后期独立判档。
- **一致性校验门 + strict-only 推导失败**:使用 PSV0 资源绑定反射与推导意图交叉校验;不可推导、超 root signature 64 DWORD 上限、register/layout 冲突、PSV0 mismatch → 6xxx codegen 诊断,无运行期 fallback。

明确不在本文件落语义本体的范围:

- **绑定二进制 ABI 布局禁区**:register/space/mask/packing、descriptor table 字节偏移、root parameter DWORD 物理布局、descriptor heap 编码均不冻结为 stable 语言保证。
- **纹理路径内存模型映射**:采样/load/store opcode、缓存一致性、LOD/导数、越界采样后果、memory-order 留独立 Full RFC。
- **bindless / unbounded descriptor array / descriptor heap 直索引**:本期 defer 至 RD-018;不登记 SG-010 gating,不永久/条件裁剪该方向。
- **PSO / resource state / barrier 运行时面与 UC-04 deferred renderer**:本文件仅覆盖绑定布局推导 spec 面,device 真跑出图归 G-G2-3 / G2.4 后续证据。

**编号区间**:本文件计划条款为 **RXS-0163 ~ RXS-0166**(全 spec 唯一、分配制递增、永不复用,见 [README.md](README.md) §1;当前最高现存 RXS-0162 @ [dxil_backend.md](dxil_backend.md))。本轮 **仅登记区间预留**,**不落带编号裸条款头**;条款体与每条 >=1 测试锚定随 PR-E2 同落。区间登记于 [README.md](README.md) §4 文件清单。

## 2. 条款计划映射（无条款体）

> 本节仅为 PR-E1 计划映射,零 `### RXS-####` 三级标题,trace_matrix 不计本节。带编号条款体随 PR-E2 实现 PR 同落。

| 条款（计划） | 标题 | 测试锚定计划 |
|---|---|---|
| RXS-0163 | 资源句柄 → SPIR-V 资源绑定降级面 | conformance accept（合法资源句柄 → 确定性绑定装饰）+ reject（不可映射 → RX6013）+ SPIR-V/golden |
| RXS-0164 | register/space 分配推导 | accept（按资源种类分轴确定性分配）+ reject（register/layout 冲突 → 6xxx）+ golden |
| RXS-0165 | root signature 形态推导 + RTS0 序列化 | accept（CBV root descriptor + SRV/UAV/Sampler descriptor table → RTS0 + validator）+ reject（超 64 DWORD 上限 → 6xxx）+ golden |
| RXS-0166 | 绑定布局推导一致性校验门 + strict-only 推导失败 | accept（PSV0 反射与推导意图一致）+ reject（篡改推导 / PSV0 mismatch → 6xxx 真实红绿）+ 确定性核对 |

## 3. 裁决摘要与实现门控

- **Feature gate**:复用 `dxil-backend`;不新增 `binding-layout` 子 gate。
- **错误码策略**:绑定布局推导失败归 6xxx codegen 段,按实现时 registry 实际最高空号续;本脚手架不预留、不预造错误码。不可映射资源复用 RX6013;超 64 DWORD、register/layout 冲突、PSV0 mismatch 等新真实可达类别新开码。
- **Bindless**:defer 至 RD-018;本期遇到 bindless / unbounded descriptor array / descriptor heap 直索引保持 deferred/out-of-scope,以 6xxx codegen 诊断显式拒绝或保持结构上不可达。
- **显式标注覆盖**:`#[binding(...)]` 不进本期。推导优先;覆盖能力后期独立判档。
- **PR 序**:PR-E1 仅本文 + [README.md](README.md) 文件清单/修订记录 + registry RD-018/RFC 记录;PR-E2 才落条款体、实现、golden、错误码与 device/validator 证据。

---

## 4. 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| v1.0 | 2026-06-28 | 新建 binding_layout.md（PR-E1 spec 脚手架，承 [RFC-0005](../rfcs/0005-binding-layout-inference.md)，owner Approved 2026-06-28）：登记文件名 + G2.3 绑定布局推导语义面说明 + **RXS-0163~0166 计划区间**（资源句柄→SPIR-V 资源绑定降级 / register-space 分配推导 / root signature 形态推导+RTS0 / 一致性校验门+strict-only）。**仅登记计划映射，不落带编号裸条款头**——条款体与每条 >=1 测试锚定随 PR-E2 同落，trace_matrix 维持全锚定。同步 owner 裁决摘要：Q-Space=B / Q-RootShape=B / Q-Sampler=B / Q-Bindless=A→RD-018 / Q-Gate=A / Q-Err=6xxx 续号策略 / Q-File=B / Q-Range=4 条 / Q-Inference-vs-Explicit=C。禁区不动：绑定二进制 ABI 布局 / 纹理路径内存模型 / DXIL-SPIR-V UB 边界只作边界声明。 | **Full RFC**（RFC-0005 / PR-E1） |
