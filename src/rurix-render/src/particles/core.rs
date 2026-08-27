//! G35-2 粒子核心运行时 host 金标准(池/发射/积分/稳定压缩/间接参数)——门
//! `g35.wave2.particle_core`(RFC-0049 §4.3;契约 = mod.rs G35-P v1 冻结头,
//! 事实源 = milestones/g35/G35_CONTRACT.md)。
//!
//! 与 device 四 kernel(`kernels/g35_sim.rx` / `g35_particle_compact.rx` /
//! `g35_emit.rx` / `g35_indirect_args.rx`)**逐字同源**;scan 三 kernel
//! ([`super::scan`])只消费不修改。G35-P v1 帧序冻结(读 A 写 B,帧末交换):
//! sim → scan(flags → scan_out + seg_offsets[nseg] = alive_total)→
//! particle_compact → emit → indirect_args;[`frame`] 为该串联的 host 单源。
//!
//! ## 确定性协议(mod.rs 契约头字面)
//! - 整数流(pid/flags/scan/args)device↔host **零容差位级**;f32 流
//!   (pos/vel/age/life)device↔host 标定容差(threshold = measured×2.0
//!   程序产禁手写,milestones/g35/g35_budget.json);device 双跑一律位级。
//! - **禁原子抢槽**:压缩/发射槽位一律经分段稳定 scan 推导(稳定序 =
//!   粒子下标序,与线程调度无关)。
//! - 随机带单源([`super::rand_table`]):device 只读消费
//!   `rand_table[(pid·RAND_K + slot) % RAND_TABLE_LEN]`;槽位表:
//!   0/1/2 = pos xyz,3/4/5 = vel xyz,6 = life。
//! - persistent ID:pid = pid_base + j(host 递增维护 u32;kernel 参数面
//!   f32 精确域 pid_base + emit_count < 2^24,[`emit_step`] 断言)。
//! - 半隐式 Euler 运算序逐字冻结(drag v1 恒 0 登记):
//!   `vy = vy + g·dt; px = px + vx·dt; py = py + vy·dt(消费更新后 vy);
//!   pz = pz + vz·dt; age = age + dt; flags = (age < life)`。
//! - indirect args 零回读链:device 端 total = seg_offsets[nseg] + emit_count
//!   直合成 dispatch/draw 参数(args[7] = meta 槽);host 平行推得
//!   n_next = alive_total + emit_count 只对拍验证、不读回 device 计数。

use super::scan;
use super::{RAND_K, RAND_TABLE_LEN, SEG};

/// 发射器描述(probe/单测夹具冻结常量的类型面;gravity_y 为 v1 唯一外力,
/// drag v1 恒 0 登记不入面)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EmitterDesc {
    /// 发射中心(kernel params[2..5))。
    pub pos: [f32; 3],
    /// 位置半幅(kernel params[5..8);px = ex + (r0·2−1)·spread_x)。
    pub spread: [f32; 3],
    /// 初速基值(kernel params[8..11))。
    pub vel_base: [f32; 3],
    /// 初速半幅(kernel params[11..14))。
    pub vel_spread: [f32; 3],
    /// 寿命基值(kernel params[14];life = life_base·(0.5 + 0.5·r6))。
    pub life_base: f32,
    /// 重力 y 分量(kernel params[3];v1 唯一积分外力)。
    pub gravity_y: f32,
}

/// SoA 粒子池单组(9 流;ping-pong 双组由调用方持有 A/B 两实例并帧末交换)。
/// 全流按容量 cap 分配;`n` = 当前有效前缀长度(帧协议 n_curr / n_next)。
#[derive(Clone, Debug)]
pub struct ParticlePools {
    /// 位置 x(f32 流)。
    pub pos_x: Vec<f32>,
    /// 位置 y。
    pub pos_y: Vec<f32>,
    /// 位置 z。
    pub pos_z: Vec<f32>,
    /// 速度 x。
    pub vel_x: Vec<f32>,
    /// 速度 y。
    pub vel_y: Vec<f32>,
    /// 速度 z。
    pub vel_z: Vec<f32>,
    /// 年龄(秒)。
    pub age: Vec<f32>,
    /// 寿命(秒)。
    pub life: Vec<f32>,
    /// persistent ID(u32 整数流,零容差协议域)。
    pub pid: Vec<u32>,
    /// 有效粒子数(≤ cap;帧协议 host 平行金标准维护)。
    pub n: usize,
}

impl ParticlePools {
    /// 按容量分配全零池(cap 须为 SEG 整倍数且 ≤ PARTICLE_CAP_MAX,
    /// mod.rs 分段布局契约)。
    pub fn with_capacity(cap: usize) -> Self {
        assert!(
            cap > 0 && cap % SEG == 0 && cap <= super::PARTICLE_CAP_MAX,
            "池容量须为 SEG={SEG} 整倍数且 ∈ (0, {}](得 {cap})",
            super::PARTICLE_CAP_MAX
        );
        Self {
            pos_x: vec![0.0; cap],
            pos_y: vec![0.0; cap],
            pos_z: vec![0.0; cap],
            vel_x: vec![0.0; cap],
            vel_y: vec![0.0; cap],
            vel_z: vec![0.0; cap],
            age: vec![0.0; cap],
            life: vec![0.0; cap],
            pid: vec![0; cap],
            n: 0,
        }
    }

    /// 池容量(全流等长分配)。
    pub fn capacity(&self) -> usize {
        self.pos_x.len()
    }
}

/// 帧统计(host 平行金标准输出;args = device g35_indirect_args 对拍面)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameStats {
    /// 压缩后存活数(= scan 总和槽 seg_offsets[nseg])。
    pub alive_total: u32,
    /// 帧末粒子数 = alive_total + emit_count(零回读链 host 平行推得)。
    pub n_next: usize,
    /// indirect args 8 槽:{total,1,1, total·6,1,0,0, total}(u32 缓冲)。
    pub args: [u32; 8],
}

/// 帧序第 1 步(= kernels/g35_sim.rx 逐字同源):半隐式 Euler 原位积分 +
/// 存活 flags。运算序逐字冻结(py 消费更新后 vy;drag v1 恒 0 登记);
/// 返回 flags(len = p.n,scan 消费面)。
pub fn sim_step(p: &mut ParticlePools, dt: f32, gravity_y: f32) -> Vec<u32> {
    let n = p.n;
    let mut flags = vec![0u32; n];
    for i in 0..n {
        // 运算序逐字冻结(kernel 同序):vy → px → py(新 vy)→ pz → age。
        p.vel_y[i] = p.vel_y[i] + gravity_y * dt;
        p.pos_x[i] = p.pos_x[i] + p.vel_x[i] * dt;
        p.pos_y[i] = p.pos_y[i] + p.vel_y[i] * dt;
        p.pos_z[i] = p.pos_z[i] + p.vel_z[i] * dt;
        p.age[i] = p.age[i] + dt;
        flags[i] = u32::from(p.age[i] < p.life[i]);
    }
    flags
}

/// 帧序第 3 步(= kernels/g35_particle_compact.rx 逐字同源):flags/scan_out
/// 消费,9 流 A→B 稳定搬运(稳定序 = 下标序;槽位 = 分段稳定 scan 产物,
/// 禁原子抢槽)。
pub fn compact_step(a: &ParticlePools, flags: &[u32], scan_out: &[u32], b: &mut ParticlePools) {
    let n = a.n;
    for i in 0..n {
        if flags[i] != 0 {
            let dst = scan_out[i] as usize;
            b.pos_x[dst] = a.pos_x[i];
            b.pos_y[dst] = a.pos_y[i];
            b.pos_z[dst] = a.pos_z[i];
            b.vel_x[dst] = a.vel_x[i];
            b.vel_y[dst] = a.vel_y[i];
            b.vel_z[dst] = a.vel_z[i];
            b.age[dst] = a.age[i];
            b.life[dst] = a.life[i];
            b.pid[dst] = a.pid[i];
        }
    }
}

/// 帧序第 4 步(= kernels/g35_emit.rx 逐字同源):确定性发射——随机带单源
/// 消费律 `r_k = rand_table[(pid·RAND_K + k) % RAND_TABLE_LEN]`(槽位表
/// 0/1/2 = pos,3/4/5 = vel,6 = life),persistent ID = pid_base + j,
/// 槽位 = alive_total + j(scan 总和槽,device 端零回读直读)。
pub fn emit_step(
    b: &mut ParticlePools,
    desc: &EmitterDesc,
    rand_table: &[f32],
    pid_base: u32,
    emit_count: usize,
    alive_total: u32,
) {
    assert_eq!(
        rand_table.len(),
        RAND_TABLE_LEN,
        "随机带长度必须为 RAND_TABLE_LEN(单源纪律)"
    );
    assert!(
        pid_base as usize + emit_count < (1usize << 24),
        "pid_base + emit_count 必须 < 2^24(kernel f32 参数面精确域)"
    );
    for j in 0..emit_count {
        let slot = alive_total as usize + j;
        let pid = pid_base as usize + j;
        let r = |k: usize| rand_table[(pid * RAND_K + k) % RAND_TABLE_LEN];
        b.pos_x[slot] = desc.pos[0] + (r(0) * 2.0 - 1.0) * desc.spread[0];
        b.pos_y[slot] = desc.pos[1] + (r(1) * 2.0 - 1.0) * desc.spread[1];
        b.pos_z[slot] = desc.pos[2] + (r(2) * 2.0 - 1.0) * desc.spread[2];
        b.vel_x[slot] = desc.vel_base[0] + (r(3) * 2.0 - 1.0) * desc.vel_spread[0];
        b.vel_y[slot] = desc.vel_base[1] + (r(4) * 2.0 - 1.0) * desc.vel_spread[1];
        b.vel_z[slot] = desc.vel_base[2] + (r(5) * 2.0 - 1.0) * desc.vel_spread[2];
        b.age[slot] = 0.0;
        b.life[slot] = desc.life_base * (0.5 + 0.5 * r(6));
        b.pid[slot] = pid as u32;
    }
}

/// 帧序第 5 步(= kernels/g35_indirect_args.rx 逐字同源):indirect args 8 槽
/// 合成——[0..3) = dispatch {total,1,1};[3..7) = draw {total·6,1,0,0}
/// (vertexCount = 6·total);[7] = total(meta 槽,零回读链对拍面)。
pub fn indirect_args(alive_total: u32, emit_count: u32) -> [u32; 8] {
    let total = alive_total + emit_count;
    [total, 1, 1, total * 6, 1, 0, 0, total]
}

/// G35-P v1 帧序串联(读 A 写 B;调用方帧末交换 A/B):sim → scan 三段
/// ([`scan::exclusive_scan_segmented`],nseg = ceil(n/SEG) 与 device
/// dispatch 同式)→ compact → emit → indirect_args。返回 [`FrameStats`]
/// (n_next = alive_total + emit_count,host 平行金标准零回读推得)。
pub fn frame(
    a: &mut ParticlePools,
    b: &mut ParticlePools,
    desc: &EmitterDesc,
    rand_table: &[f32],
    dt: f32,
    pid_base: u32,
    emit_count: usize,
) -> FrameStats {
    let cap = a.capacity();
    let flags = sim_step(a, dt, desc.gravity_y);
    let (scan_out, alive_total) = scan::exclusive_scan_segmented(&flags);
    assert!(
        alive_total as usize + emit_count <= cap,
        "容量违约:alive {alive_total} + emit {emit_count} > cap {cap}(调用方发射预算须钳到 cap − n_curr)"
    );
    compact_step(a, &flags, &scan_out, b);
    emit_step(b, desc, rand_table, pid_base, emit_count, alive_total);
    let args = indirect_args(alive_total, emit_count as u32);
    let n_next = alive_total as usize + emit_count;
    b.n = n_next;
    FrameStats {
        alive_total,
        n_next,
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::super::rand_table;
    use super::*;
    use std::collections::{HashMap, HashSet};

    /// 单测夹具(冻结常量;probe 有自己的冻结夹具,互不镜像)。
    fn fixture_desc() -> EmitterDesc {
        EmitterDesc {
            pos: [0.0, 1.0, -0.5],
            spread: [0.4, 0.2, 0.4],
            vel_base: [0.0, 3.0, 0.0],
            vel_spread: [1.0, 0.5, 1.0],
            life_base: 1.2,
            gravity_y: -9.8,
        }
    }

    /// 确定性发射预算(probe 同式:min(64 + f·17 % 192, cap − n_curr))。
    fn emit_schedule(f: usize, n_curr: usize, cap: usize) -> usize {
        (64 + (f * 17) % 192).min(cap - n_curr)
    }

    /// 多帧驱动(读 A 写 B 帧末交换);返回逐帧 stats 与末态池。
    fn run_frames(seed: u64, frames: usize, cap: usize, dt: f32) -> (Vec<FrameStats>, ParticlePools) {
        let table = rand_table(seed);
        let d = fixture_desc();
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        let mut pid_base = 0u32;
        let mut all = Vec::new();
        for f in 0..frames {
            let emit = emit_schedule(f, a.n, cap);
            let st = frame(&mut a, &mut b, &d, &table, dt, pid_base, emit);
            pid_base += emit as u32;
            all.push(st);
            std::mem::swap(&mut a, &mut b);
        }
        (all, a)
    }

    /// 逐流位级快照(f32 用 to_bits——PartialEq 的 f32 == 在 −0/NaN 面弱于
    /// 位级,g27 harness 同律)。
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

    /// ① 守恒:压缩后 n = 存活计数(flags 直数,独立于 scan 分解)+ 帧末
    /// n_next = alive + emit;死亡确已发生(非空转样本量门)。
    #[test]
    fn conservation_compacted_equals_alive_count() {
        let table = rand_table(3);
        let d = fixture_desc();
        let cap = 2048;
        let dt = 0.05; // life ∈ [0.6, 1.2) ⇒ 30 帧窗内必有死亡
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        let mut pid_base = 0u32;
        let mut total_emitted = 0usize;
        for f in 0..30 {
            let emit = emit_schedule(f, a.n, cap);
            // 平行重放:压缩前存活计数(clone 上跑同一 sim_step,flags 直数)。
            let mut replay = a.clone();
            let flags = sim_step(&mut replay, dt, d.gravity_y);
            let alive: u32 = flags.iter().sum();
            let st = frame(&mut a, &mut b, &d, &table, dt, pid_base, emit);
            assert_eq!(st.alive_total, alive, "帧 {f}: 压缩计数 ≠ flags 存活计数");
            assert_eq!(st.n_next, alive as usize + emit, "帧 {f}: n_next 守恒破");
            assert_eq!(b.n, st.n_next, "帧 {f}: 池 n 未同步");
            pid_base += emit as u32;
            total_emitted += emit;
            std::mem::swap(&mut a, &mut b);
        }
        assert!(
            a.n < total_emitted,
            "30 帧窗内应有寿命耗尽死亡(样本量门,防守恒断言空转)"
        );
    }

    /// ② pid 持久唯一:每帧无重复 pid;幸存集 ⊆ 上帧集;逐 pid 轨迹连续
    /// (状态演化 = 冻结运算序的单粒子重放,bitwise 全等)。
    #[test]
    fn pid_persistent_unique_and_trajectory_continuous() {
        // 冻结运算序的单粒子重放(sim_step 同序;索引 0..2 pos/3..5 vel/6 age)。
        fn advance(s: &mut [f32; 8], dt: f32, g: f32) {
            s[4] = s[4] + g * dt;
            s[0] = s[0] + s[3] * dt;
            s[1] = s[1] + s[4] * dt;
            s[2] = s[2] + s[5] * dt;
            s[6] = s[6] + dt;
        }
        let table = rand_table(7);
        let d = fixture_desc();
        let cap = 2048;
        let dt = 0.05;
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        let mut pid_base = 0u32;
        let mut prev: HashMap<u32, [f32; 8]> = HashMap::new();
        let mut tracked_frames = 0usize;
        for f in 0..25 {
            let emit = emit_schedule(f, a.n, cap);
            frame(&mut a, &mut b, &d, &table, dt, pid_base, emit);
            pid_base += emit as u32;
            std::mem::swap(&mut a, &mut b);
            // 帧末快照 + 唯一性。
            let mut cur: HashMap<u32, [f32; 8]> = HashMap::new();
            for i in 0..a.n {
                let st = [
                    a.pos_x[i], a.pos_y[i], a.pos_z[i], a.vel_x[i], a.vel_y[i], a.vel_z[i],
                    a.age[i], a.life[i],
                ];
                assert!(
                    cur.insert(a.pid[i], st).is_none(),
                    "帧 {f}: pid {} 重复(持久唯一破)",
                    a.pid[i]
                );
            }
            // 幸存集 ⊆ 上帧集 ∪ 本帧新发射区间;轨迹连续(bitwise)。
            for (pid, st_now) in &cur {
                if let Some(st_prev) = prev.get(pid) {
                    let mut expect = *st_prev;
                    advance(&mut expect, dt, d.gravity_y);
                    for (k, (e, g)) in expect.iter().zip(st_now.iter()).enumerate() {
                        assert_eq!(
                            e.to_bits(),
                            g.to_bits(),
                            "帧 {f}: pid {pid} 分量 {k} 轨迹不连续(冻结运算序重放 ≠ 池态)"
                        );
                    }
                    tracked_frames += 1;
                } else {
                    assert!(
                        *pid >= pid_base - emit as u32 && *pid < pid_base,
                        "帧 {f}: pid {pid} 非幸存亦非本帧发射区间(暗生成检出)"
                    );
                }
            }
            prev = cur;
        }
        assert!(tracked_frames > 100, "轨迹连续核验样本量不足({tracked_frames})");
    }

    /// ③ 固定 seed 双跑位级(f32 to_bits 全等 + stats 全等);异 seed 可分辨。
    #[test]
    fn fixed_seed_double_run_bitexact() {
        let (s1, p1) = run_frames(11, 20, 2048, 0.05);
        let (s2, p2) = run_frames(11, 20, 2048, 0.05);
        assert_eq!(s1, s2, "同 seed 双跑逐帧 stats 必须全等");
        assert_eq!(pool_bits(&p1), pool_bits(&p2), "同 seed 双跑末态池必须位级全等");
        let (_, p3) = run_frames(12, 20, 2048, 0.05);
        assert_ne!(pool_bits(&p1), pool_bits(&p3), "异 seed 必须可分辨(digest 判据有效性)");
    }

    /// ④ 发射确定性:同 pid 同属性(异槽位 bitwise 全等)+ 随机带槽位表
    /// 手工重算互核(防 slot 错位)。
    #[test]
    fn emission_deterministic_same_pid_same_attrs() {
        let table = rand_table(5);
        let d = fixture_desc();
        let mut b1 = ParticlePools::with_capacity(512);
        let mut b2 = ParticlePools::with_capacity(512);
        emit_step(&mut b1, &d, &table, 100, 8, 0);
        emit_step(&mut b2, &d, &table, 100, 8, 300); // 同 pid 异槽位
        for j in 0..8usize {
            let (i1, i2) = (j, 300 + j);
            assert_eq!(b1.pid[i1], b2.pid[i2]);
            assert_eq!(b1.pid[i1], 100 + j as u32);
            for (x, y) in [
                (b1.pos_x[i1], b2.pos_x[i2]),
                (b1.pos_y[i1], b2.pos_y[i2]),
                (b1.pos_z[i1], b2.pos_z[i2]),
                (b1.vel_x[i1], b2.vel_x[i2]),
                (b1.vel_y[i1], b2.vel_y[i2]),
                (b1.vel_z[i1], b2.vel_z[i2]),
                (b1.age[i1], b2.age[i2]),
                (b1.life[i1], b2.life[i2]),
            ] {
                assert_eq!(x.to_bits(), y.to_bits(), "同 pid 属性必须与槽位无关");
            }
            // 随机带槽位表互核(消费律手工重算)。
            let pid = 100 + j;
            let r = |k: usize| table[(pid * RAND_K + k) % RAND_TABLE_LEN];
            assert_eq!(
                b1.pos_x[j].to_bits(),
                (d.pos[0] + (r(0) * 2.0 - 1.0) * d.spread[0]).to_bits()
            );
            assert_eq!(
                b1.vel_z[j].to_bits(),
                (d.vel_base[2] + (r(5) * 2.0 - 1.0) * d.vel_spread[2]).to_bits()
            );
            assert_eq!(
                b1.life[j].to_bits(),
                (d.life_base * (0.5 + 0.5 * r(6))).to_bits()
            );
            assert_eq!(b1.age[j].to_bits(), 0.0f32.to_bits());
        }
    }

    /// ⑤ 寿命耗尽粒子消失:池内不变量 age < life;单批发射零补充 ⇒ 池必清空,
    /// 死亡 pid 不复现。
    #[test]
    fn life_exhaustion_removes_particles() {
        let table = rand_table(9);
        let mut d = fixture_desc();
        d.life_base = 0.5; // life ∈ [0.25, 0.5),dt=0.1 ⇒ ≤5 次积分后全灭
        let dt = 0.1;
        let cap = 512;
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        frame(&mut a, &mut b, &d, &table, dt, 0, 32);
        std::mem::swap(&mut a, &mut b);
        let emitted: HashSet<u32> = a.pid[..a.n].iter().copied().collect();
        assert_eq!(emitted.len(), 32);
        let mut seen_dead: HashSet<u32> = HashSet::new();
        for f in 1..=8 {
            let mut prev_alive: HashSet<u32> = a.pid[..a.n].iter().copied().collect();
            frame(&mut a, &mut b, &d, &table, dt, 32, 0);
            std::mem::swap(&mut a, &mut b);
            let now: HashSet<u32> = a.pid[..a.n].iter().copied().collect();
            for i in 0..a.n {
                assert!(a.age[i] < a.life[i], "帧 {f}: 池内出现寿命耗尽粒子(压缩漏删)");
                assert!(
                    !seen_dead.contains(&a.pid[i]),
                    "帧 {f}: 死亡 pid {} 复现(持久性破)",
                    a.pid[i]
                );
            }
            assert!(now.is_subset(&prev_alive), "帧 {f}: 幸存集非上帧子集");
            for pid in prev_alive.drain() {
                if !now.contains(&pid) {
                    seen_dead.insert(pid);
                }
            }
        }
        assert_eq!(a.n, 0, "0.5s 寿命上界 + 0.8s 积分窗后池必清空");
        assert_eq!(seen_dead.len(), 32, "32 个发射粒子必须全部经寿命耗尽消失");
    }

    /// ⑥ args 恒等式:args[7] == alive + emit;args 全 8 槽与 indirect_args
    /// 公式面全等(dispatch {total,1,1} / draw {6·total,1,0,0})。
    #[test]
    fn indirect_args_identity() {
        let (stats, _) = run_frames(13, 15, 2048, 0.05);
        let mut emitted_prev_n = 0usize;
        for (f, st) in stats.iter().enumerate() {
            let emit = emit_schedule(f, emitted_prev_n, 2048);
            assert_eq!(
                st.args[7],
                st.alive_total + emit as u32,
                "帧 {f}: args[7] ≠ alive + emit(零回读链恒等式破)"
            );
            assert_eq!(st.args, indirect_args(st.alive_total, emit as u32));
            assert_eq!(st.args[0], st.args[7], "dispatch groupCountX ≠ total");
            assert_eq!([st.args[1], st.args[2]], [1, 1]);
            assert_eq!(st.args[3], st.args[7] * 6, "draw vertexCount ≠ 6·total");
            assert_eq!([st.args[4], st.args[5], st.args[6]], [1, 0, 0]);
            assert_eq!(st.n_next, st.args[7] as usize);
            emitted_prev_n = st.n_next;
        }
    }
}
