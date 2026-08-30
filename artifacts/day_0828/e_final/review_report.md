# 独立评审报告 — 2026-08-28「生产渲染画质修复战役」全改动只读审计

- 评审性质：独立评审（目标 = 找问题），只读审计——代码零修改、GPU 零调用；全部结论基于工作树源码、git 元数据、SPV 文件指纹与在档 evidence。
- 审计对象：昨夜巡航 + 今日战役六相（A1 灯提取 / A2 自动曝光 / A2b ACES 样条修复 / B 纹理全覆盖 / C GI R2 / D TSR 质量档 + E1 预设）全部未提交改动。
- 评审时间窗：17:25–18:05；HEAD = bece24e7（2026-08-28 01:12，其提交信息已把 00_MASTER_INDEX/11_ROADMAP/G35_CONTRACT/g31_window_present.rs/aces13.rs/g31_display_encode.rx/g18_smooth_nrm.rx/artifacts/ 列为并行会话在飞面留工作树）。

## 总结论：**有条件合入**

战役六相自身全绿成立：锚链数字全量互核一致、红修记录如实完整、kernel 数值面抽查零实错、位级隔离路线在两 bin 代码路径上无漏洞。附加条件（见 CONCERN 处置清单）全部为治理/收尾面，无一触及已交付功能的正确性：

1. 工作树 6 个冻结面/并行会话文件被 11:20:27 批量事件翻了行尾（CRLF），文本零变化——合入前须归一（不归一会污染他会话 commit）。
2. 共享 `g31_display_encode.spv` 已于 10:24 被 A2 覆盖（结构性无害已论证，但 RD-045 P02 腿旧二进制消费面未机器复跑）——Phase E 必须补跑该腿。
3. 三处「源码-锚字节 divergence」（encode v2 / texture_nrm_gi pre-C / RD-045）已如实登记，合入主线时必须按交接项统一处置，在此之前任何门从源码重编共享路径件都会破锚。
4. combo7（8b1c12f3）终态重建后复验被中断未收割（已列 Phase E 回归矩阵，不重复裁决）。

## 分面裁决表

| 面 | 裁决 | 一句话结论 |
|---|---|---|
| 1 治理红线 | **CONCERN**（2 条，均不阻断） | 冻结 kernel/temporal/material 零触碰✓；ci/milestones 出现 EOL-only 字节翻转；A2 覆盖共享 SPV 一事结构合规但欠一次机器复验 |
| 2 位级确定性 | **PASS** | 三处字节隔离换载逻辑无臂组合漏洞；params 恒等声明逐槽核实 |
| 3 边界条件 | **PASS**（3 条低危观察） | heap 寻址/tritex/NEE 除零/复位/union-find 全安全；R2 f32 >100k 帧退化已如实登记且与 soak 时长相交 |
| 4 性能声明 | **PASS**（1 条口径注记） | 五组帧时数字全部与 evidence 复算一致；B 相 +0.19ms 为组合增量口径（单臂 +2.1ms 已在 summary 如实披露） |
| 5 证据链完备性 | **PASS** | 锚表六 digest 全链互核一致；C/D 红修与中断事件记录如实且证据文件在档 |
| 6 kernel 数值面 | **PASS**（1 条低危观察） | ACES 12 处修复与 aces13.rs 金标准逐系数相等；GI2 采样/NEE 无偏性成立；tsr_q 膨胀区间判据可证涵盖母版判据 |

---

## 面 1 治理红线（CONCERN）

### 1a. 冻结闭集核验 — PASS
`git status --porcelain` 全量过滤：`g14_3_direct_gi.rx / g16_gi_multibounce.rx / g18_light_transport_depth.rx / g14_8_tsr_*.rx / temporal/tsr.rs / material/` **零命中**（无 modified 无 untracked）。冻结 SPV 侧证：`.tmp/g14_gates/m_c/` 中 `g14_8_tsr_{resample,resolve}.spv / g14_3_direct_gi.spv` mtime = 8/27 18:07、`g16_gi_multibounce.spv / g18_light_transport_depth.spv` = 8/24，今日零写入。

### 1b. 【CONCERN-1】ci/** 与 milestones/** 出现 EOL-only 字节翻转
- 事实：`ci/_patch_g31_cluster_lod_schemas.py`（±103 行）与 `milestones/g31/g31_cluster_lod_evidence_schema.json`（±151 行）在 git status 中为 modified；`git diff --ignore-cr-at-eol --stat` 对这两件 + `src/rurix-asset/kernels/g31_cluster_cull.rx`、`g31_cluster_cull_device.rs`、`src/rurix-geom-build/{lod_bounds,qem}.rs` 共 6 件**输出为空** = 纯 LF→CRLF 翻转（`git ls-files --eol`：i/lf w/crlf；`core.autocrlf=false`）。
- 归因线索：6 件 + 00_MASTER_INDEX/11_ROADMAP/G35_CONTRACT/aces13.rs/g31_display_encode.rx 等 mtime **全部 = 11:20:27 同秒**（批量事件），与 A2b 会话 11:21 崩溃（CAMPAIGN_LOG L68）时间吻合——疑似编辑器/崩溃恢复工具批量落盘所致，非人为语义编辑。战役日志未记录此事件。
- 影响域：字节面上 ci/**、milestones/** 冻结红线的字面（0-byte）被破；文本/语义零变化；若 cluster-lod/G35 会话原样 commit，会把 CRLF 噪声带进历史。
- 处置建议：6 件 EOL-only 文件由持有会话（或主线收编时）`git checkout --` 恢复（EOL-insensitive diff 为空，恢复零损失）；向 cluster-lod 会话通报勿把 CRLF 版本入 commit。

### 1c. 他会话文件零触碰 — PASS（内容判定）
`00_MASTER_INDEX.md`（+2 行）、`11_ROADMAP.md`（+2 行）、`milestones/g35/G35_CONTRACT.md`（+28 行）三件 diff 内容**全部为 G35 粒子系统收口记录**（§8.1 验收批、勘误行、路线图 G35 行），零战役指纹（无 lamp/AE/纹理/GI2/TSR 字样）。bece24e7 提交信息已declare三件为并行会话在飞面。注：三件 mtime 同为 11:20:27（同 1b 批量事件刷新），bece24e7 时点的工作树未有快照故无法字节级证等，但内容归因结论明确：**战役未动他会话文件**。

### 1d. 共享 SPV 字节核验 + A2 时点覆盖合规性 — PASS 附【CONCERN-2】
- `.tmp/g14_gates/m_c/g31_display_encode.spv`：mtime **10:24:02**（= A2 增益守卫重编时点），此后零写入（A2b 走 v2 隔离、C/D 各走独立 SPV，`.tmp/night_0828/spv/` 时间线 11:12/13:12/14:27/16:49 全部落在隔离路径）。sha256 `43b0c255…` 与 A2b 治理审计记录（a2b ACCEPTANCE_SUMMARY spv_governance.audit.shared_spv）逐字一致；改前件备份 `a2_autoexp/g31_display_encode_pre_a2.spv.bak`（sha `ba638a31…`）在档。
- A2 时点覆盖的合规性论证（评审独立复核）：
  - HEAD 版 `aces13.rs` L482-483：`debug_assert_eq!(v.len(), 133); v.resize(136, 0.0)` ⇒ **一切旧二进制的 enc_params buffer 均为 136 f32 且 [133]=0**，新 SPV 读 [133] 无越界；
  - kernel 守卫（g31_display_encode.rx L76-81）`aeg≤0 → 1.0`，×1.0 对有限色值 IEEE 位级恒等；
  - 覆盖当时在战役二进制上复验 off==5596a730、dither==e989c6ee（a2 ACCEPTANCE_SUMMARY step2/3，均为「重编 SPV 后与既往锚位级相等」的强证）。
  - 结论：覆盖行为对全部消费方**结构性输出不变**，合规。
- 【CONCERN-2】残留缺口：`ci/g31_blocked_probes_smoke.py` P02 RD-045 腿 = **旧 target/release 二进制 + 共享 SPV** 消费面、硬编码 presented 锚 060e69a8——10:24 覆盖后**该腿未机器复跑**（A2b 审计发现该腿是在覆盖之后，且当时的裁决只管住了样条修复不再覆盖共享件）。结构论证成立但缺一次真跑闭环。
  - 影响域：若上述论证有未知破绽（如旧二进制路径差异），RD-045 门会在下次 CI 触发时红。
  - 处置建议：Phase E 回归矩阵**必须**加跑 P02 腿一次（预计分钟级）；绿则闭环，红则按 a2b divergence_handover 预案重收割 060e69a8。
- Divergence 登记核验：encode 源码（已修样条）vs m_c 字节（未修样条）、`g31_texture_nrm_gi.rx` 现源（含 GI2）vs 锚字节（pre-C）两处 divergence 均已在 a2b/c 两份 summary 以交接项如实登记，附重编触发条件（如 `ensure_encode_spv` 缺件重编）与备份件（`g31_texture_nrm_gi_pre_c.spv.bak` sha fd22cb19）。**合入前该状态是脆弱平衡：任何会话触发共享路径重编即破锚**——主线收编时限期执行「统一切 v2 字节 + 锚重收割」。

## 面 2 位级确定性（PASS）

- **encode**：`G31_DEFAULT_SPV_ENCODE = .tmp/night_0828/spv/g31_display_encode_v2.spv`（g31_window_present.rs L233）——本 bin 全臂（含 all-off）恒走 v2，锚系已全量重定基（55e4a92d 族），与 m_c 共享件消费方（他会话/旧二进制）互不干扰。✓
- **纹理/GI2 两级换载**（L6116-6118 / L6132-6134）：`textures&&smooth && spv_texture==默认字面` → NRM（pre-C 锚字节）；`gi2 && spv_texture==NRM 字面` → _gi2.spv。gi2 前置 fail-closed `gi2 && !(smooth && textures) → 拒`（L6127-6129）⇒ **不存在「某臂组合意外载错 SPV」路径**：gi2-off 恒 pre-C 字节、gi2-on 恒 _gi2 字节。唯一边缘：`--gi2 + 显式 --spv-texture <自定义>` 时尊重用户件不换载（与 --spv-resolve 同律的显式覆盖政策，双 bin 一致），若用户给了不含 GI2 段的 SPV 则 params[51] 写入而无消费面（静默无 GI2）——属既有「显式面尊重」政策边界，非漏洞，登记为观察。
- **TSR**（窗口 L6180-6182 / bench pipeline_perf L892-894）：`tsr_quality && spv_resolve==默认字面` → _q.spv；off 臂恒载 m_c 冻结字节（mtime 8/27 佐证从未被动）。双 bin 逻辑镜像一致。✓
- **params 恒等**（lane_body pack_frame_params_gi2 L7090-7163 + pack_tsr_params L7217-7250 + 窗口 L2831-2833）：
  - [49] lamp_contrib：smooth 车道恒写，默认 0.0 == 零填充逐位同值✓；[50] k_pix 无条件写 0.0 同律✓；[51..55) 仅 gi2=true 写，false 不写 == resize 零填充✓；tsr_params 19 项显式 + `resize(32, 0.0)`，[19]/[20] 仅 tsrq on 后写✓（冻结 resolve kernel 不读 [19..21)，g31_tsr_resolve_q.rx L38-41 布局声明与 pack 一致）。
  - AE 增益：8 组合（textures×smooth×bloom）下标决策树完备无 fall-through（窗口 L7344-7362），与 B 相 resource_indices 表一致。
- 空体 ray-query 循环 / branchless gate / 「if 包 while」禁用纪律在全部新 kernel 一致执行（A1 spirv-val 拒收事件后的绕行形态）。

## 面 3 边界条件（PASS，3 条低危观察）

| 项 | 结论 | 证据 |
|---|---|---|
| texel heap u32 寻址 | 安全 | 头表 910 = 70×13，最大下标 69×13+12=909；级内偏移 y·mw ≤ 2047×2048≈4.19M、heap 总量 74,106,060 texel（296.4MB）≪ 2³²；host 侧 u64 累加 + 2GiB fail-closed 断言（lane_body L6036-6040）；全 pow2 尺寸 ⇒ ×0.5 折半与 1/mh wrap 均 f32 精确；缺级槽位重复末级偏移 + kernel lod 钳 mips−1 双保险（L6021-6027 + g31_texture_nrm_gi.rx L199） |
| tritex 步幅 2 OOB | 安全 | kernel 最大读 (tri_count−1)×2+1；host 表恒 2 f32/tri（含灯面/未映射 [−1,0]）；bench 哑表五件同构（tritex 全 [−1,0] ⇒ tex_gate=0） |
| points 槽 6 radius | 安全（注记） | 注释「pack 槽 7」为 1-based 说法，实 pack 序 `[pos3, I3, radius, 0]` = 0-based 下标 6（lane_body L2144-2153），与 kernel `points[pb+6]`（g18_smooth_nrm.rx L388 / g31_texture_nrm_gi.rx L413）一致；契约灯 radius=0.0 ⇒ `max(2ε,0)=2ε` 位级不变；负值被 max 吸收；NaN 仅可能来自 host（提取式 `r2max.sqrt()+0.02` 恒有限，不可达） |
| R2 f32 frame_idx | 已如实登记（相交 soak） | u 粒度 ~1.5e-4（≤2.5e3 域）✓；frame_idx>100k 退化至 ~4e-3 已在 C summary leftovers 登记。**注意**：E 相 soak ≥1800s × ~120fps ≈ 216k 帧，gi2-on soak 会进入退化域——退化形态为噪声图案质量下降（无 UB/无崩溃），可接受但应在 soak 判读中知悉；根治 = 双 fract 拆和（已留窗） |
| autoexp 跨 era 复位 | 安全 | state buffer 初值 `G31_AE_STATE_INIT = [0u8;16]`（窗口 L787），era 常量面随 resize 重建归零 → initialized=0 → 首帧直取 target（state kernel L53-55）；~12 帧再适应已登记 |
| union-find 确定性 | 安全 | BTreeMap 键序 ×2 + 固定 26 邻域序 + min-root 规约 + 成员「格序×格内升序」求和序 + `total_cmp` 降序全序含质心字典序并列裁决（lane_body L2321-2428）——全链无 HashMap/无浮点序歧义 |
| GI2 NEE point_count=0 | 安全 | 无除法（权重形态 = ×point_count）；`gi_neen = (gi_hit × gate(pcf))` 零点光 ⇒ while 零迭代零读零射线；`gi_psel = (u3·pcf).min(pcf−1).max(0)` 同时钳掉 u3→1⁻ 的 f32 上取整边缘（g31_texture_nrm_gi.rx L596-607）。cornell 场景即便未来接入 gi2 腿，kernel 面 fail-safe；现行 bench --gi2 须随 --smooth-normals（fail-closed），验收面恒 bistro |

低危观察（不阻断）：
- **O-1** 退化 trinrm 行（零法线）+ smooth on 时 n=(0,0,0)，GI2 反弹射线方向经 `max(tiny)` 保底后为 (0,0,0) 零向量 ray query（g31_texture_nrm_gi.rx L520-544）——潜在驱动 UB 面。实测全程 validation 静默 + bistro glTF 法线无零行，风险极低；若后续接任意资产建议 host 装配期断言 trinrm 行非零。
- **O-2** AE NaN 中毒路径：若 TSR 输出含 NaN，log2 归约可传导 NaN 进 EMA state 并且永不自愈（encode 守卫 `NaN≤0 == false` 放行）。上游 TSR 链已有 NaN 门（encode 头注登记），且全程真跑未见；登记备查。
- **O-3** tsrq 邻域 clamp K>0 且 3×3 邻域全黑时 `limit=0` 将新样本整体置零（g31_tsr_resolve_q.rx L265-268）——孤立亮件在纯黑邻域下被全杀而非按 K 缩放。该旋钮 K=0 默认关且 D 相已登记「未实测后备旋钮」，启用前应补此边界语义评估。

## 面 4 性能声明（PASS，1 条口径注记）

| 声明 | evidence 复算 | 裁决 |
|---|---|---|
| 灯 +1.93ms | a1 summary：off 0.943/0.957 → on 2.876/2.888，Δ=+1.93ms ≤3ms 门 | ✓ |
| AE 0.11-0.19ms | a2 summary perf：单臂 0.186-0.197 / 组合 0.111（<0.1ms 期望未达已如实登记） | ✓ |
| 纹理 +0.19ms | b summary 7_frame_time：(7.902+8.288)/2 − 7.909 = **0.186ms（+2.35%）**，复算相符 | ✓（注记）|
| gi2 ×1.65 | c summary 5_frame_time：1.574474/0.952173 = **×1.654**（scene_gpu 口径，门 ≤2×） | ✓ |
| tsrq ~0 | d summary 7_frame_time：upscale_ms 0.5388/0.5432（default×2）vs 0.5422（on），增量落 ±0.005 run 噪声带 | ✓ |

注记（B 相口径）：+0.19ms 为「同组合含/不含 --textures」增量（验收门口径），且落在 run-to-run 离散 ~0.39ms 噪声带内；**tex 单臂 vs 裸五 pass 车道为 +2.1ms/+35%**——两口径均已在 b summary 7_frame_time 如实并列，CAMPAIGN_LOG 头条仅引组合口径，主线收编叙事时建议保留双口径避免误读。

## 面 5 证据链完备性（PASS）

锚 digest 全链互核（log 8-hex ↔ summary 全长 sha256 ↔ 跨相复验）：

| 锚 | 出生 | 跨相复验链 | 一致性 |
|---|---|---|---|
| c1d28ad7（bench Stage A 默认） | 冻结锚 | A1→A2b→B→C→D 五次真跑相等 | ✓ |
| 55e4a92d（窗口 all-off v2） | A2b 重定基 | B/C/D 各相复验（D 含终态重建后二验） | ✓ |
| 8b1c12f3（七臂合流） | B | C（含 E1 恒等探针 == 之）→ D 接线后复验；终态重建后复验中断→归 E（如实登记） | ✓ |
| 778f1dfc（夜巡 D2 render128） | 夜巡 | A1→C→D 三次相等（kernel/binary 演进零漂移旁证） | ✓ |
| 6144d9f7（gi2 c001 render128） | C | D 臂③复验相等 | ✓ |
| 05532d5e（bench snrm+tsrq ×2） | D | 双跑位级 | ✓ |
| 6bd3af63（窗口九臂 ×2） | D | 双跑位级 + ≠ 八臂锚 b36c3e1f | ✓ |

作废锚系登记完整：5596a730/e989c6ee/fd5ca68c/a4695558（A2b 表格逐条 old→new）+ 夜巡 presented 系（b02b08b57/12d5dc91/48353e86/2b6efac6/db7d48f7）+ 旧 tex 臂 6fab598c——归属与重收割去向（Phase E / RD-045 预案）均有落点。

红修/事故记录如实性抽查：
- **C 相 d89848b9**：首编覆盖共享 SPV 的漂移现场 evidence 保留（`ev/combo7_1.json` 标注「根因档案」）、E1 探针件（`_e1_tailadd.{rx,spv}` + ev == 8b1c12f3）、pre-C 重编字节自证 sha fd22cb19、纪律修订（ray query 站点演进不可依赖 gate=0 恒等）——全链在档 ✓。
- **D 相 v1/v2 红**：`d_metrics_v1_red.json`（v1 零效）+ `d_ladder.json`（v2 阶梯）在档，红臂 digest（f3eb1e2d/a0e7e4b2/3f9bd0f4/bec31360）保留于 arms_digests.evolution_intermediates，「两次红修额度用尽」自我裁决与战役纪律一致 ✓。
- 其余如实登记抽查：A1 弃簇（1 簇/1692 tri/flux 0.057）、A2 PermissionError+stale holder pid 6028、A2b 会话中断与 render_runs 首行缺失、B 红帘 0 顶点在框换替补验收位、D combo7 复验中断——与 CAMPAIGN_LOG 叙述一一对应，无粉饰痕迹 ✓。

## 面 6 kernel 数值面抽查（PASS，1 条低危观察）

### ACES 样条修复（12 处）——逐系数对照金标准：全对
金标准 `SPLINE_M = [[0.5,−1,0.5],[−1,1,0.5],[0.5,0,0]]`，`vmul(cf,M)` 行向量约定（HEAD aces13.rs L102/L153-161）⇒ b0=0.5·cf0−cf1+0.5·cf2 / **b1=−cf0+cf1** / **b2=0.5·cf0+0.5·cf1**。kernel 修复后 12 处（c5 L223-231/234-242…、c9 L331-339… 共 c5/c9 × RGB × low/high）与之逐字相等；b0 未动正确（两约定同值）。独立实证旁证：parity 探针 2,073,600 px exact 99.9891%/p100=1LSB/>1LSB=0、fan 像素 ±0、0.18 灰→99。旧形恰为 M 转置——根因叙述与代码事实吻合。

### GI2 段（g31_texture_nrm_gi.rx L493-661）
- 余弦半球变换：`(√u2·cosφ, √u2·sinφ, √(1−u2))` 标准形；切基 `t1 = cross(up,n)` 展开逐项核对无误，up 选择门 |ny|<0.999 下 |t1| ≥ 0.0447 远离退化；b = n×t 右手系；两次归一化防御。✓
- pdf 相消估计器：`Lo += al·scale·clamp(em₂ + al₂/π·NEE)`，cos/π pdf 与 Lambert π 相消 ⇒ al·L 无偏形。✓
- 单灯 NEE ×point_count：均匀选 1/N × N 加权 = 无偏 ✓；clamp 语义 = 逐通道对 L_bounce（scale 前）上钳 [0, params[53]]，与登记一致 ✓。
- R2/R3 常数：1/g=0.7548776662466927、1/g²=0.5698402909980532、1/g₃=0.8191725133961645、1/g₃²=0.6710436067037893、1/g₃³=0.5497004779019703 —— 与 plastic 常数 f64 展开逐位相符 ✓。
- 已知偏差（quad 无 NEE、反弹点 mats 均值直读）与登记一致。观察 O-1（零法线退化）见面 3。

### g31_tsr_resolve_q.rx
- 3×3 膨胀深度区间（L206-246）：中心 ∈ 窗 ⇒ `dp∈[dmn−slack, dmx+slack]` 可证涵盖母版逐点判据 `|dc−dp|≤tol·max`（slack 同源）——「区间自然涵盖」声明成立；真 disocclusion 拒史逻辑保留；色彩面 AABB 钳制未放松（拖影已由 dolly 面单独验收）。✓
- Karis 混合（L276-289）：`a2 = αw_c/(αw_c+(1−α)w_h).max(tiny)`——α ≥ α_q/2 > 0 且 w_c>0（有限输入）⇒ 分母恒正；inf 输入优雅退化到取史；YCoCg 正反变换逐项复核精确互逆。HDR 尖峰有偏（偏暗）已登记。✓
- alpha 档语义（L258-261）：reactive 优先 max 后钳 [α_q/2, 1]，与「下限 = 档位一半」声明一致。✓

---

## CONCERN 汇总与处置清单

| # | 内容 | 影响域 | 处置建议 | 阻断合入？ |
|---|---|---|---|---|
| C-1 | 6 件冻结面/并行会话文件 EOL-only 字节翻转（11:20:27 批量事件，疑 A2b 崩溃恢复工具） | ci/** 与 milestones/g31/** 字面 0-byte 红线；他会话 commit 卫生 | 持有会话 `git checkout --` 恢复 6 件（文本零损失）；向 cluster-lod/G35 会话通报 | 否（但须在任何 commit 前处理） |
| C-2 | A2 覆盖共享 encode SPV 后，RD-045 P02 腿（旧二进制+共享件+硬编码锚 060e69a8）未机器复跑 | ci/g31_blocked_probes_smoke.py 下次触发 | Phase E 回归矩阵加跑 P02 腿一次；红则按 a2b divergence_handover 预案重收割 | 否（E 相必办项） |
| （登记项）| 三处源码-锚字节 divergence（encode v2 / texture_nrm_gi pre-C / RD-045 共享件） | 任何从源码重编共享路径件的门/会话 | 已在 a2b/c summary 交接登记；主线收编时限期执行「统一切新字节 + 锚重收割」，此前工作树为脆弱平衡 | 否（交接项已在案） |
| （登记项）| combo7 8b1c12f3 终态重建后复验中断未收割 | D 相终态二进制 off 臂完整性（alloff+bench 已二验，仅 combo7 缺） | 已列 Phase E 回归矩阵，照办即可 | 否 |
| （提示项）| gi2-on soak（E 相 ≥1800s ≈ 216k 帧）将进入 R2 f32 >100k 帧退化域 | soak 观感/指标判读 | 知悉即可（退化非 UB）；或先落双 fract 拆和再 soak | 否 |

低危观察（O-1 零法线退化射线 / O-2 AE NaN 中毒 / O-3 tsrq clamp K 全黑邻域全杀）与 B 相帧时双口径注记见正文，均不需即时行动。

## 评审方法与覆盖度声明

- 代码面：g31_window_present.rs（SPV 常量/换载/fail-closed/params 挂载/AE 下标全路径）、g14_3_lane_body.rs（灯提取/points pack/params pack/heap 装配/哑表）、g14_3_pipeline_perf.rs（六臂 CLI/E1 展开/换载）、aces13.rs（HEAD vs 工作树）、g31_display_encode.rx（全 diff）、g31_texture_nrm_gi.rx（全文 686 行）、g31_tsr_resolve_q.rx（全文 312 行）、g31_autoexp_{reduce,state}.rx（全文）、g18_smooth_nrm.rx（A1 段抽查与统一 kernel 逐字比对）。
- 元数据面：git status/diff（含 --ignore-cr-at-eol/--eol）、HEAD 提交信息、m_c 与 night spv 双目录 mtime/长度/sha256、pre-A2 备份件哈希比对。
- 证据面：CAMPAIGN_LOG 全文 + 六份 ACCEPTANCE_SUMMARY 全文互核 + 关键指标算术复算。
- 未覆盖：夜巡五臂（bloom/dither/smooth/ggx/env）kernel 全文精读（夜巡已有自身验收与今日各相锚链回归覆盖其 off 恒等性，仅在统一 kernel 继承面与 encode dither 段复核）；bench --render EXR 像素级复验（GPU 被占用，依约零调用）；e_final 合流回归结果（另一子 agent 在跑，不重复裁决）。

*评审人：独立评审子 agent（只读）；2026-08-28 18:05*
