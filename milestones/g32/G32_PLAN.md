<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 B 验收门 Task B8） -->
# G32_PLAN — 画面完整期执行计划（波 B 批次范围）

> 事实源 = [G32_CONTRACT.md](G32_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 期定位

G32 = **画面完整期**（G31+ 待办总表 §6 波次线 2）：把"接上屏幕的引擎"补成"游戏画面"——HZB 遮挡剔除/ReSTIR/slab 材质侧表/纹理采样/蒙皮骨骼动画五大特性生产接线 + GI 默认档 measured 决策 + OIT/半透明评估窗 + 特性组合矩阵核验 + 含动态角色/贴图材质/GI 的"游戏画面"demo 定版；验收 = 组合矩阵 + demo + 零降级三面 + 守卫套件 + RD-045 观察窗复核。上游法定输入 = `milestones/g30/g30_campaign_handover_registry.json`（RFC-0047 §5.5 唯一法定输入面）+ `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` §1.2 #6~#13 + §1.3 #10 + §6 波次线 2 + `registry/deferred.json`（RD-041 蒙皮 WPO MV 触发面/RD-045 确定性观察面）。上一期 G31 波 A 已验收（§8 close-out 实测 facts 在案）。

## 2. 波次

| 波次 | 内容 | 门 | 状态 |
|---|---|---|---|
| 波 B（本波） | B1 HZB + B2 ReSTIR + B3 slab + B4 纹理 + B5 蒙皮 + B6 GI 默认档决策 + B7 OIT 评估窗 + B8 验收门 | 五硬门 + 两评估窗 + 验收六面 | 已实现并验收（G32_CONTRACT §8 close-out 实测 facts） |
| 后续（期内候选/期外） | #11 BistroExterior（G10-N6 锚挂起维持：FBX2glTF 上游修复或替代臂 + 源资产同窗齐备）；GI 默认档重判（B6 re_trigger 两条件）；#14 DLSS 焦点格两半锚 / #15 RD-045 三件同窗攻 | 后续波立项程序 | 未立项 |

## 3. 波 B 实现面（B1~B7 交付物，全绿/如实）

- **B1 HZB 生产接线**：bistro 逐 mesh 节点 BLAS 分解 + 双 TLAS + 帧内金字塔轮换 + 误剔/出新闭环重渲；g27_hzb_reduce/g27_hzb_test 0-byte 冻结消费；门 `g31.waveB.hzb` PASS。
- **B2 ReSTIR 生产接线**：G28 device reservoir kernel 接契约灯表面（bistro point_lights 4 灯 20000 trial 双臂）；门 `g31.waveB.restir` PASS。
- **B3 slab 材质侧表生产接线**：16 槽资产驱动侧表 + g29_slab.rx 0-byte device 求值 vs host 金标准 + 逐三角 albedo 预调制；门 `g31.waveB.slab` PASS。
- **B4 纹理采样管线进生产场景**：top-12 律法映射 + BC1/BC3 真实解码 + 图集/四 SSBO 侧表 + 纹理变体 kernel；门 `g31.waveB.texture` PASS。
- **B5 蒙皮/骨骼动画进生产帧**：device LBS 蒙皮 + BLAS 逐帧 refit 桥 + 类 3 蒙皮 MV 进 TSR 历史链（RD-041 兑现窗）；门 `g31.waveB.skinning` PASS。
- **B6 GI 默认档决策**：measured 权衡窗 → maintain_default_off（milestones/g31/g31_gi_default_tier_decision.json）。
- **B7 OIT/半透明评估窗**：not_triggered（milestones/g31/g31_oit_evaluation_window.json）。

## 4. 波 B 验收门（B8，本波）

1. 组合矩阵核验：可组合臂真窗口真跑（双跑 digest 确定性 + real_render/present 帧率 measured）+ 互斥组合 fail-closed 拒绝核验（零冒充可组合）→ milestones/g31/g31_waveb_combo_matrix.json。
2. 游戏画面 demo 定版：最优组合臂真窗口 ≥200 帧真跑，帧率双口径 evidence。
3. 零降级回归三面：Stage A digest 锚 18/18 canonical 重跑零漂移（ci/g31_wave_a_anchor_check.py 范式）+ G16plus M-g 18/18 canonical 复跑 VERDICT=PASS + G17-MD-F1 焦点格新鲜真跑诚实红不恶化（ratio ≥ 在案 0.960479）。
4. 守卫套件五条全跑：check_structure/check_schemas/check_number_ledger/trace_matrix --check/budget_eval。
5. RD-045 观察窗复核：波 B 各臂 digest 锚零漂移 + 三件盘点 0/3 维持不冒充（只追加登记）。
6. G32 四件套落盘（本文件族）+ G32_CONTRACT §8 close-out 实测 facts。

## 5. 编号纪律

CI 数字步骤零消费（波 B 七门 + B8 验收面均未占号；registry/number_ledger.json CI_step.next_free=525 维持）；RFC/RXS/RD/U/SG/MR/D/RX_error 共享段零消费（波 B = 既有语义面与 G26~G29 冻结 kernel 0-byte 消费，零新语义面）；evidence 前缀 `g31_hzb_wiring_`/`g31_restir_wiring_`/`g31_slab_wiring_`(+`_gate_`)/`g31_texture_sampling_`(+`_gate_`)/`g31_skinning_wiring_` 均经 check_schemas 三处纯追加登记（B1~B5 各批在案）；B6/B7/B8 = 评估/验收登记面（milestones/g31/ 下只追加 JSON），无 ci 脚本、无 evidence schema、check_schemas 零消费（g32_baseline_ 快检件同 g31_baseline_ 律跳过路由一处纯追加）。
