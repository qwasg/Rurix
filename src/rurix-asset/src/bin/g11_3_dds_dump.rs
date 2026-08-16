//! G11.3 U2 修复面 DDS 解码 dump 工具（G10-N7 承接锚兑现——派生链转码的
//! Rust 解码腿；milestones/g11/harness/g11_3_dds_transcode.py 驱动面）。
//!
//! 职责闭集：读取 `.dds`（legacy FourCC DXT1/DXT5/ATI1/ATI2 + DX10 DXGI
//! BC1/BC3/BC4/BC5/BC7 子集），经 `rurix_asset::bcdec::decode_dds` 真实解码
//! mip 0 为 RGBA8，落盘原始像素体（行主序 8-bit RGBA）+ stdout 单行 JSON
//! 登记（尺寸/格式/mip 数/像素 digest）。法线用途由驱动面重组 Z 通道
//! （BC5 XY → 全 XYZ 法线 PNG）——本工具只做容器解码，不做语义重映射。
//!
//! 用法：
//!   g11_3_dds_dump <in.dds> <out.rgba8>
//!
//! Assisted-by: Kimi-K3（G11.3 波）

#![forbid(unsafe_code)]

const TAG: &str = "G11_3_DDS_DUMP";

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        fail("用法: g11_3_dds_dump <in.dds> <out.rgba8>");
    }
    let raw = std::fs::read(&args[0]).unwrap_or_else(|e| fail(&format!("读取失败: {e}")));
    let img = rurix_asset::bcdec::decode_dds(&raw)
        .unwrap_or_else(|e| fail(&format!("DDS 解码失败: {e}")));
    std::fs::write(&args[1], &img.rgba8).unwrap_or_else(|e| fail(&format!("落盘失败: {e}")));
    let digest = rurix_pkg::sha256::hex_digest(&img.rgba8);
    println!(
        "{{\"width\":{},\"height\":{},\"format\":\"{}\",\"mip_count\":{},\"rgba8_len\":{},\"rgba8_digest\":\"sha256:{}\"}}",
        img.width,
        img.height,
        img.format.as_str(),
        img.mip_count,
        img.rgba8.len(),
        digest
    );
}
