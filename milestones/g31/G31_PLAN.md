<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 A 验收门 Task A6） -->
# G31_PLAN — 实时呈现期执行计划（波 A 范围）

> 事实源 = [G31_CONTRACT.md](G31_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 期定位

G31 = **实时呈现期**（G31+ 待办总表 §6 波次线 1）：把"没接屏幕的引擎"接成实时游戏画面——生产管线 swapchain 真窗口呈现 + 帧流水化 + 游戏循环最小面 + 动态场景更新通路 + 帧生成（FG/MFG）生产接线。上游法定输入 = `milestones/g30/g30_campaign_handover_registry.json`（RFC-0047 §5.5 唯一法定输入面；G13-N7 行 FG 生产接线窗锚）+ `G31_PLUS_COMMERCIAL_RENDERER_TODO.md` §1.1 #1~#4 + §1.2 #5。上一期 G30 已 closed（tag `g30-closed`）。

## 2. 波次

| 波次 | 内容 | 门 | 状态 |
|---|---|---|---|
| 波 A（本波） | A1 真窗口呈现 + A2 帧流水化 + A3 游戏循环 + A4 动态场景 + A5 FG 接线 + A6 验收门 | 五门 + anchor_check + soak | 已实现并验收（§8 close-out 实测 facts） |
| 波 B+（后续期） | G32 画面完整期内容（HZB/ReSTIR/slab 接线、纹理管线、蒙皮动画、GI 默认档、BistroExterior） | 后续期立项程序 | 未立项 |

## 3. 波 A 实现面（A1~A5 交付物）

- **A1 真窗口呈现**：harness `src/rurix-render/src/bin/g31_window_present.rs`（g14_3_lane_body 逐字共享统一四 pass TSR 车道 + DisplayPipeline SDR+aces13 device 编码 + `vk::ExternalImagePresent` win32 真 swapchain）；门 `g31.waveA.present`。
- **A2 帧流水化**：submit/collect 分离 in-flight 1/2/3 臂 A/B，确定性协议（固定 seed digest 锚）零破坏；门 `g31.waveA.pipelining`。
- **A3 游戏循环最小面**：`--auto-move <orbit|dolly>` 确定性轨迹 → 相机逐帧 uniform（jitter/曝光 `--ev100-ramp`）进生产车道；resize/alt-tab 健壮（swapchain era 重建）；门 `g31.waveA.gameloop`。
- **A4 动态场景通路**：`--dyn-demo` refit/rebuild 双臂逐帧 64B 实例增量，静态回归锚 = g14 Stage A 锚同格对拍；门 `g31.waveA.dynscene`。
- **A5 FG 生产接线**：`--fg <off|x2|x3>` G26 device kernel（`kernels/g26_framegen.rx`）接线 + host 金标准对拍臂维持 + presented/real 双口径分离（生成帧禁入真实渲染帧率口径）；门 `g31.waveA.framegen`。

## 4. 波 A 验收门（A6，本波）

1. 守卫套件七条全跑（check_structure/check_schemas/check_number_ledger/check_guardrails/check_contribution/trace_matrix --check/budget_eval）。
2. 五门复跑（--selftest + --gate 全 PASS）。
3. 零降级回归锚三面：Stage A digest 锚 18/18 canonical 160 帧重跑零漂移（ci/g31_wave_a_anchor_check.py）+ G16plus M-g 18/18 canonical 复跑（ci/g16_absolute_quality_closure_smoke.py --gate，UE 参照臂按在案锚不重跑）+ G17-MD-F1 焦点格新鲜真跑 frame_ms/ratio 对照（诚实红不恶化，ratio ≥ 在案 0.960479）。
4. soak：g31_window_present --auto-move ≥10000 帧（或 ≥30min 墙钟取先达），零崩 + validation 静默 + leak 账本零 + digest_seq 确定性抽查（ci/g31_wave_a_soak.py）。
5. 四件套落盘（本文件族）+ §8 close-out 实测 facts。

## 5. 编号纪律

CI 数字步骤零消费（落盘前实测 CI_step.next_free=525 维持）；RFC/RXS/RD/U/SG/MR/D/RX_error 共享段零消费（波 A = 既有 RFC-0030/0035/0036/0043 语义面实现，零新语义面）；evidence 前缀 `g31_wave_a_anchor_check_` / `g31_wave_a_soak_` 经 check_schemas 三处纯追加登记。
