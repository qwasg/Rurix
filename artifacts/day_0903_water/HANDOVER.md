# day_0903_water 交接单(G41 水面渲染前端,2026-09-03)

> 入役 HEAD `b276de60`(工作树叠加 G40 云面 + 本役 G41 水面,均未 commit)。
> **未 commit,入库归 owner**。全部 GPU 真跑 `RURIX_REQUIRE_REAL=1` +
> `RURIX_VK_VALIDATION=1`,VUID 0。
> 结论件 = [`REPORT.md`](REPORT.md);设计与许可分析 = [`rfcs/0050`](../../rfcs/0050-water-surface-rendering.md)。

## A. 门台账

| 门 | 命令 | 终态 |
|---|---|---|
| `g41.water.surface` | `py -3 ci/g41_water_smoke.py --gate g41.water.surface` | **PASS**(11 facts) |
| 门自身红绿 | 注入语法错 → rc=1;复原 → rc=0 | **验证通过**(反 YAML-only) |
| host 金标准 | `cargo test -p rurix-render --lib world::water_surface` | 24 passed / 0 failed |
| 波对拍 | `g41_water_probe --frames 90` | `1.2218952e-6` 在带内 |
| 对拍 RED 臂 | 带收紧 1e-9 | 如期红 |
| schema | `py -3 ci/check_schemas.py` | PASS |
| number ledger | `py -3 ci/check_number_ledger.py` | PASS |
| fmt(本役文件) | `cargo fmt --all -- --check` | 本役文件零 diff |

evidence:`evidence/g41_water_gate_*.json`(战役目录内;非 `evidence/`——后者由
`check_schemas.py` 按基准件 schema 强校验,本门是渲染特性门,同
day_0902_rain_night 律)。

## B. 交付面台账(git 工作树,未 commit)

| 文件 | 变更 |
|---|---|
| `src/rurix-render/src/world/water_surface.rs` | **新增** host 金标准 + 24 单测 |
| `src/rurix-render/src/world/mod.rs` | 加性:`pub mod water_surface;` + 文档行 |
| `src/rurix-render/kernels/g41_water_{wave,scene,blur,surface,encode}.rx` | **新增** 五 kernel |
| `src/rurix-render/src/bin/g41_water_present.rs` | **新增** 展示车道 |
| `src/rurix-render/src/bin/g41_water_probe.rs` | **新增** 对拍探针 |
| `src/rurix-render/Cargo.toml` | 加性:两个 `[[bin]]`(附设计说明注释) |
| `ci/g41_water_smoke.py` | **新增** 门 |
| `rfcs/0050-water-surface-rendering.md` | **新增** Mini-RFC Draft |
| `registry/number_ledger.json` | 只追加:RFC `on_tree_max` 49→50、`next_free` 50→51 + notes 一行 |
| `.gitignore` | 只追加:`day_0903_water` 块(raw / raw.* / mp4 / clip_frames / __pycache__ 不入库;rain_night 同形) |
| `artifacts/day_0903_water/**` | 战役目录:REPORT / HANDOVER / CAMPAIGN_LOG / DELIVERABLES / 冻结带 / tools(2 脚本)/ evidence(门件)/ `lagoon_orbit.mp4.json` 登记件;**留盘不入库**:previews 14 图、`lagoon_orbit.mp4`、`clip_orbit.raw.f*` 360 帧、`clip_frames/` 300 PNG |

**0-byte 未触碰**(机器可核):`g14_3_lane_body.rs`、`g31_window_present.rs`、
`display/`、`world/water.rs`(M113 冻结带)、全部既有 `kernels/*.rx`、
`milestones/**`、既有契约与 digest 锚。本役车道不 include 共享体、不复用生产 kernel。

## C. 留窗登记(如实,不冒充)

| # | 项 | 归属 |
|---|---|---|
| W-1 | **解析礁石移除**(rurixc「`if` 包 `while`」缺陷同型;详见 REPORT §6.1) | rurixc 缺陷定位修复 / 改走 TLAS |
| W-2 | 体积光完整降噪管线(半分辨率 + MV 重投影 + À-trous + JBU) | 需多次散射时 |
| W-3 | 光子累积焦散(原子 + À-trous) | 需全反射/多次折射焦散时 |
| W-4 | `--env-lut` 丢方位结构(二维 LUT 归并) | 改绑 equirect 环境图 |
| W-5 | `spv_inject_no_contraction` 第四副本 | 单源折叠 |
| W-6 | 相机不支持下潜 | 水下渲染另立 |
| W-7 | 破碎波形(浅化只收幅) | — |
| W-8 | **治理面**:Mini-RFC 仅 Draft(D-409 未评审)、未立 milestone 契约、未领 CI_step 号、未 commit | **owner** |
| W-9 | 未在真实 glTF 场景(Bistro 等)上验证 | 需接生产装载面时 |

## D. 后续窗口建议(优先级序)

1. **owner 决策 W-8**:是否把 G41 升格为正式 milestone(参照 G35/RFC-0049 全流程),
   或维持展示战役形态。**〔2026-09-04 判档落档:维持「展示战役形态」,不升格正式 milestone;RFC-0050 维持 Draft(D-409 对抗评审不在本波);升格重开条件 = 水面接进生产车道的真实需求成立(即 W-9 前置「Mega 车道真深度」解决且有带水场景);W-8 其余分项 —— 未领 CI_step 号 = 门用符号键 `g41.water.surface`,维持;未 commit = 本次 owner 入库波处置。本项闭合〕**
2. **W-1 归因**:把「`if` 包 `while` × 4 循环」做成 rurixc 的最小复现件(本役
   已有现成对照:同文件两循环正常、四循环失效),对 `g31_realism.rx` 已登记的
   同型缺陷是一份独立佐证。
3. **W-9**:若要把水面接进生产车道,前置是真深度——需先解决 Mega 车道
   `U_SCENE_DEPTH` 的 quirk 域,或走 HZB 车道形态。
4. W-2/W-3 按画质需求排期;W-4 若要做全景背景则必须先做。
