<!-- Assisted-by: Cursor Agent（G19.1 治理波） -->
# G19_PLAN — 帧生成独立层兑现期执行计划

> 事实源 = [G19_CONTRACT.md](G19_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 战役定位

G19 = 「G19-G25 七期串行战役」第一期（用户 2026-08-24 指令「帮我一次性完成G19-G25」）。上游法定输入 = G18_P2_DECISIONS §1 defer-to-G19+ 九行 + G18_CONTRACT §8.7 承接锚。本期主轨 = G13-N7 帧生成 FG/MFG 独立层兑现（G18 M-h defer 的重判条件「RFC-0035 终态落档后按只追加程序重判」已命中）。

## 2. 波次

| 波次 | 内容 | 步骤 |
|---|---|---|
| G19.1 | 治理四件套 + RFC-0036 + 对抗评审 + baseline 快检 + 治理三门 | 333/334/335 |
| 互锁 | interlock READY → `implementation_status: unlocked` | — |
| G19.2 | M-a FG host 参考臂（framegen.rs + g19_frame_gen_probe）+ M-b vendor disposition | 336/338（post-interlock 实测顺位） |
| G19.3 | M-c RD-045 长窗观察（≥12 轮 --expect-digest 对拍） | 340 |
| G19.4 | M-d fps 重评窗登记 + M-e 旧门零降级（全量测试波） | 342/344 |
| G19.5 | P2 穷举 + stabilization soak ≥1800s | 346/347 |
| G19.6 | close-out 八 facts → status flip → tag g19-closed | 348 |

波聚合门 `g19.wave.{2..6}.exit` 步骤 337/339/341/343/345（奇偶交错，G18 先例）。

## 3. 实现面设计（M-a）

- `src/rurix-render/src/temporal/framegen.rs`：FG/MFG 独立层 host 参考臂——mv 双向 warp（prev 取 p−t·mv、cur 取 p+(1−t)·mv）+ 遮挡感知混合（深度/一致性权重 + 兜底最近帧采样）+ MFG ×2/×3/×4 多档（t=i/(N+1)）。纯 f32 确定性。
- `src/rurix-render/src/bin/g19_frame_gen_probe.rs`：确定性程序化动画序列（解析式 ground truth）→ 偶帧作「真渲帧」奇帧作 FG 生成 → 逐帧 SSIM(interp vs GT) 与 SSIM(frame-hold vs GT) 对照（程序产对照阈禁手写）→ real_fps 与 presented_fps 独立登记（生成帧禁计入 real）→ 双跑位级 digest。
- vendor 三臂（M-b）：FSR3-FG（external/fidelityfx-sdk-2.0.0 C++ 集成面）、DLSS-G（需 D3D12 swapchain 宿主车道）、SL-310.6.0 换版窗（external/streamline-2.10.3-ngx310.6.0）——逐臂 disposition 登记 g19_vendor_sdk_registry.json，三态均合法。

## 4. 编号纪律

治理三门 333~335 落盘前实测领取；P0/波聚合/收口步骤 post-interlock actual-next-free 顺位（预期 336~348）；RFC-0036 实测领取并登记 rfcs/README.md §5；共享 D/U/RD/SG 段零消费（RD-045 只追加 history）。
