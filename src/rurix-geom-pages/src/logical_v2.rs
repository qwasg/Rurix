//! 逻辑页 RXPL **major=2** encode/decode(RXS-0344;RFC-0022 §4.5;G9_ACCEPTANCE_MAP M91)。
//!
//! 演进面:v1 簇记录(96B 定长)之外新增**簇误差/包围球/骨骼元数据/CLAS 输入段**
//! (字段 schema 见 spec/virtual_geometry.md RXS-0345)。v1 既有面
//! (`logical::{encode,decode}`)0-byte 不动;v2 页在 header 126..128 携带 v1 段字节数,
//! section_digest 覆盖全段(v1 段 + 变长骨骼索引段 + CLAS 段)——v1/v2 共存靠
//! major 分发,互不解析对方布局(未知 major fail-closed,沿 RXS-0331
//! `UnsupportedVersion` 族)。
//!
//! v2 段布局(LE;全 f32 逐位搬运,零精度变换):
//!
//! ```text
//! [v1 段]      与 RXS-0328 段序逐字一致(96B 记录 + 顶点 + 索引 + 依赖 + DAG 边),
//!              字节数记于 header 126..128(v1_section_bytes)
//! [骨骼元数据]  cluster_count × 12B 定长头:
//!               max_influences:u32 / bone_count:u32 / bound_inflation:f32
//! [骨骼索引集]  变长,Σ bone_count × u32(簇序同定长头;0..bone_count 升序追加)
//! [CLAS 输入]  cluster_count × 32B:
//!               triangle_offset/triangle_count(vertex_offset/vertex_count 与 v1 记录同义,
//!               不重发)+ aabb_min[3] + aabb_max[3](三角形簇几何引用 v1 顶点/索引段)
//! ```
//!
//! schema_digest preimage:域分离字串 `b"RXPL-SCHEMA-V2\0"` 起首 + `major:u16=2` +
//! minor + STREAM_PAGE_SIZE + HEADER_SIZE_V2 + RECORD_SIZE(沿用 v1 96B)+
//! PACKING_ALGO_ID + FORMAT_ID + v2 段序标识串(逐字段拼接律沿 RXS-0328)。

use rurix_pkg::sha256;

use crate::logical::{
    ENDIAN_LE, FLAG_ROOT, FORMAT_ID, PACKING_ALGO_ID, PageDecodeError, RXPL_MAGIC, STREAM_PAGE_SIZE,
};

/// v2 主版本号(与 v1 `LOGICAL_MAJOR=1` 不同且冻结)。
pub const LOGICAL_MAJOR_V2: u16 = 2;
/// v2 次版本号。
pub const LOGICAL_MINOR_V2: u16 = 0;
/// v2 header 尺寸:138..140 = v1_section_bytes:u16,140..144 = 定长骨骼头字节数:u32,
/// 144..148 = 骨骼索引集段字节数:u32,148..152 = CLAS 段字节数:u32,152..160 = 保留 8B(恒 0)。
/// (136..138 恒 0 与 v1 header 面逐字同构——v1/v2 前 138B 仅 major/header_size 不同。)
pub const HEADER_SIZE_V2: u16 = 160;
/// v2 骨骼元数据定长头每簇 12B。
pub const SKIN_RECORD_SIZE: usize = 12;
/// v2 CLAS 输入每簇 32B。
pub const CLAS_RECORD_SIZE: usize = 32;

/// 单簇 v2 深化记录(误差/包围球经 v1 记录面携带;本结构 = 骨骼元数据 + CLAS 输入)。
#[derive(Debug, Clone, PartialEq)]
pub struct V2ClusterExt {
    /// 蒙皮元数据:最大影响骨数(RXS-0345 §3.3;0 = 非蒙皮簇)。
    pub max_influences: u32,
    /// 蒙皮元数据:骨骼索引集(升序,确定性编码;元素数与定长头 bone_count 一致)。
    pub bone_indices: Vec<u32>,
    /// 蒙皮元数据:蒙皮包围体膨胀系数(Kerbl 保守界输入;非蒙皮簇 = 0.0)。
    pub bound_inflation: f32,
    /// CLAS 离线烘焙输入:簇级 AABB min(三角形簇几何引用 v1 顶点/索引段)。
    pub aabb_min: [f32; 3],
    /// CLAS 离线烘焙输入:簇级 AABB max。
    pub aabb_max: [f32; 3],
}

impl V2ClusterExt {
    /// 非蒙皮簇扩展(骨骼三字段零值 + AABB)。
    pub fn unskinned(aabb_min: [f32; 3], aabb_max: [f32; 3]) -> Self {
        Self {
            max_influences: 0,
            bone_indices: Vec::new(),
            bound_inflation: 0.0,
            aabb_min,
            aabb_max,
        }
    }
}

/// RXPL major=2 逻辑页。
#[derive(Debug, Clone, PartialEq)]
pub struct LogicalPageV2 {
    /// v1 逻辑页冻结面(RXS-0328;含簇误差/包围球/锥/层级/组)。
    pub base: crate::logical::LogicalPage,
    /// 与 `base.clusters` 等长平行的 v2 扩展表(骨骼元数据 + CLAS 输入)。
    pub ext: Vec<V2ClusterExt>,
}

/// v2 冻结 schema_digest(`b"RXPL-SCHEMA-V2\0"` preimage;RXS-0344 §1)。
pub fn schema_digest_v2() -> [u8; 32] {
    let mut pre = Vec::with_capacity(96);
    pre.extend_from_slice(b"RXPL-SCHEMA-V2\0");
    put_u16(&mut pre, LOGICAL_MAJOR_V2);
    put_u16(&mut pre, LOGICAL_MINOR_V2);
    put_u32(&mut pre, STREAM_PAGE_SIZE);
    put_u16(&mut pre, HEADER_SIZE_V2);
    put_u16(&mut pre, crate::logical::RECORD_SIZE);
    put_u32(&mut pre, PACKING_ALGO_ID);
    put_u32(&mut pre, FORMAT_ID);
    // v2 段序标识(record_size / 段序 / 新增段标识随本布局冻结并进 digest)。
    pre.extend_from_slice(
        b"v2:clusters96,vertices,indices,deps,daglinks,skin_hdr,bone_idx,clas_aabb\0",
    );
    sha256::digest(&pre)
}

/// 编码 RXPL major=2 页(确定性;v1 段字节复用 v1 段序编码,非 v1 页字节)。
pub fn encode_logical_page_v2(page: &LogicalPageV2) -> Vec<u8> {
    let base = &page.base;
    debug_assert_eq!(page.ext.len(), base.clusters.len());

    // v1 段(与 logical::encode_logical_page body 段序逐字一致)。
    let mut v1 = Vec::new();
    for c in &base.clusters {
        put_record_v1(&mut v1, c);
    }
    for v in &base.vertices {
        for &x in v {
            put_f32(&mut v1, x);
        }
    }
    v1.extend_from_slice(&base.indices);
    for &id in &base.dependency_page_ids {
        put_u64(&mut v1, id);
    }
    for &(p, c) in &base.dag_links {
        put_u32(&mut v1, p);
        put_u32(&mut v1, c);
    }
    // v1 段 4B 对齐填充(device 侧 skin/clas 段须 u32 字读;填充字节恒 0,
    // 解码按计数消费 v1 段内容、以 v1_len 定位 skin 段——填充不进内容解析)。
    while v1.len() % 4 != 0 {
        v1.push(0);
    }

    // 骨骼元数据定长头 + 变长索引集。
    let mut skin_hdr = Vec::with_capacity(page.ext.len() * SKIN_RECORD_SIZE);
    let mut bone_idx = Vec::new();
    for e in &page.ext {
        put_u32(&mut skin_hdr, e.max_influences);
        put_u32(&mut skin_hdr, e.bone_indices.len() as u32);
        put_f32(&mut skin_hdr, e.bound_inflation);
        for &b in &e.bone_indices {
            put_u32(&mut bone_idx, b);
        }
    }

    // CLAS 输入段(三角形簇几何 = v1 记录 vertex/triangle offset/count,不重发)。
    let mut clas = Vec::with_capacity(page.ext.len() * CLAS_RECORD_SIZE);
    for (c, e) in base.clusters.iter().zip(page.ext.iter()) {
        put_u32(&mut clas, c.triangle_offset);
        put_u32(&mut clas, c.triangle_count);
        put_f32(&mut clas, e.aabb_min[0]);
        put_f32(&mut clas, e.aabb_min[1]);
        put_f32(&mut clas, e.aabb_min[2]);
        put_f32(&mut clas, e.aabb_max[0]);
        put_f32(&mut clas, e.aabb_max[1]);
        put_f32(&mut clas, e.aabb_max[2]);
    }

    let mut body = Vec::with_capacity(v1.len() + skin_hdr.len() + bone_idx.len() + clas.len());
    body.extend_from_slice(&v1);
    body.extend_from_slice(&skin_hdr);
    body.extend_from_slice(&bone_idx);
    body.extend_from_slice(&clas);
    let section_digest = sha256::digest(&body);
    let schema = schema_digest_v2();

    let mut out = Vec::with_capacity(HEADER_SIZE_V2 as usize + body.len());
    out.extend_from_slice(&RXPL_MAGIC);
    put_u32(&mut out, FORMAT_ID);
    put_u16(&mut out, LOGICAL_MAJOR_V2);
    put_u16(&mut out, LOGICAL_MINOR_V2);
    out.push(ENDIAN_LE);
    out.push(base.flags);
    put_u16(&mut out, HEADER_SIZE_V2);
    put_u64(&mut out, base.page_id);
    put_u16(&mut out, base.lod_level_min);
    put_u16(&mut out, base.lod_level_max);
    put_u32(&mut out, base.clusters.len() as u32);
    put_u32(&mut out, base.vertices.len() as u32);
    put_u32(&mut out, base.indices.len() as u32);
    for &b in &base.bounds {
        put_f32(&mut out, b);
    }
    put_u32(&mut out, base.dependency_page_ids.len() as u32);
    put_u32(&mut out, base.dag_links.len() as u32);
    out.extend_from_slice(&schema);
    out.extend_from_slice(&section_digest);
    debug_assert_eq!(out.len(), 136);
    // v2 header 扩展(24B):136..138 恒 0(v1 面同构)+ v1 段字节数 + v2 三段字节数
    // + 保留 8B(恒 0)。
    put_u16(&mut out, 0);
    put_u16(&mut out, v1.len() as u16);
    put_u32(&mut out, skin_hdr.len() as u32);
    put_u32(&mut out, bone_idx.len() as u32);
    put_u32(&mut out, clas.len() as u32);
    out.extend_from_slice(&[0u8; 8]);
    debug_assert_eq!(out.len(), HEADER_SIZE_V2 as usize);
    out.extend_from_slice(&body);
    out
}

/// 解码 RXPL major=2 页。**先**校验 magic/major=2,再消费段体(RXS-0344 §3;
/// 篡改 schema_digest/section_digest 确定性拒绝,不按猜测布局解析)。
///
/// 本函数只消费 major=2;major=1 经 `decode_logical_page_any` 分发到 v1 臂,
/// 其余 major 一律 `UnsupportedVersion`。
pub fn decode_logical_page_v2(bytes: &[u8]) -> Result<LogicalPageV2, PageDecodeError> {
    // 消费前拒录:至少读到 major(8..10);坏 magic 优先(沿 v1 体例)。
    if bytes.len() < 12 {
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
    if major != LOGICAL_MAJOR_V2 {
        return Err(PageDecodeError::UnsupportedVersion { major, minor });
    }
    if bytes.len() < HEADER_SIZE_V2 as usize {
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
    if header_size != HEADER_SIZE_V2 {
        return Err(PageDecodeError::BadHeader("header_size"));
    }
    let page_id = u64::from_le_bytes(bytes[16..24].try_into().unwrap());
    let lod_level_min = u16::from_le_bytes(bytes[24..26].try_into().unwrap());
    let lod_level_max = u16::from_le_bytes(bytes[26..28].try_into().unwrap());
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

    if schema != schema_digest_v2() {
        return Err(PageDecodeError::DigestMismatch("schema_digest"));
    }

    if bytes[136] != 0 || bytes[137] != 0 {
        return Err(PageDecodeError::BadHeader("v2_pad"));
    }
    let v1_len = u16::from_le_bytes([bytes[138], bytes[139]]) as usize;
    let skin_hdr_len = u32::from_le_bytes(bytes[140..144].try_into().unwrap()) as usize;
    let bone_idx_len = u32::from_le_bytes(bytes[144..148].try_into().unwrap()) as usize;
    let clas_len = u32::from_le_bytes(bytes[148..152].try_into().unwrap()) as usize;
    if bytes[152..160].iter().any(|&b| b != 0) {
        return Err(PageDecodeError::BadHeader("v2_reserved"));
    }

    // v1 段最小尺寸下限(记录数与段计数一致性在游标消费处再核);
    // v1_len 含 4B 对齐填充(填充恒 0,见 encode 侧对齐纪律)。
    let v1_need = cluster_count * crate::logical::RECORD_SIZE as usize
        + vertex_count * 12
        + index_count
        + dependency_page_count * 8
        + dag_link_count * 8;
    let v1_need_aligned = (v1_need + 3) & !3;
    if v1_len != v1_need_aligned {
        return Err(PageDecodeError::Inconsistent("v1_section_bytes"));
    }
    // 填充字节恒 0(v1 段末)。
    if v1_need_aligned > v1_need {
        let pad_start = HEADER_SIZE_V2 as usize + v1_need;
        if bytes[pad_start..HEADER_SIZE_V2 as usize + v1_need_aligned]
            .iter()
            .any(|&b| b != 0)
        {
            return Err(PageDecodeError::Inconsistent("v1_align_pad"));
        }
    }
    if skin_hdr_len != cluster_count * SKIN_RECORD_SIZE {
        return Err(PageDecodeError::Inconsistent("skin_hdr_bytes"));
    }
    if clas_len != cluster_count * CLAS_RECORD_SIZE {
        return Err(PageDecodeError::Inconsistent("clas_bytes"));
    }
    let need = v1_len
        .checked_add(skin_hdr_len)
        .and_then(|n| n.checked_add(bone_idx_len))
        .and_then(|n| n.checked_add(clas_len))
        .ok_or(PageDecodeError::Inconsistent("section_bytes_overflow"))?;
    let body = bytes
        .get(HEADER_SIZE_V2 as usize..)
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

    // v1 段游标消费(与 v1 解码段序逐字一致)。
    let mut cur = Cursor {
        bytes: body,
        pos: 0,
    };
    let mut clusters = Vec::with_capacity(cluster_count);
    for _ in 0..cluster_count {
        clusters.push(take_record_v1(&mut cur)?);
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
    // v1 段内容按计数消费;填充字节(至 4B 对齐)跳过。
    let v1_content_end = v1_need_aligned - ((v1_need_aligned - v1_need) % 4);
    if cur.pos != v1_content_end {
        return Err(PageDecodeError::Inconsistent("v1_cursor"));
    }
    cur.pos = v1_len; // 跳过对齐填充(v1_len 已核 = v1_need_aligned)

    // 骨骼元数据段:先全量定长头(可知 bone_count),再变长索引集。
    let mut hdrs = Vec::with_capacity(cluster_count);
    for _ in 0..cluster_count {
        let max_influences = cur.u32().ok_or(PageDecodeError::Truncated("skin_hdr"))?;
        let bone_count = cur.u32().ok_or(PageDecodeError::Truncated("skin_hdr"))?;
        let bound_inflation = cur.f32().ok_or(PageDecodeError::Truncated("skin_hdr"))?;
        hdrs.push((max_influences, bone_count, bound_inflation));
    }
    if cur.pos != v1_len + skin_hdr_len {
        return Err(PageDecodeError::Inconsistent("skin_cursor"));
    }
    let bone_total: usize = hdrs.iter().map(|h| h.1 as usize).sum();
    if bone_idx_len != bone_total * 4 {
        return Err(PageDecodeError::Inconsistent("bone_idx_bytes"));
    }
    let mut all_bones = Vec::with_capacity(bone_total);
    for _ in 0..bone_total {
        all_bones.push(cur.u32().ok_or(PageDecodeError::Truncated("bone_idx"))?);
    }
    if cur.pos != v1_len + skin_hdr_len + bone_idx_len {
        return Err(PageDecodeError::Inconsistent("bone_cursor"));
    }

    // CLAS 输入段:offset/count 必须与 v1 记录一致(不猜布局,不一致即拒)。
    let mut ext = Vec::with_capacity(cluster_count);
    let mut bone_at = 0usize;
    for (ci, c) in clusters.iter().enumerate() {
        let tri_off = cur.u32().ok_or(PageDecodeError::Truncated("clas"))?;
        let tri_cnt = cur.u32().ok_or(PageDecodeError::Truncated("clas"))?;
        let mut aabb_min = [0f32; 3];
        let mut aabb_max = [0f32; 3];
        for b in aabb_min.iter_mut() {
            *b = cur.f32().ok_or(PageDecodeError::Truncated("clas"))?;
        }
        for b in aabb_max.iter_mut() {
            *b = cur.f32().ok_or(PageDecodeError::Truncated("clas"))?;
        }
        if tri_off != c.triangle_offset || tri_cnt != c.triangle_count {
            return Err(PageDecodeError::Inconsistent("clas_geometry_ref"));
        }
        let (max_influences, bone_count, bound_inflation) = hdrs[ci];
        let bone_indices = all_bones[bone_at..bone_at + bone_count as usize].to_vec();
        bone_at += bone_count as usize;
        ext.push(V2ClusterExt {
            max_influences,
            bone_indices,
            bound_inflation,
            aabb_min,
            aabb_max,
        });
    }
    if cur.pos != body.len() {
        return Err(PageDecodeError::Inconsistent("cursor"));
    }

    Ok(LogicalPageV2 {
        base: crate::logical::LogicalPage {
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
        },
        ext,
    })
}

/// major 分发器:v1 → v1 臂,v2 → v2 臂,其余 fail-closed(RXS-0344 §3)。
/// v1/v2 页可经本函数在同一流送系统共存消费,v1 路径字节 0-byte。
pub fn decode_logical_page_any(
    bytes: &[u8],
) -> Result<crate::logical::LogicalPage, PageDecodeError> {
    if bytes.len() < 12 {
        if bytes.len() >= 4 && bytes[0..4] != RXPL_MAGIC {
            return Err(PageDecodeError::BadMagic);
        }
        return Err(PageDecodeError::Truncated("header"));
    }
    if bytes[0..4] != RXPL_MAGIC {
        return Err(PageDecodeError::BadMagic);
    }
    let major = u16::from_le_bytes([bytes[8], bytes[9]]);
    match major {
        crate::logical::LOGICAL_MAJOR => crate::logical::decode_logical_page(bytes),
        LOGICAL_MAJOR_V2 => Ok(decode_logical_page_v2(bytes)?.base),
        other => Err(PageDecodeError::UnsupportedVersion {
            major: other,
            minor: u16::from_le_bytes([bytes[10], bytes[11]]),
        }),
    }
}

/// v2 页编码后总字节(header + 全段)。
pub fn encoded_len_v2(page: &LogicalPageV2) -> usize {
    let v1 = page.base.clusters.len() * crate::logical::RECORD_SIZE as usize
        + page.base.vertices.len() * 12
        + page.base.indices.len()
        + page.base.dependency_page_ids.len() * 8
        + page.base.dag_links.len() * 8;
    let v1_aligned = (v1 + 3) & !3; // v1 段 4B 对齐填充(encode 侧纪律)
    let bone: usize = page.ext.iter().map(|e| e.bone_indices.len() * 4).sum();
    HEADER_SIZE_V2 as usize
        + v1_aligned
        + page.ext.len() * SKIN_RECORD_SIZE
        + bone
        + page.ext.len() * CLAS_RECORD_SIZE
}

// —— v1 记录 96B 读写(v1 段序复用;与 logical.rs put/take_record 逐字节同构)——

fn put_record_v1(b: &mut Vec<u8>, c: &crate::logical::PageClusterRecord) {
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
    debug_assert_eq!(b.len() - start, crate::logical::RECORD_SIZE as usize);
}

fn take_record_v1(
    c: &mut Cursor<'_>,
) -> Result<crate::logical::PageClusterRecord, PageDecodeError> {
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
    Ok(crate::logical::PageClusterRecord {
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
    use crate::logical::{LogicalPage, PageClusterRecord, quantize_center};

    fn sample_page_v2() -> LogicalPageV2 {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let center = [0.25, -0.5, 0.75];
        let (qx, qy, qz) = quantize_center(center, bounds);
        LogicalPageV2 {
            base: LogicalPage {
                page_id: 0,
                flags: FLAG_ROOT,
                lod_level_min: 0,
                lod_level_max: 0,
                bounds,
                clusters: vec![
                    PageClusterRecord {
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
                    },
                    PageClusterRecord {
                        cluster_id: 1,
                        qx,
                        qy,
                        qz,
                        center,
                        radius: 1.5,
                        cone_axis: [0.0, 1.0, 0.0],
                        cone_cutoff: 0.1,
                        error: 0.25,
                        parent_error: 0.5,
                        vertex_offset: 3,
                        triangle_offset: 3,
                        vertex_count: 3,
                        triangle_count: 1,
                        level: 0,
                        group: 0,
                    },
                ],
                vertices: vec![
                    [0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 1.0],
                    [1.0, 1.0, 0.0],
                    [1.0, 0.0, 1.0],
                ],
                indices: vec![0, 1, 2, 3, 4, 5],
                dependency_page_ids: vec![],
                dag_links: vec![(0, 1)],
            },
            ext: vec![
                V2ClusterExt {
                    max_influences: 4,
                    bone_indices: vec![0, 3, 9],
                    bound_inflation: 0.125,
                    aabb_min: [-0.5, -0.5, -0.5],
                    aabb_max: [0.5, 0.5, 0.5],
                },
                V2ClusterExt::unskinned([-1.0, -1.0, -1.0], [1.0, 1.0, 1.0]),
            ],
        }
    }

    //@ spec: RXS-0344
    #[test]
    fn roundtrip_byte_equal() {
        let page = sample_page_v2();
        let bytes = encode_logical_page_v2(&page);
        assert_eq!(&bytes[0..4], b"RXPL");
        assert_eq!(bytes.len(), encoded_len_v2(&page));
        assert_eq!(u16::from_le_bytes([bytes[8], bytes[9]]), LOGICAL_MAJOR_V2);
        assert_eq!(u16::from_le_bytes([bytes[14], bytes[16]]), HEADER_SIZE_V2);
        let back = decode_logical_page_v2(&bytes).expect("decode");
        assert_eq!(back, page);
        // 编码→解码→再编码逐字节相等(往返无损)。
        assert_eq!(encode_logical_page_v2(&back), bytes);
    }

    //@ spec: RXS-0344
    #[test]
    fn schema_preimage_distinct_and_stable() {
        let v1 = crate::logical::schema_digest();
        let v2 = schema_digest_v2();
        assert_ne!(v1, v2);
        assert_eq!(v2, schema_digest_v2());
        let mut pre = Vec::new();
        pre.extend_from_slice(b"RXPL-SCHEMA-V2\0");
        pre.extend_from_slice(&2u16.to_le_bytes());
        let recomputed = sha256::digest(
            &[
                pre.as_slice(),
                &0u16.to_le_bytes(),
                &STREAM_PAGE_SIZE.to_le_bytes(),
                &HEADER_SIZE_V2.to_le_bytes(),
                &crate::logical::RECORD_SIZE.to_le_bytes(),
                &PACKING_ALGO_ID.to_le_bytes(),
                &FORMAT_ID.to_le_bytes(),
                b"v2:clusters96,vertices,indices,deps,daglinks,skin_hdr,bone_idx,clas_aabb\0",
            ]
            .concat(),
        );
        assert_eq!(v2, recomputed, "preimage 逐字段拼接律漂移");
    }

    //@ spec: RXS-0344
    #[test]
    fn unknown_major_fail_closed() {
        let bytes = encode_logical_page_v2(&sample_page_v2());
        let mut v9 = bytes.clone();
        v9[8] = 9;
        v9[9] = 0;
        assert_eq!(
            decode_logical_page_v2(&v9),
            Err(PageDecodeError::UnsupportedVersion { major: 9, minor: 0 })
        );
        assert!(matches!(
            decode_logical_page_any(&v9),
            Err(PageDecodeError::UnsupportedVersion { major: 9, .. })
        ));
        // v1 页走 any 分发到 v1 臂;v2 臂不收 v1。
        let v1 = crate::logical::encode_logical_page(&sample_page_v2().base);
        assert!(crate::logical::decode_logical_page(&v1).is_ok());
        assert!(matches!(
            decode_logical_page_v2(&v1),
            Err(PageDecodeError::UnsupportedVersion { major: 1, .. })
        ));
        let any = decode_logical_page_any(&v1).expect("v1 via any");
        assert_eq!(any.clusters.len(), 2);
        // v2 页走 any 分发到 v2 臂。
        let any2 = decode_logical_page_any(&bytes).expect("v2 via any");
        assert_eq!(any2.clusters.len(), 2);
    }

    //@ spec: RXS-0344
    #[test]
    fn tampered_digests_fail_closed() {
        let bytes = encode_logical_page_v2(&sample_page_v2());
        let mut bad_schema = bytes.clone();
        bad_schema[72] ^= 0x01;
        assert_eq!(
            decode_logical_page_v2(&bad_schema),
            Err(PageDecodeError::DigestMismatch("schema_digest"))
        );
        let mut bad_section = bytes.clone();
        bad_section[104] ^= 0x01;
        assert_eq!(
            decode_logical_page_v2(&bad_section),
            Err(PageDecodeError::DigestMismatch("section_digest"))
        );
        // 段体篡改(不重算 section_digest)→ 同拒。
        let mut bad_body = bytes.clone();
        let last = bad_body.len() - 1;
        bad_body[last] ^= 0x01;
        assert_eq!(
            decode_logical_page_v2(&bad_body),
            Err(PageDecodeError::DigestMismatch("section_digest"))
        );
    }

    //@ spec: RXS-0344
    #[test]
    fn inconsistent_section_lengths_fail_closed() {
        let bytes = encode_logical_page_v2(&sample_page_v2());
        // 篡改 v1 段字节数(不重算 digest;先被一致性拒)。
        let mut bad = bytes.clone();
        bad[138] = 0;
        bad[139] = 0;
        assert_eq!(
            decode_logical_page_v2(&bad),
            Err(PageDecodeError::Inconsistent("v1_section_bytes"))
        );
    }
}
