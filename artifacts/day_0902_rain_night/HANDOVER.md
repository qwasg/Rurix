# day_0902_rain_night 交接单(BistroExterior 雨夜街景粒子雨展示,2026-09-03)

> 入役 HEAD `b276de60`(工作树叠加 G40 未提交面 + g35 雨丝模式未提交面,本役再叠加)。
> **未 commit,入库归 owner**。全部 GPU 真跑经 `g35_run.py` → `run_render.py`
> (`ci/gpu_device_lock.py` 排他锁)+ `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`,
> VUID 0;账本 `render_runs.jsonl` 27 条。
> 结论件 = [`REPORT.md`](REPORT.md);逐波记录 = [`CAMPAIGN_LOG.md`](CAMPAIGN_LOG.md);
> digest / 帧时汇总 = [`DELIVERABLES.json`](DELIVERABLES.json)(`summarize_runs.py` 程序产)。

## A. 门台账

**本役无自有门**(展示战役,非 milestone 形态);下表为回归事实(机器证明,REPORT §4 末段 / §6)。

| 回归项 | 口径 | 终态 |
|---|---|---|
| off 锚 | `--particles off --static-camera --frames 160 --warmup 10` | `render_digest sha256:c1d28ad7…6c02` **== Stage A 锚 PASS** |
| on 缺省路径零漂移 | orbit 48+6 双跑 + 基线 exe `.tmp\g35_lane_baseline.exe` 三者 | presented `92c870e9…89b1` / render `4857b6d4…` / `digest_seq_sha 7cf143b4…`(54 项)**全等** |
| 回读无副作用 | 静态 30+6 帧 ± `--dump-present-every 1` | presented `84a56190…2237` **相等**;36 件各 8,294,408 B,末帧 == 基件 |
| 负例闭集 | 10 条(every0 / every 无 raw / spiral / amp 0·65 / amp 无 auto-move / follow 静态 / emit-max 100·off / emit-max 1024 克隆守卫) | **10/10 rc=1** 中文 FAIL |
| 定帧确定性 | C2 / C1 终版定帧 | presented C2 `7a5ec1bc…0ced` / C1 `0985ebb8…0dc5` |
| 推轨确定性 | `clip_C1_dolly_a/b` 双跑 300+100 帧 | presented `be90966d…009b` / render `313dcfdf…` / `digest_seq_sha 90dc0c30…`(400 项)**位级相等** |
| VUID | `RURIX_VK_VALIDATION=1` 全程 | **0**(首跑缺该 env 被 fail-closed 拒 = 程序化修正,如实登记) |

### 上游门补跑(owner 入库波)

| 门 | 命令 | 终态 |
|---|---|---|
| `g35.wave3.render` | `py -3 ci/g35_render_wiring_smoke.py --gate g35.wave3.render` | **PASS** `evidence/g35_render_gate_20260904T023523Z.json`(9/9 facts;`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`) |
| `g35.wave4.sort_oit` | `py -3 ci/g35_sort_oit_smoke.py --gate g35.wave4.sort_oit` | **PASS** `evidence/g35_sort_oit_gate_20260904T024359Z.json`(9/9 facts;同上) |
| `g36.wave1.geo_composition` | `py -3 ci/g36_geo_composition_smoke.py --gate g36.wave1.geo_composition` | **未收口**(2026-09-04 11:14 换机中断;本机 py 仍持 `gpu_device_lock` 跑至 `g34_mixed` 段,无 `g36_geo_composition_gate_20260904*.json`。接手机须重跑;G36 fact ⑩消费 `.tmp/g35_gates/render/*.spv`,G35-3 已重编) |

本役改动 `kernels/g35_render_splat.rx` / `g35_render_resolve.rx` 两件消费面 kernel,三门为其上游消费者,入库前须补跑;结果由 owner 入库波回填。

## B. 交付面台账(git 工作树,未 commit)

| 文件 | 变更 |
|---|---|
| `src/rurix-render/src/bin/g35_particle_lane.rs` | **唯一代码面** +254/−21:四旗标 `--dump-present-every` / `--auto-move-amp` / `--auto-move dolly-forward` / `--emitter-follow-camera` / `--emit-max`(evidence `showcase` 追加 5 键 + 顶层 `gltf{path,sha256}`) |
| `src/rurix-render/kernels/g35_render_splat.rx` / `g35_render_resolve.rx` | 入役即在树的**雨丝模式面**(更早叠在同文件的兄弟改动),lane 侧对应 `--rain-shutter` / `--rain-occlusion` / `--ev100` / `--scene` |
| `artifacts/day_0902_rain_night/**` | 战役目录:REPORT / HANDOVER / CAMPAIGN_LOG / DELIVERABLES / 契约与探针件 / `g10_corpus/` 三件 / 工具脚本;**留盘不入库**:raw 定帧、`clip_C1.raw.f0000–f0399`(400 件 3.09 GB)、`clip_frames/`(300 PNG)、`logs/` |
| `artifacts/day_0831_site/` | 三件:`render_runs.jsonl` / `bistro_rain_v2.json` / `rain_v5_probe.json` |
| `.gitignore` | 只追加:雨夜段(raw / mp4 / clip_frames / logs 不入库) |

派生资产落 `K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroExterior\`(276 件 / 1,101,085,696 B;**git 零二进制**)。

**0-byte 未触碰**(机器可核,`git diff --numstat` 冻结面数字与入役逐字同):共享体 `g14_3_lane_body.rs`、全部生产 kernel、冻结契约、`registry/`、G10 清单与许可注册表。

## C. 留窗登记(如实,不冒充)

| # | 项 | 归属 |
|---|---|---|
| W-1 | **湿地面无镜面反射**(REPORT §7.3) | 下一役「湿街」 |
| W-2 | 无雨滴溅落 | 下一役「湿街」 |
| W-3 | 无水面涟漪 | 下一役「湿街」 |
| W-4 | 雨丝无风、无阻力、无地面碰撞(穿地后寿命到期消失,靠 TLAS 遮挡隐藏) | 需求触发时 |
| W-5 | **借壳** `scene_id: bistro-interior`;真支持 `--scene bistro-exterior` 需放宽共享体 L524/L740/L749(+ Python 参照 L31/L197)并重锚 `FROZEN_CONSUMED_PATHS` | **owner**(同 TODO #11) |
| W-6 | BistroExterior 正式登记(`g10_asset_license_registry.json` / `g10_corpus_scene_manifest.json` 只追加修订)未做 | **owner** |
| W-7 | FBX2glTF v0.9.7 找到纹理即写盘失败(`Couldn't open file for writing`)根因未定,9/3 第 5 次复现 | 需求触发时 |
| W-8 | 车道 = 逐三角**均值反照率**(非贴图采样);emissive 只可见不投光;店招 / 彩灯串为常量色 | TODO #9 |
| W-9 | 132 材质全 OPAQUE(FBX2glTF 未保留 MASK)⇒ 植被 alpha-cutout 实心化 | 需求触发时 |
| W-10 | `day_0831_site` 的 `bistro_rain.png` / `bistro_dust.png` 系被过期 encode SPV「抬亮」的旧图,建议以 `--ev100` 重出图(REPORT 与 `bistro_rain_v2.png` 已给新口径) | **owner** |

**附注(编排面,如实;REPORT §7.6)**:子agent B 的 `analyze_exterior_scene.py` 重跑失控(逃逸测试 6.8 万次 AABB 全扫)被主 agent 终止;唤醒后子agent配额耗尽,B 包由主 agent 接手完成(性能修正 + 300 s 时间预算自检)。

## D. 后续窗口建议(优先级序)

1. **owner 入库波**:补跑 §A 三门(`g35.wave3.render` / `g35.wave4.sort_oit` / `g36.wave1.geo_composition`)+ 按 §B 表分层 commit。
2. **下一役「湿街」**:湿地面镜面 + 积水 + 可选溅落(消解 W-1/W-2/W-3),沿用本役契约 / 机位 / 雨参;`--wet off` 须位级等于本役 C2/C1 锚。
3. **#11 正式室外场景登记**:消解 W-5/W-6(共享体场景闭集放宽 + 许可注册表与语料清单只追加修订)。
4. **贴图采样管线(#9)进本车道**:消解 W-8(均值反照率 → 贴图采样),W-9 的 alpha-cutout 一并可解。
