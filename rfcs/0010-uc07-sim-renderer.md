# RFC-0010 — UC-07 ruridrop：以 Rurix 为主语言的生产级渲染器/仿真二合一应用与主语言判据操作化

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0010(4 位制,编号永不复用,10 §9.5) |
| 标题 | UC-07 应用 ruridrop(GPU SPH 溃坝 + 路径追踪/光线投射二合一,应用层全 `.rx`)+「以 Rurix 为主语言(非 bolt-on)」判据的机器可审计操作化 + 离线 golden 三层协议 |
| 档位 | **Full RFC**(10 §3:使命判据(01 §6 第三层)验收载体,对齐 UC-04=RFC-0006 使命级先例;主语言判据操作化是新验收语义面;判档争议向上取严,硬规则 8) |
| 状态 | Agent Approved(2026-07-14)。agent 自主批准后可推进下游实现 PR |
| 承接里程碑 | MS1.3 / MS1.4(milestones/ms1/MS1_CONTRACT.md,验收门 G-MS1-3 / G-MS1-4 / G-MS1-5 / G-MS1-6) |
| 关联条款 | **零新 spec 条款**(见 §5:应用为既有语义面组合,UC-03 先例;验收判据属契约/CI 面) |
| 依据决策 | RFC-0009(single-source 宿主编排,Agent Approved 2026-07-14,本 RFC 硬依赖其 §4)· D-406 v2.0 · D-130(present shim 边界)· D-008(多后端红线不触)· 01 §4 §6 / 11 §6 / 02 §2 U3·U4 / 06 §2 §3 / 14 §5(证据分级) |
| Provenance | `Assisted-by: claude-code:claude-fable-5`。agent 自主决策,批准后推进下游实现 |
| Agent 批准 | Approved — 2026-07-14;批准范围含 §9 八项裁决(Q-Criterion 判据操作化为最高敏感面);记录于本文件与 MS1_CONTRACT §7 |

---

## 1. 摘要

**ruridrop**(瑠璃滴):首个以 Rurix 为主语言的生产级渲染器/仿真二合一应用,落 `apps/ruridrop/`(Rurix 包,不进 cargo workspace)。一套 `.rx` 代码、一个渲染核心、两个质量档:全 GPU 3D SPH 溃坝仿真(均匀网格 64³ + bit-split 基数排序,atomics-free,确定性由构造保证)产出粒子场;渲染核心是「球体图元 + 复用仿真网格的 3D-DDA」光线追踪——**离线档**多弹射路径追踪 + 渐进累积出确定性 PPM 帧序列(golden 进 CI 步骤 53),**实时档**同核 1spp 主光线 + 影子光线直写 D3D12 present 共享 backbuffer(本机取证,evidence 面)。应用层(仿真 kernel、渲染 kernel、宿主帧循环/资源编排/出图落盘)**全部 `.rx`,零 `.rs`**,经 RFC-0009 的 std::gpu/present/imageio 面编排,`rx build` 产单 EXE。CUDA/PTX compute 路线(完整语言表达力),不触 DXIL 图形路。

## 2. 动机

使命判据(01 §6:「至少一个生产级渲染器/仿真系统选择 Rurix 作为主语言」/ 11 §6 同句)是愿景措辞,**无操作化定义**——全仓无「主语言/非 bolt-on」的可测判据;既有集成全部是宿主承载 + Rurix 当 compute pass(G1.3 引擎集成 = 3 个 C 前向函数;GRX = Godot 后处理 pass)。用户 2026-07-14 三裁定:渲染器+仿真二合一 / 离线 golden + 实时 present / 最严主语言判据。本 RFC 做两件事:① 把判据操作化为**机器可审计门**(§4.1);② 定义首个满足该判据的应用本体与其验收协议(§4.2~§4.6)。

**为何需要 Full RFC(而非 Direct/Mini)**:使命级验收载体历来 Full RFC(UC-04 = RFC-0006);主语言判据操作化定义了新验收语义(什么算「主语言」,白名单边界在哪)——该定义一旦落地即约束后续所有使命判据表述,向上取严(硬规则 8)。应用本身零新语言语义(§5),但验收协议(golden 三层/防降级硬门)与判据定义须 RFC 级留痕。

## 3. 指导级解释(用户视角)

```
apps/ruridrop> rx build src/offline.rx  -o ruridrop_offline.exe   # 离线:N 帧路径追踪 PPM 序列
apps/ruridrop> rx build src/realtime.rx -o ruridrop_realtime.exe  # 实时:D3D12 present 窗口预览
apps/ruridrop> rx build src/refcpu.rx   -o ruridrop_refcpu.exe    # CPU 参考:同一 device fn host 重放
```

打开 `src/` 看到的全是 `.rx`:`sim.rx`(SPH 十个 kernel)、`render_core.rx`(DDA/着色 device fn)、`render_pt.rx`/`render_rt.rx`(两档渲染 kernel)、`params.rx`/`dmath.rx`/`rng.rx`,以及三个入口——宿主侧建 Context、上传粒子、逐帧「排序 → 密度 → 力 → 积分 → 渲染」全在 `.rx` 里(RFC-0009 面)。溃坝水柱在盒内坍塌、飞溅、平复;离线档出 1280×720/256spp 带全局光的 beauty 帧;实时档窗口里 30fps 预览同一场景。没有一行应用层 Rust/C++。

## 4. 参考级设计

### 4.1 主语言判据操作化(Q-Criterion,G-MS1-3 的语义源)

「以 Rurix 为主语言(非 bolt-on)」在本 RFC 定义为同时满足、机器可审计的四条:

1. **应用层零外语言源**:应用包目录内(`apps/ruridrop/`)不存在 `.rs`/`.cpp`/`.c`/`.py` 等任何非 `.rx` 源文件(构建产物与 golden 清单除外);
2. **同源单包**:GPU kernel 与宿主编排(资源创建/传输/launch/帧循环/落盘)在同一 Rurix 包内,经 `rx build` 从 `.rx` 入口产出单 EXE;
3. **基础设施白名单**:仅允许链接**语言基础设施**——rurixc 产物、rurix-rt / rurix-rt-cabi / rurix-d3d12 shim / image-io(经 RFC-0009 C ABI 面),等价于任何语言的运行时与标准库;应用不得携带自有 native 胶水;
4. **防降级硬门**(措辞镜像 G-G2-4):手写 Rust/C++ 宿主 harness、host-only 模拟、桩化 launch、SKIP 充绿均不得替代;审计(源清单 + 产物链路)为 CI 步骤 53 前置检查并写 evidence。

**诚实边界**:满足以上四条 = 「首个以 Rurix 为主语言的生产级渲染器/仿真系统(第一方)」;01 §6 的「外部选择/采纳」维度显式 carve-out(MS1_CONTRACT out_of_scope),本 RFC 不宣称使命成功层整体达成。

### 4.2 仿真:全 GPU 3D SPH 溃坝(确定性由构造保证)

- 场景:单位盒容器水柱溃坝;固定初始网格布局 / 固定 dt / 固定子步数(常量全在 `params.rx`)。规模:生产档 **N = 131,072**,CI 冒烟档 **N = 4,096**。
- 邻居结构:均匀网格,cell 边长 = 光滑半径 h,网格 64³(cell id 18 bit)。
- **排序式邻居搜索(atomics-free)**:scoped atomics 无 PTX codegen(仓内既成现实,reduce.rx 明言规避)→ 采用 **bit-split 基数排序**:`split_flag`(取 bit 反)→ `scan_block`/`scan_sums`(改造既有 `scan.rx`,f32→u32)→ `split_scatter`(稳定 split)× 18 bit → `sim_reorder`(gather 重排 SoA)→ `sim_cell_bounds`(边界线程独写)。排序结果与线程调度无关 → 粒子全局序唯一。
- 物理:`sim_density`(27 cell 固定枚举序 × cell 内升序遍历,poly6 核,EOS `p = k(ρ−ρ₀)`,uc03 常数语义母本)→ `sim_forces`(spiky 压力 + 粘性 + 重力,**线程私有寄存器固定序累加**)→ `sim_integrate`(半隐式 Euler + 速度钳制 + 盒体反弹阻尼)。
- **确定性论证**:零原子、零浮点竞争(每个可写位置唯一 owner 线程);累加序 = 固定 cell 枚举序 × 排序后下标升序;NVVMReflect 精确路径(不开 FASTMATH,RXS-0081);固定初值/dt/seed → 同机同驱动逐位确定。残余变量仅驱动 JIT 版本 → 由 §4.4 三层 golden 消化。
- kernel 全部块大小 256(shared 数组等长 = 块大小的 reqntid 推导约束);SoA 布局 + `while` 循环 + 标量 device fn(device 无 struct 值/for-range 的既成纪律)。

### 4.3 渲染:一个核心、两个质量档

- **共享核心**(`render_core.rx` device fn 层):`rx_sqrt`(host/device 逐位同义软件 Newton,conformance 母本)/ xorshift32 RNG + Wang hash 种子 / **3D-DDA 均匀网格步进 + cell 内球体求交**——直接复用仿真的 `cell_bounds` + 排序 `pos`,**加速结构零额外构建** / 着色:Lambert + Schlick Fresnel(pow5,免 exp)+ 速度→色相映射(uc03 先例)+ 天空梯度 + 单矩形面光源 NEE;半球/圆盘采样用**拒绝采样**(免 sin/cos——host 无 transcendental intrinsic,渲染核设计成「仅 sqrt」,refcpu 才能同义重放)。
- **离线档**(`pt_render`,ThreadCtx<2> 16×16):每像素 spp 循环{相机抖动光线 → DDA → ≤4 弹射{命中:BRDF+NEE;未中:天空}}累加 HDR;host 按 spp 分批 launch 渐进累积;`pt_finalize` Reinhard + gamma + 量化(`sr_tonemap`/RXS-0116 口径)。逐像素串行 RNG 流 → 固定 seed + 固定 spp = 逐像素确定。生产档 1280×720/256spp/4 弹射;冒烟档 160×120/8spp/2 帧。
- **实时档**(`rt_primary`):同核退化 1spp——主光线 DDA + 影子光线 + 天空/棋盘地面,直写 present 共享 f32 RGB backbuffer(RFC-0009 §4.6 blit 契约);帧循环 = 仿真子步 + 渲染 + present typestate,零 readback。降级链(§9 Q-Perf):DDA 发散不达标 → 粒子 tile 分桶 splatting(sr_* 母本)或半分辨率。
- 应用形态:`apps/ruridrop/{rurix.toml, src/*.rx}`,共享模块经 `mod name;`(RXS-0196)组织;三入口各一 EXE(host `.rx` 无 argv,多入口零语言扩展)。预估应用层 ~1.8k LOC `.rx`,应用层 Rust = 0。

### 4.4 离线 golden 三层协议(G-MS1-4 的语义源)

| 层 | 门性质 | 判据 | 防什么 |
|---|---|---|---|
| ① 确定性 | **硬门** | 冒烟档同机两次运行,逐帧量化 PPM 字节 SHA-256 逐字节一致 | 线程调度/竞争类非确定(uc03/soft_raster 先例) |
| ② 参考容差 | **硬门** | GPU 帧 vs `refcpu` 入口(同一 `.rx` device fn host 重放,device⊂host 单向可达 + rx_sqrt 双路同义)量化域 ≥99.5% 像素每通道差 ≤1 LSB、最大 ≤2 LSB | FMA 收缩/驱动 JIT 漂移误伤,同时锚定语义正确性(gemm/scan 容差先例) |
| ③ blessed 哈希 | 软门 | 逐帧 SHA-256 == `tests/uc07/golden_manifest`;漂移红 + 重 bless 留痕(`tests/uc07/bless_log.md`,数据行忌「日期」子串) | 驱动升级基线漂移(DXIL golden/RD-008 bless 纪律同款) |

内建**数据流红绿**(反 YAML-only,镜像步骤 48 的 191↔127 先例):篡改 kernel 物理/着色常数 → 经**同一编译链**重编 → digest 变红 → 复原绿;仅「跑完了」不接受。不采用逐字节 golden 直接作硬门入库(驱动 JIT 变更即碎,§7)。

### 4.5 实时 present 通路(G-MS1-5)

realtime 入口经 RFC-0009 §4.6 typestate 帧循环真跑;验收位 = **evidence 面非 CI 硬门**(CI runner 无交互桌面,SKIP 不充绿纪律;镜像 realtime_present_smoke 双态先例):本机交互桌面真跑 ≥300 帧 → `evidence/uc07_present_*.json`(measured_local:帧数/采样像素对照/环境画像)+ MS1_CONTRACT §8 留痕。

### 4.6 性能证据形态(G-MS1-6)

`ms1.bench.uc07_sph_step_ms` / `ms1.bench.uc07_pt_frame_ms` / `ms1.bench.uc07_realtime_frame_ms`(端到端 33.3ms 目标档)——绝对值 entries,measured_local,BENCH_PROTOCOL triple_run trimmed mean;**阈值实测后定,不预设对照比值、不预造 estimated 占位**(14 §3);随 MS1.4 与 evaluator 同 PR 回填。「生产级」操作化(Q-ProdGrade):确定性(①②)+ 性能证据(本节)+ 生产档 N 帧零崩溃 + 单 EXE 分发形态 + 观感要素(自由表面/飞溅/容器地面/软硬阴影/Fresnel 水面/色调映射)。

## 5. 下游 spec 条款映射(spec diff,10 §3 要件)

**零新 spec 条款。** 依据:UC-03 先例(应用工程编排不新增 spec 语义面);ruridrop 是既有语义面的组合——RFC-0009 面(RXS-0189~0199)+ device 语义(RXS-0066~0082)+ 数学/图像/软光栅(RXS-0104~0121)+ present(RXS-0140~0143);「主语言判据/golden 协议」属契约验收面(MS1_CONTRACT G-MS1-3/4)与 CI 面(步骤 53),非语言语义事实源。若执行期发现确需新语义(如确定性 PRNG 进 stdlib),以 RXS-0200+ 走本 RFC 修订留痕后追加,不预造。

- **错误码策略**:零新 RX 码——应用层错误 = 应用退出码;工具/运行时失败走 RFC-0009 §4.5 确定性诊断。registry/error_codes.json 本 RFC 预期 0-byte。
- 锚定:应用 `.rx` 文件不进 trace 锚定源(应用非 conformance 语料);步骤 53 与 evidence schema 为验收锚。

## 6. feature gate / tracking / 实现序(10 §3 要件)

- **gate 形态 = 无**(应用非语言面);门 = MS1_CONTRACT G-MS1-3~6 + CI 步骤 53。
- **失败测试先行**:本 RFC 提案时点,`apps/ruridrop/`、`ci/uc07_offline_golden_smoke.py`、`tests/uc07/` 在 `main` 上**均不存在** = RED;MS1.3 落地后转绿。
- **实现序**(门控于本 RFC 与 RFC-0009 合入 + MS1.2/MS1.2b 面就位):
  1. **MS1.3 单 PR**:`apps/ruridrop/**` 全 `.rx` → 冒烟档本机 device 真跑两遍一致 → 首次 golden bless(`tests/uc07/{golden_manifest,bless_log.md}`)→ CI 步骤 53 `ci/uc07_offline_golden_smoke.py`(前置零 `.rs` 审计 + 三层门 + 数据流红绿)+ `ms1.counter.uc07_offline_golden_frames` + evaluator 分支 + evidence schema + pr-smoke.yml + trace 再生。纯 kernel 语料可在 MS1.2 期间预写(不依赖宿主编排面)。
  2. **MS1.4 单 PR**:realtime present 本机取证 + `ms1.bench.*` entries 回填(§4.5/§4.6)。
- **真实红绿**:步骤 53 内建数据流红绿(§4.4);run URL 归档 MS1_CONTRACT §8。RD-007 评估点:kernel 若用 turbofish const 实参则触发接通评估,执行期留痕(deferred.json v1.48)。

## 7. 备选方案

- **继续 bolt-on(宿主 Rust/C++ + Rurix kernel)**:被用户最严裁定直接否决;判据操作化后机器审计不可通过。
- **DXIL 图形管线渲染**:否决——RXS-0171 body 白名单(直线代码+加减乘除+LOD-0 采样)阻断生产着色器;compute 路线(CUDA/PTX)拥有完整表达力且真跑成熟。
- **BVH 加速结构**:否决(首期)——粒子动态场景逐帧重建 BVH 昂贵;复用仿真均匀网格做 DDA 零额外构建成本、且与排序确定性同源。BVH 留 RD-028。
- **histogram 原子计数排序**:否决——scoped atomics 无 codegen;且浮点原子累加顺序不定破坏逐位确定性。bit-split scan+scatter 同时解决两者;若原子 codegen 后续落地,可作性能升级不改语义。
- **逐字节 golden 硬门直接入库**:否决——驱动 JIT 变更即碎;改为①②硬门 + ③软门 bless(§4.4)。
- **单入口 + argv 分派**:否决(首期)——host `.rx` 无 argv 面;三入口零语言扩展;`env::args` 留 RD-026 面评估。

## 8. 不做(范围红线)

- **交互实时模式**(键鼠/摄像机/参数交互/事件循环):→ **RD-027**(承 RD-019 同类口径;D-130 维持,SG-010 留续号防「窗口框架进语言」扩张)。
- **路径追踪完备性**(BVH/重要性采样扩展/多材质系统/compute 路纹理对象/降噪):→ **RD-028**;纹理面触 06 §4.2 内存模型,须后续 Full RFC 增补 RFC-0007(邻接 RD-022/023/024)。
- **外部采纳宣称**:§4.1 诚实边界,carve-out 维持。
- **多后端**(D-008/SG-003)/ **发行打包面**(ruridrop 不进 bundle/SBOM,零 Release 层增量)/ **规划文档改写**(02 不动,UC-07 编号自 MS1_CONTRACT §7 claim)。

## 9. 未决问题 / 关键裁决(agent 自主签署)

| # | 裁决点 | 裁决(2026-07-14) |
|---|---|---|
| Q-Criterion | 主语言判据机器化 | §4.1 四条(零外语言源/同源单包/基础设施白名单/防降级硬门)+ 诚实边界 carve-out |
| Q-AppScope | 场景/规模/分辨率定数 | 溃坝定场景;生产 N=131072 / 冒烟 N=4096;离线 1280×720/256spp(冒烟 160×120/8spp/2 帧);常量集中 params.rx |
| Q-Determinism | 逐位确定性可达性 | 排序式 atomics-free 由构造保证(§4.2);若实测不可达 → 降级「量化容差 + 双跑一致」,**须回本 RFC 修订留痕**,不静默放宽 |
| Q-Present | present 验收位 | evidence 面非 CI 硬门(§4.5,双态先例) |
| Q-Perf | 性能证据形态 | 绝对值 entries measured_local,阈值实测后定;realtime 目标档 33.3ms;降级链 = 键宽收窄/N 降 65536/splatting/半分辨率,触发即留痕 |
| Q-ProdGrade | 「生产级」操作化 | §4.6 五要件(确定性/性能证据/N 帧零崩溃/单 EXE/观感要素清单) |
| Q-Loc | 应用落位 | `apps/ruridrop/`(新顶层 apps/,Rurix 包不进 cargo workspace;check_structure 白名单制不受影响) |
| Q-UCNum | UC-07 编号 | 自 MS1_CONTRACT §7 claim,承 02 §4 序列,永不复用;02 冻结不改写 |

## 10. 稳定化与 provenance

- **稳定化**(10 §5):应用非语言 stable 面,不进快照;`tests/uc07/` golden 走 bless 纪律(③软门);若执行期追加 RXS-0200+ 语义条款则按常规通道随快照重 bless。FCP-lite #121 开放通道下公开(advisory,agent 自主裁决合入)。
- **Provenance**:`Assisted-by: claude-code:claude-fable-5`;agent 自主批准并记录(D-406 v2.0)。

## 11. 规范与实现依据

- 仓内:rfcs/0009-host-gpu-orchestration.md(硬依赖);src/uc03-demo/src/lib.rs(SPH 常数/积分/边界/速度着色语义母本)+ present.rs(帧循环母本);src/rurix-rt/kernels/{scan,sr_binning,sr_raster_tile,sr_depth,sr_tonemap}.rx(排序/光栅母本);conformance/stdlib/{host/vec_ops.rx(rx_sqrt),device/geom_scalar.rx}(双路同义纪律);ci/{uc03_demo_smoke,soft_raster_smoke,dxil_uc04_device_smoke,realtime_present_smoke}.py(golden/红绿/双态先例);spec/imageio.md RXS-0114~0117 / spec/device.md RXS-0066~0082。
- 外部:SPH(poly6/spiky 核,Müller et al. 2003 语义口径);3D-DDA(Amanatides & Woo 1987);Schlick Fresnel;Reinhard tonemap;xorshift32/Wang hash。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-07-14 | AI 起草初版(MS1.1;应用设计承 Phase-2 勘察:device 能力清单/缺口替代实证 + 排序式确定性方案 + golden 三层先例核查) | Full RFC(Draft) |
| Agent approval | 2026-07-14 | agent 自主批准全文(含 §9 八项裁决,Q-Criterion 判据操作化)并记录;批准后推进 MS1.3/MS1.4 实现 PR | Full RFC(Agent Approved) |
