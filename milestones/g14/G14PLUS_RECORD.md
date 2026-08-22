<!-- Assisted-by: Claude Fable 5（G14plus 超大纠正优化案——波0 治理立项批） -->
# G14PLUS_RECORD — G14plus 超大纠正优化案叙事总档

> **性质**：G14plus（G14.8~G14.12 延续波集）叙事与指针总档。**本档不承载任何判据**——机器可核事实源恒为 [G14_CONTRACT.md](G14_CONTRACT.md) §8 只追加验收记录、[G14_ACCEPTANCE_MAP.md](G14_ACCEPTANCE_MAP.md)、[RFC-0030](../../rfcs/0030-g14plus-pipeline-structural-optimization.md) 与 `evidence/` 真跑件；本档每处结论均附 evidence/文档指针。
> **载体裁决**：G14_CONTRACT §7 裁决 7 字面二选一（G14.x 延续波 / G16+ 里程碑）取 **G14.x 延续波**——用户 2026-08-22 指令「一次性完成 G14 硬收尾，门禁严格全绿」字面指向 G14 期内收口，G16+ 另立里程碑与「硬收尾」语义相悖。

## 1. 立项授权（双授权字面逐字登记）

- **2026-08-22 用户指令（本案立项授权面）**：「帮我一次性完成G14硬收尾，要求门禁严格全绿。先优化再测试以减少工期，最大化并行且允许委派fable5的子agent。需要真实读取渲染出图判断画面质量，性能优化的同时不能削减画质。本次任务可附加为G14plus作为文档记录，允许派遣大量gork4.6的子agent进行大量项目探索和深度论文技术调研，且允许计划前委派多个fable5的子agent进行多计划设计。本次进程允许视为超越G类里程碑的超大项目纠正优化案，不需要考虑工作量，务必完成任务使项目达到预期」。
- **2026-08-19 用户全期授权面（契约 §7 裁决 2 既登记）**：「彻底完成对标UE5渲染器的目标……同时优化渲染管线效率，使帧率对标UE5略高（不降级画质）……最终交付产物需要真实可商用，否则不要停止优化，并在此时允许在G15后无限制新建里程碑继续优化」。
- **目标判据（不改 G14 既有判据字面）**：M-d 门 18 格通过线 ×1.00 全部达标（严格全绿）+ 画质零降级守护（G13 锁定对拍 deficit 基线带内 + G14.3 车道锚带 ≤0.010779849285388998）+ 固定 seed 位级确定性协议成立（RD-045 修复）+ G14.5a soak / G14.5b closeout 全绿收口。

## 2. 波次结构与依赖（治理裁决快照）

| 波 | 名称 | 内容 | 验收面 | 依赖 |
|---|---|---|---|---|
| 波0 | 治理立项批 | 异己面 patch 存档清场 + P2 表后事件登记 + 本档首建 + RFC-0030 起草/D-409 评审/Approved + M-h 门 materialize（步骤 265）+ 契约 §8.8 | 治理文件齐备 + 守卫套件全 PASS | — |
| G14.8 | 测量与确定性基线波 | 锁频环境控制 + 环境画像登记 + flip-trace 诊断臂扩展（TSR 车道）+ RD-045 基线漂移率 N=20 + 根因定位取证 | §8.9 验收记录 + 诊断产物 | 波0 |
| G14.9 | host 面与 RT 白给波 | readback HOST_CACHED 选择器 + FIF=2 submit/collect 分离 + AS PREFER_FAST_TRACE(+compaction) + rurixc first-hit 内建与阴影臂切换 + 背光 keep 跳射线 + TSR kernel 变体（8×8） | 逐项 L0 位级探针（digest==旧锚）+ §8.10 | G14.8（确定性可信后验位级） |
| G14.10 | 帧循环重构波 | TSR 并 session（RD-045 候选根因消除）+ 生产帧关回读/锚点帧 + mv GPU 化（RFC-0030 §4.1）+ cornell 样本拆散重排 + vendor 原生 VkImage tag（可选） | L1 验证（SSIM+AI 读图）+ §8.11 | G14.9 + RFC-0030 Approved |
| G14.11 | 结构条件波（**仅当 G14.10 后 M-d 探针复跑仍有未达格**） | bistro 光栅 G-buffer 主可见性 + 保守光剔除 + 阴影结构阶梯（RFC-0030 §4.4 禁跳级） | 画质锚带复核硬前置 + §8.11b | G14.10 后决策点 |
| G14.12 | 复测收口波 | digest 锚 18 格重收割（三证）+ soak/closeout 特判达标分支修订 + M-c→M-d（18/18 判定）→M-a/M-b/M-e/wave 门复跑 → 收口基线 commit → soak full-run（≥1800s）→ M-h → closeout READY → §8.13 → flip → tag | M-h 门 + closeout 八 facts + §8.12/§8.13 | 全部优化波落定 |

**G14.8~G14.11 不设新 P0 门**——契约 §4.2 M-c 判据字面（「异步回读/提交面重叠 + 逐帧同步消除 + TSR device kernel 效率面」）本身即优化改动的法定验收对象，M-d 判据为全案终审裁判；各波退出事实由 §8.x 验收记录承载当波复跑 evidence。**唯一新门 = M-h**（附录 A：锚重收割合法性三证 + 18/18 达标 + RD-045 登记完备，步骤 265）。

## 3. 优化清单（十项技术改动 × 承接锚 × 所属波）

| # | 改动 | 承接锚 | 波 | 预期收益（取证来源） | 画质/确定性等级 |
|---|---|---|---|---|---|
| 1 | readback HOST_CACHED 分路选择器 | G14-N9（表后登记窗口提前） | G14.9 | bistro scene host 余 19/32/71ms → ~2/4/8ms（DLSS 同型先例 ~1.8GB/s） | L0 位级不变 |
| 2 | TSR kernel 变体 8×8（原 LocalSize 1,1,1） | G14-N17 重评（访存结构演进）+ RFC-0030 §4.5 | G14.9 | bistro TSR upscale ~120ms 锁死 → 正常 compute 量级 | L0 位级不变（逐像素独立） |
| 3 | AS PREFER_FAST_TRACE + compaction | RFC-0030 §4.8 登记面 | G14.9 | RT 遍历 1.1~1.4×（bistro 105 万三角） | L0 试探（漂移即弃） |
| 4 | rurixc first-hit 内建 + 阴影臂切换 | RFC-0030 §4.6 | G14.9 | 阴影段 1.3~2.0×（closest-hit 全遍历 → 早退） | L0 位级不变（存在性等价） |
| 5 | 背光 keep 预判跳射线 | RFC-0030 §4.4 L2 ① | G14.9 | 阴影射线 −20~50%（背光/切向面） | L0 严格位级不变 |
| 6 | FIF=2 submit/collect 分离 | G14-N10（窗口提前） | G14.9 | host 残余与 GPU 重叠 | L0（500 帧逐帧序列比对） |
| 7 | TSR 并 session + 生产帧关回读 | G14-N9/N12 联动 + RFC-0030 §4.5 L2/§4.3 L3 | G14.10 | 消 TSR 输入再上传 + 全帧回读归零；RD-045 候选根因消除 | L0 |
| 8 | mv GPU 化 | G14-N11（RFC-0030 §4.1 显式修订行） | G14.10 | mv 3.2~12.4ms → ~0.2ms | L1（ULP 微差，锚重收割） |
| 9 | cornell 16 样本拆散重排（3-pass 同序求和） | G14.4 取证 f 条 + RFC-0030 §4.4 | G14.10 | cornell RT 段 ~1.5×（延迟隐藏） | L0 位级设计（偏差即回退） |
| 10 | 光栅 G-buffer 主可见性 / 光剔除 / 样本阶梯 | G14-N13/N14（RFC-0030 §4.4 条件条款） | G14.11（条件） | bistro GPU 段至 ~3ms 预算 | L1~L2（锚带硬前置） |

## 4. 波次记录（逐波追加）

### 波0 治理立项批（2026-08-22）

- **异己面处置档案（G14-N6 处置形态变更登记）**：14 个已跟踪异己文件（`src/rurix-rt/src/render_exec.rs`〔F1 D3D12 import 脚手架，`--ignore-cr-at-eol` 实质 +378/−6〕+ 5 个 `external_import: None` 配套 + 5 个研究面 mod.rs 挂载 + `evidence/d3d12_interop_smoke.json` + `milestones/g12/g12_pt_sampler_selection.json`〔异己 timestamp〕+ `ci/check_schemas.py`〔纯 CRLF 漂移，-w diff=0〕）经 `git diff HEAD --output` 导出 patch **双份存档**后 `git checkout` 回退到 HEAD：
  - 存档①：`K:\rurix-ext\archive\alien_worktree_20260822\alien_tracked.patch`（1,514,122 bytes，sha256:5bd8ebfa6f580b90fab0370c0eb2f8522a612f284613b25f0f4d30282e7f509e）；
  - 存档②：`.tmp/g14plus_archive/alien_tracked.patch`（同内容副本，不入 commit）；
  - 6 个 untracked 研究面文件（ktx2_read/hzb/restir/sdf_trace/smrt/ssr，共 ~96KB）同步复制入两存档目录；原文件保留工作树（mod.rs 回退后不在编译树，无害零消费）；
  - 完整性核验：`git apply --check --reverse` exit=0（patch 可逆向应用 = 异己会话可随时 `git apply` 恢复）；
  - 恢复路径（归属会话自理）：`git apply K:\rurix-ext\archive\alien_worktree_20260822\alien_tracked.patch`；
  - 三张 G14 登记表（g14_budget / g14_fps_gap_registry / g14_ue_variance_samples）为 G14 车道门产刷新与只追加样本，**保留工作树态**（回退将删除合法只追加样本；G14.12 复测将全量重写）。
- **治理产物**：RFC-0030 v1.0 Agent Approved（[rfcs/0030](../../rfcs/0030-g14plus-pipeline-structural-optimization.md) + [D-409 评审](design/rfc0030_adversarial_review.md)）；P2 表后事件登记（G14plus 立项条，[G14_P2_DECISIONS](G14_P2_DECISIONS.md) 表后区）；MAP 附录 A M-h 行（步骤 265 实测领取）；`ci/g14_continuation_closeout_smoke.py` + schema materialize；契约 §8.8 立项记录；ledger v1.150 校准（RFC 30 消费 + CI_step 265 消费）。

<!-- G14.8 起逐波追加 -->

## 5. 复测轨迹（M-d 逐版 ratio 收敛表——逐波追加）

| 版本 | 时点 | 口径 | 达标 | cornell ratio 区间 | bistro ratio 区间 | 备注 |
|---|---|---|---|---|---|---|
| v1 | 2026-08-20 012652Z | 全量 | 0/18 | 0.0606~0.1504 | 0.0116~0.0221 | 首跑诚实红 |
| v2 | 2026-08-20 053525Z | 生产 | 0/18 | 0.0791~0.4363 | 0.0116~0.0456 | 生产口径双列（M-f） |
| v3 | 2026-08-20 122608Z | 生产 | 0/18 | — | vendor 重格 +19~+97% | vendor 并行化（M-g） |
| v5 | 2026-08-21 003053Z | 生产 | 0/18 | — | — | RD-045 首检出（bistro t50 tsr run1） |
| v6 | 2026-08-21 132325Z | 生产 | 0/18 | 0.0741~0.4566 | 0.0186~0.0595 | RD-045 复发（bistro t67 tsr run3）；G14plus 立项基线 |

<!-- G14plus 复测版本逐波追加；目标终态 = 18/18 -->

## 6. 终审（G14.12 收口后回填）

<!-- 达标定盘 + 遗留面（RD-045 观察窗、G15 承接锚）登记 -->

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-22 | 首建（波0 治理立项批）：立项授权双字面 + 波次结构 + 优化清单十项 + 波0 处置档案 + 复测轨迹基线表 |
