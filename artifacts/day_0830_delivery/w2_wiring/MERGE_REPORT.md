# G37 W2 三臂窗口 bin 合入记录（MERGE_REPORT）

- 日期:2026-08-30;执行 = 合入 agent(持 `g31_window_present.rs` 独占编辑权,本报告交付即释放)。
- 输入提案:`postchain_pso/REPORT.md`(§1.5 LUT L1~L7 + §2.5 PSO P1~P5)、`visbuffer/REPORT.md`(§5 A~G)。
- 合入方式:**内容锚定位**(两报告行号为侦察期快照,transparency 臂先行合入已致行号漂移;全部按字面锚落点,零锚点失配)。
- 纪律:禁 GPU/禁 `--release`/禁 target-night 全程遵守;`g14_3_lane_body.rs`、`g14_3_pipeline_perf.rs`、`kernels/`、既有 .spv、`milestones/`、`registry/`、`ci/` 全 0-byte;本次唯一改动文件 = `src/rurix-render/src/bin/g31_window_present.rs`(全部插入带「G37 W2」注释,连接性注释带「G37 W2 合入」)。
- check 命令(每臂后 + 最终,四跑全绿):`cargo check -p rurix-render --features vendor-upscale --bin g31_window_present`(默认 dev/默认 target)。

---

## 1. 臂一:LUT 色彩分级(--lut;#79 五级链第 4 级)

提案 = postchain_pso REPORT §1.5,七锚点 L1~L7,实合入 6 处编辑 + 1 处 0-byte:

| 锚 | 合入后落点(行) | 内容 |
|---|---|---|
| L1 | 220 | `include!("g37_w2/g31_lut_assets.rs");`(lane body include 之后) |
| L2 | 256~262 | `G31_DEFAULT_SPV_ENCODE_LUT = ".tmp/night_0830/spv/g31_display_encode_lut.spv"` + 换载律 doc 注释(字节隔离/默认字面才换/fail-closed 拒显式 --spv-encode 组合) |
| L3 | 6462~6464 | `let mut lut = "off".to_owned();`(spv_* 变量组尾) |
| L4 | 6766~6768 | `"--lut" => lut = take_arg(&args, &mut i),`(--dither 解析臂后;值域校验延后 from_arg) |
| L5 | 7889~7910 | tsrq 换载块后:from_arg 闭集校验 + 互斥集(--fg/--hzb on/--svt on/--slab-table + --cluster-lod/--wp-hlod fail-closed)+ 换载(默认字面才换 `spv_encode = …_LUT`,显式 --spv-encode 同给即 fail)。报告代码块逐字。 |
| L6 | 8682~8689 | era 循环 `let enc_params` → `let mut enc_params`,紧随 `if let Some(l) = lut_asset.as_ref() { g31_lut_assets::extend_encode_params(&mut enc_params, l); }`([134] 门/[135] dim/[136..) 表体尾挂;era 不变量,resize 随车道重建自然重挂) |
| L7 | — | **0-byte**(报告主提案):spv_encode 已换载,encode_spv_json 的 path+sha 自动如实流入主 evidence;「lut 字面进 PASS 行/notes」的裁量建议**未采**(主 evidence schema additionalProperties:false 冻结,不动 PASS 行格式) |

check:**exit 0 绿**(dev 全量重编 2.58s;warning 集 = 基线,零新增,见 §4)。

## 2. 臂二:PSO precache/warmup 账本(默认开;#82/#113)

提案 = postchain_pso REPORT §2.5,锚点 P1~P5(P4 双车道 = 6 处编辑):

| 锚 | 合入后落点(行) | 内容 |
|---|---|---|
| P1 | 223 | `include!("g37_w2/g31_pso_warmup.rs");` |
| P2 | 6465~6467 + 6769~6770 | `let mut pso_report: Option<String> = None;` + `"--pso-report" => …` 解析臂 |
| P3 | 8650~8655 | `'eras: loop` 之前:`let mut pso_ledger = g31_pso_warmup::G31PsoLedger::new();` + `let pso_strict = std::env::var("RURIX_G31_PSO_STRICT").is_ok_and(|v| v == "1");` |
| P4b | 9358~9374 | `G31HzbLane::create` 之前(hzb_meta_json 定型后):`begin_session()` + 遍历 `hz_pass` 逐 pass `register`,miss ⇒ stderr 告警 + strict 即 fail。`Pass::Compute(cp)=(cp.name, cp.spirv)` / `Pass::Raster(rp)=(rp.name, rp.vs_spirv)` 与 rurix-rt 实名核对一致。 |
| P4a | 9409~9425 | `G31TsrLane::create(descs, …)` 之前(descs 定型后):同款,遍历 `descs.passes`(`G31Descs.passes: Vec<Pass>` 实名核对一致) |
| P5 | 10296~10311 | 'eras 循环结束、storm 汇总行之后、「⑦ 多口径稳态统计」evidence 组装区之前:stderr 单行 `[PSO] sessions/unique_variants/pso_runtime_creates` 恒登 + `--pso-report` sidecar 写盘(`rurix.g31.pso_warmup_report.v1`,默认 off = 0-byte) |

两车道 CLI 互斥不并存 ⇒ `begin_session` 每 era 恰一次。账本只覆窗口车道 session 构造面;visbuffer 臂循环后自建 vk 会话不进账本(证据臂非 presented 链,与 PSO 报告口径一致)。

check:**exit 0 绿**(warning 集 = 基线,零新增)。

## 3. 臂三:VisBuffer 档 2 生产证据臂(--visbuffer;#74/#111)

提案 = visbuffer REPORT §5,七锚点 A~G 全加性,报告代码块逐字:

| 锚 | 合入后落点(行) | 内容 |
|---|---|---|
| A | 225 | `include!("g14_3_lane/g31_visbuffer_arm.rs");`(共享体在 g14_3_lane/ 子目录,lane body 本体 0-byte) |
| B | 6645~6652 | `--cluster-stats-out` 变量后:`visbuffer_on/visbuffer_out/visbuffer_samples(=3)/visbuffer_res(="96x54")` 四变量 |
| C | 7082~7096 | `--cluster-stats-out` 解析臂后:`--visbuffer off\|on` 闭集 + `--visbuffer-out/-samples/-res` 四解析臂 |
| D | 7374~7410 | cluster_stats_out 校验块后:闭集校验(on 须随 `--cluster-lod leaf\|on`;samples ≥1;res 形如 96x54 两正整数;out 须随 on)→ `visbuffer_opt`(off = `VisBufferArmOpt::off()`);互斥集随 --cluster-lod 继承零新增 |
| E | 8621~8630 | `cluster_stat_ms_total` 声明后(era 循环外):`visbuffer_sample_set = visbuffer_sample_frames(total, samples)`(off 空 vec)+ `visbuffer_samples_taken` 采集缓冲 |
| F | 9740~9750 | 主循环 cluster 逐帧统计块收束后:`sample_set.contains(&fi)` ⇒ push `VisBufferCamSample{frame, spec, in_w, in_h}`(Copy 零成本,真窗口逐帧相机) |
| G | 10496~10516 | cluster 统计 sidecar 块后、WP/HLOD sidecar 注释前:`run_visbuffer_arm(GTAG, pack, &visbuffer_opt, cluster_opt.threshold_px, &samples)` device 真跑机制链 + `visbuffer_finish` sidecar(`rurix.g31.visbuffer_stats.v1`,独立文件,主 evidence 0-byte) |

check:**exit 0 绿**(warning 集 = 基线,零新增)。

## 4. 最终整体 check + 语义自查

- 最终 `cargo check -p rurix-render --features vendor-upscale --bin g31_window_present`:**Finished dev, exit 0**;IDE linter 零错误。
- warning 如实登记(**全部既有,非本合入引入**):rurix-rt lib 15 条(visbuffer REPORT §6 已登记既有状态);窗口 bin 4 条(2× unused doc comment 于 B1 probe/mip 元信息 `///` 语句注释、2× unused_assignments 于 `svt_era_pt/svt_era_pool`)——位置全在本次未触碰区;`svt_era_pt` 行 HEAD 已在,其余为本战役先行(transparency 等)未提交改动引入,四跑 warning 集恒等零漂移。
- 四段语义自查(flag 解析/校验/换载(采集)/evidence):
  - LUT:解析 6768 / 校验 7892~7909 / 换载 7904 + era 尾挂 8687 / evidence = encode_spv_json 自动流入(0-byte)✓
  - PSO:解析 6770 / strict 校验 8655 + miss 判定 9367/9418 / 账本双车道登记 9361/9412 / evidence = stderr 单行 10301 + sidecar 10309 ✓
  - VisBuffer:解析 7083~7096 / 校验 7377~7410 / 采集 8624 + 9745(无 SPV 换载——消费冻结构建件 sw_visbuffer_u64_spv)/ evidence = 循环后 device 真跑 10507 + sidecar 10514 ✓
  - 组合语义:--lut 与 --cluster-lod 互斥(L5)+ --visbuffer 须随 --cluster-lod(D)⇒ --lut × --visbuffer 传递性 fail-closed,与两报告互斥集一致;三臂间零其他交叉。

## 5. 与报告提案的偏差(如实登记)

1. **零语义偏差**。全部插入内容 = 报告代码块逐字(P5 的 eprintln 仅按 rustfmt 折行,内容同字面)。
2. L7 采报告主提案 0-byte;「lut 字面进 PASS 行/notes」裁量建议未采(理由:主 evidence schema 冻结 + 不动 PASS 行机读格式)。
3. P5 落点取「'eras 结束后、⑦ 统计区之前」(报告允许「任一收尾点」);headless 不构成此点之前的早退,全路径覆盖。报告备选「evidence 写出函数入口」未采(无必要)。
4. L2/L3/L4 连接注释文字为合入侧撰写(报告未给字面,内容 = §1.4 换载律/闭集语义),P3/P4/P5 注释在报告字面基础上加「G37 W2 合入」标记。
5. 行号全量漂移(transparency 臂先行),一律内容锚定位——报告已预告,非偏差。

## 6. GPU 验收命令汇总(抄自两报告,留验收窗执行;本合入纪律禁 GPU)

### 6.1 LUT + PSO(postchain_pso REPORT §5)

1. **LUT off 锚**:`--frames 8 --warmup 2 --hidden` digest == `55e4a92d…`(all-off)+ `--quality full` 96f == `5db2e7d7…`(十六臂)——off 不载新 SPV,必须零漂移。
2. **LUT on 双跑**:`--lut neutral` 与 `--lut warm` 各双跑位级一致;neutral vs warm digest 必不同;warm 对 off 的 A/B(无 AE 对照,day_0829 教训)呈暖移方向(R 均值升/B 降)。
3. **device/host 对拍(可选加严)**:`--dump-present-raw` + host `g31_lut_assets::sample_trilinear_f32` 复算探针像素(γ 前域一致到 1 LSB)。
4. **PSO 守护**:任意臂 + `--window-storm 3 --pso-report pso.json` → `pso_runtime_creates == 0`、`sessions == 1 + resize_eras`;RED 臂 = 临时向 era≥1 注入异 SPV(或 `RURIX_G31_PSO_STRICT=1` 下人为换载)须告警/fail。
5. VUID=0 全程;帧时记账照旧(LUT 段预期 ≪1ms)。

### 6.2 VisBuffer(visbuffer REPORT §7,PowerShell)

```powershell
# 0) 构建(release 归验收窗)
cargo build --release -p rurix-render --features vendor-upscale `
  --bin g14_3_pipeline_perf --bin g31_visbuffer_wiring
cargo build --release -p rurix-asset --bin g31_cluster_lod_bake

# 1) 资产链两步(host,无 GPU;已有 .rxcp 可复用跳过)
target\release\g14_3_pipeline_perf.exe --dump-scene --scene bistro-interior `
  --out .tmp\g37_w2\bistro.rxcs
target\release\g31_cluster_lod_bake.exe --scene-dump .tmp\g37_w2\bistro.rxcs `
  --out .tmp\g37_w2\bistro.rxcp --double-build

# 2) 独立冒烟(GPU):期望 PASS samples=3 res=96x54 退 0;双跑 digest 相等
target\release\g31_visbuffer_wiring.exe --cluster-pack .tmp\g37_w2\bistro.rxcp `
  --error-px 2.0 --samples 3 --evidence .tmp\g37_w2\vis_standalone.json

# 3) 窗口臂真跑(GPU):期望「visbuffer 帧 N」×3 +「visbuffer 臂 OK samples=3」
cargo build --release -p rurix-render --features vendor-upscale --bin g31_window_present
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\g37_w2\bistro.rxcp `
  --visbuffer on --visbuffer-out .tmp\g37_w2\vis_window.json `
  --cluster-stats-out .tmp\g37_w2\cl_stats.json `
  --evidence .tmp\g37_w2\window_on_ev.json

# 4) off == 锚不动:同 3) 去 --visbuffer 三旗标,digest 面一致;
#    默认全 off 锚 python ci\g31_cluster_lod_smoke.py 复跑绿 + Stage A 锚格照常
target\release\g31_window_present.exe --frames 24 --warmup 2 --tier 100 `
  --headless-smoke --auto-move dolly `
  --cluster-lod on --cluster-error-px 2.0 --cluster-pack .tmp\g37_w2\bistro.rxcp `
  --evidence .tmp\g37_w2\window_off_ev.json

# 5) fail-closed 反证(可选 RED):--visbuffer on 无 --cluster-lod → 拒;
#    --visbuffer-out 无 --visbuffer on → 拒
```

## 7. 编辑权

`g31_window_present.rs` 独占编辑权随本报告交付**释放**。
