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
//! 可检测产物 digest + 匹配深度容差带;harness = `bin/g9_m98_fallback_chain`);
//! [`spg_rc`] = M99 屏幕级 SPG 自适应细分 + Radiance Cache 双级 host 面
//! (RXS-0360:16 px/probe 基线 + 深度/法线不连续性 + radiance 方差判据闭集
//! 自适应细分 + 3×3 probe 空间滤波〔G8 底座权重律同面增量〕+ 屏幕 tile 级
//! 缓存〔temporal 公共底座历史复用,禁私写重投影 D2-Q14〕+ 世界级 clipmap
//! not-triggered 登记 + 第一反弹 BRDF×入射光 product IS〔关 ⇒ 方差回归 RED〕;
//! device kernel = `kernels/g9_m99_spg_probe.rx`,harness =
//! `bin/g9_m99_spg_radiance_cache`);[`multi_light`] = M100 低档多灯直接光
//! host 面(RXS-0361:多灯 fixture〔cornell 几何 + 4 光源 quad〕+ MegaLights
//! 式固定随机选灯默认档〔种子流固定双跑逐位一致〕+ 验证射线零跳过硬契约
//! 〔D2-Q4,逐样本发行记录 diag 归约逐灯计数非空;跳验证/灯子集注入 RED〕+
//! 高档 ReSTIR workload 证据不足 not-triggered 登记;device kernel =
//! `kernels/g9_m100_multi_light.rx`,harness = `bin/g9_m100_multi_light_low`);
//! [`if_tier`] = M101 IF 体素网格 + 档位阶梯 host 面(RXS-0362:八面体编解码
//! 单一源〔线性域,往返误差界单测锚定〕+ 4×4×4 体素网格〔irradiance 8×8 +
//! visibility 16×16 防漏光优先 + 每帧轮换更新摊销〕+ 档位阶梯 L0~L3 闭集
//! 〔共享 probe 着色/八面体内核同一函数实例断言,只换空间索引〕+ 每档 AS
//! 更新预算行消费 AsStats〔超预算强制逐级降档显式记录,禁静默降档〕;
//! device kernel = `kernels/g9_m101_probe_oct.rx`,harness =
//! `bin/g9_m101_if_tier_ladder`)。

pub mod fallback_chain;
pub mod filter;
pub mod if_tier;
pub mod interpolate;
pub mod multi_light;
pub mod path_trace;
pub mod pipeline;
pub mod probe;
pub mod sh;
pub mod spg_rc;
pub mod surface_cache;
pub mod temporal;
pub mod tracer;
