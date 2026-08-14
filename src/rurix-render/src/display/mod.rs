//! 显示管线（G9.5 M118；RFC-0025 D4 伞形 §4.I；spec/display_pipeline.md
//! RXS-0369 逐条对齐）。
//!
//! - [`view_transform`] = `ViewTransform` trait 插件面 + 注册表（未注册名调用
//!   拒录 RED）+ 三输出编码共享段 + golden 输入集/canonical 帧；
//! - [`aces13`] / [`aces20`] / [`agx`] / [`neutral`] = 四内置插件（host 参考
//!   公式逐字移植;AgX 对比度补偿参数随 [`agx::AgxLook`] 资产化）;
//! - [`swapchain`] = SDR/scRGB/PQ 三交换链路径闭集 + 运行时切换确定性 +
//!   合法性闭集（非 HDR 交换链携带 PQ 输出即 RED）+ HDR 元数据输出变换阶段
//!   填写 + HDR 设备标定层 NotTriggered 显式登记（不充绿、不反向否决 SDR
//!   验证面）。
//! - [`skin`] = M115 皮肤 Burley 屏单 pass separable SSS（RXS-0373:颜色/深度
//!   双 kernel + 扩散 profile 资产化经 §4.L 侧表通道 + pre-integrated LUT 回退
//!   档 + 全零衰减退化纯漫反射 RED + 32B 0-byte 机核）。
//!
//! 纪律:host 纯 safe 确定性（全库 `forbid(unsafe_code)`）；零新 FFI；无
//! device 依赖——M118 语义面 = view transform 数学 + 交换链路径状态机,窗口
//! 腿维持 D-130 红线（C++ shim）0-byte;`RURIX_REQUIRE_REAL=1` 下以 host 确定
//! 性为准,validation 不适用。golden = 参考公式 host 逐字实现 + measured 冻结
//! digest（双跑位级一致后冻结,禁手写）+ provenance。

pub mod aces13;
pub mod aces20;
pub mod agx;
pub mod color;
pub mod neutral;
pub mod post_chain;
pub mod skin;
pub mod swapchain;
pub mod view_transform;
