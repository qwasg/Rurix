<!-- Assisted-by: Cursor Grok 4.6（G16plus 治理立项） -->
# G16PLUS_RECORD — G16plus 强制收口画质战役记录

> **性质**：G16 延续波（G14plus / G14_CONTRACT §7 裁决 7 同构）只追加记录。事实源仍为 [G16_CONTRACT.md](G16_CONTRACT.md)。
> **授权字面**：用户 2026-08-24「一次性完美完成G16」+「强制收口画质，不然不算完成」。
> **G16.0 不可变 ref**：`9851915150ec07f13ab3f9d8e298688844720bcc`（tag `g15-closed`）。
> **G16.1~G16.5 基线**：§8.2 已绿（M-a~M-d 步骤 284~287）；本记录不回写。

## 1. 立项授权

| 项 | 字面 |
|---|---|
| 载体 | G16.x 延续波（G16.6~G16.10），不另立 G17 顶替「完成 G16」 |
| 退出条件 | M-g `met_count==18` ∧ 阈仍为程序产 `p100×2.0` ∧ soak≥1800s ∧ close-out READY |
| 禁项 | 手写/放宽阈；回写 G13/G15 冻结表；改 `g14_3 --gi off` 默认臂；混入异己 src；RFC-0031 Approved 前改 GI 冻结面；拉 G15-MD-F1 / NGX |
| 异己面 | `.tmp/g16plus_alien_archive/`（restir/sdf/hzb/smrt/ssr/ktx2_read）零消费 |

## 2. 波序

| 波 | 门 | 依赖 |
|---|---|---|
| G16.6 治理 | RFC-0031 + D-409 + MAP 附录 A + 步骤 288~292 materialize | G16.1~G16.5 已绿 |
| G16.7 诊断 | Rurix `--gi off` vs UE Lumen-off；能量目标入 evidence | RFC Approved |
| G16.8 cornell | M-e `--gi on` 面光/emissive NEE + ≥2 反弹 | 诊断完成 |
| G16.9 bistro | 多灯 NEE + 屏幕探针/世界缓存填光 | cornell 间接光机核非近零 |
| G16.10 再审 | M-g 18/18 | M-e/M-f 绿 |
| soak/close-out | M-h / 6a / 6b | **仅 M-g 绿** |

## 3. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 首版：立项授权 + 波序。 |
| v1.1 | 2026-08-24 | G16plus 收口：M-e 8/8 绿；M-g 18/18（阈 p100×2.0）；soak 56 迭代 1835.136s 零失败；close-out 八 facts READY；契约 status active→closed。M-c 历史 0/18 未改写。 |
