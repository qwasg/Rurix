<!-- Assisted-by: Cursor Agent（G22.1 治理波） -->
# RFC-0039 — 材质/流送/时域 P3+：slab 能量守恒闭合 host 参考臂 + SVT/KTX2/Work Graphs/FSR 分项处置程序

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0039（落盘前实测 ledger RFC next_free=39 顺位领取） |
| 状态 | Agent Approved（经对抗评审 milestones/g22/design/rfc0039_adversarial_review.md，D-409 对抗性评审要求程序） |
| 判档 | Full RFC（材质分层新语义参考面；渲染器库面零新语言语义条款，G5 先例） |
| 承接 | G22.2 M-a/M-b + G22.3 M-c/M-d |
| 上游 | RD-041（材质/流送/时域 P3+ 长线）、G21_P2 §1 |

## 1. 摘要

1. **slab 分层材质语义参考面（本期实现）**：`rurix_render::material::slab`——双层 slab（coating 无损档 + base）方向-半球反照率层级的无穷弹跳解析闭式 `R_total = r_c + t_c²·a_b/(1−r_c·a_b)`。Substrate 类分层材质的能量守恒混合语义地基。
2. **能量守恒硬不变量（程序产判据禁手写）**：白炉恒等（a_b=1 ⇒ R=1）+ 全参数域 R ≤ 1（能量不增生）+ 对 a_b 单调 + 闭式↔级数+解析尾和恒等式（数学精确，浮点级容差 1e-9）+ lerp 连续性 + 双跑位级。
3. **SVT 分项处置（M-b）**：streaming/ 页式现面（RXPL 页 ABI + 反馈驱动）vs 虚拟纹理页表目标差距闭集 `g22_svt_gap.json`。
4. **KTX2-BasisU 分项处置（M-c）**：G11.3 DDS 转码链现面盘点 + BasisU 转码器差距/收益登记 `g22_ktx2_disposition.json`（vendor C++ 集成面 + 通用转码收益 vs 现 DDS 直通链）。
5. **Work Graphs + FSR 分项重评（M-d）**：Work Graphs Vulkan 车道设备实测（AMDX shader_enqueue 扩展枚举，NVIDIA 预期 absent）+ GPU-driven 提交现面盘点（dgc.rs M102 DGC 抽象层 + 设备 DGC 三扩展实测）+ FSR 3.1.5 第二超分臂重评（无新版 SDK 在树 ⇒ maintain）。
6. **out-of-scope**：slab device kernel/侧表集成、SVT 页表实现、BasisU vendor 集成（各附承接锚）。

## 2. 不变量

- material/closure 单层生产面与 G11.3 DDS 转码链 0-byte；`rurix-render` 维持 `#![forbid(unsafe_code)]`。
- 阈值零手写：白炉/单调/恒等式均解析级程序产。

## 3. 终态程序

M-a~M-d 真跑 evidence 落档后本 RFC 终态 = approved-implemented（slab 参考臂）+ 三分项 disposition 字面；争议时按只追加程序重判。

## 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | G22.1 起草；对抗评审后 Agent Approved。 |
