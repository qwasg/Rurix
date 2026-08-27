# RFC-0049 — G35 GPU 粒子系统:确定性 GPU 粒子九波伞形(对标并超越 UE5 Niagara 五轴)

| 字段 | 值 |
|---|---|
| RFC 编号 | RFC-0049(4 位制,编号永不复用,10 §9.5;registry/number_ledger.json RFC.next_free=49 落盘前实测顺位领取,v1.191 校准同批) |
| 标题 | G35 GPU 粒子系统:确定性 GPU 粒子九波伞形(对标并超越 UE5 Niagara 五轴) |
| 档位 | **Full RFC**(10 §3:全新 GPU 子系统语义面——九波确定性协议〔禁原子抢槽/随机带单源/容差协议〕+ sim→scan→compact→emit→indirect_args 帧序 + 声明式 emitter 资产 schema + 回放 journal 语义;🔒 触执行器 alpha blend 状态面扩展(rurix-rt 库面,§9 Q2)与 TLAS 动态实例臂 FIF 互斥约束(§9 Q3);G26~G29 device 化 Full RFC 先例(RFC-0043/0044/0045/0046);AGENTS 硬规则 5) |
| 状态 | **Agent Approved(2026-08-27)**。D-409 对抗性评审完成(第 1 轮 17 findings 全 disposition,见 §9.1);G-G35-10 收口门前置(milestones/g35/G35_CONTRACT.md)由本行满足 |
| 承接里程碑 | G35(GPU 粒子系统期,milestones/g35/G35_CONTRACT.md)+ 验收门 G-G35-1~G-G35-10 |
| 关联条款 | **零新 RXS(声明)**——渲染器是库不进语言(06 §8.3),九波 kernel 全用现有语言面(compute + RayQuery RXS-0297~0300 谱系引用不扩);见 §5 |
| 依据决策 | D-402(三档变更门)· D-406(agent 完全自主)· D-409(对抗性评审)· D-131(后端路径纪律——本 RFC kernel 全族 Vulkan SPIR-V 单腿,零 DXIL 腿)· 06 §8.3(render graph/ECS「它们是库」——粒子系统同律)· RFC-0045 §1.2(随机带单源纪律先例)· RFC-0047 §5.5(G31+ 唯一法定输入面) |
| Provenance | `Assisted-by: cursor:claude-fable-5`(起草 + v0.2 评审修法批)。agent 自主决策并批准 |
| Agent 批准 | Approved — 2026-08-27;批准范围 = 全文含 §4.7/§4.13 🔒 子节——**协议面冻结**(帧序/indirect args 布局/排序反键/pid 硬域/发射钳制/事件溢出裁剪/空间哈希三阶段/SDK ABI 最小签名闭集),数值调优与 schema 加性扩展非 stable(§10;F17 disposition);记录方式 = 本行 + 文末修订记录 |
| 对抗性评审 | 评审者 provenance `Assisted-by: cursor:gpt-5.6-sol-medium`(**≠ 起草 `cursor:claude-fable-5`,跨模型合规**,D-409 / 硬规则 2);第 1 轮 17 findings(5 blocker + 11 high + 1 med)**全部采纳并修**;详见 §9.1 |

---

## 1. 摘要

本 RFC 为 G35「GPU 粒子系统期」的伞形语义 RFC:在 rurix-render 引擎库面建设**确定性 GPU 粒子系统**,对标并超越 UE5 Niagara 的五轴:

1. **确定性 GPU 粒子**——Niagara GPU sim 非确定(官方 Determinism flag 仅 CPU sim 生效,粒子索引 >64 不稳定);本系统以「禁原子抢槽 + 分段稳定 scan 推导槽位 + 随机带单源」达成固定输入 device 双跑**位级一致**(§4.1)。
2. **光追集成**——mesh 粒子实例进 TLAS 收光追阴影/GI;ray query **同帧**碰撞(vs Niagara GPU ray traced collisions 异步一帧延迟)(§4.7/§4.9)。
3. **规模**——容量可寻址百万(PARTICLE_CAP_MAX = 1048576;**性能以 measured 为准**,spine 单 invocation 串行段为已知瓶颈,scan_spine_ms/sort_spine_ms 单列登记,§4.2/§4.3——禁规模宣传语,F10),GPU-driven 零回读:存活总数经 scan spine 总和槽直供 indirect args,host 全程不读回(vs Niagara GPU sim 动态 bounds 需回读故被禁用)(§4.5)。
4. **流体统一物理**——count-sort 空间哈希邻居 + XPBD 约束求解,与粒子池同一确定性协议(对标 Niagara Fluids);MPM 为评估窗登记不承诺(§4.10)。
5. **数据驱动作者面**——声明式 emitter 资产 + 参数化 megakernel + 热重载 + SDK C ABI 加性(§4.11)。

九波(门 key 与脚本冻结于 milestones/g35/CI_GATES.md):G35-1 GPU 基元(分段稳定 scan〔已落树〕+ 24 位键 3-pass 稳定 radix sort + compact)/ G35-2 粒子核心(SoA 池 ping-pong + 确定性发射 + 半隐式 Euler + 稳定压缩 + indirect args 零回读)/ G35-3 渲染接线(billboard splat 进 G34 统一车道 + 软粒子 + 粒子 MV + mesh 粒子 TLAS)/ G35-4 半透明(深度排序 + WBOIT 双臂)/ G35-5 碰撞与力场 / G35-6 事件与 particle_view 双向桥 / G35-7 流体 / G35-8 作者面与 SDK / G35-9 确定性回放回滚 + 收口。

```
emitter 资产(声明式,host 单源)          rand_table(PCG32 host 单源,SSBO 只读)
   │ 装配期编译为参数块(SSBO)                │
   ▼                                        ▼
[sim(+ray query 碰撞/力场)] → [scan 三 kernel(禁原子)] → [compact(A→B 稳定搬运)]
   → [emit(尾部连续区;新粒子下一帧首次积分)] → [indirect_args(spine 总和槽,零回读)]
   │ (G35-P v1 帧序冻结 = core.rs::frame 字面;ping-pong 交换点 = 帧末)
   ▼
billboard splat / mesh 粒子 TLAS 臂 → 排序(24 位反键 radix)+alpha blend ∥ WBOIT 定点臂 → G34 统一车道
   │
   ▼
粒子 MV 写速度(对拍 ≤2px 门口径)+ 事件通道/particle_view + 回放 journal(digest 位级)
```

代码契约已冻结:`src/rurix-render/src/particles/mod.rs`(G35-P 冻结契约 v1)+ `scan.rs / primitives.rs / core.rs` host 金标准 + `kernels/g35_scan_*.rx`(scan 三 kernel)与 W1/W2 已落树 kernel 族(sort hist/spine/scatter + compact_u32 + emit/sim/particle_compact/indirect_args,均已过 rurixc + spirv-val)。本 RFC §4 为该契约的语义面字面化(v0.2 评审修法批已按冻结实现逐字对齐——F1/F3/F6 blocker),两者冲突时以 mod.rs 契约 + 契约修订程序为准。

## 2. 动机

- **用户立项指令字面**:「为本项目制定完整的GPU粒子系统,要求超过虚幻五」(2026-08-27,裁决全文记 G35_CONTRACT §7 谱系)。
- **待办承接**:G31_PLUS_COMMERCIAL_RENDERER_TODO #13(OIT/半透明与毛发进生产帧——触发条件字面「游戏画面含玻璃/**粒子特效**/头发时触发」,粒子特效进画面 = 命中 milestones/g31/g31_oit_evaluation_window.json not_triggered 终态的 conditional_wiring_sketch ① re-trigger 条件)+ #81(粒子/半透明/WPO 写速度——「Niagara CPU 粒子、WPO、程序动画漏写速度 → TSR/DLSS 鬼影」,本 RFC §4.6 粒子 MV 兑现)+ #80(运动模糊/景深依赖 MV 完整性——依赖登记不兑现)。
- **对标短板证据(UE5 Niagara 官方文档)**:① GPU sim 非确定——Determinism flag 仅 CPU sim 生效,GPU 粒子依赖原子计数器抢槽 + 线程调度序,粒子索引 >64 不稳定;② GPU ray traced collisions 异步执行,碰撞结果**延迟一帧**生效;③ GPU sim 动态 bounds 需 GPU→CPU 回读故被禁用(fixed bounds 强制)。本系统五轴逐项给出结构性更强承诺(§4)并冻结诚实边界(§9 Q4)。
- **底座已齐**:G34 统一车道(全特性同开真窗口 swapchain)在树三门新鲜绿件在案;compute RayQuery 生产谱系(RXS-0297~0300)、tlas_update/blas_refit 逐帧 refit(G34-2/G34-3)、rurix-physics Field 通道(RFC-0024)、粒子 scan 基元三 kernel 已落树并过 rurixc + spirv-val。

**为何需要 Full RFC(而非 Direct/Mini)**:全新 GPU 子系统**语义面**——九波共享的确定性协议(禁原子抢槽/随机带单源/整数流零容差·f32 流标定容差)、sim→scan→compact→emit→indirect_args **帧序语义**、声明式 emitter 资产 schema(数据驱动作者面 = 新资产语义)、回放 journal 语义,均为运行时语义级扩张方向(10 §3);🔒 触执行器 **alpha blend 状态面扩展**(rurix-rt 库面新管线状态,§9 Q2)与 **TLAS 动态实例臂 FIF 互斥**约束面(§9 Q3);G26(FG)/G27(HZB)/G28(ReSTIR)/G29(slab)四期 device 化均以 Full RFC(RFC-0043~0046)承载同量级语义面,判档同类先例;判档争议向上取严(AGENTS 硬规则 5)。

## 3. 指导级解释(用户视角)

**第一步:声明一个 emitter 资产**(声明式,数字全 f32 表驱动;**v1 最小字段闭集随本 RFC 冻结**——10 字段,与 host 金标准 `core.rs::EmitterDesc` 同名承载;加性扩展走 schema MINOR 修订非 stable,F17):

```toml
# assets/particles/campfire_sparks.emitter.toml(v1 最小字段闭集,冻结)
[emitter]
name       = "campfire_sparks"
pos        = [0.0, 1.0, -0.5]   # 发射中心(EmitterDesc.pos)
spread     = [0.4, 0.2, 0.4]    # 位置半幅(px = ex + (r0·2−1)·spread_x)
vel_base   = [0.0, 3.0, 0.0]    # 初速基值
vel_spread = [1.0, 0.5, 1.0]    # 初速半幅
life_base  = 1.2                # 寿命基值(life = life_base·(0.5 + 0.5·r6))
gravity_y  = -9.8               # v1 唯一积分外力(drag v1 恒 0 登记)
emit_curve = { kind = "constant", rate = 2000.0 }   # 常数 | 阶梯(step 表);配额 = floor 累计差分
render     = "billboard"        # billboard | mesh(mesh = TLAS 臂,§4.7)
blend      = "alpha"            # additive(定点累加臂,§4.6)| alpha(半透明双臂,§4.8)
```

池容量(SEG=256 整倍数,≤ PARTICLE_CAP_MAX)与随机带 seed 为**车道/系统装配参数**,不入 emitter 资产字段闭集(单源纪律:seed 归 journal/系统面,§4.12);碰撞臂/力场绑定为 G35-5 波 schema 加性扩展(§4.9/§4.13 降级链语义先行冻结)。

**第二步:进生产车道**(粒子面缺省关;`--particles off` 时缺省面与母版 Stage A 锚位级一致——加性扩展 0-byte 机器证明,G34 同律):

```text
g34_full_lane --full --particles on --emitter assets/particles/campfire_sparks.emitter.toml
```

**用户可依赖的承诺**:同 GPU 同驱动下固定输入(seed/dt/相机轨迹)双跑,粒子面输出 digest **位级一致**;发射序/压缩序/排序序与线程调度无关(粒子下标序);粒子写 MV 通道——**门口径**(F11,禁「零鬼影」宣传语):粒子 MV 与 host 投影对拍 ≤2px(G34-3 三类速度口径先例)+ TSR 互操作 digest 判别(G-G35-3);emitter 资产热重载(参数块重上传)不重启车道;回放 journal 记录后可位级重放与回滚(§4.12)。

## 4. 参考级设计(G35-P 冻结契约 v1 字面化;事实源 = src/rurix-render/src/particles/mod.rs)

### 4.1 确定性协议(对 Niagara GPU sim 非确定性的根因解法)

- **禁原子抢槽**:发射/压缩槽位一律经**分段稳定 scan**(§4.2 三 kernel)推导,顺序 = 粒子下标序,与线程调度无关 ⇒ 固定输入双跑位级一致。Niagara 式原子计数器抢槽(`atomicAdd` 领槽)被否决(§7.1)。
- **随机带单源纪律**(G28 RFC-0045 §1.2 同律):PCG32(PCG-XSH-RR,O'Neill 2014;state/inc 显式)只在 host 出现(`rand_table(seed)`:长度 RAND_TABLE_LEN=65536 的 [0,1) f32 表,`next_f32` = 高 24 位 ÷ 2^24——f32 精确域,device 消费同位型),device 经 SSBO 只读消费 `r = rand_table[(pid·RAND_K + slot) % RAND_TABLE_LEN]`(RAND_K=7919 步进素数——**去相关声明修正为 slot 通道间去相关**,F8);device 端**零超越函数、零位运算**(kernel 语言面无 `^`/`>>`,整数域一律 usize 除/模精确算术,g34_unified_gi.rx 图集 unpack 先例)。
- **随机带周期如实登记**(F8):消费律模 RAND_TABLE_LEN=65536 ⇒ pid 与 pid+65536 在**全部 slot** 消费同一表项(属性克隆);**v1 域声明 = 单 emitter 活跃 pid 窗 < 65536 时无克隆**(同帧共存粒子 pid 跨度 < 65536,由池容量与发射节奏保证的场景域,门 harness 域内);长周期方案(counter-hash——u64 位运算域可用,`visbuffer_sw_u64.rx` 语料先例)登记评估窗(§9 Q6),v1 不实现。
- **容差协议**:整数流(pid/flags/scan/sort 键与序)device↔host **零容差位级**;f32 流(pos/vel/age)device↔host 走标定容差(threshold = measured × 2.0 协议冻结 k,程序产禁手写,g35_budget.json);device 双跑一律位级。

### 4.2 分段稳定 exclusive scan 三 kernel(生产形态 = 保守分段臂)

无 shared memory / 无原子 / 无 lookback(Vulkan 前进保证缺位——decoupled-lookback/Onesweep 判为评估窗登记,§9 Q1):

1. `g35_scan_seg_sum.rx` dispatch [nseg,1,1]:线程 s 串行求段和 → seg_sums[s];
2. `g35_scan_spine.rx` dispatch [1,1,1]:单 invocation 对 seg_sums 串行 exclusive scan → seg_offsets[0..nseg],并写总和到 seg_offsets[nseg](槽位 = 存活总数,indirect args 消费面);
3. `g35_scan_seg_apply.rx` dispatch [nseg,1,1]:线程 s 段内串行 running 前缀 → out[i] = 全局 exclusive scan。

params 面(f32 SSBO):`[0]=n [1]=nseg [2..4)=reserved(恒 0)`(n ≤ 2^24 f32 精确)。元素 u32,单值 < 2^24、总和 < 2^32。host 金标准 = `particles/scan.rs`(与三 kernel 逐字同源三阶段分解 + 独立单循环参考实现互核,防「同一错误两处照抄」);对拍 = 整数域零容差位级。分段布局:SEG=256,所有池容量 N 须为 SEG 整倍数,nseg = N/SEG ≤ NSEG_MAX=4096,PARTICLE_CAP_MAX = SEG × NSEG_MAX = 1048576(1M 粒子 = 4096 段)。

**已知规模瓶颈如实登记**(F10):kernel 2 spine 为单 invocation 串行(上限 4096 段循环)——为确定性换出的串行段,禁以「百万级性能」宣传;`scan_spine_ms` 在门 evidence 单列 measured 登记(Onesweep/lookback 并行化归评估窗,§9 Q1)。

**同步契约**(F2;跨 dispatch 可见性显式化):

- **probe 面(W1/W2 现形态)**:经 `rurix_rt::vk::run_compute` 每 dispatch 独立 submit + wait——顺序可见性由提交边界保证(seg_sums → spine → seg_apply 三 dispatch 间 shader-write→shader-read 无需帧内 barrier),这是已落树 probe/门的合法同步形态。
- **生产面(DeviceFrameSession 接线,G35-3 起)**:粒子 pass 族进统一车道时**逐 pass barrier plan**——每 pass 声明(资源,TargetState)转换表:seg_sums/seg_offsets/scan_out/flags 等中间缓冲 shader-write→shader-read 转换逐段显式;args 缓冲加 **write→INDIRECT_COMMAND_READ** 转换(§4.5);禁隐式依赖提交边界。覆盖审计 = G-G35-3 门 `barrier_plan_audit` fact(facts 闭集事实源 = G35_CONTRACT acceptance_gates)。

### 4.3 排序基元(G35-1):24 位深度键 3-pass 稳定 radix sort + compact

- **深度键零位转换**(语言面无 bitcast):`key = floor(clamp(d/d_max,0,1)·16777215)`——24 位量化单调键(`depth_key24` host 同式;DEPTH_KEY_MAX=16777215=2^24−1 为 f32 精确表示域)。**边界语义冻结**(F9):非有限 depth(NaN/±Inf)与负 depth 经 clamp 落 0 或 1 端——NaN 经 clamp 语义归 0 端 ⇒ 键 0;`d_max ≤ 0` 退化域一律返回 0;构造域保证键 ≤ 2^24−1(**生产键唯一来源 = depth_key24 构造**)。
- **键域检查纪律**(F9):sort 基元 host 面键域检查(< 2^24)升级为 **release 硬检查**(违约走显式 Err/panic 语义,由 G35-1 波代码兑现——当前实现为 debug_assert 的差距如实登记,收口前清账);device 侧对 ≥2^24 输入的高位截断行为如实登记(3-pass×8bit 只覆盖 24 位)——非构造域输入不进生产路径。
- **排序方向冻结**(F4):radix 升序输出 = 键升序;**半透明 back-to-front 排序键 = DEPTH_KEY_MAX − depth_key24(d, d_max)**(反键:远者 depth 大 ⇒ depth_key24 大 ⇒ 反键小 ⇒ radix 升序排前 ⇒ 先画远者 = back-to-front);不透明/加性臂不排序(§4.6/§4.8)。
- **radix 3 pass × 8 bit**:digit = `(key as usize / 256^p) % 256`(整数除/模,零位运算);每 pass = 段直方图(256 bin × nseg)→ digit-major spine(单 invocation 双层串行 exclusive scan,§4.2 spine 同律形态;host 金标准侧同式消费 §4.2 scan 面;直方图域:单值可 >1,总和 = n < 2^32 满足域前提)→ 段内串行稳定 scatter。**稳定序 = 段序×段内序**(digit 升序 × 段升序 × 段内下标序,LSD 稳定性由段内串行保证,与线程调度无关)。
- **compact_u32**:flags∈{0,1} 经 scan 推导目标槽位的流压缩基元(排序/粒子压缩共用)。
- device 面 = `kernels/g35_sort_hist.rx / g35_sort_spine.rx / g35_sort_scatter.rx / g35_compact_u32.rx`(hist/spine/scatter 三段命名 = mod.rs 契约字面;+ 实验臂 `g35_sort_onesweep.rx`,评估窗登记不进生产,§9 Q1);host 金标准 = `particles/primitives.rs`。
- **已知规模瓶颈如实登记**(F10):sort spine 每 pass 单 invocation 串行 256×nseg(1M 域 ≈ 百万次循环)——确定性换出的串行段,`sort_spine_ms` 在门 evidence 单列 measured 登记,禁规模宣传语。

### 4.4 粒子核心(G35-2):SoA 池 ping-pong + 确定性发射 + 半隐式 Euler + 稳定压缩

- **帧序冻结**(F1;= `core.rs::frame` / 已落树 kernel 串联字面):**sim →(flags)→ scan 三 dispatch → particle_compact(A→B)→ emit(写 B 尾部)→ indirect_args**;**新发射粒子帧末入池、下一帧首次积分**(可观察语义:发射帧粒子以初始态出帧);**ping-pong 交换点 = 帧末**(读 A 写 B,swap)。
- **SoA 布局**:粒子流 SoA 各占独立 SSBO——pos_x/y/z、vel_x/y/z、age、life(f32);pid、alive_flags(u32);压缩 = ping-pong 双组(禁原地压缩)。
- **确定性发射**:每帧发射配额 = emit_curve 累计的 floor 差分(纯整数);**容量钳制语义冻结**(F7):`accepted = min(requested, cap − alive_total)`,pid 只为 accepted 递增,rejected 计数进 evidence 登记面(零随机丢弃——被拒是确定性钳制非随机;host `frame()` 现以容量 assert fail-closed 承载,库面 min 语义与 rejected 登记由 G35-2 probe 硬断言承载);新粒子槽位 = 压缩后尾部连续区 `alive_total + j`(下标序);**persistent ID** = pid_base + j 单调分配,全生命周期不变;初始属性随机数一律经随机带(slot 表:0/1/2 = pos,3/4/5 = vel,6 = life——`emit_step` 字面)。
- **pid 硬域冻结**(F6):**pid ∈ [0, 2^24)**——kernel 参数面 f32 传 pid_base 的精确可表达域(`emit_step` 断言字面 `pid_base + emit_count < 2^24`);域耗尽(累计发射逼近 2^24)= **fail-closed typed Err,禁静默回绕**;epoch 扩宽方案(pid 高位纪元段/u64 通道)登记为收口前重判项(§9 Q5,零新 RD——理由见 Q5)。
- **半隐式 Euler 积分**:运算序逐字冻结(`sim_step` 字面):`vy += g·dt; px += vx·dt; py += vy·dt(消费更新后 vy); pz += vz·dt; age += dt; flags = (age < life)`(f32 固定运算序,per-invocation 独立,零跨粒子交互——流体波例外见 §4.10;drag v1 恒 0 登记)。
- **稳定压缩(禁原子抢槽)**:alive_flags 经 scan 三 kernel 推导目标槽位 → 逐粒子搬运到 ping-pong 对侧;存活粒子相对序恒 = 下标序。
- device 面 = `kernels/g35_emit.rx / g35_sim.rx / g35_particle_compact.rx / g35_indirect_args.rx`;host 金标准 = `particles/core.rs`。

### 4.5 indirect args 零回读(规模轴)

存活总数 = scan spine 总和槽 seg_offsets[nseg](§4.2 kernel 2 直写)→ `g35_indirect_args.rx` 单 invocation 合成 `total = seg_offsets[nseg] + emit_count` → 渲染/下帧 sim 经 indirect 消费,**host 全程零回读**(host 平行推得 n_next 只对拍验证不读回;对照:Niagara GPU sim 动态 bounds 需回读故禁用)。

**args 缓冲布局冻结**(F3;= 已落树 `g35_indirect_args.rx` / `core.rs::indirect_args` 字面):

- **u32 × 8**:`[0..3) = dispatch {groupCountX = total, 1, 1}`——消费端 sim/粒子 kernel **LocalSize(1,1,1)** 语义,groups = total,一粒子一 invocation(「ceil(alive/SEG) workgroup」旧表述作废);`[3..7) = draw {vertexCount = 6·total, instanceCount = 1, firstVertex = 0, firstInstance = 0}`;`[7] = total`(meta 槽,零回读链 host 对拍面)。
- **消费面冻结**:`DispatchSpec::Indirect{res, offset = 0}`(VkDispatchIndirectCommand 12 字节 = 槽 [0..3));`DrawSpec::Indirect offset = 12 字节`(VkDrawIndirectCommand 16 字节 = 槽 [3..7));args 缓冲 `usage.indirect = true`;args write→**INDIRECT_COMMAND_READ** barrier 转换归 §4.2 同步契约生产面。
- 域前提:total ≤ cap ≤ 2^20 ⇒ 6·total < 2^32 无回绕(kernel 头字面)。

粒子 AABB/bounds 本波取 emitter 声明的保守包围盒(fixed bounds 语义,GPU 动态 bounds 归评估窗,不预支)。

### 4.6 渲染接线(G35-3):billboard splat + 软粒子 + 粒子 MV

- **billboard splat 进 G34 统一车道**:相机朝向四边形(splat)加性 pass,承载 = g34_full_lane 独立 include 区段(G34-2/G34-3 并行分区纪律同律,主 bin 仅旗标解析 + 挂点);粒子面缺省关,`--particles off` 缺省面 == 母版 Stage A 锚位级一致(加性 0-byte 机器证明)。
- **软粒子**:splat 片元对场景深度做 depth fade(fade = clamp((scene_d − particle_d)/fade_range, 0, 1)),消除硬相交线;深度源 = g34_unified_shade 谱系 out_depth_hz 本帧真深度(G34-2 同源)。
- **粒子 MV(TODO #81 字面消费;公式冻结,F11)**:`mv = project_curr(pos) − project_prev(pos − vel·dt)`(前帧位置 = pos − vel·dt 重构,相机项经 prev VP 自然含,billboard 朝向变化不入 MV——逐粒子中心速度语义);**像素归属 = u64 max 赢家**:`(depth24 << 40) | pid24` 打包取 max(近者胜、同深 pid 大者胜——确定序;`visbuffer_sw_u64.rx` 语料先例,u64 位运算域可用);**半透明重叠像素无唯一表面速度**——纯加性像素保留相机 MV,reactive mask 通道登记为 deferred(不预支);接 g31/g34 MV 谱系三类速度口径(类 1 相机/类 2 刚性实例/类 3 蒙皮,B5 在案)之上的粒子类加性扩展(g34_unified_mv.rx 谱系)。**门口径**(禁「零鬼影」宣传语):粒子 MV 与 host 投影对拍 ≤2px(G34-3 三类速度口径先例)+ TSR 互操作 digest 判别 = G-G35-3 `particle_mv_parity_2px` fact(#80 运动模糊依赖此 MV 完整性,依赖登记不兑现)。
- **加性(additive)splat 累加确定性**(F5 同律):加性混合像素累加走**定点整数累加**(§4.8 臂 B 同一 Q 格式/舍入/饱和语义)——浮点加法非结合且片元到达序不受 scan 稳定序保护,浮点累加不承诺位级。

### 4.7 mesh 粒子 TLAS 臂(光追集成轴,🔒 FIF 互斥)

- mesh 粒子(实例网格粒子)以 **TLAS 实例**进场景加速结构:逐帧实例变换更新走既有 `tlas_update` refit 通路(G34-2 双 TLAS/G34-3 角色 BLAS 逐帧 refit 同族);mesh 粒子收光追阴影/GI(ray query 命中即着色,零 billboard 近似)。
- **FIF 互斥(inflight=1)**:FIF 流水面拒 `tlas_update`/`blas_refit`(G34-3/G31 A2 现约束字面)⇒ mesh 粒子 TLAS 臂走顺序入口 inflight=1;FIF×动态共存归 TODO #90 评估窗,本 RFC 不解(§9 Q3)。
- 实例预算:mesh 粒子实例数上限随 G35-3 交付以 emitter 声明冻结(TLAS 实例粒度成本远高于 splat,百万级规模轴由 billboard 承载,mesh 粒子为中低数量高保真档)。

### 4.8 半透明双臂(G35-4):深度排序 + WBOIT

- **臂 A 排序臂(位级确定基准)**:反键(= DEPTH_KEY_MAX − depth_key24,§4.3 冻结)radix 3-pass 稳定排序 → **排序后固定序合成**(back-to-front alpha blend,合成序 = 排序输出序,段内串行/固定序语义)= **位级确定基准臂**(🔒 执行器 alpha blend 状态面扩展 = rurix-rt 库面加性管线状态,零 RXS,§9 Q2);稳定序保证同深度粒子序恒定。**近远两粒子顺序见证**进门 facts(G-G35-4 `near_far_order_witness`:构造近远两粒子,合成结果证远者先画)。
- **臂 B WBOIT 臂(定点整数累加,F5)**(McGuire & Bavoil 2013 公式面):**撤销浮点加权和位级承诺**——浮点加法非结合、片元到达序不受 scan 稳定序保护;改为**定点整数累加**:权重×颜色量化为 u32 定点(**Q 格式随 G35-4 交付冻结;舍入 = floor;饱和 = clamp 到 u32::MAX 语义冻结**),整数加法可交换可结合 ⇒ 累加结果与片元到达序无关 ⇒ **双跑位级确定**;归一化 pass 消费定点和;权函数数值进 evidence 不进硬门;饱和触发计数如实登记(G-G35-4 `wboit_fixedpoint_saturation` fact)。
- **双臂对拍**:同场景同 emitter 双臂各自双跑位级(G-G35-4 `sorted_arm_bitexact` + `wboit_fixedpoint_saturation`)+ 视觉差 measured 登记(双臂语义不同不设互拍容差硬门);**#13 OIT 评估窗 re-trigger**:粒子特效进画面 = 触发条件字面命中(milestones/g31/g31_oit_evaluation_window.json conditional_wiring_sketch ①:消费 M120 冻结测量数据启动有界近似档「WBOIT 起步」选型提交)——本波 WBOIT 臂即其兑现载体,选型提交须引 M120 benchmark 数据(无数据提交判 RED,选型纪律字面维持);精确 linked-list 档维持毛发 strand 唯一作用域(RXS-0371 L4,不扩)。

### 4.9 碰撞与力场(G35-5)

- **ray query 同帧碰撞**:sim kernel 内 ray query(compute RayQuery 既有语义面 RXS-0297~0300 引用不扩)沿 `pos → pos + vel·dt` 段查询 TLAS,命中即本帧反弹/停驻(碰撞响应 = 反射衰减,bounce 系数 emitter 声明)——**同帧生效**,vs Niagara GPU ray traced collisions 异步一帧延迟。
- **深度缓冲对照臂**:屏幕空间深度碰撞(Niagara GPU depth buffer collisions 同类)作对照臂如实登记两臂行为差(屏外/遮挡区失效为深度臂固有缺陷,不设互拍硬门)。碰撞臂三档显式降级链 ray_query → depth_buffer → off = §4.13 能力降级链字面(fail-closed 禁静默换臂,F12)。
- **力场复用 rurix-physics field**:力场求值走 RFC-0024 Field 系统通道(host 装配期把 field 参数编译进参数块,device 端解析式求值,零新物理语义);粒子↔物理双向数据面归 G35-6 particle_view 桥。

### 4.10 流体(G35-7):count-sort 空间哈希邻居 + XPBD

- **空间哈希邻居(确定序;三阶段冻结,F14)**:**与 radix 排序同形的分段三阶段**——① 段局部直方图(每段写自有直方图行,**零跨 workgroup 竞态**,无合并原子)→ ② 单线程 spine 固定序合并(段序×cell 序串行 exclusive scan,§4.2 spine 同律)→ ③ 段内串行 scatter 产 cell-major 粒子序,**零原子**(禁原子邻居表,确定性协议同律)。**cell id 语义冻结**:`cell = floor(p / cell_size)` 逐轴(f32 floor 冻结——负坐标向负无穷取整,floor-division 语义显式)+ **世界界 clamp 到边界 cell**(越界粒子计数如实登记);v1 域 = **密集 cell 网格(界内)**,稀疏哈希/开放寻址登记评估窗不实现;邻居遍历 = 27 cell 固定序。
- **XPBD 约束求解**(Macklin & Müller PBF 2013 / Macklin et al. FleX 2014 谱系):密度约束(PBF)+ 距离约束,固定迭代次数(emitter 声明,禁自适应早停——确定性),Jacobi 式并行(读上迭代写本迭代,ping-pong,禁 Gauss-Seidel 原子竞写);对标 Niagara Fluids。
- **MPM 评估窗**:MLS-MPM(Hu et al. 2018)登记评估窗不承诺(G2P/P2G 散射需原子或图着色,与确定性协议冲突待裁,镜像 M120 device 帧时腿「atomics 与确定性协议冲突待裁决」注记先例)。

### 4.11 作者面与 SDK(G35-8)

- **声明式 emitter 资产**:v1 最小字段闭集已随本 RFC 冻结(§3 十字段,F17;加性扩展走 schema MINOR 修订非 stable);host 装配期编译为**参数化 megakernel 参数块**(f32 SSBO 表驱动,禁逐 emitter 重编 kernel——PSO 数量恒定,#112 permutation 治理同向);资产 digest 确定性(同资产同字节)。
- **热重载**:参数块重上传(结构不变),池状态保留;结构性变更(capacity/渲染档)走车道重建如实登记。
- **SDK C ABI 最小签名闭集冻结**(F13;实现留 G35-8):内部符号面 `rxsdk_*`(u64 句柄 + i32 状态码,与既有 `rurix-renderer-sdk` cdylib 纪律一致——`rxsdk_*` 为内部实现面,用户面经 `apps/g31-renderer-sdk/src/sdk.rx` 转发 + 生成头承载):

```c
int32_t  rxsdk_particles_emitter_create(uint64_t sys, const uint8_t* desc_json, size_t len, uint64_t* out);
int32_t  rxsdk_particles_emitter_set_param(uint64_t h, const uint8_t* key, size_t klen, float value);
int32_t  rxsdk_particles_emitter_destroy(uint64_t h);
int32_t  rxsdk_particles_stats(uint64_t sys, uint64_t* out);
```

  线程性 = **单线程 apartment**(与现 SDK 同);句柄所有权 = create 产/destroy 收,悬空句柄 → 状态码错(fail-closed);状态码域 = i32(0 = OK,负值错误闭集随交付冻结);导出面演进 = **ABI MINOR bump + 生成头再生 + stable snapshot 重 bless** 纪律逐字引用 [apps/g31-renderer-sdk/API_VERSIONING.md](../apps/g31-renderer-sdk/API_VERSIONING.md) §2(既有导出集 0-byte,加性只增不破坏)。
- **事件与 particle_view 双向桥(G35-6)**:事件数据通道 = 有界 SSBO 队列 + scan 推导写槽(禁原子);**溢出语义冻结**(F15):按生产者稳定序键 `(producer_pid, slot)` 经 scan 裁剪,**保留前 capacity 项**(确定集——与线程调度无关)+ overflow 计数如实登记(禁静默丢);门对拍具体 payload 集(G-G35-6 `event_overflow_payload_stable` fact);particle_view = GPU↔host 双向桥(host 读粒子快照/写外部驱动源),消费 RFC-0024 统一 particle view 语义面引用不扩。

### 4.12 确定性回放回滚(G35-9)

- **journal**:逐帧输入记录(dt/相机/emitter 参数变更/外部事件)+ 初始 seed;回放 = 同 journal 重放,粒子面输出 digest 逐帧**位级一致**(同 GPU 同驱动,§9 Q4)。
- **回滚**:journal 截断至帧 k + 从初始态(或最近快照)重放至 k——重仿真式回滚(快照 = SoA 池全量拷贝,快照间隔 measured 权衡登记);零「近似回滚」(插值回滚被否,语义不确定)。
- digest 协议:粒子面独立 digest 锚(不混入生产 Stage A 锚表,RFC-0048 §4.6 witness 车道同律);Stage A 既有锚 0-byte。

### 4.13 🔒 能力协商与降级链(F12;fail-closed,禁静默换臂)

- **particles 主链**:`gpu_particles`(依赖 = compute/SSBO kernel 波能力〔W1/W2 kernel 族现能力面〕+ 逐 pass barrier/sync 面〔§4.2 生产面〕+ indirect 消费〔buffer usage.indirect,§4.5〕)→ `off`——能力缺失 = 装配期确定性 typed Err 或显式 off 档,**禁静默换臂/禁静默降质**(capability_matrix 六链 fail-closed 同律)。
- **碰撞臂链(显式三档)**:`ray_query`(TLAS + rayQuery 能力,W3/G35-5 面)→ `depth_buffer`(屏幕空间深度,零 RT 能力)→ `off`;档间切换 = 显式请求档裁决(reason 登记),禁静默 fallback(两臂行为差在案,§4.9)。
- **半透明双臂**:排序臂/WBOIT 臂均 compute + 图形 blend 面,**零新能力需求**(blend 为基线图形能力)。
- **capability_matrix 七链扩展**:现 [src/rurix-render/src/capability_matrix.rs](../src/rurix-render/src/capability_matrix.rs) 为六链冻结序——particles 链作为第七链的代码面扩展**随 G35-5/收口批实现落地登记**(本 RFC 只冻结链语义与档序,不动 src)。

**零新 RXS 条款(声明)**:GPU 粒子系统为**渲染器引擎库面**——「语言不内置 render graph/ECS——它们是库」(06 §8.3),粒子系统同律;G5(渲染器,RFC-0016「预期零新语言语义条款」)/ G6(物理库,RFC-0017 同律)/ G31/G34(波次/合流期 rfc_required「零新 RXS 条款」)先例一贯。九波 kernel 全用现有语言面:compute kernel 子语言(SSBO ViewMut/只读表/整数除模/f32 算术既有面)+ compute RayQuery(RXS-0297~0300 谱系,§4.9 引用不扩)+ 图形 pass 既有面(splat/blend 为 rurix-rt 库层管线状态,非语言面,§9 Q2);零新语法/类型/内建。确需新语言语义时(如 MPM 评估窗触发原子语义需求)按当时 ledger 实测 next_free 顺位另起 RFC/修订行,本 RFC 不预占。

- **错误码策略**:**零新 RX 码**——粒子库面违例(capacity 非 SEG 整倍数/emitter schema 违例/事件队列溢出策略违例等)走库层 typed Err(镜像 RX6029/6030 口径);编译期面复用既有诊断;确需升档按实现 commit 实测 next_free 顺位,registry/error_codes.json 只追加 + en/zh message-key 成对,不预留、不预造。

## 6. feature gate / tracking / 实现序(10 §3 要件)

- **feature gate**:零新增——`rurix-render` 的 `vulkan` feature 既有(device 腿随其生效);粒子面运行期旗标 `--particles on|off`(缺省 off,缺省面位级锚 guardrail)。
- **门 facts 闭集规范引用**(F16):九门 facts 闭集**唯一事实源 = [milestones/g35/G35_CONTRACT.md](../milestones/g35/G35_CONTRACT.md) front matter `acceptance_gates`**(G-G35-1~G-G35-10;本节与 §4 不复述判据)——含 v0.2 评审增补 facts:G-G35-3 `barrier_plan_audit`/`particle_mv_parity_2px`(§4.2/§4.6)、G-G35-4 `sorted_arm_bitexact`/`wboit_fixedpoint_saturation`/`near_far_order_witness`(§4.8)、G-G35-5 `fallback_chain_explicit`(§4.13)、G-G35-6 `event_overflow_payload_stable`(§4.11)、G-G35-7 `hash_cell_floor_semantics`(§4.10)、G-G35-10 `capability_chain_registered`(§4.13)。
- **九波实现序**(栈式,均门控于本 RFC 对抗评审 + 各波门;门 key/脚本冻结 = milestones/g35/CI_GATES.md,判据字面 = 各 smoke docstring 随波交付冻结):
  1. **G35-1 基元**:radix sort(hist/spine/scatter)+ compact_u32 + `bin/g35_primitives_device.rs` + `ci/g35_primitives_smoke.py`(scan 三 kernel 已落树为其上游);
  2. **G35-2 粒子核心**:emit/sim/particle_compact/indirect_args 四 kernel + `bin/g35_particle_core_device.rs` + `ci/g35_particle_core_smoke.py`;
  3. **G35-3 渲染接线**:splat/MV kernel + g34_full_lane 独立 include 区段 + mesh 粒子 TLAS 臂 + `ci/g35_render_wiring_smoke.py`;
  4. **G35-4 半透明**:排序臂(alpha blend 执行器加性)+ WBOIT 臂 + `ci/g35_sort_oit_smoke.py` + #13 评估窗 re-trigger 登记;
  5. **G35-5 碰撞力场**:ray query 同帧碰撞 + 深度对照臂 + field 复用 + `ci/g35_collision_smoke.py`;
  6. **G35-6 事件通道**:有界事件队列 + particle_view 双向桥 + `ci/g35_events_smoke.py`;
  7. **G35-7 流体**:count-sort 哈希 + XPBD + `ci/g35_fluids_smoke.py` + MPM 评估窗登记;
  8. **G35-8 作者面**:emitter 资产 + megakernel 参数化 + 热重载 + SDK 加性 + `ci/g35_authoring_smoke.py`;
  9. **G35-9 回放**:journal 回放回滚 + `ci/g35_replay_smoke.py` + 收口(G-G35-10)。
- **真实红绿**(反 YAML-only):各门 selftest 判读器构造缺陷 → 红 → 复原 → 绿;device 腿 RURIX_REQUIRE_REAL=1 翻硬红;三态 DEV_ENV_DEGRADE 如实登记不冒充 PASS。

## 7. 备选方案

1. **Niagara 式原子抢槽(atomicAdd 领槽发射/压缩)**:否决——槽位序 = 线程调度序,非确定(Niagara GPU sim 非确定的根因之一);本系统确定性轴为立项第一目标,scan 推导槽位的额外 dispatch 成本以 measured 登记换确定性承诺。
2. **GPU 单 pass scan(decoupled-lookback)/ Onesweep 单 pass radix sort**:否决为生产形态——两者依赖跨 workgroup 前进保证(后启动 workgroup 等待先启动者发布聚合值),Vulkan 规范**不提供 forward progress guarantee**(事实性可跑 ≠ 可承诺);判为评估窗登记(§9 Q1),保守分段臂(三 kernel/3-pass)为生产形态。
3. **AoS 粒子布局 + 结构体数组**:否决——SoA 独立 SSBO 每属性一缓冲,压缩/排序按属性流 scatter 带宽最优且 kernel 语言面零 struct 布局依赖(host/device 对拍逐流位级可判)。
4. **CPU 粒子车道(Niagara CPU sim 对位)**:否决入范围(§8)——本期确定性 GPU sim 已覆盖确定性需求(Niagara 以 CPU sim 换确定性的理由在本系统不成立),host 金标准层即 CPU 参照(对拍面,非生产车道)。
5. **粒子专用 render graph 节点系统(Niagara 式图编辑语义)**:否决——声明式资产 + 参数化 megakernel(§4.11)覆盖数据驱动需求,图编辑 GUI 为编辑器面(§8);graph 语义扩张归 06 §8.3 库面纪律。

## 8. 不做(范围红线)

- **CPU 粒子生产车道**(host 金标准 = 对拍参照,非生产形态)。
- **编辑器 GUI**(emitter 资产为文本声明式;可视化编辑器归产品面,不在渲染器库范围)。
- **跨硬件位级承诺**(§9 Q4:同 GPU 同驱动位级;跨硬件 = 协议一致 + 整数流可期位级,f32 流不承诺)。
- **Work Graphs GPU 侧调度**(TODO #40 not-available 实测维持,WG present 翻转时复评,不重开)。
- **MPM 生产化**(评估窗登记,§4.10)/ **GPU 动态 bounds**(评估窗,§4.5)/ **Onesweep/decoupled-lookback 生产化**(评估窗,§9 Q1)/ **FIF×动态共存**(TODO #90,§9 Q3)。
- **G13~G34 冻结注册表/锚改写**(0-byte);生产管线既有 pass 与 Stage A 锚 0-byte(粒子面全加性)。

## 9. 未决问题 / 关键裁决

| # | 问题 | 裁决 |
|---|---|---|
| Q1 | Onesweep(Adinets & Merrill 2022)/ decoupled-lookback(Merrill & Garland 2016)单 pass 基元是否进生产? | **评估窗登记,保守分段臂为生产形态**——两者依赖跨 workgroup 前进保证,Vulkan 规范不提供(本机事实性可跑不构成承诺面);实验臂 `g35_sort_onesweep.rx` 收益 measured 登记后,进生产须独立修订行重审(判据:收益 ≥2× 且 soak 域零挂起证据) |
| Q2 | 半透明 alpha blend 需要执行器状态面扩展,是否触语言面新条款? | **rurix-rt 库面零 RXS**——blend 为 host 侧图形管线状态(VkPipelineColorBlendState 装配参数),kernel 子语言零新语义;执行器加性扩展走库面 API(既有 render_exec 谱系),spec 面引用不扩 |
| Q3 | mesh 粒子 TLAS 臂与 FIF 流水如何共存? | **FIF 互斥,inflight=1**——FIF 流水面拒 tlas_update/blas_refit 为现约束字面(G34-3/G31 A2 同律);mesh 粒子臂走顺序入口;FIF×动态共存(每槽实例缓冲/BLAS)= TODO #90 评估窗,本 RFC 不解不预支 |
| Q4 | 确定性承诺的诚实边界? | **同 GPU 同驱动位级、跨硬件协议一致**——device 双跑位级一致 + 回放 digest 位级 = 同 GPU 同驱动承诺;跨硬件:整数流(scan/sort/槽位/pid)协议上位级同值可期,f32 流(FMA 收缩/舍入实现差)不承诺位级(host↔device 对拍走标定容差同理);cross_hardware_bitexact_promise 显式 out_of_scope(契约字面) |
| Q5(v0.2,F6) | pid 硬域 [0, 2^24) 耗尽后如何扩宽? | **收口前重判项登记,零新 RD**——v1 语义完备:域耗尽 = fail-closed typed Err 禁静默回绕(§4.4);epoch 扩宽(pid 高位纪元段/u64 通道)为条件性未来需求,触发锚 = 真实 emitter 配置累计发射逼近 2^24;不预造 RD 的理由 = number_ledger reserved_in_flight[G35].RD claim 字面「不为已知评估窗预造 RD」+ G14 先例「不为已知 out-of-scope 预造条目」;G-G35-10 收口批重判本行(维持/立项二态均合法) |
| Q6(v0.2,F8) | 随机带 65536 周期(pid+65536 属性克隆)长周期方案? | **评估窗登记不实现**——v1 域声明 = 单 emitter 活跃 pid 窗 < 65536 无克隆(§4.1 如实登记);长周期 counter-hash(u64 位运算域可用,visbuffer_sw_u64.rx 语料先例)与随机带单源纪律的相容性评估归 G35-6 窗或收口前重判,收益/需求 measured 举证后按契约修订程序启动 |

## 9.1 对抗性评审记录(对抗性评审要求,10 §3 / §7 · [`../13_DECISION_LOG.md`](../13_DECISION_LOG.md) D-409)

| 字段 | 值 |
|---|---|
| 评审者 provenance | `Assisted-by: cursor:gpt-5.6-sol-medium`(**≠ 起草 `Assisted-by: cursor:claude-fable-5`**——跨模型评审,D-409 首选形态合规,零偏差登记需求) |
| 评审轮次 | 第 1 轮,2026-08-27 |
| 评审形态 | 主控另派独立 provenance agent 对 Draft v0.1 全文对抗评审;17 findings(5 blocker + 11 high + 1 med)经主控逐条判读**全部采纳**;v0.2 修法批(本批)逐条修订并回填本表 |

**Findings 与 disposition**(每条一行;disposition 二选一:**采纳并修** §X ／ **驳回** + 理由):

| # | Finding(评审者提出) | 严重度 | Disposition |
|---|---|---|---|
| F1 | §1 图示 emit→sim→compact→indirect 与冻结实现 core.rs 的 sim→scan→compact→emit→indirect_args 帧序冲突(新粒子当帧积分 vs 下帧积分,可观察语义) | blocker | **采纳并修** §1/§2/§4.4/字段表:帧序以冻结代码为准 = sim→scan(3 dispatch)→compact→emit→indirect_args;「新发射粒子帧末入池、下一帧首次积分」可观察语义冻结;ping-pong 交换点 = 帧末 |
| F2 | §4.2 未定义跨 dispatch 可见性(seg_sums/seg_offsets/out_scan 的 shader-write→shader-read;args 的 write→indirect-read);render_exec 实际要求逐 pass barrier plan | blocker | **采纳并修** §4.2 增「同步契约」:probe 面 = vk::run_compute 每 dispatch 独立 submit+wait(提交边界保证可见性);生产面 = DeviceFrameSession 逐 pass barrier plan〔(资源,TargetState) 转换表 + args write→INDIRECT_COMMAND_READ〕;G-G35-3 增 barrier_plan_audit fact(契约 v1.1 同批) |
| F3 | §4.5「workgroup=ceil(alive/SEG)」与冻结 g35_indirect_args.rx 的 args[0]=total(LocalSize=1)冲突;VkDispatchIndirectCommand 12B 布局/offset=0/draw 16B@offset12 未冻结 | blocker | **采纳并修** §4.5:冻结 u32×8 布局 = [0..3) dispatch{total,1,1}〔LocalSize(1,1,1),一粒子一 invocation〕/ [3..7) draw{6·total,1,0,0} / [7] total meta;DispatchSpec::Indirect offset=0、DrawSpec::Indirect offset=12、usage.indirect=true;「ceil(alive/SEG)」表述作废删除 |
| F4 | §4.3 digit 升序=front-to-back,§4.8 却声称 back-to-front;未定义反键或逆序消费 | high | **采纳并修** §4.3/§4.8:冻结半透明排序键 = DEPTH_KEY_MAX − depth_key24(d,d_max)(反键,radix 升序即 back-to-front);不透明/加性臂不排序;G-G35-4 增 near_far_order_witness fact |
| F5 | §4.8 WBOIT 浮点加权和与「双跑位级」冲突(浮点加法非结合、片元到达序不受 scan 稳定序保护);§4.6 加性 pass 无确定性累加协议 | blocker | **采纳并修** §4.8/§4.6:撤销 WBOIT 浮点位级承诺;臂 A = 排序后固定序合成(位级确定基准),臂 B = WBOIT 定点整数累加(Q 格式交付冻结/舍入 floor/饱和 clamp u32::MAX;整数加法可交换 ⇒ 与到达序无关位级确定);加性 splat 同律定点累加;G-G35-4 增 sorted_arm_bitexact + wboit_fixedpoint_saturation facts |
| F6 | §4.4 pid u32「<2^32 域」与冻结实现 f32 params 传 pid_base 的 <2^24 可表达域冲突(大 256 倍);soak 不能定义回绕行为 | blocker | **采纳并修** §4.4:pid 硬域 = [0, 2^24)(emit_step 断言字面);耗尽 = fail-closed typed Err 禁静默回绕;epoch 扩宽登记 §9 Q5 收口前重判项(零新 RD,理由 = ledger G35 claim「不为已知评估窗预造 RD」字面) |
| F7 | §4.4「零随机丢弃」未定义 requested > cap−alive_total 时行为;host 实现仅 assert | high | **采纳并修** §4.4:冻结 accepted = min(requested, cap − alive_total);pid 只为 accepted 递增;rejected 计数进 evidence 登记面;host frame() 容量 assert fail-closed 承载 + 库面 min 语义由 G35-2 probe 硬断言承载(契约 G-G35-2 行文注明,FACT_IDS 闭集 0-byte) |
| F8 | §4.1 RAND_K=7919 去相关声明不成立:模 65536 下 pid 与 pid+65536 全 slot 相同(属性克隆) | high | **采纳并修** §4.1:去相关声明修正为 slot 通道间去相关;65536 周期与 pid+65536 克隆风险如实登记;v1 域声明 = 单 emitter 活跃 pid 窗 <65536 无克隆;长周期 counter-hash(u64 域,visbuffer_sw_u64.rx 先例)登记评估窗 §9 Q6 |
| F9 | §4.3 键域 ≥2^24 时三 pass 静默截断(host 仅 debug_assert);NaN/Inf depth 与非正 d_max 转换未定义 | high | **采纳并修** §4.3:冻结 depth_key24 边界语义(非有限/负 depth 经 clamp 归 0 端 ⇒ 键 0;d_max≤0 → 0;构造域保证 ≤2^24−1 = 生产键唯一来源);host 键域检查升级 release 硬检查(G35-1 波代码兑现,当前 debug_assert 差距如实登记);device 高位截断行为如实登记 |
| F10 | §4.2/§4.3 spine 单线程串行(scan 4096 段;sort 每 pass 256×nseg≈百万次)未登记为规模瓶颈,摘要却宣传「百万级」 | high | **采纳并修** §1/§4.2/§4.3:措辞改「容量可寻址百万(PARTICLE_CAP_MAX=1048576),性能以 measured 为准」;spine 串行段登记已知瓶颈,scan_spine_ms/sort_spine_ms 单列 evidence 登记;禁规模宣传语 |
| F11 | §4.6 粒子 MV 公式不完整(缺相机项/前帧位置/billboard 变化;半透明重叠像素无唯一表面速度;无 coverage/reactive mask);§3「零鬼影」无门口径 | high | **采纳并修** §4.6/§3/§1:冻结 mv = project_curr(pos) − project_prev(pos − vel·dt)(相机项经 prev VP 自然含);像素归属 = u64 max 赢家(depth24<<40 pid24 打包,visbuffer_sw_u64 先例);纯加性像素保留相机 MV + reactive mask 登记 deferred;「零鬼影」改门口径 = MV 对拍 ≤2px + TSR digest 判别(G-G35-3 particle_mv_parity_2px fact) |
| F12 | 无能力协商/降级链(ray query/TLAS/indirect/blend 缺失时行为未定义) | high | **采纳并修** 新增 §4.13:particles 链 gpu_particles→off fail-closed 禁静默换臂;碰撞臂 ray_query→depth_buffer→off 显式三档;WBOIT/排序臂零新能力;capability_matrix 第七链扩展随 G35-5/收口批实现登记;G-G35-5 增 fallback_chain_explicit、G-G35-10 增 capability_chain_registered facts |
| F13 | §4.11 SDK 缺函数签名/句柄所有权/线程性/状态码/ABI MINOR bump 与 snapshot 纪律 | high | **采纳并修** §4.11:冻结 rxsdk_particles_{emitter_create,emitter_set_param,emitter_destroy,stats} 四函数最小签名闭集(u64 句柄/i32 状态码);单线程 apartment;句柄 create/destroy 所有权 + 悬空 fail-closed;MINOR bump + 生成头 + stable snapshot bless 纪律引用 API_VERSIONING.md §2;实现留 G35-8 |
| F14 | §4.10 count-sort 直方图跨 workgroup 合并未解释无竞态构建;负坐标 floor-division 语义缺失 | high | **采纳并修** §4.10:冻结空间哈希 = 与 radix 同形分段三阶段(段局部直方图行零跨组竞态 → 单线程 spine 固定序合并 → 段内串行 scatter,零原子);cell = floor(p/cell_size) 逐轴(负坐标向负无穷取整显式)+ 世界界 clamp 边界 cell(越界如实登记);v1 = 密集网格域,稀疏哈希评估窗;G-G35-7 增 hash_cell_floor_semantics fact |
| F15 | §4.11 事件队列溢出时保留集未定义(并发生产者下 payload 集不确定) | med | **采纳并修** §4.11:冻结溢出语义 = 生产者稳定序 (producer_pid, slot) 键 scan 裁剪保留前 capacity 项(确定集)+ overflow 计数;门对拍具体 payload 集(G-G35-6 event_overflow_payload_stable fact) |
| F16 | §6 未规范引用 G35_CONTRACT acceptance_gates facts 闭集;现有 facts 未覆盖屏障/容量/pid 域/随机周期/透明 MV/降级链 | high | **采纳并修** §6 + 契约 v1.1 同批:§6 增规范引用「九门 facts 闭集唯一事实源 = G35_CONTRACT front matter acceptance_gates」;契约 G-G35-3/4/5/6/7/10 增补八 facts(F2/F4/F5/F11/F12/F14/F15 对应);G-G35-1/G-G35-2 facts 与已落树 W1/W2 smoke FACT_IDS 字面同步(smoke/schema 冻结互核为准);容量/pid 域语义由 G35-2 probe 硬断言承载注明 |
| F17 | §4 精确度不足以批准稳定语义(状态机/资源布局/同步/ABI 推迟到交付期;§3 schema 标「拟」) | high | **采纳并修** §3/§4/字段表:emitter 资产 v1 最小字段闭集冻结(name/pos/spread/vel_base/vel_spread/life_base/gravity_y/emit_curve〔常数\|阶梯〕/render〔billboard\|mesh〕/blend〔additive\|alpha〕,「拟」标撤销);§4 经 F1~F15 修订达精确设计要件;批准范围声明 = 协议面(帧序/args 布局/排序反键/pid 域/裁剪语义/ABI 签名)冻结,数值调优与 schema 加性扩展非 stable(字段表 Agent 批准行 + §10) |

## 10. 稳定化与 provenance

- **稳定化**(10 §5):本 RFC Agent Approved(2026-08-27)= 语义评审完成;随后九波 gated implementation(各波门 PASS)→ tracking evidence → 至少两个里程碑无重大语义修订 → stabilization report → FCP-lite。**批准即冻结的协议面**(F17):帧序(§4.4)/ indirect args 布局与消费面(§4.5)/ 排序反键与键边界语义(§4.3)/ pid 硬域与钳制语义(§4.4)/ 事件溢出裁剪语义(§4.11)/ 空间哈希三阶段与 cell 语义(§4.10)/ SDK 四函数签名闭集(§4.11)/ emitter 资产 v1 最小字段闭集(§3)。**明确非 stable**:emitter schema 加性扩展、WBOIT Q 格式与权函数数值(交付冻结)、快照间隔、对拍容差数值(标定程序产)、实验臂(Onesweep)全部面、rxsdk_* 内部符号面(API_VERSIONING「非用户 stable ABI」口径)。
- **Provenance**:`Assisted-by: cursor:claude-fable-5`(起草 + v0.2 修法批);对抗评审 `Assisted-by: cursor:gpt-5.6-sol-medium`(§9.1)。agent 自主批准并记录(D-406/D-409)。

## 11. 规范与实现依据

- **GPU 基元**:Adinets, A. & Merrill, D. — *Onesweep: A Faster Least Significant Digit Radix Sort for GPUs*(arXiv:2206.01784,2022;评估窗依据);Merrill, D. & Garland, M. — *Single-pass Parallel Prefix Scan with Decoupled Look-back*(NVIDIA Technical Report NVR-2016-002,2016;评估窗依据——两者的前进保证前提即 §9 Q1 否决生产化的依据)。
- **粒子物理/流体**:Macklin, M. et al. — *Unified Particle Physics for Real-Time Applications*(FleX,SIGGRAPH 2014);Macklin, M. & Müller, M. — *Position Based Fluids*(SIGGRAPH 2013);Hu, Y. et al. — *A Moving Least Squares Material Point Method*(MLS-MPM,SIGGRAPH 2018;评估窗依据)。
- **半透明**:McGuire, M. & Bavoil, L. — *Weighted Blended Order-Independent Transparency*(JCGT 2013)。
- **随机数**:O'Neill, M.E. — *PCG: A Family of Simple Fast Space-Efficient Statistically Good Algorithms for Random Number Generation*(HMC-CS-2014-0905,2014)。
- **对标短板(UE5 Niagara 官方文档)**:Niagara Emitter「Determinism」属性仅 CPU sim 生效(GPU sim 非确定,粒子索引 >64 不稳定);GPU ray traced collisions 异步一帧延迟;GPU sim 动态 bounds 需回读故禁用(fixed bounds 强制)。
- **仓内**:src/rurix-render/src/particles/{mod,scan,primitives,core}.rs(G35-P 冻结契约 v1 + host 金标准)· kernels/g35_scan_{seg_sum,spine,seg_apply}.rx(已过 rurixc + spirv-val)· RFC-0045 §1.2(随机带单源先例)· RFC-0024(Field 系统/统一 particle view)· RFC-0048 §4.6(witness digest 独立锚同律)· milestones/g31/g31_oit_evaluation_window.json(#13 re-trigger 条件字面)· milestones/g34/(统一车道/分区纪律/tlas_update 约束)· G31_PLUS_COMMERCIAL_RENDERER_TODO #13/#80/#81/#90。

---

## 修订记录

| 版本 | 日期 | 变更 | 档位 |
|---|---|---|---|
| Draft v0.1 | 2026-08-27 | AI 起草初版(G35-0 治理批;五轴九波 + G35-P 冻结契约 v1 字面化;§9.1 留待对抗评审批回填) | Full RFC(Draft) |
| Draft v0.2 | 2026-08-27 | D-409 第 1 轮对抗评审(评审 `cursor:gpt-5.6-sol-medium` ≠ 起草)17 findings〔5 blocker + 11 high + 1 med〕全部采纳并修回填:帧序对齐冻结实现(F1)/同步契约(F2)/args 布局冻结(F3)/排序反键(F4)/WBOIT 定点整数累加(F5)/pid 2^24 硬域(F6)/发射钳制(F7)/随机带周期如实登记(F8)/键边界语义(F9)/规模措辞降级(F10)/MV 公式与 2px 门口径(F11)/§4.13 能力降级链(F12)/SDK ABI 签名闭集(F13)/空间哈希三阶段(F14)/事件溢出稳定序裁剪(F15)/§6 规范引用契约 facts(F16)/schema 字段闭集冻结与批准范围声明(F17);§9 增 Q5/Q6;契约 v1.1 同批 | Full RFC(Draft) |
| Agent approval | 2026-08-27 | agent 自主批准全文并记录(D-406;批准范围含 §4.7/§4.13 🔒 子节;协议面冻结,数值调优与 schema 加性扩展非 stable) | Full RFC(Agent Approved) |
