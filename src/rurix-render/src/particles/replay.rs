//! G35-9 确定性回放/回滚 host 面——门 `g35.wave9.replay`(RFC-0049 §4.12;
//! 契约 = mod.rs G35-P v1 冻结头,帧协议 = [`super::core::frame`] host 金标准)。
//!
//! 兑现「确定性 GPU 粒子」总口径的回放半部(反打 Niagara:网络不复制粒子
//! 本体、GPU 模拟不可回放):粒子系统的**全部输入**(seed / emitter / dt /
//! 逐帧 emit 序列)可完整 journal 化;回放 = 仅凭 journal 重建输入在 GPU 上
//! **重仿真**(非 host 数据回放),逐帧 digest 与录制链**位级全等**;回滚 =
//! 检查点恢复 + 输入重放(网络回滚语义);篡改 journal 任一帧 ⇒ digest 链
//! 首异帧精确定位到该帧(分歧可定位见证,确定性系统独有性质)。
//!
//! ## Journal v1 冻结布局(全域小端;手写 to_le_bytes 确定性序列化,零外部 crate)
//!
//! ```text
//! [ 0.. 4)  magic              b"G35J"
//! [ 4.. 8)  version            u32 = 1
//! [ 8..16)  seed               u64(随机带单源 rand_table(seed))
//! [16..20)  cap                u32(池容量;SEG 整倍数 ≤ PARTICLE_CAP_MAX)
//! [20..24)  frames             u32(= records.len())
//! [24..28)  dt                 f32(位型序列化)
//! [28..32)  gravity_y          f32(积分外力单源;v1 与 emitter.gravity_y
//!                               位级恒等,validate_v1 硬校)
//! [32..44)  emitter.pos        [f32; 3]
//! [44..56)  emitter.spread     [f32; 3]
//! [56..68)  emitter.vel_base   [f32; 3]
//! [68..80)  emitter.vel_spread [f32; 3]
//! [80..84)  emitter.life_base  f32
//! [84..88)  emitter.gravity_y  f32
//! [88..88+4·frames)  records[f].emit_count u32(v1 emitter 恒定;
//!                     扩展字段走版本位,非加性改布局须 version bump)
//! ```
//!
//! header = 88 B([`JOURNAL_HEADER_BYTES`]),逐帧记录 = 4 B
//! ([`JOURNAL_RECORD_BYTES`])。写/读往返位级(单测);魔数/版本/长度
//! fail-closed 拒错([`JournalError`])。
//!
//! ## Checkpoint 语义(回滚 = 检查点 + 输入重放)
//!
//! checkpoint(frame = k)= 帧 k **开始前**(= 帧 k−1 帧末 ping-pong 交换后)
//! 的九流全容量原字节(序 = pos_x/pos_y/pos_z/vel_x/vel_y/vel_z/age/life/pid,
//! 与 device SSBO 序及 probe DevState 下标序严格一致)+ pid_base + n_curr。
//! 回滚 = 恢复该态(device 上传恢复 / host [`Checkpoint::restore_pools`])→
//! 以 journal 输入重仿真帧 k..=j;digest 链种子 = 录制链 digest[k−1]
//! (k = 0 用全零 64 hex 种子)⇒ 恢复帧自身 digest 与录制链位级全等
//! (checkpoint_restore_bitexact),重仿真至 j 逐帧位级全等
//! (rollback_resim_bitexact)。B 组/中间流恢复期清零即可:digest 只覆盖
//! 各流有效前缀,而该前缀每帧被 compact/emit/scan/args 全量重写,前缀外
//! 字节不进 digest 也不被后续帧读取。
//!
//! 检查点文件 "G35C" v1 布局(小端):`magic[4] version:u32 cap:u32
//! interval:u32 count:u32` + 逐检查点 `{frame:u32 pid_base:u32 n_curr:u32
//! streams:9×cap·4 B}`。

use super::core::{EmitterDesc, ParticlePools};
use super::{PARTICLE_CAP_MAX, SEG};

/// journal 魔数(v1 冻结)。
pub const JOURNAL_MAGIC: [u8; 4] = *b"G35J";
/// journal 版本(布局非加性变更须 bump;错版 fail-closed 拒读)。
pub const JOURNAL_VERSION: u32 = 1;
/// journal header 字节数(v1 冻结布局,见模块头)。
pub const JOURNAL_HEADER_BYTES: usize = 88;
/// journal 逐帧记录字节数(v1 = emit_count u32)。
pub const JOURNAL_RECORD_BYTES: usize = 4;
/// 检查点文件魔数(v1 冻结)。
pub const CHECKPOINT_MAGIC: [u8; 4] = *b"G35C";
/// 检查点文件版本。
pub const CHECKPOINT_VERSION: u32 = 1;
/// SoA 粒子池流数(9 流序 = mod.rs 分段布局契约/probe DevState 下标序)。
pub const POOL_STREAMS: usize = 9;

/// journal 反序列化错误闭集(fail-closed;`Truncated`/`TrailingBytes` 同时
/// 覆盖检查点文件面)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalError {
    /// 魔数不符(非 journal/检查点字节流)。
    BadMagic([u8; 4]),
    /// 版本不符(错版拒读,禁静默降级解读)。
    BadVersion(u32),
    /// 字节流截断(need = 还需字节数下界,got = 剩余字节数)。
    Truncated { need: usize, got: usize },
    /// 尾随多余字节(长度必须精确;防拼接/截断类损伤静默通过)。
    TrailingBytes { extra: usize },
}

impl std::fmt::Display for JournalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BadMagic(m) => write!(f, "魔数不符(得 {m:?})"),
            Self::BadVersion(v) => write!(f, "版本不符(得 {v},支持 v1)"),
            Self::Truncated { need, got } => write!(f, "字节流截断(需 ≥{need},剩 {got})"),
            Self::TrailingBytes { extra } => write!(f, "尾随多余字节({extra} B)"),
        }
    }
}

/// journal header(v1 冻结;gravity_y 为积分外力单源,emitter 携带
/// EmitterDesc 全字段镜像——v1 二者位级恒等,[`Journal::validate_v1`] 硬校)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JournalHeader {
    /// 随机带单源 seed([`super::rand_table`])。
    pub seed: u64,
    /// 池容量(SEG 整倍数 ≤ PARTICLE_CAP_MAX)。
    pub cap: u32,
    /// 帧数(= records.len())。
    pub frames: u32,
    /// 帧步长(位型序列化)。
    pub dt: f32,
    /// 重力 y(v1 唯一积分外力;与 emitter.gravity_y 位级恒等)。
    pub gravity_y: f32,
    /// 发射器描述全字段(v1 恒定;变更走版本位)。
    pub emitter: EmitterDesc,
}

/// 逐帧输入记录(v1 = emit_count 单字段;扩展走版本位)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FrameRecord {
    /// 本帧发射数(录制 = 确定性脚本产;回放/回滚 = 直接消费)。
    pub emit_count: u32,
}

/// 粒子输入 journal(header + 逐帧记录;确定性序列化往返位级)。
#[derive(Clone, Debug, PartialEq)]
pub struct Journal {
    /// v1 冻结 header。
    pub header: JournalHeader,
    /// 逐帧输入记录(len == header.frames)。
    pub records: Vec<FrameRecord>,
}

/// 小端字节流读取器(零外部 crate;越界 = Truncated fail-closed)。
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], JournalError> {
        let got = self.bytes.len() - self.pos;
        if got < n {
            return Err(JournalError::Truncated {
                need: self.pos + n,
                got: self.bytes.len(),
            });
        }
        let s = &self.bytes[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    fn u32(&mut self) -> Result<u32, JournalError> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn u64(&mut self) -> Result<u64, JournalError> {
        let b = self.take(8)?;
        Ok(u64::from_le_bytes([
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
        ]))
    }

    fn f32(&mut self) -> Result<f32, JournalError> {
        let b = self.take(4)?;
        Ok(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    fn remaining(&self) -> usize {
        self.bytes.len() - self.pos
    }
}

impl Journal {
    /// 确定性序列化(v1 冻结布局;全域 to_le_bytes,f32 位型)。
    /// header.frames 必须等于 records.len()(编程面不变量,assert 硬断)。
    pub fn serialize(&self) -> Vec<u8> {
        assert_eq!(
            self.header.frames as usize,
            self.records.len(),
            "header.frames 必须等于 records.len()(journal 不变量)"
        );
        let h = &self.header;
        let mut out =
            Vec::with_capacity(JOURNAL_HEADER_BYTES + self.records.len() * JOURNAL_RECORD_BYTES);
        out.extend_from_slice(&JOURNAL_MAGIC);
        out.extend_from_slice(&JOURNAL_VERSION.to_le_bytes());
        out.extend_from_slice(&h.seed.to_le_bytes());
        out.extend_from_slice(&h.cap.to_le_bytes());
        out.extend_from_slice(&h.frames.to_le_bytes());
        out.extend_from_slice(&h.dt.to_le_bytes());
        out.extend_from_slice(&h.gravity_y.to_le_bytes());
        for v in h.emitter.pos {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in h.emitter.spread {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in h.emitter.vel_base {
            out.extend_from_slice(&v.to_le_bytes());
        }
        for v in h.emitter.vel_spread {
            out.extend_from_slice(&v.to_le_bytes());
        }
        out.extend_from_slice(&h.emitter.life_base.to_le_bytes());
        out.extend_from_slice(&h.emitter.gravity_y.to_le_bytes());
        debug_assert_eq!(out.len(), JOURNAL_HEADER_BYTES, "header 布局漂移");
        for r in &self.records {
            out.extend_from_slice(&r.emit_count.to_le_bytes());
        }
        out
    }

    /// 反序列化(魔数/版本/长度 fail-closed;长度必须精确无尾随)。
    pub fn deserialize(bytes: &[u8]) -> Result<Self, JournalError> {
        let mut r = Reader::new(bytes);
        let magic = r.take(4)?;
        if magic != JOURNAL_MAGIC {
            return Err(JournalError::BadMagic([magic[0], magic[1], magic[2], magic[3]]));
        }
        let version = r.u32()?;
        if version != JOURNAL_VERSION {
            return Err(JournalError::BadVersion(version));
        }
        let seed = r.u64()?;
        let cap = r.u32()?;
        let frames = r.u32()?;
        let dt = r.f32()?;
        let gravity_y = r.f32()?;
        let mut f3 = || -> Result<[f32; 3], JournalError> {
            Ok([r.f32()?, r.f32()?, r.f32()?])
        };
        let pos = f3()?;
        let spread = f3()?;
        let vel_base = f3()?;
        let vel_spread = f3()?;
        let life_base = r.f32()?;
        let e_gravity_y = r.f32()?;
        let mut records = Vec::with_capacity(frames as usize);
        for _ in 0..frames {
            records.push(FrameRecord {
                emit_count: r.u32()?,
            });
        }
        if r.remaining() != 0 {
            return Err(JournalError::TrailingBytes {
                extra: r.remaining(),
            });
        }
        Ok(Self {
            header: JournalHeader {
                seed,
                cap,
                frames,
                dt,
                gravity_y,
                emitter: EmitterDesc {
                    pos,
                    spread,
                    vel_base,
                    vel_spread,
                    life_base,
                    gravity_y: e_gravity_y,
                },
            },
            records,
        })
    }

    /// v1 语义校验(结构面之外的协议不变量;消费方装载期硬校 fail-closed):
    /// frames 头与记录数一致且非零、cap 为 SEG 整倍数 ∈ (0, PARTICLE_CAP_MAX]、
    /// dt 正有限、gravity_y 镜像域位级恒等。
    pub fn validate_v1(&self) -> Result<(), String> {
        let h = &self.header;
        if h.frames as usize != self.records.len() {
            return Err(format!(
                "frames 头 {} ≠ 记录数 {}",
                h.frames,
                self.records.len()
            ));
        }
        if h.frames == 0 {
            return Err("frames 必须 ≥ 1".into());
        }
        let cap = h.cap as usize;
        if cap == 0 || cap % SEG != 0 || cap > PARTICLE_CAP_MAX {
            return Err(format!(
                "cap 必须为 SEG={SEG} 整倍数 ∈ (0, {PARTICLE_CAP_MAX}](得 {cap})"
            ));
        }
        if !(h.dt > 0.0 && h.dt.is_finite()) {
            return Err(format!("dt 必须正有限(得 {})", h.dt));
        }
        if h.gravity_y.to_bits() != h.emitter.gravity_y.to_bits() {
            return Err(format!(
                "gravity_y 镜像域不恒等(header {:e} ≠ emitter {:e};v1 冻结不变量)",
                h.gravity_y, h.emitter.gravity_y
            ));
        }
        Ok(())
    }
}

/// SoA 池检查点(帧 k 开始前态;九流全容量原字节 + pid_base + n_curr——
/// device 面直接由 probe DevState 九缓冲 clone 得,host 面经
/// [`Checkpoint::from_pools`] 得,两面字节布局同一)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checkpoint {
    /// 检查点帧下标 k(该态 = 帧 k 开始前)。
    pub frame: u32,
    /// 帧 k 开始前已发行 pid 计数。
    pub pid_base: u32,
    /// 帧 k 开始前有效粒子数(池 n)。
    pub n_curr: u32,
    /// 九流全容量原字节(len == 9,各 cap·4 B;序 = pos_x/pos_y/pos_z/
    /// vel_x/vel_y/vel_z/age/life/pid,f32 流 LE 位型、pid 流 u32 LE)。
    pub streams: Vec<Vec<u8>>,
}

impl Checkpoint {
    /// host 面捕获(f32 位型序列化;全容量含前缀外字节——与 device readback
    /// 语义一致,恢复后前缀外字节不进 digest 也不被读取)。
    pub fn from_pools(frame: u32, pid_base: u32, p: &ParticlePools) -> Self {
        let f32s = |v: &[f32]| -> Vec<u8> { v.iter().flat_map(|x| x.to_le_bytes()).collect() };
        let streams = vec![
            f32s(&p.pos_x),
            f32s(&p.pos_y),
            f32s(&p.pos_z),
            f32s(&p.vel_x),
            f32s(&p.vel_y),
            f32s(&p.vel_z),
            f32s(&p.age),
            f32s(&p.life),
            p.pid.iter().flat_map(|x| x.to_le_bytes()).collect(),
        ];
        Self {
            frame,
            pid_base,
            n_curr: p.n as u32,
            streams,
        }
    }

    /// host 面恢复(位级重建九流 + n = n_curr;容量由流长推得)。
    pub fn restore_pools(&self) -> ParticlePools {
        assert_eq!(self.streams.len(), POOL_STREAMS, "检查点必须为 9 流");
        let cap = self.streams[0].len() / 4;
        assert!(
            self.streams.iter().all(|s| s.len() == cap * 4),
            "检查点九流长度必须一致(cap·4)"
        );
        assert!(self.n_curr as usize <= cap, "n_curr 超容量");
        let rf = |b: &[u8]| -> Vec<f32> {
            b.chunks_exact(4)
                .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
                .collect()
        };
        let mut p = ParticlePools::with_capacity(cap);
        p.pos_x = rf(&self.streams[0]);
        p.pos_y = rf(&self.streams[1]);
        p.pos_z = rf(&self.streams[2]);
        p.vel_x = rf(&self.streams[3]);
        p.vel_y = rf(&self.streams[4]);
        p.vel_z = rf(&self.streams[5]);
        p.age = rf(&self.streams[6]);
        p.life = rf(&self.streams[7]);
        p.pid = self.streams[8]
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect();
        p.n = self.n_curr as usize;
        p
    }
}

/// 检查点文件("G35C" v1;录制腿落盘、回滚腿消费;确定性字节序同 journal)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CheckpointFile {
    /// 池容量(与 journal.header.cap 互核)。
    pub cap: u32,
    /// 检查点间隔 K(录制脚本冻结面登记)。
    pub interval: u32,
    /// 检查点序列(录制序;frame 严格递增)。
    pub checkpoints: Vec<Checkpoint>,
}

impl CheckpointFile {
    /// 确定性序列化(布局见模块头)。
    pub fn serialize(&self) -> Vec<u8> {
        let cap = self.cap as usize;
        let mut out = Vec::with_capacity(
            20 + self.checkpoints.len() * (12 + POOL_STREAMS * cap * 4),
        );
        out.extend_from_slice(&CHECKPOINT_MAGIC);
        out.extend_from_slice(&CHECKPOINT_VERSION.to_le_bytes());
        out.extend_from_slice(&self.cap.to_le_bytes());
        out.extend_from_slice(&self.interval.to_le_bytes());
        out.extend_from_slice(&(self.checkpoints.len() as u32).to_le_bytes());
        for ck in &self.checkpoints {
            assert_eq!(ck.streams.len(), POOL_STREAMS, "检查点必须为 9 流");
            out.extend_from_slice(&ck.frame.to_le_bytes());
            out.extend_from_slice(&ck.pid_base.to_le_bytes());
            out.extend_from_slice(&ck.n_curr.to_le_bytes());
            for s in &ck.streams {
                assert_eq!(s.len(), cap * 4, "检查点流长度必须为 cap·4");
                out.extend_from_slice(s);
            }
        }
        out
    }

    /// 反序列化(魔数/版本/长度 fail-closed,错类型同 [`JournalError`])。
    pub fn deserialize(bytes: &[u8]) -> Result<Self, JournalError> {
        let mut r = Reader::new(bytes);
        let magic = r.take(4)?;
        if magic != CHECKPOINT_MAGIC {
            return Err(JournalError::BadMagic([magic[0], magic[1], magic[2], magic[3]]));
        }
        let version = r.u32()?;
        if version != CHECKPOINT_VERSION {
            return Err(JournalError::BadVersion(version));
        }
        let cap = r.u32()?;
        let interval = r.u32()?;
        let count = r.u32()?;
        let stream_bytes = cap as usize * 4;
        let mut checkpoints = Vec::with_capacity(count as usize);
        for _ in 0..count {
            let frame = r.u32()?;
            let pid_base = r.u32()?;
            let n_curr = r.u32()?;
            let mut streams = Vec::with_capacity(POOL_STREAMS);
            for _ in 0..POOL_STREAMS {
                streams.push(r.take(stream_bytes)?.to_vec());
            }
            checkpoints.push(Checkpoint {
                frame,
                pid_base,
                n_curr,
                streams,
            });
        }
        if r.remaining() != 0 {
            return Err(JournalError::TrailingBytes {
                extra: r.remaining(),
            });
        }
        Ok(Self {
            cap,
            interval,
            checkpoints,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::core::{FrameStats, frame};
    use super::super::rand_table;
    use super::*;

    /// 单测夹具 journal(冻结常量;probe 有自己的冻结夹具,互不镜像)。
    /// life_base = 0.6 / dt = 0.05 ⇒ 12 帧寿命窗内必有死亡(压缩腿非空转)。
    fn fixture_journal(frames: u32) -> Journal {
        let emitter = EmitterDesc {
            pos: [0.0, 1.0, -0.5],
            spread: [0.4, 0.2, 0.4],
            vel_base: [0.0, 3.0, 0.0],
            vel_spread: [1.0, 0.5, 1.0],
            life_base: 0.6,
            gravity_y: -9.8,
        };
        let records = (0..frames)
            .map(|f| FrameRecord {
                emit_count: 8 + (f * 3) % 16,
            })
            .collect();
        Journal {
            header: JournalHeader {
                seed: 42,
                cap: 512,
                frames,
                dt: 0.05,
                gravity_y: emitter.gravity_y,
                emitter,
            },
            records,
        }
    }

    /// 池有效前缀位级快照(f32 用 to_bits;core.rs 单测同律)。
    fn pool_prefix_bits(p: &ParticlePools) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + p.n * 36);
        out.extend_from_slice(&(p.n as u32).to_le_bytes());
        for i in 0..p.n {
            for v in [
                p.pos_x[i], p.pos_y[i], p.pos_z[i], p.vel_x[i], p.vel_y[i], p.vel_z[i], p.age[i],
                p.life[i],
            ] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&p.pid[i].to_le_bytes());
        }
        out
    }

    /// host 面链式帧 digest(篡改定位/恢复语义单测判据;device 面 digest 由
    /// probe 以同一链式协议对 device 字节计算,两面不互拍只各自成链)。
    fn chain_digest(prev: &str, b: &ParticlePools, stats: &FrameStats) -> String {
        let mut t = Vec::with_capacity(prev.len() + 4 + b.n * 36 + 32);
        t.extend_from_slice(prev.as_bytes());
        t.extend_from_slice(&pool_prefix_bits(b));
        for a in stats.args {
            t.extend_from_slice(&a.to_le_bytes());
        }
        rurix_pkg::sha256::hex_digest(&t)
    }

    /// host 帧循环驱动(帧 from..journal.frames;init = None 全零起步 /
    /// Some = 检查点恢复起步)。返回(逐帧 digest 链, 末态池)。
    fn drive(
        journal: &Journal,
        from: usize,
        init: Option<(&Checkpoint, &str)>,
        to_exclusive: usize,
    ) -> (Vec<String>, ParticlePools) {
        let h = &journal.header;
        let cap = h.cap as usize;
        let table = rand_table(h.seed);
        let (mut a, mut pid_base, mut digest) = match init {
            None => (
                ParticlePools::with_capacity(cap),
                0u32,
                "0".repeat(64),
            ),
            Some((ck, seed)) => (ck.restore_pools(), ck.pid_base, seed.to_string()),
        };
        let mut b = ParticlePools::with_capacity(cap);
        let mut digests = Vec::new();
        for f in from..to_exclusive {
            let emit = journal.records[f].emit_count as usize;
            let stats = frame(&mut a, &mut b, &h.emitter, &table, h.dt, pid_base, emit);
            digest = chain_digest(&digest, &b, &stats);
            digests.push(digest.clone());
            pid_base += emit as u32;
            std::mem::swap(&mut a, &mut b);
        }
        (digests, a)
    }

    /// ① 序列化往返位级:serialize → deserialize → 再 serialize 字节全等 +
    /// 结构全等 + 冻结长度恒等式(88 + 4·frames)。
    #[test]
    fn journal_roundtrip_bitexact() {
        let j = fixture_journal(16);
        let bytes = j.serialize();
        assert_eq!(
            bytes.len(),
            JOURNAL_HEADER_BYTES + 16 * JOURNAL_RECORD_BYTES,
            "v1 冻结长度恒等式破"
        );
        assert_eq!(&bytes[..4], &JOURNAL_MAGIC, "魔数字面");
        let back = Journal::deserialize(&bytes).expect("往返必须可读");
        assert_eq!(back, j, "结构往返全等");
        assert_eq!(back.serialize(), bytes, "字节往返位级全等");
        back.validate_v1().expect("夹具必须过 v1 语义校验");
    }

    /// ② 版本头拒错版 + 魔数/截断/尾随 fail-closed 闭集。
    #[test]
    fn journal_rejects_bad_version_and_malformed() {
        let j = fixture_journal(4);
        let good = j.serialize();
        // 错版(version 字节 [4..8) 改 2)。
        let mut bad_ver = good.clone();
        bad_ver[4..8].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(
            Journal::deserialize(&bad_ver),
            Err(JournalError::BadVersion(2)),
            "错版必须拒读"
        );
        // 错魔数。
        let mut bad_magic = good.clone();
        bad_magic[0] = b'X';
        assert!(
            matches!(Journal::deserialize(&bad_magic), Err(JournalError::BadMagic(_))),
            "错魔数必须拒读"
        );
        // header 截断 / 记录区截断。
        assert!(
            matches!(
                Journal::deserialize(&good[..JOURNAL_HEADER_BYTES - 1]),
                Err(JournalError::Truncated { .. })
            ),
            "header 截断必须拒读"
        );
        assert!(
            matches!(
                Journal::deserialize(&good[..good.len() - 1]),
                Err(JournalError::Truncated { .. })
            ),
            "记录区截断必须拒读"
        );
        // 尾随字节。
        let mut trailing = good.clone();
        trailing.push(0);
        assert_eq!(
            Journal::deserialize(&trailing),
            Err(JournalError::TrailingBytes { extra: 1 }),
            "尾随字节必须拒读"
        );
        // 空流。
        assert!(matches!(
            Journal::deserialize(&[]),
            Err(JournalError::Truncated { .. })
        ));
    }

    /// ③ v1 语义校验红臂:gravity_y 镜像域破 / cap 非 SEG 倍 / frames 计数
    /// 不符 / dt 非正——逐一必拒。
    #[test]
    fn journal_validate_v1_red_arms() {
        let good = fixture_journal(4);
        good.validate_v1().expect("正例必须过");
        let mut g = good.clone();
        g.header.gravity_y = -9.7;
        assert!(g.validate_v1().is_err(), "gravity_y 镜像域破必拒");
        let mut c = good.clone();
        c.header.cap = 100;
        assert!(c.validate_v1().is_err(), "cap 非 SEG 整倍数必拒");
        let mut f = good.clone();
        f.header.frames = 5;
        assert!(f.validate_v1().is_err(), "frames 头与记录数不符必拒");
        let mut d = good.clone();
        d.header.dt = 0.0;
        assert!(d.validate_v1().is_err(), "dt 非正必拒");
    }

    /// ④ 篡改定位(host 面模拟):帧 10 emit_count +1 ⇒ digest 链首异帧
    /// 精确 == 10(之前逐帧全等)——分歧可定位见证的 host 金标准面。
    #[test]
    fn tamper_first_divergence_host_sim() {
        let j = fixture_journal(16);
        let frames = j.header.frames as usize;
        let (chain, _) = drive(&j, 0, None, frames);
        let mut tampered = j.clone();
        tampered.records[10].emit_count += 1;
        let (chain_t, _) = drive(&tampered, 0, None, frames);
        let first_div = chain
            .iter()
            .zip(chain_t.iter())
            .position(|(a, b)| a != b)
            .expect("篡改必须可检出");
        assert_eq!(first_div, 10, "首异帧必须精确定位到篡改帧");
        assert_eq!(&chain[..10], &chain_t[..10], "篡改帧前必须逐帧全等");
        assert_ne!(chain[frames - 1], chain_t[frames - 1], "链尾必异(链式传染)");
    }

    /// ⑤ 检查点恢复语义(host 面模拟):帧 8 检查点恢复 + 输入重放 ⇒
    /// 恢复帧自身与其后逐帧 digest 位级全等 + 末态池前缀位级全等
    /// (B 组清零起步不影响——digest/前缀外字节论证见模块头)。
    #[test]
    fn checkpoint_restore_semantics_host() {
        let j = fixture_journal(12);
        let frames = j.header.frames as usize;
        let h = &j.header;
        let cap = h.cap as usize;
        let table = rand_table(h.seed);
        // 录制跑:逐帧 digest + 每 4 帧检查点(帧开始前捕获)。
        let mut a = ParticlePools::with_capacity(cap);
        let mut b = ParticlePools::with_capacity(cap);
        let mut pid_base = 0u32;
        let mut digest = "0".repeat(64);
        let mut chain = Vec::new();
        let mut checkpoints = Vec::new();
        for f in 0..frames {
            if f % 4 == 0 {
                checkpoints.push(Checkpoint::from_pools(f as u32, pid_base, &a));
            }
            let emit = j.records[f].emit_count as usize;
            let stats = frame(&mut a, &mut b, &h.emitter, &table, h.dt, pid_base, emit);
            digest = chain_digest(&digest, &b, &stats);
            chain.push(digest.clone());
            pid_base += emit as u32;
            std::mem::swap(&mut a, &mut b);
        }
        assert!(a.n > 0, "夹具末态非空(样本量门)");
        // 回滚:检查点 k=8 恢复 → 重放帧 8..12;digest 链种子 = 录制链 [7]。
        let ck = checkpoints.iter().find(|c| c.frame == 8).expect("k=8 检查点在档");
        assert_eq!(ck.pid_base as usize, (0..8).map(|f| j.records[f].emit_count as usize).sum::<usize>());
        let (resim, final_pool) = drive(&j, 8, Some((ck, &chain[7])), frames);
        assert_eq!(resim[0], chain[8], "恢复帧自身 digest 必须位级全等");
        assert_eq!(resim, chain[8..], "重仿真逐帧 digest 必须与录制链位级全等");
        assert_eq!(
            pool_prefix_bits(&final_pool),
            pool_prefix_bits(&a),
            "末态池有效前缀必须位级全等"
        );
        // restore_pools 自身往返:恢复态与捕获态位级一致。
        let rp = ck.restore_pools();
        assert_eq!(Checkpoint::from_pools(8, ck.pid_base, &rp), *ck, "检查点恢复往返位级");
    }

    /// ⑥ 检查点文件往返位级 + 魔数/版本 fail-closed。
    #[test]
    fn checkpoint_file_roundtrip_and_reject() {
        let mut p = ParticlePools::with_capacity(256);
        p.pos_x[0] = 1.5;
        p.pid[0] = 7;
        p.n = 1;
        let ckf = CheckpointFile {
            cap: 256,
            interval: 4,
            checkpoints: vec![
                Checkpoint::from_pools(0, 0, &ParticlePools::with_capacity(256)),
                Checkpoint::from_pools(4, 32, &p),
            ],
        };
        let bytes = ckf.serialize();
        let back = CheckpointFile::deserialize(&bytes).expect("往返必须可读");
        assert_eq!(back, ckf, "结构往返全等");
        assert_eq!(back.serialize(), bytes, "字节往返位级全等");
        let mut bad_ver = bytes.clone();
        bad_ver[4..8].copy_from_slice(&9u32.to_le_bytes());
        assert_eq!(
            CheckpointFile::deserialize(&bad_ver),
            Err(JournalError::BadVersion(9))
        );
        let mut bad_magic = bytes.clone();
        bad_magic[0] = b'X';
        assert!(matches!(
            CheckpointFile::deserialize(&bad_magic),
            Err(JournalError::BadMagic(_))
        ));
        assert!(matches!(
            CheckpointFile::deserialize(&bytes[..bytes.len() - 4]),
            Err(JournalError::Truncated { .. })
        ));
        let mut trailing = bytes.clone();
        trailing.extend_from_slice(&[0, 0]);
        assert_eq!(
            CheckpointFile::deserialize(&trailing),
            Err(JournalError::TrailingBytes { extra: 2 })
        );
    }
}
