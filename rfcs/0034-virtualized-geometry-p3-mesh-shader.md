# RFC-0034 — 虚拟化几何 P3 Mesh Shader 第三光栅路径

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0034（next_free=34） |
| 状态 | Agent Approved（评估程序） |
| 承接 | G18.5 M-g |

## 摘要

评估 `VK_EXT_mesh_shader` 设备面，实现 VisBuffer mesh shader 第三光栅路径或 no-go/defer 留档。像素零差判据：与 VS 光栅 fallback 输出 digest 一致。

## 重判记录（只追加）

| 日期 | 事件 |
|---|---|
| 2026-08-24 | G18.5 M-g 终态 = no-go（evidence/g18_m_g_virtualized_geometry_p3_*.json）；重判条件 = G19+ HZB/cluster P4 触发条件齐备 |
| 2026-08-24 | G20.3 M-c 重判（RFC-0037 §1.4 程序）：重判条件两半核验——HZB 半边**已兑现**（geometry/hzb.rs host 参考臂 + g20_hzb_probe 800 rect 零假阳性双约定绿件 evidence/g20_hzb_probe_20260824T162347Z.json）；cluster P4 半边**差距闭集 4 行全 open**（milestones/g20/g20_cluster_streaming_p4_gap.json，disposition=defer——cluster 页流送依赖 HZB device 化与剔除 pass 反馈链）。mesh shader HW 管线性能差 measured 证据仍缺（HW 路径零实现 ⇒ 无 A/B 可测面）。裁决 = **maintain-no-go**：VS 光栅唯一 fallback 维持（字面 0-byte）；重判条件顺延 = cluster P4 差距闭集清零 + HZB device 化落地后按只追加程序再判 |
| 2026-08-25 | G27.2 M-b 重判（RFC-0044 §2 程序，三项机器盘点闭集）：①HZB device 化半边**本期兑现**（kernels/g27_hzb_reduce.rx + g27_hzb_test.rx device kernel 真跑对拍绿件，M-a 门 g27.p0.m_a.hzb_device_kernel）；②cluster P4 清零半边**四行仍全 open**（g20 差距表实测 + milestones/g27/g27_cluster_p4_rejudgment.json 四行重判维持 open，P4-2 依赖解除事实登记≠兑现）；③mesh shader HW 管线性能差 measured 证据**零命中**（searched-paths manifest 三 pattern 在档）。三项合取未全齐（①命中不得单独启动——RFC-0044 §2.3 防冒充硬线）。裁决 = **maintain-no-go**：VS 光栅唯一 fallback 维持（字面 0-byte）；重判条件顺延字面不变 |
