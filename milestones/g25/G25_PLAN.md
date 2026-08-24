<!-- Assisted-by: Cursor Agent（G25.1 治理波） -->
# G25_PLAN — 全量商用终审收官期执行计划

> 事实源 = [G25_CONTRACT.md](G25_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G25 = 「G19-G25 七期串行战役」收官期。上游法定输入 = G24_P2_DECISIONS §1 SAFE-GPU 行 + fps 终判锚（G19 M-d「终判归 G25」字面链）+ 全量 open RD/历史清册。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G25.1 | 治理四件套 + RFC-0042 + 对抗评审 + baseline 快检 + 治理三门 | 429/430/431 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G25.2 | M-a 画质终态维持核验 + M-b fps 18 格终判（焦点格新鲜单测真跑） | 432/434（post-interlock 实测顺位） |
| G25.3 | M-c 全链零降级 + M-d 承接锚归档 | 436/438 |
| G25.4 | M-e 旧门零降级（全量测试波） | 440 |
| G25.5 | P2 穷举 + stabilization soak ≥1800s | 442/443 |
| G25.6 | close-out 八 facts → status flip → tag g25-closed（战役收官） | 444 |

波聚合门 `g25.wave.{2..6}.exit` 步骤 433/435/437/439/441。

## 3. 实现面设计

- 画质表面 0-byte 机核闭集：display/post_chain/view_transform + presentation 契约 + kernels 默认臂 + g14_3_pipeline_perf vs g18-closed git-diff。
- fps 终判：最新 g14_m_d 18 格定盘 + 焦点格（bistro-interior/t100/dlss_sr）canonical 160 帧 bench 一轮真跑（GPU 独占窗）ratio 新鲜登记。
- 承接锚归档：g25_campaign_handover_registry.json 汇总七期 P2 表 defer/maintain 行 + RD 八条 + 历史清册十一条 + RD-045 累计观察。

## 4. 编号纪律

治理三门 429~431 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 432~444）；RFC-0042 实测领取并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费。
