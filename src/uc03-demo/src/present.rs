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

use rurix_rt::interop::{InteropError, scope};

/// 运行交互式窗口呈现帧循环（窗口关闭或错误退出）。返回已 present 的帧数。
///
/// 无 `d3d12-present-real`（stub shim）/ 无 GPU / 非交互桌面 → 返回
/// [`InteropError`]（scope 在 shim/CUDA 边界确定性不可用），不 panic。
pub fn run_present(
    cuda_ordinal: i32,
    render: [u32; 2],
    window: [u32; 2],
) -> Result<u64, InteropError> {
    scope(cuda_ordinal, render, window, |_cx, mut ready| {
        let mut frames: u64 = 0;
        loop {
            let mut acquired = ready.wait()?; // CUDA wait acquire(2n)：取得本帧写权
            // —— runner 集成点（d3d12-present-real）——
            // G0 软光栅 kernel 写共享 backbuffer（sr_tonemap，RXS-0121，语义 0-byte）:
            //   let buf = acquired.buffer_mut();
            //   acquired.launch(&sr_tonemap, grid, block,
            //       &mut [InteropKernelArg::buffer(buf), InteropKernelArg::usize(n)])?;
            // stub 构建下不 launch（共享 buffer 由 present pass 读出黑帧），仅驱动 typestate。
            let _ = acquired.buffer_mut(); // 触达可写 backbuffer（Acquired 态独有，RXS-0142）
            let presentable = acquired.signal()?; // CUDA signal cuda_done(2n+1)
            let should_close = presentable.pump()?; // 抽干窗口消息泵
            ready = presentable.present()?; // shim wait(2n+1)→present→signal(2n+2) → Ready
            frames += 1;
            if should_close {
                break;
            }
        }
        Ok(frames)
    })
}
