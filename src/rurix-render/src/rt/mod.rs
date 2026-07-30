//! 光追效果与 AS 管理(报告4 P0–P1;RFC-0016 章 F)。
//!
//! AS 管理器(BLAS 缓存网格哈希 / 动态 refit 分级 / TLAS 快速重建)+ ray query
//! 封装 + 效果(RTAO/硬阴影)+ 时域滤波。DXIL RT 腿维持 blocked(RD-034),
//! device 腿全走 Vulkan(rurix-rt vk 底座)。host 侧提供 CPU 参考追踪作对拍金标准。
//!
//! 本波落地(P0 前半,host 纯 Rust,零外部依赖,`forbid(unsafe_code)`):
//! - [`bvh`]:最小 BVH(三角形 BVH 中位数切分 SAH 简化版 + 两级 TLAS 遍历)——
//!   W2 device ray query 效果的对拍金标准几何内核;
//! - [`as_manager`]:AS 生命周期策略单源——`BlasKey` 网格哈希缓存、
//!   `DynamicPolicy` 动态分级(Static/Deformable 队列轮转/FullRebuild)、
//!   `TlasBuilder` 每帧快速重建(实例变换标脏)、`AsStats` evidence 计数面;
//! - [`ref_tracer`]:CPU 参考效果(`rtao_reference` / `hard_shadow_reference`,
//!   PCG32 确定性采样)——签名与语义即 W2-J device 效果的对拍契约。
//!
//! 本波落地(P0 后半,报告4 §3.2 效果 pass + 时域降噪;RFC-0016 §4.F F2,
//! host 纯 Rust,零外部依赖,`forbid(unsafe_code)`):
//! - [`effects`]:GBuffer 驱动的 RTAO / 硬阴影效果 pass(逐像素反投影世界
//!   位置 + 法线半球余弦加权 any_hit;RNG 调度与 ref_tracer 对齐——frame 0
//!   同 seed 位级一致,frame_index 混入种子帧间去相关)+ `gbuffer_from_scene`
//!   同源 GBuffer 生成辅助(对拍几何同源保证)+ `EffectStats` 度量埋点;
//! - [`denoise`]:单通道效果时域滤波(重投影/历史验证/邻域方差裁剪全经
//!   temporal 公共底座,禁私写重投影——RFC 章 H 纪律,G-G5-7 审计点)+
//!   5×5 深度相似邻域空间补洞(P1 最小)。
//!
//! 下一波(device 腿,不在本模块):Vulkan ray query 封装、AS 句柄/scratch/
//! VkCompaction、RHI 效果 pass 与时域滤波。

pub mod as_manager;
pub mod bvh;
pub mod denoise;
pub mod effects;
pub mod ref_tracer;
