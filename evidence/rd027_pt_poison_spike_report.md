# RD-027 生产档 PT 毒径判别 Spike — 取证报告（G3.1,RD-027,验收门 G-G3-1 归因闸门）

| 字段 | 值 |
|---|---|
| 类型 | **Spike 取证报告**（机器事实汇总 + 复现清单;**非立项、非实现、非性能基准、非常驻 CI 门**）。本报告只摆证据,归因裁决单独成节（§5）且逐条由 §3/§4 证据支撑,不以叙事代替数据。 |
| 承接 | G3.1 RD-027 毒径判别 spike（验收门 G-G3-1);MS1.4 性能取证实录发现 → params.rx 切片锁定（STUB(RD-027));owner 2026-07-18 裁「归因落地即开闸」(G3_CONTRACT §7 ②)。 |
| 范围 | 四层嫌疑（rurixc IR / LLVM NVPTX / ptxas / 驱动 JIT）判别矩阵 + 最小化复现 + 归因裁决;**不含处置尾项实现**（上游 DRAFT 备包与护栏落地归处置 PR,G3 close-out 前置）。 |
| 跟踪锚 | RD-027（[registry/deferred.json](../registry/deferred.json)）· 闸门载体 [milestones/g3/G3_CONTRACT.md](../milestones/g3/G3_CONTRACT.md) G-G3-1 |
| 机器证据 | [evidence/rd027_pt_poison_spike_20260718.json](rd027_pt_poison_spike_20260718.json)（schema:[milestones/g3/rd027_spike_evidence_schema.json](../milestones/g3/rd027_spike_evidence_schema.json),经 `ci/check_schemas.py` PASS);逐 run 原始记录 `build/spike-rd027/campaign.jsonl`（工作区工件,不入库） |
| 探针 | [spike/rd027-pt-poison/](../spike/rd027-pt-poison/)（标 `// SPIKE(RD-027)`,不入 src/ 生产路径、不随产品编译,spike 结束可弃;MRP 常驻 `mrp/`） |
| 纪律 | measured-first / blocked-honest（硬规则 3/4):全部 GPU 运行经 `bench/proc_guard` guarded_run（硬超时 = 诚实红 exit 124、杀进程树、僵尸 exe 隔离,零裸 launch,R-606)+ 挂起判定后金丝雀门;**绝不杜撰数字**,一切数值取自机器证据。 |
| Provenance | `Assisted-by: claude-code:claude-fable-5`（agent 自主记录机器可核对事实与自主签署归因,D-406 v2.0） |

---

## 1. 背景与承接

MS1.4 性能取证实录（2026-07-15,RTX 4070 Ti / driver 620.02 / CUDA 13.2,行为逐位确定同配置必现;registry/deferred.json RD-027 reason 原文）:apps/ruridrop 生产档 offline（1280×720 / N=131072 / 64³ 网格）`pt_render` 在特定样本/弹射深度组合下挂起——720p/8spp:PT_BOUNCES≤2 秒级完成、=3 挂起 >300s、=4 挂起 >15min;PT_BOUNCES=2:8spp/32spp 秒级、256spp 挂起 >590s 零帧;同数据同 cell 表 rt_primary 600 帧与 160×120 冒烟档 bounces=4 全绿;kernel 源内全部循环编译期有界,源语义不构成死循环。处置当时闭环为:登记 RD-027 + params.rx 生产档锁 32spp/2 弹射切片 + `ms1.bench.uc07_offline_frame_s` 以切片档回填,归因调查归后续里程碑。

G3 立项时 RD-027 被判为「一切图形特性扩张」的可靠性前置,G3.1 以本判别 spike 开局(纯取证不占 RFC,G2.2 DXIL spike 先例)。owner 2026-07-18 裁定 **归因落地即开闸**(G3_CONTRACT §7 ②):G-G3-1 以归因证据(evidence JSON 过 schema + 本取证报告)合入 main 为开闸点,五特性面 RFC 自此可合;处置尾项(修复或上游备包+护栏)不阻塞开闸但属 close-out 前置。另,本 spike 时点工具链已发生 13.2→13.3 漂移(ptxas 13.3 V13.3.33 在位),按 G-G3-1 ① 须先重立毒径基线,见 §3 E0a。

## 2. 取证方法

### 2.1 判别矩阵设计

四层嫌疑与实验映射(嫌疑 = 谁把「有界源语义」变成了「不终止执行」):

| 嫌疑层 | 假说 | 判别实验 |
|---|---|---|
| ① rurixc IR / LLVM NVPTX | 源或 PTX 编码了无限循环/语义错 | E5 静态审计(PTX 循环拓扑/定值链)+ E1 ptxas -O0 对同一 PTX 的行为对照 + E7b 源循环硬封顶 |
| ② 应用/数据 | OOB/粒子表损坏诱发数据性不终止 | E7a compute-sanitizer memcheck 前置排除 + MS1.4 同数据他 kernel 全绿实录 + E4 SUBSTEPS=0 数据依赖探针 |
| ③ ptxas(AOT 汇编器) | ptxas 优化重构引入缺陷 | E1 优化档扫描(-O0/1/2/3)+ E5 round-2 O0↔O1 SASS 差量 |
| ④ 驱动 JIT | 驱动内置编译器独有缺陷 | E0b 同源构型双装载路(cubin AOT vs PTX JIT)对照 |

实验清单:**E0a** 基线复现(13.3 工具链下重立)+ 单 artifact 核验;**E0b** 双装载路判别;**E1** ptxas 优化档扫描(含 -O0 完整生产档验证);**E4** 删减阶梯 + d6 细分(d6a~d6d)+ SUBSTEPS=0 数据依赖探针;**E5** SASS 静态分析(round-1 静态审计 + round-2 O0↔O1 差量;纯 CPU,无 GPU 运行);**E7a** compute-sanitizer memcheck;**E7b** 全源循环硬封顶。探针脚本 = `spike/rd027-pt-poison/run_e{0a,0b,1,4,7a,7b}.py` + `spike_common.py` + `make_evidence.py`。

### 2.2 安全纪律实录

- **proc_guard 全覆盖**:全部 GPU 运行经 `bench/proc_guard` guarded_run(常规 120s 判定线 / memcheck 300s / 完整生产档 600s);超时 = 杀进程树 + exit 124 诚实红(NOT skip)+ 被锁 exe 移入 `build/quarantine/` 隔离区。全程零裸 subprocess(R-606)。本次隔离 exe 共 **13** 个。
- **金丝雀门**:每次挂起判定后跑已知绿基准(ctrl 档,秒级)+ nvidia-smi 响应性检查,复绿方采信后续实验——本次 **14 过 / 0 失败**,全部记录 campaign.jsonl(`canary_verdict` 行)。
- **错峰**:实验窗与 CI runner/nightly 错峰;TDR/系统态零改动如实记录(tdr_delay/tdr_level = not_set(os_default),HAGS enabled)。
- **ptxas 输入恒 ASCII 路径**;SASS 分析(E5)纯 CPU 不占 GPU 窗。

### 2.3 首轮 E7a 作废与重测(诚实记录)

首轮 E7a 对 poison_b3 的判定 **completes_under_sanitizer(heisenbug_signal)无效**:该 exe 已被 E0a 超时处置收入隔离区,compute-sanitizer 实际报 `Target application doesn't exist or is not a valid executable`(exit 1),并非「在 sanitizer 下跑完」。campaign.jsonl 以 `correction` 行留痕作废(不擦除原始记录),重建 exe 后重测,重测判定 = **still_hangs_under_sanitizer**(300s 超时,见 §3)。作废与重测两轮记录均保留在 evidence JSON experiments 数组内。

## 3. 实验事实矩阵

环境(evidence JSON `environment`):NVIDIA GeForce RTX 4070 Ti(CC 8.9)/ driver 620.02 / CUDA 驱动 13.2 / ptxas 13.3(V13.3.33)/ Windows 10.0.28120 / TDR 系统默认 / HAGS on。

**单 artifact 核验**:五个配置变体(ctrl_b2 / ctrl_32 / poison_b3 / poison_b4 / poison_256)构建产物 `distinct_ptx_digests = 1`,PTX 逐字节同一(sha256 `85d597dd…`),**挂/不挂纯由运行期实参 + 数据决定**。JIT 腿(无 ptxas 保守档)单独一份 PTX(sha256 `8d9c3c05…`,.version 7.8 vs AOT 腿 8.8,diff 仅版本头 4 行、语义同码;腿内同样单 artifact,判别有效性不受影响)。

逐实验实测(全部取自 evidence JSON experiments / campaign.jsonl;挂起判定线 120s,E7a 300s,完整档 600s):

| 实验 | run | 装载路 | 判定 | exit | wall(s) |
|---|---|---|---|---|---|
| E0a | ctrl_b2(8spp/b2) | ptxas AOT 默认(O3) | completed | 0 | 0.62 |
| E0a | ctrl_32(32spp/b2) | ptxas AOT 默认(O3) | completed | 0 | 0.94 |
| E0a | poison_b3(8spp/b3) | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.32 |
| E0a | poison_b4(8spp/b4) | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.32 |
| E0a | poison_256(256spp/b2) | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.32 |
| E0b | jit_ctrl_b2 | 驱动 JIT(PTX 7.8) | completed | 0 | 0.81 |
| E0b | jit_poison_b3 | 驱动 JIT(PTX 7.8) | **hang_timeout** | 124 | 120.21 |
| E0b | jit_poison_b4 | 驱动 JIT(PTX 7.8) | **hang_timeout** | 124 | 120.36 |
| E0b | jit_poison_256 | 驱动 JIT(PTX 7.8) | **hang_timeout** | 124 | 120.32 |
| E7a(首轮,作废) | memcheck_ctrl_b2 | O3 + compute-sanitizer | completed(0 errors) | 0 | — |
| E7a(首轮,作废) | memcheck_poison_b3 | O3 + compute-sanitizer | error(exe 被隔离区收走) | 1 | — |
| E7a(重测) | memcheck_ctrl_b2 | O3 + compute-sanitizer | completed(**ERROR SUMMARY: 0 errors**) | 0 | — |
| E7a(重测) | memcheck_poison_b3 | O3 + compute-sanitizer | **hang_timeout**(300s) | 124 | — |
| E1 | aotO0_ctrl_b2 | ptxas **-O0** | completed | 0 | 0.62 |
| E1 | aotO0_poison_b3 | ptxas **-O0** | **completed** | 0 | **0.66** |
| E1 | aotO1_ctrl_b2 | ptxas -O1 | completed | 0 | 0.40 |
| E1 | aotO1_poison_b3 | ptxas -O1 | **hang_timeout** | 124 | 120.24 |
| E1 | aotO2_ctrl_b2 | ptxas -O2 | completed | 0 | 0.55 |
| E1 | aotO2_poison_b3 | ptxas -O2 | **hang_timeout** | 124 | 120.28 |
| E1 | aotO0_full_prod(**256spp/batch32/b4**) | ptxas **-O0** | **completed** | 0 | **9.49** |
| E7b | e7b_cap_all(全源循环硬封顶) | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.36 |
| E4 | d1 / d2 / d3 / d4 | ptxas AOT 默认(O3) | completed ×4 | 0 | 0.41 / 0.41 / 0.37 / 0.46 |
| E4 | d5 | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.25 |
| E4 | d6 | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.21 |
| E4 | d7 | ptxas AOT 默认(O3) | completed | 0 | 0.41 |
| E4 | d6_nosim(**SUBSTEPS=0**) | ptxas AOT 默认(O3) | **completed** | 0 | 0.54 |
| E4 | d6a | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.31 |
| E4 | d6b | ptxas AOT 默认(O3) | completed | 0 | 0.38 |
| E4 | d6c | ptxas AOT 默认(O3) | **hang_timeout** | 124 | 120.20 |
| E4 | d6d | ptxas AOT 默认(O3) | completed | 0 | 0.51 |

注:
- **E0a 基线重立成立**(reproduced = true):13.3 工具链下 2 绿 3 挂与 MS1.4 实录同构 → 「13.2→13.3 漂移已把问题修掉」假说排除。E7a 首轮两行 wall 未记录(evidence JSON 以 -1.0 占位),判定以 exit/timeout 为准。
- **E1 -O3 档即 E0a 默认档**(campaign E1 header 注),不重复跑;-O0 完整生产档(256spp/4 弹射)9.49s/帧完成——该档在默认档下此前 >15min 挂起不可测。
- **E4 删减阶梯非单调**:d1(删 NEE)~d4(删地面/天空)绿、d5(内环单粒子)/d6(删 shadow_walk)挂、d7 绿;d6 细分再现非单调(d6a 删 bounce-0 光源块仍挂 / d6b 绿 / d6c 仍挂 / d6d 绿)。SUBSTEPS=0(跳过 sim,粒子保持初始排布)不触发 → **数据依赖:需 4 子步 sim 后粒子分布**。
- **挂起签名**(evidence JSON gpu_hang_signature + campaign gpu_during 采样):util 100% / ~63.19W 低功耗 / SM 满频 2745MHz,120s 窗口内持续平稳——兼容「少数 warp 栅栏死等,非计算推进」;round-1 曾记 ~54W,以本轮 63W 为准。

## 4. SASS 层证据(引 `build/spike-rd027/e5/analysis.md`,round-1 + round-2)

> SASS 分析工件(`aot_O0/O1/O2/O3.sass`、`e7b_O1/O3.sass`、analysis.md 本体)驻 `build/spike-rd027/e5/`,**不入库**(工作区工件;上游备包阶段按需固化)。本节只引结论与行号,不复制大段 SASS。

- **Round-1 静态审计(嫌疑层①的静态排除支柱)**:PTX(`ctrl_b2.ptx`,3838 行,ISA 8.8)全文件零 spill;全部循环(spp/弹射/DDA/cell/10×sqrt/2×拒绝采样/2×阴影 DDA 克隆,约 20 个)均单 header 自然循环、可归约、计数定值链唯一(不存在分支两臂不同步更新计数器的构型);多级 break 全部折叠汇入外层 latch。**PTX 层结论:语义上不存在可无限执行的路径**(analysis §1.1)。
- **Round-2 档位地形(证据模式 F)**:`cmp aot_O2.sass aot_O3.sass` **逐字节相同**(O2≡O3);pt_render 段本地 `@!P0 CALL.REL.NOINC` latch 出口计数 = **O0:0 / O1:4 / O3:4**;寄存器 SHI_REGISTERS = O0 141 / O1 77 / O2=O3 77。**行为分界(O0 绿 | O1/O2/O3 挂)与协议分界(无 CALL 出口 | 有)在 O0→O1 严格重合**(analysis §4.1)。
- **挂档协议(证据模式 A)**:O1 起同一循环并存两套出口——空间出口走 `BREAK + BRA → BSYNC` 正规 reconvergence 记账,计数出口被重构为 `@!P0 CALL.REL.NOINC` **无 BREAK、无记账的谓词跳转**,共 4 处(O1 行 2945 主 DDA / 3850 阴影 DDA-1 / 5306 阴影 DDA-2 / 5625 spp;O3 对应 3664/4576/6049/6377,逐环 1:1 同构;analysis §4.2 表)。
- **绿档对照(证据模式 B)**:O0 同一主 DDA 环(aot_O0.sass 6534–6540 / 6922–6926 / 6961)全部出口对称、全部 BREAK 记账、普通 BRA 汇入 BSYNC——**与挂档的唯一语义差 = 出口是否经 BREAK 记账/是否用 CALL 边**(analysis §4.3)。
- **竞态窗口(证据模式 C)与 uniform 反例(模式 D)**:site2/3 的 CALL 出口继续路在 2~3 条指令内即 `BSSY B5`(O1 3845–3855 / 5301–5310),而被弃环体每迭代再臂 B7/B6/B5/B8——非 uniform 触发时同 warp 两半对同一物理 barrier id 并发再臂;barrier id 分配 **9/9 全满零裕度**。对照 site4(spp)计数走 uniform 数据路(O1 5617–5626)= ptxas 有能力做结构性安全出口,site1–3 却依赖静态 uniform 假设。
- **长寿命谓词(证据模式 E)**:sqrt 环回边谓词 O1 起改为头定值(O1 2792–2875),跨 5 个发散 CALL 区存活;O0 为尾定值 3 条指令内闭合(5374–5378)。
- **机理 M1′(analysis §5/§8)**:`CALL.REL.NOINC` 是对 reconvergence 记账不可见的谓词控制流边,合法性悬于「出口谓词对残余 barrier 参与者一致」这一 ptxas 静态断言,硬件无兜底;任一动态事件使其非 uniform 触发即把 warp 无记账切分 → 同 id 并发再臂 → barrier 参与者掩码破坏 → 任意下游 BSYNC 永等。六组事实在 M1′ 下全部闭合:O0 绿(无无记账边)/ O1/O2/O3 挂(协议自 O1 引入,O2≡O3)/ JIT 同挂(驱动 JIT 施同类变换)/ 封顶不解(E7b 封顶版 SASS 仍保留全部 4 处协议,e7b_O1.sass 2969/3891/5352/5670)/ 触发随实参放大(协议事件数 ∝ spp×bounces×steps)/ 挂起签名(warp 停驻 BSYNC 不发射:util 100%、低功耗、满频)。

## 5. 归因裁决(`spike/rd027-pt-poison/attribution.json`,与 evidence JSON attribution 节一致)

**verdict = `nvidia_optimizing_backends`,confidence = high**:挂起由 NVIDIA 优化后段(ptxas -O1 及以上,驱动 JIT 内置编译器同类变换)的 latch 出口协议重构引入,机理 = M1′(无记账 CALL 出口 → barrier 掩码破坏 → BSYNC 死等)。按四层判别矩阵:**定罪层③+④合并**(双装载路一致挂 = 两个 NVIDIA 优化后段同酿,非单一后段);**排除层①**(rurixc IR / LLVM NVPTX 语义错);**强削弱层②**(应用/数据缺陷)。

key_evidence(六条,逐条锚定本报告支撑节):

1. E1:ptxas -O0 完成(毒径 0.7s;完整 256spp/4 档 9.5s/帧)而 -O1/-O2/-O3 全挂;O2 与 O3 SASS 逐字节相同,绿/挂分界精确落在 O0→O1(§3 E1 + §4 模式 F)。
2. E0b:同源构型双装载路(ptxas AOT cubin PTX8.8 / 驱动 620.02 JIT PTX7.8)全部一致挂起——两个 NVIDIA 优化后段同酿,单一后段假说排除(§3 E0b)。
3. E5 round-2 SASS 差量:O1 即引入全部 4 处 `@!P0 CALL.REL.NOINC` latch 出口(无 reconvergence 记账的谓词边,O0 同环为 `@P0 BREAK+BRA→BSYNC` 正规记账);同 barrier-id 并发再臂窗口 9/9 零裕度——掩码一坏 BSYNC 永等(机理 M1′);E7b 封顶版 SASS 仍保留全部 4 处协议,与「封顶不解」自洽(§4 模式 A/B/C/F)。
4. E7b:全源循环硬封顶(DDA≤1000 / cell≤4096 / bounce≤8 / spp≤64)后仍挂——自旋不在源循环迭代层,SASS 协议层死锁(§3 E7b)。
5. 单 artifact 事实:五配置变体 PTX 逐字节同一,挂/不挂纯由运行期实参+数据决定;挂起签名 util 100%/~63W/满频 2745MHz = 少数 warp 栅栏死等非计算(§3)。
6. E4 删减阶梯非单调(d1~d4 绿 / d5,d6 挂 / d7 绿)= 代码形态敏感的优化器陷阱签名,非单一源语义构造(§3 E4)。

excluded(五条):

1. **rurixc/LLVM 语义错**(源或 PTX 编码了无限循环):排除——ptxas -O0 对同一 PTX 正确终止(0.7s/9.5s),refcpu 逐位重放有限终止,E5 round-1 静态分析 20 循环全可归约、计数定值链干净。
2. **应用/数据缺陷**(OOB/表损坏):强削弱——对照档 compute-sanitizer memcheck 0 errors;同数据同表在 rt_primary 600 帧与冒烟档 bounces=4 全绿(MS1.4 实录)。
3. **ptxas 13.2→13.3 工具链漂移已修**:排除——13.3 下全复现(E0a)。
4. **单独 ptxas 或单独驱动 JIT**:排除——双装载路一致挂(E0b)。
5. **源循环跑飞**(计数回绕类 M2 主嫌):削弱至次嫌——全封顶不解(E7b);仅 10 个 sqrt 环(-5 步长等值出口)不在封顶集,列本节诚实限界②。

触发构型(trigger_shape 摘要):合法 PTX(LLVM NVPTX 后端形态,rurixc device MIR→LLVM IR→clang 22.1.x)的多层嵌套发散循环+多级 break 构型,经 ptxas -O1+(驱动 JIT 同变换)的 latch→CALL.REL.NOINC 出口协议重构后,在特定运行期实参(弹射≥3 / 256spp)与 sim 后粒子分布下 barrier 掩码破坏→BSYNC 死等;数据依赖(SUBSTEPS=0 初始排布不触发)。

最小化复现(minimized_repro,status = done):最小挂起变体 = E4 **d6a**(`spike/rd027-pt-poison/mrp/render_pt_mrp_d6a.rx`,attribution 记 276 行,全循环有界);触发构型 = spp×弹射×DDA 三层嵌套 + 多级 break + 单粒子 sphere_t + RNG 种子 CALL 链 + 方向弹射改写(d6b/d6d 单删各自溶解 = 参与形态;d6a/d6c 单删仍挂 = 光源块与法线着色非必要);阶梯与细分全记录 campaign.jsonl。

**诚实限界**(不伪造完备性,G-G3-1 ④):

- ① **E6(独立前端对照)与 cuda-gdb warp 停驻取证未做**:未用 nvcc/clang-CUDA 生成语义等价 kernel 对照,也未在挂起态 attach 采样 warp PC(analysis §5 已给出 O1 预测停驻点表)。二者列为**上游备包阶段可选补强**,非开闸前置。因此**不能完全排除** LLVM-NVPTX 产出的 PTX 在某条未验证契约(未文档化约束)上不合规的残余可能;但强反证在案:同一 PTX 经 ptxas -O0 正确终止(0.66s/9.49s)、ptxas 各优化档均正常汇编通过(AOT 构建含 rurixc ptxas 干门,RXS-0073)、E5 静态审计全循环可归约计数链干净、refcpu 逐位重放有限终止——「PTX 本身语义错」与这四组事实不相容,「PTX 合法但触发后段缺陷」为当前证据下唯一自洽解释。
- ② **M2@sqrt 残余次嫌留档**:10 个 sqrt 环(常量 30、步长 −5、等值出口、O1 起头定值长寿命谓词)不在 E7b 封顶集,其「计数被动态破坏后回绕自旋」变体在 E7b 下技术上未被排除(analysis §4.5)。但 O0 的 sqrt 环同为等值出口而不挂 → 损坏源仍须是 O1 引入的变换,收敛回同一根因面(NVIDIA 优化后段),**不改变 verdict**;一次 cuda-gdb PC 采样即可终裁(PC 停 BSYNC → M1′;PC 在 sqrt 体 + 计数寄存器回绕大数 → M2@sqrt)。

按 G-G3-1 ②,本裁决构成「至少一个排除或定罪结论」的兑现:一个定罪(NVIDIA 优化后段)+ 三项排除 + 一项强削弱,证据链四要件(双装载路对照 / 优化档扫描 / 最小化触发构造 / memcheck 前置排除)齐备。

## 6. 护栏与处置指向

- **护栏实测(workaround,verified = true)**:`ptxas -O0` pin(AOT cubin 腿,wrapper 注入):毒径 8spp/b3 **0.66s** 完成(E1 aotO0_poison_b3;attribution 摘要记 0.7s 量级),完整生产档 256spp/4 弹射/720p **9.49s/帧** 完成(E1 aotO0_full_prod,600s 判定线内;该档此前 >15min 挂起不可测)。
- **护栏限界**:驱动 JIT fallback 腿**无对应优化档开关**——护栏必须保证 cubin AOT 路径生效(ptxas 在位),JIT 腿构型上不受本护栏保护,如实留痕。
- **处置尾项**(G-G3-1 ③(b) 路线,归因落驱动/工具侧):NVIDIA 侧 `evidence/upstream-reports/` **DRAFT 备包(do NOT file,owner 复核门,agent 不对外提报)**+ ptxas -O0 pin 护栏落地留痕;RD-027 **诚实存续**(不 force-close)。二者不阻塞开闸,但属 **G3 close-out 前置**。自包含合成数据版 MRP 再最小化归上游备包阶段按需处理(mrp/README)。
- **升档回填**:RD-027 backfill_condition **原文维持、不伪造**——「修复后把 ms1.bench.uc07_offline_frame_s 的 32spp+2 弹射可测切片升回完整生产档(256spp/4 弹射)重测回填(ci/uc07_bench.py offline_frame 切片补丁摘除)」。归因落上游侧 = 「修复」未发生,本报告**不预支回填**;-O0 pin 使完整档可测(9.49s/帧)仅证明护栏路线下升档具备实测可行性,回填动作本体(params.rx 升档 / bench 补丁摘除 / 预算重测)归处置 PR 按各自修订纪律留痕兑现。

## 7. 复现清单

MRP 全文见 [spike/rd027-pt-poison/mrp/README.md](../spike/rd027-pt-poison/mrp/README.md),逐字命令:

1. `cargo build -p rurixc -p rx`
2. 取 `apps/ruridrop/src` 整目录副本,以 `spike/rd027-pt-poison/mrp/render_pt_mrp_d6a.rx` 替换其中 `render_pt.rx`;`params.rx` 打毒径参数:`SPP 32→8 / SPP_BATCH 32→8 / PT_BOUNCES 2→3 / REND_FRAMES 8→1`(切片值为 STUB(RD-027) 现值)。
3. `target\debug\rx.exe build <副本>/offline.rx -o poison.exe`
4. 运行(**必须带看门狗**):`py -3 bench/proc_guard.py --timeout 120 -- poison.exe`
   - 默认构建(ptxas -O3 AOT cubin):**挂起**(util 100%/~63W/满频,BSYNC 死等)
   - `RURIXC_PTXAS` 失效强制 PTX JIT(驱动 620.02):**挂起**
   - ptxas 注入 `-O0`(cubin AOT):**0.4~0.7s 正确完成**
   - `PT_BOUNCES=2` 对照:任意档秒级完成
5. 全量 campaign 重放:依次 `py -3 spike/rd027-pt-poison/run_e0a.py / run_e0b.py / run_e7a.py / run_e1.py / run_e7b.py / run_e4.py`(逐 run 追加 `build/spike-rd027/campaign.jsonl`),`py -3 spike/rd027-pt-poison/make_evidence.py` 汇总产 evidence JSON 并过 `ci/check_schemas.py`。
6. SASS 差量再生(纯 CPU,ptxas/nvdisasm = CUDA v13.3;analysis.md §9 原文):
   ```
   ptxas -arch=sm_89 -O1 bin/ctrl_b2.ptx     -o e5/aot_O1.cubin   # + nvdisasm -c → e5/aot_O1.sass
   ptxas -arch=sm_89 -O2 bin/ctrl_b2.ptx     -o e5/aot_O2.cubin   # + nvdisasm -c → e5/aot_O2.sass(≡ aot_O3.sass)
   ptxas -arch=sm_89 -O1 bin/e7b_cap_all.ptx -o e5/e7b_O1.cubin   # + nvdisasm -c → e5/e7b_O1.sass
   ptxas -arch=sm_89 -O3 bin/e7b_cap_all.ptx -o e5/e7b_O3.cubin   # + nvdisasm -c → e5/e7b_O3.sass
   ```

## 8. G-G3-1 门判据逐项对照

| G-G3-1 判据 | 本报告兑现 |
|---|---|
| ① 当前工具链下毒径基线判定留档(13.2→13.3 漂移须先重立;每 GPU 运行经 bench/proc_guard 硬超时,超时=诚实红 124,零裸 launch,R-606) | §3 E0a:13.3 工具链下 5/5 复现(2 绿 3 挂,reproduced=true),漂移假说排除;§2.2:proc_guard 全覆盖、超时一律 exit 124 诚实红、13 exe 隔离、零裸 launch |
| ② 四层判别矩阵至少给出一个排除或定罪结论;证据链含双装载路对照 + 优化档扫描 + 最小化触发构造 + compute-sanitizer 前置排除 | §5:定罪 NVIDIA 优化后段(层③④)+ 排除层①/漂移/单一后段 + 强削弱层②;双装载路 = §3 E0b;优化档扫描 = §3 E1;最小化构造 = §5 minimized_repro(E4 d6a)+ §7;memcheck = §3 E7a(对照档 0 errors;「OOB→应用缺陷改道」未发生,毒径在 sanitizer 下仍挂) |
| ③ 开闸判定 = ② 归因证据(evidence JSON 过 check_schemas + 取证报告)合入 main 即成立,五面 RFC PR 自此可合;处置尾项 (a) 修复回填 或 (b) 上游 DRAFT 备包 + 护栏留痕,不阻塞开闸但属 close-out 前置 | 机器证据 = 表头行(evidence JSON 已过 schema);本文件即取证报告本体——**二者合入 main 即开闸成立**;处置走 (b) 路线(§6):上游 DRAFT 备包 + ptxas -O0 pin 护栏,RD-027 诚实存续,close-out 前置 |
| ④ 不伪造归因、不以超时无果充『驱动缺陷』;零 nightly/CI 新增毒径步骤;挂起后金丝雀门记录在案方可采信后续实验 | §5:归因以 O0/O1 行为分界 + SASS 协议差量(§4)+ 双装载路一致性为据,**非以超时无果为据**;两条诚实限界明列(E6/cuda-gdb 未做、M2@sqrt 残余);§2.3 首轮 E7a 作废 correction 留痕;金丝雀 14 过/0 失败全记录(§2.2);本 spike 零 nightly/CI 步骤新增,探针隔离 spike/ 可弃 |

> 本 spike 纯取证:不落 codegen / 不创建 spec 条款 / 不造错误码 / 不入 golden / 不登 spike_gating(语义错位,G2.2 先例)。归因后续动线:RD-027 history 已追加 G3.1 归因行(registry/deferred.json RD-027 history 2026-07-18 行,同 PR 落),处置 PR(上游备包 + 护栏)另行,五特性面 RFC 自本报告合入 main 起解锁。
