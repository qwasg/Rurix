<!-- Assisted-by: Cursor Agent（G24.1 治理波） -->
# RFC-0041 — 呈现与尾门清理：M114-strand/M118-hdr-cal/G10-N6 机器取证重判 + SAFE-GPU 处置 + 历史 RD 清册程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0041（落盘前实测 ledger RFC next_free=41 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g24/design/rfc0041_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（清册重判程序留档；零新实现语义面） |
| 承接 | G24.2 M-a/M-b + G24.3 M-c/M-d |
| 上游 | G23_P2 §1 四行（G18 承接池尾）、历史 open RD 十一条 |

## 1. 摘要

1. **毛发精确 OIT 重判（M-a）**：两半——M120 七算法 OIT benchmark measured 裁决数据在案性（g9_m120 绿件只读盘点：linked-list 精确档与排序真值 diff=0 + 帧时/内存曲线）+ strand 档生产需求面（压测闭集毛发资产存在性核验）；数据半命中 + 需求半未命中 ⇒ maintain-card-mesh。
2. **HDR 标定重判（M-b）**：两半——HDR 设备面实测（vulkaninfo 表面色彩空间枚举：HDR10_ST2084/BT2020 token 取证）+ HDR 资产/产品需求面（压测闭集 SDR 全量验证面现状）；设备半以实测为准 + 需求半未命中 ⇒ maintain-SDR。
3. **BistroExterior 复查（M-c）**：FBX2glTF pin 与替代转换臂（assimp/Blender 管线）工具链在树性实测 + BistroExterior FBX 源资产在树性核验；工具链/资产任一缺 ⇒ maintain 双场景闭集，登记 `g24_bistro_exterior_recheck.json`。
4. **SAFE-GPU 处置 + 历史 RD 清册（M-d）**：SAFE-GPU 独立期立项判据核验（G9~G23 零交付 + 独立期资源窗不存在 ⇒ maintain-defer-to-G25+ 或关闭评估留痕）；历史 open RD 十一条（RD-007/011/012/014/015/026/027/030/032/033/036）逐条重判闭集 `g24_legacy_rd_registry.json` + 逐条 history 只追加——backfill 条件字面逐条核验，成立 ⇒ close、未成立 ⇒ maintain（附最新核验事实）。
5. **out-of-scope**：strand 精确 OIT 实现、HDR 显示管线实现、BistroExterior 转换执行、SAFE-GPU 平台实现。

## 2. 不变量

- 旧门只读消费（g9_m120/g9_m118 绿件禁 --gate 重跑）；历史 RD status 翻转仅当 backfill 字面成立。

## 3. 终态程序

M-a~M-d 真跑 evidence 落档后本 RFC 终态 = 四行重判记录 + 清册闭集字面；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G24.1 起草；对抗评审后 Agent Approved。 |
