# MS1 执行计划 — 子里程碑分解

> 所属契约:[MS1_CONTRACT.md](MS1_CONTRACT.md)
> 版本:v1.0(2026-07-14)
> 粒度依据:11 §7(小里程碑两级结构);本计划是工作分解,验收以契约 §4 为准,本文不重定义成功。
> agent 裁决(契约 §7 v1.0):MS1.1~MS1.5 严格串行(MS1.1 内两 RFC 可并行起草、串行合入);MS1.3 的纯 kernel .rx 语料可在 MS1.2 期间预写(不依赖宿主编排面);条款先行 + 快照重 bless 同 PR(两次:MS1.2 / MS1.2b)。
> **定位口径**:使命判据(01 §6 第三层)第一期操作化落地;最严主语言判据(宿主编排也须 .rx)= 关键路径在 MS1.2 语言特性,应用是其验收载体。

---

## 0. 总览与依赖

```mermaid
flowchart LR
    ms11[MS1.1 双 Full RFC 0009/0010] --> ms12[MS1.2 single-source 宿主编排 stdlib + 步骤52]
    ms12 --> ms12b[MS1.2b present typestate + 落盘桥]
    ms12b --> ms13[MS1.3 UC-07 应用 ruridrop + 离线 golden 步骤53]
    ms13 --> ms14[MS1.4 present 取证 + 性能回填]
    ms14 --> ms15[MS1.5 close-out]
```

| 子里程碑 | 时长(估) | 交付物映射 | 阻塞关系 / gating |
|---|---|---|---|
| MS1.1 | ~2–3 天 | D-MS1-1(RFC-0009 + RFC-0010) | **MS1 入口,先做**;两 RFC 并行起草、串行合入(rfcs/README 台账冲突);Full RFC 档(硬规则 5 / 使命级先例) |
| MS1.2 | ~1–2 周 | D-MS1-2(编排 stdlib + 步骤 52) | 依赖 RFC-0009 Approved;条款先行 RXS-0189~0196;快照重 bless 184→192 同 PR;**关键路径** |
| MS1.2b | ~2–3 天 | D-MS1-3(present typestate + 落盘桥) | 依赖 MS1.2(cabi 基座);条款 RXS-0197~0199;快照重 bless 192→195 同 PR;uc03 回归网守卫(scope()/brand 系 0-byte) |
| MS1.3 | ~1 周 | D-MS1-4(ruridrop + 步骤 53) | 依赖 RFC-0010 Approved + MS1.2/MS1.2b 全面;应用全 .rx 零 .rs;golden 三层 + 数据流红绿 |
| MS1.4 | ~2–3 天 | D-MS1-5(present evidence + ms1.bench.*) | 依赖 MS1.3;本机交互桌面真跑;性能 entries 与 evaluator 同 PR 回填 measured_local |
| MS1.5 | ~1 天 | close-out 终审 | 依赖 MS1.4;契约翻 closed + 基准 v1-closed→ms1-closed + ms1-closed tag + RD/SG 处置(agent 自主签署) |

时长为 `estimated`,仅作排程参考,不构成验收承诺。子里程碑不另立 contract(单 MS1 阶段契约,契约 §7 v1.0)。

## 1. MS1.1 — 双 Full RFC(D-MS1-1,入口先做)

| # | 任务 | 验证方式 / gating |
|---|---|---|
| 1 | rfcs/0009-host-gpu-orchestration.md:std::gpu 首期收敛子集(Context/Stream/Buffer/PinnedBuffer/launch/同步)/ affine 类型面复用 RXS-0130~0134 语义 / rurix-rt-cabi C ABI 绑定(rxrt_*,staticlib,RXS-0125 口径)/ launch marshalling(实参子集 Buffer + {i32,u32,f32,usize})/ 同源 PTX 嵌入 + 装载协商复用(RXS-0150/0151/0076)/ 运行期失败 = 确定性诊断 + 终止(无 UB)/ 前端机械面(extern 保名 + #[link] + mod name;)/ present·落盘面(RXS-0197~0199 承载)/ §8 不做(AsyncBuffer/Event 跨线程/多 stream 重叠 → RD-026)/ §9 裁决清单(Q-Surface/Q-Link/Q-Marshal/Q-Embed/Q-Affine/Q-Err/Q-Present) | 失败测试先行声明:ci/host_orch_smoke.py 与 std::gpu lowering 在提案时点 main 上不存在 = RED;RFC 合入先于实现(10 §3) |
| 2 | rfcs/0010-uc07-sim-renderer.md:ruridrop 应用形态(GPU SPH 溃坝 + 二合一渲染:DDA 路径追踪离线 / 光线投射实时)/ **主语言判据操作化**(应用包零 .rs + 宿主与 kernel 同包 + rx build 单 EXE + 语言基础设施白名单)/ golden 三层协议(确定性硬门 + CPU 参考容差硬门 + blessed 哈希软门)/ 确定性设计约束(bit-split 排序 atomics-free、固定序累加、禁 order-dependent 浮点原子)/ 防降级硬门措辞(镜像 G-G2-4)/ §8 不做(交互模式 → RD-027;BVH/多材质/纹理对象 → RD-028)/ §9 裁决清单(Q-Criterion/Q-AppScope/Q-Determinism/Q-Present/Q-Perf/Q-ProdGrade/Q-Loc/Q-UCNum) | 同上;RFC-0010 引用 RFC-0009 §4 设计,合入序 0009 → 0010 |
| 3 | rfcs/README.md §5 台账两行(下一未用滚动至 RFC-0011;Mini-RFC 台账不动,MR-0010 留续) | 台账一致 |

**出口判据**:两 RFC Approved 合入 main(agent 自主判档/合入,D-406 v2.0),台账一致。

## 2. MS1.2 — single-source 宿主 GPU 编排 stdlib(D-MS1-2,关键路径)

单 PR 栈式 commit(条款在前;条款/实现/重 bless/步骤 52 不可分 PR,步骤 49 硬红):

| # | 任务 | 验证方式 / gating |
|---|---|---|
| 1 | **spec 条款先行**:新建 spec/host_orchestration.md RXS-0189~0196(FLS 体例,严禁 UB 节)——0189 std::gpu 类型面与 affine 语义(含宿主 API 着色合法性)/ 0190 方法签名与元素类型推断 / 0191 launch lowering 与实参借用+marshalling / 0192 single-source 嵌入与装载协商 / 0193 运行期错误与 poisoned / 0194 rxrt C ABI 边界(🔒 FFI)/ 0195 extern "C" 保名 + #[link] / 0196 out-of-line 模块;spec/README §4/§5 同步(修订数据行避「版本」子串) | spec 档位 guardrail;每条 ≥1 `//@ spec:` 锚定同 PR(trace_matrix --check 全锚定) |
| 2 | 前端机械:mir_build extern 无 body fn 符号保名(不 mangle)+ driver #[link(name)] 接线(定位失败 RX7022)+ parser/driver `mod name;` out-of-line 装配(缺失/循环 RX1005) | conformance/UI reject 语料 + cargo test -p rurixc |
| 3 | src/rurix-rt-cabi(staticlib):rxrt_*(ctx/stream/buf/pinned/launch)包 rurix-rt pipeline ownership 系 + fatbin 装载协商;u64 句柄表 + 销毁纪律(free 前 sync,镜像 D-231);unsafe 逐处 // SAFETY: + unsafe-audit U25 | cargo test -p rurix-rt-cabi(host-only 绿) |
| 4 | rurixc lowering:resolve 新 lang item(PinnedBuffer)+ typeck 编译器已知签名分支(Context::create/ctx.stream/alloc/alloc_pinned/sync/buf.upload/download/len/pinned.get/set/stream.launch/sync;元素不可推断 RX2010;宿主 API 进 device 上下文 RX3015)+ mir_build gpu 方法/launch → CallTarget 字面符号 rxrt_*(marshalling 子集外 RX6024)+ driver:device 段产 PTX/cubin 嵌入(@__rx_gpu_artifacts,失败 RX6025)+ link 段追加 rurix_rt_cabi.lib(定位失败 RX7021,缺库回退 cargo build 编排) | conformance/host_orch accept+reject + `rx build` 单源 saxpy 端到端(本机 device 真跑数值对照) |
| 5 | 新错误码 en+zh 成对(error_codes.json 追加,bilingual 门复绿) | py -3 ci/bilingual_coverage.py |
| 6 | **stable 快照重 bless 184→192(同 PR)** + bless_log 追加(数据行忌「日期」子串);registry/deferred.json 追加 RD-026(std::gpu 首期子集外面) | 步骤 49 ci/edition_smoke.py 复绿 |
| 7 | CI 步骤 52:新建 ci/host_orch_smoke.py(host 段总跑:编译+链接+reject 拦截;device 段 runner 真跑:数值对照;红绿:篡改嵌入 PTX → 装载拒 / 桩化 → 数值红 / 复原绿)+ evidence schema + ms1.counter.host_orch_single_source 登记 + budget_eval eval_counter 分支 + pr-smoke.yml 接线 + trace 矩阵再生 | 真实红绿 + run URL 归档契约 §8 |

**出口判据**:单源 .rx(宿主编排 + kernel)→ rx build 单 EXE → RTX 4070 Ti 真跑数值对照通过;步骤 52 红绿闭合;trace/快照/双语/guardrails 全绿。

## 3. MS1.2b — present typestate + 宿主图像落盘桥(D-MS1-3)

| # | 任务 | 验证方式 / gating |
|---|---|---|
| 1 | spec RXS-0197(present 宿主 typestate,镜像 RXS-0142,D-130 边界)/ 0198(backbuffer 借用与 blit 契约,对齐 RXS-0143)/ 0199(宿主图像落盘桥,RXS-0114~0117 语义) | 同 MS1.2 条款纪律 |
| 2 | rurix-rt interop.rs 重构出 OwnedPresentSession(非闭包持有形态;既有 scope()/brand API 0-byte,uc03 零漂移)+ cabi 增 rxp_*(create/wait/backbuffer/signal/pump/present/destroy,fence 偶/奇协议单一事实源留 interop.rs)+ rxio_write_ppm(桥 image-io) | cargo test -p rurix-rt / -p uc03-demo(回归网)+ conformance/host_orch present/imageio 语料锚定 |
| 3 | .rx stdlib 面:Present/Ready/Acquired/Presentable affine 消费式(错序 = 既有 RX4xxx move 违例) | UI reject 语料(错序编译期拦截) |
| 4 | **stable 快照重 bless 192→195(同 PR)** + bless_log 追加 | 步骤 49 复绿 |

**出口判据**:.rx 帧循环语料编译期 typestate 拦截齐;落盘桥 host 真跑出 PPM;uc03 既有测试零漂移。

## 4. MS1.3 — UC-07 应用 ruridrop + 离线 golden 门(D-MS1-4)

| # | 任务 | 验证方式 / gating |
|---|---|---|
| 1 | apps/ruridrop 全 .rx(rurix.toml + src/):sim 10 kernel(cell_key/split_flag/scan_block/scan_sums/split_scatter/reorder/cell_bounds/density/forces/integrate,块 256,bit-split 基数排序 atomics-free)+ 渲染核(rx_sqrt/rng/DDA 球体求交/着色 device fn)+ pt_render/pt_finalize(离线)+ rt_primary(实时)+ 三入口 offline/realtime/refcpu(mod 文件组织) | rx build 三入口出 EXE;冒烟档(N=4096,160×120,8spp,2 帧)device 真跑 |
| 2 | tests/uc07/golden_manifest(逐帧 SHA-256)+ bless_log.md 首次 bless 留痕 | 同机两跑逐字节一致后 bless |
| 3 | CI 步骤 53:ci/uc07_offline_golden_smoke.py(前置零 .rs 审计 + 三层 golden + 数据流红绿:篡改物理常数重编 → digest 红 → 复原绿)+ evidence schema + ms1.counter.uc07_offline_golden_frames 登记 + evaluator 分支 + pr-smoke.yml + trace 再生 | 真实红绿 + run URL 归档 |

**出口判据**:步骤 53 全绿(硬门①②+软门③);应用零 .rs 审计过;RD-007 接通评估留痕(用没用 turbofish const 实参)。

## 5. MS1.4 — present 取证 + 性能回填(D-MS1-5)

| # | 任务 | 验证方式 / gating |
|---|---|---|
| 1 | realtime 入口本机交互桌面真跑 ≥300 帧(1280×720,生产档 N=131072;不达标按 RFC-0010 §9 降级链留痕) | evidence/uc07_present_*.json(measured_local:帧数/采样像素/环境画像)+ 契约 §8 |
| 2 | ms1.bench.uc07_{sph_step_ms,pt_frame_ms,realtime_frame_ms} entries + evaluator 接线同 PR 回填(BENCH_PROTOCOL triple_run trimmed mean;阈值实测后定,不预设对照比值) | py -3 ci/budget_eval.py(entries measured_local)|

**出口判据**:present evidence 落档;≥2 项 bench measured_local 回填。

## 6. MS1.5 — close-out(agent 自主签署)

| # | 任务 | 验证方式 |
|---|---|---|
| 1 | 全量回归冻结:cargo test/clippy/fmt + trace + budget --strict(零 estimated)+ stable_snapshot --check + bilingual + guardrails 真实输出 | 全绿原文追加契约 §8 |
| 2 | close-out 终审:G-MS1-1~6 留痕指针表 + deferred 处置(RD-007 接通与否留痕 / RD-009/RD-025 carry-forward / RD-026~028 状态核)+ SG 复评(维持 not_triggered;D-008 红线不解除) | 契约 §8 追加 |
| 3 | 签署兑现:契约 status active→closed;check_guardrails 基准 v1-closed→ms1-closed;推 annotated ms1-closed tag(不匹配 release.yml 触发器);双基准 advisory 复核;(可选)11 §6 落地标注独立勘误 PR(00 §6.3,诚实措辞) | agent 签署留痕(对齐 V1 §8.4 先例) |

**出口判据**:MS1 期验收达成;close-out 终审完成。

## 7. 风险提示(引用,不另建登记)

- **staticlib↔link.exe 接线(MS1.2)**:CRT 静态合并/native-static-libs 集漂移;对策:实测 pin 库集,翻车降级 cdylib+旁置 DLL(RFC-0009 §9 Q-Link 预案)。
- **launch marshalling ABI(MS1.2)**:slot 打包/kernelParams 生命周期;对策:锁死实参子集(RX6024)+ 复刻 interop.rs 已验证物化纪律 + 单源 saxpy 数值门先行。
- **GPU 浮点确定性(MS1.3)**:排序式 atomics-free 由构造保证;不可达时降级「量化容差 + 双跑一致」走 RFC-0010 §9 修订留痕,不静默放宽。
- **实时 30fps 不达(MS1.4)**:键宽收窄(64³→32³)/ N 降 65536 / splatting 降级链;阈值实测后定。
- **brand 合成打穿推断(MS1.2)**:降级单 brand + cabi 运行期 context-id 校验(RX3006 保泛型签名面)。
- **快照重 bless 原子性(MS1.2/2b)**:条款+重 bless+bless_log 同 PR,分 PR 必卡死(步骤 49 硬红)。
- **GRX 交错(全期)**:MS1 期间不合入(契约 §7 ⑦),例外时 rebase 重 bless。
- **LF/CRLF 纪律(全期)**:新文件 LF+尾换行;registry/*.json 等 CRLF 例外追加行保持原风格;禁 Python 文本模式写文件,逐文件核 CR+尾字节(g2.2 教训)。

## 8. 修订记录

| 版本 | 日期 | 变更 |
|---|---|---|
| v1.0 | 2026-07-14 | 初版(MS1 契约配套;MS1.1~MS1.5 子里程碑分解 + 依赖图;双 Full RFC 前置 / 条款先行 + 两次快照重 bless 同 PR / 步骤 52/53 计划项随实现 PR 回填 / present 取证 evidence 面 / close-out 独立排程;deferred 承接 RD-007 inherited + RD-009 open + RD-025 open 顺延 → MS1;开工裁决引用户 2026-07-14 三项 AskUserQuestion 裁决,留痕契约 §7) |
