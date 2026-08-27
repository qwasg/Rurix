<!-- Assisted-by: Cursor:Claude（G34 全特性合流收口批） -->
# G34_PLAN — 全特性合流期执行计划

> 事实源 = [G34_CONTRACT.md](G34_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 期定位

G34 = **全特性合流期**（全流程无降级实时渲染管线收口期）：把 G31+ 波 B 各特性接线期的互斥降级面（组合矩阵互斥 12/12 fail-closed 在案，[../g31/g31_waveb_combo_matrix.json](../g31/g31_waveb_combo_matrix.json)）收敛为**统一生产车道**——单 bin `g34_full_lane` 真窗口 swapchain 车道内特性同开；「无降级」判据 = 全特性缺省关时缺省面与母版 Stage A 锚位级一致；正确性三面 = host 金标准对拍（容差程序产禁手写）+ 逐特性贡献 digest 区分（防暗接线冒充）+ 确定性双跑位级。上游法定输入 = G31/G32/G33 三期契约 close-out（[../g31/G31_CONTRACT.md](../g31/G31_CONTRACT.md) §8 / [../g32/G32_CONTRACT.md](../g32/G32_CONTRACT.md) §8 / [../g33/G33_CONTRACT.md](../g33/G33_CONTRACT.md) §8）+ [../g30/g30_campaign_handover_registry.json](../g30/g30_campaign_handover_registry.json) 谱系；上游定位 = [../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) §6 三条波次线与 §7 调研镜像 + [../g31_plus_campaign_record.md](../g31_plus_campaign_record.md)（三波 56 项终态映射）。

## 2. 波次

| 波次 | 内容 | 门 | 状态 |
|---|---|---|---|
| G34-1 合流地基 | 纹理 + slab + 动态实例三特性同开统一 kernel 车道 | g34.wave1.unified | 已验收（八 facts 全绿，[../../evidence/g34_unified_lane_gate_20260827T041754Z.json](../../evidence/g34_unified_lane_gate_20260827T041754Z.json)） |
| G34-2 HZB 接统一车道 | TLAS 实例粒度剔除 + 双 TLAS + 帧内金字塔轮换 + 两阶段闭环第二段 | g34.wave2.hzb | 本批收口（实测数字待收口验收批填写） |
| G34-3 蒙皮进统一车道 | 蒙皮 × 纹理 × slab × 动态四特性同开 36 资源六 pass | g34.wave2.skin | 本批收口（实测数字待收口验收批填写） |
| 验收面 | 守卫七条 + 三门新鲜复跑 + 零降级三锚 + soak 登记面 + 四件套 | G-G34-4 | 收口验收批 |
| 后续波 | FG/MFG 合流、HZB × 蒙皮同车道合并 | 后续波立项程序 | 未立项（out_of_scope 显式排除，接口预留不预支） |

## 3. 实现面（G34-1~G34-3 交付物）

- **G34-1 合流地基**（已验收）：统一 GI kernel [`kernels/g34_unified_gi.rx`](../../src/rurix-render/kernels/g34_unified_gi.rx)（母版 g14_3_direct_gi 语义 + fork A 图集采样块 + fork B 实例分派块合一，两 fork 面互不交叠证明与缺省面 == 母版位级逐 op 论证见 kernel 头注释；引用既有条款 RXS-0405，零新条款）+ 统一 shade kernel [`kernels/g34_unified_shade.rx`](../../src/rurix-render/kernels/g34_unified_shade.rx)（shade_reduce 语义 + out_depth_hz 恒输出——HZB 合流接口预留）；合并语义 = 贴图三角 采样×(mod×R_slot) / 非贴图 常量×(R_slot 若 slab 映射)，host 装配期预调制承载，kernel 零新增面；harness = [`src/rurix-render/src/bin/g34_full_lane.rs`](../../src/rurix-render/src/bin/g34_full_lane.rs)（UnifiedDescs::G34Full 27 SSBO 真窗口 swapchain 车道）；host 金标准对拍容差 = [g34_budget.json](g34_budget.json) 条目 `g34.unified_lane.host_parity_tol`（threshold = measured × 2.0 协议冻结 k 程序产禁手写）；门 `g34.wave1.unified`（八 facts）。
- **G34-2 HZB 接统一车道**（本批收口）：剔除对象粒度 = **TLAS 实例**（bistro 逐 mesh 节点 BLAS 分解 + 动态实例尾槽——动态实例为 A4 核验对象恒可见不参剔，如实登记）；消费点 = 主射线 pass TLAS 实例 mask（被剔实例 mask=0x00 ⇒ ray query 零遍历其 BLAS）——[`kernels/g34_unified_primary.rx`](../../src/rurix-render/kernels/g34_unified_primary.rx)（G34-2 加性）相机射线走初剔后 TLAS（表 0），`g34_unified_shade.rx` 扩展阴影射线走全量 TLAS（表 1，被剔实例仍投阴影——遮挡物阴影正确性面；RXS-0297 单 TLAS 签名纪律 ⇒ 拆 pass）；双 TLAS 逐帧 refit（render_exec G34-2 加性 `execute_with_frame_update_dual_tlas` 第二更新位）；帧内金字塔轮换 = G27 M-a 冻结 kernel（g27_hzb_reduce/g27_hzb_test）+ g31_hzb_pack glue 0-byte 消费，本帧真深度 = g34_unified_shade ④b 段 out_depth_hz，pass 序 = primary → shade → mv → tsr×2 → encode → test_p1 → reduce×(L−1)+pack×L → test_p2；两阶段闭环第二段（RFC-0044 §5.8：应见集 = p1 可见 ∪ p2 翻回，≤4 迭代未收敛 ⇒ 全掩码兜底 = 零剔除精确收敛）+ RURIX_HZB_ALL_VISIBLE=1 像素中性登记实验臂（digest_seq 逐帧对拍承载「剔除零假阳性 ⇒ 画面与全集渲染位级一致」）；host 金标准 = geometry/{hzb,cull}.rs 只读消费 0-byte；承载 = 独立 include 区段 [`src/bin/g34_full_lane/g34_2_hzb.rs`](../../src/rurix-render/src/bin/g34_full_lane/g34_2_hzb.rs)（与 G34-3 同窗并行分区，bin 本体仅加性挂点）；门 `g34.wave2.hzb`（六 facts）。
- **G34-3 蒙皮进统一车道**（本批收口）：蒙皮 × 纹理 × slab × 动态实例四特性同开——G34Full 27 SSBO 加性扩蒙皮七件（27=hit 命中信息通道 / 28=REST 绑定姿态 / 29=WT 权重 / 30/31=PAL_CUR/PAL_PREV palette 双表逐帧上传 / 32=PREV 蒙皮 prev 顶点表 / 33=SKIN_PARAMS）+ encode 两件（34=ACES / 35=BGRA8）= **36 资源六 pass**：g31_skin（0-byte 复用）→ blas_refit 桥（角色 BLAS 2 逐帧 UPDATE）→ [`kernels/g34_unified_gi_skin.rx`](../../src/rurix-render/kernels/g34_unified_gi_skin.rx)（G34-1 统一 kernel + out_hit 命中信息通道 + 角色实例分派）→ [`kernels/g34_unified_mv.rx`](../../src/rurix-render/kernels/g34_unified_mv.rx)（g31_skin_mv 镜像 + 类 2 刚性实例臂——A4 登记缺口统一车道蒙皮腿顺手接通）→ TSR 双 pass → display_encode；3 BLAS + 3 实例 TLAS 逐帧 tlas_update refit，顺序入口 inflight=1（FIF 流水面拒 tlas_update/blas_refit，A2 同律）；核验三面 fail-closed（蒙皮 device/host 逐顶点 max_abs == 0 位级 + 位置核验质心 ≤4px/AABB ≤6px + MV 通道类 3/类 1/类 2——B5 在案口径）；承载 = 独立 include 区段 [`src/bin/g34_full_lane/g34_skin_section.rs`](../../src/rurix-render/src/bin/g34_full_lane/g34_skin_section.rs)（G34S*/g34skin* 前缀自持与 G34-2 写零交叠，主 bin 仅 `--skin` 旗标解析 + 早分支两行挂钩）；门 `g34.wave2.skin`（九面判据）。

## 4. 验收门（G-G34-4，收口验收批）

1. 守卫套件七条全跑（check_structure/check_schemas/check_number_ledger/check_guardrails/check_contribution/trace_matrix --check/budget_eval），exit 全 0。
2. 三门新鲜复跑（--selftest + --gate 全 PASS：g34.wave1.unified / g34.wave2.hzb / g34.wave2.skin）。
3. 零降级回归锚三面（[ci/g31_wave_a_anchor_check.py](../../ci/g31_wave_a_anchor_check.py) 既有门复用）：Stage A digest 锚 18/18 canonical 160 帧重跑零漂移 + G16plus M-g 18/18 canonical 复跑 VERDICT=PASS + G17-MD-F1 焦点格新鲜多样本中位如实登记（诚实红维持/恶化均合法终态，禁冒充——G-G33-4 字面同律；锚检门焦点格轨迹面严判 verdict 如实留档，波 B/C 三次同面 FAIL 先例在档）。
4. soak 登记面：`g34_full_lane --full` ≥5000 帧零崩 + validation 静默——close-out 只追加登记，无新硬门。
5. 四件套落盘（本文件族）+ 契约 §8 close-out 实测 facts（收口验收批只追加填写）。

## 5. 编号纪律

CI 数字步骤零消费声明（三门均 symbolic gate key 未占号，pr-smoke.yml 无 g34 条目；[registry/number_ledger.json](../../registry/number_ledger.json) CI_step.next_free=525 维持——收口验收批实测核验）；RFC/RXS/RD/U/SG/MR/D/RX_error 共享段零消费（统一 kernel 全族引用既有条款 RXS-0405，零新条款零新 RFC）；evidence 前缀 `g34_unified_lane_`（已注册路由在案）/ `g34_skin_unified_`（本批）/ `g34_hzb_unified_gate_`（本批）经 check_schemas 三处纯追加登记（分岔分析见 [CI_GATES.md](CI_GATES.md) §4）。
