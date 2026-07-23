# G4 执行计划 — 子里程碑分解

> 所属契约:[G4_CONTRACT.md](G4_CONTRACT.md)(status active,2026-07-23 开工)
> 版本:v1.0(2026-07-23)
> 粒度依据:11 §7(小里程碑两级结构);本计划是工作分解,验收以契约 §4 为准,本文不重定义成功。
> **串行口径**:G4.0→G4.7 严格串行(G3.0→G3.7 先例),无并行轨、无等待点;每子期完成即一段简报归契约 §8。
> **定位口径**:把「图形 RHI 化 + Vulkan RHI + RD-035 执行面 + BLACKHOLE 生产档验收」做成 measured 工程事实;RFC-0015 伞形 Approved 先于一切实现 PR;BLACKHOLE 修复先于测量。

---

## 0. 总览与依赖

```mermaid
flowchart LR
    g40[G4.0 治理包+台账校准] --> g41[G4.1 RFC-0015 Draft→对抗评审→Approved]
    g41 --> g42[G4.2 图形 RHI 化<br/>首切片 artifacts v2]
    g42 --> g43[G4.3 RD-035 三项]
    g43 --> g44[G4.4 Vulkan RHI 通道<br/>RD-031 处置]
    g44 --> g45[G4.5 C ABI v2 判档]
    g45 --> g46[G4.6 BLACKHOLE 收尾]
    g46 --> g47[G4.7 close-out]
    g42 -. engine_host v3 真实硬需求是 G4.5 判档输入 .-> g45
    g42 -. artifacts v2 切片是 G4.4 通道前提 .-> g44
```

| 子里程碑 | 时长(估) | 交付物映射 | 阻塞关系 / gating |
|---|---|---|---|
| G4.0 | ~1 天 | D-G4-1(契约四件套 + number_ledger v1.13 校准 + reserved_in_flight[G4]) | 无(开工件);**零语义实现、零条款头、零 workflow 步骤、零预算条目** |
| G4.1 | ~3–5 天 | D-G4-2(RFC-0015 Draft→D-409 对抗性评审→Agent Approved) | 依赖 G4.0;RFC Approved 先于任何实现 PR;失败测试先行(步骤 76+ 脚本与机制代码在 RFC 合入时点 main 不存在 = RED) |
| G4.2 | ~3–4 周 | D-G4-3(图形 RHI 化全栈:条款 RXS-0270 段 + artifacts v2 前置切片 + rhi.rs/vk.rs 执行面 + uc05 图形 demo + engine_host v3 + 步骤 76~78) | 依赖 RFC-0015 Approved;**关键路径 + 本期最重单段**;首切片 artifacts v2(契约 §7 ④:图形 pass device 出图的工程前置,RD-031 codegen 本体) |
| G4.3 | ~2 周 | D-G4-4(RD-035 三项:别名复用+峰值计数器 I10 measured / 重排+并行调度+新拦截项 / RXS-0262 const 容量 + reject 语料) | 依赖 G4.2(图形面矩阵与执行器就位后扩执行模型);三项彼此独立可分批兑现分批留痕 |
| G4.4 | ~1 周 | D-G4-5(Vulkan RHI 通道本体:compute+graphics 双腿 .rx 单源经 artifacts v2 通道 device 真跑 + RD-031 处置) | 依赖 G4.2 artifacts v2 切片(契约 §7 ④);前置核实已留痕(emit_gpu_artifact_globals 在 main,src/rurixc/src/codegen.rs:99/1028) |
| G4.5 | ~2–3 天 | D-G4-6(C ABI v2 判档留痕;条件臂兑现或 RD-036+ 登记) | 依赖 G4.2(engine_host v3 真实硬需求为判档输入);两种结局均合法 |
| G4.6 | ~1 周 | D-G4-7(BLACKHOLE realtime 归因 + 修复 + 30fps measured + REALTIME_OK + 帧对照) | 依赖 G4.5(串行纪律);修复先于测量;判档 Direct/Mini 执行期定(零新语义) |
| G4.7 | ~2–3 天 | D-G4-8(close-out 终审 + 基准切换 + g4-closed tag + RD/SG 处置) | 依赖 G4.2~G4.6 全部门;agent 自主签署 |

## 1. G4.0 治理包(本子期即本 PR)

- 契约四件套(G4_CONTRACT / G4_PLAN / CI_GATES / g4_budget.json 空壳)。
- number_ledger v1.13 校准:RFC next_free 13→15 / MR 11→12 / RXS 0266~0269 burned 跳号 next_free 266→270 / D 408→410;reserved_in_flight[G4] 登记(claim 全表见契约 §7 ⑤);revision_log 追加。
- 验收:check_number_ledger / check_schemas / check_structure PASS;milestones/g4/ 与 number_ledger 之外全 0-byte。

## 2. G4.1 RFC-0015 伞形

- 四章:A 图形 RHI 化(库面扩 raster/mesh pass 类型 + 采样/bindless/present 库化 + 自动 barrier 库面语义 + export 面;薄映射 std::gpu lang items + G3 既有条款面 RXS-0223~0248/RXS-0197/0198/0225/0235,默认零新语法)/ B RD-035 执行面三项 / C artifacts v2 + .rx 单源 Vulkan RHI 通道(RD-031)/ D C ABI v2 条件臂(repr(C) struct 按值 + 回调指针,判档成立才落实现,G-EA1-3/RXS-0249 条件分支先例)。
- 体例母本 rfcs/0013(伞形五章)+ rfcs/0014(单 RFC 双面);§9 未决问题预记录;§9.1 对抗性评审记录段(D-409:评审 provenance ≠ 起草 provenance,逐条 disposition)。
- Approved 合入先于一切实现 PR;条款 commit 序在实现 commit 前。

## 3. G4.2 图形 RHI 化(主面)

- **首切片 artifacts v2**:@__rx_gpu_spirv 段 + @__rx_gpu_artifacts blob v2 bump(Spirv 变体)+ emit_gpu_artifact_globals 扩展 + codegen 单测/golden;NVIDIA PTX/cubin 路径逐字节不变(RXS-0209 IR1 纪律)。
- **条款**:RXS-0270 段(spec/rhi.md 扩章)——图形 pass 类型面(raster/mesh;RT pass 类型面可条款化但 DXIL RT 腿维持 RD-034 blocked)/ 自动 barrier 覆盖图形 pass(复用 graph.rs 推导单源,P-11)/ export 面扩展。
- **执行面**:rhi.rs 图形 PassSpec + vk.rs RHI 图形执行入口(replay 推导产物,既有 run_*_offscreen 入口 0-byte 语义);.rx 侧薄映射 lang items(resolve/mir_build 加性)。
- **demo 与嵌入**:apps/uc05-rhi 图形 demo(≥1 raster + ≥1 mesh pass 出图,像素判据同 G3 headless readback 纪律);engine_host v3(C++/D3D12,engine_host v2 母本升级**新增文件**,v2 资产 0-byte)链接 rurix_rhi 图形导出面,三方数值精确相等(.rx RHI / D3D12 宿主 / host 参考)。
- **CI**:步骤 76(图形 RHI 冒烟,device)/ 77(图形不变量门,host)/ 78(引擎嵌入 v3,device);生成头 CI 再生成逐字节比对;零 .rs 审计维持。
- 像素对照工程注:跨后端(rurix Vulkan vs 宿主 D3D12)逐像素比较有光栅化器边缘差异风险——用确定性无边缘依赖用例(全屏程序化色 / 中心化图元),判据设计进 RFC 章 A。

## 4. G4.3 RD-035 三项

- **① transient 别名复用 + 执行期峰值计数器**:rhi.rs 生命期区间(首写→末读)不重叠的 transient 资源别名复用分配器;执行期峰值并发存活字节计数器 device 采集;I10 自 report_only 升 measured(峰值 < 声明容量可 device 见证);矩阵 I10 note/tiers 同步,三方一致性维持(步骤 75 机制扩)。
- **② 依赖驱动重排 + 并行调度**:依赖 DAG 拓扑重排(独立 pass 可换序/并行),RXS-0239 pass 边界 happens-before 语义在新执行模型下的条款化修订(同 PR);新增确定性拦截项入不变量矩阵(重排后违依赖 = 装配期确定性拒,漏拦即红)。
- **③ RXS-0262 const 泛型定长容量**:.rx 侧 const 泛型实参接通评估(RD-007 turbofish const 实参面——若需 RD-007 跨层接通,按 10 §3 判档随 RFC-0015 章 B 落);编译期越界拒 + conformance/uc05/reject 新语料。
- **CI**:步骤 79(RD-035 执行面门,host+device 段)。

## 5. G4.4 Vulkan RHI 通道(RD-031 承接)

- 前置核实留痕(开工已核:artifacts blob / emit_gpu_artifact_globals 在 main)。
- 通道本体:.rx 单源 compute RHI 经 artifacts v2 SPIR-V 段走 Vulkan 执行(rxrt_rhi_* 现为 CUDA-only → Vulkan 变体),graphics 腿复用 G4.2 路径;复用 G3 vk 运行时底座(run_compute/run_graphics_offscreen_v2/run_mesh_offscreen/run_graph_offscreen 既有入口 0-byte 语义)。
- device 真跑数值对照(Vulkan 侧 vs host 参考)+ spirv-val;RD-031 处置(close / 收窄)留痕。
- **CI**:步骤 80(Vulkan RHI 通道冒烟,device)。

## 6. G4.5 C ABI v2 判档

- 输入 = engine_host v3 图形嵌入的真实硬需求(嵌入面签名是否需要 repr(C) struct 按值 / 回调指针)。
- 判档(10 §3,争议向上取严):成立 → RFC-0015 章 D 臂条款先行 + ABI 往返 device 真跑 + RED 三路;不成立 → RD-036+ 登记存续(RD-009 close 注先例)。判档依据留痕契约 §8。

## 7. G4.6 BLACKHOLE 收尾

- **归因**(先于修复):rxp_create Shim(-2147467263)=0x80004001 E_NOTIMPL 精确归因——feature 链 `rurix-d3d12/real-shim ← rurix-rt/d3d12-interop-real ← rurix-rt-cabi/present-real` 与 shim 源(src/rurix-d3d12/shim/rx_d3d12_shim.cpp,371 行)实际覆盖面的核对;归因证据归 evidence/。
- **修复**:按归因落(real-shim 接线或 shim 面补齐);禁绕过禁静默降级;G3.2 present 既有路径(步骤 61)零回归。
- **测量**(修复后):30fps measured(BENCH_PROTOCOL:锁频 + 三次 trimmed mean + 环境画像)+ REALTIME_OK 六项物理自检 + 帧对照(offline 144 帧 vs realtime 帧)。
- apps/blackhole 入库姿态随本相定(现 untracked;验收对象与证据应可复核)。
- **CI**:步骤 81(BLACKHOLE realtime 验收冒烟;device,判据含 REALTIME_OK + 帧对照;30fps 数值 evidence 面不进硬门,EA1 冷启动先例)。

## 8. G4.7 close-out

- 全量回归冻结(契约 G-G4-8 清单)+ 门终审表 + RD-031/035/新 RD 处置 + SG 复评 + status flip + 基准切 g4-closed + annotated tag + ledger 收口 revision。

## 9. 风险

| # | 风险 | 对策 |
|---|---|---|
| R1 | artifacts v2 blob bump 触碰既有 PTX 装载路径 | RXS-0209 IR1 纪律:NVIDIA 路径逐字节不变;v1 blob 零改动继续工作;版本门 reject 路径既有(RFC-0011 §4.7) |
| R2 | 跨后端像素对照(Vulkan vs D3D12)边缘差异 | 确定性无边缘依赖用例;判据设计进 RFC 章 A;失败即换用例不降判据 |
| R3 | RD-035 ③ 触发 RD-007 跨层接通(turbofish const 实参) | 按 10 §3 判档随章 B;若接通成本越界,收窄登记留痕(RD-035 三项独立可分批) |
| R4 | 重排/并行破坏 RXS-0239 happens-before 承诺 | 语义修订条款先行同 PR;新拦截项漏拦即红;独立 pass 才允许换序 |
| R5 | BLACKHOLE shim 面缺口大于预期(C++ shim 大改) | 归因先行定量;G3.2 present 路径为已验证参照;修复面判档向上取严 |
| R6 | engine_host v3 D3D12 mesh 管线和 .rx Vulkan mesh 语义错位 | mesh 用例程序化生成;三方对照 host 参考为金标准 |
| R7 | LF/CRLF 字节纪律 | 新文件 LF + 尾换行;提交前逐文件字节核 CR + 尾字节(git numstat + 二进制读) |

## 10. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-23 | 初版(G4.0 治理包同 PR) |
