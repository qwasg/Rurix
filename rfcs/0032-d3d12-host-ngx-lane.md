<!-- Assisted-by: Cursor Claude Fable 5（G17.1 RFC-0032 起草 + v0.2 修法批） -->
# RFC-0032 — D3D12 宿主 NGX 车道（跨 device 同步面 / 单 device 化评估）

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0032（按 2026-08-24 实测 `registry/number_ledger.json` namespaces.RFC `next_free=32` 领取，非推测号） |
| 标题 | D3D12 宿主 NGX 车道评估与实现语义（NGXCubinD3D12 宿主形态 / 跨 device 资源桥接与同步 / 单 device 化评估 / 决策树终态程序） |
| 档位 | **Full RFC**（① 触车道架构面——G15-MD-F1 承接锚②字面「车道架构面立项触冻结面独立 Full RFC」；② 潜在触 FFI ABI 高敏面（D3D12 device/fence/resource FFI 声明）；③ 触 vendor interop 生产车道形态。判档向上取严，10 §3） |
| 状态 | **Agent Approved（决策程序 + 实现语义）**——D-409 第 1 轮 findings 全部 disposition（§9.1）；**终态 disposition = defer（G17.4 M-c 决策树分支③程序产出，2026-08-24；evidence/g17_m_c_d3d12_host_lane_disposition_20260824T091333Z.json；见 v0.3 修订行）** |
| 承接里程碑 | G17（G17.4 M-c 波；验收面 = `g17.p0.m_c.d3d12_host_lane_disposition`） |
| 关联条款 | 拟落 spec 条款号 **post-interlock actual-next-free allocation**（现快照 RXS next_free=408；禁推测号；仅当终态 = implement 时消费）。UpscaleBackend trait 签名 / temporal 底座 / RXS-0357 面 0-byte（触碰须另立独立 Full RFC） |
| 依据决策 | D-406 v3.0 · D-409 · P-09 · P-13 · 用户 2026-08-24「帮我一次性完成G17」· 用户 2026-08-19 可商用授权 · [G15_CONTRACT](../milestones/g15/G15_CONTRACT.md) §8.6/§8.7 |
| Provenance | `Assisted-by: Cursor Claude Fable 5（G17.1 治理波起草）` |
| Agent 批准 | **已批准（决策程序 + 实现语义，2026-08-24）**；终态 disposition 待 G17.4 M-c |
| 对抗性评审 | **已完成**（[rfc0032_adversarial_review.md](../milestones/g17/design/rfc0032_adversarial_review.md)） |

---

## 1. 摘要

G15plus-II 取证定论（G15_CONTRACT §8.7，0-byte 消费）：NGX 在 Vulkan 臂的 DLSS 执行 = **NGXCubinVulkan**（CUDA cubin kernels 经 `VK_NVX_binary_import` 注入命令流），纯 Vulkan（非 CUDA）DLSS 执行面在 NGX 内不存在；UE 参照臂同为 cubin 执行，宿主 API = **NGXCubinD3D12**。双臂黑盒差归因三面中，①宿主 API 差（D3D12 vs Vulkan 的 cubin 发射/同步路径）与 ②NGX 版本差（310.6.0 PaddedWindowNetwork vs 310.5.2 encoder 族）均为 G15-MD-F1 承接锚已命名重判触发面。

本 RFC 冻结 **D3D12 宿主 NGX 车道**的评估程序与（若实施）实现语义：

1. **评估先行、归因分离**：宿主 API 差的收益上界必须以 G17.3 M-b（NGX 310.6.0+ 在 Vulkan 宿主下的 in-stream 重测分解）为输入隔离测算——**版本差收益不得冒充宿主差收益**。
2. **决策树终态程序**（§5）：implement / no-go / defer 三态均合法，按实测输入程序产出，禁拍脑袋。
3. **实现语义（仅当 implement）**：跨 device 形态——Vulkan 场景管线 0-byte + 新增 D3D12 宿主 device 承载 NGX evaluate，资源经 OPAQUE_WIN32 external memory 桥接（G14.11 FSR 臂 D3D12 反向共享驻留为工程先例），同步经跨 API fence；`UpscaleBackend` trait 签名 0-byte（加性 backend 变体）。
4. **单 device 化评估结论**：全 D3D12 场景管线重写 = 触 RXS-0171 光栅冻结面 + Vulkan 主腿契约字面（G13 立项裁决）+ 全部 digest 锚重收割——**评估即 no-go**（本 RFC 定盘，非实现候选；理由 §4.3）。

Agent Approved ≠ 实现许可。终态 disposition 由 G17.4 M-c 波按 §5 决策树兑现并落 evidence。

## 2. 动机、范围与治理门

### 2.1 为什么需要 Full RFC

bistro-interior/t100/dlss_sr 单格 ratio 四轮 0.786~0.862 定盘未达 ×1.00（G15 §8.5~§8.7）。地板算术：NGX in-stream ≈1.90ms + 提交固定 ≈0.10ms + scene ≈1.02ms ⇒ GPU-only 地板 ≈3.02ms > G15 期通过线 ≈2.96ms 物理不可达。UE 臂同款网络在 D3D12 宿主 + 310.6.0 下全帧 GPUTime ≈2.27ms（含场景）——宿主/版本两面是仅存的车道级重判触发面。车道架构变更触 vendor interop 生产车道形态 + 潜在 FFI ABI 高敏面，MR 不承载。

### 2.2 双门

| 门 | 允许 | 禁止 |
|---|---|---|
| G17.1 governance | 本 RFC / D-409 / MAP M-c 行 materialize | 改 src（治理期 0-byte） |
| G17.4 M-c | 按 §5 决策树兑现终态；implement 时按 §4 实现 | 跳过 M-a/M-b 输入直接实现；no-go/defer 不留机器可核证据 |

### 2.3 终态开放 ≠ 承诺（P-09）

implement/no-go/defer 三态均合法终态；不预设结论、不为「实现了才算完成」的叙事偏好扭曲判定。达标不可能时维持未达标登记（G17 契约 guardrail 字面）。

### 2.4 范围 / 非范围

**in**：D3D12 宿主 NGX evaluate 车道评估 + （条件）实现；跨 device 资源桥接/同步语义；单 device 化评估结论留档。

**out**：FG/MFG（独立层另判 0-byte）；DXIL 编译腿（RD-034 blocked 维持——本 RFC 的 D3D12 = NGX 宿主 API 面，非 shader 编译面，两面不混淆）；改坐标尺度；UpscaleBackend trait 签名/temporal 底座/RXS-0357 触碰；`--gi off/on` 车道；G13/G15/G16 冻结面回写。

## 3. 术语

- **cubin 宿主 API**：NGX 把 CUDA cubin kernels 注入哪个图形 API 的命令流（NGXCubinVulkan = `vkCreateCuModuleNVX`；NGXCubinD3D12 = D3D12 等价面）。
- **跨 device 形态**：场景渲染留 Vulkan device，NGX evaluate 迁往独立 D3D12 device，输入/输出资源跨 API 共享。
- **单 device 化**：场景渲染整体迁 D3D12（评估即 no-go，§4.3）。

## 4. 拟议语义（仅当 §5 决策树输出 implement 时生效）

### 4.1 跨 device 车道（L1~L4）

**L1 资源桥接**：color（RGBA16F）/ depth（R32F）/ mv（RG32F）/ output 四资源以 `VK_KHR_external_memory_win32`（OPAQUE_WIN32 或 D3D12_RESOURCE handle 形态）导出，D3D12 侧 `OpenSharedHandle` 导入；G14.11 FSR 臂 D3D12 反向共享驻留先例形态复用（additive API 面，不触 trait/temporal 0-byte——G14-N12 承接锚字面）。

**L2 同步语义**：Vulkan 渲染完成 → 跨 API timeline fence（`VK_KHR_external_semaphore_win32` ↔ `ID3D12Fence` 共享）→ D3D12 队列执行 NGX evaluate → fence 回签 → Vulkan 消费输出。同步税预算：2×fence 信号 + 队列提交 ≈0.1~0.3ms（以 G14.11 FSR 臂实测面为参照锚——**参照锚方向性限制如实登记（F3）**：FSR 臂 = D3D12 DLL 反向消费 Vulkan 资源，方向与本车道相反，参照效力有限，实施时以新鲜命令输出重测，禁以参照锚数字直接落档）。

**L3 FFI/unsafe 纪律**：D3D12 device/queue/fence/resource FFI 声明 = 手写 Rust repr(C) 声明面（对齐 sys.rs/vk.rs 零外部依赖纪律）；每个 unsafe 块 `// SAFETY:` + unsafe-audit 注册条目（按实测 U next_free 领取）+ 单块单操作。

**L4 车道选择与回退**：D3D12 宿主车道 = 环境变量显式 opt-in（生产默认维持既有 Vulkan interop 车道 0-byte）；A/B 门禁 = L0 位级探针（输出 digest 与锚对照，DLSS 网络输出允许与 Vulkan 宿主位面不同——以画质锚带复核替代位级门禁，超带即弃）+ 全协议复跑收益 measured。

### 4.2 归因分离纪律

宿主差收益 = （D3D12 宿主 + 同版本 NGX）vs（Vulkan 宿主 + 同版本 NGX）单变量对照；禁以「版本差 + 宿主差」混合收益冒充宿主差收益。M-b 的 310.6.0 Vulkan 宿主重测分解为本对照的必要输入。

### 4.3 单 device 化评估结论（本 RFC 定盘 no-go）

全 D3D12 场景管线重写触：① Vulkan 主腿契约字面（G13 立项裁决 10 vendor interop 臂 = vulkan_interop）；② RXS-0171 光栅冻结面 + 全部 Stage A digest 锚重收割同型程序（G14.11 已评估未立项面同型代价）；③ 工程量级 = 全渲染后端重写（非单格收口的成比例手段）。**评估结论 = no-go 留档**（重判条件 = G18+ 独立立项窗全后端演进评估；兜底 = Vulkan 主腿维持 0-byte）。

## 5. 决策树终态程序（G17.4 M-c 执行，输入 = M-a/M-b 实测 evidence）

```text
输入：M-a 暖态重标定后终判差距 Δ = Rurix_ms − UE_ms（bistro/t100/dlss_sr，逐项引 evidence 字段）
      M-b NGX 310.6.0+ Vulkan 宿主 in-stream 重测分解与 A/B 实测（fresh_in_stream_ms / ab_delta_ms）
① 若 M-b 后预估达标：预估 Rurix_ms = M-a 终判格实测 Rurix_ms − M-b A/B 实测中位差值
   （F1：预估式写死，各项逐一引用 evidence JSON 字段路径，禁一切无字段引用的预估）
   且 预估 Rurix_ms ≤ UE_ms
   → D3D12 宿主车道 defer（触发条件 = 后续窗复测未达标 + 宿主差 measured 主因证据；
      兜底 = Vulkan interop 车道生产默认维持）——避免为已达标格引入跨 API 车道复杂度。
② 若 M-b 后仍未达标 ∧ 宿主差可分离收益【上界估算】（UE GPUTime 口径内 NGX 份额 vs
   fresh_in_stream_ms 差——F2：必须标注口径限制 = UE CSV GPUTime 对 CUDA 引擎工作的
   口径面〔G15 §8.7 归因三面之③字面 0-byte〕，精确归因 implement 前不可得）
   > 残余差距 Δ' ∧ 同步税预算（§4.1 L2）不吞噬收益
   → implement（按 §4 实现，A/B measured 定采纳）。
③ 若宿主差可分离收益上界估算 ≤ Δ' 或同步税预算吞噬收益或 310.6.0+ 不可获得致归因无法分离
   → no-go 或 defer（附可机器核验测算式：各项数字逐一引用 evidence JSON 字段路径 + 上界
      口径限制标注），维持未达标登记不冒充。
```

**时序注（F4）**：M-c 终态按当时输入（M-a/M-b evidence）定盘不回翻；M-d 终判若与 ① 预估翻转（预期达标实际未达）构成新事实时，按只追加程序留档 G18+ 承接锚（重判条件字面），不 retroactive 改写 M-c evidence。

终态字面（implement / no-go / defer + 测算式 + evidence 字段引用）入 `evidence/g17_m_c_d3d12_host_lane_disposition_*.json` 与本表 §9 修订记录。

## 6. RED 臂

- M-c 未消费 M-a/M-b evidence 直接产终态 → RED
- 版本差收益冒充宿主差收益（归因混淆）→ RED
- no-go/defer 无机器可核测算式留档 → RED
- implement 态触 UpscaleBackend trait 签名/temporal 底座/RXS-0357 → RED（须另立 Full RFC）
- implement 态 unsafe 块无 SAFETY 注释/无 unsafe-audit 注册条目 → RED

## 7. 兼容

G5~G16 closed 判据 0-byte。既有门 `--verify-latest`。旧脚本禁 `--gate`。RD 八条 open 维持（RD-034 DXIL 腿与本 RFC D3D12 宿主面两面不混淆字面登记）。生产默认车道（Vulkan interop）在 implement 未采纳前 0-byte。

## 8. 测试与验收

M-c 独立 evidence（`g17.p0.m_c.d3d12_host_lane_disposition`）；implement 态另附 A/B measured + 画质锚带复核 + `--selftest` 红绿臂；no-go/defer 态附测算式 evidence 字段引用链。

## 9. 修订与评审

### 9.1 D-409

评审全文见 `milestones/g17/design/rfc0032_adversarial_review.md`。F1~F6 已落入 §4.2、§5、§6、§2.4、§4.3、§7。单模型会话族 provenance 偏差如实登记，留 M-c/close-out 复核锚。

| 版本 | 日期 | 变更 |
|---|---|---|
| v0.1 | 2026-08-24 | 起草 |
| v0.2 | 2026-08-24 | D-409 修法批；翻 Agent Approved（决策程序 + 实现语义）；终态 disposition 待 G17.4 M-c |
| v0.3 | 2026-08-24 | **终态 disposition = defer**（G17.4 M-c §5 决策树分支③程序产出）：est_rurix = M-a 窗 Rurix 中位 3.771125ms − M-b 采纳差值 0（拒绝换版态 ab_delta 零混入，F1）> ue_med 3.1922ms（Δ'=+0.5789ms 未达标预估）∧ 宿主差可分离收益上界估算不可紧化（UE 侧 NGX 份额 CSV GPUTime 口径不可分解 = G15 §8.7 归因三面之③；F2 口径限制）∧ 同步税预算下界 0.1ms 与 Δ' 同量级净收益判定不可得 → defer + 测算式留档（evidence/g17_m_c_d3d12_host_lane_disposition_20260824T091333Z.json 全字段引用链）。重判条件 = G18+ 宿主差可分离 measured 证据出现（NGX 分解 profiling 或 UE 侧插桩）；兜底 = Vulkan interop 车道生产默认维持。§4.3 单 device 化结构性 no-go 维持。M-d 翻转构成新事实时按只追加程序留档 G18+ 承接锚不回翻本终态（F4）。 |
