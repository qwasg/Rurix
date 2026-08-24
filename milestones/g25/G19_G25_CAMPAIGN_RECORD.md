<!-- Assisted-by: Cursor Agent（G19-G25 七期串行战役收官） -->
# G19-G25 七期串行战役总结报告（2026-08-24 单日战役）

> **授权**：用户 2026-08-24 指令「帮我一次性完成G19-G25」（七期全周期串行授权，波次内可并行、里程碑间不越级）。
> **事实源**：各期 `milestones/g<N>/G<N>_CONTRACT.md` §8 + `G<N>_P2_DECISIONS.md` + `registry/`；本报告只作汇总镜像，不改任何原始锚。

## 1. 战役总览

| 期 | 主题 | 关键终态 | soak | tag |
|---|---|---|---|---|
| G19 | 帧生成独立层兑现 | FG/MFG host 参考臂 **implemented**（×2/×3/×4 逐帧优于 frame-hold，min_margin +0.0166；真实渲染帧率口径 0-byte）；vendor 三臂 rejected/not-available；RD-045 观察窗 12/12 零漂移 maintain-open | 1832.1s/63 迭代/0 失败 | `g19-closed` |
| G20 | 虚拟化几何 P4 | HZB host 参考臂 **implemented**（双约定 800 rect 零假阳性）；cluster P4 defer 四行闭集；M61 **maintain-no-go**；M98-l4 维持三级链 | 1845.0s/67/0 | `g20-closed` |
| G21 | 光照 P3+ 深化 | ReSTIR 高档 reservoir **implemented**（方差收益 **15.955×** + 时域 **7.27×** + 无偏 3σ）；M52 SER **maintain-defer**（capability 实测 available + workload 未命中）；RD-040 五分项闭集；RD-034 复查维持 blocked | 1854.7s/69/0 | `g21-closed` |
| G22 | 材质/流送/时域 | slab 能量守恒参考臂 **implemented**（白炉 dev=0 + 恒等式 1e-15）；SVT/KTX2 defer 闭集；Work Graphs **not-available 实测** + DGC available 实测；FSR maintain | 1846.2s/69/0 | `g22-closed` |
| G23 | 物理平台深化 | Jolt 5.6 采纳臂 **maintain-5.3**（三件条件 1/3 机器取证）；M127 **maintain**（两半实测未命中）；RD-042/043 四轨 maintain-observe；RD-044 三分项闭集 | 1836.9s/69/0 | `g23-closed` |
| G24 | 呈现与尾门清理 | 毛发 OIT **maintain card/mesh**；HDR **maintain-SDR**（设备半实测 not-available）；BistroExterior 维持双场景闭集（工具链三缺实测）；SAFE-GPU defer-to-G25+；历史 RD 十一条清册**零 close 诚实** | 1847.4s/69/0 | `g24-closed` |
| G25 | 全量商用终审收官 | 画质终态**维持达标**（表面 10 项 0-byte 机核 + 加性零接线）；fps **17/18 诚实红终判**（焦点格 ratio 0.856326 + 新鲜单测 3.5520ms）；全链零降级；承接锚归档闭集（G26+ 法定输入） | 1840.4s/69/0（四探针轮换 13 次） | `g25-closed` |

## 2. 真实实现件（战役期新增，全部单测/探针绿）

| 模块 | 内容 | 判据 |
|---|---|---|
| `temporal/framegen.rs` + `g19_frame_gen_probe` | FG/MFG 帧插值（mv 双向 warp + 遮挡感知混合 + M-cap 时域语义预留）| 6 单测 + 三档逐帧 SSIM(interp)>SSIM(hold) + 双跑位级 + 两口径账目恒等式 |
| `geometry/hzb.rs` + `g20_hzb_probe` | HZB 层级深度金字塔遮挡剔除（farther-of 归约 + ≤2×2 窗保守测试，双约定） | 5 单测 + 800 rect 零假阳性硬不变量 + 剔除率 231/800 |
| `gi/restir_reservoir.rs` + `g21_restir_probe` | ReSTIR DI 高档 reservoir（WRS/RIS 无偏权 + 时域合并 M-cap） | 5 单测 + 20k trial 无偏 3σ + 方差收益 15.955× + 时域 7.27× |
| `material/slab.rs` + `g22_slab_probe` | Substrate 类双层 slab 能量守恒闭合（解析闭式 + 级数+尾和恒等式） | 5 单测 + 16641 样本白炉 dev=0 + 恒等式 1e-15 |

cargo test -p rurix-render --lib：**486 passed 0 failed**（战役新增 21 测全绿）；harness 统计单测 163 passed。

## 3. 机器取证重判（诚实终态，零冒充）

- **SER capability**：vulkaninfo 实测 `VK_NV/EXT_ray_tracing_invocation_reorder` **available**（三 token 取证）——workload 半边（RT pipeline/SBT 车道）未命中 ⇒ maintain-defer。
- **Work Graphs**：`VK_AMDX_shader_enqueue` **absent** 实测；DGC 三扩展 available + dgc.rs M102 现面。
- **HDR 设备半**：表面色彩空间 HDR token 全 **absent** 实测 ⇒ maintain-SDR。
- **BistroExterior**：fbx2gltf/assimp/blender PATH 三缺 + 独立源资产缺实测 ⇒ 维持双场景闭集。
- **Jolt 5.6**：sys56 评估臂 cargo check 新鲜绿 + A/B 绿件盘点；生产切换需求证据三类全空 ⇒ maintain-5.3。
- **RD-034**：meshrt 探针真跑复查——spirv-cross 仍拒 raygen ⇒ 维持 blocked。
- **RD-045**：G19.3 观察窗 canonical 160 帧 **12/12 中锚零漂移** + 六期 soak 全零失败 ⇒ maintain-open（backfill 三件未全齐不冒充 close）。

## 4. 治理面账目

- **RFC**：RFC-0036~0042 七份（各经对抗评审 Agent Approved）+ RFC-0034/0035 只追加重判记录 + rfcs/README §5 台账七行（0033~0035 漏登补登）。
- **CI 步骤**：333~444（112 步：七期 × 16 步）全部落盘前实测顺位领取 + workflow 同批接线；ledger v1.165~v1.178 十四行修订。
- **registry**：deferred.json 只追加 history 19 条（RD-034/040/041/042/043/044/045 + 历史清册十一条 + RD-045 观察窗）；条目零删除零改判；共享 D/U/RD/SG 段零消费。
- **P2 穷举**：七期 14+14+13+11+11+9+7 = 79 行闭集零空行；G18 承接池（九行 defer + 重判锚）全量消化清零。
- **G26+ 法定输入**：`milestones/g25/g25_campaign_handover_registry.json`（15 期行 + RD 八条 + 清册十二行引用）。

## 5. 商用终审终态（G25 定盘）

- **画质**：G18 M-d 商用画质终审**达标**终态维持有效（战役期画质表面 10 项 0-byte 机核 + 加性四模块零接线证明——重渲无信息增量）。
- **性能**：**17/18 诚实红终判**（焦点格 bistro-interior/t100/dlss_sr ratio 0.856326；G15「物理不可达维持未达标登记」兜底同源；顺延锚 = NGX 分解 profiling / UE 侧插桩）。
- **确定性**：Stage A 18 格 digest 锚全战役零漂移（G19.3 12/12 + 六期 soak + G25 M-a/M-b 0-byte 机核链）。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-08-24 | 战役收官首版（G25 soak 数字随 close-out 落档回填）。 |
