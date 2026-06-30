# M7 G0 软光栅 safe 覆盖率报告(契约 G-M7-3 / D-M7-6)

> 所属契约:[M7_CONTRACT.md](M7_CONTRACT.md) §4 G-M7-3
> 版本:v1.0(2026-06-16,M7.5)
> 机器事实源(只增不删不改):[../../evidence/soft_raster_smoke.json](../../evidence/soft_raster_smoke.json)
> 计数器:`m7.counter.soft_raster_kernels_safe`(预设 ≥4,[m7_budget.json](m7_budget.json))

---

## 1. 结论

G0 compute 软光栅四 kernel(binning / tile 光栅 / 深度 / tonemap)**全 safe 代码达标**:
safe 覆盖 **4 / 4**,**unsafe 落点为空**(`unsafe_kernels: []`)。因零 unsafe,本轮 `unsafe-audit/`
**无新增注册条目**,views 扩展清单(05 §7 / RXS-0078 / `hir::ViewOp`)**无反哺项**。M7.5 软光栅
L3 端到端基准(D-M7-5)所串联的同一批 device kernel 亦零 unsafe,host 编排 harness 为 Python,
不引入 Rust unsafe 边界。

| 项 | 现状 | 背书 |
|---|---|---|
| safe kernel 覆盖数 | **4**(≥ 预设 4) | `m7.counter.soft_raster_kernels_safe` PASS |
| unsafe 落点 | **0** | `evidence/soft_raster_smoke.json` `unsafe_kernels: []` |
| unsafe-audit 新增条目 | **无** | 全 safe,无 `// SAFETY:` 块需注册 |
| views 扩展清单反哺 | **无** | 无 unsafe 逃生 → 无待反哺算子 |

## 2. safe kernel 清单(计数源 = `safe_kernels` 去重基数)

| # | kernel | spec 条款 | device 源 | host 参考(同义) | safe 依据 |
|---|---|---|---|---|---|
| 1 | `sr_binning` | RXS-0118 | [src/rurix-rt/kernels/sr_binning.rx](../../src/rurix-rt/kernels/sr_binning.rx) | `bin_triangles` | 每 tile 单 agent 升序遍历,atomics-free,无 `unsafe` |
| 2 | `sr_raster_tile` | RXS-0119 | [src/rurix-rt/kernels/sr_raster_tile.rx](../../src/rurix-rt/kernels/sr_raster_tile.rx) | `shade_pixel` | 每像素独立边函数/重心,无 `unsafe` |
| 3 | `sr_depth` | RXS-0120 | [src/rurix-rt/kernels/sr_depth.rx](../../src/rurix-rt/kernels/sr_depth.rx) | `render_hdr` | 每像素 agent 固定片元序 less 合成,atomics-free,无 `unsafe` |
| 4 | `sr_tonemap` | RXS-0121 | [src/rurix-rt/kernels/sr_tonemap.rx](../../src/rurix-rt/kernels/sr_tonemap.rx) | `tonemap_channel` | 每分量 agent 独写量化,无 `unsafe` |

host 参考 crate [src/soft-raster/src/lib.rs](../../src/soft-raster/src/lib.rs) 与 [src/image-io](../../src/image-io)
均继承 workspace lints `unsafe_code = "deny"`([Cargo.toml](../../Cargo.toml) L12–16),源码零 `unsafe` 块。

## 3. 可复现审计事实(facts)

| # | claim | command | result |
|---|---|---|---|
| F1 | 四 kernel device codegen(NVPTX IR)全通过 | `rurixc <sr_*.rx> --emit=nvptx-ir`(经 `ci/soft_raster_smoke.py`) | 4× exit 0,ptxas ran(`device_facts`) |
| F2 | 软光栅全 safe(零 unsafe) | workspace `unsafe_code=deny` + 源扫描 | `unsafe_kernels: []`;crate 无 `unsafe {` 块 |
| F3 | host 参考帧确定性(6 帧逐字节复现) | `ci/soft_raster_smoke.py`(两次落盘 content SHA-256 比对) | `frame_sha256_match: true` |
| F4 | 反 YAML-only 红绿 | 篡改一帧像素 R/B 通道 → SHA 变(红);复原 → exit 0(绿) | `redgreen.red_sha_changed: true` |
| F5 | L3 端到端管线四 stage 真跑零 unsafe | `bench/sr_pipeline_bench.py`(Driver API 串联同批 kernel) | correctness PASS,无 unsafe(D-M7-5,[../../evidence/sr_l3_20260616_agg.json](../../evidence/sr_l3_20260616_agg.json)) |

## 4. unsafe 落点与 views 反哺(本轮为空)

契约 G-M7-3 约定:凡落 unsafe 的 kernel 须每 `unsafe` 块 `// SAFETY:` 并在本报告留痕原因,
反哺 views 扩展清单(05 §7:`split` / `group` / `transpose` / `reverse` / `zip`;RXS-0078:
`split_at` / `chunks` / `windows`;`src/rurixc/src/hir.rs::ViewOp`)。

**本轮零 unsafe**:四 kernel 以 `View<global>` 索引 + 每像素/每 tile/每分量单一 agent 独写实现
覆盖/深度/量化,未触发原子合成或别名可变写,无需 unsafe 逃生。故:

- `unsafe-audit/` 无新增条目(仅既有 [unsafe-audit/rurix-rt.md](../../unsafe-audit/rurix-rt.md) U1–U8 运行时原语)。
- views 扩展清单**无待反哺算子**;后续若软光栅引入原子深度合成 / tile 间共享可变 view 才触发
  反哺(届时按本报告 §4 体例追加)。

## 5. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-16 | 初版(M7.5,D-M7-6):汇总四 kernel safe 覆盖 4/4 + unsafe 落点空 + 无 views 反哺;事实源 `evidence/soft_raster_smoke.json`;计数器 `m7.counter.soft_raster_kernels_safe` PASS |
