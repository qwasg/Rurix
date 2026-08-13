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
//!
//! G9.4 波(GI 语义波,spec/global_illumination.md):[`path_trace`] = M96 M17
//! Path Tracer 参照器 host 面(RXS-0357:确定性冻结场景 fixtures + PCG32 流布局 +
//! host oracle + pbrt-v4 对照/容差带面;device megakernel =
//! `kernels/g9_m96_path_tracer.rx`,harness = `bin/g9_m96_path_tracer`);
//! [`surface_cache`] = M97 Surface Cache host 面(RXS-0358:离线 Card 参数化器
//! [≤12/mesh 可配 fail-closed] + RXPL v2 图集页打包 + 漏光检测/空洞注入 +
//! 匹配深度容差带 + capture host oracle;device kernels =
//! `kernels/g9_m97_cache_{capture,render}.rx`,harness = `bin/g9_m97_surface_cache`);
//! [`fallback_chain`] = M98 四级追踪降级链 host 面(RXS-0359:L1 Screen Trace
//! [屏幕高度场 march,host 参照 + device kernel `kernels/g9_m98_screen_trace.rx`]→
//! L2 SWRT[host 解析场景暴力求值,BVH 金标准对拍]→ L3 HWRT[device RayQuery,
//! `kernels/g9_m98_hwrt.rx`,含 hit lighting 档]→ L4 Far Field[HLOD 未就绪 ⇒
//! not-triggered 登记];选档器 + 逐档计数面 + 转移日志禁静默回退审计 + 逐级强关
//! 可检测产物 digest + 匹配深度容差带;harness = `bin/g9_m98_fallback_chain`)。

pub mod fallback_chain;
pub mod filter;
pub mod interpolate;
pub mod path_trace;
pub mod pipeline;
pub mod probe;
pub mod sh;
pub mod surface_cache;
pub mod temporal;
pub mod tracer;
