<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C7 性能剖析与调试工具面） -->
# Rurix 渲染器剖析与调试指南（profiler / Nsight 标注 / RenderDoc 捕获）

> 所属：G31+ 波 C Task C7（性能剖析与调试工具面，G31_PLUS_COMMERCIAL_RENDERER_TODO §5 #54）。
> 兑现面：`--profile-json` 统一 profiler 输出面 + VK_EXT_debug_utils 逐 pass 标注 +
> RenderDoc 捕获兼容面；验收锚 = **外部用户可自助定位帧内热点**。
> 口径纪律（先读 §1）：本文数字**全部**引在案 measured_local（RTX 4070 Ti + Vulkan 本机真跑，
> 门 `g31.waveC.profiling` 2026-08-26 复跑件），禁编造新数字；你的机器上重测会得到不同绝对值。
> 姊妹篇：[performance_tuning.md](performance_tuning.md)（调优杠杆与基线） ·
> [integration_guide.md](integration_guide.md) · [feature_matrix.md](feature_matrix.md)。

---

## 1. 口径纪律（勿混）

1. **默认关，开启零渲染语义变更**：`--profile-json <path>` 不加 = 零收集零写盘；加上 =
   仅 host 侧簿记 + JSON 落盘。机器证明 = 门腿 on/off 双臂同参复跑 **digest 位级一致**
   （g31 双锚 presented+render；g14_3 last_frame；§5 在案数字）。
2. **GPU 段 = device timestamp，不是墙钟**：`gpu_passes` 各段 =
   `DeviceFrameTelemetry` 逐 pass GPU timestamp（`vkCmdWriteTimestamp2` 双点 ×
   `timestampPeriod` 驱动实采），与 host 墙钟段（`cpu_segments`/`frame_segments`）
   是**两个时间域**——GPU 段之和 ≤ 帧墙钟是物理下界关系，不是恒等关系（§4 恒等式）。
3. **渲染口径不含 present/digest 税**：g31 窗 `render_wall` 含 BGRA8 8.3MB 强制回读
   （`render_includes_forced_readback=true`，evidence 同口径）；`present_wall`/`digest`
   税单列永不混入渲染帧率口径（performance_tuning §1 同律）。
4. **统计窗**：post-warmup（`frames_measured == --frames`，与 evidence `real_render_frame_ms`
   / `frame_ms_production_mean` 同窗）；统计组 = mean/p50/p99/min/max（线性插值 percentile）。

## 2. `--profile-json` 快速上手

### 2.1 真窗口生产车道（g31_window_present）

```powershell
.\target\release\g31_window_present.exe --frames 120 --warmup 10 --hidden `
    --evidence .tmp/ev.json --profile-json .tmp/profile.json
```

输出 schema = `rurix.g31.profile_output.v1`
（机器校验面 [milestones/g31/g31_profile_output_schema.json](../../milestones/g31/g31_profile_output_schema.json)）。
五段 GPU 分解（telemetry 声明序）：

| pass | 内容 |
|---|---|
| `g14_3_direct_gi` | scene megakernel（ray query 直接光 + 阴影射线） |
| `g14_mv` | 相机 MV |
| `g14_8_tsr_resample` / `g14_8_tsr_resolve` | TSR 重建两段 |
| `g31_display_encode` | device 显示编码（ACES1.3 RRT+ODT + BT.1886 → BGRA8） |

FG 开（`--fg x2|x3`）追加 `g31_mv_negate`/`g26_framegen_fg*`/`g31_display_encode_fg*`；
HZB 开（`--hzb on`）为 HZB 车道全 pass 列（`g31_hzb_*`/`g27_hzb_*`）——profiler
按 telemetry 声明序全量直出，**不需要**先验 pass 表。

### 2.2 bench 车道（g14_3_pipeline_perf）

```powershell
.\target\release\g14_3_pipeline_perf.exe --bench --scene bistro-interior --tier 100 `
    --backend tsr_device --frames 160 --warmup 10 `
    --out-root .tmp/bench --profile-json .tmp/profile.json
```

四段 GPU 分解（无 encode——bench 车道 TSR 输出即终点）。**闭集约束（fail-closed
拒跑不冒充）**：首接面 = `--bench --backend tsr_device --inflight 1` 静态臂；
vendor 双臂（dlss_sr/fsr_3_1_5）/FIF 流水（inflight 2|3）/`--dyn-demo`/`--skin-demo`
组合未接线，CLI 直接拒跑（归后续波）。

### 2.3 输出字段字典

| 字段 | 语义 |
|---|---|
| `gpu_passes[]` | 逐 pass GPU 段（`unit=gpu_timestamp_ms`）mean/p50/p99/min/max |
| `cpu_segments[]` | `cpu_record`/`cpu_submit`/`cpu_fence_wait`（telemetry 实测）+ `readback_convert`（host 回读转换；g14_3 臂属 tail 段） |
| `frame_segments[]` | g31：`render_wall`（含强制回读）/`present_wall`（present 腿）/`digest`；g14_3：`frame_wall`/`production_wall`（=frame−tail）/`tail`（回读/校验/digest 非生产段） |
| `identity` | 恒等式字段（§4） |
| `debug_labels` | 标注面状态（§6；`active` + `annotated_pass_count`） |
| `profiler_overhead` | profiler 自身开销如实登记（`assembly_ms` = JSON 组装段实测；逐帧簿记 ~µs 级，on/off mean 差属跑间抖动不冒充 profiler 税） |
| `render_digest` | 本跑末帧渲染 digest（与 evidence/receipt 锚同值） |

### 2.4 定位帧内热点的工作流

1. **先看 `gpu_passes` mean/p99 排序**——最大段即 GPU 热点。例：bistro 1080p t100
   五段中 `g14_3_direct_gi`（scene）≈ 60% GPU 时间 = 第一优化对象；`g14_mv` ~0.03ms
   不值得动。
2. **再看 `identity.gpu_sum_mean_ms` vs `render_wall_mean_ms`**——GPU 占比低而墙钟高
   ⇒ 瓶颈在 host/同步侧（fence 等待、回读、present 背压），不是 kernel。
3. **分解验证**：`cpu_fence_wait` 高 + GPU 和远小于墙钟 ⇒ 提交-同步结构问题
   （考虑 `--inflight 2|3`，performance_tuning §3.1）；`readback_convert`/digest 税高
   ⇒ 测量面固有（生产帧零回读，诚实口径见 §1.3）。
4. **p99 ≫ p50 ⇒ 抖动源定位**：逐段 p99/p50 比锁定是哪一段抖（冷启/背压/闭环保守
   重渲——HZB 面 `closure_extra_gpu` 见 g31_hzb_wiring evidence）。

## 3. 在案分解数字样例（measured_local，2026-08-26 门复跑件）

`g31_window_present --frames 24 --warmup 6 --hidden`（真窗口；bistro-interior t100）：

| 段 | mean_ms | p50_ms | p99_ms |
|---|---|---|---|
| g14_3_direct_gi（scene） | 0.947 | 0.959 | 1.128 |
| g14_mv | 0.033 | 0.033 | 0.035 |
| g14_8_tsr_resample | 0.143 | 0.142 | 0.164 |
| g14_8_tsr_resolve | 0.401 | 0.394 | 0.511 |
| g31_display_encode | 0.103 | 0.105 | 0.106 |
| **GPU 和（identity.gpu_sum）** | **1.627** | — | 1.80(p99) |
| cpu_record / submit / fence_wait | 0.284 / 0.062 / 2.875 | — | — |
| render_wall（含 8.3MB 回读） | 5.106 | 4.647 | — |
| present_wall | 3.534 | 3.120 | — |
| host_residual（§4） | 1.666 | — | — |

`g14_3_pipeline_perf --bench` 同窗（24+6；无 present/encode 段）：
scene 0.981 / mv 0.033 / resample 0.150 / resolve 0.402，**GPU 和 1.566**，
production_wall 2.659，host_residual 0.070。on/off 双臂 digest 位级一致：
g31 presented `30b0ff46…` + render `101998653e95…`、g14 `101998653e95…`（双锚，
evidence/g31_profiling_20260826T143523Z.json 在案）。

## 4. 分解恒等式（门检面，容差 = 三面同一事实源）

```
gpu_sum_mean ≤ render_wall_mean + 0.10      （GPU 忙时不超过帧墙钟——物理下界）
−0.10 ≤ host_residual_mean ≤ 2.00           （未归属宿主段包络——分解完整性）
```

- `host_residual` := render_wall −（cpu_record + cpu_submit + cpu_fence_wait +
  readback_convert）；g14_3 臂 := production_wall − telemetry 三分项（readback_convert
  属 tail 不入和，口径差如实注明）。
- residual 内含：帧参数打包/相机 jitter 求逆/telemetry 提取等未插桩宿主工作 +
  present 背压经 fence/submit 边界的传导噪声。2.00ms 包络 = bistro 1080p 真窗口
  车道实测上界（1.666）× ~1.2 余量取整——**判分解断裂，不判性能预算**。
- 容差字面三处同一事实源：双 bin JSON 组装代码 / 输出 schema `const` /
  `ci/g31_profiling_smoke.py` `IDENTITY_*_TOL_MS`——**改动三面同步**（selftest 机核互核）。

## 5. Nsight 标注（VK_EXT_debug_utils 逐 pass label）

- **机制**：`src/rurix-rt/src/render_exec.rs` 建 instance 时枚举
  `VK_EXT_debug_utils`——**在位即启用**（validation 关也在位），
  `record_frame_body` 逐 pass 录 `vkCmdBeginDebugUtilsLabelEXT`/
  `vkCmdEndDebugUtilsLabelEXT`（label 串 = telemetry pass 名，包裹 timestamp
  区间 + pass 本体）。扩展/符号 absent ⇒ 双 `None` 录制零开销跳过
  （**fail-silent 不崩**；`label_names` 不分配）。
- **标注约定**：label 名 == `--profile-json` `gpu_passes[].name` == telemetry
  pass 名（`g14_3_direct_gi` 等）——三面同一名词，工具间互查无映射表。
- **消费面**：Nsight Graphics（frame debugger 事件树按 label 分组）/ RenderDoc
  （Event Browser 同名节点）。Nsight Systems/Compute 不消费 cmd label（工具定位
  不同，勿混）。
- **机器核验**：profile JSON `debug_labels.active` + `annotated_pass_count`
  （本机 = true + 5/4 全段；门 fact `debug_labels_recorded`）。**本机 Nsight
  Graphics 不在机**（PATH(nsg) + 安装位双探 absent）——UI 复核未跑，如实降级
  登记（evidence `capture_compat.dev_env_degrade`），不冒充。

## 6. RenderDoc 帧捕获

### 6.1 使用要点（RenderDoc 在机窗）

```powershell
renderdoccmd.exe capture -w -f 8 -c 3 -o .tmp/g31cap -- `
  .\target\release\g31_window_present.exe --frames 12 --warmup 6 --hidden --evidence .tmp/ev.json
```

帧定界 = swapchain present（`vkQueuePresentKHR` 标准腿）；捕获后 Event Browser
按 §5 label 名辨识各 pass；**--profile-json 可与捕获同跑**（host 面零干扰）。

### 6.2 捕获兼容面（本门核验纪律）

- **兼容断言**：① 全门腿 `RURIX_VK_VALIDATION=1` 真跑 rc=0（harness 逐帧
  `validation_error_count==0` fail-closed——规范合法 API 使用是捕获前提）；
  ② 捕获不兼容 API 模式 blocklist 0 命中（`render_exec.rs`/`vk_g31_present.rs`
  静态核验：无 discard rectangles/sparse/video queue/NVX binary import/
  low_latency/cluster AS/opacity micromap）;③ present = 标准 swapchain 腿
  （staging→copy→present,无外部内存跨界——DLSS exportable 车道非本 bin 面）。
- **本机状态（如实）**：RenderDoc **不在机**（PATH + `C:\Program Files\RenderDoc`
  双探 absent）——真捕获腿未跑，门以静态核验面兑现 + `DEV_ENV_DEGRADE` 降级
  登记（evidence/g31_profiling_20260826T143523Z.json `capture_compat`），
  **不冒充真捕获**。RenderDoc 在机窗复跑本门自动切换 `real_capture` 口径
  （.rdc 产出 + 尺寸阈核验）。

## 7. CI 门

```powershell
py -3 ci\g31_profiling_smoke.py --selftest                  # 判读器红绿自证（无 GPU 依赖）
py -3 ci\g31_profiling_smoke.py --gate g31.waveC.profiling  # 真门（构建 + 4 GPU 腿 + 7 facts）
```

七判据：① profile JSON schema 合规 ② 分解全 measured ③ 分解和≈帧墙钟恒等式
④ 标注段存在 ⑤ profiler on/off 位级零漂移 ⑥ 捕获兼容核验（真捕获/静态两臂如实）
⑦ 工具探测登记。三态：无 GPU/资产/SPV → `DEV_ENV_DEGRADE` SKIP（`RURIX_REQUIRE_REAL=1`
下翻硬 FAIL，禁 mock 充真跑）。门 evidence 前缀 `g31_profiling_`（check_schemas
三处纯追加已驻留）。

## 8. 遗留与边界（如实登记）

- g14_3 `--profile-json` 未接臂：vendor 双臂 / inflight 2|3 / dyn / skin（CLI
  fail-closed;归后续波,不冒充）。
- Nsight Graphics / RenderDoc 不在机——标注 UI 复核与真捕获两腿待工具在机窗
  （门自动切换口径;静态面已兑现）。
- p99 尖峰（冷启首帧/末帧 digest 帧）为测量面固有,定位热点以 mean/p50 为主读数。
