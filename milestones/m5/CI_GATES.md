# M5 CI 门禁增量

> 所属契约:[M5_CONTRACT.md](M5_CONTRACT.md)
> 版本:v1.0(2026-06-14)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md) + [../m3/CI_GATES.md](../m3/CI_GATES.md) + [../m4/CI_GATES.md](../m4/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–21 步、guardrail 含 M1.1/M1.2/M1.4/M3.3/M4.2/M4.3 激活项、nightly 工作流);本文只规定 M5 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)+ M4 §1(device 路径门禁:CUDA Toolkit 含 `ptxas` + Driver API 装载环境)。M5 新增 runner 预置项:

- **Compute Sanitizer**(`compute-sanitizer`,CUDA Toolkit 自带)纳入 GPU 队列 nightly(racecheck + memcheck);探测复用运行时探测器(`CUDA_PATH` 枚举,**禁硬编码版本文件名**——沿用 M4 r6 教训),预置落地时本表修订行留痕。
- **libdevice bc**(`$CUDA_PATH/nvvm/libdevice/libdevice.*.bc`)定位用同一探测器,禁硬编码版本文件名。
- CPU 任务(views 不相交检查、shared+barrier 一致性、scoped atomics 类型契约、views conformance、黄金路径 5)不占 GPU 队列;**GPU 任务**(gpu 并行基元真跑 + measured 基准 + Compute Sanitizer)占 GPU 队列,沿用 BENCH_PROTOCOL §2 锁频/环境画像/进程隔离纪律。

## 2. PR Smoke 追加步骤(编号接 M4 §2 的 17–21)

| # | 步骤 | 失败即红 |
|---|---|---|
| 22 | views 不相交证明 conformance 批跑:`conformance/views/reject/<category>/` 反例全拦截(重叠 split / 别名可变 view / view 越界 / shared view 与 barrier 不一致,逐文件断言产生预期 3xxx 诊断)+ `accept/` 正例 0 诊断 + 类别目录覆盖核对(契约 G-M5-2 通道;M5.1 落地接入)。**实测命令**:`cargo test -p rurixc --test views_corpus`(占位,落地时回填);计数核对 `py -3 ci/budget_eval.py`(`m5.counter.views_conformance_categories` ≥4) | 是 |
| 23 | 黄金路径 5 snapshot 核对:`tests/ui/{views,shared,atomics}/` 并行安全错误 .stderr snapshot(views 重叠/别名 3xxx + shared+barrier 一致性 + scoped atomics scope 误用)全绿 + bless 守卫(契约 G-M5-3 通道,复用 M1.4 UI 通道与 check_ui_bless)。**实测命令**:`cargo test -p rurixc --test ui_golden`;计数核对 `py -3 ci/budget_eval.py`(`m5.counter.ui_golden_path5_snapshots` ≥10,计数目录 = tests/ui/{views,shared,atomics}/) | 是 |
| 24 | (GPU)gpu 并行基元端到端真跑:自研 `reduce`/`scan`/`transpose`/`tiled GEMM` kernel(Rurix 源,全 safe 代码目标)经 rurixc 全管线(着色/views 不相交/shared+barrier 检查 → NVPTX codegen → libdevice 链接 → ptxas 关卡 → 嵌入)产 PTX → 装载 → launch → D2H → 与 host 参考实现核对 exit 0(契约 G-M5-1 真跑通道,对齐步骤 20 SAXPY 形态;M5.3 落地,GPU 队列)。**实测命令(M5.3 回填)**:`cargo test -p rurix-rt`(含 `rurix_reduce_e2e_isolated`/`rurix_scan_e2e_isolated`/`rurix_transpose_e2e_isolated`/`rurix_gemm_tile_e2e_isolated` 子进程隔离 + SAXPY 回归)+ `cargo run -p rurix-rt --bin reduce`/`--bin scan`/`--bin transpose`/`--bin gemm_tile`;kernel 源 `src/rurix-rt/kernels/{reduce,scan,transpose,gemm_tile}.rx`,build.rs 全管线嵌入 PTX。本机 RTX 4070 Ti 真跑:reduce/scan/transpose 精确核对、tiled GEMM rel err ≤2.6e-7,均 PASS。bench harness `bench/{reduce,scan,transpose,gemm_tile}_bench.py --smoke` 正确性 PASS(L1/L2 measured 回填随 M5.4)。**构建期无 clang/CUDA → 空哨兵降级 SKIP**;**无 GPU/驱动 → SKIP**(真红绿在带 clang+GPU 的 self-hosted runner) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m5_budget.json](m5_budget.json)(命名空间冲突即红;evaluator 已配 `m5.counter.views_conformance_categories`/`m5.counter.ui_golden_path5_snapshots`/`m5.counter.compute_sanitizer_clean` 分支,目录/证据缺失 → 0 → normal SKIP,对齐 M4 计数器先例)。**M5 期 PR Smoke 跑 normal 模式**:`m5.counter.*` 建设期未达标 SKIP 属预期;`m5.ratio.*_vs_cuda` estimated 占位在 M5.4 回填前继续 SKIP。**M5 close-out 必须跑 `--strict` 且全局零 estimated 残留**(契约 G-M5-1;本占位在 M5 内生灭,不跨里程碑欠债,14 §3)。

## 3. Nightly 追加

- 既有 nightly 全保留(lexer/parser/SAXPY(手写基线 + Rurix)冒烟 + budget normal + self-profile 归档,M2/M3/M4 实体化)。
- **Compute Sanitizer 纳入 nightly(契约 G-M5-4,M4 §3 标注"全量 racecheck 随 M5"的到期时点)**:`compute-sanitizer --tool racecheck`(数据竞争)+ `--tool memcheck`(越界/未初始化)对 M5 全部自研 kernel(reduce/scan/transpose/GEMM)+ M4 SAXPY 回归;GPU 队列、子进程隔离(14 §6);报告归档 `evidence/compute_sanitizer_<date>.json`(`clean=true` 计入 `m5.counter.compute_sanitizer_clean`)。**激活经真实红绿验证**(构造已知竞争 kernel → racecheck 红 → 修复转绿,run URL 归档,反 YAML-only)。Sanitizer 运行只作正确性维度,不用于 measured 基准(显著拖慢 kernel)。
- **gpu 并行基元 measured 基准纳入 nightly(M5.3/M5.4 落地,RD-002 承接)**:L1/L2 全量微基准 harness(复用 BENCH_PROTOCOL §3 协议;reduce/scan/transpose/GEMM-tile + 手写 CUDA C++ 对照实现作 denominator 锚点),三次进程级独立运行 + 回填(锁频 L0 前置,unlocked 整组作废拒绝回填);对手写 CUDA C++ 对照的回归判定(BENCH_PROTOCOL §5)。
- self-profile 归档自然覆盖 M5 新增阶段计数器(views 不相交/shared+barrier/libdevice 链接布点随实现扩列,非门禁,趋势参考)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless)+ M3 一项(MIR golden bless,check_mir_bless)+ M4 一项(PTX/IR golden bless,check_ptx_bless)+ M4 unsafe-audit(rurix-rt `undocumented_unsafe_blocks=deny`)。三项 M5 期动作:

1. **基准 ref 切换**:M5.1 第 1 项先打 `m4-closed` tag(随 M5 开工,对齐 `m3-closed` 随 M3 终审打出的先例),再将 `ci/check_guardrails.py` 本地/push 回退基准 `m3-closed → m4-closed`(PR 路径仍以 GITHUB_BASE_REF 为准),切换前双基准核对(`py -3 ci/check_guardrails.py m3-closed` PASS + `py -3 ci/check_guardrails.py m4-closed` PASS),落地留痕本表修订行。
2. **NVIDIA 再分发白名单审计 formal 激活**(14 §2 常驻集,M4 §8.2/CI_GATES §4 标注的到期时点):M5 引入 **libdevice 链接**(链 libdevice bc → internalize → DCE)——若产物嵌入 NVIDIA 再分发物(libdevice 派生码),formal 审计门(再分发清单逐项核对 NVIDIA EULA 白名单)激活;若 internalize+DCE 后仅保留派生于用户 kernel 调用的数学函数实现且符合再分发条款,逐项结论入本表修订行 + close-out。**libdevice 链接落地 PR 必须附白名单审计结论**(M5.3/M5.4)。

   **M5.3 审计结论(草拟,pending-human-review)**:(a) **不分发 libdevice bc**——`libdevice.10.bc` 不入 rurixc/rurix-rt 任何产物,链接期经 `CUDA_PATH/nvvm/libdevice/libdevice.10.bc` 从用户本地 CUDA 安装定位(`toolchain::locate_libdevice`,禁硬编码版本文件名,r6 教训);(b) **M5.3 四类交付 kernel(reduce/scan/transpose/gemm_tile)不调用任何 libdevice 数学函数**(`ir_needs_libdevice` = false,无 `__nv_*` 符号),其嵌入 PTX **不含 libdevice 派生码**——再分发面为空;(c) device 数学 intrinsic 链接能力(RXS-0081/0082)仅在 kernel 显式调用 `sqrt`/`exp`/`fma` 等时触发,届时经 internalize+DCE 仅保留用户调用可达的数学函数实现并内联进 PTX,产物为 PTX 文本(开发期 PTX-only,不打包 cubin/fatbin,→ G1);(d) PTX 文本中内联的 libdevice 派生实现是否构成 NVIDIA EULA Attachment A 意义下的"再分发"、及其白名单逐项核对,**需所有者/法务人工签署**(AI 仅起草事实清单,不代签)。结论:**M5.3 当前交付物无 libdevice 再分发面**;数学 kernel 真分发场景的逐项白名单核对随首个含 `__nv_*` 的分发产物(G1 cubin/fatbin)由人工 formal 审计。

   **M5.4 第 5 步 formal 激活(草拟 → formal,事实结论 pending-human-review 维持)**:M5.3 四点结论的机器可复核事实已落地证据 + 闸门留痕(反 YAML-only,H06 D11.8-2):
   - **机器复核闸门** `ci/check_redistribution.py`(check_* 守卫风格,不分配错误码,CPU-only):① 版本化嵌入 PTX `bench/kernels/rurix_*.ptx`(经 `check_bench_ptx_sync.py` 与 rurix-rt build 产物哈希同步)不含 `__nv_*` libdevice 派生符号;② rurixc/rurix-rt 源不把 `libdevice*.bc` 经 `include_bytes!/include_str!` 打包。挂 **pr-smoke**(`.github/workflows/pr-smoke.yml`「NVIDIA redistribution audit」步),GPU 队列串行不互扰(本身 CPU-only)。**真实红绿验证**:注入伪 `__nv_sqrtf` → 闸门红(exit 1)→ 复原 → 绿(exit 0),run URL 随本 PR CI 回填。
   - **审计证据** [evidence/redistribution_audit_20260614.json](../../evidence/redistribution_audit_20260614.json):机器事实(`ir_needs_libdevice=false`、`embedded_ptx_has_nv_symbols=false`、`libdevice_bc_packaged=false`、`redistribution_surface_empty=true`)逐条配复现命令背书;EULA 白名单裁决字段 `eula_whitelist_verdict="pending-human-review"`(AI 不代签)。schema `milestones/m5/redistribution_audit_evidence_schema.json`,经 `ci/check_schemas.py` 前缀路由校验;计入预算计数器 `m5.counter.redistribution_audit_clean`(`ci/budget_eval.py`,键于 `redistribution_surface_empty`,不键于法律签署)。
   - **结论(formal,事实部分激活)**:**M5.3/M5.4 当前交付物 NVIDIA libdevice 再分发面为空**,事实由上述闸门 + 证据机器复核背书。**EULA Attachment A 白名单逐项核对/再分发法律裁决保持 pending-human-review**,留所有者/法务于 M5 close-out(第 6 步)人工签署;数学 kernel 真分发(G1 cubin/fatbin 含 `__nv_*`)的逐项法律核对随首个分发产物 formal 签署(AI 仅起草事实清单)。

   **close-out(第 6 步)白名单审计结论段落草稿**:
   > NVIDIA libdevice 白名单审计 —— 事实层(formal 激活,机器复核背书):(a) `libdevice.10.bc` 不入任何产物,运行期经 `CUDA_PATH`/`RURIXC_LIBDEVICE` 定位(`toolchain::locate_libdevice`);(b) 四类交付 kernel `ir_needs_libdevice=false`,嵌入 PTX 无 `__nv_*` 派生符号,再分发面为空;(c) device 数学 intrinsic 链接仅在显式调用时触发,届时 internalize+DCE 内联进 PTX(开发期 PTX-only,不打包 cubin/fatbin);(d) 上述事实由 `ci/check_redistribution.py`(pr-smoke,红绿验证 run URL: https://github.com/qwasg/Rurix/actions/runs/27502668248/job/81288281409 —— #26 pr-smoke 第 7 步「NVIDIA redistribution audit」= success;该 run 整体 FAILURE 系第 18 步 MIR golden 被 cancelled,与再分发无关。本步本地复核红绿:红 exit 1 注入伪 `__nv_sqrtf` / 绿 exit 0 复原,详见 M5_CONTRACT §8.4)+ [evidence/redistribution_audit_20260614.json](../../evidence/redistribution_audit_20260614.json)机器复核背书。**法律层(pending-human-review,待所有者/法务签署)**:PTX 内联 libdevice 派生实现是否构成 NVIDIA EULA Attachment A 意义下"再分发",以及白名单逐项核对结论。签署人:________ 日期:________ 裁决:________。
3. **Compute Sanitizer racecheck+memcheck 纳入 nightly**(§3):M5 device 并行 kernel 落地时激活,激活经真实红绿验证(本表修订行留痕)。

14 §2 常驻集其余项的 M5 期评估结论:

| 项 | 结论 |
|---|---|
| MIR 文本 golden | M3.3 WP6 已激活(check_mir_bless),M5 沿用;views 不相交 pass 的 MIR 形态变更纳入既有 golden 核对 |
| PTX/IR 文本 golden | M4.2 已激活(check_ptx_bless);M5 gpu 基元 kernel + libdevice 链接后的 IR/PTX 形态纳入既有 golden 核对 |
| stable API 快照 | M5 无 stable 面,不激活 |
| unsafe-audit 完整性 | M4.3 已激活(rurix-rt);M5 新增 unsafe 边界(若 scoped atomics 映射/libdevice 链接引入)按 AGENTS 硬规则 9 注册条目,每 unsafe 块 `// SAFETY:`;全仓其余 crate 维持 deny |
| Compute Sanitizer | **M5 期激活**(§3 第 2 项 + 本节第 3 项动作),racecheck + memcheck nightly 全绿(契约 G-M5-4) |
| SG-002 复评(Tensor Core/WGMMA/TMA) | **M5 期复评留痕**:tiled GEMM 自研 kernel 走经典 shared-memory tiling,**不触 Tensor Core/WGMMA intrinsics**;触发条件("L2 基准证明 GEMM 类负载是真实用户瓶颈 且 中层抽象成熟度复评通过")未满足 → 维持 `not_triggered`,复评结论追加 [../../registry/spike_gating.json](../../registry/spike_gating.json) SG-002 decisions(M5.3 落地) |

m0~m4 历史预算的回填/冻结走 `check_guardrails.py` 既有机制("estimated 条目只允许回填为 measured_local";measured_local 条目 0-byte),不属新增激活项。

## 5. 验证程序(对应契约 G-M5-1/G-M5-2/G-M5-3/G-M5-4 与步骤 22–24)

1. 步骤 22 落地后,构造**故意放行某 views 不相交反例类别**(或篡改 reject 语料预期)的 PR → 必须红;修复后转绿,run URL 归档。
2. 步骤 23(黄金路径 5)走 bless 审批:篡改 .stderr 不附 bless 行 → `check_ui_bless` FAIL;补 bless 留痕 → 绿,run URL 归档。
3. 步骤 24(GPU 并行基元)落地后,构造拷回核对失败(篡改 kernel 语义)→ exit 1 红;复原 → exit 0 绿,run URL 归档。
4. Compute Sanitizer 激活时(§3/§4 第 3 项):构造已知数据竞争 kernel → `racecheck` 红;修复(加 barrier/收窄 view)→ 绿,两次 run URL 随 close-out 归档。
5. 基准 ref 切换(§4 第 1 项)落地后,切换前双基准核对输出附本表修订行。
6. close-out 附 `budget_eval --strict` 输出原文(契约 G-M5-1 三条比值 ≥0.90 与全局零 estimated 残留判定)+ measured_local 证据 JSON 路径 + NVIDIA libdevice 白名单审计结论 + Compute Sanitizer 红绿 run URL + SG-002 复评结论。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-14 | 初版(M5 契约配套;步骤 22–24 为 M5.1/M5.2/M5.3 计划项,落地时回填实测命令;guardrail 三项动作:基准切换 m3-closed→m4-closed(先打 m4-closed tag)、NVIDIA libdevice 白名单 formal 激活、Compute Sanitizer racecheck+memcheck nightly 均为计划项)。配套 `ci/budget_eval.py` 新增 `m5.counter.views_conformance_categories`(≥4)/ `m5.counter.ui_golden_path5_snapshots`(≥10)/ `m5.counter.compute_sanitizer_clean`(≥1)evaluator 分支(目录/证据缺失 → 0 → normal SKIP,对齐 M4 计数器先例);`py -3 ci/budget_eval.py` = PASS(23 pass, 6 skip, normal mode),M5 期 `m5.*` 占位/计数器 SKIP 属预期。`tests/test_budget_namespace.py` 20 passed |
| v1.1 | 2026-06-14 | §4 第 1 项落地(M5.1 任务 1):`m4-closed` tag 已随 M5 开工打出(annotated,锚定 M4 闭合合并点 `47a3be5`,M4_PLAN exit ticked #13;push origin 对齐 m0~m3-closed 先例)。`ci/check_guardrails.py` 本地/push 回退基准 `m3-closed → m4-closed`(`resolve_base()` 默认值 + 文件头 docstring + 行内注释;PR 路径仍以 `GITHUB_BASE_REF` 优先,既有逻辑不变)。切换前双基准核对均 PASS:`py -3 ci/check_guardrails.py m3-closed` = PASS(101 changed paths);`py -3 ci/check_guardrails.py m4-closed` = PASS(7 changed paths);默认(无参)= PASS(base=m4-closed, 7 changed paths) |
| v1.2 | 2026-06-14 | M5.3 落地回填:步骤 24 实测命令回填(`cargo test -p rurix-rt` 含四类 kernel `*_e2e_isolated` 子进程隔离测试 + `cargo run -p rurix-rt --bin {reduce,scan,transpose,gemm_tile}`;本机 RTX 4070 Ti 真跑全 PASS,GEMM rel err ≤2.6e-7;bench `--smoke` 正确性 PASS,measured 回填随 M5.4)。§4 第 2 项 NVIDIA libdevice 白名单审计 M5.3 结论草拟(pending-human-review):四类交付 kernel 无 libdevice 调用→无再分发面,libdevice.10.bc 不入产物(运行期从 CUDA_PATH 定位),数学 kernel 真分发白名单核对随 G1 cubin 人工 formal 审计。§4 表 SG-002 复评结论已追加 spike_gating.json decisions(tiled GEMM 经典 shared tiling,不触 Tensor Core,维持 not_triggered) |
| v1.3 | 2026-06-14 | M5.4 第 5 步落地:§4 第 2 项 NVIDIA 再分发白名单审计 **草拟 → formal 激活(事实层)**。新增机器复核闸门 `ci/check_redistribution.py`(check_* 守卫风格,CPU-only:嵌入 PTX 无 `__nv_*` + 源无 libdevice `.bc` 打包),挂 pr-smoke「NVIDIA redistribution audit」步,经真实红绿验证(注入伪 `__nv_sqrtf` → 红 exit 1 → 复原 → 绿 exit 0,run URL 随 PR CI)。审计证据 [evidence/redistribution_audit_20260614.json](../../evidence/redistribution_audit_20260614.json) + schema `redistribution_audit_evidence_schema.json`(`ci/check_schemas.py` 前缀路由)+ 预算计数器 `m5.counter.redistribution_audit_clean`(`ci/budget_eval.py`,键于 `redistribution_surface_empty`)。事实结论机器复核背书,**EULA 白名单法律裁决保持 pending-human-review**(close-out 人工签署,AI 不代签);close-out 白名单审计结论段落草稿已备。`py -3 ci/budget_eval.py --strict` = PASS(36 pass, 0 skip) |
| v1.4 | 2026-06-15 | M5.4 第 6 步落地(M5 close-out 终审):traceability 矩阵确定性重生成(`ci/trace_matrix.py`,RXS-0079 +3 / RXS-0080 +2 纳入 M5 新增 UI 用例,82/82 全锚定),新增 pr-smoke 步骤「traceability matrix freshness (M5 CI_GATES §4.6, G-M5-5)」= `py -3 ci/trace_matrix.py --check`(check_* 守卫,CPU-only,矩阵新鲜度从此受门禁)。§4 第 2 项 close-out 白名单审计草稿 `run URL: ____` 回填真实 URL(#26 run 27502668248 第 7 步 success;**evidence/redistribution_audit_20260614.json 受「只增不删不改」守卫不修改**,run URL 登记于本表 §4 + M5_CONTRACT §8.4)。M5 close-out 六条收口验收记录追加 M5_CONTRACT §8;**M5 关闭判定与 EULA 法律裁决保持 pending-human-review(人工签署位 §8.8,AI 不代签)**。`py -3 ci/budget_eval.py --strict` = PASS(36 pass, 0 skip);`py -3 ci/trace_matrix.py --check` = PASS;`py -3 ci/check_guardrails.py` / `check_schemas.py` = PASS |
