<!-- Assisted-by: Cursor Claude Fable 5（G17.1 治理波） -->
# G17_PLAN — DLSS 性能缺口收口期（G15-MD-F1 字面兑现）定稿版

> **性质**：G17.1 治理交付物（契约四件套之一）；事实源为 [G17_CONTRACT.md](G17_CONTRACT.md) v1.0（冲突时以契约为准）。
> **法定输入三份（逐字，G17 全部范围由此导出）**：[G15_P2_DECISIONS.md](../g15/G15_P2_DECISIONS.md) §4 承接锚清单 G15-MD-F1 行（三件套字面）+ §5 汇总；[G16_CONTRACT.md](../g16/G16_CONTRACT.md) §7 立项裁决 3（十四行 + G15-MD-F1 → defer-to-G17+）；[G15_CONTRACT.md](../g15/G15_CONTRACT.md) §8.5~§8.7（G15-MD-F1 四轮复跑定盘留痕）。

## 1. 目标字面（G15-MD-F1 承接锚逐字兑现面）

G17 只做三件事 + 一套穷举：

1. **双端同协议复测与暖态重标定**（M-a）：G14 M-d 同口径协议，复测窗内双端同会话同协议复跑 bistro-interior/t100/dlss_sr（18 格全协议四轮）；UE 参照臂缓存暖态跨会话 −25% 环境迁移（G15-MD-F1 定盘事实）→ 暖态为新环境基线面，程序产重标定（禁手写阈，P-09），新阈值入 `g17_budget.json` 新条目（g14/g15/g16_budget 既有条目 0-byte）。
2. **NGX 版本演进面对齐评估**（M-b）：nvngx_dlss.dll 310.5.2 → 310.6.0+，PaddedWindowNetwork 实例化形态对齐评估；实测约束字面（G15 §8.7 留痕）：NGX in-stream ≈1.90ms + 提交固定 ≈0.10ms + scene ≈1.02ms，物理地板 ≈3.02ms；若 vendor SDK 演进带来 in-stream 成本变化，以新鲜命令输出重测分解并留档；无变化如实登记。
3. **车道架构面 D3D12 宿主 NGX**（M-c，Full RFC 触冻结面）：现状事实 = NGX Vulkan 执行 = CUDA cubin 宿主面（NGXCubinVulkan），无纯 Vulkan DLSS 路径；起草 RFC-0032（D3D12 宿主 NGX 车道，含跨 device 同步面/单 device 化评估），必须经 D-409 对抗评审后 Agent Approved 方可实现；结论可以是 go / no-go / defer——no-go 也是合法终态，但必须留档可机器核验的评估证据。
4. **P2 穷举决策**：G16 defer-to-G17+ 十四行 + G15-MD-F1 + 本期新增候选，逐行 go/no-go/defer 零空行，defer 必有承接锚（G18+）。G18 方向不在 G17 内实现。

**终判**（M-d）：bistro/t100/dlss_sr ratio ≥ ×1.00 → 性能 18/18；若物理地板/vendor 面使达标不可能 → 维持未达标登记不冒充，兜底字面与 G15 同源。两种结局都允许 close。

## 2. 波次推进表（十阶段）

| 阶段 | 波次 | 内容 | 门 |
|---|---|---|---|
| ① 立项 | G17.0 | 用户指令留痕 + 立项裁决 + 不可变 ref = g16-closed commit `8fc1fdaa`（实测 `git rev-parse g16-closed`） | 契约 §7 |
| ② 治理波 | G17.1 | 契约四件套 + 候选决策表 19 行 + 验收映射 5 P0 + RFC-0032 起草与 D-409 对抗评审 + measured baseline（G14 M-d 同口径一轮）+ 治理三门 materialize（步骤 293/294/295） | G-G17-1 |
| ③ 互锁门 | — | `ci/g17_interlock_check.py --require-ready` VERDICT=READY → `implementation_status: blocked → unlocked` | G-G17-2 |
| ④ 实施波 | G17.2 | M-a 双端复测与暖态重标定（四轮全协议 + 程序产新阈入 g17_budget + 差异如实分解） | G-G17-3 |
| ④ 实施波 | G17.3 | M-b NGX 310.6.0+ 对齐（换版评估 + provenance 登记 + X2 分解重测 + 画质守护双门禁 + A/B） | G-G17-4 |
| ④ 实施波 | G17.4 | M-c D3D12 宿主车道 RFC 终态兑现（approved 实现 / no-go / defer 留档） | G-G17-5 |
| ④ 实施波 | G17.5 | M-d t100 档有界优化 + 终判 18 格全协议复测（两态均合法收口） | G-G17-6 |
| ④ 实施波 | G17.6 | M-e 旧门零降级（G13/G14/G15/G16 受影响门 `--verify-latest`） | G-G17-7 |
| ⑤ 波聚合 | 每波 | `ci/g17_wave{N}_exit_check.py --gate g17.wave.{N}.exit`（只读汇总不代绿） | — |
| ⑥ 决策门 | G17.7a | P2 穷举（`G17_P2_DECISIONS.md` + `ci/g17_p2_decisions_check.py`）零空行 | G-G17-8 |
| ⑦ soak | G17.7a | `ci/g17_stabilization_soak.py` ≥1800s 零失败 + `budget_eval --strict` 零 estimated | G-G17-8 |
| ⑧ close-out | G17.7b | `ci/g17_closeout_check.py` 八 facts VERDICT=READY → status flip 独立洁净 commit | G-G17-9 |
| ⑨ tag | — | `g17-closed` + guardrail 基准链切换 | G-G17-9 |
| ⑩ 战后 | — | 未竟事项 → deferred.json 追加 RD（实测 next_free）+ `// STUB(RD-###)` 双侧标注 | — |

## 3. 关键工程事实（实施输入面，全部实测在案）

- G14 M-d 门 = `ci/g14_dual_end_fps_parity_smoke.py`（UE 臂 harness `milestones/g14/harness/g14_2_ue_bench.py` + Rurix 臂 `target/release/g14_3_pipeline_perf.exe`，三轮进程级独立 50×3 trimmed mean 跨轮中位数，GPU 锁纪律沿脚本既有面）。
- Streamline SDK 目录经环境变量覆盖（默认 `external/streamline-2.10.3`，见 `src/rurix-rt/src/vendor_upscale.rs`）——M-b 换版 A/B 走新缓存目录 + env 切换，不动既有缓存。
- G15 §8.7 税源分解（G17 重评输入面）：NGX in-stream ≈1.90ms + 提交固定 ≈0.10ms + scene_gpu ≈1.02ms ⇒ GPU-only 地板 ≈3.02ms vs G15 期通过线 ≈2.96ms。
- UE 臂 NGX = 310.6.0（CL 37642667，PaddedWindowNetwork 容器族）；Rurix 臂 = 310.5.2（encoder 族 + InternalHistoryA/B 全分辨率历史）——版本差为 G15-MD-F1 承接锚已命名重判触发面。

## 4. 风险与诚实边界

- **M-b 换版收益不确定**：310.6.0+ in-stream 成本可能下降/不变/上升——三种结果均如实登记；画质守护双门禁（digest 锚 + 锚带）超带即拒绝换版。
- **M-c RFC 结论开放**：D3D12 宿主车道收益（宿主 API 差 vs NGX 版本差归因）以 M-a/M-b 实测为输入评估；跨 device 同步税可能吞噬收益——no-go/defer 合法。
- **终判两态均合法**：达标 18/18 或维持未达标如实登记；禁冒充、禁放宽阈、禁改 ×1.00 口径。
- **RD-045 漂移监控**：复测面 Stage A digest 守护逐轮登记，检出即升级评估。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版（G17.1 治理波定稿；十阶段推进表 + 三件事字面 + 关键工程事实 + 诚实边界）。 |
