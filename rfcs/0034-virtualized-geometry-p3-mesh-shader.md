# RFC-0034 — 虚拟化几何 P3 Mesh Shader 第三光栅路径

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0034（next_free=34） |
| 状态 | Agent Approved（评估程序） |
| 承接 | G18.5 M-g |

## 摘要

评估 `VK_EXT_mesh_shader` 设备面，实现 VisBuffer mesh shader 第三光栅路径或 no-go/defer 留档。像素零差判据：与 VS 光栅 fallback 输出 digest 一致。
