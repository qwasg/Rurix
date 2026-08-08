# G8.8a soak 语义假绿清零 NOTES（2026-08-08）

## 清零对象（已证伪）

基线：`.a3_evidence/01_baseline_fake_soak.txt`（100 帧 + sleep 凑 `soak_seconds=20.000`，
`validation_messages`/`device_lost_count` 字面量 0，`rss_final` 恒 0，exit=0）。

三处假绿：

1. **sleep 充墙钟**：`apps/uc08-physics/src/main.rs::run_soak` 跑完 `min_frames` 后
   `thread::sleep` 补齐 `min_seconds`——「30 分钟稳定性 soak」实为几分钟真实负载 +
   剩余时间空转。
2. **字面量 0 充 device 零错门**：host soak 无 Vulkan validation/device-lost 面，
   却输出 `validation_messages:0, device_lost_count:0` 并被 smoke 当硬门判 `==0`。
3. **假 RSS 采样**：`process_rss_bytes()` 在 Windows 恒返回 0，冒充已采样。

## 新诚实语义（本版起生效）

- **墙钟只来自真实帧循环**：`run_soak` 无任何 sleep；循环跑到
  `frames ≥ min_frames 且 elapsed ≥ min_seconds` 双阈值同时满足，不足就继续跑真实帧
  （帧索引按 `min_frames` 回绕，补帧仍是每帧真实物理步 + 十五 pass 全管线）。
  输出 `active_frame_seconds == soak_seconds`、`sleep_seconds` 恒 0（构造保证）。
- **smoke 交叉核验**：`ci/g8_stabilization_soak.py::judge_soak` 用 smoke 侧外测墙钟
  核对——外测 < 自称 seconds − 2s 判红（谎报时长）；缺 honesty 字段判红。
- **subject=host-soak**：host soak 不再输出/判定 validation_messages、
  device_lost_count；schema 中两键降为可选遗留（旧 evidence 仍可校验），
  新 evidence 不写这两个键。
- **RSS 未门禁**：无 Windows 采样器（不引 winapi），二进制与 evidence 均不再报
  假采样；notes 声明「rss 未门禁」。泄漏门如未来需要，须先落真采样器再立门。
- **checks 新增** `soak_no_sleep_padding`（可选键，新 evidence 恒写）。

## 判据效力对照

| 项 | 旧判据 | 新判据 |
|---|---|---|
| frames ≥ 10000 | 硬门（不变） | 硬门 |
| seconds ≥ 1800 | 硬门，但可被 sleep 凑 | 硬门，只能由真实帧循环达成 |
| validation/device_lost == 0 | 伪硬门（字面量 0） | 移除（host-soak 无 device 面） |
| RSS | 假采样，未门禁 | 移除假采样，notes 声明未门禁 |
| 无 sleep/墙钟诚实 | 无 | 硬门（selftest A1~A3 红臂证伪） |

## legacy evidence 兼容

2026-08-08 前的旧格式 evidence（无 honesty 字段）：sleep 凑时只会把 seconds 顶到
`min_seconds` 整值（如 1800.000），**真实跑超时**（seconds ≥ min_seconds+30，如
`g8_wave8a_soak_20260806T095945Z.json` 的 10000 帧实测 2079.47s）sleep 造不出，
予以兼容接受；否则判红，需重跑 `--gate`。

## 验证入口

- 反假绿 selftest：`py -3 ci/g8_stabilization_soak.py --selftest`
  （A1 复现基线假绿→红；A2 sleep>0→红；A3 外测墙钟戳穿谎报→红；A4 诚实样本→绿；
  A5 legacy 两臂）。
- 短跑证伪：`uc08-physics --soak --min-seconds 20 --min-frames 100` 在新二进制下
  产出 ~194 真实帧/20.0s/sleep=0（旧二进制为 100 帧+sleep 凑 20.000）——
  见 `.a3_evidence/02_new_binary_no_sleep_demo.txt`。
- 全量门：`py -3 ci/g8_stabilization_soak.py --gate g8.wave.8a.soak`
  （21 门回归 + 真实 ≥1800s/≥10000 帧 soak + budget --strict；约 30~35 分钟）。
