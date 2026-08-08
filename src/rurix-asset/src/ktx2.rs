//! 最小 KTX2 写入器(无 supercompression;禁 zstd)。
//! 过渡路径把真实 BCn/ASTC 块装入标准容器(magic / scheme 可复核)。

/// KTX2 文件标识(12 字节)。
pub const KTX2_MAGIC: &[u8; 12] = b"\xABKTX 20\xBB\r\n\x1A\n";

/// VkFormat::VK_FORMAT_BC7_UNORM_BLOCK
pub const VK_FORMAT_BC7_UNORM_BLOCK: u32 = 145;
/// VkFormat::VK_FORMAT_ASTC_4x4_UNORM_BLOCK
pub const VK_FORMAT_ASTC_4X4_UNORM_BLOCK: u32 = 157;

/// 写入单 mip / 单 face、`supercompressionScheme = 0` 的 KTX2。
pub fn write_ktx2_uncompressed(vk_format: u32, width: u32, height: u32, level0: &[u8]) -> Vec<u8> {
    // 极简 DFD:totalSize + 一个占位 descriptor block。
    // smoke 核验 magic / scheme / level 计数;完整 DFD 随 basis_universal 合入升级。
    // 布局: u32 totalSize | u32 vendorId | u16 descriptorType | u16 version |
    //       u16 descriptorBlockSize | 14B pad → block 本体 24B,含 totalSize 共 28B。
    let mut dfd = Vec::new();
    dfd.extend_from_slice(&0u32.to_le_bytes()); // totalSize placeholder
    dfd.extend_from_slice(&0u32.to_le_bytes()); // vendorId
    dfd.extend_from_slice(&0u16.to_le_bytes()); // descriptorType
    dfd.extend_from_slice(&0u16.to_le_bytes()); // version
    let block_size: u16 = 24;
    dfd.extend_from_slice(&block_size.to_le_bytes());
    dfd.extend_from_slice(&[0u8; 14]); // pad to 24B block (4+2+2+2+14=24)
    let dfd_total = dfd.len() as u32;
    dfd[0..4].copy_from_slice(&dfd_total.to_le_bytes());
    debug_assert_eq!(dfd.len(), 28);

    // After 12-byte identifier: 68 bytes header + 24 bytes level index (1 level).
    let level_index_off = 12 + 68;
    let dfd_off = level_index_off + 24;
    let data_off_raw = dfd_off + dfd.len();
    let data_off = (data_off_raw + 15) & !15;

    let mut out = Vec::with_capacity(data_off + level0.len());
    out.extend_from_slice(KTX2_MAGIC);
    out.extend_from_slice(&vk_format.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes()); // typeSize
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // depth
    out.extend_from_slice(&0u32.to_le_bytes()); // layerCount
    out.extend_from_slice(&1u32.to_le_bytes()); // faceCount
    out.extend_from_slice(&1u32.to_le_bytes()); // levelCount
    out.extend_from_slice(&0u32.to_le_bytes()); // supercompressionScheme = NONE
    out.extend_from_slice(&(dfd_off as u32).to_le_bytes());
    out.extend_from_slice(&dfd_total.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // kvd offset
    out.extend_from_slice(&0u32.to_le_bytes()); // kvd length
    out.extend_from_slice(&0u64.to_le_bytes()); // sgd offset
    out.extend_from_slice(&0u64.to_le_bytes()); // sgd length
    debug_assert_eq!(out.len(), level_index_off);

    out.extend_from_slice(&(data_off as u64).to_le_bytes());
    out.extend_from_slice(&(level0.len() as u64).to_le_bytes());
    out.extend_from_slice(&(level0.len() as u64).to_le_bytes());
    debug_assert_eq!(out.len(), dfd_off);
    out.extend_from_slice(&dfd);
    while out.len() < data_off {
        out.push(0);
    }
    out.extend_from_slice(level0);
    let _ = vk_format; // reserved for future DFD sample binding
    out
}

/// RXBC 自制 BCn 容器: magic `RXBC` + ver u16 + format u16 + w/h u32 + payload。
///
/// 注:RXBC 是**离线 BCn 块的 Rurix 自有封装**(非冒充任何标准容器);
/// format 值语义 = BCn 家族编号。`.basis` / `.ktx2` 两腿一律用真实
/// basis_universal 码流,不经本封装。
pub const RXBC_MAGIC: &[u8; 4] = b"RXBC";
pub const RXBC_FMT_BC7: u16 = 7;
/// BC5_UNORM(双 BC4 = XY;normal 语义腿)。
pub const RXBC_FMT_BC5: u16 = 5;
/// BC4_UNORM(单通道;mask 语义腿)。
pub const RXBC_FMT_BC4: u16 = 4;

pub fn write_rxbc(format: u16, width: u32, height: u32, blocks: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(16 + blocks.len());
    o.extend_from_slice(RXBC_MAGIC);
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&format.to_le_bytes());
    o.extend_from_slice(&width.to_le_bytes());
    o.extend_from_slice(&height.to_le_bytes());
    o.extend_from_slice(blocks);
    o
}

/// RXAS 自制 ASTC 容器。
pub const RXAS_MAGIC: &[u8; 4] = b"RXAS";

pub fn write_rxas(width: u32, height: u32, blocks: &[u8]) -> Vec<u8> {
    let mut o = Vec::with_capacity(16 + blocks.len());
    o.extend_from_slice(RXAS_MAGIC);
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&4u16.to_le_bytes());
    o.extend_from_slice(&width.to_le_bytes());
    o.extend_from_slice(&height.to_le_bytes());
    o.extend_from_slice(blocks);
    o
}

// RXBS(过渡 Basis/ETC1S 腿容器)已**删除**:`.basis` 腿现由真实
// basis_universal ETC1S 码流产出(`rurix_basis_sys::encode_container`),
// 自制容器冒充 `.basis` 属假绿形态,禁止复活。
//
// 真实 `.basis` 文件签名 = `packed_uint<2>` LE 存 `('B'<<8)|'s'` → 磁盘字节 `b"sB"`。
/// 真实 `.basis` 磁盘签名字节(校验锚;非写入器)。
pub const BASIS_FILE_SIG: [u8; 2] = [b's', b'B'];
