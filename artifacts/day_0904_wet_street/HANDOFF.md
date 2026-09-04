# 换机接手单(四层入库 + 湿街;2026-09-04 11:14 收束)

> 本机停做。未完成项原样留给下一台。分支
> `cursor/g5-rd038-w1w2-degradation-waves`,远程 `origin = https://github.com/qwasg/Rurix.git`。
> **不要从 `main` 另开**——三层已入库 commit 在本分支。

## 已落地(git)

| commit | 内容 |
|---|---|
| `55a582d0` | `feat(g40)` G40 系统(day_0901 + G39 两件判档 + A2 回填) |
| `8476168c` | `feat(g40-clouds)` 云面 + `ci/g40_cloud_smoke.py` + day_0903_clouds |
| `55f12521` | `feat(g41)` 水面 + RFC-0050 + ledger + day_0903_water |
| (本收束) | `feat(g35)` 雨丝+雨夜 + G35-3/4 evidence |
| (本收束) | `wip(g42)` 湿街半成品(不宣称完工) |

README 战役表四行 / TODO v1.2.4~v1.2.6 **未写**(原计划 commit 5,缺雨夜 hash 时停)。

## 门

| 门 | 状态 | 接手 |
|---|---|---|
| `g35.wave3.render` | PASS `evidence/g35_render_gate_20260904T023523Z.json` | 已入库 |
| `g35.wave4.sort_oit` | PASS `evidence/g35_sort_oit_gate_20260904T024359Z.json` | 已入库 |
| `g36.wave1.geo_composition` | 本机仍在跑,结果**未落盘** | **重跑**(先等本机锁释放,或直接在接手机跑;持锁自理,不要外包一层 `gpu_device_lock.py`) |
| `g40.clouds.present` | 未跑;`DELIVERABLES.json` `gate_status=pending` | `--selftest` 再 `--gate`;evidence 只写 `artifacts/day_0903_clouds/evidence/` |
| `g42.wet.street` | 脚本在、车道未接线 | 先接线再跑 |

本机 G36:`py -3 ci/g36_geo_composition_smoke.py`(约 10:59 起,已过 `g34_mixed`)。换机后其 evidence 不会自动出现在本分支。

## 湿街还没做的

已有(未接线):

- `recon/R0.md` + `CAMPAIGN_LOG.md`(W0)
- `world/wet_ground.rs`(20 单测;公式面与 kernel 同源;参数段 **`[49..56)`**,不是计划草稿的 `[42..48)`)
- `kernels/g42_direct_gi_wet.rx`(本机 `rurixc` + `spirv-val` 已绿,SPV 在 `.tmp/g42/spv/` 不入库)
- `ci/g42_wet_street_smoke.py`(八 facts;CLI 认 `--wet` / `--wet-dark` / `--wet-spec` / `--puddle` / `--spv-scene-wet`)

未做:

1. `world/mod.rs` 加 `pub mod wet_ground;`(入库波切分面,当时故意未动)
2. `g35_particle_lane.rs` 加湿旗标 + `pack_frame_params` 后覆写 `[49..56)` + `--wet on` 换载 `--spv-scene-wet`
3. release 构建 `g35_particle_lane`、host 单测、门、C2/C1 定帧与 C1 dolly 出图
4. REPORT / HANDOVER / DELIVERABLES / TODO v1.2.7
5. README zh/en 战役表 + TODO v1.2.4~1.2.6(应用入库 hash)

`--wet off` 锚:C2 `7a5ec1bc…0ced` / C1 `0985ebb8…0dc5`。溅落不做,留窗。

## 接手命令(顺序)

```powershell
git fetch origin
git checkout cursor/g5-rd038-w1w2-degradation-waves
git pull --ff-only origin cursor/g5-rd038-w1w2-degradation-waves

$env:RURIX_REQUIRE_REAL="1"; $env:RURIX_VK_VALIDATION="1"
# 不设 CARGO_TARGET_DIR
py -3 ci/g36_geo_composition_smoke.py --gate g36.wave1.geo_composition
py -3 ci/g40_cloud_smoke.py --selftest
py -3 ci/g40_cloud_smoke.py --gate g40.clouds.present
```

然后按原计划:docs 战役表 → 湿街 W1 接线 → 门 → 出图。冻结面(`g14_3_lane_body.rs` / `g14_3_direct_gi.rx` / `g31_window_present.rs` / `world/water.rs`)保持 0-byte。
