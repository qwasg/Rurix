//! 时域重建(报告7 P0–P1;RFC-0016 章 H)。
//!
//! 公共底座(完整 MV + Halton jitter + 深度/法线历史验证 + disocclusion +
//! 邻域裁剪)+ TAA + TSR 类超分 + UpscaleBackend trait(自研主实现,vendor
//! 后端留口)。历史颜色/深度为跨帧外部资源;禁效果 pass 私写重投影(全部
//! 时域滤波复用本底座)。
//!
//! G5.2-H 前半(报告7 P0,已落地):[`image`](f32 像素容器)+ [`common`]
//! (Halton jitter/最小 Mat4/相机 MV/历史验证/YCoCg 邻域裁剪/TemporalFrameDesc
//! 图集成描述)+ [`taa`](参考实现与静态收敛验收,门 G-G5-7)。host 纯 Rust
//! CPU 参考实现,同时是 device shader 的对拍金标准与单测载体。
//!
//! G5.3-H 后半 W2-K(报告7 P1,本波):[`upscale`](冻结后端接口
//! UpscaleBackend,RFC-0016 §4.0-3;vendor 后端 FSR 3.1/DirectSR 留口,
//! 本期不接 SDK,接入评估归 RD-037+ 存续)+ [`tsr`](自研 TSR 类主实现:
//! 输出分辨率常驻历史 + jitter 对齐 Catmull-Rom 重采样 + 闪烁时域分析 +
//! reactive mask 双通道,不做锐化)+ [`ssim`](G-G5-7 静态收敛 SSIM 门禁)。

pub mod abi;
pub mod cas;
pub mod common;
pub mod contract;
pub mod image;
pub mod ssim;
pub mod taa;
pub mod tsr;
pub mod upscale;
