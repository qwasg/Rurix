//! 最小 KTX2 写入器(无 supercompression;禁 zstd) + KTX2 容器解析器
//! (G31+ 波 C Task C14 KTX2-1;host safe Rust,`#![forbid(unsafe_code)]` crate 内)。
//! 过渡路径把真实 BCn/ASTC 块装入标准容器(magic / scheme 可复核)。
//!
//! 解析面 = KTX2 头(68B) / level index(24B×levelCount) / key-value 数据(KVD) /
//! supercompression 元数据(scheme + SGD 偏移/长度 + 逐 level uncompressed 长度)。
//! 全部偏移边界机核,fail-closed(截断/越界/非法 scheme → 确定性 `Err`,不 panic)。

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

// ---------------------------------------------------------------------------
// KTX2 容器解析器(G31+ 波 C Task C14 KTX2-1)
// ---------------------------------------------------------------------------

/// KTX2 头总长:12B 标识 + 68B 头字段(9×u32 + 4×u32 + 2×u64)。
pub const KTX2_HEADER_LEN: usize = 12 + 68;
/// level index 单项长(u64 byteOffset + u64 byteLength + u64 uncompressedByteLength)。
pub const KTX2_LEVEL_INDEX_ENTRY_LEN: usize = 24;
/// levelCount 上界(KTX2 spec:mip 数 ≤ 32;越界即非法件)。
pub const KTX2_MAX_LEVELS: u32 = 32;

/// supercompressionScheme = NONE(无超压缩;在树唯一产出面)。
pub const KTX2_SS_NONE: u32 = 0;
/// supercompressionScheme = BASIS_LZ(ETC1S 内嵌;vendor 可解码面)。
pub const KTX2_SS_BASIS_LZ: u32 = 1;
/// supercompressionScheme = ZSTD(**在树禁**:`BASISD_SUPPORT_KTX2_ZSTD=0`)。
pub const KTX2_SS_ZSTD: u32 = 2;
/// supercompressionScheme = ZLIB(在树不支持)。
pub const KTX2_SS_ZLIB: u32 = 3;

/// KHR Data Format colorModel = UASTC(真实 basis UASTC KTX2 的 DFD 字面)。
pub const KHR_DF_MODEL_UASTC: u8 = 166;
/// KHR Data Format colorModel = ETC1S。
pub const KHR_DF_MODEL_ETC1S: u8 = 163;

/// KTX2 头字段(12B 标识之后的 68B;逐字面对齐 KTX 2.0 spec §3.2)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ktx2Header {
    /// vkFormat;Basis/UASTC 容器恒 0(VK_FORMAT_UNDEFINED,格式由 DFD 承载)。
    pub vk_format: u32,
    pub type_size: u32,
    pub pixel_width: u32,
    pub pixel_height: u32,
    pub pixel_depth: u32,
    pub layer_count: u32,
    pub face_count: u32,
    pub level_count: u32,
    /// supercompressionScheme(supercompression 元数据主字段)。
    pub supercompression_scheme: u32,
    pub dfd_byte_offset: u32,
    pub dfd_byte_length: u32,
    pub kvd_byte_offset: u32,
    pub kvd_byte_length: u32,
    /// supercompression global data 区(supercompression 元数据;scheme=0 时恒 0/0)。
    pub sgd_byte_offset: u64,
    pub sgd_byte_length: u64,
}

/// level index 单项(mip 布局元数据)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ktx2Level {
    pub byte_offset: u64,
    pub byte_length: u64,
    /// scheme=0 时须 == byte_length(spec §3.6;解析器机核)。
    pub uncompressed_byte_length: u64,
}

/// KVD key-value 对(key 为 NUL 前 UTF-8 段;value 原样字节)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ktx2KeyValue {
    pub key: String,
    pub value: Vec<u8>,
}

/// DFD 最小解析面(totalSize 机核 + colorModel 提取;完整块体不展开)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ktx2Dfd {
    pub total_size: u32,
    /// 首 descriptor block 块体首字节(colorModel);长度不足时为 None。
    pub color_model: Option<u8>,
}

/// 解析产物:头 + mip 布局(level index)+ key-value + DFD 摘要。
/// `PartialEq` 派生 = 确定性双读位级一致判据载体。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ktx2File {
    pub header: Ktx2Header,
    pub levels: Vec<Ktx2Level>,
    pub key_values: Vec<Ktx2KeyValue>,
    pub dfd: Ktx2Dfd,
}

/// 解析失败(确定性;同输入两次解析同错)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Ktx2ParseError {
    /// 文件短于头总长。
    TooShort,
    BadIdentifier,
    /// typeSize != 1(块压缩容器面 typeSize 恒 1)。
    BadTypeSize(u32),
    /// levelCount == 0 或 > 32。
    BadLevelCount(u32),
    /// faceCount == 0。
    BadFaceCount,
    /// 保留/未知 supercompressionScheme。
    UnsupportedScheme(u32),
    /// 区段越界(区段名)。
    OutOfBounds(&'static str),
    /// DFD totalSize 与登记长度不符或长度 < 4。
    BadDfd,
    /// KVD 记录越界/缺 NUL/零长记录。
    BadKeyValue,
    /// scheme=0 但 uncompressedByteLength != byteLength(spec §3.6)。
    SchemeZeroLengthMismatch(usize),
    /// level byteLength == 0(空 level 非法)。
    EmptyLevel(usize),
}

impl std::fmt::Display for Ktx2ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ktx2ParseError::TooShort => write!(f, "KTX2: 文件短于头总长 {KTX2_HEADER_LEN}"),
            Ktx2ParseError::BadIdentifier => write!(f, "KTX2: 标识非法(非 KTX 2.0)"),
            Ktx2ParseError::BadTypeSize(t) => write!(f, "KTX2: typeSize {t} != 1"),
            Ktx2ParseError::BadLevelCount(n) => write!(f, "KTX2: levelCount {n} 越 [1,32]"),
            Ktx2ParseError::BadFaceCount => write!(f, "KTX2: faceCount == 0"),
            Ktx2ParseError::UnsupportedScheme(s) => {
                write!(f, "KTX2: supercompressionScheme {s} 为保留/未知值")
            }
            Ktx2ParseError::OutOfBounds(r) => write!(f, "KTX2: 区段越界 {r}"),
            Ktx2ParseError::BadDfd => write!(f, "KTX2: DFD totalSize 不符/长度 < 4"),
            Ktx2ParseError::BadKeyValue => write!(f, "KTX2: KVD 记录非法"),
            Ktx2ParseError::SchemeZeroLengthMismatch(i) => {
                write!(f, "KTX2: level {i} scheme=0 但 uncompressed != length")
            }
            Ktx2ParseError::EmptyLevel(i) => write!(f, "KTX2: level {i} byteLength == 0"),
        }
    }
}

impl std::error::Error for Ktx2ParseError {}

fn le_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes(b[off..off + 4].try_into().expect("le_u32 边界由调用方机核"))
}

fn le_u64(b: &[u8], off: usize) -> u64 {
    u64::from_le_bytes(b[off..off + 8].try_into().expect("le_u64 边界由调用方机核"))
}

/// 解析 KTX2 容器(纯函数;同字节输入两次解析 `PartialEq` 位级一致)。
///
/// fail-closed:一切截断/越界/非法字段 → `Err`,不 panic、不部分产出。
pub fn parse_ktx2(bytes: &[u8]) -> Result<Ktx2File, Ktx2ParseError> {
    if bytes.len() < KTX2_HEADER_LEN {
        return Err(Ktx2ParseError::TooShort);
    }
    if &bytes[..12] != KTX2_MAGIC {
        return Err(Ktx2ParseError::BadIdentifier);
    }
    let header = Ktx2Header {
        vk_format: le_u32(bytes, 12),
        type_size: le_u32(bytes, 16),
        pixel_width: le_u32(bytes, 20),
        pixel_height: le_u32(bytes, 24),
        pixel_depth: le_u32(bytes, 28),
        layer_count: le_u32(bytes, 32),
        face_count: le_u32(bytes, 36),
        level_count: le_u32(bytes, 40),
        supercompression_scheme: le_u32(bytes, 44),
        dfd_byte_offset: le_u32(bytes, 48),
        dfd_byte_length: le_u32(bytes, 52),
        kvd_byte_offset: le_u32(bytes, 56),
        kvd_byte_length: le_u32(bytes, 60),
        sgd_byte_offset: le_u64(bytes, 64),
        sgd_byte_length: le_u64(bytes, 72),
    };
    if header.type_size != 1 {
        return Err(Ktx2ParseError::BadTypeSize(header.type_size));
    }
    if header.level_count == 0 || header.level_count > KTX2_MAX_LEVELS {
        return Err(Ktx2ParseError::BadLevelCount(header.level_count));
    }
    if header.face_count == 0 {
        return Err(Ktx2ParseError::BadFaceCount);
    }
    match header.supercompression_scheme {
        KTX2_SS_NONE | KTX2_SS_BASIS_LZ | KTX2_SS_ZSTD | KTX2_SS_ZLIB => {}
        s => return Err(Ktx2ParseError::UnsupportedScheme(s)),
    }
    let file_len = bytes.len() as u64;
    let level_count = header.level_count as usize;

    // level index 区段边界。
    let index_len = (level_count as u64)
        .checked_mul(KTX2_LEVEL_INDEX_ENTRY_LEN as u64)
        .ok_or(Ktx2ParseError::OutOfBounds("level_index"))?;
    let index_end = (KTX2_HEADER_LEN as u64)
        .checked_add(index_len)
        .ok_or(Ktx2ParseError::OutOfBounds("level_index"))?;
    if index_end > file_len {
        return Err(Ktx2ParseError::OutOfBounds("level_index"));
    }
    let mut levels = Vec::with_capacity(level_count);
    for i in 0..level_count {
        let base = KTX2_HEADER_LEN + i * KTX2_LEVEL_INDEX_ENTRY_LEN;
        let lv = Ktx2Level {
            byte_offset: le_u64(bytes, base),
            byte_length: le_u64(bytes, base + 8),
            uncompressed_byte_length: le_u64(bytes, base + 16),
        };
        if lv.byte_length == 0 {
            return Err(Ktx2ParseError::EmptyLevel(i));
        }
        let end = lv
            .byte_offset
            .checked_add(lv.byte_length)
            .ok_or(Ktx2ParseError::OutOfBounds("level_data"))?;
        if end > file_len {
            return Err(Ktx2ParseError::OutOfBounds("level_data"));
        }
        if header.supercompression_scheme == KTX2_SS_NONE
            && lv.uncompressed_byte_length != lv.byte_length
        {
            return Err(Ktx2ParseError::SchemeZeroLengthMismatch(i));
        }
        levels.push(lv);
    }

    // DFD 区段:totalSize 机核 + colorModel 提取。
    let dfd_off = header.dfd_byte_offset as u64;
    let dfd_len = header.dfd_byte_length as u64;
    let dfd_end = dfd_off
        .checked_add(dfd_len)
        .ok_or(Ktx2ParseError::OutOfBounds("dfd"))?;
    if dfd_end > file_len {
        return Err(Ktx2ParseError::OutOfBounds("dfd"));
    }
    if dfd_len < 4 {
        return Err(Ktx2ParseError::BadDfd);
    }
    let dfd_region = &bytes[dfd_off as usize..dfd_end as usize];
    let total_size = le_u32(dfd_region, 0);
    if total_size as u64 != dfd_len {
        return Err(Ktx2ParseError::BadDfd);
    }
    // 首 block 布局(KHR DF):u32 totalSize | u32 vendorId:17|descriptorType:15 |
    // u16 versionNumber | u16 descriptorBlockSize | 块体(colorModel 为首字节)。
    let color_model = if dfd_len >= 13 {
        let block_size = u16::from_le_bytes([dfd_region[10], dfd_region[11]]) as u64;
        if block_size >= 9 && 4 + block_size <= dfd_len {
            Some(dfd_region[12])
        } else {
            None
        }
    } else {
        None
    };
    let dfd = Ktx2Dfd {
        total_size,
        color_model,
    };

    // KVD 区段:key-value 序列(u32 kvByteLength | key\0value | 4B 对齐 pad)。
    let kvd_off = header.kvd_byte_offset as u64;
    let kvd_len = header.kvd_byte_length as u64;
    let kvd_end = kvd_off
        .checked_add(kvd_len)
        .ok_or(Ktx2ParseError::OutOfBounds("kvd"))?;
    if kvd_end > file_len {
        return Err(Ktx2ParseError::OutOfBounds("kvd"));
    }
    let mut key_values = Vec::new();
    let kvd = &bytes[kvd_off as usize..kvd_end as usize];
    let mut pos = 0usize;
    while pos < kvd.len() {
        if pos + 4 > kvd.len() {
            return Err(Ktx2ParseError::BadKeyValue);
        }
        let rec_len = le_u32(kvd, pos) as usize;
        if rec_len == 0 || pos + 4 + rec_len > kvd.len() {
            return Err(Ktx2ParseError::BadKeyValue);
        }
        let rec = &kvd[pos + 4..pos + 4 + rec_len];
        let Some(nul) = rec.iter().position(|&c| c == 0) else {
            return Err(Ktx2ParseError::BadKeyValue);
        };
        let Ok(key) = std::str::from_utf8(&rec[..nul]) else {
            return Err(Ktx2ParseError::BadKeyValue);
        };
        key_values.push(Ktx2KeyValue {
            key: key.to_string(),
            value: rec[nul + 1..].to_vec(),
        });
        pos += 4 + ((rec_len + 3) & !3);
    }

    // SGD 区段(supercompression 元数据):仅边界机核,不展开。
    if header.sgd_byte_length > 0 {
        let sgd_end = header
            .sgd_byte_offset
            .checked_add(header.sgd_byte_length)
            .ok_or(Ktx2ParseError::OutOfBounds("sgd"))?;
        if sgd_end > file_len {
            return Err(Ktx2ParseError::OutOfBounds("sgd"));
        }
    }

    Ok(Ktx2File {
        header,
        levels,
        key_values,
        dfd,
    })
}

impl Ktx2File {
    /// level `i` 的逻辑像素尺寸(mip 布局:`max(1, dim >> i)`;深度同理)。
    pub fn level_dims(&self, level: u32) -> Option<(u32, u32, u32)> {
        if level >= self.header.level_count {
            return None;
        }
        let w = (self.header.pixel_width >> level).max(1);
        let h = (self.header.pixel_height >> level).max(1);
        let d = (self.header.pixel_depth >> level).max(1);
        Some((w, h, d))
    }

    /// level `i` 负载字节切片(偏移经解析器边界机核,直接索引安全)。
    pub fn level_slice<'a>(&self, bytes: &'a [u8], level: u32) -> Option<&'a [u8]> {
        let lv = self.levels.get(level as usize)?;
        let start = lv.byte_offset as usize;
        let end = start + lv.byte_length as usize;
        bytes.get(start..end)
    }

    /// DFD 原始区段切片(供多级组装器复用真实 encoder 产 DFD)。
    pub fn dfd_slice<'a>(&self, bytes: &'a [u8]) -> Option<&'a [u8]> {
        let start = self.header.dfd_byte_offset as usize;
        let end = start + self.header.dfd_byte_length as usize;
        bytes.get(start..end)
    }

    /// 全 mip 链负载字节合计(mip 布局体量面)。
    pub fn mip_chain_bytes(&self) -> u64 {
        self.levels.iter().map(|l| l.byte_length).sum()
    }

    /// 指定 key 的 KVD 值(如 `KTXwriter`)。
    pub fn key_value(&self, key: &str) -> Option<&[u8]> {
        self.key_values
            .iter()
            .find(|kv| kv.key == key)
            .map(|kv| kv.value.as_slice())
    }

    /// 在树 vendor transcoder 可消费面(禁 zstd/zlib supercompression)。
    pub fn is_vendor_transcodable(&self) -> bool {
        matches!(
            self.header.supercompression_scheme,
            KTX2_SS_NONE | KTX2_SS_BASIS_LZ
        )
    }
}

/// 多级 KTX2 组装器(最小合成件面;G31+ 波 C Task C14 KTX2-1/KTX2-3)。
///
/// 语义:把**真实 encoder 产的逐 level 负载**装入 spec 合规容器
/// (level index 自 0 起索引最大级;文件数据自最小级向最大级铺陈,spec §3.10;
/// 逐级 16B 对齐)。DFD 字节由调用方供给(真实 encoder 产 KTX2 的 DFD 区段),
/// KVD 写入 `KTXwriter` = "rurix-asset ktx2.rs write_ktx2_multilevel"
/// (作者如实登记,不冒充上游 encoder 产物)。
///
/// 禁手写二进制冒充:负载必须来自 `rurix_basis_sys::encode_container` 真实产出
/// (或等价真实 codec),本函数只做容器布局,不构造像素/块数据。
pub fn write_ktx2_multilevel(
    vk_format: u32,
    width: u32,
    height: u32,
    dfd: &[u8],
    levels: &[&[u8]],
) -> Vec<u8> {
    assert!(!levels.is_empty(), "levels 至少 1 级");
    assert!(levels.len() <= KTX2_MAX_LEVELS as usize, "levels ≤ 32");
    assert!(width > 0 && height > 0, "尺寸 > 0");
    assert!(dfd.len() >= 4, "DFD 至少含 totalSize");
    for (i, lv) in levels.iter().enumerate() {
        assert!(!lv.is_empty(), "level {i} 负载非空");
    }
    let level_count = levels.len() as u32;

    // KVD:单条 KTXwriter。
    let writer = b"rurix-asset ktx2.rs write_ktx2_multilevel";
    let kv_rec_len = ("KTXwriter".len() + 1 + writer.len()) as u32;
    let mut kvd = Vec::new();
    kvd.extend_from_slice(&kv_rec_len.to_le_bytes());
    kvd.extend_from_slice(b"KTXwriter\0");
    kvd.extend_from_slice(writer);
    while kvd.len() % 4 != 0 {
        kvd.push(0);
    }

    let level_index_off = KTX2_HEADER_LEN;
    let dfd_off = level_index_off + levels.len() * KTX2_LEVEL_INDEX_ENTRY_LEN;
    let kvd_off = dfd_off + dfd.len();
    let data_off_raw = kvd_off + kvd.len();
    // 数据区自最小级向最大级铺陈,逐级 16B 对齐。
    let mut offsets = vec![0u64; levels.len()];
    let mut cursor = (data_off_raw + 15) & !15;
    for i in (0..levels.len()).rev() {
        offsets[i] = cursor as u64;
        cursor += levels[i].len();
        cursor = (cursor + 15) & !15;
    }

    let mut out = Vec::with_capacity(cursor);
    out.extend_from_slice(KTX2_MAGIC);
    out.extend_from_slice(&vk_format.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes()); // typeSize
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes()); // depth
    out.extend_from_slice(&0u32.to_le_bytes()); // layerCount
    out.extend_from_slice(&1u32.to_le_bytes()); // faceCount
    out.extend_from_slice(&level_count.to_le_bytes());
    out.extend_from_slice(&KTX2_SS_NONE.to_le_bytes()); // supercompressionScheme
    out.extend_from_slice(&(dfd_off as u32).to_le_bytes());
    out.extend_from_slice(&(dfd.len() as u32).to_le_bytes());
    out.extend_from_slice(&(kvd_off as u32).to_le_bytes());
    out.extend_from_slice(&(kvd.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u64.to_le_bytes()); // sgd offset
    out.extend_from_slice(&0u64.to_le_bytes()); // sgd length
    debug_assert_eq!(out.len(), level_index_off);
    for (i, lv) in levels.iter().enumerate() {
        out.extend_from_slice(&offsets[i].to_le_bytes());
        out.extend_from_slice(&(lv.len() as u64).to_le_bytes());
        out.extend_from_slice(&(lv.len() as u64).to_le_bytes()); // uncompressed == len
    }
    debug_assert_eq!(out.len(), dfd_off);
    out.extend_from_slice(dfd);
    debug_assert_eq!(out.len(), kvd_off);
    out.extend_from_slice(&kvd);
    // 铺陈 level 数据(最小级先行)。
    for i in (0..levels.len()).rev() {
        while (out.len() as u64) < offsets[i] {
            out.push(0);
        }
        out.extend_from_slice(levels[i]);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bcdec::{decode_bc7_rgba8, max_channel_delta};
    use crate::texture::COLOR_MAX_CHANNEL_DELTA;
    use rurix_basis_sys::{self as basis, ContainerMode, SrcKind, TargetFormat};

    fn gradient(w: u32, h: u32) -> Vec<u8> {
        let mut v = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                let on = ((x / 2) + (y / 2)) % 2 == 0;
                if on {
                    v.extend_from_slice(&[200, 30, 40, 255]);
                } else {
                    v.extend_from_slice(&[30, 60, 200, 220]);
                }
            }
        }
        v
    }

    fn encode_ktx2(w: u32, h: u32) -> (Vec<u8>, Vec<u8>) {
        let rgba = gradient(w, h);
        let ktx2 = basis::encode_container(&rgba, w, h, ContainerMode::UastcKtx2, false).unwrap();
        (rgba, ktx2)
    }

    /// 真实 .ktx2 测试件(经 basis encoder 产)——头/KV/level index/supercompression
    /// 元数据逐字段互核。
    #[test]
    fn parse_real_encoder_ktx2_crosschecked() {
        let (_rgba, ktx2) = encode_ktx2(16, 16);
        let f = parse_ktx2(&ktx2).unwrap();
        let h = &f.header;
        assert_eq!(h.vk_format, 0, "Basis/UASTC 容器 vkFormat 恒 UNDEFINED");
        assert_eq!(h.type_size, 1);
        assert_eq!((h.pixel_width, h.pixel_height), (16, 16));
        assert_eq!(h.pixel_depth, 0);
        assert_eq!(h.layer_count, 0);
        assert_eq!(h.face_count, 1);
        assert_eq!(h.level_count, 1, "在树 encoder 钳制单 mip");
        assert_eq!(
            h.supercompression_scheme, KTX2_SS_NONE,
            "禁 supercompression"
        );
        assert_eq!(h.sgd_byte_offset, 0);
        assert_eq!(
            h.sgd_byte_length, 0,
            "scheme=0 ⇒ SGD 空(supercompression 元数据)"
        );
        assert_eq!(
            f.dfd.color_model,
            Some(KHR_DF_MODEL_UASTC),
            "真实 UASTC KTX2 DFD colorModel=166"
        );
        // 上游 encoder 写 KTXwriter(含 basis 版本字面);mipPadding dummy key 亦合法。
        let writer = f
            .key_value("KTXwriter")
            .unwrap_or_else(|| panic!("真实 encoder 件须含 KTXwriter;KV={:?}", f.key_values));
        assert!(!writer.is_empty());
        assert!(f.is_vendor_transcodable());
        // mip 布局:单级 16×16 UASTC = 4×4 块 ×16B = 256B。
        assert_eq!(f.levels.len(), 1);
        assert_eq!(f.level_dims(0), Some((16, 16, 1)));
        assert_eq!(f.level_dims(1), None);
        assert_eq!(f.levels[0].byte_length, 4 * 4 * 16);
        assert_eq!(f.mip_chain_bytes(), 4 * 4 * 16);
        assert_eq!(f.level_slice(&ktx2, 0).unwrap().len(), 256);
        assert!(f.level_slice(&ktx2, 1).is_none());
    }

    /// 确定性解析:同文件双读(双解析)位级一致。
    #[test]
    fn parse_deterministic_double_read() {
        let (_rgba, ktx2) = encode_ktx2(16, 16);
        let a = parse_ktx2(&ktx2).unwrap();
        let b = parse_ktx2(&ktx2).unwrap();
        assert_eq!(a, b, "同文件双解析须位级一致(PartialEq 闭集)");
        assert_eq!(a.level_slice(&ktx2, 0), b.level_slice(&ktx2, 0));
    }

    /// 最小合成件(多级):真实 encoder 产逐级负载 + 本组装器容器布局 →
    /// 解析 mip 布局机核 + vendor transcoder 逐级真转码对拍。
    #[test]
    fn assemble_multilevel_layout_and_transcode() {
        let (rgba0, k0) = encode_ktx2(16, 16);
        let (rgba1, k1) = encode_ktx2(8, 8);
        let f0 = parse_ktx2(&k0).unwrap();
        let f1 = parse_ktx2(&k1).unwrap();
        let dfd = f0.dfd_slice(&k0).unwrap();
        assert_eq!(dfd, f1.dfd_slice(&k1).unwrap(), "同格式 DFD 与尺寸无关");
        let p0 = f0.level_slice(&k0, 0).unwrap();
        let p1 = f1.level_slice(&k1, 0).unwrap();

        let assembled = write_ktx2_multilevel(0, 16, 16, dfd, &[p0, p1]);
        let fa = parse_ktx2(&assembled).unwrap();
        assert_eq!(fa.header.level_count, 2);
        assert_eq!(fa.header.supercompression_scheme, KTX2_SS_NONE);
        assert_eq!(fa.level_dims(0), Some((16, 16, 1)));
        assert_eq!(fa.level_dims(1), Some((8, 8, 1)));
        assert_eq!(fa.dfd.color_model, Some(KHR_DF_MODEL_UASTC));
        assert_eq!(
            fa.key_value("KTXwriter").unwrap(),
            b"rurix-asset ktx2.rs write_ktx2_multilevel",
            "作者如实登记(不冒充上游 encoder)"
        );
        // 负载位级保真(合成件 = 真实负载 + 容器布局,非手写二进制)。
        assert_eq!(fa.level_slice(&assembled, 0).unwrap(), p0);
        assert_eq!(fa.level_slice(&assembled, 1).unwrap(), p1);
        assert_eq!(fa.mip_chain_bytes(), p0.len() as u64 + p1.len() as u64);

        // vendor transcoder 消费合成件:level 0 与单级件转码位级一致。
        let t0a =
            basis::transcode_level(&assembled, SrcKind::Ktx2, TargetFormat::Bc7Rgba, 0).unwrap();
        let t0s = basis::transcode(&k0, SrcKind::Ktx2, TargetFormat::Bc7Rgba).unwrap();
        assert_eq!(t0a, t0s, "合成件 level0 转码须与真实单级件位级一致");
        // level 1 逐级转码:尺寸随 mip 布局,像素回解码对拍 ≤ AP-TEX 冻结容差。
        let t1 =
            basis::transcode_level(&assembled, SrcKind::Ktx2, TargetFormat::Bc7Rgba, 1).unwrap();
        assert_eq!((t1.width, t1.height), (8, 8));
        assert_eq!(t1.blocks.len(), 2 * 2 * 16);
        let dec1 = decode_bc7_rgba8(&t1.blocks, 8, 8);
        let d1 = max_channel_delta(&rgba1, &dec1);
        assert!(
            d1 <= COLOR_MAX_CHANNEL_DELTA,
            "level1 BC7 回解码对拍越 AP-TEX 冻结容差: {d1}"
        );
        let dec0 = decode_bc7_rgba8(&t0a.blocks, 16, 16);
        let d0 = max_channel_delta(&rgba0, &dec0);
        assert!(d0 <= COLOR_MAX_CHANNEL_DELTA, "level0 对拍越容差: {d0}");
        // level 越界 fail-closed。
        assert!(
            basis::transcode_level(&assembled, SrcKind::Ktx2, TargetFormat::Bc7Rgba, 2).is_err()
        );
    }

    /// 既有单级写入器(write_ktx2_uncompressed)与解析器交叉一致。
    #[test]
    fn legacy_writer_roundtrips_parser() {
        let blocks = vec![0xABu8; 4 * 4 * 16];
        let bytes = write_ktx2_uncompressed(VK_FORMAT_BC7_UNORM_BLOCK, 16, 16, &blocks);
        let f = parse_ktx2(&bytes).unwrap();
        assert_eq!(f.header.vk_format, VK_FORMAT_BC7_UNORM_BLOCK);
        assert_eq!(f.header.level_count, 1);
        assert_eq!(f.header.supercompression_scheme, KTX2_SS_NONE);
        assert_eq!(f.level_slice(&bytes, 0).unwrap(), &blocks[..]);
    }

    /// punchthrough(BC1 三色+透明黑解码形态 RGBA(0,0,0,0))内容的 UASTC/ETC1S
    /// 双腿回环机制锚:A/B 对拍口径的事实源——
    /// alpha=0 像素的 RGB 属「透明像素 RGB 自由域」(premultiplied 语义,不参与
    /// 对拍);alpha 通道按 codec 端点量化容差判定(UASTC mode 7 alpha = 5+1 bit
    /// 端点 → 量化阶 ≤ 16,语义翻转 = ≥128 级差,二者可机器区分)。
    /// ALPHA_DELTA_BOUND = 16:量化噪声带内 / 语义翻转带外 的判别界。
    const ALPHA_DELTA_BOUND: u8 = 16;

    #[test]
    fn punchthrough_alpha_roundtrip_semantics() {
        let w = 16u32;
        let h = 16u32;
        let mut rgba = Vec::with_capacity((w * h * 4) as usize);
        for y in 0..h {
            for x in 0..w {
                if (x + y) % 3 == 0 {
                    rgba.extend_from_slice(&[0, 0, 0, 0]); // punchthrough 透明黑
                } else {
                    let r = (x * 16) as u8;
                    let g = (y * 16) as u8;
                    rgba.extend_from_slice(&[r, g, 200, 255]);
                }
            }
        }
        for (mode, src, name, rgb_bound) in [
            (
                ContainerMode::UastcKtx2,
                SrcKind::Ktx2,
                "UASTC",
                COLOR_MAX_CHANNEL_DELTA,
            ),
            // ETC1S 在逐像素棋盘极值合成图案上为对抗性输入(码书全局匹配,
            // 实测 max≈90)——合成件面给宽 bound(语义面 alpha 检查双腿同严);
            // 真实纹理面对拍界 = AP-TEX 冻结 48(A/B harness 实测判据面)。
            (ContainerMode::Etc1sBasis, SrcKind::Basis, "ETC1S", 96u8),
        ] {
            let c = basis::encode_container(&rgba, w, h, mode, false).unwrap();
            let t = basis::transcode(&c, src, TargetFormat::Bc7Rgba).unwrap();
            let dec = decode_bc7_rgba8(&t.blocks, w, h);
            for i in 0..(w * h) as usize {
                let sa = rgba[i * 4 + 3];
                let da = dec[i * 4 + 3];
                let ad = sa.abs_diff(da);
                assert!(
                    ad <= ALPHA_DELTA_BOUND,
                    "{name} 像素 {i} alpha 差 {ad} 越量化界 {ALPHA_DELTA_BOUND}(语义翻转判红)"
                );
                if sa == 0 {
                    assert!(da <= ALPHA_DELTA_BOUND, "{name} 透明像素 {i} 复现为 {da}");
                }
                if sa == 255 {
                    assert!(
                        da >= 255 - ALPHA_DELTA_BOUND,
                        "{name} 不透明像素 {i} 复现为 {da}"
                    );
                }
                if sa > 0 {
                    for ch in 0..3 {
                        let d = rgba[i * 4 + ch].abs_diff(dec[i * 4 + ch]);
                        assert!(
                            d <= rgb_bound,
                            "{name} 不透明像素 {i} 通道 {ch} 差 {d} 越容差 {rgb_bound}"
                        );
                    }
                }
            }
        }
    }

    /// fail-closed 臂:截断/篡改/非法 scheme 一律确定性 Err,不 panic。
    #[test]
    fn parse_fail_closed_arms() {
        let (_rgba, ktx2) = encode_ktx2(16, 16);
        // 截断于头内。
        assert_eq!(parse_ktx2(&ktx2[..40]), Err(Ktx2ParseError::TooShort));
        // 标识篡改。
        let mut bad = ktx2.clone();
        bad[1] = b'X';
        assert_eq!(parse_ktx2(&bad), Err(Ktx2ParseError::BadIdentifier));
        // 全零垃圾(长度越头但标识非法)。
        assert_eq!(parse_ktx2(&[0u8; 256]), Err(Ktx2ParseError::BadIdentifier));
        // 截断于 level 数据区。
        let trunc = &ktx2[..ktx2.len() - 1];
        assert_eq!(
            parse_ktx2(trunc),
            Err(Ktx2ParseError::OutOfBounds("level_data"))
        );
        // 保留 scheme。
        let mut bad_scheme = ktx2.clone();
        bad_scheme[44..48].copy_from_slice(&7u32.to_le_bytes());
        assert_eq!(
            parse_ktx2(&bad_scheme),
            Err(Ktx2ParseError::UnsupportedScheme(7))
        );
        // zstd scheme:容器面可解析,但 vendor 禁面如实登记。
        let mut zstd = ktx2.clone();
        zstd[44..48].copy_from_slice(&KTX2_SS_ZSTD.to_le_bytes());
        let fz = parse_ktx2(&zstd).unwrap();
        assert!(!fz.is_vendor_transcodable(), "zstd 在树禁面须登记不可转码");
        // scheme=0 但 uncompressed != length。
        let mut mm = ktx2.clone();
        let l0 = mm[88..96].try_into().unwrap();
        let l0 = u64::from_le_bytes(l0);
        mm[96..104].copy_from_slice(&(l0 + 8).to_le_bytes());
        assert_eq!(
            parse_ktx2(&mm),
            Err(Ktx2ParseError::SchemeZeroLengthMismatch(0))
        );
        // levelCount = 0 / 越界。
        let mut lc0 = ktx2.clone();
        lc0[40..44].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(parse_ktx2(&lc0), Err(Ktx2ParseError::BadLevelCount(0)));
        let mut lc99 = ktx2.clone();
        lc99[40..44].copy_from_slice(&99u32.to_le_bytes());
        assert_eq!(parse_ktx2(&lc99), Err(Ktx2ParseError::BadLevelCount(99)));
        // typeSize != 1。
        let mut ts = ktx2.clone();
        ts[16..20].copy_from_slice(&2u32.to_le_bytes());
        assert_eq!(parse_ktx2(&ts), Err(Ktx2ParseError::BadTypeSize(2)));
        // 确定性:同一非法输入两次解析同错。
        let e1 = parse_ktx2(&mm);
        let e2 = parse_ktx2(&mm);
        assert_eq!(e1, e2);
    }
}
