//! RXS-0220~0222(G3.2 present 面,RFC-0013 §4.A):UC-04 可见窗口 flip-model swapchain
//! present 的 **host 侧 safe 装配/重建核验模型**。
//!
//! §4.A 把 §3 裁决地基 Q-Present=offscreen-first(窗口 present 登 RD-019)按其
//! backfill_condition 全量兑现:UC-04 deferred 渲染器从「离屏出图 + 回读断言」升级为
//! 「可见窗口逐帧呈现 + 回读断言」+ 拖动 resize 重建 + Vulkan `OUT_OF_DATE` 重建收尾。
//!
//! **语言面零新语法**(D-130):`.rx` 侧 present 面维持 RXS-0197/0198 typestate 0-byte,
//! 全部增量在 C++ shim / rurix-rt 运行时层——**UC-04 窗口 present 是纯 D3D12 图形管线,
//! 独立走 C++ shim,不实例化 RXS-0197 的 CUDA↔D3D12 interop present typestate**(SC-5)。
//!
//! 本模块只承诺 host 侧可判定的:
//! - **RXS-0220**:present 会话装配(flip-model swapchain desc ↔ lighting final RT 格式/
//!   缓冲数一致性 + swap effect + 逐帧 RT→COPY_SOURCE→PRESENT 迁移锚点)([`assemble_present`])。
//! - **RXS-0221**:swapchain 失效与重建(D3D12 ResizeBuffers / Vulkan OUT_OF_DATE 协商 →
//!   重建后格式/缓冲数恒定 + 视图重建核验)([`classify_swapchain_status`] / [`rebuild_swapchain`])。
//! - **RXS-0222**:present headless readback 断言点纪律(首帧 / 重建后首帧 / 末帧 ≥3 点,
//!   布局复用 RXS-0170)([`ReadbackCadence`])。
//!
//! device N 帧逐帧 `S_OK` + readback 数值断言 + resize 重建后再断言由
//! [`ci/uc04_present_smoke.py`](../../../ci/uc04_present_smoke.py)(步骤 61)覆盖(有显示
//! 环境;无则 SKIP=dev-env degrade,`RURIX_REQUIRE_REAL=1` 硬红)。**步骤 48 offscreen 硬门
//! 0-byte 不动,present 不得替代 offscreen 真跑**(RD-019 backfill_condition 原文)。

use crate::Format;
use crate::error::Uc04Error;

/// DXGI swap effect(flip-model 恒定;blt-model 不进本面,§8)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapEffect {
    /// `DXGI_SWAP_EFFECT_FLIP_DISCARD`(默认;RXS-0220 恒定 flip-model)。
    FlipDiscard,
    /// `DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL`(flip-model 备选)。
    FlipSequential,
    /// `DXGI_SWAP_EFFECT_DISCARD`(blt-model,**不进本面** → RX6027)。
    BltDiscard,
    /// `DXGI_SWAP_EFFECT_SEQUENTIAL`(blt-model,**不进本面** → RX6027)。
    BltSequential,
}

impl SwapEffect {
    /// 是否为 flip-model(RXS-0220 恒定要求)。
    pub fn is_flip_model(self) -> bool {
        matches!(self, SwapEffect::FlipDiscard | SwapEffect::FlipSequential)
    }
}

/// 呈现循环内 backbuffer 状态迁移锚点(RXS-0220:`RENDER_TARGET → COPY_SOURCE → PRESENT`,
/// 沿 RXS-0169 手动编排口径;本面不引入自动状态推导,自动推导 = RFC-0013 §4.D)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentState {
    /// `D3D12_RESOURCE_STATE_RENDER_TARGET`(录制 draw 目标)。
    RenderTarget,
    /// `D3D12_RESOURCE_STATE_COPY_SOURCE`(readback copy 源)。
    CopySource,
    /// `D3D12_RESOURCE_STATE_PRESENT`(呈现前终态)。
    Present,
}

/// present 会话装配请求(host 侧已知参数;shim `rx_uc04_present_run` 消费同一状态空间)。
pub struct PresentRequest {
    /// swapchain image 格式。
    pub swapchain_format: Format,
    /// lighting pass final RT 格式(须与 swapchain image 格式一致,镜像 RXS-0167 口径)。
    pub final_rt_format: Format,
    /// swapchain buffer 数(`BufferCount ∈ {2,3}`,默认 3)。
    pub buffer_count: u32,
    /// swap effect(须 flip-model;blt-model → RX6027)。
    pub swap_effect: SwapEffect,
    /// 客户区宽(像素)。
    pub width: u32,
    /// 客户区高(像素)。
    pub height: u32,
    /// `Present(sync_interval)`(`sync_interval ∈ {0,1}`)。
    pub sync_interval: u32,
    /// 是否请求 tearing(须 `sync_interval == 0` 成对;能力探测属 device/shim 运行期,
    /// 缺失确定性拒不占码,Q-P-TearingFail,不在 host 装配核验)。
    pub tearing_requested: bool,
    /// present N 帧(呈现循环帧数)。
    pub frames: u32,
    /// 逐帧 backbuffer 状态迁移锚点序(须含 `RENDER_TARGET → COPY_SOURCE` 与
    /// `COPY_SOURCE → PRESENT`;缺 PRESENT 锚点 → RX6027)。
    pub present_transitions: Vec<(PresentState, PresentState)>,
}

/// 校验通过的 present 会话装配描述(host 侧;device 呈现循环承 shim / smoke)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentSession {
    /// swapchain image / final RT 一致格式。
    pub format: Format,
    /// swapchain buffer 数。
    pub buffer_count: u32,
    /// 客户区宽。
    pub width: u32,
    /// 客户区高。
    pub height: u32,
    /// `Present` sync_interval。
    pub sync_interval: u32,
    /// tearing 意图(device/shim 侧据能力探测最终决定是否启用;host 记录意图)。
    pub tearing_requested: bool,
    /// present 帧数。
    pub frames: u32,
}

/// RXS-0220:核验可见窗口 flip-model swapchain present 会话装配。
///
/// # Errors
/// swapchain desc ↔ final RT 格式/缓冲数失配 / 请求 blt-model 或不支持 swap effect /
/// `sync_interval` 越界 / tearing 与 sync_interval 未成对 / 缺 `PRESENT` 态迁移锚点 →
/// [`Uc04Error::PresentAssembly`](RX6027)。无运行期 fallback(P-01 strict-only)。
pub fn assemble_present(req: &PresentRequest) -> Result<PresentSession, Uc04Error> {
    // flip-model 恒定(blt-model 不进本面,§8)。
    if !req.swap_effect.is_flip_model() {
        return Err(Uc04Error::PresentAssembly {
            detail: format!(
                "swap effect {:?} 非 flip-model(本面恒定 FLIP_DISCARD/FLIP_SEQUENTIAL;blt-model 不进本面)",
                req.swap_effect
            ),
        });
    }
    // swapchain image 格式须与 lighting final RT 格式一致(镜像 RXS-0167 PSO↔RT 一致性)。
    if req.swapchain_format != req.final_rt_format {
        return Err(Uc04Error::PresentAssembly {
            detail: format!(
                "swapchain 格式 {:?} 与 lighting final RT 格式 {:?} 不一致",
                req.swapchain_format, req.final_rt_format
            ),
        });
    }
    // BufferCount ∈ {2,3}。
    if !(2..=3).contains(&req.buffer_count) {
        return Err(Uc04Error::PresentAssembly {
            detail: format!("BufferCount {} 越界(须 ∈ {{2,3}})", req.buffer_count),
        });
    }
    // 客户区尺寸非零。
    if req.width == 0 || req.height == 0 {
        return Err(Uc04Error::PresentAssembly {
            detail: format!("客户区尺寸 {}×{} 含 0 维", req.width, req.height),
        });
    }
    // sync_interval ∈ {0,1}。
    if req.sync_interval > 1 {
        return Err(Uc04Error::PresentAssembly {
            detail: format!("sync_interval {} 越界(须 ∈ {{0,1}})", req.sync_interval),
        });
    }
    // tearing 与 sync_interval 成对:请求 tearing 须 sync_interval == 0(不静默降级为 vsync,
    // Q-P-TearingFail;tearing 能力探测属 device/shim 运行期,缺失确定性拒不占码,不在本核验)。
    if req.tearing_requested && req.sync_interval != 0 {
        return Err(Uc04Error::PresentAssembly {
            detail: "请求 tearing 但 sync_interval != 0(tearing 须与 sync_interval=0 成对)"
                .to_owned(),
        });
    }
    // 逐帧迁移锚点须含 RENDER_TARGET → COPY_SOURCE 与 COPY_SOURCE → PRESENT。
    let has_rt_to_copy = req
        .present_transitions
        .contains(&(PresentState::RenderTarget, PresentState::CopySource));
    let has_copy_to_present = req
        .present_transitions
        .contains(&(PresentState::CopySource, PresentState::Present));
    if !has_rt_to_copy {
        return Err(Uc04Error::PresentAssembly {
            detail: "缺 RENDER_TARGET → COPY_SOURCE 迁移锚点(readback copy 前置)".to_owned(),
        });
    }
    if !has_copy_to_present {
        return Err(Uc04Error::PresentAssembly {
            detail: "缺 COPY_SOURCE → PRESENT 迁移锚点(呈现前终态;缺 PRESENT 锚点即装配违例)"
                .to_owned(),
        });
    }
    Ok(PresentSession {
        format: req.swapchain_format,
        buffer_count: req.buffer_count,
        width: req.width,
        height: req.height,
        sync_interval: req.sync_interval,
        tearing_requested: req.tearing_requested,
        frames: req.frames.max(1),
    })
}

/// swapchain acquire/present 后端无关状态(RXS-0221:失效是正常路径非错误)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapchainStatus {
    /// 成功(D3D12 `S_OK` / Vulkan `VK_SUCCESS`)。
    Ok,
    /// 次优但可用(Vulkan `SUBOPTIMAL_KHR`;首期与 OutOfDate 同走重建收尾以保守正确)。
    Suboptimal,
    /// 失效须重建(Vulkan `VK_ERROR_OUT_OF_DATE_KHR`)。
    OutOfDate,
    /// 窗口尺寸变(D3D12 `WM_SIZE` → `ResizeBuffers`)。
    WindowResized,
    /// 非预期失败(终止,不重建)。
    Fatal,
}

/// 重建协商动作(RXS-0221 L3:纯 host 确定性三分类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwapchainAction {
    /// 正常呈现(不重建)。
    Present,
    /// 等 GPU idle → 释放 → 重建(重查 surface caps extent)→ 重建后首帧再校验。
    Rebuild,
    /// 终止。
    Fatal,
}

/// RXS-0221 L3:present/acquire 后端无关状态 → 三分类动作(纯 host 确定性判定,可单测)。
///
/// `Suboptimal` 首期与 `OutOfDate` 同走重建(保守正确:窄化收益登 RD-034+)。
pub fn classify_swapchain_status(status: SwapchainStatus) -> SwapchainAction {
    match status {
        SwapchainStatus::Ok => SwapchainAction::Present,
        SwapchainStatus::Suboptimal
        | SwapchainStatus::OutOfDate
        | SwapchainStatus::WindowResized => SwapchainAction::Rebuild,
        SwapchainStatus::Fatal => SwapchainAction::Fatal,
    }
}

/// RXS-0221:swapchain 失效后重建请求(D3D12 `ResizeBuffers`:尺寸变,格式/缓冲数**恒定**;
/// Vulkan:重查 surface caps extent 重建 swapchain/imageView/framebuffer)。
pub struct RebuildRequest {
    /// 失效前的 present 会话(格式/缓冲数为重建后不变式基准)。
    pub old: PresentSession,
    /// 重建后新客户区宽(取新尺寸)。
    pub new_width: u32,
    /// 重建后新客户区高。
    pub new_height: u32,
    /// 重建后 swapchain 格式(`ResizeBuffers(…, UNKNOWN, …)` 保持,须 == old.format)。
    pub rebuilt_format: Format,
    /// 重建后 buffer 数(须 == old.buffer_count)。
    pub rebuilt_buffer_count: u32,
    /// RTV / imageView·framebuffer 是否已重建(未重建即录制 → 违例)。
    pub views_rebuilt: bool,
}

/// RXS-0221:核验 swapchain 重建序(idle → 释放 → 重建 → 首帧再校验)。
///
/// # Errors
/// 重建后格式/缓冲数漂移 / 视图未重建即录制 / 新尺寸含 0 维 →
/// [`Uc04Error::ResizeRebuild`](RX6028)。失效本身是正常路径(非错误);仅**重建违例**装配期拒。
pub fn rebuild_swapchain(req: &RebuildRequest) -> Result<PresentSession, Uc04Error> {
    if req.rebuilt_format != req.old.format {
        return Err(Uc04Error::ResizeRebuild {
            detail: format!(
                "重建后格式 {:?} 漂移(须恒定 == {:?})",
                req.rebuilt_format, req.old.format
            ),
        });
    }
    if req.rebuilt_buffer_count != req.old.buffer_count {
        return Err(Uc04Error::ResizeRebuild {
            detail: format!(
                "重建后 BufferCount {} 漂移(须恒定 == {})",
                req.rebuilt_buffer_count, req.old.buffer_count
            ),
        });
    }
    if !req.views_rebuilt {
        return Err(Uc04Error::ResizeRebuild {
            detail: "视图(RTV / imageView·framebuffer)未重建即录制(重建序违例)".to_owned(),
        });
    }
    if req.new_width == 0 || req.new_height == 0 {
        return Err(Uc04Error::ResizeRebuild {
            detail: format!("重建后尺寸 {}×{} 含 0 维", req.new_width, req.new_height),
        });
    }
    Ok(PresentSession {
        format: req.old.format,
        buffer_count: req.old.buffer_count,
        width: req.new_width,
        height: req.new_height,
        sync_interval: req.old.sync_interval,
        tearing_requested: req.old.tearing_requested,
        frames: req.old.frames,
    })
}

/// RXS-0222:present headless readback 断言点(≥3:首帧 / resize 重建后首帧 / 末帧)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadbackPoint {
    /// 首帧 present 前 readback。
    FirstFrame,
    /// resize 重建后首帧 present 前 readback。
    AfterRebuild,
    /// 末帧 present 前 readback。
    LastFrame,
}

/// RXS-0222:present 面必要 device 证据的 readback 断言频度(逐帧 copy,断言 ≥3 点)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadbackCadence {
    /// 断言点集(判据与步骤 48 offscreen 同族,布局复用 RXS-0170)。
    pub points: Vec<ReadbackPoint>,
}

impl ReadbackCadence {
    /// 标准三断言点(首帧 / 重建后首帧 / 末帧,Q-P-ReadbackCadence)。
    pub fn standard() -> Self {
        ReadbackCadence {
            points: vec![
                ReadbackPoint::FirstFrame,
                ReadbackPoint::AfterRebuild,
                ReadbackPoint::LastFrame,
            ],
        }
    }

    /// 断言点数是否满足 ≥3(MB1 W6 纪律:present 面必要 device 证据,非「看起来动了」)。
    pub fn is_sufficient(&self) -> bool {
        self.points.len() >= 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 合法 present 会话请求(64×64 RGBA8,BufferCount=3,FLIP_DISCARD,vsync,完整迁移锚点)。
    fn valid_request() -> PresentRequest {
        PresentRequest {
            swapchain_format: Format::Rgba8Unorm,
            final_rt_format: Format::Rgba8Unorm,
            buffer_count: 3,
            swap_effect: SwapEffect::FlipDiscard,
            width: 64,
            height: 64,
            sync_interval: 1,
            tearing_requested: false,
            frames: 8,
            present_transitions: vec![
                (PresentState::RenderTarget, PresentState::CopySource),
                (PresentState::CopySource, PresentState::Present),
            ],
        }
    }

    /// accept:一致 present 请求 → PresentSession(RXS-0220)。
    //@ spec: RXS-0220
    #[test]
    fn assembles_valid_present() {
        let s = assemble_present(&valid_request()).expect("合法 present 请求应通过");
        assert_eq!(s.format, Format::Rgba8Unorm);
        assert_eq!(s.buffer_count, 3);
        assert_eq!(s.frames, 8);
    }

    /// accept:tearing 请求须与 sync_interval=0 成对(RXS-0220 L3 tearing 参数面)。
    //@ spec: RXS-0220
    #[test]
    fn accepts_tearing_with_sync_zero() {
        let mut req = valid_request();
        req.sync_interval = 0;
        req.tearing_requested = true;
        assert!(assemble_present(&req).is_ok());
    }

    /// reject:blt-model swap effect → PresentAssembly(RX6027)。
    //@ spec: RXS-0220
    #[test]
    fn rejects_blt_model() {
        let mut req = valid_request();
        req.swap_effect = SwapEffect::BltDiscard;
        match assemble_present(&req) {
            Err(e @ Uc04Error::PresentAssembly { .. }) => assert_eq!(e.rx_code(), Some("RX6027")),
            other => panic!("blt-model 应 PresentAssembly,实得 {other:?}"),
        }
    }

    /// reject:swapchain 格式与 final RT 失配 → PresentAssembly(RX6027)。
    //@ spec: RXS-0220
    #[test]
    fn rejects_format_mismatch() {
        let mut req = valid_request();
        req.final_rt_format = Format::Rgba16Float;
        assert!(matches!(
            assemble_present(&req),
            Err(Uc04Error::PresentAssembly { .. })
        ));
    }

    /// reject:BufferCount 越界(1 或 4)→ PresentAssembly(RX6027)。
    //@ spec: RXS-0220
    #[test]
    fn rejects_buffer_count_out_of_range() {
        let mut req = valid_request();
        req.buffer_count = 1;
        assert!(matches!(
            assemble_present(&req),
            Err(Uc04Error::PresentAssembly { .. })
        ));
        req.buffer_count = 4;
        assert!(matches!(
            assemble_present(&req),
            Err(Uc04Error::PresentAssembly { .. })
        ));
    }

    /// reject:缺 COPY_SOURCE → PRESENT 迁移锚点 → PresentAssembly(RX6027)。
    /// **red_self_test 同族**(篡改 PRESENT 态迁移 → 装配核验拒,ci/uc04_present_smoke.py IR1)。
    //@ spec: RXS-0220
    #[test]
    fn rejects_missing_present_transition() {
        let mut req = valid_request();
        // 删去 COPY_SOURCE → PRESENT(篡改 PRESENT 态迁移锚点)。
        req.present_transitions
            .retain(|&t| t != (PresentState::CopySource, PresentState::Present));
        match assemble_present(&req) {
            Err(e @ Uc04Error::PresentAssembly { .. }) => assert_eq!(e.rx_code(), Some("RX6027")),
            other => panic!("缺 PRESENT 锚点应 PresentAssembly,实得 {other:?}"),
        }
    }

    /// reject:tearing 请求但 sync_interval != 0 → PresentAssembly(RX6027,不静默降级)。
    //@ spec: RXS-0220
    #[test]
    fn rejects_tearing_with_vsync() {
        let mut req = valid_request();
        req.tearing_requested = true; // sync_interval 仍为 1
        assert!(matches!(
            assemble_present(&req),
            Err(Uc04Error::PresentAssembly { .. })
        ));
    }

    /// RXS-0221 L3:present/acquire 状态 → 三分类动作(纯 host 确定性)。
    //@ spec: RXS-0221
    #[test]
    fn classifies_swapchain_status() {
        assert_eq!(
            classify_swapchain_status(SwapchainStatus::Ok),
            SwapchainAction::Present
        );
        // 失效是正常路径 → 重建(OUT_OF_DATE / SUBOPTIMAL / WM_SIZE 同走保守重建)。
        assert_eq!(
            classify_swapchain_status(SwapchainStatus::OutOfDate),
            SwapchainAction::Rebuild
        );
        assert_eq!(
            classify_swapchain_status(SwapchainStatus::Suboptimal),
            SwapchainAction::Rebuild
        );
        assert_eq!(
            classify_swapchain_status(SwapchainStatus::WindowResized),
            SwapchainAction::Rebuild
        );
        assert_eq!(
            classify_swapchain_status(SwapchainStatus::Fatal),
            SwapchainAction::Fatal
        );
    }

    /// accept:合法重建序(格式/缓冲数恒定 + 视图重建 + 新尺寸)→ 重建后 session(RXS-0221)。
    //@ spec: RXS-0221
    #[test]
    fn rebuilds_swapchain_ok() {
        let old = assemble_present(&valid_request()).unwrap();
        let req = RebuildRequest {
            old: old.clone(),
            new_width: 128,
            new_height: 96,
            rebuilt_format: old.format,
            rebuilt_buffer_count: old.buffer_count,
            views_rebuilt: true,
        };
        let s = rebuild_swapchain(&req).expect("合法重建序应通过");
        assert_eq!(s.width, 128);
        assert_eq!(s.height, 96);
        assert_eq!(s.format, old.format); // 格式恒定
        assert_eq!(s.buffer_count, old.buffer_count); // 缓冲数恒定
    }

    /// reject:重建后格式漂移 → ResizeRebuild(RX6028)。
    //@ spec: RXS-0221
    #[test]
    fn rejects_rebuild_format_drift() {
        let old = assemble_present(&valid_request()).unwrap();
        let req = RebuildRequest {
            old: old.clone(),
            new_width: 128,
            new_height: 96,
            rebuilt_format: Format::Rgba16Float, // 漂移
            rebuilt_buffer_count: old.buffer_count,
            views_rebuilt: true,
        };
        match rebuild_swapchain(&req) {
            Err(e @ Uc04Error::ResizeRebuild { .. }) => assert_eq!(e.rx_code(), Some("RX6028")),
            other => panic!("格式漂移应 ResizeRebuild,实得 {other:?}"),
        }
    }

    /// reject:视图未重建即录制 → ResizeRebuild(RX6028)。
    //@ spec: RXS-0221
    #[test]
    fn rejects_rebuild_without_views() {
        let old = assemble_present(&valid_request()).unwrap();
        let req = RebuildRequest {
            old: old.clone(),
            new_width: 128,
            new_height: 96,
            rebuilt_format: old.format,
            rebuilt_buffer_count: old.buffer_count,
            views_rebuilt: false, // 未重建
        };
        assert!(matches!(
            rebuild_swapchain(&req),
            Err(Uc04Error::ResizeRebuild { .. })
        ));
    }

    /// reject:重建后 BufferCount 漂移 → ResizeRebuild(RX6028)。
    //@ spec: RXS-0221
    #[test]
    fn rejects_rebuild_buffer_count_drift() {
        let old = assemble_present(&valid_request()).unwrap();
        let req = RebuildRequest {
            old: old.clone(),
            new_width: 128,
            new_height: 96,
            rebuilt_format: old.format,
            rebuilt_buffer_count: old.buffer_count + 1, // 漂移
            views_rebuilt: true,
        };
        assert!(matches!(
            rebuild_swapchain(&req),
            Err(Uc04Error::ResizeRebuild { .. })
        ));
    }

    /// RXS-0222:标准三断言点(首/重建后/末帧)满足 ≥3(present 面必要 device 证据)。
    //@ spec: RXS-0222
    #[test]
    fn readback_cadence_has_three_points() {
        let cadence = ReadbackCadence::standard();
        assert!(cadence.is_sufficient());
        assert_eq!(cadence.points.len(), 3);
        assert!(cadence.points.contains(&ReadbackPoint::AfterRebuild));
    }

    /// RXS-0222:少于 3 断言点不充分(不得以「看起来动了」冒充数值校验)。
    //@ spec: RXS-0222
    #[test]
    fn readback_cadence_insufficient_below_three() {
        let cadence = ReadbackCadence {
            points: vec![ReadbackPoint::FirstFrame, ReadbackPoint::LastFrame],
        };
        assert!(!cadence.is_sufficient());
    }
}
