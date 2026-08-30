# G37 W3 — 逐帧 device cut → AS 更新（TODO #77 × #89 合流窗）侦察·判档·最小实现·合入提案

> 任务：解冻 G36 五留窗之一「出帧几何冻结于装配期选层（逐帧 AS 更新归 #77/#89
> 合流窗）」的最小判档面。窗口 bin 与 lane_body 只读——本报告含合入提案，合入由
> 主 agent 执行。纪律遵守：未跑 GPU、未 `cargo build --release`、未碰
> target-night、窗口 bin/lane_body/g34/既有 kernel 与 SPV/graph 计划面/
> milestones/registry/ci 全 0-byte。
>
> 交付件：`src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs`（臂共享体,
> 1292 行）+ `src/rurix-render/src/bin/g31_frame_cut_probe.rs`（独立 harness,
> 224 行）+ `Cargo.toml` 加性 `[[bin]]` 一段。cargo check(dev) 退 0、
> `--selftest` host 腿 PASS 退 0（§6）。

---

## 1. 侦察记录（任务①–④）

> 行号 = 各读取时点工作树。**窗口 bin 在本交付期间被主线并行合入推进,其行号
> 会话内已漂移两轮**——窗口 bin 的行号仅供定位参考,以引用字面为准（vk.rs/
> render_exec.rs/lane_body 未见漂移）。

### ① 窗口 --cluster-lod 臂的装配期 cut 与 AS 建立结构

- **装配期一次 cut**：`g31_window_present.rs` L8309–8331（③.4 施加点）——
  `apply_cluster_lod(scene, &cluster_opt, init_in_w, init_in_h)`（lane_body
  L3064–3198）以**契约初始相机**逐块 `select_lod_cut_grouped` +
  `verify_cut_coverage`，然后**重建 SceneData 三角汤**（源三角升序在前、粗簇
  三角块序×簇序在后），交给既有单 BLAS 车道出帧。簇包保留在
  `cluster_ctx: Option<(ClusterLodReport, ClusterPack)>` 复用。
- **主循环只做统计不出帧**：L10061–10071 逐帧 `cluster_lod_frame_stat`
  （host cut 重算 measured，每 16 帧覆盖性机核），sidecar L10782–10820 自注
  「出帧几何冻结于装配 cut;逐帧 AS 更新归 C/E 阶段——如实登记不冒充」。
- **结构含义**：cut 变 ⇒ 三角**数量**变 ⇒ 单 BLAS 车道无法帧内换几何（会话
  AS 常驻、`FrameUpdate` 面无「重建整 BLAS」操作）——这就是冻结的机制根源。
- **RXCP 簇包**（lane_body L2605–2646）：`ClusterPackBlock` 含
  `records`（冻结 64B `ClusterRecord`：`error/parent_error/vertex_offset/
  triangle_offset/triangle_count`）+ `nodes/children`（DAG 拓扑）+ **几何段**
  （块局部顶点池 + 3×u8 局部索引，**世界空间已烘焙**，叶簇几何经
  `verify_cluster_pack` 钉死与源三角逐位一致）+ 组共享 LOD 判定球两表 +
  逐簇继承属性。`pack.passthrough` = 恒直通源三角（emissive/灯尾/病态块）。
- **实测规模**（`.tmp/cluster_lod/window_stats.json`，bistro 真跑）：
  blocks=187、**total_clusters=129,709**、src_tris=1,046,609、
  passthrough=44,024；2px 装配 cut = 20,306 簇 / 794,443 tri；dolly 下逐帧
  cut 变化 = 每帧几个~几十簇、数百~两千 tri（frames 0→13 严格单调升
  794,443→808,852，折返后回落——相邻帧增量极小是增量协议的实证前提）。

### ② rurix-rt 的 AS 更新 API 面（什么在树）

全部在树、全部生产冻结件（vk.rs + render_exec.rs）：

| 面 | 位置 | 语义 |
|---|---|---|
| `create_scene_ex(..., updatable_blas)` | vk.rs L12693 | 创建期把指定 BLAS 打 `ALLOW_UPDATE`（scratch 取 build/update 双尺寸上界、vbuf 附 `TRANSFER_DST`）——B5 蒙皮通路 |
| `record_blas_refit` | vk.rs L13366 | 逐帧 BLAS 顶点 **UPDATE build**（src=dst 原地 refit;**顶点数/拓扑不变 = 合法域**;非 updatable → 确定性 Err fail-closed） |
| `record_tlas_update(action)` | vk.rs L13331 | TLAS `Rebuild`/`Refit`（`TlasBuildAction` L12456）+ generation 报告 |
| `write_transforms` | vk.rs L13242 | TLAS 实例缓冲 host 写：**实例数必须恒定**（L13248「数量变化须走 rebuild/new manager」）；逐 64B 槽与 host 影子 diff，**仅变化槽上传**（A4 槽位级增量）；`mask`/`custom_index`/`transform`/BLAS 引用逐槽可换（`RayQueryTransformedInstanceDesc` 含 `mask: u8`，L12433–12444） |
| `FrameUpdate` | render_exec L466–482 | 数据驱动每帧重录：`tlas_update: Option<(as表下标, Vec<实例>, action)>` + `buffer_uploads`(host-visible 目标,offset 定位) + `blas_refit: Option<BlasRefitUpdate>` |
| `BlasRefitUpdate` | render_exec L429–442 | `{as_index, blas_index, src(session buffer), src_offset, byte_len, after_pass}`——在 pass `after_pass` 录完后插入**桥接 copy → UPDATE build → consume barrier**（L6008–6077;后续 ray query pass 读新 BLAS） |
| 录制序 | render_exec L5573–5582 | TLAS update 录在 **pass 链前**;BLAS refit 录在 `after_pass` **后**（timestamp 分段不含桥,refit GPU 计入帧墙钟——如实计量） |
| FIF 约束 | render_exec L1651–1663 | **FIF 流水面 fail-closed 拒 `tlas_update` 与 `blas_refit`**（共享 host 写面在飞帧读取中不可改写）——「本窗单槽 inflight 先判、FIF×每槽 AS 归 #90」的既有 API 佐证 |
| 双 TLAS | render_exec L1502 `execute_with_frame_update_dual_tlas` | 同帧第二 TLAS 更新（G34-2 加性;本臂不消费） |
| buffer 传输位 | render_exec L2728–2744 | 一切 session buffer 恒附 TRANSFER_SRC/DST ⇒ 任意 storage buffer 可作 refit 桥 src |
| 计时面 | render_exec L933–955 `DeviceFrameTelemetry` | 逐 pass GPU ns + cpu record/submit/fence ns + heap budget + allocation ledger |

**关键否决事实**：`VkBlasEntry`（vk.rs L12486–12505）每 BLAS 独立分配
`vmem`+`mem`+`scratch_mem` **三份 VkDeviceMemory**。全簇粒度 BLAS 池 =
129,709 × 3 ≈ **38.9 万次内存分配**，远超 Vulkan
`maxMemoryAllocationCount`（典型 4096）。次分配器/池化属 AS 管理器改造
（#90 RFC 域）。HZB 臂停在 mesh 节点粒度 1186 BLAS（3×1186=3,558 < 4096）
是同一约束的在案旁证。

### ③ HZB 双 TLAS 帧内轮换先例（窗口 hzb 臂）

`g31_window_present.rs` `G31HzbLane`（L5296+）：bistro 逐 mesh 节点 1186 BLAS
分解 + 双 TLAS（表 0 = 初剔后供相机射线、表 1 = 全量供阴影射线）。逐帧掩码
更新 L5442–5459：

- 实例描述 = `RayQueryTransformedInstanceDesc { blas: i, custom_index: i,
  mask: 0x00|0xFF, transform: IDENTITY }`，动作 = **`TlasBuildAction::Refit`**；
- **等价跳过**：`masks != uploaded_masks` 才发 tlas_update（静态相机稳态零
  TLAS 税）——内容驱动、确定性；
- 会话 = `new_with_accel_structs(..., frame_slots=2, ...)` 顺序入口
  （L5333–5342 自注「FIF 流水面拒 tlas_update,逐帧 host 决策在环本就顺序」）。

先例结论：**逐帧 TLAS 实例掩码通路成熟**，但它建立在「实例数恒定 + BLAS 池
粒度受内存分配数上限约束」之上——掩码粒度到不了簇（129,709 ≫ 分配上限）。

### ④ dyn-demo 臂的动态 AS 通路（TODO #4）

lane_body `frame_dyn`（L10462–10501）：**顺序入口专用**（恒 inflight=1，FIF
入口拒 tlas_update 的 A2 约束登记）——实例变换 host 写（槽位级增量仅动态槽
64B）→ `execute_with_frame_update` 携 `tlas_update:(0, insts, Refit|Rebuild)`
→ 四 pass GPU 链内执行。MegaDyn = 2 BLAS（静态场景 + 动态发光立方体，
L16006 登记），**实例集恒定、只动 transform**。另有 `frame_skin`
（L10503+，B5 MegaSkin）= `blas_refit` 顶点 refit 通路的生产消费者（蒙皮输出
SSBO → pass0 后桥 → UPDATE build），**不逐帧 tlas_update**（BLAS 原地 refit、
实例/TLAS 不动——TLAS 实例 AABB 陈旧性由「形变不出界」吸收，本臂同律并把
前提做成结构保证，见 §3）。

### ⑤ 渲染腿先例（补充侦察）

`rurix-rt/src/bin/vk_clas_rt.rs`（M94 CLAS）：**手编 ray query compute
SPIR-V**（`m94_ray_query_spv` L77–292，set0: b0=TLAS/b1=光线 8f32/b2=输出
4u32〔committed,t_bits,InstanceId,PrimitiveIndex〕，LocalSize 1×1×1）+
「per-簇传统 BLAS × 逐簇实例」回退腿（L479–536）。本臂 RQ kernel 逐指令同
形制适配（bin-local，无新 rurixc 编译面、冻结 kernels/SPV 全 0-byte）。
render_exec 绑定规则（L17–28）：binding [0..A) = `accel_structs` 声明序、
[A..A+N) = `storage_buffers` 声明序——与该 kernel 布局恰合。

---

## 2. 判档裁决

### 候选核算（按侦察事实）

| 候选 | 机制 | 裁决 |
|---|---|---|
| **A（字面）**：全簇粒度 BLAS 池 + 逐帧 TLAS 实例集/掩码 | 129,709 簇 BLAS | **否决（事实性）**：现行 `VkAsManager` 每 BLAS 3 份内存分配 ⇒ ~38.9 万 alloc ≫ `maxMemoryAllocationCount`(典型 4096)；且 `write_transforms` 拒实例数变化 ⇒「实例集=cut」须全簇常驻实例掩码,同样撞分配账。池化次分配器 = AS 管理器改造,归 #90 RFC 域,不在最小判档面私造 |
| **A′**：逐帧重建单 BLAS（cut 重排三角汤 → 全量 build） | 每帧 ~80 万 tri 全量 BLAS build + ~29MB 顶点上传 | 否决（预算+API）：会话 `FrameUpdate` 面**没有**「重建变长 BLAS」操作（仅 refit/tlas_update）；每帧全量 build 正是任务深水点①要规避的爆预算形 |
| **A″（选定）**：**BLAS 顶点 refit 竞技场** —— 全簇固定槽位拓扑,cut 以顶点内容切换（进 cut 写真几何/出 cut 写零面积折叠）,`record_blas_refit` UPDATE build | 单 BLAS 单实例;拓扑/图元数恒定 = **refit 合法域**;逐帧增量上传 = 相邻帧仅几十簇变化(±数百 KB) | **选定**：全部消费 B5 蒙皮冻结通路（`create_scene_ex(updatable_blas)` + `FrameUpdate::blas_refit`）,零 rurix-rt 改动、零新 kernel 编译面;簇粒度精确表达 cut;确定性协议直给（§3） |
| **B**：cut 变化率驱动惰性重建 | 超阈值才重建,登记 hitch | **并入 A″ 同一实现降档**：`--cut-every N`（N=1 逐帧 = A″ 本体;N>1 = 惰性节拍,hitch = refit/非 refit 帧 exec_ms 对照 measured 登记）——一个旗标覆盖两候选,GPU 验收各跑一臂 |

### 与 #89/#90 的诚实分界

- 本臂恒**单槽顺序入口**（`execute_with_frame_update`,frame_slots=2 顺序语义
  ——HZB/dyn/skin 三车道同形）。**FIF 流水面 fail-closed 拒
  tlas_update/blas_refit 是 render_exec 既有纪律**（L1651–1663），不是本臂
  规避——FIF×动态每槽 AS 副本归 **#90 RFC**（另一子任务），本臂不越界。
- cut 仍为 **host 金标准**（`select_lod_cut_grouped` + `verify_cut_coverage`
  直调，#58/W2 visbuffer 同源）。device cut kernel（g31_cluster_cull 链）替换
  host cut 归 **#77 生产接线**自身——本臂判的是「逐帧 AS 更新」通路，
  cut 来源可插拔（判据不变）。
- **presented 面 0-byte**：窗口合入 = 循环后证据臂（§5），出帧翻转（生产六
  pass 车道消费竞技场几何 + Stage A 重锚）归 #77 全量。

### 零面积折叠的合法性与 AABB 陈旧性（正确性深坑,已结构性堵死）

- 出 cut 簇槽写 `[0;9]`（三顶点同点）= **零面积 active 图元**（非 NaN
  inactive）——UPDATE build「active 恒 active/图元数恒定」约束满足，零面积
  三角不产生命中（动态几何行业标准形）。
- BLAS 原地 refit 后 **TLAS 实例 AABB 不自动更新**（B5 蒙皮同律不逐帧
  tlas_update）。若创建期 BLAS 只含帧 0 cut，后续更细几何可能越出创建期根
  AABB ⇒ 设备相关假漏命中。**堵法**：创建期竞技场 = **全簇真几何超集**
  （根 AABB 覆盖一切后续 refit 内容——任意 cut ⊆ 全簇 ∪ 原点折叠，折叠图元
  漏遍历无害因其本就零命中），帧 0 以**单条全量上传**把竞技场收到 cut0，
  逐槽增量自帧 1 起。TLAS 全程不动（tlas_update 恒 None）。

### 确定性协议（判据②的骨架）

digest 序列 = 纯函数(RXCP 字节, 契约相机, `--step-m`×帧号轨迹, `--error-px`,
`--cut-every` 节拍, `--res`, 设备/驱动)：

- **固定轨迹**：帧 k 相机 = 装配相机 + k×step 前向 XZ dolly（host f32 闭式）；
- **固定重建节拍**：`fi % cut_every == 0` 才施加 cut→refit（帧 0 恒施加）——
  refit 序列是帧号纯函数，BVH refit 历史双跑逐字节同 ⇒ 遍历结果同；
- **canonical 竞技场**：槽位 = 块序×簇序（RXCP 序同源），槽内容 = cut 布尔的
  纯函数——无任何运行态依赖的分配/压缩;
- ⇒ **双跑逐帧 digest 位级可复现**（本臂内建双会话重放断言）。跨设备不作
  golden（RT 遍历并列命中 tie-break 依设备,W2 visbuffer digest 同口径登记）。

---

## 3. 最小实现（交付件）

### 机制链（`g31_frame_cut_arm.rs`,窗口合入消费同一文件——单源纪律）

```
RXCP（read_cluster_pack + verify_cluster_pack 冻结校验直调,fail-closed）
  │  frame_cut_arena_layout:全簇固定槽位（canonical 块序×簇序,槽长 =
  │  ClusterRecord::triangle_count）+ passthrough 恒活尾段 + owner 二分表
  ▼
会话创建（DeviceFrameSession::new_with_accel_structs,frame_slots=2 顺序）:
  BLAS×1 = 全簇真几何超集流（frame_cut_full_stream;AABB 保守超集）
  instances×1（identity,mask 0xFF）;updatable_blas=[0]（ALLOW_UPDATE）
  资源:R0 光线 SSBO / R1 命中 SSBO（host 直回读）/ R2 竞技场 SSBO（桥 src）
  pass0 fc_clear（哨兵清写 canary）→ pass1 fc_rq（ray query 命中流）
  ▼
逐帧 k（0..N）:
  ① host 金标准 cut（select_lod_cut_grouped 组共享判定球 +
     verify_cut_coverage **逐帧** fail-closed）
  ② 槽位增量（帧 0 = 全量单条上传收到 cut0;帧 ≥1 = 仅 applied⊕cut 变化槽:
     进 cut 写真几何/出 cut 写 [0;9] 零面积折叠）+ 光线每帧上传（host 针孔,
     确定性 f32）
  ③ FrameUpdate{ buffer_uploads, blas_refit: refit 帧 Some(桥 R2→vbuf 全量
     copy + UPDATE build + consume barrier,after_pass=0) } →
     execute_with_frame_update（pass1 读新 BLAS——B5 冻结通路字面）
  ④ 判据（fail-closed）:哨兵 canary 零残留 + 命中 prim ∈ 竞技场域 +
     **命中槽 ∈ 已施加 cut ∪ passthrough**（陈旧几何零容忍 =「出帧几何随
     相机更新」的机核字面）+ 逐帧命中数 > 0 + validation ERROR = 0
  ⑤ digest = sha256(命中流)——(inst,prim,t) 含竞技场槽号,cut 变 ⇒ digest 变
  ▼
双跑（第二次独立会话重放全轨迹）→ 逐帧 digest 位级断言
cut_tris 单调门（probe 单向 dolly 严门;窗口真轨迹宽门 = 非常量 + 方向登记）
sidecar rurix.g31.frame_cut_probe.v1（独立 JSON,不动既有 evidence schema）
```

measured 分解（不设通过线,sidecar 逐帧行）：`cut_ms`（host 选层+覆盖机核）/
`delta_ms`（增量字节构建）/ `exec_ms`（上传+提交+fence 墙钟,**refit 帧含桥接
copy + UPDATE build**）/ `gpu_clear_ms`+`gpu_rq_ms`（telemetry 逐 pass 时戳,
refit GPU 段不入 pass 时戳、入 exec 墙钟——render_exec L6013 口径如实登记）/
`fence_ms` / `changed_slots` / `upload_bytes`。**AS 更新增量的干净读法** =
`--cut-every N>1` 臂内 refit 帧 vs 非 refit 帧 exec_ms 对照（同跑同会话）。

### 规模账（bistro 全量,启动期精确打印 `arena_tris`/`arena_mb`）

竞技场 ≈ 叶层 1,002,585 + 各粗层合计（层间近减半,合计 ≈ 叶层）+ passthrough
44,024 ≈ **~2.0M tri**：竞技场 SSBO/BLAS vbuf 各 ~72MB host-visible、AS 本体
device 侧另计（telemetry allocation ledger 全登记）。每帧摊销：refit 桥 =
全竞技场 device 内 copy ~72MB（PCIe 不过线,≪1ms 量级）+ 2M tri BLAS UPDATE
build（**待 measured——本判档的核心帧时问题**）;host 增量上传仅变化槽
（相邻帧几十簇 ⇒ 数百 KB）。若超预算：`--blocks-limit N` 子集逃生阀（结构同
判,子集面如实登记）+ `--cut-every` 降节拍 = 候选 B 降档。

### 新文件清单（全部注释「G37 W3 frame-cut」;冻结面零触碰）

| 文件 | 性质 |
|---|---|
| `src/rurix-render/src/bin/g14_3_lane/g31_frame_cut_arm.rs` | 臂共享体（include 件,1292 行;两消费方单源）：`FrameCutArmOpt/FrameCutCamSample/FrameCutFrameStat`、竞技场布局/写器（`frame_cut_arena_layout/full_stream/apply_cut/passthrough_stream/owner`）、逐帧 cut（`frame_cut_select`,金标准直调）、host 光线、两枚 bin-local 手编 SPIR-V（`frame_cut_clear_spv`/`frame_cut_rq_spv`,m94 形制）、会话帧循环（`frame_cut_run_session`）、编排+判据（`run_frame_cut_arm` 内建双跑）、sidecar（`frame_cut_finish`）、host 自检（`frame_cut_selftest`+合成 DAG）。函数级 use（lane body 拼接免 E0252,W2 同律）。 |
| `src/rurix-render/src/bin/g31_frame_cut_probe.rs` | 独立判档 harness（224 行,visbuffer_wiring 模式）：`include!` lane_body+臂 → 契约装配（prelude digest 门）→ RXCP 校验 → 施加前 passthrough 流提取 → 固定 dolly N 帧 → 臂 → sidecar。三态 `skipped_dev_env` 退 0;`--selftest` 纯 host 腿。 |
| `src/rurix-render/Cargo.toml` | 追加 `[[bin]] g31_frame_cut_probe`（`required-features = ["vendor-upscale"]`,lane body 依赖面同 g31 诸 bin）。 |

**无新 rurixc 编译面/SPV 文件**：两 kernel 为 bin-local Rust 手编 SPIR-V
（vk_clas_rt M94 先例;冻结 kernels/*.rx 与 .tmp SPV 全 0-byte）。
`g14_3_lane_body.rs` 0-byte（工作树中该文件的 M 态为他会话既有,本任务未触）。

---

## 4. cargo check / selftest 结果

```
cargo check -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe
→ 退 0;rurix-render 零警告（输出中 15 条 warning 全在依赖 rurix-rt lib,
  既有状态——W2 visbuffer 报告同口径）
cargo check … --bin g31_window_present --bin g14_3_pipeline_perf --bin g31_visbuffer_wiring
→ 退 0（既有三 bin 未受扰旁证）

cargo run … --bin g31_frame_cut_probe -- --selftest（dev 构建,纯 host,无 GPU）
→ [g31_frame_cut_probe]: selftest OK（布局/owner 二分/单调细化 12 帧 1→4 tri/
   增量写器/零面积折叠/双跑确定性/kernel 结构,全 fail-closed 已过）
→ PASS 退 0
```

selftest 覆盖（合成 4叶+2组+1根 DAG,600m→0.8m 对数逼近）：槽基连续/owner
二分闭环、cut 单调细化恰过根→组（px≈1@d≈187）与组→叶（@d≈47）两翻转点、
逐帧覆盖性、全量流/施加写器逐位、零面积折叠、host 双跑逐位、两 kernel
magic+入口名。RED 面：CLI 闭集（--frames<2/--step-m≤0/--cut-every=0 必拒）。

---

## 5. 窗口 bin 合入提案（`--cluster-per-frame-cut` 臂;八处全加性,零删改既有行）

> **锚点一律以字面为准**（八个字面锚均已核唯一命中）。本交付期间窗口 bin
> 正被主线并行合入推进（同一会话内锚行号漂移两轮:visbuffer_finish
> 10845→11209 等）,行号仅供末次核对参考（括注 = 末次复核值）,**合入时必须
> 按字面重定位**。臂 = **循环后证据臂**（presented 面 0-byte,visbuffer 档 2
> 同律）：真窗口逐帧相机 × 真 RXCP → 循环后重放 cut→refit→RQ digest 链,
> 独立 sidecar。

### A. include（字面锚：`include!("g14_3_lane/g31_visbuffer_arm.rs");` 之后插入;末核 L228）

```rust
// G37 W3 frame-cut:#77×#89 合流窗判档臂共享体（加性 include;lane body 0-byte）。
include!("g14_3_lane/g31_frame_cut_arm.rs");
```

### B. 旗标变量（字面锚:`let mut visbuffer_res = String::from("96x54");` 之后插入;末核 L7177）

```rust
    // G37 W3 #77×#89 逐帧 device cut→AS 更新证据臂（off 默认 = 既有面 0-byte;
    // on = 循环后以真窗口逐帧相机重放 refit 竞技场 cut→UPDATE build→RQ digest
    // 链——presented 面不变;出帧翻转归 #77 全量,FIF×每槽 AS 归 #90）。
    let mut frame_cut_on = false;
    let mut frame_cut_out: Option<String> = None;
    let mut frame_cut_every: u32 = 1;
    let mut frame_cut_res = String::from("96x54");
    let mut frame_cut_blocks_limit: usize = 0;
```

### C. 解析臂（字面锚:`"--visbuffer-res" => visbuffer_res = take_arg(&args, &mut i),` 之后插入;末核 L7644）

```rust
            // G37 W3 #77×#89:逐帧 cut 判档臂五参数（模式/sidecar/节拍/画布/子集阀）。
            "--cluster-per-frame-cut" => {
                frame_cut_on = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--cluster-per-frame-cut {other}：只接受 off|on")),
                }
            }
            "--frame-cut-out" => frame_cut_out = Some(take_arg(&args, &mut i)),
            "--frame-cut-every" => {
                frame_cut_every = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frame-cut-every 非 u32"))
            }
            "--frame-cut-res" => frame_cut_res = take_arg(&args, &mut i),
            "--frame-cut-blocks-limit" => {
                frame_cut_blocks_limit = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--frame-cut-blocks-limit 非 usize"))
            }
```

### D. 闭集校验（字面锚:visbuffer 闭集块收束 `VisBufferArmOpt::off()` + `};` 之后插入;末核 L7966）

```rust
    // G37 W3 #77×#89 --cluster-per-frame-cut 闭集校验（fail-closed）：须随
    // --cluster-lod leaf|on（消费 cut 与 RXCP 簇 DAG）;互斥集随 --cluster-lod
    // 继承（hzb/textures/slab/wp-hlod/九臂组合已在其面裁掉,零新增互斥）。
    let frame_cut_opt = if frame_cut_on {
        if cluster_opt.mode == ClusterLodMode::Off {
            fail("--cluster-per-frame-cut on 须随 --cluster-lod leaf|on（消费 cut 与簇 DAG）");
        }
        if frame_cut_every == 0 {
            fail("--frame-cut-every 必须 ≥1（1 = 逐帧;>1 = 惰性节拍臂）");
        }
        let (rw, rh) = {
            let mut it = frame_cut_res.split('x');
            let w: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--frame-cut-res 形如 96x54"));
            let h: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--frame-cut-res 形如 96x54"));
            if it.next().is_some() || w == 0 || h == 0 {
                fail("--frame-cut-res 形如 96x54（两正整数）");
            }
            (w, h)
        };
        FrameCutArmOpt {
            enabled: true,
            res_w: rw,
            res_h: rh,
            frames: 0,  // 真轨迹帧数 = 实采样本数（登记字段,臂消费 samples.len()）
            step_m: 0.0, // 真轨迹非合成 dolly——0 如实登记
            cut_every: frame_cut_every,
            blocks_limit: frame_cut_blocks_limit,
            // 真窗口轨迹可折返（auto-move dolly 往返实测:window_stats.json
            // 帧 13 折返）⇒ 宽门 = 非常量 + 方向 measured 登记,不误红。
            monotone_gate: false,
            out_path: frame_cut_out.clone().unwrap_or_default(),
        }
    } else {
        if frame_cut_out.is_some() {
            fail("--frame-cut-out 须随 --cluster-per-frame-cut on");
        }
        FrameCutArmOpt::off()
    };
```

### E. 施加前 passthrough 流提取（字面锚:`// ③.4 G31+ #58 簇 LOD 施加点` 注释行**之前**插入;`scene` 源装配态在位;末核 L8665）

```rust
    // G37 W3 frame-cut:passthrough 源三角流提取（须先于 apply_cluster_lod——
    // cut 重建后源三角序不复存在;off 空 vec 零消费。簇包预读为 on 臂加性
    // 成本（~49MB 双读）,与 ctx 内簇包同文件同校验,fail-closed 互证）。
    let frame_cut_pt_stream: Vec<f32> = if frame_cut_opt.enabled {
        let p = read_cluster_pack(Path::new(&cluster_opt.pack_path))
            .unwrap_or_else(|e| fail(&format!("--cluster-per-frame-cut 簇包预读: {e}")));
        verify_cluster_pack(&p, &scene)
            .unwrap_or_else(|e| fail(&format!("--cluster-per-frame-cut 簇包校验: {e}")));
        frame_cut_passthrough_stream(&scene, &p.passthrough)
    } else {
        Vec::new()
    };
```

### F. 循环前采集面（字面锚:`let mut visbuffer_samples_taken: Vec<VisBufferCamSample> = Vec::new();` 之后插入;末核 L9267）

```rust
    // G37 W3 frame-cut:真窗口逐帧相机样本采集面（--cluster-per-frame-cut on
    // 才消费;off 空 vec 零消费。device 链循环后跑——不污染 real_render_frame_ms）。
    let mut frame_cut_samples_taken: Vec<FrameCutCamSample> = Vec::new();
```

### G. 主循环采样（字面锚:visbuffer 逐帧采样 `if visbuffer_opt.enabled && …{…}` 块收束 `}` 之后插入;`fi`/`spec`/`in_w`/`in_h` 在作用域;末核 L10439）

```rust
            // G37 W3 frame-cut:逐帧相机样本（Copy 零成本;全帧采集 = 逐帧判档字面）。
            if frame_cut_opt.enabled {
                frame_cut_samples_taken.push(FrameCutCamSample {
                    frame: fi,
                    spec,
                    in_w,
                    in_h,
                });
            }
```

### H. 循环后 device 真跑 + sidecar（字面锚:visbuffer 循环后块 `visbuffer_finish(GTAG, …);` + 收束 `}` 之后、`// ── G31+ #95/#99 逐帧 WP/HLOD 统计 sidecar` 注释行之前插入;末核 L11209）

```rust
    // ── G37 W3 #77×#89 逐帧 cut→AS 更新判档臂（--cluster-per-frame-cut on 才
    //    消费;循环后重放真窗口轨迹——全簇 refit 竞技场 + RQ 命中流 digest;
    //    presented 面 0-byte,独立 sidecar 不动既有五臂 evidence schema;判据/
    //    确定性协议 = g31_frame_cut_arm.rs 头注,双跑位级内建）──
    if frame_cut_opt.enabled {
        let Some((_, pack)) = &cluster_ctx else {
            fail("--cluster-per-frame-cut on 但簇包上下文缺失（闭集校验面破坏）");
        };
        let stats = run_frame_cut_arm(
            GTAG,
            pack,
            &frame_cut_pt_stream,
            &frame_cut_opt,
            cluster_opt.threshold_px,
            &frame_cut_samples_taken,
        );
        frame_cut_finish(GTAG, pack, &frame_cut_opt, cluster_opt.threshold_px, &stats);
    }
```

合入安全性佐证：窗口 bin 现无任何 `FrameCut*`/`frame_cut_*` 符号（grep 零
命中）——include 拼接零冲突;臂文件全函数级 use,无顶层 import 碰撞;H 段在
窗口自身 Vulkan 三态之后执行（device 必在场）;off 路径新增码 = bool 判断 +
空 vec ×2,presented/digest/主 evidence 全 0-byte。**确定性登记**：窗口臂
digest 序列锚定于（真轨迹 + `--frame-cut-every` 节拍 + canonical 竞技场）,
`--auto-move dolly --frames N` 固定入参下双跑位级可复现（臂内建双跑断言）;
折返轨迹单调门降宽已如实登记于 D 段注释与 sidecar `determinism_note`。

---

## 6. GPU 验收步骤清单（留给主 agent;本任务纪律禁跑 GPU/release）

```powershell
# 0) 构建（release 归验收窗）
cargo build --release -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe

# 1) host selftest（无 GPU;应 PASS 退 0）
target\release\g31_frame_cut_probe.exe --selftest

# 2) 判档主跑（GPU;RXCP 直接复用 #58 在盘包——或按 W2 报告两步重生成）
target\release\g31_frame_cut_probe.exe --cluster-pack .tmp\cluster_lod\bistro.rxcp `
  --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 `
  --evidence .tmp\g37_w3\frame_cut_ev.json
#    期望:「会话就绪 arena_tris=~2.0M」→「双跑 digest 位级 16 帧全等;
#    cut_tris 794443 → ~80.9万（单调不减）」→「逐帧 cut→AS 更新臂 OK …
#    exec_ms(refit均)=…」→ PASS 退 0。双跑/哨兵/命中∈已施加 cut/覆盖性
#    全在 bin 内 fail-closed。sidecar frames_data[].digest 相邻帧互异
#    （相机+cut 双驱动）,cut_tris 与 window_stats.json 同轨迹段数值可互证。
#    帧时读法:exec_ms(refit) ≈ 上传+72MB 桥 copy+2M tri UPDATE build+RQ+fence,
#    gpu_rq_ms/gpu_clear_ms 为逐 pass 时戳（refit GPU 不入 pass 时戳,入墙钟）。

# 3) 惰性节拍臂（候选 B 对照;AS 更新增量 = refit/非 refit 帧 exec_ms 差,同跑同会话）
target\release\g31_frame_cut_probe.exe --cluster-pack .tmp\cluster_lod\bistro.rxcp `
  --error-px 2.0 --frames 16 --cut-every 4 `
  --evidence .tmp\g37_w3\frame_cut_lazy.json

# 4) 逃生阀（仅当 2) 显存/建面超预算时）:--blocks-limit 64（子集面如实登记）

# 5) RED 反证（CLI 闭集,应必拒）:--frames 1 / --step-m 0 / --cut-every 0

# 6) 合入 §5 八段 → 重建 g31_window_present → 窗口臂真跑（GPU）
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\cluster_lod\bistro.rxcp `
  --cluster-per-frame-cut on --frame-cut-out .tmp\g37_w3\frame_cut_window.json `
  --evidence .tmp\g37_w3\window_on_ev.json
#    期望:循环后「逐帧 cut→AS 更新臂 OK frames=24 …」+ sidecar 落盘;
#    折返轨迹 cut_tris 非常量（宽门）;主 evidence/presented digest 与不带
#    三旗标的对照跑一致（加性回归证,visbuffer §7-4 同式）。

# 7) off == 锚不动:python ci\g31_cluster_lod_smoke.py 复跑绿 + Stage A 锚格照常。
```

---

## 7. TODO 登记建议（主 agent 合入验收后）

#77 行现状更新方向：「逐帧 device cut→AS 更新**判档在案**（选定 = 全簇固定
槽位 BLAS refit 竞技场,B5 蒙皮 `blas_refit` 冻结通路直调;全簇 BLAS 池被
`maxMemoryAllocationCount` 账否决,池化归 #90 RFC）。`g31_frame_cut_probe`
harness 全 fail-closed（双跑位级/单调 cut/命中∈已施加 cut/哨兵 canary）;
窗口证据臂 `--cluster-per-frame-cut` 合入提案八锚在案。**留窗** = device cut
kernel 换 host 金标准（#77 生产接线本体）+ presented 出帧翻转（生产车道消费
竞技场,重锚面）+ FIF×每槽 AS（#89/#90,RFC 面）」。不充绿、不冒充出帧。
