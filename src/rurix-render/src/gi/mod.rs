//! 屏幕探针 GI(报告2 P0–P1;RFC-0016 章 E)。
//!
//! 1/16 均匀屏幕探针 + ray query 单反弹 + SH(L1)+ 平面加权插值 + 3×3 探针
//! 空间滤波 + 时域累积。追踪层统一契约 =「输出命中点辐射度」;探针历史为
//! 跨帧外部资源。host 侧提供 CPU 参考追踪器(方向一致性对拍金标准)。
//!
//! 本波落地(W2-I host 波,G5.3-E host 面,报告2 P0–P1 全组件 host 完整实现;
//! device 腿 W3 接线):
//! - [`tracer`]:追踪层统一契约 [`tracer::RadianceTracer`](P0 冻结,SDF/
//!   ReSTIR 未来同接口可替换)+ 本期唯一实现 [`tracer::RayTracedRadiance`]
//!   (TLAS 命中 → 方向光直接光照 × 阴影可见性;未命中 → 天空常量色);
//! - [`probe`]:1/16 均匀探针放置(像素中心锚定 + 相机逆投影)+ 余弦加权
//!   探针追踪(Pcg32 固定种子 + 探针索引去相关);
//! - [`sh`]:SH L1 投影/余弦卷积求值(Ramamoorthi–Hanrahan 系数);
//! - [`interpolate`]:平面加权 2×2 双线性插值(薄几何泄漏缓解);
//! - [`filter`]:探针空间 3×3 深度/法线相似性滤波;
//! - [`temporal`]:探针 SH + 像素 irradiance 时域累积(一律经 temporal
//!   公共底座重投影验证,禁私写重投影;历史 = 跨帧外部资源双缓冲);
//! - [`pipeline`]:单反弹闭环组装 + host 对拍工具(蛮力参考/GBuffer 合成)。
//!
//! 下一波(W3 device 腿,不在本模块):ray query 计算着色器管线、探针纹理
//! 图集与历史双缓冲资源接线、device/host 方向一致性对拍(G-G5-6)。

pub mod filter;
pub mod interpolate;
pub mod pipeline;
pub mod probe;
pub mod sh;
pub mod temporal;
pub mod tracer;
