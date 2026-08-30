# 默认翻转治理方案（DEFAULT FLIP PLAN）— 画质战役 Phase E1 交付

> **性质：只写方案不执行。** 本文档规划「窗口生产默认从 all-off 翻转为 `--quality full` 画质终态」的完整治理路径：受影响锚清单、翻转步骤、回滚方案。执行须另立会话获批后进行。
> 日期 2026-08-28；前置在案：九臂 + `--quality off|full` 预设（解析层展开，位级等价三跑证明）+ 全量回归绿（Stage A 18/18、soak、风暴，见 E_ACCEPTANCE_SUMMARY.json）。

## 0. 翻转范围界定

- **只翻窗口 presented 面**（`g31_window_present` 的默认臂字面）：`--quality` 默认 `off` → `full`。
- **bench Stage A 默认臂永不动**：`g14_3_pipeline_perf` 默认路径（18 格锚 c1d28ad7 系）是跨里程碑回归事实源，任何翻转不触碰；bench `--quality` 默认维持 `off`。
- 冻结面红线沿用：默认路径 kernel（g14_3_direct_gi/g16_gi_multibounce/g18_light_transport_depth）与共享 m_c SPV 0-byte。

## 1. 受影响锚清单

### 1.1 RD-045 P02 腿（跨门硬编码 presented 锚）

- `ci/g31_blocked_probes_smoke.py` L63：`RD045_ANCHOR_DIGEST = sha256:060e69a81e26dea4fce8be99d78c9a180fd3b76c8f6747ae548f44d10de28ff9`。
- device 腿消费面 = **`target/release/g31_window_present.exe`（旧二进制,非 target-night）** `--frames 64 --warmup 10 --hidden --auto-move orbit`——经旧构建 + 共享 m_c encode SPV 旧字节产出。
- 翻转影响：若旧二进制被重建（吸收 ACES 修复源码/新默认），060e69a8 必漂；若不重建，锚暂稳但源码-字节 divergence 持续。**处置 = 收编步骤 §2.3 后统一重收割该锚并改写 L63 字面（ci/** 修改须专项授权）。**

### 1.2 共享 SPV 源码-字节 divergence 三处（v2 隔离件收编步骤）

战役期间源码单一事实源已改、共享冻结字节未动（他会话零影响路线），主线收编时统一处置：

| # | 源码（已修/已演进） | 共享旧字节（消费面） | v2 隔离件（战役锚承载） | 收编步骤 |
|---|---|---|---|---|
| 1 | `kernels/g31_display_encode.rx`（ACES 1.3 样条 12 处 b1/b2 转置修复,A2b） | 共享 m_c `g31_display_encode.spv`（95,088B——g34_full_lane/g35_particle_lane 他会话 3 绑定消费 + RD-045 旧二进制） | `.tmp/night_0828/spv/g31_display_encode_v2.spv`（95,660B,G31_DEFAULT_SPV_ENCODE 已指向） | ①共享路径切 v2 字节（或重编共享件并 spirv-val）②g34/g35 消费面全门复跑（presented 锚重收割）③RD-045 锚重收割（§1.1）④encode_parity_probe.py 挂门防复发 |
| 2 | `kernels/g31_texture_gi.rx`（fx/fy 双线性 5 处修复,Phase B）+ svt 同源 | 旧 tex 形态字节（`ci/g31_texture_sampling_smoke.py` 现编面按旧形态判读） | `g31_texture_gi_v2.spv`/`g31_texture_probe_v2.spv`/`g31_svt_gi_fyfix.spv`/`g31_svt_probe_fyfix.spv`（heap 形态,探针步幅 3→4） | ①判读器同步 heap 形态（交接项在案）②旧 tex 臂锚 6fab598c 作废字面清理③g34 三 kernel 同源 fx/fy bug 一并修（HANDOVER §1） |
| 3 | `kernels/g31_texture_nrm_gi.rx`（现源含 GI2 段 686 行） | gi2-off 锚字节 = pre-C 编译 `g31_texture_nrm_gi.spv`（94,124B,sha fd22cb19,备份 `g31_texture_nrm_gi_pre_c.spv.bak` 在案） | `g31_texture_nrm_gi_gi2.spv`（124,744B,sha 75d08aec,--gi2 on 独载） | 二选一：**A**（推荐）保持双 SPV 路线为长期形态（off 恒载锚字节纪律已两相验证）；**B** 收编单 SPV = 接受重编字节 + 全窗口组合臂锚重收割（C 相 d89848b9 教训:gate=0 恒等不可依赖） |

### 1.3 作废 presented 锚清单（翻转后归档,不再消费）

- **夜巡旧锚系**（A2b ACES 修复重定基作废）：`5596a730`（off）/`e989c6ee`（dither）/`b02b08b5`/`12d5dc91`/`48353e86`/`2b6efac6`/`db7d48f7`。
- **A2/A2b 中间系**：`fd5ca68c`（AE v1）/`a4695558`（六臂 v1）→ `790809aa`/`f0c46b87`（A2b 重定基,仍为 pre-B 形态,B 后已被组合臂锚替代）。
- **D 终态重建作废**（E1 归因在案,e2_reanchor_registry.json）：`6bd3af63`（九臂）/`8b1c12f3`（七臂）→ 现值 `9e5f6300`/`d89848b9`。
- **E1 教训（治理律）**：窗口纹理合流臂 presented 锚 = **二进制绑定锚**——任何重建后整批重收割；跨重建可沿用面仅 all-off（`55e4a92d`）与 bench 面。

## 2. 翻转步骤（获批后按序执行）

1. **冻结窗口**：翻转会话独占 GPU + 源码面（g31_window_present.rs 单文件字面改 `let mut quality_full = false;` 的默认来源——加 `--quality off` 显式恢复路径不变）。
2. **字面翻转**：解析层默认 `quality_full = true`（显式 `--quality off` 可关——off 字面语义从「中性默认」升为「显式回退档」，帮助文案同步）；`--window-storm`/`--fault-probe` 等诊断臂与 full 默认的互斥矩阵复核（storm×textures 已由 E1 解除并验收——e4_storm_summary.json:rc=0/resize_eras=1/validation 静默;残余互斥 = fg/hzb/slab/svt,翻转后这些诊断臂须显式 `--quality off`）。
3. **收编三处 divergence**（§1.2 表——推荐顺序 #1 encode → #2 tex 判读器 → #3 维持双 SPV 路线 A）。
4. **锚整批重收割**：all-off（预期仍 55e4a92d,若 §2.3 收编 encode 则必漂须重收）/ full 默认新锚 / RD-045 060e69a8 替换值（旧二进制重建后 orbit 64+10 双跑）。
5. **全门复跑清单**：`ci/g31_blocked_probes_smoke.py`（P02 腿）/ `ci/g31_texture_sampling_smoke.py`（heap 判读器同步后）/ g31 五门窗口回归锚 / Stage A 18 格（**必须仍 18/18,bench 面零动作自证**）/ soak ≥1800s @新默认 / 风暴 ×3。
6. **登记**：G31_PLUS_COMMERCIAL_RENDERER_TODO.md 追加修订行 + 锚 registry 更新 + 本文档标记「已执行」。

## 3. 回滚方案

- **一级回滚（字面）**：默认字面 `full`→`off` 单行还原 + 重建 → all-off 锚 55e4a92d 即时复验（off 面非二进制敏感,可跨重建对锚）。
- **二级回滚（收编逆转）**：encode 共享路径回指 m_c 旧字节（v2 文件保留不删）；RD-045 锚字面还原 060e69a8（旧二进制若未覆盖则零动作）；判读器还原旧形态判读。
- **锚表回退事实源**：`e2_reanchor_registry.json`（E1 终态锚全表）+ 本文档 §1.3（历史作废谱系）——回滚后按谱系逐级对锚,禁止跨代混用。
- **不可逆面清单（回滚豁免）**：源码 kernel 修复（ACES/fx-fy/GI2 段）不回滚——bug 修复非默认翻转的一部分,回滚仅针对「默认档位与锚字面」。

## 4. 风险登记

- 翻转后帧时 = full 档口径（scene_gpu ×~2.9 + gi2 ×1.65 段 + bloom/AE ~0.3ms;窗口全栈实测 107-124fps 含回读税,A3 在案）——生产预算面需产品侧确认。
- AE resize 复位 ~12 帧半衰、α=0.02 收敛 ~50 帧（场景切换后 ~1s 完全收敛）——默认体验属预期行为,登记进产品说明。
- `--textures on` 与 --svt/--fg/--hzb/--slab/--cluster-lod/--wp-hlod 互斥集在默认 full 下变为「诊断臂须显式 --quality off」——CI 中消费这些臂的门全部要加 off 字面（§2.5 清单外补扫）。
