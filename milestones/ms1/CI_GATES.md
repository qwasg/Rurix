# MS1 CI 门禁增量

> 所属契约:[MS1_CONTRACT.md](MS1_CONTRACT.md)
> 版本:v1.0(2026-07-14)
> 基线:[../m0/CI_GATES.md](../m0/CI_GATES.md) ~ [../v1/CI_GATES.md](../v1/CI_GATES.md)(全部沿用:runner 约定、PR Smoke 1–51 步、Release 层门禁(含 channel-manifest 第 8 子门)、guardrail 全部激活项(含 stable 快照 bless)、nightly 全量回归冻结);本文只规定 MS1 期的**增量**。
> 铁律不变:任何新增门禁必须在真实 PR 上以真实失败/通过路径验证过(反 YAML-only)。
> 开工脚手架口径:本文 MS1 增量步骤(52/53)为 **MS1.2/MS1.3 计划项**,开工**不**写入 workflow YAML 真实步骤(随实现 PR 落地回填,对齐 M8~V1 计划 → 回填范式)。**MS1 开工脚手架零 CI 代码改动**:预算 glob 已泛化为 `*_budget.json`,自动纳入 `ms1_budget.json`;`check_closed_contracts` glob 与无参默认基准 `v1-closed` 均已就位;**counter/entries 不预造**(登记与 `ci/budget_eval.py` evaluator 新分支同实现 PR 落,未知 id 强制 FAIL)。

---

## 1. Runner

沿用 M0 §1(自托管 RTX 4070 Ti 开发机 `rurix-dev-4070ti`)~ V1 §1。MS1 新增 runner 预置项:**无**——步骤 52/53 的 device 段依赖既有 CUDA 工具链 + 真 GPU(步骤 22/43/46~48 先例);离线 golden 为确定性出图,runner 可硬门。实时 present 需交互桌面,**不进 CI**(evidence 面,本机人工链路真跑取证,镜像 `ci/realtime_present_smoke.py` 双态先例:SKIP 不充绿)。

## 2. PR Smoke 追加步骤(计划项,编号接 V1 §2 的 50 与 MR-0009 的 51;落地随 MS1.2/MS1.3 实现 PR 回填 workflow)

| # | 步骤 | 失败即红 |
|---|---|---|
| 52 | single-source 宿主编排冒烟(契约 G-MS1-2 通道;MS1.2 落地接入,**RFC-0009 前置后**):`ci/host_orch_smoke.py` —— host 段(总跑,无 GPU 也跑):conformance/host_orch 最小单源 .rx(宿主编排 + kernel 同编译单元)经 `rx build` 产 EXE + rurix_rt_cabi 链接成功;reject 语料(affine 误用 / launch 契约违例 / 宿主 API 进 device 上下文)编译期拦截。device 段(runner 真 GPU):运行 EXE → 真 launch → 数值对照(见证行 `HOST_ORCH: ok ...`);本机无 CUDA 降级 SKIP(除 `RURIX_REQUIRE_REAL=1`,runner 置位)。red→绿闭合(反 YAML-only,内建):① 篡改嵌入 PTX 字节 → 装载协商拒(RXS-0192)→ 红;② 桩化 device 写回 → 数值对照红;复原 → 绿。写 `evidence/host_orch_smoke.json`(schema 校验) | 是 |
| 53 | UC-07 离线 golden 冒烟(契约 G-MS1-3/G-MS1-4 通道;MS1.3 落地接入,**RFC-0010 前置后**):`ci/uc07_offline_golden_smoke.py` —— 前置审计(主语言判据机器面):apps/ruridrop 源清单**零 .rs** + 宿主/kernel 同包 + EXE 由 `rx build` 产出。device 段(runner 真 GPU,冒烟档 N=4096/160×120/8spp/2 帧):① 确定性硬门:同机两次运行逐帧量化 PPM 字节 SHA-256 逐字节一致;② 参考容差硬门:GPU 帧 vs refcpu 入口 CPU 参考,量化域 ≥99.5% 像素每通道 ≤1 LSB 且最大 ≤2 LSB;③ blessed 哈希软门:逐帧 SHA-256 == tests/uc07/golden_manifest(漂移 → 红 + 重 bless 留痕路径提示)。数据流红绿(内建):篡改 kernel 物理/着色常数经同一编译链重编 → digest 变红 → 复原绿(镜像步骤 48 先例,仅「跑完了」不接受)。写 `evidence/uc07_offline_golden_*.json` | 是 |

预算 evaluator 自动合并加载 [ms1_budget.json](ms1_budget.json)(命名空间冲突即红;**开工全空,counter 登记与 evaluator 分支随 MS1.2/MS1.3 实现 PR 同落,性能 entries 随 MS1.4 measured_local 回填**)。**MS1 close-out 必须跑 `--strict` 且全局零 estimated 残留**(14 §3)。

## 3. Release 层门禁

MS1 **零 Release 层增量**:不动 release.yml 触发器/门集(既有 8 子门 0-byte);ruridrop 为仓内应用非分发产物,不进 bundle/SBOM 面。若后续裁决把 ruridrop 纳入发行示例包,另按 10 §3 判档(预期 Mini-RFC),本期不做。

## 4. Nightly 追加

- 既有 nightly 全保留(Compute Sanitizer racecheck+memcheck + measured 基准 + 全量回归冻结)。
- **MS1 无新增 nightly 项**:离线 golden 归 PR smoke 步骤 53(冒烟档秒级);生产档性能为 MS1.4 一次性 evidence,非趋势项。ruridrop kernel 可选纳入 nightly Sanitizer 扫描面留 MS1.3 执行期评估(若纳入,随该 PR 回填本表)。

## 5. Guardrail

沿用 M0~V1 全部激活项。MS1 期动作:

1. **基准 ref 默认 `v1-closed`**:V1 close-out 已切,MS1 开工无需再切;PR 路径以 `GITHUB_BASE_REF` 为准。MS1.5 close-out 时切至 `ms1-closed`(agent 自主签署;`ms1-closed` 不匹配 release.yml 收窄触发器 `v[0-9]+.[0-9]+.[0-9]+*`,零误触发)。
2. **stable 快照 bless(check_stable_snapshot_bless)**:MS1 预期触发**两次**——MS1.2 条款 RXS-0189~0196(spec_clauses 184→192)、MS1.2b 条款 RXS-0197~0199(192→195);各与条款/实现同 PR 重 bless + `tests/stable/bless_log.md` 同 diff 追加(数据行忌「日期」子串);不可分 PR(步骤 49 硬红)。
3. **错误码按段新增**:MS1 预期新码(RX1005/RX2010/RX3015/RX6024/RX6025/RX7021/RX7022,以实现实际为准)——`registry/error_codes.json` 只追加 + en/zh messages 成对(`ci/bilingual_coverage.py` 缺键即红,88→N)。
4. **unsafe 边界**:全仓 `unsafe_code=deny` 维持;**src/rurix-rt-cabi 为 FFI 边界例外 crate**,新 unsafe 逐处 `// SAFETY:` + unsafe-audit **U25** 续号登记(镜像 rurix-interop / rurix-d3d12 先例);apps/ruridrop 全 .rx 无 unsafe 面。
5. **新 golden 面**:tests/uc07/(出图 golden manifest)纳入 bless 纪律;UI golden 新增(host_orch reject .stderr)经既有审批 bless 机制。
6. **spec 修订行纪律**:spec/host_orchestration.md 修订表表头「版本」;数据行避「版本」子串(用「版号」)。
7. **规划文档冻结**:00–14(含 13)执行 PR 0-byte;开工裁决记 MS1_CONTRACT §7;02 不因 UC-07 改写;11 §6 落地标注(可选)走 00 §6.3 独立勘误 PR(close-out 后)。
8. **trace 矩阵扫描面**:新 crate src/rurix-rt-cabi 计入 `ci/trace_matrix.py` `gather_repo()` 扫描列表(随 MS1.2 实现 PR 同落);apps/ruridrop 的 .rx 锚定经 conformance/ 语料承载(应用文件本身不进锚定源,避免 golden 语料与应用耦合,MS1.3 执行期按需评估)。
9. **LF byte-exact**:新文件 LF+尾换行;registry/*.json 等既有 CRLF 例外文件追加行保持原行尾风格、既有行 0-byte;禁 Python 文本模式写文件。

14 §2 常驻集其余项的 MS1 期评估结论:

| 项 | 结论 |
|---|---|
| stable API 快照 | 已激活;MS1 两次加性重 bless(RXS-0180 L2,同 edition 2026 只增不破坏) |
| MIR/PTX/DXIL/UI golden | 已激活;MS1 预期新增 UI golden(host_orch reject),PTX/MIR/DXIL 零变更预期 |
| unsafe-audit 完整性 | 已激活;U25(rurix-rt-cabi)随 MS1.2 登记 |
| Compute Sanitizer / NVIDIA 白名单 | 已激活维持;ruridrop kernel Sanitizer 扫描面 MS1.3 评估 |
| 多后端(D-008,SG-003) | **维持 not_triggered**(CUDA/PTX compute 路线 = NVIDIA 单栈纵深,红线 3 不解除) |
| registry sumdb(D-312,SG-007)/ MLIR(SG-001)/ Tensor Core(SG-002)/ autodiff·fusion(SG-004/005)/ Python 嵌入(SG-008)/ 自举(SG-009) | 维持 not_triggered;SG-010 留续号(窗口/UI 框架进语言、通用异步宿主运行时扩张诱惑 → 登记 gating 而非提案) |
| 贡献校验门(ci/check_contribution.py) | 已激活延续(provenance / 条款号 / 验证标记三类缺项即红) |

## 6. 验证程序(对应契约 G-MS1-1~G-MS1-6 与计划步骤 52/53)

1. MS1.1:双 RFC 合入序核验(RFC 合入 commit 先于任何实现 commit;失败测试先行声明成立——步骤 52/53 脚本在 RFC 合入时点 main 上不存在)。
2. MS1.2 步骤 52 落地后:本机 `py -3 ci/host_orch_smoke.py`(host 段 + device 段真跑 + 篡改 PTX/桩化双红绿);runner PR run URL 归档;`trace_matrix --check` / `stable_snapshot --check`(184→192 重 bless 后)/ `bilingual_coverage` / `check_guardrails`(基准 v1-closed)全绿。
3. MS1.2b:uc03 回归网(cargo test -p uc03-demo / -p rurix-rt)零漂移;快照 192→195 重 bless 复绿;.rx present 错序语料编译期拦截。
4. MS1.3 步骤 53 落地后:零 .rs 审计 + 三层 golden + 数据流红绿本机与 runner 双真跑;首次 golden bless 留痕 tests/uc07/bless_log.md。
5. MS1.4:present evidence JSON(measured_local)+ ms1.bench.* entries 回填后 `budget_eval`(entries 走 evidence_file trimmed_mean 判读)。
6. close-out:`budget_eval --strict` 输出原文(全局零 estimated)+ G-MS1-1~6 留痕指针 + RD 处置 + SG 复评 + 双基准(v1-closed / ms1-closed)advisory 复核输出。

## 7. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-14 | 初版(MS1 契约配套;计划步骤 52/53 为 MS1.2/MS1.3 计划项,落地时回填 workflow YAML 实测命令与 run URL;Release 层零增量;nightly 零增量;guardrail 动作:基准 v1-closed 无需再切、快照两次重 bless、错误码按段新增 en/zh 成对、rurix-rt-cabi FFI unsafe 例外 U25、tests/uc07 golden bless 纪律、close-out 切 ms1-closed;SG 全维持 not_triggered + SG-010 留续号)。**MS1 开工脚手架零 CI 代码改动**:ms1_budget.json 经 *_budget.json glob 自动纳入,counter/entries 不预造(登记与 evaluator 分支同实现 PR 落);开工不写入 workflow YAML 真实步骤 |
| v1.1 | 2026-07-14 | **MS1.2 步骤 52 落地回填(G-MS1-2,RFC-0009,RXS-0189~0196)**:计划步骤 52 落地为 `.github/workflows/pr-smoke.yml` 真实步骤 `ci/host_orch_smoke.py`(步骤 51 之后;env `RURIX_REQUIRE_REAL: "1"`,runner 真 GPU 不许 SKIP,镜像步骤 46~48)——① host 段(总跑,无 GPU 也跑):reject 语料六类编译期拦截(mod_missing/mod_cycle → RX1005 / elem_infer → RX2010 / gpu_in_kernel → RX3015 / launch_arg_subset → RX6024 / buffer_move → RX4001)+ accept 语料三例(mod_file/extern_link/saxpy_single_source)`rx build` 全绿(EXE 落 %TEMP%,不留仓库)+ 链接面见证(saxpy 单 EXE 存在且非空 = kernel PTX 嵌入 + rurix_rt_cabi 链接成功);② device 段(真 GPU,探测 = CUDA_PATH + ptxas 抄 fatbin 先例):saxpy 单 EXE 真跑 → 真 launch → 数值自校验 exit 0,见证行 `HOST_ORCH: ok single_source=true device_run=true`;③ red→绿闭合(内建,防降级硬门,反 YAML-only,仅 device 可用时执行):红① 篡改 EXE 内嵌 PTX 6 字节(非 UTF-8 字节)→ 装载协商拒 `RXRT: error` + 非零退出 / 红② 变体源桩化 kernel 写回(`out[i]` 赋值改纯读)经**同一 rx build 链**重编 → 数值自校验红 / 复原核验原 EXE 复跑 exit 0。配套(同 PR 落,不预造纪律兑现):`milestones/ms1/host_orch_smoke_evidence_schema.json`(evidence 仅 device 真跑时写)+ `ci/check_schemas.py` 前缀路由 `host_orch_smoke` + `ms1_budget.json` v1.1 counter `ms1.counter.host_orch_single_source`(>=1)+ `ci/budget_eval.py` eval_counter 新分支(无 device 真跑 = 建设期 normal SKIP)。本机真实全链:`py -3 ci/host_orch_smoke.py` PASS(host 六 reject + 三 accept + 链接面 + device 真跑 + 双红绿 + 复原绿);run URL 随 PR 合入后归档契约 §8(不伪造) |
| v1.2 | 2026-07-14 | **MS1.2b present typestate + 宿主图像落盘桥回填(G-MS1-1,RFC-0009 §4.6/§4.7,RXS-0197~0199)**:步骤 52 冒烟扩面——reject 六类 → **八类**(+present_out_of_order → RX4001 move 违例 / +present_in_kernel → RX3015),accept 三例 → **五例**(+present_loop 编译+链接冒烟(rxp_* declare 面,不运行,交互桌面依赖归 MS1.4)/ +imageio_write **device 真跑落盘**:kernel 着色 → download → write_ppm → PPM P6 头 + 字节见证,RXS-0199/RXS-0116 量化 0-byte)。配套:src/rurix-rt interop OwnedPresentSession 重构(scope()/uc03 公共面 0-byte,回归网 cargo test -p uc03-demo 零漂移)+ cabi rxp_* 七符号(thread_local 会话表,零新 unsafe)/ rxio_write_ppm + Borrowed 句柄(free no-op)+ rurixc present 四态 lang items/typeck/mir_build lowering + conformance/host_orch 语料四例 + stable 快照重 bless 192→195(bless_log 同 diff)。MS1.2b **零新 RX 码**(错序 = 既有 RX4xxx;运行期 = RXS-0193 口径);trace 195/195 全锚定。本机真实全链:`py -3 ci/host_orch_smoke.py` PASS(八 reject + 五 accept + saxpy/imageio device 真跑 + 双红绿 + 复原绿) |
| v1.3 | 2026-07-15 | **MS1.3 步骤 53 落地回填(G-MS1-3/G-MS1-4,RFC-0010 §4.1/§4.4)**:计划步骤 53 落地为 `.github/workflows/pr-smoke.yml` 真实步骤 `ci/uc07_offline_golden_smoke.py`(步骤 52 之后;env `RURIX_REQUIRE_REAL: "1"`,runner 真 GPU 不许 SKIP,镜像步骤 52)——① 前置审计(主语言判据机器面,host 段总跑):apps/ruridrop 文件集仅 .rx + rurix.toml(发现 .rs/.py/.cpp/.c/.h 等任何其他源即红并列出违例)+ src/*.rx 集合含 kernel fn 定义(kernel 与宿主编排同包)+ offline_smoke/refcpu 两 EXE 均经 `rx build` 产出(产物链路防降级硬门,措辞镜像 G-G2-4);② device 段(真 GPU,冒烟档 N=4096/160×120/8spp/2 帧,探测 = CUDA_PATH + ptxas 抄步骤 52):确定性硬门(独立 tmp 子目录两跑逐帧量化 PPM 字节 SHA-256 相等)+ 参考容差硬门(GPU 帧 vs refcpu 入口(同一 .rx device fn host 重放)逐像素 \|Δ\|≤1 占比 ≥99.5% 且 max ≤2,P6 头解析后纯像素字节)+ blessed 哈希软门(逐帧 SHA-256 == tests/uc07/golden_manifest;漂移 → 红 + 重 bless 路径提示 `RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py`,bless 后仍走完全部硬门,留痕 tests/uc07/bless_log.md);③ 数据流红绿(内建,防降级硬门,反 YAML-only):篡改 params_smoke.rx 传给 sim_forces 的重力常数 GRAVITY 10.0→2.5 经**同一 rx build 链**重编 → 逐帧 digest ≠ golden(变红)→ 变体丢弃、原树 0-byte 未动即复原绿(镜像步骤 48 先例,仅「跑完了」不接受)。配套(同 PR 落,不预造纪律兑现):`milestones/ms1/uc07_offline_golden_evidence_schema.json`(evidence 仅 device 真跑全绿时写)+ `ci/check_schemas.py` 前缀路由 `uc07_offline_golden` + `ms1_budget.json` v1.2 counter `ms1.counter.uc07_offline_golden_frames`(>=1)+ `ci/budget_eval.py` eval_counter 新分支(无 device 真跑 = 建设期 normal SKIP)+ 首次 golden bless(tests/uc07/{golden_manifest,bless_log.md},RTX 4070 Ti 真跑定基)。本机真实全链:`py -3 ci/uc07_offline_golden_smoke.py` PASS(审计三见证 + 两跑逐帧一致 + 容差实测 + manifest 全等 + 篡改红/复原绿);run URL 随 PR 合入后归档契约 §8(不伪造) |
| v1.4 | 2026-07-15 | **MS1.4 实时 present 取证 + 三项性能预算回填(G-MS1-5/G-MS1-6,RFC-0010 §4.5/§4.6;evidence 面,零新 CI 步骤——实时窗口需交互桌面,SKIP 不充绿双态先例,§1 口径不变)**:① realtime.rx 取证扩面(仅 .rx,apps/ruridrop 维持零 .rs):帧数上限 `MAX_FRAMES=600`(G-MS1-5 ≥300,窗口关闭提前退出),循环结束后同一末帧排序态 rt_primary 重渲进**普通 Buffer(非 backbuffer)**download 采样——全域 min/max/mean 粗测 + 天空区顶行两点(确定性天空梯度,蓝>红)+ 水体区两点(初始水柱脚印内世界点经相机基底投影)范围核验,EXE 内自校验打印 `REALTIME_OK frames=<n> sample_ok=true` / 失败退出非 0(数值行经 CRT putchar,RXS-0195 sim_check 先例);present-real 系统库(user32/d3d12/dxgi/d3dcompiler)经 `#[link]` 接线(RXS-0195);+ sim-only 双基准入口 sim_bench_short/long.rx(8/72 帧,params 生产档,墙钟差分口径)。② 取证/采集 = `ci/uc07_bench.py`(操作者工具,不进 CI;L0 锁频 `-lgc/-lmc` 读回校验,未锁频 evidence_level=unlocked 如实降级不得回填):present 子命令真窗口跑(present-real cabi 独立 target-dir crt-static 构建 + `RURIX_RT_CABI_LIB` 指向,RX7021 定位序)→ `evidence/uc07_present_20260715.json`(frames=600 / sample_ok=true / 采样对照 / 环境画像,经 milestones/ms1/uc07_present_evidence_schema.json 校验,check_schemas 前缀路由 `uc07_present_`);三项 bench 各 3 次进程级独立运行 + trimmed mean 聚合(BENCH_PROTOCOL 三次运行规则;计时 = 进程级墙钟 wall_clock_process,与 m0 cuda_event 内层协议差异在 schema description / sampling.method 如实声明)→ `evidence/uc07_{sph_step_ms,offline_frame_s,realtime_frame_ms}_20260715_{1..3,agg}.json`(经 milestones/ms1/uc07_bench_evidence_schema.json 校验,前缀路由三支)。③ ms1_budget.json v1.3 entries 回填三条 measured_local:uc07_sph_step_ms = 11.4610 ms(阈值 = 实测 ×1.5 = 17.1915)/ uc07_offline_frame_s = 0.1157 s(阈值 0.1736;生产档 8 帧全序列的 32spp+2 弹射**可测切片**——完整 256spp/4 弹射档在当前工具链下毒径挂起,pt_render 样本序号/弹射深度相关,rt_primary 同数据全绿,疑 PTX 发散重汇聚类工具链缺陷,登记 RD-027(deferred v1.50)跟进不在本轮修;计划名 uc07_pt_frame_ms 按实测口径改名)/ uc07_realtime_frame_ms = 14.8334 ms(实测 ≤22.2 → 阈值按 30fps 档定 33.3);evaluator entries 数据驱动零新分支。本机真实全链:realtime 真窗口 600 帧 `REALTIME_OK frames=600 sample_ok=true` + `budget_eval` 71→74 pass + `check_schemas` PASS |
