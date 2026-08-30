# Rurix 渲染器产品说明（交付默认档与预期行为）

> 所属：G37 商业化收官战役 W5（渲染器文档面刷新）。本文承载 DEFAULT_FLIP_PLAN §4 登记的
> 「产品说明」条目——交付默认画质档、自动曝光预期行为、帧时与帧率口径承诺。定位 = 产品面
> 承诺与预期行为的单一落点；工程细节与在案数字**引用不复制**，以姊妹篇为准：
> [integration_guide.md](integration_guide.md) · [feature_matrix.md](feature_matrix.md) ·
> [performance_tuning.md](performance_tuning.md) · [support_policy.md](support_policy.md)。
> 纪律：数字一律引在案 measured（RTX 4070 Ti + Vulkan 本机真跑），不新造；依赖 W4 整批重锚的
> 终值以「见 W4_ANCHORS」占位（`artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json`），落值后
> 按修订程序追加、历史行不回写。

---

## 1. 交付默认档 = `--quality full`（十九臂画质终态）

- `g31_window_present`（真窗口呈现车道）**缺省即 full**（G37 W4 默认翻转，2026-08-30 执行在案）：
  十九臂清单与各臂语义见 [feature_matrix.md](feature_matrix.md) §8.1/§8.2。
- `--quality off` = 显式回退档（all-off 基线）；**单臂显式写法与诊断/互斥臂须显式给
  `--quality off`**（fg base 点 / hzb / slab / svt / storm / fault / cluster-lod / wp-hlod）。
- **bench 车道（`g14_3_pipeline_perf`）默认维持 off**：Stage A 18 格 digest 锚是跨里程碑回归
  事实源，任何默认翻转不触碰——产品画质档与回归契约档分离是设计内形态。

## 2. 自动曝光（AE）预期行为（协议内，非缺陷）

默认 full 内含自动曝光（presented 亮度自适应）。以下为**设计内预期行为**（DEFAULT_FLIP_PLAN §4
字面登记），不构成缺陷：

- **resize / era 重建 = AE 状态复位再适应**，约 **~12 帧半衰**——窗口尺寸变化后短暂的亮度
  回摆属预期。
- **EMA α=0.02 收敛 ~50 帧**：场景切换（相机进亮/暗区）后约 **~1s 完全收敛**——期间画面亮度
  渐变属预期。
- 判缺陷界线：收敛窗后仍持续振荡/不收敛/收敛到明显错误亮度带，按
  [support_policy.md](support_policy.md) §1 渲染正确性类报告（附 `--present-luma-out` 逐帧亮度
  序列 sidecar）。
- 旋钮（`--autoexp-key/rate/min/max`）与测量纪律见 [performance_tuning.md](performance_tuning.md)
  §3.8/§4。

## 3. 帧时口径（full 默认档）

- **生产预算锚 = 90fps ⇒ 11.11ms/帧**。
- 在案 measured：day_0829 **十六臂** soak 1955.4s 帧时 **9.54~10.70ms**，峰值 10.70 ≤ 11.11
  预算全程达标（`artifacts/day_0829_realism/final/F3_SUMMARY.json`）；G37 W0 复验 full16
  9.63ms（`w0_baseline/W0_BASELINE.json`）。
- **G37 十九臂（当前交付默认）帧时终值 = W4 整批重收割在途——占位「见 W4_ANCHORS」**。
- 绝对值随硬件不同：本文数字为开发对照机口径（measured_local 纪律），你的 SLA 以自测为准，
  自测协议 = [performance_tuning.md](performance_tuning.md) §6。

## 4. 帧率口径承诺（FG 生成帧）

- **生成帧（`--fg`）永不进入真实渲染帧率口径**：`real_render_fps` 只由真渲帧构成，
  `presented_fps`（含生成帧的呈现流畅度）独立登记——恒等式组 schema 层钉死
  （[integration_guide.md](integration_guide.md) §8）。
- 对外宣称帧率一律注明口径；presented 冒充真实渲染帧率是契约级红线（G31/G32 out_of_scope
  字面）。
- fg 合法形态 = 两点式闭集（all-off base ∪ full 预设，[feature_matrix.md](feature_matrix.md)
  §8.2）；fg × full 组合点的在案双口径数字待 W4 后登记。

---

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-30 | 初版（G37 商业化收官 W5）：交付默认档（full 十九臂/off 回退档/bench 分离）+ AE 预期行为（~12 帧半衰、α=0.02 收敛 ~50 帧、场景切换 ~1s，DEFAULT_FLIP_PLAN §4 字面）+ 帧时口径（90fps 预算 11.11ms、day_0829 在案带、十九臂终值 W4_ANCHORS 占位）+ FG 帧率口径承诺（生成帧不入真实渲染帧率） |
