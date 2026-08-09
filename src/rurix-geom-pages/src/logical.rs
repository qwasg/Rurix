//! 逻辑页 RXPL encode/decode（RXS-0328 / RXS-0331）。
//!
//! 布局字面见 `spec/geometry_pages.md`。手写 LE，零 struct memcpy。

use rurix_pkg::sha256;

/// 与 `rurix_render::graph::types::STREAM_PAGE_SIZE` 字面同值（本 crate 不依赖 render）。
pub const STREAM_PAGE_SIZE: u32 = 128 * 1024;

pub const RXPL_MAGIC: [u8; 4] = *b"RXPL";
pub const FORMAT_ID: u32 = 1;
pub const LOGICAL_MAJOR: u16 = 1;
pub const LOGICAL_MINOR: u16 = 0;
pub const ENDIAN_LE: u8 = 1;
pub const FLAG_ROOT: u8 = 0x01;
pub const HEADER_SIZE: u16 = 136;
pub const RECORD_SIZE: u16 = 96;
pub const PACKING_ALGO_ID: u32 = 1;

/// 逻辑页解码错误（消费前拒录）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PageDecodeError {
    BadMagic,
    UnsupportedVersion { major: u16, minor: u16 },
    BadHeader(&'static str),
    Truncated(&'static str),
    Inconsistent(&'static str),
    DigestMismatch(&'static str),
}

impl std::fmt::Display for PageDecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PageDecodeError::BadMagic => write!(f, "RXPL 魔数不符"),
            PageDecodeError::UnsupportedVersion { major, minor } => {
                write!(f, "RXPL 版本不支持:{major}.{minor}")
            }
            PageDecodeError::BadHeader(s) => write!(f, "RXPL header 损坏:{s}"),
            PageDecodeError::Truncated(s) => write!(f, "RXPL 截断:{s}"),
            PageDecodeError::Inconsistent(s) => write!(f, "RXPL 不一致:{s}"),
            PageDecodeError::DigestMismatch(s) => write!(f, "RXPL digest 不匹配:{s}"),
        }
    }
}

impl std::error::Error for PageDecodeError {}

/// 页内簇记录（96B 逻辑模型；编码见 RXS-0328）。
#[derive(Debug, Clone, PartialEq)]
pub struct PageClusterRecord {
    pub cluster_id: u32,
    pub qx: u16,
    pub qy: u16,
    pub qz: u16,
    pub center: [f32; 3],
    pub radius: f32,
    pub cone_axis: [f32; 3],
    pub cone_cutoff: f32,
    pub error: f32,
    pub parent_error: f32,
    pub vertex_offset: u32,
    pub triangle_offset: u32,
    pub vertex_count: u32,
    pub triangle_count: u32,
    pub level: u32,
    pub group: u32,
}

/// 逻辑页（未压缩）。
#[derive(Debug, Clone, PartialEq)]
pub struct LogicalPage {
    pub page_id: u64,
    pub flags: u8,
    pub lod_level_min: u16,
    pub lod_level_max: u16,
    pub bounds: [f32; 6],
    pub clusters: Vec<PageClusterRecord>,
    pub vertices: Vec<[f32; 3]>,
    pub indices: Vec<u8>,
    /// 升序去重的依赖页 id。
    pub dependency_page_ids: Vec<u64>,
    /// 升序 `(parent, child)` 边。
    pub dag_links: Vec<(u32, u32)>,
}

impl LogicalPage {
    pub fn is_root(&self) -> bool {
        self.flags & FLAG_ROOT != 0
    }

    /// 编码后总字节（header + 段体）。
    pub fn encoded_len(&self) -> usize {
        HEADER_SIZE as usize
            + self.clusters.len() * RECORD_SIZE as usize
            + self.vertices.len() * 12
            + self.indices.len()
            + self.dependency_page_ids.len() * 8
            + self.dag_links.len() * 8
    }
}

/// 冻结 schema_digest（RXS-0328 preimage）。
pub fn schema_digest() -> [u8; 32] {
    let mut pre = Vec::with_capacity(64);
    pre.extend_from_slice(b"RXPL-SCHEMA-V1\0");
    put_u16(&mut pre, LOGICAL_MAJOR);
    put_u16(&mut pre, LOGICAL_MINOR);
    put_u32(&mut pre, STREAM_PAGE_SIZE);
    put_u16(&mut pre, HEADER_SIZE);
    put_u16(&mut pre, RECORD_SIZE);
    put_u32(&mut pre, PACKING_ALGO_ID);
    put_u32(&mut pre, FORMAT_ID);
    sha256::digest(&pre)
}

/// 编码逻辑页 → 字节（确定性）。
pub fn encode_logical_page(page: &LogicalPage) -> Vec<u8> {
    let mut body = Vec::new();
    for c in &page.clusters {
        put_record(&mut body, c);
    }
    for v in &page.vertices {
        for &x in v {
            put_f32(&mut body, x);
        }
    }
    body.extend_from_slice(&page.indices);
    for &id in &page.dependency_page_ids {
        put_u64(&mut body, id);
    }
    for &(p, c) in &page.dag_links {
        put_u32(&mut body, p);
        put_u32(&mut body, c);
    }
    let section_digest = sha256::digest(&body);
    let schema = schema_digest();

    let mut out = Vec::with_capacity(HEADER_SIZE as usize + body.len());
    out.extend_from_slice(&RXPL_MAGIC);
    put_u32(&mut out, FORMAT_ID);
    put_u16(&mut out, LOGICAL_MAJOR);
    put_u16(&mut out, LOGICAL_MINOR);
    out.push(ENDIAN_LE);
    out.push(page.flags);
    put_u16(&mut out, HEADER_SIZE);
    put_u64(&mut out, page.page_id);
    put_u16(&mut out, page.lod_level_min);
    put_u16(&mut out, page.lod_level_max);
    put_u32(&mut out, page.clusters.len() as u32);
    put_u32(&mut out, page.vertices.len() as u32);
    put_u32(&mut out, page.indices.len() as u32);
    for &b in &page.bounds {
        put_f32(&mut out, b);
    }
    put_u32(&mut out, page.dependency_page_ids.len() as u32);
    put_u32(&mut out, page.dag_links.len() as u32);
    out.extend_from_slice(&schema);
    out.extend_from_slice(&section_digest);
    debug_assert_eq!(out.len(), HEADER_SIZE as usize);
    out.extend_from_slice(&body);
    out
}

/// 解码逻辑页。**先**校验 magic/major，再消费段体（RXS-0331）。
pub fn decode_logical_page(bytes: &[u8]) -> Result<LogicalPage, PageDecodeError> {
    // 消费前拒录：至少读到 major（偏移 8..10）需要 10 字节；完整 header 校验前先 magic/major。
    if bytes.len() < 12 {
        // 仍需区分 bad magic（若有足够字节）
        if bytes.len() >= 4 && bytes[0..4] != RXPL_MAGIC {
            return Err(PageDecodeError::BadMagic);
        }
        return Err(PageDecodeError::Truncated("header"));
    }
    if bytes[0..4] != RXPL_MAGIC {
        return Err(PageDecodeError::BadMagic);
    }
    let major = u16::from_le_bytes([bytes[8], bytes[9]]);
    let minor = u16::from_le_bytes([bytes[10], bytes[11]]);
    if major != LOGICAL_MAJOR {
        return Err(PageDecodeError::UnsupportedVersion { major, minor });
    }
    if bytes.len() < HEADER_SIZE as usize {
        return Err(PageDecodeError::Truncated("header"));
    }

    let format_id = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    if format_id != FORMAT_ID {
        return Err(PageDecodeError::BadHeader("format_id"));
    }
    if bytes[12] != ENDIAN_LE {
        return Err(PageDecodeError::BadHeader("endian"));
    }
    let flags = bytes[13];
    if flags & !FLAG_ROOT != 0 {
        return Err(PageDecodeError::BadHeader("flags"));
    }
    let header_size = u16::from_le_bytes([bytes[14], bytes[15]]);
    if header_size != HEADER_SIZE {
        return Err(PageDecodeError::BadHeader("header_size"));
    }
    let page_id = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let lod_level_min = u16::from_le_bytes([bytes[24], bytes[25]]);
    let lod_level_max = u16::from_le_bytes([bytes[26], bytes[27]]);
    let cluster_count = u32::from_le_bytes(bytes[28..32].try_into().unwrap()) as usize;
    let vertex_count = u32::from_le_bytes(bytes[32..36].try_into().unwrap()) as usize;
    let index_count = u32::from_le_bytes(bytes[36..40].try_into().unwrap()) as usize;
    let mut bounds = [0f32; 6];
    for (i, b) in bounds.iter_mut().enumerate() {
        let o = 40 + i * 4;
        *b = f32::from_le_bytes(bytes[o..o + 4].try_into().unwrap());
    }
    let dependency_page_count = u32::from_le_bytes(bytes[64..68].try_into().unwrap()) as usize;
    let dag_link_count = u32::from_le_bytes(bytes[68..72].try_into().unwrap()) as usize;
    let mut schema = [0u8; 32];
    schema.copy_from_slice(&bytes[72..104]);
    let mut section_dg = [0u8; 32];
    section_dg.copy_from_slice(&bytes[104..136]);

    if schema != schema_digest() {
        return Err(PageDecodeError::DigestMismatch("schema_digest"));
    }

    let need = cluster_count * RECORD_SIZE as usize
        + vertex_count * 12
        + index_count
        + dependency_page_count * 8
        + dag_link_count * 8;
    let body = bytes
        .get(HEADER_SIZE as usize..)
        .ok_or(PageDecodeError::Truncated("body"))?;
    if body.len() < need {
        return Err(PageDecodeError::Truncated("sections"));
    }
    if body.len() != need {
        return Err(PageDecodeError::Inconsistent("trailing_bytes"));
    }
    if sha256::digest(body) != section_dg {
        return Err(PageDecodeError::DigestMismatch("section_digest"));
    }

    let mut cur = Cursor {
        bytes: body,
        pos: 0,
    };
    let mut clusters = Vec::with_capacity(cluster_count);
    for _ in 0..cluster_count {
        clusters.push(take_record(&mut cur)?);
    }
    let mut vertices = Vec::with_capacity(vertex_count);
    for _ in 0..vertex_count {
        let x = cur.f32().ok_or(PageDecodeError::Truncated("vertex"))?;
        let y = cur.f32().ok_or(PageDecodeError::Truncated("vertex"))?;
        let z = cur.f32().ok_or(PageDecodeError::Truncated("vertex"))?;
        vertices.push([x, y, z]);
    }
    let indices = cur
        .take(index_count)
        .ok_or(PageDecodeError::Truncated("indices"))?
        .to_vec();
    let mut dependency_page_ids = Vec::with_capacity(dependency_page_count);
    for _ in 0..dependency_page_count {
        dependency_page_ids.push(cur.u64().ok_or(PageDecodeError::Truncated("deps"))?);
    }
    let mut dag_links = Vec::with_capacity(dag_link_count);
    for _ in 0..dag_link_count {
        let p = cur.u32().ok_or(PageDecodeError::Truncated("links"))?;
        let c = cur.u32().ok_or(PageDecodeError::Truncated("links"))?;
        dag_links.push((p, c));
    }
    if cur.pos != body.len() {
        return Err(PageDecodeError::Inconsistent("cursor"));
    }

    Ok(LogicalPage {
        page_id,
        flags,
        lod_level_min,
        lod_level_max,
        bounds,
        clusters,
        vertices,
        indices,
        dependency_page_ids,
        dag_links,
    })
}

/// 页 AABB 量化中心 → u16×3（RXS-0328）。
pub fn quantize_center(center: [f32; 3], bounds: [f32; 6]) -> (u16, u16, u16) {
    const EPS: f32 = 1.175_494_4e-38; // 2^-126
    let q = |c: f32, lo: f32, hi: f32| -> u16 {
        let span = (hi - lo).max(EPS);
        let t = ((c - lo) / span).clamp(0.0, 1.0);
        (t * 65535.0).round() as u16
    };
    (
        q(center[0], bounds[0], bounds[3]),
        q(center[1], bounds[1], bounds[4]),
        q(center[2], bounds[2], bounds[5]),
    )
}

fn put_record(b: &mut Vec<u8>, c: &PageClusterRecord) {
    let start = b.len();
    put_u32(b, c.cluster_id);
    put_u16(b, c.qx);
    put_u16(b, c.qy);
    put_u16(b, c.qz);
    put_u16(b, 0); // pad
    put_f32(b, c.center[0]);
    put_f32(b, c.center[1]);
    put_f32(b, c.center[2]);
    put_f32(b, c.radius);
    put_f32(b, c.cone_axis[0]);
    put_f32(b, c.cone_axis[1]);
    put_f32(b, c.cone_axis[2]);
    put_f32(b, c.cone_cutoff);
    put_f32(b, c.error);
    put_f32(b, c.parent_error);
    put_u32(b, c.vertex_offset);
    put_u32(b, c.triangle_offset);
    put_u32(b, c.vertex_count);
    put_u32(b, c.triangle_count);
    put_u32(b, c.level);
    put_u32(b, c.group);
    put_u32(b, 0);
    put_u32(b, 0);
    put_u32(b, 0);
    put_u32(b, 0);
    put_u32(b, 0);
    debug_assert_eq!(b.len() - start, RECORD_SIZE as usize);
}

fn take_record(c: &mut Cursor<'_>) -> Result<PageClusterRecord, PageDecodeError> {
    let cluster_id = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    let qx = c.u16().ok_or(PageDecodeError::Truncated("record"))?;
    let qy = c.u16().ok_or(PageDecodeError::Truncated("record"))?;
    let qz = c.u16().ok_or(PageDecodeError::Truncated("record"))?;
    let pad = c.u16().ok_or(PageDecodeError::Truncated("record"))?;
    if pad != 0 {
        return Err(PageDecodeError::Inconsistent("record.pad"));
    }
    let cx = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let cy = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let cz = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let radius = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let ax = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let ay = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let az = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let cone_cutoff = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let error = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let parent_error = c.f32().ok_or(PageDecodeError::Truncated("record"))?;
    let vertex_offset = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    let triangle_offset = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    let vertex_count = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    let triangle_count = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    let level = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    let group = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
    for _ in 0..5 {
        let r = c.u32().ok_or(PageDecodeError::Truncated("record"))?;
        if r != 0 {
            return Err(PageDecodeError::Inconsistent("record.reserved"));
        }
    }
    Ok(PageClusterRecord {
        cluster_id,
        qx,
        qy,
        qz,
        center: [cx, cy, cz],
        radius,
        cone_axis: [ax, ay, az],
        cone_cutoff,
        error,
        parent_error,
        vertex_offset,
        triangle_offset,
        vertex_count,
        triangle_count,
        level,
        group,
    })
}

fn put_u16(b: &mut Vec<u8>, v: u16) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_u64(b: &mut Vec<u8>, v: u64) {
    b.extend_from_slice(&v.to_le_bytes());
}
fn put_f32(b: &mut Vec<u8>, v: f32) {
    b.extend_from_slice(&v.to_le_bytes());
}

struct Cursor<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.pos.checked_add(n)?;
        let s = self.bytes.get(self.pos..end)?;
        self.pos = end;
        Some(s)
    }
    fn u16(&mut self) -> Option<u16> {
        Some(u16::from_le_bytes(self.take(2)?.try_into().ok()?))
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_page() -> LogicalPage {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.25, -0.5, 0.75];
        let (qx, qy, qz) = quantize_center(center, bounds);
        LogicalPage {
            page_id: 0,
            flags: FLAG_ROOT,
            lod_level_min: 0,
            lod_level_max: 0,
            bounds,
            clusters: vec![PageClusterRecord {
                cluster_id: 0,
                qx,
                qy,
                qz,
                center,
                radius: 1.5,
                cone_axis: [0.0, 1.0, 0.0],
                cone_cutoff: 0.1,
                error: 0.0,
                parent_error: f32::MAX,
                vertex_offset: 0,
                triangle_offset: 0,
                vertex_count: 3,
                triangle_count: 1,
                level: 0,
                group: 0,
            }],
            vertices: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            dependency_page_ids: vec![],
            dag_links: vec![],
        }
    }

    //@ spec: RXS-0328
    #[test]
    fn roundtrip_byte_equal() {
        let page = sample_page();
        let bytes = encode_logical_page(&page);
        assert_eq!(&bytes[0..4], b"RXPL");
        assert_eq!(bytes.len(), page.encoded_len());
        let back = decode_logical_page(&bytes).expect("decode");
        assert_eq!(back.page_id, page.page_id);
        assert_eq!(back.flags, page.flags);
        assert_eq!(back.clusters.len(), 1);
        assert_eq!(
            back.clusters[0].center.map(f32::to_bits),
            page.clusters[0].center.map(f32::to_bits)
        );
        assert_eq!(encode_logical_page(&back), bytes);
    }

    //@ spec: RXS-0331
    #[test]
    fn rejects_bad_magic_before_sections() {
        let mut bytes = encode_logical_page(&sample_page());
        bytes[0] = b'X';
        assert_eq!(decode_logical_page(&bytes), Err(PageDecodeError::BadMagic));
    }

    //@ spec: RXS-0331
    #[test]
    fn rejects_unknown_major_before_sections() {
        let mut bytes = encode_logical_page(&sample_page());
        bytes[8] = 9;
        bytes[9] = 0;
        assert_eq!(
            decode_logical_page(&bytes),
            Err(PageDecodeError::UnsupportedVersion { major: 9, minor: 0 })
        );
    }

    //@ spec: RXS-0331
    #[test]
    fn rejects_truncated() {
        let bytes = encode_logical_page(&sample_page());
        assert!(matches!(
            decode_logical_page(&bytes[..20]),
            Err(PageDecodeError::Truncated(_))
        ));
    }

    //@ spec: RXS-0328
    #[test]
    fn schema_digest_stable() {
        let d = schema_digest();
        assert_eq!(d, schema_digest());
        assert_ne!(d, [0u8; 32]);
    }

    //@ spec: RXS-0328
    #[test]
    fn header_size_literal() {
        let bytes = encode_logical_page(&sample_page());
        assert_eq!(HEADER_SIZE, 136);
        assert_eq!(u16::from_le_bytes([bytes[14], bytes[15]]), HEADER_SIZE);
        assert_eq!(&bytes[72..104], &schema_digest());
    }
}
