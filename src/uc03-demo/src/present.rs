//! 实时窗口呈现通路（G1.1，feature `d3d12-present`；RFC-0001 / RXS-0142~0143）。
//!
//! 复用 [`rurix_rt::interop`] scope 帧 typestate：每帧 `Ready.wait`（CUDA wait `acquire(2n)`
//! 取写权）→ `Acquired`（G0 软光栅 kernel 写共享 backbuffer，`sr_tonemap` RXS-0121 语义
//! **0-byte**）→ `signal`（CUDA signal `cuda_done(2n+1)`）→ `Presentable.present`（shim wait
//! `2n+1` → 固定 present pass → Present → signal `2n+2`）→ `Ready`，共享 fence 偶/奇值 handoff
//! （RFC-0001 §4.3）。present 同步序由消费式 typestate 编译期保证（跳过 wait/signal 无对应态句柄）。
//!
//! 窗口内容（SPH 软光栅）真跑需 `d3d12-present-real`（MSVC + Windows SDK + 交互桌面会话）;
//! 本（stub）构建仅驱动帧 typestate 与 fence handoff 编译通过，G0 kernel 写共享 backbuffer
//! 为 runner 集成点（见下 launch 注释）。

use rurix_rt::interop::{InteropError, InteropKernelArg, SR_TONEMAP_KERNEL, SR_TONEMAP_PTX, scope};

const FILL_PTX: &str = r#".version 8.0
.target sm_89
.address_size 64

.visible .entry rx_present_fill(
    .param .u64 out,
    .param .u64 n
)
{
    .reg .pred %p<4>;
    .reg .b32 %r<7>;
    .reg .b64 %rd<7>;

    ld.param.u64 %rd1, [out];
    ld.param.u64 %rd2, [n];
    mov.u32 %r1, %ctaid.x;
    mov.u32 %r2, %ntid.x;
    mov.u32 %r3, %tid.x;
    mad.lo.s32 %r4, %r1, %r2, %r3;
    cvt.u64.u32 %rd3, %r4;
    setp.ge.u64 %p1, %rd3, %rd2;
    @%p1 bra DONE;
    rem.u64 %rd4, %rd3, 3;
    mov.b32 %r5, 0f00000000;
    setp.eq.u64 %p2, %rd4, 0;
    @%p2 mov.b32 %r5, 0f3F800000;
    setp.eq.u64 %p3, %rd4, 1;
    @%p3 mov.b32 %r5, 0f3F000000;
    shl.b64 %rd5, %rd3, 2;
    add.s64 %rd6, %rd1, %rd5;
    st.global.b32 [%rd6], %r5;
DONE:
    ret;
}
"#;

/// 运行交互式窗口呈现帧循环（窗口关闭或错误退出）。返回已 present 的帧数。
///
/// 无 `d3d12-present-real`（stub shim）/ 无 GPU / 非交互桌面 → 返回
/// [`InteropError`]（scope 在 shim/CUDA 边界确定性不可用），不 panic。
pub fn run_present(
    cuda_ordinal: i32,
    render: [u32; 2],
    window: [u32; 2],
) -> Result<u64, InteropError> {
    let frame_limit = std::env::var("RURIX_PRESENT_FRAMES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    scope(cuda_ordinal, render, window, |cx, mut ready| {
        if SR_TONEMAP_PTX.trim().is_empty() || SR_TONEMAP_KERNEL.is_empty() {
            return Err(InteropError::DeviceVerificationFailed);
        }
        let fill_module = cx.load_module(FILL_PTX)?;
        let fill_kernel = fill_module.kernel("rx_present_fill")?;
        let tonemap_module = cx.load_module(SR_TONEMAP_PTX)?;
        let tonemap_kernel = tonemap_module.kernel(SR_TONEMAP_KERNEL)?;
        let mut frames: u64 = 0;
        loop {
            let mut acquired = ready.wait()?; // CUDA wait acquire(2n)：取得本帧写权
            let n = acquired.buffer_mut().len();
            let grid = [(n as u32).div_ceil(256), 1, 1];
            let fill_buffer = {
                let buffer = acquired.buffer_mut();
                InteropKernelArg::buffer(&*buffer)
            };
            acquired.launch(
                &fill_kernel,
                grid,
                [256, 1, 1],
                &mut [fill_buffer, InteropKernelArg::usize(n)],
            )?;
            let hdr = {
                let buffer = acquired.buffer_mut();
                InteropKernelArg::buffer(&*buffer)
            };
            let ldr = {
                let buffer = acquired.buffer_mut();
                InteropKernelArg::buffer(&*buffer)
            };
            acquired.launch(
                &tonemap_kernel,
                grid,
                [256, 1, 1],
                &mut [hdr, ldr, InteropKernelArg::usize(n)],
            )?;
            let mut sample = [0.0f32; 3];
            acquired.readback_f32(&mut sample)?;
            if sample != [255.0, 128.0, 0.0] {
                return Err(InteropError::DeviceVerificationFailed);
            }
            let presentable = acquired.signal()?; // CUDA signal cuda_done(2n+1)
            let should_close = presentable.pump()?; // 抽干窗口消息泵
            ready = presentable.present()?; // shim wait(2n+1)→present→signal(2n+2) → Ready
            frames += 1;
            if should_close || frame_limit.is_some_and(|limit| frames >= limit) {
                break;
            }
        }
        Ok(frames)
    })
}
