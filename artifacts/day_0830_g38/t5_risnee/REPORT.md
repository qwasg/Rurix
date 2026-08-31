# G38 T5:RIS/NEE 方差收缩 A/B + lamp-k 阶梯(脚本面设计登记)

> 2026-08-30。T5 实施 agent 产出,纯 python 零 .rs 编辑零 GPU 真跑(--selftest 全伪数据);
> GPU 真跑归主 agent 批次 2(GPU 锁外部持有,脚本不管锁)。
> 既有工具(ab_metrics.py 等)只调用不修改。

## 1. 文件清单

| 文件 | 职责 |
| --- | --- |
| `run_ab.py` | RIS/NEE 四臂 A/B 跑批(base/+ris/+nee/+both,逐臂双跑 digest 自证,r1 带 raw 周期 dump) |
| `judge_ab.py` | A/B 判读 → `ris_nee_ab.json`(四 ROI 时域噪声 + 收缩百分比 + variance_verdict) |
| `run_kladder.py` | lamp-k 六档阶梯跑批(env 旋钮 + --lamp-k 散臂,单跑,抓 stderr 灯簇行) |
| `judge_kladder2.py` | 阶梯判读 → `lamp_k_ladder.json`(逐档预算 margin + verdict/go_candidate) |
| 产物目录 | `ab/<臂>/r1|r2/`(p.raw.fXXXX + ev.json)、`kladder/<档>/`(ev/prof/stderr) |

## 2. EXPLICIT 无 AE 集推导(十九臂时代字面)

day_0829 红修 #1 纪律:A/B 一律无 AE 显式组合。十臂时代字面(run_arm.py L34-40 EXPLICIT_NOAE)
已过时;十九臂时代显式集从窗口 bin **full 展开赋值区字面直接抄取**:

- 源 = `src/rurix-render/src/bin/g31_window_present.rs` L7850-7874 赋值区
  (展开表 QUALITY_FULL_EXPANSION 22 项在 L7815-7839,dup 校验用;赋值区才是行为字面)。
- 赋值区字面 → CLI 翻译(逐行,含子参数字面):
  `smooth_nrm=true; ggx=true; lamp_lights=true; lamp_gain=Some(4.0); textures=true;`
  `bloom=true; dither=true; autoexp=true; tsr_quality=true; gi2=true; gi2_clamp=Some(0.01);`
  `emissive_tex=true; metal_f0=true; rt_ao=true; soft_shadows=true; soft_shadow_samples=Some(1);`
  `rt_reflect=true; gi2_tex=true; normal_maps=true; transparency=true; gi2_ris=true; gi2_nee=true`
  (+ `G18_AMBIENT_PRESET.set("0.004")` = 环境光预设字面,env 等价注入)。
- G37 新臂核查:**transparency 在赋值区(在集);lut/visbuffer/pso 不在赋值区(不在集)**
  ——以赋值区字面为准,展开表与赋值区一致(22 项 = 19 布尔臂 + 3 子参数字面)。
- 推导:赋值区全集 **− `--auto-exposure`(无 AE 纪律恒减)− `--gi2-ris`/`--gi2-nee`(被测臂)
  + `--quality off` 打头**(G37 W4 默认翻转后缺省=full,显式组合不给 off 即与预设 dup fail-closed)。

**最终 base 臂字面**(run_ab.py `EXPLICIT_NOAE_BASE`,dry-run 已验证):

```
--quality off --smooth-normals on --ggx on --lamp-lights on --lamp-gain 4
--textures on --bloom on --dither on --tsr-quality on --gi2 on --gi2-clamp 0.01
--emissive-tex on --metal-f0 on --rt-ao on --soft-shadows on --soft-shadow-samples 1
--rt-reflect on --gi2-tex on --normal-maps on --transparency on
```

臂增量:ris = `--gi2-ris on`;nee = `--gi2-nee on`;both = 两者(须随 --gi2/--smooth-normals/
--textures on 的 fail-closed 前提,base 集内全有;--gi2-ris-m 走默认 6 不显式)。
env 恒注:`RURIX_G18_AMBIENT=0.004`(预设 OnceLock 同字面同 parse 位级同值)
+ `RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`(run_arm.py env_of L71-78 同律)。

## 3. 口径声明(判读链)

- **raw 口径 = display-encode 后 u8**:`--dump-present-raw`(8B 头 w/h u32 LE + BGRA8)
  + `--dump-present-every 4`(每 4 帧写 `p.raw.f<帧号:04>`,96f ⇒ f0000..f0092 + 末帧本体)。
  同路 on/off 对照有效;**与 bench EXR f32 不可跨比**(如实登记,不混算)。
- **帧源 = 尾段 f0064..f0092 恰 8 张**(warmup2 + TSR 帧旋转收敛后,序列尾段时域噪声
  即 RIS/NEE 方差口径;judge_ab `pick_tail_frames` 不足 8 张 fail-closed)。
- **度量 = day_0829 既有工具复用链**:`artifacts/day_0829_realism/tools/ab_metrics.py`
  noise 子命令(load_raw 跳 8B 头 + BGRA→RGB /255,逐像素跨帧 std → mean/p95),
  经 `build_parser()` 走 CLI 同语义 import 调用,零修改。
- **四 ROI 字面**(1920×1080,交接单钉死):wall=(1400,150,480,270)
  floor=(1100,800,480,270) dark_arch=(360,0,360,180) dark_table=(560,560,560,200);
  dark_* 为 GI2 微光承载区 = RIS/NEE 主战场。
- **variance_verdict 阈值口径**(写进 json 注记):shrink_pct =
  (base_p95 − arm_p95)/base_p95×100,取 dark 两 ROI **min(保守)**:
  ≥10% = effective;[0,10%) = marginal;<0 = worse。
- **阶梯帧时口径**:主源 = `--profile-json`(C7 面,默认关零渲染语义变更)
  frame_segments[render_wall] 的 p50_ms/max_ms;缺失回退 evidence
  real_render_frame_ms(mean 代 p50)+ stats.render_max_ms,`p50_source` 如实登记。
  预算 11.11ms(90fps),margin = 11.11 − p50。

## 4. env 旋钮接口约定(lamp-k 阶梯)

- 事实:`--lamp-k` 已在(默认 12,L8003 `unwrap_or(12)`,须随 --lamp-lights on,
  不进 full dup 表可与 `--quality full` 组合);聚类网格 `GRID_M=0.6` 为 lane_body
  `extract_lamp_lights` 内 const(L2279),bistro 44,024 emissive 三角在 0.6m 网格下
  只产 13 簇(kept 12/dropped 1)⇒ 现网格 --lamp-k 24/48 无效。
- **接口约定(主 agent 稍后接线,脚本按此写)**:环境变量 `RURIX_G31_LAMP_GRID_M`
  ——缺席 = 0.6 字面;在位 parse f32,非法即 fail(RURIX_G18_AMBIENT 同律先例)。
- 档位(散臂 = env + 显式 --lamp-k,**不动任何在案锚**):
  s1 基线(env 缺席/不传 k,零画质参数 = 缺省 full19,s02 锚位 7636f72f)/
  s2 证伪(0.6, k24:预期簇仍 13、kept 不变)/ s3(0.3, 24)/ s4(0.3, 48)/
  s5(0.15, 48)/ s6(0.15, 96)。每档单跑(阶梯是量测不是锚;双跑自证留给
  定档后的 GO 档位)。阶梯 env 不注 G18_AMBIENT(full 预设自供 0.004,
  显式 env 面不进基线锚位口径)。
- 灯簇统计抓取:stderr 行字面
  `[g14_3_pipeline_perf]: lamp-lights 提取 emissive_tris=… clusters=… kept=… dropped=…`
  (lane_body apply_lamp_lights eprintln,L2474-2480;正则只锚前四个 key=value 段,
  对尾段全角括号字面稳健;缺行如实登记 None 不冒充)。
- judge_kladder2 verdict:提档 = kept>13;存在 margin≥0 的提档 ⇒ `go_candidate`
  (lamp_k_go_candidate = 预算内最高 kept 档,Wave3 决策口);kept>13 的一切档均
  超预算 ⇒ `restir_precondition_confirmed`(逐盏 K 提档预算内不存在,ReSTIR 大件
  开窗条件成立 measured——EVAL_RESTIR §2 斜率预判 ≈0.16ms/盏、K=24 ≈+1.9ms 贴线、
  K≥48 超线的曲线由此钉成 measured)。附旋钮生效性检测:收细档 clusters_total
  仍全 =13 ⇒ `grid_knob_suspect_not_wired` 注记(接线前误跑防线,量测判无效留人工)。

## 5. 锚零影响声明

- 本任务全部为**散臂组合 + 验证面 flag**(dump/evidence/profile-json 均 host 写盘,
  不入渲染语义不入 digest):A/B 四臂 = 显式无 AE 组合(非锚位);阶梯 s2-s6 =
  env + --lamp-k 散臂(非锚位);唯一触及锚位的 s1 基线 = 零参数缺省,**正是
  full19 s02 锚(7636f72f)本位,只读复证不改写**。
- 零 .rs 编辑;改 --lamp-k 默认 12 / GRID_M 0.6 字面(动 full19 锚)是 Wave3
  判 GO 后的事,不在本任务面。

## 6. 批次 2 执行顺序清单(主 agent)

前提:target-night\release\g31_window_present.exe 在位;GPU 锁批次外部持有。

| # | 命令 | 条数 | 估时 |
| --- | --- | --- | --- |
| 1 | `py -3 artifacts\day_0830_g38\t5_risnee\run_ab.py` | 8 跑(4 臂×2,96f/warmup2 each) | ≈15-25 min(bistro 装配主导,单跑 ≈2-3 min) |
| 2 | `py -3 artifacts\day_0830_g38\t5_risnee\judge_ab.py` | 零 GPU | <1 min |
| 3 | (待 RURIX_G31_LAMP_GRID_M 旋钮接线)`py -3 artifacts\day_0830_g38\t5_risnee\run_kladder.py` | 6 档单跑(96f each) | ≈12-20 min(K=96 档帧时若 ~20ms 渲染段仍 <2s,装配主导) |
| 4 | `py -3 artifacts\day_0830_g38\t5_risnee\judge_kladder2.py` | 零 GPU | <1 min |

合计 GPU 面 14 跑,预计 ≈30-45 min。备注:阶梯 s1/s2 不依赖旋钮可提前跑;
s3-s6 在旋钮接线前跑会落 `grid_knob_suspect_not_wired` 注记(judge 判无效,须重跑)。
任一跑 fail(rc≠0 / VUID>0 / 缺 digest / 双跑不一致)即停,fail-closed。
参考帧时:base 腿 c80cb6ae 4.71ms / ris+nee 单开锚 s06 851a61ba 6.78ms /
full19 s02 7636f72f 9.75/10.59ms(预算 11.11ms)。

## 7. 自证结果(本机,零 GPU)

- `py -3 -m py_compile run_ab.py judge_ab.py run_kladder.py judge_kladder2.py` → **全 4 PASS**。
- `run_ab.py --selftest` → PASS(伪 raw 8B 头+随机 BGRA 经 ab_metrics noise 全链:
  噪声序列 p95=0.0297 >0,恒定序列 std=5.6e-17 ≈0〔np.std 浮点尾数,容差 1e-12,
  远低于 u8 量化级 3.9e-3〕;8 条命令构造断言:无 AE 字面不在集/臂旗标正确)。
- `judge_ab.py --selftest` → PASS(四臂伪 dump 全链:ris=effective(+50.0%)/
  nee=marginal(+4.65%)/ both=worse(−50.56%) 三档方向全中;尾段选帧 8/8)。
- `run_kladder.py --selftest` → PASS(真 eprintln 字面〔含全角括号〕正则抓取
  44024/13/12/1;缺行 None;基线零画质参数/档位 --quality full --lamp-k 构造;
  env 三态〔缺席/0.15/恒注面〕断言)。
- `judge_kladder2.py --selftest` → PASS(三情景:预算内提档 ⇒ go_candidate@s3;
  提档全超线 ⇒ restir_precondition_confirmed + go_candidate 退化基线;
  旋钮未接线 ⇒ suspect 旗标 + 注记)。
- `run_ab.py --dry-run` / `run_kladder.py --dry-run` → 8+6 条命令字面人工核对通过
  (EXPLICIT 集序 = 赋值区序;dump/evidence 路径逐臂隔离;r1/r2 除路径外字面全同)。
