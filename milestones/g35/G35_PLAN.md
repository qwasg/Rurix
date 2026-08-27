<!-- Assisted-by: cursor:claude-fable-5（G35-0 治理文档套件起草批） -->
# G35_PLAN — GPU 粒子系统期执行计划

> 事实源 = [G35_CONTRACT.md](G35_CONTRACT.md)。本文件只作波次视图，不复述判据。

## 1. 期定位

G35 = **GPU 粒子系统期**：在 rurix-render 引擎库面建设确定性 GPU 粒子系统，对标并超越 UE5 Niagara 五轴（① 确定性 GPU 粒子〔Niagara GPU sim 非确定、粒子索引 >64 不稳定〕② 光追集成〔mesh 粒子进 TLAS + ray query 同帧碰撞 vs Niagara RT 碰撞延迟一帧〕③ 规模〔百万级 GPU-driven 零回读 vs Niagara 动态 bounds 禁回读〕④ 流体统一物理〔XPBD/FleX 式，对标 Niagara Fluids〕⑤ 数据驱动作者面〔emitter 资产 + SDK〕）。语义面 = [../../rfcs/0049-gpu-particle-system.md](../../rfcs/0049-gpu-particle-system.md)（Draft，D-409 对抗评审后 Agent Approved 方可收口）；代码契约 = [src/rurix-render/src/particles/mod.rs](../../src/rurix-render/src/particles/mod.rs)（G35-P 冻结契约 v1：SEG=256 分段 / 禁原子抢槽 / 随机带单源 / 整数流零容差·f32 流标定容差 / 24 位深度键；scan 三 kernel + host 金标准 [particles/scan.rs](../../src/rurix-render/src/particles/scan.rs) 已落树并过 rurixc + spirv-val）。上游法定输入 = [../g34/G34_CONTRACT.md](../g34/G34_CONTRACT.md) §8 close-out（统一车道底座；落笔时三门新鲜绿件在案，收口验收批同窗）+ [../g30/g30_campaign_handover_registry.json](../g30/g30_campaign_handover_registry.json) 谱系 + [../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md](../../G31_PLUS_COMMERCIAL_RENDERER_TODO.md) #13/#80/#81（去重消费不重开）+ [../g31/g31_oit_evaluation_window.json](../g31/g31_oit_evaluation_window.json)（not_triggered 的 re-trigger 条件本期命中：粒子特效进画面）。

## 2. 波次

| 波 | 主题 | 交付物 | 门 key | 依赖 |
|---|---|---|---|---|
| G35-1 | GPU 基元：24 位键 3-pass 稳定 radix sort（段直方图 → digit-major spine → 段内串行稳定 scatter）+ compact_u32 | D-G35-1（kernels/g35_sort_{hist,spine,scatter}.rx + g35_compact_u32.rx〔+ 实验臂 g35_sort_onesweep.rx 评估窗〕+ [particles/primitives.rs](../../src/rurix-render/src/particles/primitives.rs) + bin/g35_primitives_device.rs + ci/g35_primitives_smoke.py） | g35.wave1.primitives | scan 三 kernel（已落树，0-byte 消费） |
| G35-2 | 粒子核心：SoA 池 ping-pong + 确定性发射（随机带单源 + persistent ID）+ 半隐式 Euler + 稳定压缩（禁原子抢槽）+ indirect args 零回读 | D-G35-2（kernels/g35_{emit,sim,particle_compact,indirect_args}.rx + [particles/core.rs](../../src/rurix-render/src/particles/core.rs) + bin/g35_particle_core_device.rs + ci/g35_particle_core_smoke.py + f32 容差条目程序产） | g35.wave2.particle_core | G35-1（compact/scan 基元） |
| G35-3 | 渲染接线：billboard splat 进 G34 统一车道 + 软粒子 + 粒子 MV（TODO #81 消费）+ mesh 粒子 TLAS 臂（inflight=1） | D-G35-3（splat/MV kernel + g34_full_lane 粒子独立 include 区段 + ci/g35_render_wiring_smoke.py） | g35.wave3.render | G35-2 + G34 统一车道底座 |
| G35-4 | 半透明：深度排序臂（24 位键 radix + alpha blend 执行器加性）+ WBOIT 臂；#13 OIT 评估窗 re-trigger 消费 | D-G35-4（双臂 + ci/g35_sort_oit_smoke.py + re-trigger 登记件） | g35.wave4.sort_oit | G35-1（排序）+ G35-3（出帧） |
| G35-5 | 碰撞与力场：ray query 同帧碰撞 + 深度缓冲对照臂 + rurix-physics field 复用 | D-G35-5（sim 碰撞腿 + 对照臂 + ci/g35_collision_smoke.py） | g35.wave5.collision | G35-2（sim kernel）；TLAS 面复用 G35-3 臂 |
| G35-6 | 事件数据通道（有界队列，scan 推导写槽）+ particle_view GPU↔host 双向桥 | D-G35-6（事件队列 + 桥 + ci/g35_events_smoke.py） | g35.wave6.events | G35-2 |
| G35-7 | 流体：count-sort 空间哈希邻居 + XPBD 约束求解；MPM 评估窗登记 | D-G35-7（哈希 + XPBD + ci/g35_fluids_smoke.py + MPM 登记件） | g35.wave7.fluids | G35-1（count-sort 用 scan）+ G35-2（池） |
| G35-8 | 作者面与 SDK：声明式 emitter 资产 + 参数化 megakernel + 热重载 + SDK C ABI 加性 | D-G35-8（资产 schema + 参数块 + SDK 导出 + ci/g35_authoring_smoke.py） | g35.wave8.authoring | G35-2~G35-7（被参数化的特性面） |
| G35-9 | 确定性回放回滚 + 收口验收面 | D-G35-9（journal + 回放/回滚 + ci/g35_replay_smoke.py + 独立 digest 锚）；四件套 + 守卫/九门复跑/三锚/soak = G-G35-10 | g35.wave9.replay（收口面 = G-G35-10） | 全部前波 |

评估窗登记项（不占波次不预支，事实源 = CONTRACT §2.2）：Onesweep/decoupled-lookback（RFC-0049 §9 Q1）/ MPM（§4.10）/ GPU 动态 bounds（§4.5）/ FIF×动态共存（TODO #90）/ Work Graphs（#40 维持）。

## 3. 编号纪律

CI 数字步骤零消费声明（九门均 symbolic gate key 未占号，pr-smoke.yml 无 g35 条目；[registry/number_ledger.json](../../registry/number_ledger.json) CI_step.next_free=525 维持——收口验收批实测核验）；RFC 段消费 = RFC-0049 单号（落盘前实测 next_free=49 顺位领取，v1.191 校准同批，Draft 待 D-409 对抗评审）；RXS/RD/U/SG/MR/D/RX_error 共享段零消费（零新 RXS 声明——渲染器是库不进语言 06 §8.3，kernel 全用现有语言面；RD 确需自 46 顺位，评估窗登记不预造 RD）；evidence 前缀闭集九支（分岔分析见 [CI_GATES.md](CI_GATES.md) §3）。
