//! 实时窗口呈现通路（G1.1，feature `d3d12-present`；RFC-0001 / RXS-0142~0143）。
//!
//! 复用 [`rurix_rt::interop`] scope 帧 typestate：每帧 `Ready.wait`（CUDA wait `acquire(2n)`
//! 取写权）→ `Acquired`（UC-03 SPH → G0 完整软光栅 HDR 帧上传共享 backbuffer，
//! `sr_tonemap` RXS-0121 kernel 原位量化，kernel 语义 **0-byte**）→ `signal`（CUDA signal
//! `cuda_done(2n+1)`）→ `Presentable.present`（shim wait
//! `2n+1` → 固定 present pass → Present → signal `2n+2`）→ `Ready`，共享 fence 偶/奇值 handoff
//! （RFC-0001 §4.3）。present 同步序由消费式 typestate 编译期保证（跳过 wait/signal 无对应态句柄）。
//!
//! 窗口内容（SPH 软光栅）真跑需 `d3d12-present-real`（MSVC + Windows SDK + 交互桌面会话）;
//! stub 构建仅驱动帧 typestate 与 fence handoff 编译通过。真实路径逐帧回读完整 LDR 帧，
//! 与 host 确定性量化结果逐元素核对后才提交 Present。

use rurix_rt::interop::{InteropError, InteropKernelArg, SR_TONEMAP_KERNEL, SR_TONEMAP_PTX, scope};
use soft_raster::tonemap_channel;

use crate::{Particle, initial_particles, render_particles_hdr, step_frame};

/// 实时呈现完成后的可机器核对摘要。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresentReport {
    /// 已提交到 swapchain 的帧数。
    pub frames: u64,
    /// 首帧量化 RGB 字节的 FNV-1a 64 位校验值。
    pub first_frame_checksum: u64,
    /// 首帧非黑像素数。
    pub first_frame_lit_pixels: usize,
    /// 运行期间是否观察到与首帧不同的完整帧（SPH 动画确实推进）。
    pub animation_changed: bool,
}

struct SceneFrame {
    hdr: Vec<f32>,
    expected_ldr: Vec<f32>,
    checksum: u64,
    lit_pixels: usize,
}

fn scene_frame(particles: &[Particle]) -> SceneFrame {
    let pixels = render_particles_hdr(particles);
    let mut hdr = Vec::with_capacity(pixels.len() * 3);
    let mut expected_ldr = Vec::with_capacity(pixels.len() * 3);
    let mut checksum = 0xcbf2_9ce4_8422_2325u64;
    let mut lit_pixels = 0usize;
    for [r, g, b] in pixels {
        hdr.extend_from_slice(&[r, g, b]);
        let quantized = [tonemap_channel(r), tonemap_channel(g), tonemap_channel(b)];
        if quantized != [0, 0, 0] {
            lit_pixels += 1;
        }
        for value in quantized {
            checksum ^= u64::from(value);
            checksum = checksum.wrapping_mul(0x0000_0100_0000_01b3);
            expected_ldr.push(f32::from(value));
        }
    }
    SceneFrame {
        hdr,
        expected_ldr,
        checksum,
        lit_pixels,
    }
}

/// 运行交互式窗口呈现帧循环（窗口关闭或错误退出）。
///
/// 无 `d3d12-present-real`（stub shim）/ 无 GPU / 非交互桌面 → 返回
/// [`InteropError`]（scope 在 shim/CUDA 边界确定性不可用），不 panic。
pub fn run_present(
    cuda_ordinal: i32,
    render: [u32; 2],
    window: [u32; 2],
) -> Result<PresentReport, InteropError> {
    let frame_limit = std::env::var("RURIX_PRESENT_FRAMES")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|value| *value > 0);
    scope(cuda_ordinal, render, window, |cx, mut ready| {
        if SR_TONEMAP_PTX.trim().is_empty() || SR_TONEMAP_KERNEL.is_empty() {
            return Err(InteropError::DeviceVerificationFailed);
        }
        let tonemap_module = cx.load_module(SR_TONEMAP_PTX)?;
        let tonemap_kernel = tonemap_module.kernel(SR_TONEMAP_KERNEL)?;
        let mut particles = initial_particles();
        let mut frames: u64 = 0;
        let mut first_frame_checksum = 0u64;
        let mut first_frame_lit_pixels = 0usize;
        let mut animation_changed = false;
        loop {
            let scene = scene_frame(&particles);
            let mut acquired = ready.wait()?; // CUDA wait acquire(2n)：取得本帧写权
            let n = acquired.buffer_mut().len();
            if scene.hdr.len() != n {
                return Err(InteropError::DeviceVerificationFailed);
            }
            acquired.upload_f32(&scene.hdr)?;
            let grid = [(n as u32).div_ceil(256), 1, 1];
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
            let mut readback = vec![0.0f32; n];
            acquired.readback_f32(&mut readback)?;
            if readback != scene.expected_ldr {
                return Err(InteropError::DeviceVerificationFailed);
            }
            if frames == 0 {
                first_frame_checksum = scene.checksum;
                first_frame_lit_pixels = scene.lit_pixels;
                if first_frame_lit_pixels == 0 {
                    return Err(InteropError::DeviceVerificationFailed);
                }
            } else if scene.checksum != first_frame_checksum {
                animation_changed = true;
            }
            let presentable = acquired.signal()?; // CUDA signal cuda_done(2n+1)
            let should_close = presentable.pump()?; // 抽干窗口消息泵
            ready = presentable.present()?; // shim wait(2n+1)→present→signal(2n+2) → Ready
            frames += 1;
            particles = step_frame(&particles);
            if should_close || frame_limit.is_some_and(|limit| frames >= limit) {
                break;
            }
        }
        Ok(PresentReport {
            frames,
            first_frame_checksum,
            first_frame_lit_pixels,
            animation_changed,
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scene_frame_contains_complete_visible_sph_frame() {
        let frame = scene_frame(&initial_particles());
        assert_eq!(frame.hdr.len(), 32 * 24 * 3);
        assert_eq!(frame.expected_ldr.len(), frame.hdr.len());
        assert!(frame.lit_pixels > 0);
        assert_ne!(frame.checksum, 0);
        assert!(
            frame
                .expected_ldr
                .chunks_exact(3)
                .any(|rgb| rgb[0] != 0.0 || rgb[1] != 0.0 || rgb[2] != 0.0)
        );
    }

    #[test]
    fn scene_frame_changes_when_sph_advances() {
        let mut particles = initial_particles();
        let first = scene_frame(&particles);
        for _ in 0..6 {
            particles = step_frame(&particles);
        }
        let later = scene_frame(&particles);
        assert_ne!(first.checksum, later.checksum);
        assert_ne!(first.expected_ldr, later.expected_ldr);
    }
}
