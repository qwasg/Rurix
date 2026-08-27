<!-- Assisted-by: TraeCode:Kimi-K3（G31+ 波 C Task C9 NGX 分解 profiling 调查） -->
# G31+ 波 C Task C9 — NGX 分解 profiling 报告（G17-MD-F1 承接锚兑现）

> 承接锚 = `milestones/g30/g30_campaign_handover_registry.json` campaign_period_rows G30 期行（G17-MD-F1）：「NGX 分解 profiling **或** UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 重判条件同源）」。本报告 = 第一臂（NGX 分解 profiling）的 measured 兑现面；提前至波 B 同窗执行。证据件 = `evidence/g31_ngx_decomposition_20260826T094439Z.json`（schema `milestones/g31/g31_ngx_decomposition_evidence_schema.json`，门 `g31.waveC.ngx_decomp` PASS 7/7 facts）。

## 1. 焦点格与在案口径

- 格：`bistro-interior/t100/dlss_sr`（G17-MD-F1 性能焦点格）。
- 在案（G30.2 M-b 20260825T102813Z）：frame_ms_production_mean=**3.5767ms**，UE 暖态 ue_median=**3.4353ms**，ratio=**0.960479** < 1.00 → 17/18 诚实红终判。
- 本窗 canonical 复跑（同口径 160 帧 warmup 10，RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1 + GPU 独占锁）：frame_ms_production_mean=**3.504630625ms**，末帧 digest == G14.12 冻结锚 `sha256:55ea0c2b…578d4a` 位级 MATCH（既有测量口径零破坏的机器证明），fresh ratio=**0.980232** ≥ 在案 0.960479（不恶化；本机本窗快态）。

## 2. 测量方法（双 env 门控探针，默认关零行为变更）

插桩面 = `src/rurix-rt/src/vendor_upscale.rs` `upscale_resident_external`（dlss_sr 生产臂同一函数）：

1. **GPU 时间戳直测**（`RURIX_G31_NGX_TS=1`）：3 槽 timestamp query pool（session 惰性创建，Drop 销毁），`vkCmdWriteTimestamp` 三槽围 evaluate——ts0 cmd 首（TOP_OF_PIPE）/ ts1 evaluate 前（BOTTOM_OF_PIPE）/ ts2 evaluate 后（BOTTOM_OF_PIPE）；`vkQueueWaitIdle` 后 `vkGetQueryPoolResults` 64-bit 读回 × timestampPeriod（物理设备 props blob@720 实采，render_exec 同源偏移）。得：pre_eval GPU（acquire barrier ×3 段）/ **NGX in-stream 纯 GPU** / cmd GPU；配 evaluate CPU 返回点墙钟锚 → **提交-同步税** = submit_wait 墙钟 − cmd GPU。
2. **X2 边际互核**（`RURIX_G31_DLSS_EVAL_X2=1`，G17.3 同型撤除面复置）：同 cmd 第二次 `slEvaluateFeature`，submit_wait 边际 = NGX 单次 in-stream 净成本（独立第二方法）。
3. **scene 渲染段**：DeviceFrameTelemetry 逐 pass GPU timestamp（pass0 scene + pass1 mv；pack pass GPU 经 stderr 均值单列）。
4. **其余宿主段**：receipt 逐帧列对齐残差 = frame_ms_production − scene − mv − upscale 墙钟（内含 pack GPU + 帧参数上传 + 三 pass submit/fence + jitter）；upscale 内 SL 簿记/录制/evaluate CPU 经既有 `RURIX_VENDOR_TIMING=1` dlss-ext 行单列。

三轮（单 GPU 锁串行同窗）：A canonical（在案口径复跑核对）/ B timing+TS（四段直测）/ C timing+X2（边际互核），各 160 帧 warmup 10，逐帧 post-warmup n=160。

## 3. 四段分解数字（measured，轮 B 直测）

| 段 | 值（median / mean，ms） | 方法 |
|---|---|---|
| ① NGX in-stream（evaluate 纯 GPU） | **1.8365 / 1.9136** | GPU 时间戳 ts2−ts1 × timestampPeriod |
| ② NGX 提交-同步税 | **0.153 / 0.156** | submit_wait 墙钟 − cmd GPU；pre_eval GPU 子项 = **0.000**（acquire barrier ×3 实测为零） |
| ③ scene 渲染段 | **0.9817**（mean；scene 0.9497 + mv 0.0320） | 逐 pass GPU timestamp；pack GPU 0.1540 单列 |
| ④ 其余宿主段 | **0.3585 / 0.3646** | 逐帧残差；upscale 内 sl_book 0.0059 + record 0.0084 + evaluate CPU 0.0722 单列 |

互核与一致性：

- **X2 边际 = 2.019ms**（x1 submit_wait 2.039 / x2 4.058）vs TS 直测 1.8365ms → |Δ|=0.1825ms ≤ max(0.15, 15%)=0.2755 → **两独立方法互核通过**。方向 = 边际 > 直测（第二次 evaluate 略贵，G17.3 M-b 同方向在案：慢态边际 2.224）；幅差 9.9% 在容差内，两法同证 NGX in-stream ≈1.8~2.0ms 量级（G15 快态基线 1.90 同族）。
- cmd GPU 1.8365 ≤ submit_wait 墙钟 1.992 + 0.05；TS 墙钟 vs dlss-ext 墙钟 |Δ|=0.047 ≤ 0.15 → 墙钟两独立面一致。
- 段和闭合：scene 0.9497 + mv 0.0320 + upscale 墙钟（0.0059+0.0084+0.0722+2.039≈2.126）+ 残差 0.365 ≈ 3.47ms ≈ 轮 B 自身 frame 口径（轮间机态漂移 ≤  canonical 3.5046 口径内）。

## 4. UE 侧差对照（宿主差可分离定位）

UE 暖态 3.4353ms 在案（g14_m-d dual_end 最新 evidence ue_median_ms）。本窗 fresh Δ = 3.5046 − 3.4353 = **+0.0693ms**。

- **NGX in-stream 段（1.8365ms）= 双边同一硬件同一 NGX 310.5.2 网络同一 cubin 族执行**（G15plus-II 在案：Rurix NGXCubinVulkan vs UE 臂 NGXCubinD3D12 同族 cubin 同 Preset 同模型库）——物理不可分离且等量，**不构成差源**。
- Rurix 侧可分离宿主段包络 = 提交-同步税 0.153 + SL 簿记/录制/evaluate CPU 0.086 + 车道宿主残差 0.365 ≈ **0.604ms ≥ |Δ|=0.0693ms**——**差完全落在宿主可分离段包络内**（Rurix 逐帧孤立 submit+waitIdle 提交边界 vs UE 帧内 in-stream evaluate 集成形态的宿主构成差）。
- **主源定位 = host_residual_separable**（宿主残差族 0.365+0.086 ≈ 0.451ms，为包络内最大构成；提交-同步税 0.153ms 次之）。

## 5. 重判评估

- fresh ratio = **0.980232 < 1.00** → ratio ≥ 1.00 重判条件**未命中**。
- 结论 = **rejudge_not_triggered_honest_red_maintained**：维持 G30 终判 17/18 诚实红；**分解证据落档 = 承接锚兑现形态**（宿主差可分离 measured 证据——差完全落在宿主可分离段包络内、NGX in-stream 不可分离等量段非差源）。
- 在案行 0-byte 不回写（g30_campaign_handover_registry / G30_P2_DECISIONS 均 0-byte）；新证（ratio ≥ 1.00）出现时只追加重判，程序不变。

## 6. 口径守护与冻结面

- canonical 复跑末帧 digest == G14.12 冻结锚（位级 MATCH）——dlss 臂既有测量口径零破坏；在案 3.5767ms 行 0-byte（本窗 3.5046ms 为同窗新鲜对照非改写）。
- 探针双 env 默认关：生产默认面（env 未置）行为 0-byte；X2 探针轮 digest 漂移 = 注入预期（第二 evaluate 改变历史态），不入锚判定（G17.3 同律）。
- unsafe-audit `unsafe-audit/rurix-rt.md` U58 扩注（同一 vendor FFI 边界，0 新 U 号）。
- check_schemas.py 三处纯追加（load / validator / 前缀路由 `g31_ngx_decomposition_`，与既有 g31_* 全族及 gpu fallthrough 互不包含）+ 全量 PASS；check_number_ledger PASS（CI_step next_free=525 零消费维持，symbolic gate 未占号）。
- CI 冒烟 = `ci/g31_ngx_decomposition_smoke.py`（--selftest 47 项红绿全过；--gate g31.waveC.ngx_decomp PASS 7/7 facts；DEV_ENV_DEGRADE 三态——无 NGX/GPU/资产 SKIP 退 0 不冒充，REQUIRE_REAL=1 下降级翻硬 FAIL）。

## 7. 遗留风险

1. **X2 边际系统性高于 TS 直测 ~10%**（2.019 vs 1.8365）：第二次 evaluate 的边际含历史态/排队效应，两法差在 15% 容差内但若未来用边际法单独定地板需注意方向（G17.3 M-b 同方向在案）。
2. **机态漂移**：本窗快态（3.5046）vs G30 窗（3.5767）vs G17.3 慢态（4.05）——绝对值随窗漂移，ratio 判据锚 UE 同窗在案值（3.4353）未新鲜复测 UE 臂（承接锚第一臂只要求 NGX 侧分解；UE 侧插桩臂若后续立项可闭合此项）。
3. **宿主残差构成**：残差 0.365ms 内含 pack GPU 0.154 + 帧参数上传 + 三 pass submit/fence + jitter 等混合面，未再细分（对本结论非关键——包络已覆盖 Δ）。
4. **pre_eval GPU = 0.000 的口径**：acquire barrier ×3 实测低于 timestamp 分辨率（~0.5μs 级以下），如实登记为零非缺失。
