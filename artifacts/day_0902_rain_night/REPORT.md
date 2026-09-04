# day_0902_rain_night 结论件：BistroExterior 雨夜街景粒子雨展示

> 入役 git HEAD `b276de60`（工作树叠加 G40 + g35 雨丝模式未提交面）；本役 2026-09-03；**未 commit，入库归 owner**。
> 全部 GPU 真跑经 `g35_run.py` → `run_render.py`（`ci/gpu_device_lock.py` 排他锁，`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`）登记 `render_runs.jsonl`（27 条），digest / 帧时汇总见 `DELIVERABLES.json`（`summarize_runs.py` 程序产）。

## 1. 交付件

| 件 | 说明 | sha256 |
|---|---|---|
| `bistro_rain_night_C2.png`（**主图**） | 机位 C2：北街南口回望 NNE，近灯背光、药店绿十字 + 面包店招牌；1920×1080 tier100，`--warmup 100 --frames 96` | `403edaf8c2089a7dad0c66a17d7c4831b70d5866d1f2289958756dcbaeefdfa5` |
| `bistro_rain_night_C1.png` | 机位 C1：北街街心顺街朝 SSW 望 bistro 转角（书店招牌 + SL136/SL151 + 远处彩灯串） | `7de8afefd46b89d2fb3eb19786219c6c80344159b7501c03a2db8b83b4d19b83` |
| `bistro_rain_night_C1_dolly.mp4` | C1 推轨短片：`dolly-forward` 8 m 幅（可见段 6 m）+ 发射器随相机，300 帧 @30 fps = 10.0 s，libx264 crf 21，18,526,990 B | `91906ff283ec26dc6d6c3d1aa8b485040922c5ab9efc93cc4f23257b7908b57c` |
| `probe_C3_cd050_t50.png` | 机位 C3（广场望 bistro 西立面：吊灯笼 + 彩灯串 + 露台）tier50 探针图，留档 | `cccf6d41dd1233788ae7521274981a937be4ccbbfbe0e6342e23cacc0f79d3b1` |
| `contract_rain_night_{C1,C2,C3}_cd050.json` | 借壳展示契约（终版参数）；expect-digest：C1 `5a5e8f70…5f75` / C2 `37eea825…e681` / C3 `3409f255…f21e` | 见 `DELIVERABLES.json` |
| `exterior_scene_facts.json` | 场景事实（世界坐标；灯位 / 机位 / 发射盒） | `8aeaeb53…85ec` |
| `exterior_asset_verify.json` | 资产回接校验件（9/9 PASS） | `d0329960…3643` |

raw 定帧（8,294,408 B/件）、`clip_C1.raw.f0000–f0399`（400 件 3.09 GB）、`clip_frames/`（300 PNG）、`logs/` 全量日志：留盘、不入库（`.gitignore`）。

## 2. 资产：BistroExterior glTF（URI 回接派生）

- 源：8/15 FBX2glTF v0.9.7「无纹理臂」产物 `H:\rurix\.tmp\g10_conv_ext\BistroExterior.gltf`（几何 / 材质完整，274 image 为 1×1 data-URI 占位）；工具在找到 `Textures\` 时写 `.gltf` 必报 `Couldn't open file for writing`（9/3 `fbx2gltf_retry.log` 第 5 次复现，根因未定）。
- `fix_exterior_textures.py`：`images[i]` 重建为 Interior 同形 `{name:"<n>.dds", uri:"<n>.dds"}`，拷贝 `buffer.bin` + 274 张 DDS → `K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroExterior\`（276 件 / 1,101,085,696 B；`extracted\` 零写入，M131 digest 覆盖面未触）。
- 产物 glTF `sha256:d830984b7469ad4b1cf25386f4efa24e9d3d7ce1e987d189f46fcb6178f2a67c`（4,537,956 B）；`buffer.bin` `sha256:31d0fae4…43bd`（与源逐字节同）。
- 校验 9/9 PASS：三角 2,832,120 / 1591 primitive mode4 + indices / 7966 accessor 无稀疏 / 274/274 URI 命中 / baseColor fourCC DXT1 96 + DXT5 36（共享体 `dds_mean_linear_rgb` 闭集内 ⇒ `texture_mean_albedo: true` 成立）/ emissive 10 全 DXT1 / normal ATI2 132（仅登记，车道不消费）。
- 装载 smoke（`ext_load_smoke`）：`tris=2832120` 装配 + 6 帧 61 s（与室内同量级），tier50 render 6.3 ms/帧。

## 3. 场景事实与借壳契约

- `analyze_exterior_scene.py`（198 s）：根节点 `BistroExterior` 1.6 均匀缩放 ⇒ 世界 = 1.6 × 局部；地面 Y≈0.32；28 盏路灯（01B 壁挂 16 + 01a 杆灯 12）+ 5 吊灯笼 + 4 檐下面；**口径位（玻璃底面 −0.35 m）逃逸比全为 0**（灯具下部包住），点光改取侧偏 0.5–0.7 m 逃逸最优位（0.51–0.97）；店招 38/39 PCA 法线；作者相机 (14.61, 4.29, −41.81) forward (−0.318, −0.069, 0.945)；机位 C0/C1/C2/C3 净空全过。
- `derive_rain_night_contract.py`：克隆冻结契约，**只改 bistro-interior 行**（借壳：共享体 `parse_contract` L524/L740/L749 与 `g13_parity_contract.py` L31/L197 钉死场景闭集，`--scene bistro-exterior` 不可达；先例 `artifacts/day_0828/uhd/contract_4k.json`）：
  - camera：look-at → quat（`--selftest` 对室内 corpus ↔ 冻结契约 max|Δq| = 0，forward 复现 2e-16 PASS）；C2 = eye (2.0, 2.004, −24.0) fov 55° near 0.05 far 500；C1 = eye (22.0, 2.028, −52.0) fov 52°。
  - exposure ev100 **−7**；`sun/sky = 0`（车道不消费）。
  - point_lights：视锥内可见路灯 top-K（K=14）+ 吊灯笼 ×0.35：C2 = SL151/SL136/SL137/SL138 各 **0.5 cd** 暖白 (1.0, 0.72, 0.42)；C1 = SL136/SL151/SL125/SL152/SL147/SL140 0.5 cd + LANT23/24 0.175 cd。
  - emissive_materials 10 条：Le = display 目标 × 2^ev100 × DDS 均值色向（路灯 4.0 → 0.031 / 彩灯串 2.5 → 0.0195 / spotlight 2.0 / 店招 0.5 → 0.004；area_m2 实算）。
  - `gltf_product_digest` = 上表 glTF sha256；`m133_manifest_digest` 沿冻结值；`provenance.showcase_note / showcase_params` 如实登记借壳与全部派生参数。
  - `g10_corpus/` 三件（文件名按 `bistro_interior` 派生规则借壳，内容如实 BistroExterior）供 `--g10-dir`，evidence `g10_provenance` 登记其 sha。

## 4. bin 局部加性旗标（唯一代码面 `src/rurix-render/src/bin/g35_particle_lane.rs`，+254/−21）

| 旗标 | 语义 | 零漂移论证 |
|---|---|---|
| `--dump-present-every <n>` | 每 n 帧写 `{base}.f{fi:04}`（w/h u32 LE + BGRA8，g31 同布局；须随 `--dump-present-raw`） | `dump_hit ≡ false` ⇒ `rb.bgra` 表达式同值，`digest_seq` 门不动 |
| `--auto-move-amp <f>` ∈ (0,64] | orbit/dolly 位移倍率，**yaw 不缩** | `0.35 * amp * sin` 左结合 ×1.0 IEEE 精确 |
| `--auto-move dolly-forward` | 单向匀速 d = amp·t（无起伏无摆头） | 新闭集值，缺省不可达 |
| `--emitter-follow-camera on\|off` | `mirror.step` 前 `mirror.desc.pos = desc.pos + (eye(fi) − eye(0))`，host 金标准与 device `emit_params` 同源 | 缺省 off 分支不执行 |
| `--emit-max <n>` ∈ [256,4096] | `emit_schedule` 非缺省臂 ×n/256，dispatch `Direct([n,1,1])`；克隆守卫 `peak·ceil(life/dt) < 65536`（7919 与 2^16 互素 ⇒ 全槽克隆 ⇔ pid ≡ mod 65536）与 `peak·total < 2^24` | 缺省臂字面不变、守卫块不评估 |
| evidence | `showcase` 追加 5 键；顶层新增 `gltf{path,sha256}` | run 件 schema 不注册；缺省路径唯一可见变化 = 新增 `gltf` 键 |

回归（机器证明）：off 锚 `render_digest sha256:c1d28ad7…6c02` == Stage A 锚；on 缺省 orbit 48+6 双跑 + 基线 exe（`.tmp\g35_lane_baseline.exe`）三者 presented `92c870e9…89b1` / render `4857b6d4…` / digest_seq_sha `7cf143b4…`（54 项）全等；静态 30+6 帧 ± `--dump-present-every 1` presented `84a56190…2237` 相等、36 件各 8,294,408 B、末帧 == 基件；负例 10 条 rc=1 中文 FAIL。构建 `cargo build --release` warning 17→17（rurix-render 0）。SPV 三件现编与 9/2 现存件字节相同（encode `e7291c79…` / splat `2cf1ca80…` / resolve `a85775a8…`），spirv-val 绿。

## 5. 雨 / 光参数面（终版）

```
--particles on --rain-shutter 1.0 --rain-occlusion on --r-world 0.0015
--particle-tint 0.40,0.44,0.52 --particle-alpha-scale 0.45 --emit-max 640
--emitter-vel 0.4,-9.0,0.2 --emitter-vel-spread 0.3,0.5,0.3 --emitter-gravity -3 --emitter-life 1.659
C2 紧凑盒：--emitter-pos 6.492,10.304,-30.620 --emitter-spread 15.417,1.500,13.587
C1 紧凑盒：--emitter-pos 17.916,10.328,-45.121 --emitter-spread 14.757,1.500,12.659
推轨：--auto-move dolly-forward --auto-move-amp 8 --emitter-follow-camera on --frames 300 --warmup 100
```

- 密度：`--emit-max 640` ⇒ 峰值 637/帧 × ceil(1.659 s × 60) = 63,700 < 65,536（克隆守卫过）；稳态 n ≈ 29.6k（定帧）/ 29.7k（推轨）；紧凑盒（相机前 8 m、1–15 m 视锥，2,242 m³）密度 = 主盒 3×。
- 稳态前提：life 1.659 s ⇒ 雨柱落地 ≈ 1 s，**warmup ≥ 100 帧**（首探 16 帧只落 2.5 m 是判读教训）。
- 帧时（measured_local，非门）：定帧 C2 render 6.367 ms / particle_gpu 2.295 ms；C1 7.484 / 2.342 ms（≤ 90 fps 预算 11.11 ms）；推轨 auto-move 面 27.6 / 23.6 ms 含逐帧 8 MB BGRA8 回读（+3.09 GB 写盘），particle_gpu 4.2 / 3.8 ms。
- 探针序（6 跑）：0.0103 cd 点光完全照不亮（按 E=I/d²、ρ/π、×2^7 推算需 ~0.5 cd）→ 0.4 → **0.5 cd**；店招 Le 目标 2.0 白盘 → 1.0 → **0.5**；主盒 → 紧凑盒；alpha 0.5 → 0.45。

## 6. 确定性

- 定帧：C2 presented `7a5ec1bc48b49fb06dd5c3c2353fb05ed113fb56b06161f3e8f92367bfff0ced`；C1 `0985ebb84663188e63761c0131d7b6eade53dc4d4647ab4d32bed1120c400dc5`。
- 推轨双跑：presented `be90966dfea357ee59a97c0e898a4f4de58f7405edbffae333915e23ae01009b` / render_digest `313dcfdf…` / `digest_seq_sha 90dc0c30…`（400 项）位级相等；发射盒终位 (13.84, 10.33, −38.26) = 位移 7.98 m（= 8 × 399/400）。
- VUID：`RURIX_VK_VALIDATION=1` 全程开启，全部 rc=0 PASS（首跑缺该 env 被 fail-closed 拒 = 程序化修正，如实登记）。

## 7. 诚实边界

1. **借壳**：契约 `scene_id` 字面 `bistro-interior`，evidence `scene` 与 G10 语料文件名因此失真；补救 = 契约 `provenance.showcase_note`、本 REPORT、CAMPAIGN_LOG、evidence 顶层 `gltf{path,sha256}` 四处登记。真支持 `--scene bistro-exterior` 需放宽共享体 L524/L740/L749（+ Python 参照 L31/L197）并重锚 `FROZEN_CONSUMED_PATHS`，归 owner。
2. 车道 = `g14_3_direct_gi` 逐三角**均值反照率**（非贴图采样）；emissive **只可见不投光**，照明全靠契约点光（radius 0、阴影无半径截断 ⇒ 点光位为灯罩外侧逃逸最优位，非物理灯丝位）；店招/彩灯串为常量色（书店招牌呈平色盘）。
3. 132 材质全 OPAQUE（FBX2glTF 未保留 MASK）⇒ 植被 alpha-cutout 实心化（机位已避开树冠占中）；湿地面无镜面反射 / 无雨滴溅落 / 无水面涟漪；雨丝无风无阻力无地面碰撞（穿地后寿命到期消失，靠 TLAS 遮挡隐藏）。
4. 资产可复现性：`.tmp/` 源件为 gitignore 临时件，FBX2glTF 找到纹理即失败根因未定；派生资产在 K:（git 零二进制），`fix_exterior_textures.py --verify-only` 可复核。BistroExterior 正式登记（TODO #11「替代臂命中」信号、`g10_asset_license_registry.json` / `g10_corpus_scene_manifest.json` 只追加修订）归 owner。
5. 帧时口径：auto-move 面 `real_render_frame_ms` 含逐帧回读 + 写盘，不能与静态面直接比较；本役无帧时门。
6. 子agent事故：B 包分析脚本重跑失控（逃逸测试 6.8 万次 AABB 全扫）被主 agent 终止，唤醒后子agent配额耗尽，B 包由主 agent 接手完成（性能修正 + 300 s 时间预算自检）。

## 8. 复现命令（终版定帧 C2）

```
py -3 artifacts\day_0902_rain_night\g35_run.py --tag still_C2_final -- --particles on --frames 96 --warmup 100 ^
  --contract artifacts\day_0902_rain_night\contract_rain_night_C2_cd050.json ^
  --expect-digest sha256:37eea8257bc0faaf64bb8082d810fb3764bcb98f0ec3a0789252add6edb3e681 ^
  --gltf K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroExterior\BistroExterior.gltf ^
  --g10-dir artifacts\day_0902_rain_night\g10_corpus ^
  --rain-shutter 1.0 --r-world 0.0015 --particle-tint 0.40,0.44,0.52 --particle-alpha-scale 0.45 --emit-max 640 ^
  --emitter-pos 6.492,10.304,-30.620 --emitter-spread 15.417,1.500,13.587 --emitter-vel 0.4,-9.0,0.2 ^
  --emitter-vel-spread 0.3,0.5,0.3 --emitter-gravity -3 --emitter-life 1.659 ^
  --dump-present-raw artifacts\day_0902_rain_night\bistro_rain_night_C2.raw --evidence artifacts\day_0902_rain_night\bistro_rain_night_C2.json
py -3 artifacts\day_0829_realism\tools\raw2png.py artifacts\day_0902_rain_night\bistro_rain_night_C2.raw
```
