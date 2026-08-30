# G37 W2 — VisBuffer + classify/resolve 生产臂（TODO #74/#111）方案·实现·合入提案

> 任务：把 day_0827 harness 机制链臂（`g31_cluster_cull_device --visbuffer`）变成
> `g31_window_present` 的窗口生产加性臂 `--visbuffer`（off == 锚不动）。
> 窗口 bin 只读——本报告含合入 diff 提案，合入由主 agent 执行。
> 纪律遵守：未跑 GPU、未 `cargo build --release`、未碰 target-night、冻结件全 0-byte。

---

## 1. 判档：**档 2（生产证据臂）**

`--visbuffer on` = 窗口会话内 device 真跑机制链（消费**真窗口逐帧相机样本** ×
**真场景簇 DAG**），产出覆盖集合 digest + host oracle 对拍 + 统计进独立 sidecar
evidence；presented 面 0-byte（仍 ray 车道出帧），出帧留窗如实登记。

弃档 1（全出帧）的差距核算（超一臂当量，诚实拒绝）：

1. **软光栅 kernel 量级**：机制链消费的 M95 SW kernel（`sw_visbuffer_u64_spv`）
   是 LocalSize 1×1×1、dispatch = `tris×W×H` 的蛮力腿（harness 自注「生产 tile
   化归 #75 后续」）。1080p 内部分辨率 × bistro cut ~80 万三角 ≈ 1.7e12 线程，
   物理不可行；生产级 tile 化 compute 光栅是 **#75 自己的行**，不在本臂私造。
2. **着色桥**：classify/resolve 之后的「简化着色出帧」要进 presented 链 =
   替换/叠加 TSR 上游输入（TSR 依赖 MV/历史）+ 显示编码链，Stage A 18 格锚与
   五门回归锚全数重锚——超一臂当量且与「off == 锚不动」矛盾（on 臂本身也须
   有稳定不摧毁性）。
3. 档 2 完全满足任务给的生产接线定义：窗口会话消费（真相机/真 DAG）、device
   真跑、digest + oracle + 统计进 evidence。harness 臂的输出语义（见 §2.④）也
   证实机制链的自然产物就是证据面而非可 presented 帧。

**出帧留窗**（登记于 sidecar note 与本报告）：#74 shade 桥（classify/resolve 后
简化着色进 presented 链）+ #75 生产 tile 化 compute 软光栅 + HW device 图形腿
窗口化（本臂 HW 箱 = host 保守光栅 + M95 门 diff=0 锚复用，harness 同律）。

---

## 2. 侦察记录（任务①–④）

① **harness bin** = `src/rurix-asset/src/bin/g31_cluster_cull_device.rs`（非
rurix-render；bin 名与 TODO 简称 g31_cluster_lod_device 略异）。`--visbuffer`
臂 = `run_visbuffer_arm`（L516–754）：两阶段 HZB 最终可见集 →
`compact_draw_args` 32px SW/HW 分箱 → SW 箱投影三角流 → `sw_visbuffer_u64_spv`
device 真跑（u64 atomicMax）→ device 双跑位级 + host `VisBufferCpu` oracle
覆盖集合全等 → HW 箱 host 保守光栅（M95 diff=0 锚复用登记）→ 哨兵感知 u64
max 合并 → `classify`/`resolve`。cluster27 载荷 = **帧内可见列表下标**（Nanite
口径），合成材质 = `id % 7 + 1`。

② **API 面**（全部生产冻结件，本臂 0-byte 直调）：
- `geometry/visbuffer.rs`：`VisBufferCpu`（`raster_triangle` 边函数 + top-left +
  reverse-Z 30 位量化 + atomicMax 语义）、`VISBUFFER_CLEAR = 2³⁴−1`、投影口径
  （clip.w ≤ 1e-20 整三角保守丢弃）。
- `geometry/visbuffer_swhw_spv.rs`：`sw_visbuffer_u64_spv()`（绑定 0=triangles
  f32[9t] / 1=ids u32[2t] / 2=vis u64[W·H]；push consts = tri_count/W/H；
  capability Int64+Int64Atomics）；HW VS/FS 构建件在（本臂不消费，M95 锚复用）。
- `geometry/material_pass.rs`：`classify(vis, cluster_to_material, tile)`（tile×
  材质分桶 + 前缀和）、`resolve`（16 位窄缓冲）、`MATERIAL_INVALID = u16::MAX`。
- `geometry/cull.rs`：`compact_draw_args(visible, instances, clusters, cam,
  DEFAULT_BIN_THRESHOLD_PX=32)` → `DrawArgsCpu{sw_clusters, hw_clusters, …}`。
- `graph/types.rs`：`MAX_TRIS_PER_CLUSTER = 128`（tri7 域契约恰合）。

③ **窗口 --cluster-lod 臂结构**（`g14_3_lane_body.rs` 共享体承载）：
- 资产装载：`--cluster-pack <RXCP>` → `read_cluster_pack`（L2680，fail-closed
  逐字段游标）→ `verify_cluster_pack`（gltf sha / 叶∪passthrough 覆盖恰一次 /
  叶几何位级）。`ClusterPackBlock` 含 `records/nodes/children` + **几何段**
  （块局部顶点池 + 3×u8 局部索引，世界空间已烘焙）+ `cluster_mat`（叶后代
  众数）+ 组共享 LOD 判定球两表。
- 装配期：窗口 L7687 `apply_cluster_lod`（契约初始相机 cut 重建三角汤出帧，
  **簇包保留复用**返回给 `cluster_ctx`）。
- 逐帧：窗口 L9195–9208 `cluster_lod_frame_stat`（真轨迹相机 `cam.spec()` →
  `cluster_cull_camera(spec, in_w, in_h, threshold)` → 逐块
  `select_lod_cut_grouped`，每 16 帧覆盖性机核）——**本臂的相机/资产消费路径
  与此逐字同源**。
- evidence：`--cluster-stats-out` 独立 sidecar JSON（L9892–9943），主五臂
  evidence schema 0-byte——**#58/#95 同律，本臂沿用**。

④ **出帧语义**：harness 软光栅输出 = u64 VisBuffer（`depth30|cluster27|tri7`
打包值缓冲，非颜色帧）+ classify 桶表 + resolve u16 材质窄缓冲。生产出帧须过
shade（classify 后按桶着色）——即档 1 的缺口；本臂取证据面（覆盖集合 +
digest + 统计）。

---

## 3. 机制链在窗口的资产/相机消费路径（档 2 实现）

```
--cluster-pack <RXCP>（g31_cluster_lod_bake 产物,bistro 全量 DAG）
      │  read_cluster_pack + verify_cluster_pack（既有冻结校验,fail-closed）
      ▼
cluster_ctx: (ClusterLodReport, ClusterPack)      ←—— 窗口既有面（L7687）,0-byte
      │
主循环（每帧 spec = cam.spec()——auto-move/交互真轨迹）
      │  采样帧 ∈ visbuffer_sample_frames(total, N)（默认 3 = 首/中/末等距）
      │  → 零成本记录 VisBufferCamSample{frame, spec, in_w, in_h}
      ▼
循环后（不污染 real_render_frame_ms 口径）run_visbuffer_arm:
  逐样本: cluster_cull_camera(spec, in_w, in_h, --cluster-error-px)
    → 逐块 select_lod_cut_grouped + verify_cut_coverage（fail-closed）→ 可见簇集
    → compact_draw_args（32px 投影直径,SW/HW 分箱;块=identity 实例全局表）
    → SW 箱: 投影（visbuffer.rs 口径逐字）→ sw_visbuffer_u64_spv device 分块
      dispatch（块界 = 2³⁰/(W·H) 组;u64 atomicMax 交换结合 ⇒ 跨块累积与单发
      等价）→ 双跑位级断言 → host VisBufferCpu oracle 覆盖集合全等断言
    → HW 箱: host 保守光栅（HW device 图形腿 = M95 门 diff=0 锚复用登记）
    → 哨兵感知 u64 max 合并 → classify(16px tile)/resolve（材质 = RXCP
      cluster_mat 收窄 u16;SLAB_TRI_NONE → 65534 无材质槽）
      → resolve 像素数 == 合并覆盖数断言
    → merged VisBuffer sha256 digest + 全统计
      ▼
--visbuffer-out <sidecar.json>（schema rurix.g31.visbuffer_stats.v1;
  独立文件,主 evidence/presented 面 0-byte）
```

fail-closed 判据（跑时机内断言，破坏即非零退出）：cut 覆盖性（逐样本）、
cluster27/tri7 域界、SW device 双跑位级、SW device 覆盖集合与 host oracle
全等、合并零覆盖防空接线、resolve 像素数恒等、classify 非零桶。
measured 如实登记（不设通过线）：分箱簇/三角数、覆盖数、桶数、digest、
cut/投影/device/对拍/classify 分项 ms。
digest 语义：同设备同会话确定性锚（device f32 打包深度位含 FMA，跨设备
不作 golden——M95 归因同构，oracle 对拍只比覆盖集合）。

---

## 4. 新文件清单（全部注释「G37 W2 visbuffer」）

| 文件 | 性质 |
|---|---|
| `src/rurix-render/src/bin/g14_3_lane/g31_visbuffer_arm.rs` | 臂实现共享体（include 件；两消费方单源，禁旁路复刻）：`VisBufferArmOpt`/`VisBufferCamSample`/`VisBufferFrameStat`、`visbuffer_sample_frames`、`visbuffer_build_tables`、`visbuffer_run_sample`（机制链）、`run_visbuffer_arm`（编排）、`visbuffer_stats_json`/`visbuffer_finish`（sidecar）。全函数级 use（lane body 拼接免 E0252）。 |
| `src/rurix-render/src/bin/g31_visbuffer_wiring.rs` | 独立接线 harness（合入前编译/device 冒烟）：契约装配（`prelude` digest 门）→ RXCP 校验 → 装配相机 + 前向 dolly 样本梯（k×0.15m）→ 同一臂函数 → sidecar。三态 `skipped_dev_env` 退 0。 |
| `src/rurix-render/Cargo.toml` | 追加 `[[bin]] g31_visbuffer_wiring`（`required-features = ["vendor-upscale"]`，lane body 依赖面同 g31/g34/g35 诸 bin）。 |

冻结面零触碰：`g31_window_present.rs` / `g14_3_lane_body.rs` /
`g14_3_pipeline_perf.rs` / 既有 kernels 与 .spv / `geometry/` 既有金标准 /
`milestones/` / `registry/` / `ci/` 全部 0-byte。**无新 kernel**（SW 腿 =
`sw_visbuffer_u64_spv()` 冻结构建件直调）。

---

## 5. 窗口 bin 合入 diff 提案（g31_window_present.rs；锚点行号 = 本交付时点）

> 七处全加性插入，零删改既有行。锚点给「行号 + 字面」双定位（合入时以字面为准）。

### A. include（锚 L218：`include!("g14_3_lane/g14_3_lane_body.rs");` 之后插入）

```rust
// G37 W2 visbuffer:#74/#111 窗口生产证据臂共享体（加性 include;lane body 0-byte）。
include!("g14_3_lane/g31_visbuffer_arm.rs");
```

### B. 旗标变量（锚 L6389：`let mut cluster_stats_out: Option<String> = None;` 之后插入）

```rust
    // G37 W2 #74/#111 VisBuffer + classify/resolve 生产证据臂（off 默认 =
    // 既有面 0-byte;on = 窗口会话内 device 真跑机制链——真窗口相机样本 ×
    // 真场景簇包,presented 面不变;出帧留窗 = #74 shade 桥 + #75 tile 化）。
    let mut visbuffer_on = false;
    let mut visbuffer_out: Option<String> = None;
    let mut visbuffer_samples: u32 = 3;
    let mut visbuffer_res = String::from("96x54");
```

### C. 解析臂（锚 L6799：`"--cluster-stats-out" => cluster_stats_out = Some(take_arg(&args, &mut i)),` 之后插入）

```rust
            // G37 W2 #74/#111:visbuffer 证据臂四参数（模式/sidecar/采样帧数/画布）。
            "--visbuffer" => {
                visbuffer_on = match take_arg(&args, &mut i).as_str() {
                    "off" => false,
                    "on" => true,
                    other => fail(&format!("--visbuffer {other}：只接受 off|on")),
                }
            }
            "--visbuffer-out" => visbuffer_out = Some(take_arg(&args, &mut i)),
            "--visbuffer-samples" => {
                visbuffer_samples = take_arg(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--visbuffer-samples 非 u32"))
            }
            "--visbuffer-res" => visbuffer_res = take_arg(&args, &mut i),
```

### D. 闭集校验（锚 L7068–7070 块之后插入）

锚（字面）：

```rust
    if cluster_stats_out.is_some() && cluster_opt.mode == ClusterLodMode::Off {
        fail("--cluster-stats-out 须随 --cluster-lod leaf|on（统计面无 cut 无意义）");
    }
```

插入：

```rust
    // G37 W2 #74/#111 --visbuffer 闭集校验（fail-closed）：须随 --cluster-lod
    // leaf|on（机制链消费 cut 与 RXCP 簇 DAG）;互斥集随 --cluster-lod 继承
    //（hzb/textures/slab-table/wp-hlod/九臂组合已在其面裁掉,零新增互斥）。
    let visbuffer_opt = if visbuffer_on {
        if cluster_opt.mode == ClusterLodMode::Off {
            fail("--visbuffer on 须随 --cluster-lod leaf|on（机制链消费 cut 与簇 DAG）");
        }
        if visbuffer_samples == 0 {
            fail("--visbuffer-samples 必须 ≥1");
        }
        let (rw, rh) = {
            let mut it = visbuffer_res.split('x');
            let w: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--visbuffer-res 形如 96x54"));
            let h: u32 = it
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or_else(|| fail("--visbuffer-res 形如 96x54"));
            if it.next().is_some() || w == 0 || h == 0 {
                fail("--visbuffer-res 形如 96x54（两正整数）");
            }
            (w, h)
        };
        VisBufferArmOpt {
            enabled: true,
            res_w: rw,
            res_h: rh,
            samples: visbuffer_samples,
            out_path: visbuffer_out.clone().unwrap_or_default(),
        }
    } else {
        if visbuffer_out.is_some() {
            fail("--visbuffer-out 须随 --visbuffer on");
        }
        VisBufferArmOpt::off()
    };
```

### E. 循环前采集面声明（锚 L8198：`let mut cluster_stat_ms_total = 0.0f64;` 之后插入；`total` L8110 已在作用域）

```rust
    // G37 W2 #74/#111 visbuffer 相机样本采集面（--visbuffer on 才消费;off 空
    // vec 零消费。device 链循环后跑——不污染 real_render_frame_ms 口径）。
    let visbuffer_sample_set: Vec<u32> = if visbuffer_opt.enabled {
        visbuffer_sample_frames(total, visbuffer_opt.samples)
    } else {
        Vec::new()
    };
    let mut visbuffer_samples_taken: Vec<VisBufferCamSample> = Vec::new();
```

### F. 主循环采样（锚 L9208：cluster 逐帧统计块的收束 `}`——即
`cluster_frame_stats.push(stat);` 的下一行——之后插入；`spec`/`in_w`/`in_h`/`fi` 在作用域）

```rust
            // G37 W2 visbuffer:真窗口逐帧相机样本采集（Copy 零成本;device 链
            // 循环后跑）。
            if visbuffer_opt.enabled && visbuffer_sample_set.contains(&fi) {
                visbuffer_samples_taken.push(VisBufferCamSample {
                    frame: fi,
                    spec,
                    in_w,
                    in_h,
                });
            }
```

### G. 循环后 device 真跑 + sidecar（锚 L9943：cluster 统计 sidecar 块
`if let Some((rep, _)) = &cluster_ctx { … }` 的收束 `}` 之后、
`// ── G31+ #95/#99 逐帧 WP/HLOD 统计 sidecar` 注释行之前插入）

```rust
    // ── G37 W2 #74/#111 visbuffer 生产证据臂（--visbuffer on 才消费;循环后
    //    device 真跑机制链——真窗口相机样本 × 真场景簇包;presented 面
    //    0-byte,独立 sidecar 不动既有五臂 evidence schema,#58/#95 同律）──
    if visbuffer_opt.enabled {
        let Some((_, pack)) = &cluster_ctx else {
            fail("--visbuffer on 但簇包上下文缺失（闭集校验面破坏）");
        };
        let stats = run_visbuffer_arm(
            GTAG,
            pack,
            &visbuffer_opt,
            cluster_opt.threshold_px,
            &visbuffer_samples_taken,
        );
        visbuffer_finish(GTAG, pack, &visbuffer_opt, cluster_opt.threshold_px, &stats);
    }
```

合入安全性佐证：窗口 bin 现无任何 `VisBuffer*`/`visbuffer_*` 符号（grep 零命中），
lane body 仅注释提及——include 拼接零冲突；臂文件全函数级 use，无顶层 import
碰撞；G 段在窗口自身 Vulkan 三态之后执行（device 必在场）。off 路径新增码 =
一次 bool 判断 + 空 vec，presented/digest/主 evidence 全 0-byte。

---

## 6. cargo check 结果

```
cargo check -p rurix-render --features vendor-upscale --bin g31_visbuffer_wiring
→ Finished `dev` profile … 退出码 0;rurix-render 零警告
  （输出中 15 条 warning 全在依赖 rurix-rt lib,既有状态）
```

clippy 补充核查：`cargo clippy … --bin g31_visbuffer_wiring` 在 **rurix-rt lib**
即报 8 个既有 error（unsafe 缺 safety 注释）；对照跑既有
`--bin g31_window_present` 同参数**同样报错** ⇒ 既有状态非本臂引入（`--no-deps`
下 rurix-render lib 也有 1 个既有 error 于 `particles/oit_arms.rs:115`，未触碰）。
本臂新码经 rustc（含 workspace lints）零警告。

---

## 7. GPU 验收步骤清单（留给主 agent；本任务纪律禁跑 GPU/release）

```powershell
# 0) 构建（release 归验收窗;ci/g31_cluster_lod_smoke.py 同惯例）
cargo build --release -p rurix-render --features vendor-upscale `
  --bin g14_3_pipeline_perf --bin g31_visbuffer_wiring
cargo build --release -p rurix-asset --bin g31_cluster_lod_bake

# 1) 资产链两步（host,无 GPU;#58 门同两步,已有 .rxcp 可直接复用跳过）
target\release\g14_3_pipeline_perf.exe --dump-scene --scene bistro-interior `
  --out .tmp\g37_w2\bistro.rxcs
target\release\g31_cluster_lod_bake.exe --scene-dump .tmp\g37_w2\bistro.rxcs `
  --out .tmp\g37_w2\bistro.rxcp --double-build

# 2) 合入前独立冒烟（本臂交付件;GPU）
target\release\g31_visbuffer_wiring.exe --cluster-pack .tmp\g37_w2\bistro.rxcp `
  --error-px 2.0 --samples 3 --evidence .tmp\g37_w2\vis_standalone.json
#    期望:「PASS samples=3 res=96x54」退 0;sidecar frames[] 3 项逐项
#    sw_device_ran=true、merged_covered>0、classify_buckets>0、digest 非空。
#    双跑确定性:重复本步,两次 sidecar 逐帧 visbuffer_digest 相等（同设备）。

# 3) 合入 §5 七段 → 重建 g31_window_present → 窗口臂真跑（GPU）
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\g37_w2\bistro.rxcp `
  --visbuffer on --visbuffer-out .tmp\g37_w2\vis_window.json `
  --cluster-stats-out .tmp\g37_w2\cl_stats.json `
  --evidence .tmp\g37_w2\window_on_ev.json
#    期望:「visbuffer 帧 N …」×3 +「visbuffer 臂 OK samples=3」+ sidecar 落盘;
#    dolly 下 frames[].cut_tris 随帧号变化（相机真实驱动）。

# 4) off == 锚不动（加性回归两证）
#    4a) 同 3) 去掉 --visbuffer 三旗标 → 主 evidence/receipt digest 面与 3) 一致
#        （visbuffer = 循环后证据臂,不触帧流水）。
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\g37_w2\bistro.rxcp `
  --evidence .tmp\g37_w2\window_off_ev.json
#    4b) 默认全 off 既有锚:python ci\g31_cluster_lod_smoke.py 复跑绿 +
#        Stage A 锚格照常（合入为纯加性,off 零展开）。

# 5) fail-closed 反证（可选 RED）:--visbuffer on 不带 --cluster-lod → 必拒;
#    --visbuffer-out 不带 --visbuffer on → 必拒。
```

预期量级（供排程）：`--error-px 2.0` 下 cut ~70–80 万三角，SW 箱占多数；
96×54 画布分块界 = 2³⁰/5184 ≈ 20.7 万三角/块 ⇒ 每样本 ≈ 3–4 dispatch × 双跑，
每 dispatch 独立 `vk::run_compute`（自建 instance/device）——秒级/样本，
3 样本合计 < 1 分钟。组数假设 = NVIDIA 级 `maxComputeWorkGroupCount[0]`
（2³¹−1;harness 同假设已在夜间设备过门）。画布升维用 `--visbuffer-res`，
成本 O(tris×px) 线性放大——生产分辨率归 #75 tile 化，勿用本臂硬扛。

---

## 8. TODO 登记建议（主 agent 合入验收后）

#74/#111 行现状更新方向：「窗口生产证据臂 `--visbuffer` 在案（真窗口相机 ×
真 DAG device 真跑机制链,SW 覆盖集合与 oracle 全等 + 双跑位级 + classify/
resolve 材质分箱,sidecar rurix.g31.visbuffer_stats.v1）；**出帧留窗** = #74
shade 桥 + #75 生产 tile 化 + HW device 图形腿窗口化」。不充绿、不冒充出帧。
