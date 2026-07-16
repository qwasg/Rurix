//! `uc04-demo` — UC-04 deferred 渲染器 host 侧 safe 装配/编排模型
//! (RXS-0167~0170;承 [RFC-0006](../../../rfcs/0006-uc04-deferred-renderer.md),
//! owner Approved 2026-06-28;**PR-F2 blocked-honest interim slice**,owner 2026-06-29
//! 裁定)。
//!
//! # 职责(host 侧可判定的装配/编排一致性模型)
//! - **RXS-0167**:DXIL + RTS0 → graphics PSO 装配一致性([`pso::assemble_graphics_pso`])。
//! - **RXS-0168**:deferred 多 pass 编排([`deferred::plan_deferred_passes`];几何 MRT →
//!   lighting 采样 G-buffer → readback)。
//! - **RXS-0169**:资源状态 + barrier 编排锚点([`barrier::plan_barriers`];RT→SRV→RT /
//!   Copy / Readback;首期手动编排,自动状态跟踪 defer RD-020)。
//! - **RXS-0170**:offscreen readback 缓冲布局([`readback::plan_readback`])。
//!
//! # blocked-honest 边界(G-G2-4 防降级硬门)
//! **device 段(hardware 多 pass deferred draw + offscreen 像素对照)阻塞于 RD-013**——
//! 图形=B 入口 body 数据流降级未实现(`rurixc::dxil_spirv::emit_spirv` 仅产接口 + 平凡
//! `main`),无 Rurix 自产可出图着色器。device 执行入口 [`device::execute_offscreen`]
//! (gate `d3d12-runtime`)显式返回 [`Uc04Error::BlockedOnRd013`],**不**以手写 HLSL/DXIL、
//! CPU 预填、单 pass、fullscreen copy、固定像素、host-only 模拟、窗口截图或 SKIP 伪造
//! device 绿(G2_CONTRACT G-G2-4 / CI_GATES 步骤 48)。device 真跑 + CI step 48 + golden
//! bless + G-G2-4 签字归 RD-013 解锁后的 device PR + owner。
//!
//! # 纯 host/safe(零新 unsafe)+ P-11 单一事实源
//! 全 crate 无 `unsafe`(workspace `unsafe_code = "deny"` 继承,无 FFI 执行 → 不消费
//! unsafe-audit U23)。绑定布局取 RFC-0005 编译期推导([`rurixc::binding_layout`]),
//! 运行时不手维护第二份(P-11)。

pub mod barrier;
pub mod deferred;
pub mod error;
pub mod pso;
pub mod readback;

#[cfg(feature = "d3d12-runtime")]
pub mod device;

pub use error::Uc04Error;

/// 渲染目标 / readback 像素格式(最小建模;🔒 非 stable ABI 冻结)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 8-bit RGBA UNORM(albedo / lighting 输出)。
    Rgba8Unorm,
    /// 16-bit RGBA FLOAT(normal G-buffer)。
    Rgba16Float,
    /// 32-bit depth FLOAT(depth G-buffer)。
    D32Float,
}

impl Format {
    /// 每像素字节数(readback 行距对齐用;🔒 非 ABI 冻结)。
    pub fn bytes_per_pixel(self) -> u32 {
        match self {
            Format::Rgba8Unorm => 4,
            Format::Rgba16Float => 8,
            Format::D32Float => 4,
        }
    }
}
