# G37 W3 — `--cluster-per-frame-cut` 窗口 bin 合入记录（frame_cut 臂,8 锚全加性）

> 合入执行会话（持窗口 bin 独占编辑权）。依据 = 同目录 REPORT.md §5 八处字面锚
> 合入提案。纪律遵守：未跑 GPU、未 `cargo build --release`、未碰 target-night、
> lane_body / g14_3_pipeline_perf / kernels / 既有 SPV / milestones / registry /
> ci 全 0-byte;本次唯一改动文件 = `src/rurix-render/src/bin/g31_window_present.rs`
> （八段全加性,零删改既有行）。

## 1. 八锚落点与偏差

行号漂移符合报告预警（「窗口 bin 行号已漂两轮」+ 此后 FG 组合 16 锚再合入又漂
一轮）——八个字面锚在当前工作树**全部唯一命中**,按内容锚重定位后插入。下表
「报告末核」= REPORT.md §5 括注行号,「合入前实测」= 本次重定位命中行,
「合入后落点」= 插入块首行（`G37 W3 frame_cut 合入` 注释行,grep 可复核）。

| 锚 | 字面锚 | 报告末核 | 合入前实测 | 合入后落点 | 内容 |
|---|---|---|---|---|---|
| A | `include!("g14_3_lane/g31_visbuffer_arm.rs");` 之后 | L228 | L235 | L236–237 | 臂共享体 include |
| B | `let mut visbuffer_res = String::from("96x54");` 之后 | L7177 | L7248 | L7251–7259 | 五旗标变量 |
| C | `"--visbuffer-res" => …take_arg…,` 之后 | L7644 | L7715 | L7727–7747 | 五参数解析臂 |
| D | visbuffer 闭集块收束 `VisBufferArmOpt::off()` + `};` 之后 | L7966 | L8043–8044 | L8077–8121 | 闭集校验 + `frame_cut_opt` 构造 |
| E | `// ③.4 G31+ #58 簇 LOD 施加点` 注释行之前 | L8665 | L8775 | L8852–8864 | 施加前 passthrough 源三角流提取 |
| F | `let mut visbuffer_samples_taken: Vec<VisBufferCamSample> = Vec::new();` 之后 | L9267 | L9377 | L9467–9470 | 循环前相机样本采集面 |
| G | visbuffer 逐帧采样块收束 `}` 之后 | L10439 | L10583 | L10677–10686 | 主循环逐帧相机样本 |
| H | `visbuffer_finish(GTAG,…);` + `}` 之后、`// ── G31+ #95/#99 逐帧 WP/HLOD 统计 sidecar` 之前 | L11209 | L11346–11348 | L11451–11469 | 循环后 `run_frame_cut_arm` + `frame_cut_finish` |

**与提案的偏差（仅注释面,零代码语义偏差）**：

- 各块首注释前缀按合入纪律统一为「G37 W3 frame_cut 合入:」（提案原文为
  「G37 W3 frame-cut:」/「G37 W3 #77×#89:」）,注释正文语义原样保留,个别行
  按行宽重折行。
- 旗标名/闭集校验/`FrameCutArmOpt` 字段值（`frames: 0`、`step_m: 0.0`、
  `monotone_gate: false` 宽门）/E 段双读互证/H 段调用签名——**与提案逐项一致,
  无任何改写**。
- 合入前置核验（提案「合入安全性佐证」复核）：窗口 bin 合入前
  `FrameCut*`/`frame_cut_*` 符号 grep 零命中;`Path`/`ClusterLodMode::Off`/
  `cluster_opt.pack_path`/`read_cluster_pack`/`verify_cluster_pack`（lane_body
  L2680/L2871,经既有 include 在作用域）全部在位;
  `run_frame_cut_arm(tag,pack,pt_stream,opt,threshold_px,samples)` 与
  `frame_cut_finish(tag,pack,opt,threshold_px,stats)` 签名与 H 段字面吻合。

## 2. 与 --visbuffer 臂共存检查（两臂同开）

两臂同为 cluster-pack 消费的**循环后证据臂**,插点相邻但状态完全独立——同开
无冲突：

- **闭集面（D）**：两臂闭集块前后相邻（visbuffer L8039–8076,frame_cut
  L8077–8121）,各自独立要求 `--cluster-lod leaf|on`,互斥集均随 --cluster-lod
  继承,**两臂之间零互斥字面**——`--visbuffer on --cluster-per-frame-cut on`
  合法组合。
- **循环前采集面（F）**：visbuffer = `visbuffer_sample_set` +
  `visbuffer_samples_taken`（L9459–9466）;frame_cut 仅新增独立
  `frame_cut_samples_taken` 一个 vec（L9467–9470,紧随其后）。无共享可变状态。
- **主循环采样（G）**：两块为相邻独立 `if`（visbuffer L10666–10675 稀疏采样
  `visbuffer_sample_set.contains(&fi)`,默认 3 样本;frame_cut L10677–10686
  全帧采集）。两者只读拷贝 `fi/spec/in_w/in_h`（`CameraSpec` Copy——visbuffer
  先按值 push、其后 L10696 `build_vp(&spec,…)` 仍可用即为证）,互不影响、也
  不影响后续帧参数消费;两块均在 `t_render` 计时外,`real_render_frame_ms`
  口径不染。
- **循环后 device 真跑（H）**：严格串行——visbuffer 块（L11434–11450）整块
  跑完（其 device 会话在 `run_visbuffer_arm` 内创建并析构）,frame_cut 块
  （L11451–11469）随后才建自己的会话;两者对 `cluster_ctx` 簇包只读共享,
  sidecar 各写各文件(`--visbuffer-out` / `--frame-cut-out`),既有五臂 evidence
  schema 两臂皆 0-byte。
- **E 段预读**：frame_cut 在 apply_cluster_lod 前对簇包独立双读+校验,与
  visbuffer 消费的 `cluster_ctx` 内簇包同文件同校验,fail-closed 互证,无写面。

结论：**采样面/相机采集/会话/输出四面均不冲突,两臂同开成立**。

## 3. cargo check 结果

```
cargo check -p rurix-render --features vendor-upscale --bin g31_window_present
→ Finished `dev` profile [unoptimized + debuginfo], EXIT=0（dev/默认 target,
  未跑 release、未碰 target-night）
```

- 合入块自身**零 error 零 warning**;ReadLints 对窗口 bin 零诊断。
- 输出中 warning 均为既有状态,不由本合入引入：rurix-rt lib 15 条（W2/W3 报告
  同口径）+ 窗口 bin 4 条（L9432/9435 unused doc comment 于 HZB probe 暂存区、
  L9452/9453 svt_era 赋值未读——均在他会话已合入的 HZB/SVT 区段,本次八块未触）。
- **过程登记（如实）**：首两次 check 在 rurix-rt lib 报
  `frame_update_state` 调用点 E0061——为并行会话（#90 FIF 面,
  `render_exec.rs`/`render_exec_g37_fif_dyn.rs` 活跃编辑中）的瞬时中间态,
  与本合入无关（错误位于依赖 lib,窗口 bin 尚未进入编译）。等待其 06:13 收敛
  后复跑即全绿;本会话未触 rurix-rt 任何文件。

## 4. GPU 验收命令（留给验收窗;本会话纪律禁跑 GPU/release）

RXCP 现成资产二选一（均在盘）：**优先复用 g36 门资产**
`.tmp\g36_gates\wave1_geo_composition\bistro.rxcp`（47.8MB,2026-08-30 03:53,
g36 门 double-build 确定性已验）;REPORT §6 原写 `.tmp\cluster_lod\bistro.rxcp`
（49.2MB,08-27）亦可（verify_cluster_pack fail-closed 兜底,包-场景不符即拒）。

```powershell
# 0) release 构建（验收窗;默认 target,禁 target-night）
cargo build --release -p rurix-render --features vendor-upscale --bin g31_frame_cut_probe
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present

# 1) probe host selftest（无 GPU;应 PASS 退 0）
target\release\g31_frame_cut_probe.exe --selftest

# 2) probe 判档主跑（GPU;固定单向 dolly = 严单调门）
target\release\g31_frame_cut_probe.exe --cluster-pack .tmp\g36_gates\wave1_geo_composition\bistro.rxcp `
  --error-px 2.0 --frames 16 --step-m 0.15 --res 96x54 `
  --evidence .tmp\g37_w3\frame_cut_ev.json
#    期望: arena_tris≈2.0M → 双跑 digest 位级 16 帧全等 → cut_tris 单调不减 → PASS 退 0

# 3) probe 惰性节拍臂（候选 B 对照;AS 更新增量 = refit/非 refit 帧 exec_ms 差）
target\release\g31_frame_cut_probe.exe --cluster-pack .tmp\g36_gates\wave1_geo_composition\bistro.rxcp `
  --error-px 2.0 --frames 16 --cut-every 4 `
  --evidence .tmp\g37_w3\frame_cut_lazy.json

# 4) RED 反证（CLI 闭集,应必拒）: --frames 1 / --step-m 0 / --cut-every 0;
#    窗口面: --cluster-per-frame-cut on 不带 --cluster-lod / --frame-cut-out 不带 on

# 5) 窗口臂真跑（GPU;本合入的验收主体——真窗口折返轨迹 = 宽门非常量）
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\g36_gates\wave1_geo_composition\bistro.rxcp `
  --cluster-per-frame-cut on --frame-cut-out .tmp\g37_w3\frame_cut_window.json `
  --evidence .tmp\g37_w3\window_on_ev.json
#    期望: 循环后「逐帧 cut→AS 更新臂 OK frames=24 …」+ sidecar 落盘;主 evidence/
#    presented digest 与不带三旗标对照跑一致（加性回归证,visbuffer §7-4 同式）

# 5b) 两臂共存跑（本合入新增共存面的机器验证;两 sidecar 各自落盘）
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\g36_gates\wave1_geo_composition\bistro.rxcp `
  --visbuffer on --visbuffer-out .tmp\g37_w3\visbuffer_coexist.json `
  --cluster-per-frame-cut on --frame-cut-out .tmp\g37_w3\frame_cut_coexist.json `
  --evidence .tmp\g37_w3\window_both_ev.json

# 6) off == 锚不动: python ci\g31_cluster_lod_smoke.py 复跑绿 + Stage A 锚格照常
```

## 5. 编辑权释放

`src/rurix-render/src/bin/g31_window_present.rs` 独占编辑权**即时释放**。本次
八段全加性合入已完成且 cargo check 全绿;八块首行注释均带
「G37 W3 frame_cut 合入」标记,grep 该字面即得全部落点。
