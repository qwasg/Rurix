//! device 执行入口(gate `d3d12-runtime`;真出图 gate `real-shim`;RFC-0006 §9 Q-Gate /
//! G2.4 选项 B:不采样 G-buffer 的最小多 pass deferred)。
//!
//! **G-G2-4 防降级硬门兑现**:hardware 多 pass deferred draw + offscreen readback + 像素
//! 对照经 **Rurix source → 图形=B DXIL(RXS-0171 body 降级 + RXS-0172/0173 签名保真)→
//! RFC-0005 RTS0 → D3D12 PSO → hardware 多 pass deferred draw → offscreen readback** 全链
//! 兑现(`real-shim` 下经 [`crate::device`] FFI 调 `shim/uc04_offscreen.cpp`)。
//! **不**以手写 HLSL/DXIL、CPU 预填、单 pass、fullscreen copy、固定像素、host-only 模拟、
//! 窗口截图或 SKIP 伪造 device 绿:VS/FS 全部来自 Rurix 源经图形=B DXIL 容器
//! (`rurixc::dxil_codegen::emit_dxil_b_container`,见 `cargo example emit_uc04_dxil`)。
//!
//! 缺 `real-shim`(无 MSVC/D3D12 SDK)→ [`Uc04Error::ShimUnavailable`](环境缺失,非语言 RX,
//! 不伪造 device 绿);shim 真跑失败 → [`Uc04Error::DeviceRunFailed`]。
//!
//! **选项 B 折中边界**:lighting/合成 pass 走自身插值输入,**不采样 G-buffer**(真采样触
//! RD-021 / 06§4.2 纹理路径内存模型禁区,本期 defer);几何 pass 真写 G-buffer MRT,
//! lighting pass 真出 final,两 pass 均 Rurix 源 DXIL,真 hardware 多 pass draw。

use crate::barrier::BarrierAnchor;
use crate::deferred::DeferredPlan;
use crate::error::Uc04Error;
use crate::pso::AssembledPso;
use crate::readback::ReadbackLayout;

/// UC-04 offscreen shim C ABI 版本(与 `shim/uc04_offscreen.cpp` `kAbiVersion` 一致)。
pub const RX_UC04_ABI_VERSION: u32 = 1;

/// offscreen 出图请求(host 侧已校验的装配/编排/barrier/readback 产物 + Rurix 图形=B
/// DXIL 着色器对象字节 + 尺寸)。
pub struct OffscreenRequest<'a> {
    /// RXS-0167 装配出的 graphics PSO 描述(`rts0_bytes` = RFC-0005 RTS0,P-11 单一事实源)。
    pub pso: &'a AssembledPso,
    /// RXS-0168 校验通过的 deferred 编排计划。
    pub plan: &'a DeferredPlan,
    /// RXS-0169 校验通过的 barrier 锚点集。
    pub barriers: &'a [BarrierAnchor],
    /// RXS-0170 校验通过的 readback 布局。
    pub readback: &'a ReadbackLayout,
    /// offscreen 宽度(像素)。
    pub width: u32,
    /// offscreen 高度(像素)。
    pub height: u32,
    /// 几何 pass vertex DXIL 容器字节(Rurix 源经图形=B,`uc04_gbuffer_vs.rx`)。
    pub geom_vs_dxil: &'a [u8],
    /// 几何 pass fragment DXIL 容器字节(写 G-buffer MRT,`uc04_gbuffer_fs.rx`)。
    pub geom_fs_dxil: &'a [u8],
    /// lighting/合成 pass vertex DXIL 容器字节(`uc04_lighting_vs.rx`)。
    pub light_vs_dxil: &'a [u8],
    /// lighting/合成 pass fragment DXIL 容器字节(不采样 G-buffer,`uc04_lighting_fs.rx`)。
    pub light_fs_dxil: &'a [u8],
}

/// offscreen 真跑结果(device 见证):adapter 名 + G-buffer albedo 中心像素 + final 中心像素。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OffscreenResult {
    /// 选中的硬件 adapter 名(device 见证)。
    pub adapter: String,
    /// 几何 pass G-buffer albedo 中心像素 RGBA8(证几何 pass FS 写 MRT)。
    pub gbuffer_albedo: [u8; 4],
    /// lighting/合成 final 中心像素 RGBA8(证 lighting pass FS 出图)。
    pub final_pixel: [u8; 4],
}

#[cfg(feature = "real-shim")]
#[allow(unsafe_code)] // FFI extern 块(D3D12 shim 边界);unsafe-audit/uc04-demo.md U24。
mod ffi {
    unsafe extern "C" {
        pub fn rx_uc04_abi_version() -> u32;

        #[allow(clippy::too_many_arguments)]
        pub fn rx_uc04_offscreen_run(
            abi_version: u32,
            width: u32,
            height: u32,
            rts0: *const u8,
            rts0_len: usize,
            geom_vs: *const u8,
            geom_vs_len: usize,
            geom_fs: *const u8,
            geom_fs_len: usize,
            light_vs: *const u8,
            light_vs_len: usize,
            light_fs: *const u8,
            light_fs_len: usize,
            out_gbuffer_pixel: *mut u8,
            out_final_pixel: *mut u8,
            out_adapter: *mut u8,
            out_adapter_cap: usize,
        ) -> i32;
    }
}

/// 把以 NUL 结尾的 UTF-8 字节缓冲转为 [`String`](截到首个 NUL;无 NUL 取全长)。
#[cfg(feature = "real-shim")]
fn cstr_to_string(buf: &[u8]) -> String {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    String::from_utf8_lossy(&buf[..end]).into_owned()
}

/// device offscreen 出图 + 像素回读(G-G2-4:真 hardware 多 pass deferred draw)。
///
/// `real-shim` 下经 D3D12 shim 真跑:几何 pass(Rurix VS/FS)写 G-buffer MRT → lighting/合成
/// pass(Rurix VS/FS,不采样 G-buffer = 选项 B)写 final → 手动 barrier(RXS-0169)→ offscreen
/// readback 取 albedo 与 final 中心像素。
///
/// # Errors
/// - 缺 `real-shim`(无 MSVC/D3D12 SDK)→ [`Uc04Error::ShimUnavailable`](环境缺失,非语言 RX,
///   不伪造 device 绿)。
/// - shim 真跑失败(adapter/PSO/draw/readback 返回非 0)→ [`Uc04Error::DeviceRunFailed`]。
#[cfg_attr(feature = "real-shim", allow(unsafe_code))] // unsafe-audit/uc04-demo.md U24。
pub fn execute_offscreen(req: &OffscreenRequest<'_>) -> Result<OffscreenResult, Uc04Error> {
    #[cfg(feature = "real-shim")]
    {
        let mut gbuffer = [0u8; 4];
        let mut final_px = [0u8; 4];
        let mut adapter_buf = [0u8; 256];
        // SAFETY: 全部入参指针指向本调用栈上有效存储——req.* 字节切片为只读有效内存(配对
        // 长度参数 = 各切片实际 `len()`),out_gbuffer_pixel/out_final_pixel 为 4 字节可写数组、
        // out_adapter 为 256 字节可写缓冲(配对 cap)。shim 按版本化 C ABI(`rx_uc04_abi_version`
        // = `RX_UC04_ABI_VERSION` = 1,首参核对)只读入 DXIL/RTS0 字节、回填 out 像素与 adapter
        // 名,不持有任何指针越出本调用;返回 i32 状态码(0=成功)。unsafe-audit/uc04-demo.md U24。
        let code = unsafe {
            ffi::rx_uc04_offscreen_run(
                RX_UC04_ABI_VERSION,
                req.width,
                req.height,
                req.pso.rts0_bytes.as_ptr(),
                req.pso.rts0_bytes.len(),
                req.geom_vs_dxil.as_ptr(),
                req.geom_vs_dxil.len(),
                req.geom_fs_dxil.as_ptr(),
                req.geom_fs_dxil.len(),
                req.light_vs_dxil.as_ptr(),
                req.light_vs_dxil.len(),
                req.light_fs_dxil.as_ptr(),
                req.light_fs_dxil.len(),
                gbuffer.as_mut_ptr(),
                final_px.as_mut_ptr(),
                adapter_buf.as_mut_ptr(),
                adapter_buf.len(),
            )
        };
        if code != 0 {
            return Err(Uc04Error::DeviceRunFailed {
                code,
                detail: format!(
                    "rx_uc04_offscreen_run 返回 {code}(adapter 选取 / RTS0 解析 / PSO 装配 / \
                     多 pass draw / readback 失败;非 0 即真实 device 失败,不伪造绿)"
                ),
            });
        }
        let adapter = cstr_to_string(&adapter_buf);
        Ok(OffscreenResult {
            adapter,
            gbuffer_albedo: gbuffer,
            final_pixel: final_px,
        })
    }
    #[cfg(not(feature = "real-shim"))]
    {
        let _ = req;
        Err(Uc04Error::ShimUnavailable {
            detail: "real-shim feature 未编入(device 真出图需 --features real-shim + MSVC + \
                     Windows SDK D3D12);按 G-G2-4 防降级硬门标环境缺失,不以替代物伪造 device 绿"
                .to_owned(),
        })
    }
}

/// shim ABI 版本核对(`real-shim` 下查询 C 侧版本,确保 Rust↔C ABI 一致)。
#[cfg(feature = "real-shim")]
#[cfg_attr(feature = "real-shim", allow(unsafe_code))] // unsafe-audit/uc04-demo.md U24。
pub fn shim_abi_version() -> u32 {
    // SAFETY: `rx_uc04_abi_version` 无参纯返回 C 侧编译期常量(kAbiVersion),无副作用、
    // 不解引用任何指针。unsafe-audit/uc04-demo.md U24。
    unsafe { ffi::rx_uc04_abi_version() }
}

/// 本 build 是否含真实 D3D12 offscreen shim(`real-shim` feature)。
pub const fn has_real_shim() -> bool {
    cfg!(feature = "real-shim")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Format;
    use crate::barrier::{BarrierAnchor, BarrierTransition, ResourceState};
    use crate::deferred::{DeferredPlan, GBufferTarget};
    use crate::pso::AssembledPso;
    use crate::readback::ReadbackLayout;
    use rurixc::binding_layout::RootSignature;

    fn sample_request_parts() -> (
        AssembledPso,
        DeferredPlan,
        Vec<BarrierAnchor>,
        ReadbackLayout,
    ) {
        let pso = AssembledPso {
            root_signature: RootSignature {
                parameters: Vec::new(),
                flags: 0,
            },
            rts0_bytes: Vec::new(),
            rtv_formats: vec![Format::Rgba8Unorm],
            dsv_format: Some(Format::D32Float),
        };
        let plan = DeferredPlan {
            gbuffer_color: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
            has_depth: true,
            lighting_srv: vec![GBufferTarget::Albedo, GBufferTarget::Normal],
        };
        let barriers = vec![BarrierAnchor {
            at: "after-lighting",
            transition: BarrierTransition {
                resource: "lighting_out".to_owned(),
                from: ResourceState::RenderTarget,
                to: ResourceState::CopySource,
            },
        }];
        let readback = ReadbackLayout {
            row_pitch: 256,
            buffer_size: 256 * 64,
            format: Format::Rgba8Unorm,
        };
        (pso, plan, barriers, readback)
    }

    /// device 段:无 `real-shim` 时 execute_offscreen 显式返回 ShimUnavailable(环境缺失
    /// sentinel,非语言 RX),**不**伪造 device 绿(G-G2-4 防降级硬门)。real-shim build
    /// 跳过此断言(真跑路径由 ci/dxil_uc04_device_smoke.py 覆盖)。
    //@ spec: RXS-0170
    #[test]
    fn device_path_shim_unavailable_without_real_shim() {
        if has_real_shim() {
            return; // real-shim build:真跑路径由 device smoke 覆盖,不在此断言。
        }
        let (pso, plan, barriers, readback) = sample_request_parts();
        let req = OffscreenRequest {
            pso: &pso,
            plan: &plan,
            barriers: &barriers,
            readback: &readback,
            width: 64,
            height: 64,
            geom_vs_dxil: &[],
            geom_fs_dxil: &[],
            light_vs_dxil: &[],
            light_fs_dxil: &[],
        };
        let err = execute_offscreen(&req).expect_err("无 real-shim 时 device 段须 ShimUnavailable");
        assert!(matches!(err, Uc04Error::ShimUnavailable { .. }));
        // 环境缺失 sentinel 非语言诊断码(不伪造 device 绿、不滥发 RX)。
        assert_eq!(err.rx_code(), None);
    }
}
