# day_0903_clouds 交接单(G40 体积云展示车道,2026-09-03 / 门与文档补齐 2026-09-04)

> 入役 HEAD `b276de60`(工作树叠加 G40 云面 + G41 水面,均未 commit)。
> **未 commit,入库归 owner**。
> 结论件 = [`REPORT.md`](REPORT.md);施工实录 = [`CAMPAIGN_LOG.md`](CAMPAIGN_LOG.md)。

## A. 门台账

| 门 | 命令 | 终态 |
|---|---|---|
| `g40.clouds.present` | `py -3 ci/g40_cloud_smoke.py --gate g40.clouds.present` | **pending**(门为补齐件,尚未真跑) |
| 门语法静态校验 | `py -3 -c "import ast; ast.parse(open('ci/g40_cloud_smoke.py',encoding='utf-8').read())"` | 绿(无输出) |
| 未知门键闭集 | `py -3 ci/g40_cloud_smoke.py --gate bogus` | rc=2(如期拒) |
| host 金标准 | `cargo test -p rurix-render --lib world::clouds` / `world::sky` | 战役当时全工作区 609 tests 全绿 |
| 真窗口实时 | `target/release/g40_cloud_present.exe --preset golden` | 1280×720 ≈ 110 fps(含回读 + present) |

evidence:`artifacts/day_0903_clouds/evidence/g40_cloud_gate_*.json`(战役目录内;
**非**仓库根 `evidence/`——后者由 `ci/check_schemas.py` 按基准件 schema 强校验,
未登记前缀直接红,而本门是渲染特性门不是基准件;day_0902_rain_night /
day_0903_water 同律)。schema id = `rurix.g40.cloud_gate_evidence.v1`。
fail-closed:只有 PASS 才落 evidence。

门的 7 条 fact(闭集,与脚本 `facts` 元组名逐字对应):

| fact | 判据 |
|---|---|
| `kernel_g40_volumetric_cloud` | rurixc 产 SPV + **真调** `spirv-val` 接受 |
| `kernel_g40_cloud_encode` | 同上 |
| `kernels_compile` | 聚合 2/2 |
| `host_gold_tests` | `world::clouds` + `world::sky` 两组 `test result: ok … 0 failed` |
| `arms_distinguishable` | 五臂(noon/clear/golden/sunset + clear 且 `--phi-fwd off`)digest 两两不等 |
| `double_run_bit_equal` | 默认臂 `--preset clear` 双跑 digest 位级相等 |
| `red_arm_kernel_syntax` | 临时副本注入语法错后 rurixc 须 rc ≠ 0 |

## B. 交付面台账(git 工作树,未 commit)

| 文件 | 变更 |
|---|---|
| `src/rurix-render/src/world/sky.rs` | **新增** 解析天空 + 四档预设 |
| `src/rurix-render/src/world/clouds.rs` | **新增** host 金标准 + `CloudFrontend` |
| `src/rurix-render/kernels/g40_volumetric_cloud.rx` | **新增** 主 kernel |
| `src/rurix-render/kernels/g40_cloud_encode.rx` | **新增** 编码 kernel |
| `src/rurix-render/src/bin/g40_cloud_present.rs` | **新增** 展示车道 + 真窗口 |
| `ci/g40_cloud_smoke.py` | **新增** 门(7 facts) |
| `.gitignore` | 只追加:`day_0903_clouds` 块(raw / raw.* / mp4 / `__pycache__` 不入库;day_0903_water 同形) |
| `artifacts/day_0903_clouds/**` | 战役目录:REPORT / HANDOVER / CAMPAIGN_LOG / DELIVERABLES / tools / evidence(待门产);**留盘不入库**:previews 13 图 |

**0-byte 未触碰**(机器可核):`g31_window_present.rs`、`g14_3_lane_body.rs`、
`display/`、全部**既有** `kernels/*.rx`、`milestones/**`、既有契约与 digest 锚。
本役车道**不 include 共享体、不复用任何生产 kernel**;门的 RED 臂只动
`.tmp/g40/red/` 下的临时副本,树内 kernel 字节不动。

## C. 留窗登记(如实,不冒充)

| # | 项 | 归属 |
|---|---|---|
| W-1 | **`--clouds` 臂未接进 `g31_window_present`**(改走独立真窗口;理由见 REPORT §7)。替代路线:scene 与 mv 之间插 pass 就地读改写 `U_OUT_COLOR`——逐像素独立、下游绑定零变更,但需补十来条组合臂的描述符下标常量并**重新收割 `full19` / `RD-045` 锚** | **owner**(2026-09-04 决策:维持独立窗口,本条挂起) |
| W-2 | **froxel 未合流**:`CloudFrontend` 与 `FogFrontend` 写同一 `FroxelVolume`,生产车道**两个都不消费** | 接生产体积面时 |
| W-3 | **无真实 glTF 场景**:本车道纯天空,无地形、无建筑 | 需实景时 |
| W-4 | **治理面**:无 Mini-RFC、未立 milestone 契约、未领 CI_step 号(门用符号键)、未 commit | **owner** |
| W-5 | **噪声体启动现烘**,未落盘缓存(启动固定开销,bin 以 `bake_ms` 打印) | 启动时延成问题时 |
| W-6 | **门尚未真跑**(`gate_status = pending`);密度求值双份字面待 rurixc 补齐 device `fn` 调用后折叠 | 下一个持锁窗口 |

## D. 后续窗口建议(优先级序)

1. **跑门**:`py -3 ci/g40_cloud_smoke.py --gate g40.clouds.present`(需 target-dir 空闲
   + 设备空闲;门自持 `gpu_device_lock`)。绿后把 `DELIVERABLES.json` 的
   `gate_status` 由 `pending` 改为 `pass` 并重跑 `tools/make_deliverables.py`。
2. **owner 决策 W-4**:是否把 G40 升格为正式 milestone,或维持展示战役形态。
3. **W-1 归因备料**:若日后要做 `--clouds` 臂,先评估 `full19` / `RD-045` 重锚的代价;
   本役已把「就地读改写 `U_OUT_COLOR`」这条最小侵入路线写在 REPORT §7,可直接接手。
4. **rurixc 两条硬限**(REPORT §6.1)是跨战役的:device `fn` 调用未接线
   (`vulkan_codegen.rs:2584`)与 `while` 条件 `&&` 破支配序。前者一旦补齐,本役与
   G41 的多处手工内联可同时折叠。
