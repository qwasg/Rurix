# M4 CI 门禁增量

> 所属契约:[M4_CONTRACT.md](M4_CONTRACT.md)
> 版本:v1.0(2026-06-13)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) + [../m1/CI_GATES.md](../m1/CI_GATES.md) + [../m2/CI_GATES.md](../m2/CI_GATES.md) + [../m3/CI_GATES.md](../m3/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–16 步、guardrail 含 M1.1/M1.2/M1.4/M3.3 激活项、nightly 工作流);本文只规定 M4 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only,H06 D11.8-2)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机)。M4 首次出现 **device 路径门禁**:

- CPU 任务(着色/地址空间检查、NVPTX codegen、ptxas 干验证、launch conformance、黄金路径 4)不占 GPU 队列。
- **GPU 任务**(SAXPY 端到端真跑 + measured 基准)占 GPU 队列,沿用 M0 基准 harness 的锁频/环境画像/进程隔离纪律(BENCH_PROTOCOL §2)。
- 新增 runner 预置项:CUDA Toolkit(含 `ptxas`)+ Driver API 装载环境。探测复用运行时探测器(NVML / `CUDA_PATH` 枚举,**禁硬编码版本文件名**——r6 的 `CUDA 13.2.props` 教训,07 §10);预置项落地时在本表修订行留痕。

## 2. PR Smoke 追加步骤(编号接 M3 §2 的 15–16)

| # | 步骤 | 失败即红 |
|---|---|---|
| 17 | ptxas 干验证关卡:示例 kernel 经 rurixc device codegen 产 PTX → `ptxas -arch=sm_89` 干验证通过;构造拒绝场景 → RX6xxx 编译期诊断(契约 G-M4-4 通道;M4.2 落地接入)。**实测命令**:`rurixc <kernel>.rx --emit=ptx -o <out>.ptx`(IR→PTX 经 pin 的 clang `--target=nvptx64-nvidia-cuda -Xclang -target-cpu sm_89 -Xclang -target-feature +ptx78 -S`;产 PTX 后调 `ptxas -arch=sm_89` 干验证,拒绝 → RX6004,工具链定位失败 → RX7001);**ptxas 缺失(无 CUDA 工具链)→ 关卡 SKIP**(开发环境降级,真实红绿在带 CUDA 的 self-hosted runner;§1 预置 ptxas 后实体化)。本地 `--emit=nvptx-ir` 产 NVPTX IR(无外部工具)由 `ptx_golden` 字节 golden 守(§4.3)。**关卡红绿永久测试**:`cargo test -p rurixc --test ptxas_gate`(`rurixc::ptxas::dry_gate` 合法 PTX → `Pass` / 注入非法 PTX → `Rejected`=RX6004 通道;无 ptxas 降级 SKIP) | 是 |
| 18 | launch 类型契约 conformance 批跑:`conformance/launch/reject/<category>/` 反例全拦截(逐文件断言产生预期 3xxx/RX2001 诊断)+ `accept/` 正例 0 诊断 + 四类目录覆盖核对(契约 G-M4-2 通道)。**实测命令**:`cargo test -p rurixc --test launch_corpus`(accept 0 诊断 / reject 全拦截 / 四类目录核对 / spec 锚定);计数核对 `py -3 ci/budget_eval.py`(`m4.counter.launch_conformance_categories` ≥4) | 是 |
| 19 | 黄金路径 4 snapshot 核对:`tests/ui/` 目标后端错误 .stderr snapshot(3xxx 着色/地址空间 + 6xxx codegen/ptxas + launch 类型契约)全绿 + bless 守卫(契约 G-M4-3 通道,复用 M1.4 UI 通道与 check_ui_bless)。**实测命令**:`cargo test -p rurixc --test ui_golden`;计数核对 `py -3 ci/budget_eval.py`(`m4.counter.ui_golden_path4_snapshots` ≥10,计数目录 = tests/ui/{coloring,addrspace,codegen,launch}/) | 是 |
| 20 | (GPU)Rurix SAXPY 端到端真跑:**Rurix 源** `src/rurix-rt/kernels/saxpy.rx` 经 rurixc 全管线(着色检查 → NVPTX codegen → ptxas 关卡 → clang IR→PTX -O2)产 PTX,由 `rurix-rt` build.rs 嵌入 EXE data 段(06 §5.2)→ 装载 → H2D → `cuLaunchKernel` → D2H → 逐元素 f32 精确核对 exit 0(契约 G-M4-1 真跑通道,对齐步骤 12/16/21 真跑形态;M4.4 落地,GPU 队列)。**实测命令**:`cargo test -p rurix-rt`(`rurix_saxpy_e2e_isolated` 子进程隔离真跑)+ `cargo run -p rurix-rt --bin saxpy`(单 EXE 真跑 exit 0)。**构建期无 clang/rurixc → 嵌入空哨兵 PTX,bin/test 运行时 SKIP**;**无 GPU/驱动 → SKIP**(降级,真红绿在带 clang+GPU 的 self-hosted runner;本机 RTX 4070 Ti + clang 22.1.7 + CUDA 13.3 ptxas 真跑通过) | 是 |
| 21 | (GPU)rurix-rt 运行时全链路真跑:`cuModuleLoadDataEx` 装载协商 → `cuMemAlloc`/H2D → `cuLaunchKernel` → D2H → 逐元素 f32 精确核对(契约 D-M4-4 出口判据;08 §2,子进程隔离 14 §6,GPU 队列)。**实测命令**:`cargo test -p rurix-rt`(`saxpy_roundtrip_isolated` / `context_smoke_isolated` 子进程隔离真跑;**无 GPU/驱动 → 降级 SKIP**,真红绿在带 GPU 的 self-hosted runner;本机 RTX 4070 Ti 经 driver JIT 真跑,无需 ptxas/Toolkit) | 是 |

预算 evaluator(M0 步骤 6)自动合并加载 [m4_budget.json](m4_budget.json)(命名空间冲突即红)。**M4 期 PR Smoke 跑 normal 模式**:`m4.counter.*` 建设期未达标 SKIP 属预期;`m4.ratio.saxpy_vs_m0_baseline` estimated 占位在 M4.4 回填前继续 SKIP。**M4 close-out 必须跑 `--strict` 且全局零 estimated 残留**(契约 G-M4-1;本占位在 M4 内生灭,不跨里程碑欠债,14 §3)。

## 3. Nightly 追加

- 既有 nightly 全保留(lexer/parser/SAXPY(手写基线)冒烟 + budget normal + self-profile 归档,M2/M3 实体化)。
- **Rurix SAXPY measured 基准纳入 nightly**(M4.4 落地):harness `bench/rurix_saxpy_bench.py`(冒烟 `--smoke` / 完整协议 `--emit`,3N 带宽口径与 M0 同),三次进程级独立运行 + 回填经 `bench/rurix_saxpy_triple.py`(锁频 L0 前置,unlocked 整组作废拒绝回填);全量 bench 含 Rurix kernel SAXPY 采样 + 对 M0 手写基线的回归判定(BENCH_PROTOCOL §5,Mann-Whitney U + 效应量门,1% Warning / 5% Critical)。
- **Compute Sanitizer 评估**:device 运行时出现后,memcheck nightly 评估接入(全量 racecheck 随 M5 scoped atomics/barrier,r5/08 §5);M4 期结论入修订行。
- self-profile 归档自然覆盖 M4 新增阶段计数器(着色/codegen/ptxas 布点随实现扩列,非门禁,趋势参考)。
- Release 层仍不建(RD-001,承接 M8 不变)。

## 4. Guardrail

沿用 M0 五项 + M1 三项(spec 档位 / 错误码冻结 / UI bless)+ M3 一项(MIR golden bless,check_mir_bless)。三项 M4 期动作:

1. **基准 ref 切换**:`m3-closed` tag 已随 M3 终审打出(2026-06-13);M4.1 第 1 项将 `ci/check_guardrails.py` 本地/push 回退基准 `m2-closed → m3-closed`(PR 路径仍以 GITHUB_BASE_REF 为准),切换前双基准核对,落地留痕本表修订行。
2. **NVIDIA 再分发白名单审计激活评估**(14 §2 常驻集,M0~M3 CI_GATES 标注"device 路径 M4 起评估"的到期时点):M4 产物为 **PTX-only**(开发期),运行时经已安装驱动 JIT 装载(`cuModuleLoadDataEx`),**不打包任何 NVIDIA 再分发二进制**(libdevice 链接随 M5、cubin/fatbin 分发随 G1)。故 M4 期审计结论 = 无再分发物需白名单核对,formal 审计门(再分发清单逐项核对)随 libdevice/cubin 引入时激活(M5/G1)。评估结论于 M4 期实体化(本表修订行 + close-out)。
3. **PTX 文本 golden / NVPTX 雷区回归集**(14 §2 IR golden 机制,07 §11):M4.2 **激活**——形态裁决为 **cargo test 字节 golden**(`src/rurixc/tests/ptx_golden.rs`,语料 `tests/ptx/**/*.rx`,golden = 同名 `.nvptx`),基线取 **device codegen 产出的 NVPTX 约束 LLVM IR 文本**(rurixc 自有产物,确定性、无外部工具依赖;PTX 为下游 clang/NVPTX 后端汇编产物,字节稳定性绑定工具链版本,故 golden 取 IR 层,clang IR→PTX→ptxas 真跑由步骤 17 覆盖)。bless 纪律对齐 UI/MIR(`RURIX_BLESS=1` 重写 + `tests/ptx/bless_log.md` 追加留痕,`ci/check_guardrails.py` `check_ptx_bless` 机器核对既有行 0-byte)。NVPTX 雷区(shfl 选择失败/sqrt 近似约束类)遇雷登记雷区回归集并 pin 绕行——SAXPY 雏形子集不触发,机制就位备 M4.3+ 扩展。

14 §2 常驻集其余项的 M4 期评估结论:

| 项 | 结论 |
|---|---|
| MIR 文本 golden | M3.3 WP6 已激活(check_mir_bless),M4 沿用;device MIR 形态变更纳入既有 golden 核对 |
| stable API 快照 | M4 无 stable 面,不激活 |
| unsafe-audit 完整性 | rurixc 实现侧 `unsafe_code = deny` 维持;**M4.3 运行时落地激活**——`rurix-rt`(CUDA Driver API FFI 首个 unsafe 边界)按 AGENTS 硬规则 9 注册条目 [unsafe-audit/rurix-rt.md](../../unsafe-audit/rurix-rt.md)(U1~U8 原语 + 验证义务),crate 级 `unsafe_code = allow` + `undocumented_unsafe_blocks = deny`(每 unsafe 块强制 `// SAFETY:`);全仓其余 crate 维持 deny |
| Compute Sanitizer | M4 device 运行时落地后 memcheck nightly 评估(§3);全量 racecheck 随 M5 |

m0~m3 历史预算的回填/冻结走 `check_guardrails.py` 既有机制("estimated 条目只允许回填为 measured_local";measured_local 条目 0-byte),不属新增激活项。

## 5. 验证程序(对应契约 G-M4-1/G-M4-2/G-M4-3/G-M4-4 与步骤 17–20)

1. 步骤 17 落地后,构造 ptxas 拒绝场景(注入非法 PTX 或破坏 codegen 产出)→ rurixc 必须报 RX6xxx、step 红;修复后同 PR 转绿,两次 run URL 随 close-out 归档。
2. 步骤 18 落地后,构造**故意放行某 launch 反例类别**(或篡改 reject 语料预期)的 PR → 必须红;修复后转绿,run URL 归档。
3. 步骤 19(黄金路径 4)走 bless 审批:篡改 .stderr 不附 bless 行 → `check_ui_bless` FAIL;补 bless 留痕 → 绿,run URL 归档。
4. 步骤 20(GPU SAXPY)落地后,构造拷回核对失败(篡改 kernel 语义)→ exit 1 红;复原 → exit 0 绿,run URL 归档。
5. PTX golden 激活时(§4 第 3 项),故意改 PTX 输出不更新基线 → 红;按审批更新基线 → 绿,run URL 归档。
6. close-out 附 `budget_eval --strict` 输出原文(契约 G-M4-1 比值 ≥0.95 与全局零 estimated 残留判定)+ SAXPY measured_local 证据 JSON 路径 + NVIDIA 白名单评估结论。

## 6. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-06-13 | 初版(M4 契约配套;步骤 17–20 为 M4.2/M4.3/M4.4 计划项,落地时回填实测命令;guardrail 三项动作:基准切换 m2-closed→m3-closed、NVIDIA 白名单评估、PTX golden 评估均为计划项) |
| v1.1 | 2026-06-13 | §4 第 1 项落地(M4.1 任务 1):`ci/check_guardrails.py` 本地/push 回退基准 `m2-closed → m3-closed`(`resolve_base()` 默认值 + 文件头 docstring;PR 路径仍以 GITHUB_BASE_REF 优先,既有逻辑不变)。切换前双基准核对均 PASS:`py -3 ci/check_guardrails.py m2-closed` = PASS(118 changed paths);`py -3 ci/check_guardrails.py m3-closed` = PASS(2 changed paths)。配套 `ci/budget_eval.py` 新增 `m4.counter.launch_conformance_categories`(≥4)/ `m4.counter.ui_golden_path4_snapshots`(≥10)evaluator 分支(目录缺失 → 0 → normal SKIP,对齐 M3 计数器先例);`py -3 ci/budget_eval.py` = PASS(19 pass, 3 skip, normal mode),M4 期 `m4.*` 占位/计数器 SKIP 属预期。`tests/test_budget_namespace.py` 17 passed;`check_schemas` / `trace_matrix --check`(65/65)/ `check_structure` 全 PASS |
| v1.2 | 2026-06-13 | M4.1 任务 2~5 落地(spec device 条款先行 → 着色/地址空间检查):spec/device.md 新增 RXS-0066~0069(着色规则/地址空间映射/barrier uniform 骨架/诊断要求,档位 Direct);registry 分配 RX3001~RX3003(3xxx 着色/地址空间段位首批)+ en.messages `coloring.*`/`addrspace.*`;rurixc 新增 HIR 层着色检查 pass(`coloring.rs`,RX3001 跨着色调用 + RX3003 barrier 骨架)+ typeck 地址空间一致性(RX3002,`View` 族空间类型参数合一裁决);conformance/coloring + conformance/addrspace accept/reject 语料 + `coloring_corpus.rs` 批跑;tests/ui/coloring(×3)+ tests/ui/addrspace(×1)黄金路径 4 的 3xxx 子集 snapshot(bless 留痕 bless_log.md)。`m4.counter.ui_golden_path4_snapshots` 计数目录由 `codegen` 扩为 `coloring`+`addrspace`+`codegen`(G-M4-3 覆盖 3xxx+6xxx 两段;当前 4 条 < 10,normal SKIP,6xxx 随 M4.3 补足)。验证:`cargo test --workspace` 全绿;`check_schemas` / `trace_matrix --check`(69/69 锚定)/ `check_guardrails m3-closed` / `budget_eval`(19 pass, 3 skip)全 PASS |
| v1.7 | 2026-06-14 | M4.4 落地(Rurix SAXPY 上 GPU,契约 D-M4-5 / G-M4-1 真跑通道):新增 Rurix kernel 源 `src/rurix-rt/kernels/saxpy.rx`;rurixc 库抽出 `toolchain::ir_to_ptx`(clang NVPTX 后端,bin `--emit=ptx` 与 rurix-rt build.rs 复用单一事实源)+ **IR→PTX 改用 `-O2`**(NVPTX `-O0` 对 i64 索引 lowering 产错误地址 → 越界访存;`-O2` 修正且打满带宽,IR golden 在 IR 层不受影响)。rurix-rt 加 `build-dependencies = rurixc` + `build.rs`(saxpy.rx → 着色检查 → device codegen → IR→PTX → ptxas 关卡 → 嵌入 `$OUT_DIR/saxpy.ptx` + 入口符号名;工具链缺失写空哨兵降级 SKIP);新增 host 驱动 bin `src/rurix-rt/src/bin/saxpy.rs`(嵌入 PTX,alloc/H2D/launch/D2H/逐元素核对,exit 0)+ 端到端测试 `rurix_saxpy_e2e_isolated`(子进程隔离 + GPU/工具链 SKIP,锚定 RXS-0070~0072/0076)。**§2 步骤 20 实测命令回填**(`cargo test -p rurix-rt` + `cargo run --bin saxpy`)。bench harness `bench/rurix_saxpy_bench.py` + 操作者三次运行/回填工具 `bench/rurix_saxpy_triple.py`(§3 nightly)+ 提交 rurixc 生成 PTX `bench/kernels/rurix_saxpy.ptx`。**真跑验证(本机 RTX 4070 Ti + clang 22.1.7 + CUDA 13.3 ptxas)**:`cargo run -p rurix-rt --bin saxpy` exit 0(1048576 元素 f32 精确相等)、`cargo test -p rurix-rt` 全绿(含 e2e)、`bench/rurix_saxpy_bench.py --smoke` 正确性 PASS。**G-M4-1 measured 回填(三次锁频采样 + `m4_budget.json` 比值回填 + close-out 终稿)留操作者在锁频环境执行**(命令与回填位置见 M4_CONTRACT §8 / `bench/rurix_saxpy_triple.py`)。验证:`cargo test --workspace` 全绿;`check_schemas` / `trace_matrix --check` / `check_guardrails m3-closed` / `budget_eval`(normal,`m4.ratio` 仍 SKIP 属预期)全 PASS |
| v1.6 | 2026-06-13 | M4.3 收口(G-M4-4 ptxas 干验证关卡真红绿达成):开发机安装 CUDA Toolkit v13.3(ptxas V13.3.33);ptxas 关卡逻辑自 bin 抽出至 lib `src/rurixc/src/ptxas.rs`(`dry_gate` / `locate_ptxas` / 非 ASCII 路径防御),bin `--emit=ptx` 经此关卡,供红绿单测复用。新增永久红绿测试 `src/rurixc/tests/ptxas_gate.rs`(`cargo test -p rurixc --test ptxas_gate`):**GREEN** 合法 PTX 过 `ptxas -arch=sm_89`(`Pass`);**RED** 注入非法 PTX → ptxas 拒绝(`Rejected`,驱动映射 RX6004);无 ptxas 降级 SKIP。**真跑验证(本机 CUDA 13.3 ptxas)**:`rurixc tests/ptx/saxpy.rx --emit=ptx` exit 0 + 空 stderr = ptxas 关卡 PASS(非 SKIP);`ptxas_gate` 测试 GREEN+RED 全绿(ptxas 诊断落 stdout,关卡两路合并取摘要)。**至此 M4.3 出口判据 G-M4-2 / G-M4-3 / G-M4-4 + 运行时全链路真跑全部达成**;G-M4-1(SAXPY measured ≥95%)属 M4.4。验证:`cargo test --workspace` 全绿;`check_schemas` / `trace_matrix --check`(77/77)/ `check_guardrails m3-closed` / `budget_eval` 全 PASS;rurixc/rurix-rt clippy 无新增警告 |
| v1.5 | 2026-06-13 | M4.3 续(运行时全链路真跑落地,契约 D-M4-4 出口判据):新建 `src/rurix-rt` crate(workspace 成员)——CUDA Driver API 薄层(Context affine 根 + poisoned 状态机 / Stream / DeviceBuffer / PinnedBuffer / Module / Kernel + 经典内存路径 `cuMemAlloc`/`cuMemAllocHost` + 显式 H2D/D2H + `cuLaunchKernel`,D-230~D-232);`nvcuda.dll` 经 `LoadLibraryA`/`GetProcAddress` **运行时动态加载**(不依赖 CUDA Toolkit 的 `nvcuda.lib`,沿用 M0 ctypes 先例,14 §2 PTX-only)。spec/device.md 续写 RXS-0076(装载协商:`.version` 比对降版 + JIT 日志 + 可执行指引)/ RXS-0077(poisoned 状态机:`CUDA_ERROR_ASSERT`/`CONTEXT_IS_DESTROYED` → 确定性 `Err`),档位 Direct——运行时结构化 `CudaError`(Result,保留原始 CUresult),不占编译期 RX#### 段位。**§2 步骤 21**(运行时全链路真跑,GPU 队列,子进程隔离 14 §6)接入。**§4 unsafe-audit 激活**(首个 unsafe 边界,注册 [unsafe-audit/rurix-rt.md](../../unsafe-audit/rurix-rt.md);crate 级 `unsafe_code=allow` + `undocumented_unsafe_blocks=deny`)。trace_matrix 锚定源新增 `src/rurix-rt/**/*.rs`。**真跑验证(本机 RTX 4070 Ti,driver JIT,无 Toolkit)**:`cargo test -p rurix-rt` 全绿——装载协商 `.version=8.0` 通过,SAXPY 4096 元素 f32 全链路(装载→H2D→launch→D2H)逐元素精确相等;无 GPU 环境降级 SKIP(对齐 ptxas 关卡纪律,真红绿在带 GPU runner)。ptxas 严格干验证关卡(步骤 17 / G-M4-4)仍 SKIP(本机无 Toolkit ptxas)。验证:`cargo test --workspace` 全绿;`check_schemas` / `trace_matrix --check`(77/77 锚定)/ `check_guardrails m3-closed` / `budget_eval` 全 PASS |
| v1.4 | 2026-06-13 | M4.3 落地(编译器侧 launch 类型契约闭环):spec/device.md 续写 RXS-0074~0075(launch 类型契约:着色/维度/参数/context-brand 四类 + 诊断要求,档位 Direct);registry 分配 RX3004~RX3006(3xxx 段位续接;launch 参数类型不符复用 RX2001、View 空间不符复用 RX3002)+ en.messages `launch.*`。rurixc 新增 launch 类型契约检查:resolve 注册 `Context`/`Module`/`Stream`(brand)/`GridDim`/`BlockDim` lang items(类型/值位置兜底,可遮蔽)、typeck 容忍 `Stream::launch` 与 `GridDim/BlockDim` 构造、`launch_check.rs`(HIR 层四类裁决,query `check_launch()` 插入 coloring 之后)。**§2 步骤 18/19 实测命令回填**:步骤 18 = `cargo test -p rurixc --test launch_corpus`(conformance/launch accept/reject 四类 + 计数器 `launch_conformance_categories`=4);步骤 19 = `cargo test -p rurixc --test ui_golden`(tests/ui/launch ×4 黄金路径 4 launch 子集,`budget_eval` 计数目录扩为 {coloring,addrspace,codegen,launch},`ui_golden_path4_snapshots`=11 ≥10 达成 G-M4-3)。tests/ui/launch ×4 snapshot bless 留痕 `tests/ui/bless_log.md`。**运行时(rurix-rt:Context/Stream/Buffer/launch Driver API + 装载协商 + poisoned 状态机)+ 装载协商/poisoned spec 条款随后续 WP**(需 CUDA toolkit/GPU 真跑,本机 ptxas 缺失;规范先行:装载协商/poisoned 条款与运行时实现同 PR)。验证:`cargo test --workspace` 全绿;`check_schemas` / `trace_matrix --check`(75/75 锚定)/ `check_guardrails m3-closed` / `budget_eval` 全 PASS |
| v1.3 | 2026-06-13 | M4.2 落地(NVPTX codegen 与 ptxas 关卡):spec/device.md 续写 RXS-0070~0073(codegen 目标与 `ptx_kernel` 调用约定 / addrspace codegen 建模 / 线程索引与 launch bounds / ptxas 干验证关卡,档位 Direct);registry 分配 RX6003~RX6005(6xxx codegen/目标 device 首批)+ en.messages `codegen.device_*`/`codegen.ptxas_rejected`。rurixc 新增 device codegen 链路:`mir_build::build_device_crate`(`kernel fn` 为根)、MIR `Body.color` / `ProjElem::Index` / `CallTarget::DeviceIntrinsic`、`ThreadCtx` lang item + 线程索引 device intrinsics(RXS-0072)、`View` 族索引 typeck/MIR 落位(RXS-0071)、`device_codegen.rs`(MIR→NVPTX IR:`ptx_kernel` cc / `ptr addrspace(N)` / sreg intrinsics);驱动 `--emit=nvptx-ir`(NVPTX IR)/ `--emit=ptx`(clang `--target=nvptx64-nvidia-cuda` IR→PTX + ptxas -arch=sm_89 干验证,缺 ptxas → SKIP)。步骤 17 实测命令回填(本表 §2)。**§4 第 3 项 PTX golden 激活**:`ptx_golden.rs` + `tests/ptx/`(saxpy / thread_index,×2)字节 golden + `check_ptx_bless` guardrail(bless 留痕 `tests/ptx/bless_log.md`);tests/ui/codegen(×3,RX6001/RX6003/RX6005)黄金路径 4 的 6xxx 子集 snapshot(`m4.counter.ui_golden_path4_snapshots` 现 7 条 < 10,normal SKIP,余随 M4.3)。验证:`cargo test --workspace` 全绿;`check_schemas` / `trace_matrix --check`(73/73 锚定)/ `check_guardrails m3-closed` / `budget_eval` 全 PASS;SAXPY 雏形经 `--emit=ptx` clang 真跑产合法 PTX(`.version 7.8` / `.target sm_89` / `.visible .entry` / `ld.global`/`st.global`),ptxas 关卡本地 SKIP(无 CUDA),真实红绿待 self-hosted runner |
