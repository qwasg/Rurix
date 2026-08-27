//! G35-6 事件/数据通道 host 金标准(Niagara Data Channels 等价物)——门
//! `g35.wave6.events`(RFC-0049 §4.9/评审 F15 修订后基线;契约 = mod.rs
//! G35-P v1 冻结头,事实源 = milestones/g35/G35_CONTRACT.md)。
//!
//! 与 device 两 kernel(`kernels/g35_event_collect.rx` /
//! `kernels/g35_event_spawn.rx`)**逐字同源**;W2 七 kernel(sim/compact/
//! emit/indirect_args + 3 scan)与 host 面([`super::core`]/[`super::scan`])
//! **只消费不修改**。
//!
//! ## G35-6 冻结协议 v1(事件布局/溢出语义/发射次序;改动走契约修订)
//!
//! ### 事件定长布局(32B = 8 词;device 面拆两 SSBO 免位转换)
//! 事件 = `{producer_id u32, slot u32, kind u32, payload[5] f32}`:
//! - meta 流(u32×[`EVENT_META_WORDS`]/事件):
//!   `[e·3+0]=producer_id [e·3+1]=slot [e·3+2]=kind`;
//! - payload 流(f32×[`EVENT_PAYLOAD_WORDS`]/事件):`[e·5 .. e·5+5)`。
//! kind 闭集:[`EVENT_KIND_DEATH`] = 1(GPU 死亡事件)、[`EVENT_KIND_HOST`]
//! = 2(host 合成事件);0 = 空位保留。
//! **死亡事件 payload 布局冻结**:`[0..3) = 死亡帧积分后 pos.xyz`、
//! `[3..5) = vel.xy`(vel_z 不入载荷,发射侧 vel_z 基值 0 + 随机带半幅,
//! 见 [`event_spawn_step`]);`producer_id` = 死亡粒子 pid;`slot` = 死亡槽
//! (death_flags 分段稳定 scan 槽位 = 帧内死亡序,禁原子抢槽)。
//!
//! ### host→GPU 事件队列(Data Channels 等价物)
//! - 容量 [`EVENT_CAP`] = 1024 冻结;[`EventQueue::push`] / [`EventQueue::trim`]
//!   为唯一装配面(host 金标准);每帧整队列上传(meta/payload 两 SSBO,
//!   计数走 kernel 参数面)。
//! - **溢出语义冻结(禁静默丢)**:trim = 按全序键
//!   `(producer_id, slot, kind, payload[0..5).to_bits())` 升序排序(主序 =
//!   (producer_id, slot) 字典序;kind/payload 位型为决胜次键 ⇒ 同集乱序
//!   push 必同果)→ 保留前 EVENT_CAP 项 → 裁剪数如实累计
//!   [`EventQueue::overflow_count`]。
//!
//! ### GPU 事件驱动二次发射(零回读)
//! - [`event_collect_step`](= g35_event_collect 两相):本帧死亡粒子
//!   (sim 后 age ≥ life;池不变量 = 池内粒子上帧皆活)`death_flags[i] =
//!   1 − alive_flags[i]` → **复用 W2 三 scan kernel** 求稳定死亡槽 →
//!   scatter 压入 GPU 事件缓冲(kept = min(death_total, EVENT_CAP),
//!   ev_count 双槽 `[0]=kept [1]=death_total` 如实登记溢出,禁静默丢)。
//! - [`event_spawn_step`](= g35_event_spawn):**下一帧**读 GPU 事件缓冲
//!   计数(SSBO 直读,零回读)+ 当帧 host 事件队列,双源合并发射——
//!   **次序冻结:host 事件先、GPU 死亡事件后**;槽位 = alive_total +
//!   scripted_emit + j;`accepted = min(host_n + gpu_n, cap − alive −
//!   scripted)`(发射上限语义与 core 同律,host 事件优先占预算);
//!   pid 连续递增,pid_base 涵盖三段:`[pid_base, +scripted)` 脚本发射、
//!   `[+scripted, +scripted+host_acc)` host 事件、`[.., +accepted)` GPU
//!   死亡事件;帧末 pid_base += scripted + accepted。
//! - 发射随机仍走随机带单源([`super::rand_table`] 消费律
//!   `r_k = rand_table[(pid·RAND_K + k) % RAND_TABLE_LEN]`,槽位表与 emit
//!   同:0/1/2 = pos xyz,3/4/5 = vel xyz,6 = life)。
//!
//! ### particle_view 双向桥
//! - 方向 A(GPU→host):[`GpuParticleSnapshot`](readback 子集构造)→
//!   物理 crate `particle_view/external_adapter.rs`(plain 数据适配器,
//!   物理 crate 不依赖本 crate 的 device 面);roundtrip 判别 = 逐粒子
//!   位级(反打 Niagara GPU↔CPU 互读静默失败)。
//! - 方向 B(host→GPU):host 事件(v1 演示域 = 合成事件,不真接物理
//!   世界)→ [`EventQueue::push`] → GPU 发射,粒子属性 == host 金标准
//!   (pid/age 整数与零常量位级;pos/vel/life f32 标定容差)。
//!
//! ### 确定性
//! 事件裁剪稳定全序 + 发射槽位 scan 推导 + 随机带单源 ⇒ 固定输入双跑
//! 位级([`event_frame`] 为帧串联 host 单源,probe 平行对拍消费)。

use super::core::{self, EmitterDesc, ParticlePools};
use super::scan;
use super::{RAND_K, RAND_TABLE_LEN};

/// 事件队列容量(冻结;host 队列与 GPU 死亡事件缓冲同容量)。
pub const EVENT_CAP: usize = 1024;
/// 事件 meta 词数(producer_id/slot/kind,u32×3)。
pub const EVENT_META_WORDS: usize = 3;
/// 事件 payload 词数(f32×5)。
pub const EVENT_PAYLOAD_WORDS: usize = 5;
/// kind:GPU 死亡事件(g35_event_collect 产)。
pub const EVENT_KIND_DEATH: u32 = 1;
/// kind:host 合成事件(EventQueue 装配面产;v1 演示域)。
pub const EVENT_KIND_HOST: u32 = 2;

/// 32B 定长事件(布局冻结,模块头注)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ParticleEvent {
    /// 生产者 ID(死亡事件 = 死亡粒子 pid;host 事件 = 装配方自定)。
    pub producer_id: u32,
    /// 槽序(死亡事件 = scan 死亡槽;host 事件 = 装配方队内序)。
    pub slot: u32,
    /// 种类(kind 闭集:1 death / 2 host;0 空位保留)。
    pub kind: u32,
    /// 净荷(死亡事件布局冻结:pos.xyz + vel.xy)。
    pub payload: [f32; 5],
}

impl ParticleEvent {
    /// 溢出裁剪全序键(冻结:主序 (producer_id, slot) 字典序,kind/payload
    /// 位型决胜——同集乱序 push 必同果;f32 走 to_bits,−0/NaN 面强于 ==)。
    pub fn order_key(&self) -> (u32, u32, u32, [u32; 5]) {
        (
            self.producer_id,
            self.slot,
            self.kind,
            self.payload.map(f32::to_bits),
        )
    }

    /// 位级等值(对拍/单测面;PartialEq 的 f32 == 在 −0/NaN 面弱于位级)。
    pub fn bits_eq(&self, other: &Self) -> bool {
        self.order_key() == other.order_key()
    }
}

/// host→GPU 事件队列(装配唯一面;溢出语义冻结,模块头注)。
#[derive(Clone, Debug, Default)]
pub struct EventQueue {
    events: Vec<ParticleEvent>,
    overflow_count: u64,
}

impl EventQueue {
    /// 空队列。
    pub fn new() -> Self {
        Self::default()
    }

    /// 装配面:追加事件(无界暂存;上界由 [`Self::trim`] 执行)。
    pub fn push(&mut self, ev: ParticleEvent) {
        self.events.push(ev);
    }

    /// 溢出语义冻结执行体:按全序键([`ParticleEvent::order_key`])升序
    /// 排序 → 保留前 [`EVENT_CAP`] 项 → 裁剪数累计 overflow_count(如实
    /// 登记,禁静默丢)。≤ 容量时同样规范化排序(同集乱序 push 必同果);
    /// 幂等(二次 trim 0 变化)。上传面([`Self::meta_words`] /
    /// [`Self::payload_words`])消费前必经本面。
    pub fn trim(&mut self) {
        self.events.sort_by_key(ParticleEvent::order_key);
        if self.events.len() > EVENT_CAP {
            self.overflow_count += (self.events.len() - EVENT_CAP) as u64;
            self.events.truncate(EVENT_CAP);
        }
    }

    /// 队列视图(trim 后 = 规范序)。
    pub fn events(&self) -> &[ParticleEvent] {
        &self.events
    }

    /// 队列长度。
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// 累计裁剪数(诚实登记面;evidence 消费)。
    pub fn overflow_count(&self) -> u64 {
        self.overflow_count
    }

    /// 整队列上传面之 meta 流(u32×3×EVENT_CAP,空位 0;须先 trim)。
    pub fn meta_words(&self) -> Vec<u32> {
        assert!(
            self.events.len() <= EVENT_CAP,
            "上传面须先 trim(len {} > EVENT_CAP {EVENT_CAP})",
            self.events.len()
        );
        let mut out = vec![0u32; EVENT_CAP * EVENT_META_WORDS];
        for (e, ev) in self.events.iter().enumerate() {
            out[e * EVENT_META_WORDS] = ev.producer_id;
            out[e * EVENT_META_WORDS + 1] = ev.slot;
            out[e * EVENT_META_WORDS + 2] = ev.kind;
        }
        out
    }

    /// 整队列上传面之 payload 流(f32×5×EVENT_CAP,空位 0.0;须先 trim)。
    pub fn payload_words(&self) -> Vec<f32> {
        assert!(
            self.events.len() <= EVENT_CAP,
            "上传面须先 trim(len {} > EVENT_CAP {EVENT_CAP})",
            self.events.len()
        );
        let mut out = vec![0.0f32; EVENT_CAP * EVENT_PAYLOAD_WORDS];
        for (e, ev) in self.events.iter().enumerate() {
            out[e * EVENT_PAYLOAD_WORDS..e * EVENT_PAYLOAD_WORDS + EVENT_PAYLOAD_WORDS]
                .copy_from_slice(&ev.payload);
        }
        out
    }
}

/// 死亡事件收集产物(= g35_event_collect 两相 + 死亡 scan 中间流,probe
/// 整数零容差对拍面)。
#[derive(Clone, Debug)]
pub struct EventCollectOut {
    /// death_flags = 1 − alive_flags(相 0 产物;len = n)。
    pub death_flags: Vec<u32>,
    /// 死亡槽稳定 scan(W2 三 scan kernel 复用产物;len = n)。
    pub death_scan: Vec<u32>,
    /// 本帧死亡总数(死亡 scan 总和槽)。
    pub death_total: u32,
    /// 入缓冲事件数 = min(death_total, EVENT_CAP)(溢出如实登记)。
    pub kept: u32,
    /// 死亡事件(死亡槽序;len = kept;payload 布局冻结 pos.xyz + vel.xy)。
    pub events: Vec<ParticleEvent>,
}

/// 帧序·死亡事件收集(= kernels/g35_event_collect.rx 两相逐字同源):
/// 消费 sim 后 A 池与存活 flags;death_flags = 1 − flags → 复用冻结
/// [`scan::exclusive_scan_segmented`](= W2 三 scan kernel 串联 host 镜像)
/// 求稳定死亡槽 → 死亡槽 < EVENT_CAP 者压入事件(禁原子抢槽,稳定序 =
/// 粒子下标序)。
pub fn event_collect_step(a: &ParticlePools, alive_flags: &[u32]) -> EventCollectOut {
    let n = a.n;
    assert_eq!(alive_flags.len(), n, "alive_flags 长度必须 = 池 n");
    let death_flags: Vec<u32> = alive_flags.iter().map(|&f| 1 - f).collect();
    let (death_scan, death_total) = scan::exclusive_scan_segmented(&death_flags);
    let kept = (death_total as usize).min(EVENT_CAP) as u32;
    let mut events = Vec::with_capacity(kept as usize);
    for i in 0..n {
        if death_flags[i] != 0 {
            let dst = death_scan[i] as usize;
            if dst < EVENT_CAP {
                events.push(ParticleEvent {
                    producer_id: a.pid[i],
                    slot: dst as u32,
                    kind: EVENT_KIND_DEATH,
                    // payload 布局冻结:死亡帧积分后 pos.xyz + vel.xy。
                    payload: [a.pos_x[i], a.pos_y[i], a.pos_z[i], a.vel_x[i], a.vel_y[i]],
                });
            }
        }
    }
    debug_assert_eq!(events.len(), kept as usize);
    EventCollectOut {
        death_flags,
        death_scan,
        death_total,
        kept,
        events,
    }
}

/// 事件发射参数(位置抖动半幅/初速半幅/寿命基值;probe/单测夹具冻结常量
/// 的类型面,kernel 经参数面消费)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EventSpawnParams {
    /// 位置抖动半幅(px = payload[0] + (r0·2−1)·spread,y/z 槽 1/2 同)。
    pub spread: f32,
    /// 初速半幅(vx = payload[3] + (r3·2−1)·vel_spread;vy 槽 4 同;
    /// vz 基值 0 + 槽 5 半幅)。
    pub vel_spread: f32,
    /// 寿命基值(life = life_base·(0.5 + 0.5·r6))。
    pub life_base: f32,
}

/// 双源发射统计(device spawn_counts 零回读见证槽的 host 平行金标准)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventSpawnStats {
    /// 实收发射数 = min(host_n + gpu_n, cap − alive − scripted)。
    pub accepted_total: u32,
    /// host 事件实收(次序冻结 host 先 ⇒ = min(host_n, accepted_total))。
    pub host_accepted: u32,
    /// GPU 死亡事件实收(= accepted_total − host_accepted)。
    pub gpu_accepted: u32,
}

/// 帧序·双源合并发射(= kernels/g35_event_spawn.rx 逐字同源):次序冻结
/// **host 事件先、GPU 死亡事件后**;槽位 = alive_total + scripted_emit + j;
/// pid = pid_base_events + j(pid_base 三段涵盖,模块头注);accepted =
/// min(host_n + gpu_n, cap − alive − scripted) 与 core 同律。gpu_events
/// 为**上一帧** [`event_collect_step`] 产物(device 面 = GPU 事件缓冲 SSBO
/// 计数直读,零回读)。
#[allow(clippy::too_many_arguments)]
pub fn event_spawn_step(
    b: &mut ParticlePools,
    host_events: &[ParticleEvent],
    gpu_events: &[ParticleEvent],
    rand_table: &[f32],
    sp: &EventSpawnParams,
    pid_base_events: u32,
    alive_total: u32,
    scripted_emit: usize,
) -> EventSpawnStats {
    let cap = b.capacity();
    assert_eq!(
        rand_table.len(),
        RAND_TABLE_LEN,
        "随机带长度必须为 RAND_TABLE_LEN(单源纪律)"
    );
    assert!(
        host_events.len() <= EVENT_CAP && gpu_events.len() <= EVENT_CAP,
        "事件源必经容量面(host {} / gpu {} > EVENT_CAP {EVENT_CAP})",
        host_events.len(),
        gpu_events.len()
    );
    let used = alive_total as usize + scripted_emit;
    assert!(
        used <= cap,
        "容量违约:alive {alive_total} + scripted {scripted_emit} > cap {cap}(调用方脚本预算须先钳)"
    );
    let budget = cap - used;
    let host_n = host_events.len();
    let gpu_n = gpu_events.len();
    let accepted = (host_n + gpu_n).min(budget);
    assert!(
        pid_base_events as usize + accepted < (1usize << 24),
        "pid_base_events + accepted 必须 < 2^24(kernel f32 参数面精确域)"
    );
    for j in 0..accepted {
        // 次序冻结:host 事件先、GPU 死亡事件后。
        let ev = if j < host_n {
            &host_events[j]
        } else {
            &gpu_events[j - host_n]
        };
        let slot = used + j;
        let pid = pid_base_events as usize + j;
        let r = |k: usize| rand_table[(pid * RAND_K + k) % RAND_TABLE_LEN];
        b.pos_x[slot] = ev.payload[0] + (r(0) * 2.0 - 1.0) * sp.spread;
        b.pos_y[slot] = ev.payload[1] + (r(1) * 2.0 - 1.0) * sp.spread;
        b.pos_z[slot] = ev.payload[2] + (r(2) * 2.0 - 1.0) * sp.spread;
        b.vel_x[slot] = ev.payload[3] + (r(3) * 2.0 - 1.0) * sp.vel_spread;
        b.vel_y[slot] = ev.payload[4] + (r(4) * 2.0 - 1.0) * sp.vel_spread;
        // vel_z 不入 payload(布局冻结):基值 0 + 随机带半幅。
        b.vel_z[slot] = (r(5) * 2.0 - 1.0) * sp.vel_spread;
        b.age[slot] = 0.0;
        b.life[slot] = sp.life_base * (0.5 + 0.5 * r(6));
        b.pid[slot] = pid as u32;
    }
    let host_accepted = host_n.min(accepted) as u32;
    EventSpawnStats {
        accepted_total: accepted as u32,
        host_accepted,
        gpu_accepted: accepted as u32 - host_accepted,
    }
}

/// 事件帧统计(host 平行金标准输出;probe 对拍面)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EventFrameStats {
    /// 压缩后存活数(alive scan 总和槽)。
    pub alive_total: u32,
    /// 本帧脚本发射数(调用方已钳)。
    pub scripted_emit: u32,
    /// 双源事件发射统计。
    pub spawn: EventSpawnStats,
    /// 本帧死亡总数(下一帧 GPU 事件源;kept = min(·, EVENT_CAP))。
    pub death_total: u32,
    /// 本帧入缓冲死亡事件数。
    pub death_kept: u32,
    /// 帧末粒子数 = alive + scripted + accepted_total。
    pub n_next: usize,
    /// indirect args 8 槽(emit_count 面 = scripted + accepted_total,
    /// host 平行推得——零回读链与 core 同律)。
    pub args: [u32; 8],
}

/// 事件帧全产物(stats + 整数中间流;probe 零容差对拍消费)。
#[derive(Clone, Debug)]
pub struct EventFrameOut {
    /// 帧统计。
    pub stats: EventFrameStats,
    /// sim 存活 flags(len = 帧前 n)。
    pub flags: Vec<u32>,
    /// 存活稳定 scan(len = 帧前 n)。
    pub scan_out: Vec<u32>,
    /// 死亡事件收集产物(death_flags/death_scan/事件)。
    pub collect: EventCollectOut,
}

/// G35-6 帧序串联 host 单源(读 A 写 B,调用方帧末交换;G35-P v1 帧序 +
/// 事件三步扩展,顺序冻结):
/// 1. sim(W2)→ 2. 存活稳定 scan(W2 三 kernel)→ 3. compact(W2)→
/// 4. emit 脚本发射(W2;pid 段 1)→ 5. **event_spawn**(host 队列 +
/// 上帧 GPU 死亡事件双源;pid 段 2/3)→ 6. **event_collect**(本帧死亡
/// → 下帧 GPU 事件源)→ 7. indirect_args(W2;emit_count = scripted +
/// accepted_total)。`host_queue` 须已 trim(装配面纪律)。
#[allow(clippy::too_many_arguments)]
pub fn event_frame(
    a: &mut ParticlePools,
    b: &mut ParticlePools,
    desc: &EmitterDesc,
    sp: &EventSpawnParams,
    rand_table: &[f32],
    dt: f32,
    host_queue: &EventQueue,
    gpu_events_prev: &[ParticleEvent],
    pid_base: u32,
    scripted_emit: usize,
) -> EventFrameOut {
    let cap = a.capacity();
    let flags = core::sim_step(a, dt, desc.gravity_y);
    let (scan_out, alive_total) = scan::exclusive_scan_segmented(&flags);
    assert!(
        alive_total as usize + scripted_emit <= cap,
        "容量违约:alive {alive_total} + scripted {scripted_emit} > cap {cap}(脚本预算须钳到 cap − n_curr)"
    );
    core::compact_step(a, &flags, &scan_out, b);
    core::emit_step(b, desc, rand_table, pid_base, scripted_emit, alive_total);
    let spawn = event_spawn_step(
        b,
        host_queue.events(),
        gpu_events_prev,
        rand_table,
        sp,
        pid_base + scripted_emit as u32,
        alive_total,
        scripted_emit,
    );
    let collect = event_collect_step(a, &flags);
    let emit_effective = scripted_emit as u32 + spawn.accepted_total;
    let args = core::indirect_args(alive_total, emit_effective);
    let n_next = alive_total as usize + emit_effective as usize;
    b.n = n_next;
    let stats = EventFrameStats {
        alive_total,
        scripted_emit: scripted_emit as u32,
        spawn,
        death_total: collect.death_total,
        death_kept: collect.kept,
        n_next,
        args,
    };
    EventFrameOut {
        stats,
        flags,
        scan_out,
        collect,
    }
}

/// GPU 粒子快照(方向 A:GPU 九流 readback 子集 → host 统一粒子视图桥的
/// plain 数据源;物理 crate `particle_view/external_adapter.rs` 只吃本
/// 结构的三个 plain Vec,不依赖本 crate 的 device 面)。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct GpuParticleSnapshot {
    /// 位置(readback 位型原样)。
    pub positions: Vec<[f32; 3]>,
    /// 速度(readback 位型原样)。
    pub velocities: Vec<[f32; 3]>,
    /// persistent ID(池不变量:帧内唯一)。
    pub ids: Vec<u32>,
}

impl GpuParticleSnapshot {
    /// 自 SoA 九流有效前缀构造(readback 子集;位型原样零算术)。
    #[allow(clippy::too_many_arguments)]
    pub fn from_streams(
        pos_x: &[f32],
        pos_y: &[f32],
        pos_z: &[f32],
        vel_x: &[f32],
        vel_y: &[f32],
        vel_z: &[f32],
        pid: &[u32],
        n: usize,
    ) -> Self {
        assert!(
            pos_x.len() >= n
                && pos_y.len() >= n
                && pos_z.len() >= n
                && vel_x.len() >= n
                && vel_y.len() >= n
                && vel_z.len() >= n
                && pid.len() >= n,
            "快照前缀 n = {n} 超流长"
        );
        Self {
            positions: (0..n).map(|i| [pos_x[i], pos_y[i], pos_z[i]]).collect(),
            velocities: (0..n).map(|i| [vel_x[i], vel_y[i], vel_z[i]]).collect(),
            ids: pid[..n].to_vec(),
        }
    }

    /// 粒子数。
    pub fn len(&self) -> usize {
        self.ids.len()
    }

    /// 是否空。
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// pid 定址读(线性稳定扫,首个匹配;pid 唯一为池不变量)——物理侧
    /// 适配器 ref 解析的同语义 host 面(roundtrip 判别消费)。
    pub fn lookup(&self, pid: u32) -> Option<([f32; 3], [f32; 3])> {
        self.ids
            .iter()
            .position(|&p| p == pid)
            .map(|i| (self.positions[i], self.velocities[i]))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Pcg32, rand_table};
    use super::*;
    use std::collections::HashSet;

    /// 单测夹具(冻结常量;probe 有自己的冻结夹具,互不镜像)。
    fn fixture_desc() -> EmitterDesc {
        EmitterDesc {
            pos: [0.0, 1.0, -0.5],
            spread: [0.4, 0.2, 0.4],
            vel_base: [0.0, 3.0, 0.0],
            vel_spread: [1.0, 0.5, 1.0],
            life_base: 0.9,
            gravity_y: -9.8,
        }
    }

    fn fixture_spawn() -> EventSpawnParams {
        EventSpawnParams {
            spread: 0.05,
            vel_spread: 0.5,
            life_base: 0.8,
        }
    }

    /// 确定性合成事件(payload 各异;producer/slot 由 (tag, k) 派生)。
    fn synth_event(tag: u32, k: u32) -> ParticleEvent {
        ParticleEvent {
            producer_id: tag,
            slot: k,
            kind: EVENT_KIND_HOST,
            payload: [
                tag as f32 * 0.5,
                k as f32 * 0.25,
                -(k as f32) * 0.125,
                1.0 + tag as f32 * 0.0625,
                -2.0 + k as f32 * 0.03125,
            ],
        }
    }

    /// 逐流位级快照(core 单测同律:f32 走 to_bits)。
    fn pool_bits(p: &ParticlePools) -> Vec<u32> {
        let mut out = Vec::with_capacity(p.n * 9 + 1);
        out.push(p.n as u32);
        for i in 0..p.n {
            out.push(p.pos_x[i].to_bits());
            out.push(p.pos_y[i].to_bits());
            out.push(p.pos_z[i].to_bits());
            out.push(p.vel_x[i].to_bits());
            out.push(p.vel_y[i].to_bits());
            out.push(p.vel_z[i].to_bits());
            out.push(p.age[i].to_bits());
            out.push(p.life[i].to_bits());
            out.push(p.pid[i]);
        }
        out
    }

    /// ① 溢出裁剪稳定性:同集乱序 push 必同果(全序键裁剪);保留集 =
    /// 全序前 EVENT_CAP 项;overflow_count 如实登记;trim 幂等。
    #[test]
    fn overflow_trim_stable_regardless_of_push_order() {
        let total = EVENT_CAP + 476;
        let all: Vec<ParticleEvent> = (0..total)
            .map(|i| synth_event((i % 97) as u32, (i / 97) as u32))
            .collect();
        // 顺序 A:自然序;顺序 B:确定性乱序(Fisher-Yates,Pcg32 单源)。
        let mut qa = EventQueue::new();
        for ev in &all {
            qa.push(*ev);
        }
        let mut shuffled = all.clone();
        let mut rng = Pcg32::new(2026, 54);
        for i in (1..shuffled.len()).rev() {
            let j = (rng.next_u32() as usize) % (i + 1);
            shuffled.swap(i, j);
        }
        let mut qb = EventQueue::new();
        for ev in &shuffled {
            qb.push(*ev);
        }
        qa.trim();
        qb.trim();
        assert_eq!(qa.len(), EVENT_CAP);
        assert_eq!(qb.len(), EVENT_CAP);
        assert_eq!(qa.overflow_count(), (total - EVENT_CAP) as u64, "溢出数如实登记");
        assert_eq!(qb.overflow_count(), qa.overflow_count());
        for (x, y) in qa.events().iter().zip(qb.events()) {
            assert!(x.bits_eq(y), "乱序 push 同集必同果(位级)");
        }
        // 保留集 = 独立参考(全集全序排序取前 EVENT_CAP)。
        let mut reference = all.clone();
        reference.sort_by_key(ParticleEvent::order_key);
        for (x, y) in qa.events().iter().zip(reference.iter().take(EVENT_CAP)) {
            assert!(x.bits_eq(y), "保留集必须 = 全序前 capacity 项");
        }
        // 幂等:二次 trim 0 变化。
        let snap: Vec<_> = qa.events().to_vec();
        qa.trim();
        assert_eq!(qa.len(), snap.len());
        assert_eq!(qa.overflow_count(), (total - EVENT_CAP) as u64);
        for (x, y) in qa.events().iter().zip(&snap) {
            assert!(x.bits_eq(y));
        }
    }

    /// ①b 容量内 trim:零溢出、规范序、上传面词流与事件逐字对应。
    #[test]
    fn trim_under_cap_canonicalizes_and_upload_words_match() {
        let mut q = EventQueue::new();
        for k in [5u32, 1, 3] {
            q.push(synth_event(7, k));
        }
        q.trim();
        assert_eq!(q.len(), 3);
        assert_eq!(q.overflow_count(), 0);
        let slots: Vec<u32> = q.events().iter().map(|e| e.slot).collect();
        assert_eq!(slots, vec![1, 3, 5], "容量内亦规范化全序");
        let meta = q.meta_words();
        let pay = q.payload_words();
        assert_eq!(meta.len(), EVENT_CAP * EVENT_META_WORDS);
        assert_eq!(pay.len(), EVENT_CAP * EVENT_PAYLOAD_WORDS);
        for (e, ev) in q.events().iter().enumerate() {
            assert_eq!(meta[e * 3], ev.producer_id);
            assert_eq!(meta[e * 3 + 1], ev.slot);
            assert_eq!(meta[e * 3 + 2], ev.kind);
            for k in 0..EVENT_PAYLOAD_WORDS {
                assert_eq!(pay[e * 5 + k].to_bits(), ev.payload[k].to_bits());
            }
        }
        assert_eq!(meta[9], 0, "空位恒 0");
    }

    /// ② 双源发射顺序:host 先 GPU 后(槽位/pid/属性逐字);预算压力下
    /// host 优先占额;发射公式随机带槽位表手工重算互核。
    #[test]
    fn dual_source_spawn_order_host_first_gpu_after() {
        let table = rand_table(5);
        let sp = fixture_spawn();
        let host: Vec<ParticleEvent> = (0..3).map(|k| synth_event(100, k)).collect();
        let gpu: Vec<ParticleEvent> = (0..2)
            .map(|k| ParticleEvent {
                producer_id: 900 + k,
                slot: k,
                kind: EVENT_KIND_DEATH,
                payload: [9.0 + k as f32, -1.0, 2.5, 0.5, -0.25],
            })
            .collect();
        let mut b = ParticlePools::with_capacity(512);
        let stats = event_spawn_step(&mut b, &host, &gpu, &table, &sp, 4000, 5, 2);
        assert_eq!(
            stats,
            EventSpawnStats {
                accepted_total: 5,
                host_accepted: 3,
                gpu_accepted: 2
            }
        );
        for j in 0..5usize {
            let slot = 5 + 2 + j;
            let pid = 4000 + j;
            let ev = if j < 3 { &host[j] } else { &gpu[j - 3] };
            assert_eq!(b.pid[slot], pid as u32, "pid 连续递增(段 2/3)");
            let r = |k: usize| table[(pid * RAND_K + k) % RAND_TABLE_LEN];
            assert_eq!(
                b.pos_x[slot].to_bits(),
                (ev.payload[0] + (r(0) * 2.0 - 1.0) * sp.spread).to_bits(),
                "j={j}: pos_x 公式面(源次序 host 先)"
            );
            assert_eq!(
                b.vel_y[slot].to_bits(),
                (ev.payload[4] + (r(4) * 2.0 - 1.0) * sp.vel_spread).to_bits()
            );
            assert_eq!(
                b.vel_z[slot].to_bits(),
                ((r(5) * 2.0 - 1.0) * sp.vel_spread).to_bits(),
                "vel_z 基值 0(payload 布局冻结)"
            );
            assert_eq!(b.age[slot].to_bits(), 0.0f32.to_bits());
            assert_eq!(
                b.life[slot].to_bits(),
                (sp.life_base * (0.5 + 0.5 * r(6))).to_bits()
            );
        }
        // 预算压力:budget = 4 < 5 ⇒ host 3 全收,GPU 只收 1(次序冻结)。
        let mut b2 = ParticlePools::with_capacity(256);
        let stats2 = event_spawn_step(&mut b2, &host, &gpu, &table, &sp, 4000, 250, 2);
        assert_eq!(
            stats2,
            EventSpawnStats {
                accepted_total: 4,
                host_accepted: 3,
                gpu_accepted: 1
            }
        );
        assert_eq!(b2.pid[252 + 3], 4003, "预算内最后一个 = GPU 事件 0");
    }

    /// ③ 死亡收集:death_flags 补集恒等式、死亡槽 = 稳定 scan、payload
    /// 位级 = 死亡粒子积分后状态、溢出钳制 + 如实登记。
    #[test]
    fn death_collect_stable_slots_payload_and_overflow() {
        let table = rand_table(9);
        let mut d = fixture_desc();
        d.life_base = 0.4; // life ∈ [0.2, 0.4) ⇒ dt=0.5 一帧全灭
        let cap = 2048;
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        // 帧 0:发射 1500(> EVENT_CAP,溢出腿)。
        core::emit_step(&mut b, &d, &table, 0, 1500, 0);
        b.n = 1500;
        std::mem::swap(&mut a, &mut b);
        let flags = core::sim_step(&mut a, 0.5, d.gravity_y);
        assert!(flags.iter().all(|&f| f == 0), "夹具:一帧全灭");
        let out = event_collect_step(&a, &flags);
        assert_eq!(out.death_total, 1500);
        assert_eq!(out.kept, EVENT_CAP as u32, "溢出钳制 = min(death_total, EVENT_CAP)");
        assert_eq!(out.events.len(), EVENT_CAP);
        for (k, ev) in out.events.iter().enumerate() {
            assert_eq!(ev.slot, k as u32, "死亡槽 = 稳定 scan 序(下标序)");
            assert_eq!(ev.kind, EVENT_KIND_DEATH);
            assert_eq!(ev.producer_id, a.pid[k], "producer = 死亡粒子 pid");
            for (w, want) in [
                a.pos_x[k], a.pos_y[k], a.pos_z[k], a.vel_x[k], a.vel_y[k],
            ]
            .iter()
            .enumerate()
            {
                assert_eq!(
                    ev.payload[w].to_bits(),
                    want.to_bits(),
                    "payload[{w}] = 积分后状态位级(布局冻结 pos.xyz+vel.xy)"
                );
            }
        }
        // 补集恒等式:death_scan[i] = i − alive_scan[i](稳定 scan 互核)。
        let (alive_scan, alive_total) = scan::exclusive_scan_segmented(&flags);
        assert_eq!(alive_total + out.death_total, 1500);
        for i in 0..1500 {
            assert_eq!(out.death_scan[i], i as u32 - alive_scan[i]);
        }
    }

    /// ④ 守恒 + 死亡→二次发射链:多帧脚本 n_next = alive + scripted +
    /// accepted;pid 帧内唯一(三段涵盖);GPU 二次发射确实发生(样本量门)。
    #[test]
    fn conservation_and_secondary_emission_chain() {
        let table = rand_table(11);
        let d = fixture_desc(); // life ∈ [0.45, 0.9),dt = 0.1 ⇒ ~5..9 帧死亡
        let sp = fixture_spawn();
        let cap = 2048;
        let dt = 0.1;
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        let mut pid_base = 0u32;
        let mut gpu_prev: Vec<ParticleEvent> = Vec::new();
        let mut secondary_frames = 0usize;
        let mut host_events_sent = 0u64;
        for f in 0..24 {
            let scripted = (32 + (f * 7) % 48).min(cap - a.n);
            let mut q = EventQueue::new();
            for k in 0..(f % 4) {
                q.push(synth_event(50_000 + f as u32, k as u32));
            }
            q.trim();
            host_events_sent += q.len() as u64;
            let out = event_frame(
                &mut a, &mut b, &d, &sp, &table, dt, &q, &gpu_prev, pid_base, scripted,
            );
            let st = out.stats;
            // 守恒:n_next = alive + scripted + accepted;args[7] 同值。
            assert_eq!(
                st.n_next,
                st.alive_total as usize + scripted + st.spawn.accepted_total as usize,
                "帧 {f}: n_next 守恒破"
            );
            assert_eq!(st.args[7] as usize, st.n_next, "帧 {f}: args 恒等式破");
            assert_eq!(b.n, st.n_next);
            // 双源合并计数守恒。
            assert_eq!(
                st.spawn.host_accepted + st.spawn.gpu_accepted,
                st.spawn.accepted_total
            );
            assert_eq!(
                st.spawn.accepted_total.min(q.len() as u32 + gpu_prev.len() as u32),
                st.spawn.accepted_total
            );
            // pid 帧内唯一(三段涵盖)。
            let uniq: HashSet<u32> = b.pid[..b.n].iter().copied().collect();
            assert_eq!(uniq.len(), b.n, "帧 {f}: pid 重复(三段涵盖破)");
            if st.spawn.gpu_accepted > 0 {
                secondary_frames += 1;
            }
            pid_base += scripted as u32 + st.spawn.accepted_total;
            gpu_prev = out.collect.events;
            std::mem::swap(&mut a, &mut b);
        }
        assert!(
            secondary_frames >= 5,
            "GPU 死亡→二次发射链必须真实发生(样本量门,得 {secondary_frames})"
        );
        assert!(host_events_sent > 0);
    }

    /// ⑤ 双跑位级:同 seed 全链(队列/死亡链/双源发射)双跑池位级 +
    /// 统计全等;异 seed 可分辨。
    #[test]
    fn event_pipeline_double_run_bitexact() {
        fn run(seed: u64) -> (Vec<u32>, Vec<EventFrameStats>) {
            let table = rand_table(seed);
            let d = fixture_desc();
            let sp = fixture_spawn();
            let cap = 2048;
            let mut a = ParticlePools::with_capacity(cap);
            let mut b = ParticlePools::with_capacity(cap);
            let mut pid_base = 0u32;
            let mut gpu_prev: Vec<ParticleEvent> = Vec::new();
            let mut stats = Vec::new();
            let mut rng = Pcg32::new(seed, 91);
            for f in 0..20 {
                let scripted = (24 + (f * 5) % 40).min(cap - a.n);
                let mut q = EventQueue::new();
                for k in 0..((f * 3) % 5) {
                    let mut ev = synth_event(60_000 + f as u32, k as u32);
                    ev.payload[0] = rng.next_f32() * 4.0 - 2.0;
                    q.push(ev);
                }
                q.trim();
                let out = event_frame(
                    &mut a, &mut b, &d, &sp, &table, 0.1, &q, &gpu_prev, pid_base, scripted,
                );
                pid_base += scripted as u32 + out.stats.spawn.accepted_total;
                stats.push(out.stats);
                gpu_prev = out.collect.events;
                std::mem::swap(&mut a, &mut b);
            }
            (pool_bits(&a), stats)
        }
        let (p1, s1) = run(7);
        let (p2, s2) = run(7);
        assert_eq!(s1, s2, "同 seed 双跑逐帧 stats 必须全等");
        assert_eq!(p1, p2, "同 seed 双跑末态池必须位级全等");
        let (p3, _) = run(8);
        assert_ne!(p1, p3, "异 seed 必须可分辨(判据有效性)");
    }

    /// ⑥ 快照 roundtrip:九流前缀 → 快照 → pid 定址读 == 原流位级;
    /// 未知 pid → None(方向 A 桥的 host 同语义面)。
    #[test]
    fn snapshot_roundtrip_bitexact_by_pid() {
        let table = rand_table(13);
        let d = fixture_desc();
        let mut b = ParticlePools::with_capacity(512);
        core::emit_step(&mut b, &d, &table, 300, 40, 0);
        b.n = 40;
        let mut a = b.clone();
        core::sim_step(&mut a, 0.05, d.gravity_y); // 非平凡位型
        let snap = GpuParticleSnapshot::from_streams(
            &a.pos_x, &a.pos_y, &a.pos_z, &a.vel_x, &a.vel_y, &a.vel_z, &a.pid, a.n,
        );
        assert_eq!(snap.len(), 40);
        for i in 0..a.n {
            let (p, v) = snap.lookup(a.pid[i]).expect("池内 pid 必命中");
            for (k, (got, want)) in [
                (p[0], a.pos_x[i]),
                (p[1], a.pos_y[i]),
                (p[2], a.pos_z[i]),
                (v[0], a.vel_x[i]),
                (v[1], a.vel_y[i]),
                (v[2], a.vel_z[i]),
            ]
            .into_iter()
            .enumerate()
            {
                assert_eq!(got.to_bits(), want.to_bits(), "pid {} 分量 {k} 非位级", a.pid[i]);
            }
        }
        assert_eq!(snap.lookup(999_999), None, "未知 pid 必 None(不静默伪值)");
    }
}
