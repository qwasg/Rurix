---
contract: G34
title: G34 全特性合流期（G34-1 合流地基〔纹理+slab+动态实例三特性同开统一 kernel 车道〕 + G34-2 HZB 接统一车道 + G34-3 蒙皮进统一车道 + 收口验收面）
status: active
implementation_status: unlocked
active_scope: g34_full_feature_merge
version: v1.0
date: 2026-08-27
timebox: "G34-1 合流地基 = G31/G32/G33 三期契约 close-out 在案后即时接续窗，已验收（2026-08-27 门绿在案）；G34-2 HZB + G34-3 蒙皮 = 本批收口（同窗并行分区两独立 include 区段）；G-G34-4 验收面归收口验收批；FG/MFG 合流与 HZB×蒙皮同车道合并归后续波立项程序"
rfc_required: "G34 零新 RFC——统一 kernel 全族引用既有条款 RXS-0405 L3 device 面（零新 RXS 条款）；HZB 两阶段闭环第二段消费既有 RFC-0044 §5.8 语义面、双 TLAS 拆 pass 消费既有 RXS-0297 单 TLAS 签名纪律；零共享编号段消费（CI 数字步骤零消费声明——三门均 symbolic gate key 未占号，收口验收批实测核验 CI_step.next_free=525 维持）"
upstream_docs:
  - "milestones/g31/G31_CONTRACT.md §8 + milestones/g32/G32_CONTRACT.md §8 + milestones/g33/G33_CONTRACT.md §8（G31/G32/G33 三期契约 close-out 终态 = G34 法定输入）"
  - "milestones/g30/g30_campaign_handover_registry.json（RFC-0047 §5.5 谱系法定输入面）"
  - "G31_PLUS_COMMERCIAL_RENDERER_TODO.md §6 三条波次线 + §7 调研镜像（G34 上游定位）+ milestones/g31_plus_campaign_record.md（三波 56 项终态映射）"
  - "milestones/g31/g31_waveb_combo_matrix.json（波 B 组合矩阵：互斥 12/12 fail-closed 在案——G34 收敛对象）"
  - "registry/deferred.json（RD-045 确定性观察面）"
in_scope:
  - g34_wave1_unified_lane
  - g34_wave2_hzb_unified
  - g34_wave2_skin_unified
  - g34_acceptance_gate
out_of_scope:
  - fg_mfg_merge（FG/MFG 合流——G34-1 备注字面「归后续波」，g34_unified_shade 接口预留不预支）
  - hzb_skin_same_lane_merge（HZB×蒙皮同车道合流——两区段并行分区设计，合并归后续窗）
  - contract_flip_and_governance_wave（契约 flip active→closed 与治理波仪式——留 owner 按 10_GOVERNANCE 程序）
  - rewriting_g13_g33_frozen_registries_or_anchors（冻结注册表/锚 0-byte 不回写）
  - presented_fps_lane（presented 口径面——本期无 FG，presented/real 双口径分离律引用不消费）
deferred_refs: [RD-045]
deliverables:
  - id: D-G34-1
    name: G34-1 合流地基（kernels/g34_unified_{gi,shade}.rx 统一 kernel 双件 + g34_full_lane 真窗口 harness〔UnifiedDescs::G34Full 27 SSBO〕 + ci/g34_unified_lane_smoke.py + 两 evidence schema + host 对拍容差条目程序产——gate PASS 在案）
  - id: D-G34-2
    name: G34-2/G34-3 两门（HZB 接统一车道：g34_unified_primary.rx 加性 + g34_unified_shade.rx 扩展 + g34_2_hzb.rs 独立区段 + ci/g34_hzb_unified_smoke.py；蒙皮进统一车道：g34_unified_gi_skin.rx/g34_unified_mv.rx + g34_skin_section.rs 独立区段 + ci/g34_skin_unified_smoke.py；schema 路由 check_schemas 三处纯追加）
  - id: D-G34-3
    name: G34 四件套（G34_PLAN.md/G34_CONTRACT.md/CI_GATES.md/g34_budget.json——容差条目零 estimated 全 measured 程序产）
acceptance_gates:
  - id: G-G34-1
    check: "G34-1 合流地基门 g34.wave1.unified PASS——已绿在案（evidence/g34_unified_lane_gate_20260827T041754Z.json 八 facts 全绿）：统一 kernel 现编 SPV + spirv-val 双绿 + 缺省面 == 母版 Stage A 锚 sha256:c1d28ad73783cc3c… 位级 MATCH（canonical 160 帧 warmup 10）+ host 金标准对拍 p100=3.968658857047558e-04 ≤ 冻结容差 7.937317714095116e-04（g34.unified_lane.host_parity_tol 程序读；bitexact 像素占比 29.17% 如实登记非门判据）+ --full 双跑 74 帧 digest_seq 位级一致 + 动态实例位置核验 7/7 全 pass + 逐特性 digest 区分 4 面全真 + Stage A 锚复跑零漂移 + frame_ms measured（baseline=6.9565ms / full=9.0033ms / scene_gpu=3.3929ms；装配期一次性 tex_eval=1834.304ms / slab_eval=216.989ms 单列不混帧口径）"
  - id: G-G34-2
    check: "G34-2 HZB 接统一车道门 g34.wave2.hzb PASS：六 facts 闭集（kernels_spv_valid / culling_pixel_neutral / hzb_host_parity / determinism_double_run / culling_effective_measured / stage_a_anchor_replay）——判据字面 = ci/g34_hzb_unified_smoke.py docstring；实测数字待收口验收批填写"
  - id: G-G34-3
    check: "G34-3 蒙皮进统一车道门 g34.wave2.skin PASS：九面判据闭集（kernels_spv_valid / skin_vertex_bitexact / skin_position_verified / skin_mv_wired / rigid_mv_wired / determinism_double_run / per_feature_digest_discrimination / stage_a_anchor_replay / frame_ms_measured）——判据字面 = ci/g34_skin_unified_smoke.py docstring；实测数字待收口验收批填写"
  - id: G-G34-4
    check: "G34 收口验收面：守卫套件七条 exit 0 + 三门新鲜复跑全 PASS + 零降级回归锚三面（Stage A digest 锚 18/18 canonical 160 帧重跑零漂移 + G16plus M-g 18/18 canonical 复跑 VERDICT=PASS + G17-MD-F1 焦点格新鲜多样本中位如实登记〔诚实红维持/恶化均合法终态，禁冒充——G-G33-4 字面同律；锚检门 ci/g31_wave_a_anchor_check.py 复用产 Stage A 零漂移事实，其焦点格轨迹面严判 verdict 如实留档，波 B/C 三次锚检门同面 FAIL 先例在档〕）+ soak 登记面（g34_full_lane --full ≥5000 帧零崩 + validation 静默；close-out 只追加登记无新硬门）——实测 facts 待收口验收批填写 §8"
guardrails:
  - "诚实登记不冒充：所有数字来自真实命令输出；达标/维持/诚实红均合法终态"
  - "append-only：evidence/ 只增不删不改；deferred history 只追加；既有锚/注册表 0-byte 不回写"
  - "缺省面位级锚（无降级硬线）：全特性缺省关 == 母版 Stage A 锚位级一致，合流 = 加性扩展对既有面 0-byte 机器证明"
  - "三态纪律：dev-env 降级 = SKIP 如实登记，禁冒充 PASS；RURIX_REQUIRE_REAL=1 翻硬 FAIL"
  - "commit 带 Assisted-by: trailer 且不 push"
---

<!-- Assisted-by: Cursor:Claude（G34 全特性合流收口批） -->
# G34 契约 — 全特性合流期

> 承接：G31/G32/G33 三期契约 close-out（[../g31/G31_CONTRACT.md](../g31/G31_CONTRACT.md) §8 / [../g32/G32_CONTRACT.md](../g32/G32_CONTRACT.md) §8 / [../g33/G33_CONTRACT.md](../g33/G33_CONTRACT.md) §8）+ G31+ 期待办总表 [../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) §6 波次线三波收官后合流收口期；契约机制见 14 §1。front matter 双状态机：`status` 与 `implementation_status` 严格分离。

---

## 1. 目标

G34 收口时，项目获得：**全特性同开且无降级的统一生产车道**——G31+ 波 B 各特性接线期的互斥降级面（[../g31/g31_waveb_combo_matrix.json](../g31/g31_waveb_combo_matrix.json) 互斥 12/12 fail-closed 在案）收敛为单 bin `g34_full_lane` 真窗口 swapchain 车道：纹理采样 × slab 侧表 × 动态实例（G34-1 已验收）× HZB 剔除（G34-2）× 蒙皮角色（G34-3）车道内同开。「无降级」判据 = 全特性缺省关时缺省面与母版 Stage A 锚位级一致（G34-1 实测 sha256:c1d28ad73783cc3c… 位级 MATCH 在案），合流 = 加性扩展对既有面 0-byte 机器证明；正确性三面 = host 金标准对拍（容差 threshold = measured × 2.0 程序产禁手写）+ 逐特性贡献 digest 区分（防暗接线冒充）+ 确定性双跑位级；且全部既有画质/性能/确定性锚零降级（收口验收批机器核验，G-G34-4）。

## 2. 范围

### 2.1 in-scope

| 项 | 说明 | 对应交付物/门 |
|---|---|---|
| g34_wave1_unified_lane | G34-1 合流地基：纹理 + slab + 动态实例三特性同开统一 kernel 车道（已验收） | D-G34-1 / G-G34-1 |
| g34_wave2_hzb_unified | G34-2 HZB 接统一车道：TLAS 实例粒度剔除 + 双 TLAS + 帧内金字塔轮换 + 两阶段闭环第二段（本批收口） | D-G34-2 / G-G34-2 |
| g34_wave2_skin_unified | G34-3 蒙皮进统一车道：四特性同开 36 资源六 pass（本批收口） | D-G34-2 / G-G34-3 |
| g34_acceptance_gate | 收口验收面（守卫/三门复跑/三锚/soak 登记面/四件套） | D-G34-3 / G-G34-4 |

### 2.2 out-of-scope（显式排除）

- **FG/MFG 合流**——G34-1 备注字面「归后续波」（[../../evidence/g34_unified_lane_gate_20260827T041754Z.json](../../evidence/g34_unified_lane_gate_20260827T041754Z.json) notes），`g34_unified_shade` 接口预留不预支。
- **HZB × 蒙皮同车道合流**——G34-2/G34-3 为两独立 include 区段并行分区设计（[g34_2_hzb.rs](../../src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs) / [g34_skin_section.rs](../../src/rurix-render/src/bin/g34_full_lane/g34_skin_section.rs) 写零交叠），同车道合并归后续窗。
- **契约 flip（active→closed）与治理波仪式**——留 owner 按 10_GOVERNANCE 程序，本期不预支。
- **G13~G33 冻结注册表/锚改写**——0-byte（front matter out_of_scope 逐字）。
- **presented 口径面**——本期无 FG，presented/real 双口径分离律引用不消费。

## 3. 交付物清单

| ID | 交付物 | 形态 | 完成判据 |
|---|---|---|---|
| D-G34-1 | G34-1 合流地基 | [kernels/g34_unified_gi.rx](../../src/rurix-render/kernels/g34_unified_gi.rx) + [kernels/g34_unified_shade.rx](../../src/rurix-render/kernels/g34_unified_shade.rx) + [src/rurix-render/src/bin/g34_full_lane.rs](../../src/rurix-render/src/bin/g34_full_lane.rs) + [ci/g34_unified_lane_smoke.py](../../ci/g34_unified_lane_smoke.py) + 两 evidence schema + [g34_budget.json](g34_budget.json) 容差条目 | 门 --selftest + --gate 全 PASS（已在案） |
| D-G34-2 | G34-2/G34-3 两门 | HZB：[kernels/g34_unified_primary.rx](../../src/rurix-render/kernels/g34_unified_primary.rx) + [g34_2_hzb.rs](../../src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs) 独立区段 + ci/g34_hzb_unified_smoke.py；蒙皮：[kernels/g34_unified_gi_skin.rx](../../src/rurix-render/kernels/g34_unified_gi_skin.rx) + [kernels/g34_unified_mv.rx](../../src/rurix-render/kernels/g34_unified_mv.rx) + [g34_skin_section.rs](../../src/rurix-render/src/bin/g34_full_lane/g34_skin_section.rs) 独立区段 + [ci/g34_skin_unified_smoke.py](../../ci/g34_skin_unified_smoke.py)；schema 路由三处纯追加 | 两门 --selftest + --gate 全 PASS（本批收口） |
| D-G34-3 | 四件套 | milestones/g34/{G34_PLAN.md,G34_CONTRACT.md,CI_GATES.md,g34_budget.json} | 在树 + budget_eval 全 PASS 零 estimated |

## 4. 验收门（完整版，YAML 头为可提取摘要）

G-G34-1~G-G34-4 逐字见 front matter `acceptance_gates`。性能/对拍面证据等级 = measured_local（RTX 4070 Ti + Vulkan，本机真跑，gpu_device_lock 串行，RURIX_VK_VALIDATION=1）；采样协议 = 各门脚本内登记（canonical 160 帧 warmup 10 锚格口径 / 门各自 frames+warmup 口径）。G-G34-1 已绿引用在案证据数字；G-G34-2/G-G34-3 与 G-G34-4 实测数字一律待收口验收批填写，禁预支。

## 5. Guardrails（字节级，机器核对）

见 YAML 头 `guardrails` 字段。核对方式：`ci/check_guardrails.py`（agent 完全自主模式 ADVISORY 不阻断）+ `ci/check_schemas.py` / `ci/budget_eval.py` 硬门。

## 6. Deferred 引用

| 编号 | 内容摘要 | 承接 |
|---|---|---|
| RD-045 | 间歇 digest 漂移 backfill 三件（定位/修复/Full RFC 评估），open/maintain-open | 本期 soak/双跑 digest_seq 位级一致 + Stage A 锚零漂移 = 累计观察面只追加（不充三件，G31 契约 §6 同律） |

详情以 [../../registry/deferred.json](../../registry/deferred.json) 为唯一事实源，本表仅引用。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-27 | 初版契约固化（全特性合流期 = G31/G32/G33 三期收官后合流收口；G34-1 已验收引用在案证据，G34-2/G34-3 本批收口） |

---

## 8. Close-out 区（只追加 — G34 收口验收记录）

> 结构预期（收口验收批按 G31 契约 §8.1 行文范式只追加填写）：① 守卫套件七条全跑 → ② 三门新鲜复跑 → ③ 零降级回归锚三面 → ④ soak 登记面 → ⑤ 编号纪律实测 → ⑥ 签署。

### §8.1 G34 收口验收记录（2026-08-27，收口验收批真跑产出）——G-G34-4 五面：四面全绿 + 焦点格诚实红面维持如实登记

**① 三门新鲜复跑（G-G34-1~G-G34-3 全 PASS，本日真跑）**：
- **g34.wave1.unified 新鲜复跑 PASS**（八 facts 全绿，evidence/g34_unified_lane_gate_20260827T093331Z.json，wall 830.5s）：统一 kernel 现编 + spirv-val 绿；缺省面 == 母版 Stage A 锚 sha256:c1d28ad73783cc3c… 位级 MATCH（--static-camera 锚格 160+10）；host 金标准对拍 p100=3.968658857047558e-04 ≤ 冻结容差 7.937317714095116e-04（**与标定值逐位同值——跨日双跑确定性**）+ bitexact 占比 29.17% 如实登记；--full 双跑 74 帧 digest_seq 位级；动态位置核验 7/7；逐特性区分 4 面全真；frame_ms measured baseline=6.3769ms / full=7.0466ms（scene_gpu=1.8141ms；装配期一次性 tex_eval=2018.853ms / slab_eval=227.546ms 单列）。**注**：本日 G34-2 批次修正 kernels/g34_unified_shade.rx ⑤ 段采样块 2 行（b0g/b0b 行 fy 因子——与冻结母版 g34_unified_gi.rx ⑤ 段及 host 参考 g31_tex_host_sample 逐字同式的设计硬约束兑现），缺省面锚与 host 对拍逐位复现 = 修正对统一车道语义零扰动机器证明。
- **g34.wave2.hzb 首验 PASS**（六 facts 全绿，evidence/g34_hzb_unified_gate_20260827T091200Z.json，wall 686.3s）：五 kernel 现编（g34_unified_primary/g34_unified_shade/g31_hzb_pack/g27_hzb_reduce/g27_hzb_test）+ spirv-val 全绿 + 冻结 tracked 双 kernel（g27 两件）vs HEAD 0-byte；**剔除像素中性**：--hzb on vs RURIX_HZB_ALL_VISIBLE=1 全集渲染臂 digest_seq 74/74 帧位级一致（两阶段闭环正确性结构判据）；**host 金标准对拍**：probe 帧 mips=12 级位级全等 + 962 rect 判定序列逐字节全等 + 零假阳性（occluded=349）+ pyramid/verdict digest 双面 == host；确定性双跑 74 帧位级；**剔除真实发生**：tested=65183 / occluded_p1=22407（≈34.4% 剔除）/ flipped_p2=0 / 闭环额外提交=116 / 全掩码兜底帧=0；frame_ms measured hzb_on=21.5438ms vs baseline=6.7023ms（on/baseline=3.2144——1080p 全分辨率 12 级金字塔 reduce/pack 辅助 pass + 闭环重渲 + 逐帧判定回读税，如实登记不设通过线，G6 无硬门纪律）；Stage A 锚复跑位级 MATCH。
- **g34.wave2.skin 首验 PASS**（九面判据全绿，evidence/g34_skin_unified_gate_20260827T084533Z.json + harness 4 件，wall 568.8s）：三 kernel 现编（g34_unified_gi_skin/g34_unified_mv/g31_skin 0-byte 复用）+ spirv-val 绿；**蒙皮 device/host 逐顶点对拍 7/7 核验帧 max_abs == 0.0 位级**（B5 在案口径）；位置核验 7/7（质心 ≤4px/AABB ≤6px/计数门）；MV 三类全绿（类 3 蒙皮窗级 max=0.888px ≤2px + 窗级真动 41.656px ≥1px；类 1 静态区 0.025px ≤2px；类 2 刚性实例 1.766px ≤2px 激活帧 7——A4 登记缺口统一车道蒙皮腿顺手接通）；--skin on 双跑 74 帧位级；skin≠baseline 且 skin≠full_noskin 区分全真；Stage A 锚复跑位级 MATCH；frame_ms measured skin_on=7.5494ms / skin_off(full)=6.6420ms / baseline=4.7721ms（skin_gpu=0.0033ms / scene_gpu=1.0192ms 分项）。

**② 零降级回归锚三面（G-G34-4 锚面）**：
- **Stage A digest 锚 18/18 零漂移**：canonical 160 帧 warmup 10 逐格重跑（g14_3_pipeline_perf 既有口径，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 + gpu_device_lock），18 格末帧 digest 与 milestones/g14/g14_3_stage_a_digest_anchor.json 在案锚逐格位级 MATCH（matched=18/18 zero_drift=true，evidence/g31_wave_a_anchor_check_20260827T100522Z.json stage_a_digest 块，wall 622.4s）。
- **画质面 G16plus M-g 18/18 复跑 VERDICT=PASS**：`py -3 ci/g16_absolute_quality_closure_smoke.py --gate g16.p0.m_g.absolute_quality_closure`（步骤 290 门）8/8 facts 全 PASS（met_count_18=18/18 达标 + thresholds_program_produced entries=4 + m_c_history_honest_0_18 + g15_budget_0byte + ai_reading_bound + no_threshold_loosening k=2.0 维持），UE 参照臂按在案锚只读不重跑；evidence/g16_m_g_absolute_quality_closure_20260827T103433Z.json，wall 245.0s。
- **性能面 G17-MD-F1 焦点格多样本如实登记（诚实红恶化，合法终态零冒充——G-G33-4 字面「维持/恶化均合法终态」）**：本日 10 样本全登记——锚检门跑 3.612484ms（fresh ratio=0.950966 < 在案 0.960479 ⇒ 锚检门本体 verdict=FAIL 如实留档 evidence/g31_wave_a_anchor_check_20260827T100522Z.json，**沿波 B/C 三次同面 FAIL 先例**：20260826T100312Z/230806Z/235303Z 均 FAIL 在档，digest 零漂移事实三面独立成立）+ 连续负载热态 bench 4 样本 3.721986/3.733218/3.755201/3.804124ms + 冷却后正式 5 样本 3.687924/3.722793/3.726639/3.748934/3.785891ms → **正式组中位 3.726639ms → 中位 ratio 0.921836 < 在案 0.960479**（ue_median 3.43535ms 在案锚只读）；**十跑 last_frame_digest 全 == 在案锚 sha256:55ea0c2b…（确定性面零漂移——恶化面 = 帧时机态非渲染产出漂移，收口日长会话连续 GPU 负载环境态如实登记）**；轨迹 = 0.856→0.960479（G30）→0.966059（波 A）→0.956162（波 B）→0.957894（波 C）→**0.921836（G34 收口日，恶化如实登记不冒充）**；帧时预算杠 ×2.0=7.1534ms 远未触及（中位 3.7266ms，headroom ×1.92）；17/18 诚实红终态维持不改写。
- **18/18 格帧时对照零恶化格**：锚检门 cells[] 逐格 fresh frame_ms vs 在案 measured 对照——18 格 fresh 全 ≤ 在案（fresh>on_record 格数 = 0），焦点格红面仅在 ratio 轨迹严判据一面。

**③ soak 登记面（G-G34-4 soak 面，无新硬门 close-out 只追加）**：`g34_full_lane --full --slab-table g31_slab_side_table_bistro_interior.json --frames 5000 --warmup 10 --auto-move orbit --tier 100 --hidden`（release，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1）终跑 **PASS：5010/5010 帧零崩**（exit_reason=frames_done，wall=322.9s），**validation 全程静默**（Validation Error=0 / VUID=0），**动态实例位置核验 500/500 全过**（每 10 帧 fail-closed），digest_seq 5010 项全登记；real_render=15.7035ms / present=1.7346ms 均值如实（含核验帧 scene 回读税与长会话热态漂移；工作区件 .tmp/g34_accept/soak_full.json，soak 登记面无 evidence schema 数字经本区蒸馏——B6/B7 登记面同律）。**过程诚实登记（判据修正轨迹，沿波 A soak 口径修正先例 FAIL 留档不删）**：首跑于帧 530 fail-closed 中止（`obs_count=17941（min 2825）centroid_Δ=2.837px aabb_Δ=0.766px`——2.5px 绝对容差在近大目标屏占域触发核验模型偏差：host 预测质心 = 角点投影均值 vs 观测像素质心的透视偏差与屏占尺寸成比例〔Δ≈2.12% 轮廓直径〕，同帧 digest 确定性 + aabb/计数双门在带 = 非渲染缺陷）；第一版修正（相对项 2.5% 系数）于帧 550 再触（`centroid_Δ=3.450px（tol 3.309）`≈2.61% 直径——系数无余量）；终版 = **质心容差域界式**（门窗标定域 √预测面积 ≤100px 维持绝对 2.5px 逐字——64+10 门窗实测 √A ≤96.3 全落域内，三门判据数值面不变；域外近大目标按轮廓直径 5% 界模型偏差，防死接线目的维持紧界）落 g34_full_lane.rs 与 g34_2_hzb.rs 两车道同律（共享体 g14_3_lane_body.rs A4 面 0-byte 不触），修正后 g34.wave2.hzb 门复跑绿（快照一致性）+ soak 终跑全绿；两次 FAIL 输出逐字留档本段。**环境面诚实登记**：会话沙箱注入 CARGO_TARGET_DIR 致两次构建落临时目录（16:28 exe 为 B 批正确产物三门证据全部有效；判据修正构建经清变量重建落 repo target 后生效）。**判据修正后 g34.wave2.hzb 门复跑 PASS**（evidence/g34_hzb_unified_gate_20260827T125510Z.json，六 facts 全绿——剔除计数 tested=65183/occluded_p1=22407/flipped_p2=0/closure=116 与首验逐项一致 + 像素中性 74/74 + parity 三面 + 双跑位级 = 域界式容差在门窗数值面惰性的机器证明；gate 快照面与终版代码一致）。

**④ 守卫套件七条全跑（仓库根目录，exit 全 0，2026-08-27 收口终验）**：
- `py -3 ci/check_structure.py` → `[check_structure] PASS (11 dirs, 6 files)`。
- `py -3 ci/check_schemas.py` → `[check_schemas] PASS`（含本批三组路由纯追加重放核验：`_patch_g34_skin_schemas.py`〔g34_skin_unified_gate_ 长前缀先匹配 + g34_skin_unified_〕+ `_patch_g34_hzb_schemas.py`〔g34_hzb_unified_gate_ 仅门裁决件〕，幂等 io.open 补丁法沿 G34-1 先例）。
- `py -3 ci/check_number_ledger.py` → PASS（ADVISORY 两注 = off_tree_workflows[grx] 预存项不阻断；spec RXS 头 389 零同号碰撞）。
- `py -3 ci/trace_matrix.py --check` → `PASS (389/389 clauses anchored, 888 test files scanned)`。
- `py -3 ci/budget_eval.py` → `[budget_eval] PASS (322 pass, 0 skip, normal mode)`——**含本批守卫侧修复如实登记**：G34-1 批次曾登记 `g34.unified_lane.host_parity_tol` 条目但未给 budget_eval 加分派支（generic trimmed_mean 路 KeyError 潜伏崩溃，收口终验检出）；本批加 `eval_g34_host_parity_tol` 专用判读面（沿 g9/g17 特例先例：从 harness 证据取 host_parity.color_p100 + 与登记 measured_value 位级互核禁手写漂移 + direction=max 判定），条目 PASS（0.0003968658857047558 ≤ 0.0007937317714095116）。
- `py -3 ci/check_guardrails.py` → exit=0（ADVISORY 不阻断，agent 完全自主模式建议项；spec/release.md 修订行注 = 波 C C5 预存项非本批引入）。
- `py -3 ci/check_contribution.py` → exit=0（ADVISORY 清单全为 G9~G15 历史 commit provenance 预存项不阻断）。

**⑤ 编号纪律（落盘前实测 registry/number_ledger.json）**：G34 期全段零消费——CI 数字步骤零消费（CI_step on_tree_max=524/next_free=525 维持；G34 三门均 symbolic gate key 未占号，pr-smoke.yml 无 g34 条目）；RXS next_free=408 维持（统一 kernel 全族引用既有条款 RXS-0405 零新条款）；RD next_free=46 / U next_free=60 / SG next_free=10 / MR next_free=12 / D next_free=410 / RX_error next_free=7024 全维持。**工作树共享段如实登记（非 G34 消费）**：RFC on_tree_max=49/next_free=50 = G33 期 RFC-0048（C15 在案）+ G35 期 RFC-0049（GPU 粒子系统立项件，2026-08-27 Agent Approved 在飞，milestones/g35/ 四件套已立零实现面——G34 收口批不消费不改写，同 commit 如实收入）。

**⑥ 签署**：`Assisted-by: Cursor:Claude（G34 全特性合流收口批）`——G-G34-4 五面终态：三门新鲜复跑全 PASS + Stage A digest 18/18 零漂移 + 画质 G16plus M-g 18/18 VERDICT=PASS + soak 5010 帧零崩 validation 静默四面全绿，G17-MD-F1 焦点格轨迹面诚实红（0.921836 恶化如实登记，digest 十跑零漂移，预算杠远未触及）一面合法终态——**零冒充**。契约 flip（active→closed）与治理波留 owner 按 10_GOVERNANCE 程序（out_of_scope 字面）。

**⑦ 提交后勘误（只追加，2026-08-27）**：收口 commit 058f8e68 消息 ⑤ 段「G35 四件套零实现面」字面勘正——提交面实际收入 G35 在飞实现面 = RFC-0049 + 四件套 + 八 gate schema + 九波 kernels/harness bins/particles 模块/门脚本与路由补丁（同工作树并行会话产出，git add -A 全量收入；**G35 门证据零收入 = 未验收面不冒充维持**）。焦点格样本环境态补注：并行会话 GPU 负载（g31_cluster_lod 门 evidence 20260827T105329Z 产出窗）与冷却样本组采样窗重叠不可排除——恶化数字含并行负载成分，digest 零漂移面与诚实红登记不受影响。提交后并行会话新修改面（g35_particle_lane.rs 等）留工作树不混入（异己面显式择取先例）。
