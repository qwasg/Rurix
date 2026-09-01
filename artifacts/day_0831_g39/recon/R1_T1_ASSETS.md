# R1 — T1 ReSTIR 证据链与资产侦察交接单(2026-08-31,行号为侦察时快照,实施以字面锚为准)

## 法定输入面

- `milestones/g30/g30_campaign_handover_registry.json` M100-high 行(campaign_period_rows):`"final": "closed-go(ReSTIR device 化 implemented:M-a 对拍 p100=2.831e-6 + y 整数锚 20000/20000 + 无偏 3σ + M-b 空间重用加性臂)", "g31_anchor": "M100 车道集成窗(锚第三件余项;RFC-0038 out-of-scope 锚)"`。
- RFC-0047 §5.5(`rfcs/0047-campaign-final-review.md` L65):战役外新特性承接锚 = G31+ 立项程序,g30 归档表为唯一法定输入面。
- TODO #7 行(`G31_PLUS_COMMERCIAL_RENDERER_TODO.md` L51):ReSTIR 高档 reservoir 车道集成;划界:#107(Clustered/Tiled)/#108(GPU light culling)/#87 独立不互充。

## 开窗证据(G38 T5 measured)

`artifacts/day_0830_g38/t5_risnee/lamp_k_ladder.json`(budget_ms=11.11,p50 源 profile render_wall):

| 档 | grid(m) | k | clusters kept | p50 ms | margin |
|---|---|---|---|---|---|
| s1 基线 | 0.6 | 12 | 12 | 10.798 | +0.312 |
| s3 | 0.3 | 24 | 16 | 10.702 | +0.408(贴线不稳健,同 digest s4 −0.895) |
| s5 | 0.15 | 48 | 26 | 12.959 | −1.849(确定超线) |
| s6 | 0.15 | 96 | 26 | 13.060 | −1.95 |

裁决(day_0830_g38/CAMPAIGN_LOG L27):默认不提档(维持 12/0.6);**ReSTIR 前置条件登记(measured):逐盏直接光 16 簇贴线、26 簇超线——提灯数须 ReSTIR**。

## 冻结面(0-byte)

- `src/rurix-render/kernels/g28_restir.rx`(127 行,单 invocation 验证件,非 per-pixel;WRS 链 L80-110,`u < w/w_sum` 除法比较形不可改写;时域/空间重用均在 host)。冻结由 `g31_restir_wiring.rs` 头注 L30-35 CI 机核。
- `src/rurix-render/src/gi/restir_reservoir.rs`(334 行,host 金标准:`Reservoir{y:usize,phat_y:f32,w_sum:f64,m:u32}` + update/merge(m_cap)/unbiased_weight/estimate_ris;五 #[test] 含 3σ 无偏/双跑位级)。
- `src/rurix-render/src/gi/multi_light.rs`(1257 行,M100 低档生产面;`check_restir_trigger`/`restir_serve` fail-closed 恒拒面 L784-796——本役不解除,低档 MegaLights 默认档不动)。
- 既有对拍门(收役回归证明「维持」):`ci/g31_restir_wiring_smoke.py --gate g31.waveB.restir`(读 g28 冻结容差带 `evidence/g28_restir_device_calibration.json` host_device_estimate_tol measured 2.831e-6 ×2.0);`ci/g28_restir_device_kernel_smoke.py`/`ci/g28_restir_spatial_reuse_arm_smoke.py`。
- 既有 harness 集成件:`src/rurix-render/src/bin/g31_restir_wiring.rs`(1381 行,G31 B2 交付;单着色点 20k trial,非渲染车道)。SEED `0xB261_0007_2026_0825`,N_TRIALS 20_000,m_cap 60;最新 evidence variance_reduction=15.844。

## EVAL_RESTIR 结论(`artifacts/day_0829_realism/evals/EVAL_RESTIR.md`,371 行)

- 开窗条件①已成立(G38 阶梯 measured)。推荐形态 = **方案 A**:scene 单 pass megakernel 内嵌 temporal RIS——M 候选闭式 R2/R3 随机 + prev 帧 VP 重投影读 prev reservoir + merge(m_cap 8–16 取小)+ 1 条验证射线;pass 数不变;新增 2 个 reservoir SSBO parity 轮换(DI 点灯集 16B/px ≈31.6MiB/份 @1080p)。方案 B(拆 3 段式)排除。
- 已知税:scene params 空槽紧张(G37 已扩至 [69]-[71],实施时实测容量;prev VP 16+ 槽建议走新小 SSBO,off 臂参数面保锚);新 SPV 工件字节隔离;新增 ray query 站点走 SPV 隔离;首版只接 tex_nrm(+gi2)合流形态;g28 随机带协议不可搬(>132MB/帧)必换闭式 R2/R3。
- 低配替身两件已于 G37 交付:`--gi2-ris`(M=6,`--gi2-ris-m`)/`--gi2-nee`(44k 灯片 CDF 面光 NEE)。

## G38 A/B 判读口径(T1 验收沿用)

- `artifacts/day_0830_g38/t5_risnee/run_ab.py`:EXPLICIT_NOAE_BASE = full19 展开字面 − `--auto-exposure` + `--quality off` 打头;env 恒注 `RURIX_G18_AMBIENT=0.004`/`RURIX_REQUIRE_REAL=1`/`RURIX_VK_VALIDATION=1`;`--frames 96 --warmup 2 --hidden --dump-present-raw <dir>/p.raw --dump-present-every 4 --evidence <dir>/ev.json`;每臂 r1/r2 双跑 digest 位级 + VUID=0。
- 方差:`artifacts/day_0829_realism/tools/ab_metrics.py` noise 子命令(presented u8,尾段 f0064..f0092 恰 8 张,BGRA→RGB /255 逐像素跨帧 std → mean/p95);四 ROI:wall(1400,150,480,270)/floor(1100,800,480,270)/dark_arch(360,0,360,180)/dark_table(560,560,560,200);verdict = dark 两 ROI min 的 p95 shrink_pct。
- 阶梯:`run_kladder.py` 形——s1 基线零画质参数(缺省 full19);提簇档 = env `RURIX_G31_LAMP_GRID_M=<0.3|0.15|…>` + `--quality full --lamp-k <K>`(--lamp-k 不进 full dup 表可组合);簇统计抓 stderr `lamp-lights 提取 emissive_tris=… clusters=… kept=… dropped=…`;帧时 = `--profile-json` frame_segments[render_wall] p50。
