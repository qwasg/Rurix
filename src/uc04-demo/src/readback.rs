//! RXS-0170:offscreen readback 缓冲布局(host 侧 safe 核验)。
//!
//! §9 Q-Present=offscreen-first:offscreen 渲染后回读像素做数值对照为 G-G2-4 device
//! 必要面;窗口 swapchain present 不进必要条款(defer → RD-019)。本模块 host 面只核验
//! **readback 缓冲布局/格式**(row pitch 对齐 `TEXTURE_DATA_PITCH_ALIGNMENT` / 格式与源
//! 一致 / 尺寸充足);失配 → strict-only 显式错。
//!
//! **device 段(hardware offscreen draw + 像素逐值对照,REQUIRE_REAL,CI step 48)阻塞于
//! RD-013** → 本轮 blocked-honest,不达成、不以替代物伪造(承 [`crate::device`])。

use crate::Format;
use crate::error::Uc04Error;

/// D3D12 `D3D12_TEXTURE_DATA_PITCH_ALIGNMENT`(readback 行距对齐;非语言 ABI 冻结)。
pub const TEXTURE_DATA_PITCH_ALIGNMENT: u32 = 256;

/// offscreen readback 请求(回读 lighting 输出到 host 可读 buffer)。
pub struct ReadbackRequest {
    /// 渲染目标宽(像素)。
    pub width: u32,
    /// 渲染目标高(像素)。
    pub height: u32,
    /// 源(lighting 输出)格式。
    pub src_format: Format,
    /// readback 目标格式(须与源一致)。
    pub dst_format: Format,
    /// 调用方声明的 readback 行距(须 = 对齐后行距)。
    pub row_pitch: u32,
    /// 调用方声明的 readback buffer 字节数(须 ≥ 对齐行距 × 高)。
    pub buffer_size: u64,
}

/// 校验通过的 readback 布局(host 侧;device 像素对照承 blocked-on-RD-013)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReadbackLayout {
    /// 对齐后的行距(`TEXTURE_DATA_PITCH_ALIGNMENT` 对齐)。
    pub row_pitch: u32,
    /// readback buffer 字节数(对齐行距 × 高)。
    pub buffer_size: u64,
    /// readback 像素格式。
    pub format: Format,
}

/// 向上对齐到 `align`(`align` 为 2 的幂或正整数)。
fn align_up(value: u32, align: u32) -> u32 {
    value.div_ceil(align) * align
}

/// RXS-0170:核验 readback 缓冲布局/格式。
///
/// # Errors
/// row pitch 未对齐/不一致 / 格式与源不一致 / 尺寸不足 →
/// [`Uc04Error::ReadbackLayout`](RX6022)。
pub fn plan_readback(req: &ReadbackRequest) -> Result<ReadbackLayout, Uc04Error> {
    // 格式须与源一致(readback 不做格式转换)。
    if req.dst_format != req.src_format {
        return Err(Uc04Error::ReadbackLayout {
            detail: format!(
                "readback 格式 {:?} 与源格式 {:?} 不一致",
                req.dst_format, req.src_format
            ),
        });
    }
    // 行距须对齐 TEXTURE_DATA_PITCH_ALIGNMENT 且与声明一致。
    let unaligned = req.width * req.src_format.bytes_per_pixel();
    let aligned = align_up(unaligned, TEXTURE_DATA_PITCH_ALIGNMENT);
    if req.row_pitch != aligned {
        return Err(Uc04Error::ReadbackLayout {
            detail: format!(
                "readback 行距 {} 未对齐(应为 {} = {} 对齐 {})",
                req.row_pitch, aligned, unaligned, TEXTURE_DATA_PITCH_ALIGNMENT
            ),
        });
    }
    // buffer 尺寸须 ≥ 对齐行距 × 高。
    let needed = u64::from(aligned) * u64::from(req.height);
    if req.buffer_size < needed {
        return Err(Uc04Error::ReadbackLayout {
            detail: format!(
                "readback buffer {} 字节不足(须 ≥ {needed})",
                req.buffer_size
            ),
        });
    }
    Ok(ReadbackLayout {
        row_pitch: aligned,
        buffer_size: req.buffer_size,
        format: req.dst_format,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 64×64 RGBA8 的合法 readback 请求(行距 = 256 已对齐,buffer 充足)。
    fn valid_request() -> ReadbackRequest {
        // 64 px × 4 B = 256 已对齐;buffer = 256 × 64 = 16384。
        ReadbackRequest {
            width: 64,
            height: 64,
            src_format: Format::Rgba8Unorm,
            dst_format: Format::Rgba8Unorm,
            row_pitch: 256,
            buffer_size: 256 * 64,
        }
    }

    /// accept:合法 readback 请求 → ReadbackLayout。
    //@ spec: RXS-0170
    #[test]
    fn plans_valid_readback() {
        let layout = plan_readback(&valid_request()).expect("合法请求应通过");
        assert_eq!(layout.row_pitch, 256);
        assert_eq!(layout.buffer_size, 256 * 64);
        assert_eq!(layout.format, Format::Rgba8Unorm);
    }

    /// accept:非 256 倍数宽度的行距须向上对齐(100 px × 4 B = 400 → 512)。
    //@ spec: RXS-0170
    #[test]
    fn aligns_row_pitch_up() {
        let req = ReadbackRequest {
            width: 100,
            height: 10,
            src_format: Format::Rgba8Unorm,
            dst_format: Format::Rgba8Unorm,
            row_pitch: 512, // 400 对齐 256 → 512
            buffer_size: 512 * 10,
        };
        assert_eq!(plan_readback(&req).unwrap().row_pitch, 512);
    }

    /// reject:行距未对齐 → ReadbackLayout(RX6022)。
    //@ spec: RXS-0170
    #[test]
    fn rejects_unaligned_row_pitch() {
        let mut req = valid_request();
        req.row_pitch = 300; // 非 256 对齐
        match plan_readback(&req) {
            Err(e @ Uc04Error::ReadbackLayout { .. }) => assert_eq!(e.rx_code(), Some("RX6022")),
            other => panic!("未对齐行距应 ReadbackLayout,实得 {other:?}"),
        }
    }

    /// reject:格式与源不一致 → ReadbackLayout(RX6022)。
    //@ spec: RXS-0170
    #[test]
    fn rejects_format_mismatch() {
        let mut req = valid_request();
        req.dst_format = Format::Rgba16Float; // 与源 Rgba8Unorm 不一致
        assert!(matches!(
            plan_readback(&req),
            Err(Uc04Error::ReadbackLayout { .. })
        ));
    }

    /// reject:buffer 尺寸不足 → ReadbackLayout(RX6022)。
    //@ spec: RXS-0170
    #[test]
    fn rejects_undersized_buffer() {
        let mut req = valid_request();
        req.buffer_size = 256 * 63; // 少一行
        assert!(matches!(
            plan_readback(&req),
            Err(Uc04Error::ReadbackLayout { .. })
        ));
    }
}
