<!-- Assisted-by: Cursor Claude Fable 5（G17.1 D-409 对抗性评审轮次——与起草轮次隔离） -->
# RFC-0032 D-409 对抗性评审记录（第 1 轮，2026-08-24）

> **性质**：D-409 对抗性评审交付物（G17.1 治理批）。评审对象 = `rfcs/0032-d3d12-host-ngx-lane.md` v0.1。
> **Provenance 偏差如实登记**：评审者与起草者同模型同会话族；独立性 = 评审轮次隔离 + 独立重读事实源。效力自限：跨工具评审者可得时建议补一轮；留 M-c disposition / close-out 终审复核锚。与 RFC-0031/0030/0029 先例同族。

## 1. 事实源独立核对清单

| 事实源 | 核对结论 |
|---|---|
| `G15_CONTRACT.md` §8.7 NGX 执行路径取证 | NGXCubinVulkan 日志逐字 + `vkCreateCuModuleNVX` 双证 + UE NGXCubinD3D12 对拍在案——「纯 Vulkan DLSS 执行面在 NGX 内不存在」定论成立；v0.1 §1 转引无漂移 |
| `G15_CONTRACT.md` §8.7 税源分解 | in-stream ≈1.90ms（X2 边际中位）+ 提交固定 ≈0.10ms + scene_gpu ≈1.02ms ⇒ 地板 3.02ms；v0.1 §2.1 数字与留痕逐字一致 |
| `G15_P2_DECISIONS.md` §4 G15-MD-F1 行 | 承接锚②车道架构面字面 =「D3D12 宿主 NGX〔NGXCubinD3D12，UE 同款宿主；G14.11 FSR 臂 D3D12 反向共享驻留为工程先例〕」——v0.1 §4.1 L1 先例引用成立 |
| `G15_P2_DECISIONS.md` §1 G14-N12 行 | 「additive API 面，不触 trait/temporal 0-byte」承接锚字面——v0.1 L1 括注一致 |
| `milestones/g13/g13_vendor_sdk_registry.json` | FSR 臂 `integration_arm: d3d12` + 「DLSS 维持 Streamline Vulkan interop 臂（契约字面）」——单 device 化触 Vulkan 主腿契约字面成立（v0.1 §4.3 ①） |
| `registry/deferred.json` RD-034 | DXIL RT/mesh 腿 blocked-on-upstream；本 RFC D3D12 = NGX 宿主 API 面非 shader 编译面——v0.1 §2.4/§7 两面不混淆声明必要且成立 |
| `src/rurix-rt/src/vendor_upscale.rs` | UpscaleBackend/vendor 驻留车道现状面 = 生产默认；v0.1 §4.1 L4 opt-in + 默认 0-byte 成立 |
| `unsafe-audit/rurix-rt.md` | U 命名空间实测 next_free=59；v0.1 L3「按实测 U next_free 领取」字面成立（禁推测号） |
| G17_CONTRACT §4.2 M-c 行 ↔ 本 RFC | 「approved/no-go/defer 三态均合法终态 + no-go/defer 须留档可机器核验评估证据」双向一致 |

## 2. Findings（分级 + disposition）

| # | 级别 | finding 字面 | disposition（v0.2 落实） |
|---|---|---|---|
| F1 | high | §5 决策树①「预估 Rurix_ms ≤ UE_ms」的「预估」无算式锚——会被实施成拍脑袋预估（P-09 违例面） | §5 写死预估式：预估 Rurix_ms = M-a 终判格实测 Rurix_ms − M-b A/B 实测中位差值（各项逐一引用 evidence JSON 字段路径）；禁一切无字段引用的预估 |
| F2 | high | 宿主差「可分离收益上界」依赖 UE GPUTime 口径反推（含场景不可直接分解）——精确归因在 implement 前不可得，测算若以精确值口吻落档即造假面 | §5 ③ 写死「上界估算」字面 + 测算式必须标注口径限制（UE CSV GPUTime 对 CUDA 引擎工作的口径面 = G15 §8.7 归因三面之③，0-byte 转引） |
| F3 | med | §4.1 L2 同步税参照锚 = G14.11 FSR 臂，但 FSR 是 D3D12 DLL 反向消费 Vulkan 资源（方向相反），参照效力有限 | L2 已写「实施时以新鲜命令输出重测」；v0.2 补「参照锚方向性限制如实登记」字面 |
| F4 | med | 决策树①时序：M-c 在 M-d 终判之前执行，「终判预期达标」只是预估——若 M-d 实测翻转（预期达标实际未达），M-c 终态是否回翻无规定 | §5 补时序注：M-c 终态按当时输入定盘不回翻；M-d 翻转构成新事实时按只追加程序留档 G18+ 承接锚（重判条件字面），不 retroactive 改写 M-c evidence |
| F5 | low | 单 device 化 no-go 在治理波定盘，疑似越过「以实测为输入」纪律 | 不成立面如实登记：§4.3 no-go 依据 = 冻结面/契约字面/工程量级三项结构性事实（不依赖本期实测），结构性 no-go 合法；三项已逐一列出 |
| F6 | low | RD-034（DXIL 编译腿）与 D3D12 宿主 API 面同词根混淆风险 | §2.4/§7 两面不混淆声明在位；候选决策表 §2 RD-034 行同步登记 |

## 3. 评审结论

v0.1 与在树事实源零冲突（核对清单 9 项）。F1~F6 全部于 v0.2 修法批落实。建议翻 Agent Approved（决策程序 + 实现语义）的前置：主会话核对 RFC ↔ 契约 §4.2 M-c 行 ↔ MAP M-c 行三面一致；终态 disposition 保持开放待 G17.4 M-c 按 §5 决策树程序产出。

签署：白栀（依 10 §7 / P-13 / D-406 v3.0；D-409 评审轮次隔离；provenance 偏差见头注）。
