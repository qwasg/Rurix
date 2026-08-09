//! World-Field 唯一出口(RFC-0024 §4.B4,R-10 🔒;骨架期提交口骨架)。
//!
//! 冻结纪律:
//! - **唯一合法路径 = Physics→GpuScene 桥**:场采样参数按 tick 提交为
//!   GpuScene 只读 buffer;渲染侧零回写(纪律 1 单向事实源 0-byte)。
//! - 骨架期 = **提交口骨架 + 零扩面登记**:本模块只构造
//!   [`WorldFieldBuffer`](加性提交载荷)并把它挂到既有桥的类型面上
//!   ([`PhysicsBridge::submit_world_field`] 加性方法);**GpuScene 冻结面
//!   0-byte 不动**——骨架期不向 GpuScene 增加任何 buffer 字段(R-10 🔒
//!   字面:扩面须渲染侧 RFC 显式修订行,骨架期预期 0-byte 扩面,登记
//!   即止)。
//! - 时间域:`WorldFieldSampleSet` 归属 `RenderFrameId`(cloth 模块既有
//!   newtype 复用),记录消费的 `PhysicsTickId` 与映射 render frame,
//!   禁隐式「当前帧」读取(R-4 🔒)。

use rurix_pkg::sha256::{digest, hex};

use crate::cloth::RenderFrameId;
use crate::net::frame::PhysicsTickId;

/// World-Field 采样集时间域记录(R-4 🔒 骨架面:消费的 physics tick +
/// 归属 render frame 显式成对,禁隐式当前帧)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldFieldSampleSet {
    /// 场参数提交 tick(消费的物理时间域)。
    pub physics_tick: PhysicsTickId,
    /// 归属渲染帧(本采样集的时间域归属)。
    pub render_frame: RenderFrameId,
}

/// GpuScene 只读 buffer 提交载荷(骨架期 = 规范字节 + digest;**不触
/// GpuScene 冻结面**,渲染侧消费归完整期)。
#[derive(Debug, Clone, PartialEq)]
pub struct WorldFieldBuffer {
    /// 时间域记录。
    pub sample_set: WorldFieldSampleSet,
    /// 场采样参数 canonical 字节(场 digest × 参数位表示)。
    pub payload: Vec<u8>,
}

impl WorldFieldBuffer {
    /// 载荷 digest(提交对拍锚)。
    pub fn digest(&self) -> String {
        let mut pre = Vec::new();
        pre.extend_from_slice(&self.sample_set.physics_tick.0.to_le_bytes());
        pre.extend_from_slice(&self.sample_set.render_frame.0.to_le_bytes());
        pre.extend_from_slice(&self.payload);
        hex(&digest(&pre))
    }
}

/// 提交口骨架(构造 + digest 对拍;**只读提交,渲染侧零回写**——
/// 本类型无任何渲染回写通道,桥侧只把载荷登记进 `PhysicsBridge` 的
/// 帧报告,GpuScene 面 0-byte)。
#[derive(Debug, Default)]
pub struct WorldFieldSubmitter {
    committed: Vec<WorldFieldBuffer>,
}

impl WorldFieldSubmitter {
    /// 空提交口。
    pub fn new() -> Self {
        Self::default()
    }

    /// 按 tick 提交(顺序 = 提交序;确定性面)。
    pub fn submit(&mut self, buffer: WorldFieldBuffer) {
        self.committed.push(buffer);
    }

    /// 已提交载荷(只读)。
    pub fn committed(&self) -> &[WorldFieldBuffer] {
        &self.committed
    }

    /// 提交序列 digest(门对拍锚)。
    pub fn sequence_digest(&self) -> String {
        let mut buf = String::new();
        for b in &self.committed {
            buf.push_str(&b.digest());
            buf.push('\n');
        }
        hex(&digest(buf.as_bytes()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submit_sequence_deterministic() {
        let mut a = WorldFieldSubmitter::new();
        let mut b = WorldFieldSubmitter::new();
        for i in 0..3u64 {
            let mk = |tick: u64| WorldFieldBuffer {
                sample_set: WorldFieldSampleSet {
                    physics_tick: PhysicsTickId(tick),
                    render_frame: RenderFrameId(tick * 2),
                },
                payload: vec![tick as u8; 4],
            };
            a.submit(mk(i));
            b.submit(mk(i));
        }
        assert_eq!(a.sequence_digest(), b.sequence_digest());
        assert_eq!(a.committed().len(), 3);
    }
}
