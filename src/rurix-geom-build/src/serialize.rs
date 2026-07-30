//! RXGB 二进制序列化(报告1 §5:P0 序列化格式 v0 **预留页表字段**——
//! `ClusterRecord::page_id` 恒写 0,P4 流送启用;§6.1 磁盘格式簇记录定长)。
//!
//! 布局(全部小端,手写零外部依赖):
//! ```text
//! header(16B): magic "RXGB" | version u32 | section_count u32 | reserved u32(0)
//! dir(×16B):   kind u32 | offset u32 | length u32(字节) | reserved u32
//! sections(kind 升序,起始 4B 对齐,尾部 0 填充):
//!   1 CLUSTER_RECORDS:  count u32 + count × 64B(字段序 = 冻结契约字段序)
//!   2 VERTICES:         count u32 + count × 12B(f32×3)
//!   3 TRIANGLE_INDICES: count u32 + count × u8(3/三角形)
//!   4 DAG_NODES:        count u32 + count × 16B
//!   5 DAG_CHILDREN:     count u32 + count × u32
//!   6 LEVELS:           count u32 + count × 12B
//! ```

use rurix_render::graph::types::ClusterRecord;

use crate::dag::{ClusterDag, DagLevel, DagNode};

pub const RXGB_MAGIC: [u8; 4] = *b"RXGB";
pub const RXGB_VERSION: u32 = 1;

const SECT_RECORDS: u32 = 1;
const SECT_VERTICES: u32 = 2;
const SECT_TRI_INDICES: u32 = 3;
const SECT_DAG_NODES: u32 = 4;
const SECT_DAG_CHILDREN: u32 = 5;
const SECT_LEVELS: u32 = 6;
const SECTION_COUNT: u32 = 6;

/// RXGB 解析错误(离线工具,错误即失败不静默)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RxgbError {
    BadMagic,
    UnsupportedVersion(u32),
    /// 段目录越界/重叠/长度非元素整数倍。
    BadDirectory(&'static str),
    /// 字节流在预期位置前结束。
    Truncated(&'static str),
    /// 段内计数与数据段尺寸不一致。
    Inconsistent(&'static str),
}

impl std::fmt::Display for RxgbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RxgbError::BadMagic => write!(f, "RXGB 魔数不符"),
            RxgbError::UnsupportedVersion(v) => write!(f, "RXGB 版本不支持:{v}"),
            RxgbError::BadDirectory(s) => write!(f, "RXGB 段目录损坏:{s}"),
            RxgbError::Truncated(s) => write!(f, "RXGB 字节流截断:{s}"),
            RxgbError::Inconsistent(s) => write!(f, "RXGB 段计数不一致:{s}"),
        }
    }
}

impl std::error::Error for RxgbError {}

/// DAG → RXGB 字节序列(确定性:同输入逐字节同输出,roundtrip 单测锚定)。
pub fn write_dag(dag: &ClusterDag) -> Vec<u8> {
    // 段体先行(目录需要偏移/长度)。
    let mut bodies: Vec<(u32, Vec<u8>)> = Vec::with_capacity(SECTION_COUNT as usize);

    let mut b = Vec::with_capacity(4 + dag.records.len() * 64);
    put_u32(&mut b, dag.records.len() as u32);
    for r in &dag.records {
        put_record(&mut b, r);
    }
    bodies.push((SECT_RECORDS, b));

    let mut b = Vec::with_capacity(4 + dag.vertices.len() * 12);
    put_u32(&mut b, dag.vertices.len() as u32);
    for v in &dag.vertices {
        for &x in v {
            put_f32(&mut b, x);
        }
    }
    bodies.push((SECT_VERTICES, b));

    let mut b = Vec::with_capacity(4 + dag.triangle_indices.len());
    put_u32(&mut b, dag.triangle_indices.len() as u32);
    b.extend_from_slice(&dag.triangle_indices);
    bodies.push((SECT_TRI_INDICES, b));

    let mut b = Vec::with_capacity(4 + dag.nodes.len() * 16);
    put_u32(&mut b, dag.nodes.len() as u32);
    for n in &dag.nodes {
        put_u32(&mut b, n.first_child);
        put_u32(&mut b, n.child_count);
        put_u32(&mut b, n.level);
        put_u32(&mut b, n.group);
    }
    bodies.push((SECT_DAG_NODES, b));

    let mut b = Vec::with_capacity(4 + dag.children.len() * 4);
    put_u32(&mut b, dag.children.len() as u32);
    for &c in &dag.children {
        put_u32(&mut b, c);
    }
    bodies.push((SECT_DAG_CHILDREN, b));

    let mut b = Vec::with_capacity(4 + dag.levels.len() * 12);
    put_u32(&mut b, dag.levels.len() as u32);
    for l in &dag.levels {
        put_u32(&mut b, l.record_start);
        put_u32(&mut b, l.record_count);
        put_u32(&mut b, l.triangle_count);
    }
    bodies.push((SECT_LEVELS, b));

    let header_len = 16 + SECTION_COUNT as usize * 16;
    let mut out =
        Vec::with_capacity(header_len + bodies.iter().map(|(_, b)| b.len() + 3).sum::<usize>());
    out.extend_from_slice(&RXGB_MAGIC);
    put_u32(&mut out, RXGB_VERSION);
    put_u32(&mut out, SECTION_COUNT);
    put_u32(&mut out, 0); // reserved
    let mut cursor = header_len as u32;
    let mut dir = Vec::with_capacity(SECTION_COUNT as usize * 16);
    for (kind, body) in &bodies {
        let aligned = pad4(cursor);
        put_u32(&mut dir, *kind);
        put_u32(&mut dir, aligned);
        put_u32(&mut dir, body.len() as u32);
        put_u32(&mut dir, 0); // reserved
        cursor = aligned + body.len() as u32;
    }
    out.extend_from_slice(&dir);
    for (_, body) in &bodies {
        while out.len() % 4 != 0 {
            out.push(0);
        }
        out.extend_from_slice(body);
    }
    out
}

/// RXGB 字节序列 → DAG(边界全校验;计数与数据段一致性校验)。
pub fn read_dag(bytes: &[u8]) -> Result<ClusterDag, RxgbError> {
    let mut cur = Cursor { bytes, pos: 0 };
    let magic = cur.take(4).ok_or(RxgbError::Truncated("header"))?;
    if magic != RXGB_MAGIC {
        return Err(RxgbError::BadMagic);
    }
    let version = cur.u32().ok_or(RxgbError::Truncated("version"))?;
    if version != RXGB_VERSION {
        return Err(RxgbError::UnsupportedVersion(version));
    }
    let section_count = cur.u32().ok_or(RxgbError::Truncated("section_count"))?;
    if section_count != SECTION_COUNT {
        return Err(RxgbError::BadDirectory("section_count"));
    }
    let _reserved = cur.u32().ok_or(RxgbError::Truncated("reserved"))?;
    let mut dir: Vec<(u32, u32, u32)> = Vec::with_capacity(SECTION_COUNT as usize);
    for _ in 0..section_count {
        let kind = cur.u32().ok_or(RxgbError::Truncated("dir.kind"))?;
        let offset = cur.u32().ok_or(RxgbError::Truncated("dir.offset"))?;
        let length = cur.u32().ok_or(RxgbError::Truncated("dir.length"))?;
        let _ = cur.u32().ok_or(RxgbError::Truncated("dir.reserved"))?;
        let end = offset as usize + length as usize;
        if end > bytes.len() || offset as usize > bytes.len() {
            return Err(RxgbError::BadDirectory("section 越界"));
        }
        dir.push((kind, offset, length));
    }
    for want in [
        SECT_RECORDS,
        SECT_VERTICES,
        SECT_TRI_INDICES,
        SECT_DAG_NODES,
        SECT_DAG_CHILDREN,
        SECT_LEVELS,
    ] {
        if !dir.iter().any(|&(k, _, _)| k == want) {
            return Err(RxgbError::BadDirectory("缺段"));
        }
    }

    let sb = section_slice(&dir, bytes, SECT_RECORDS)?;
    let mut c = Cursor { bytes: sb, pos: 0 };
    let n = c.u32().ok_or(RxgbError::Truncated("records.count"))? as usize;
    if n > (sb.len() - 4) / 64 {
        return Err(RxgbError::Inconsistent("records 计数越出段长"));
    }
    let mut records = Vec::with_capacity(n);
    for _ in 0..n {
        records.push(take_record(&mut c)?);
    }
    let mut dag = ClusterDag {
        records,
        ..ClusterDag::default()
    };

    let sb = section_slice(&dir, bytes, SECT_VERTICES)?;
    let mut c = Cursor { bytes: sb, pos: 0 };
    let n = c.u32().ok_or(RxgbError::Truncated("vertices.count"))? as usize;
    if n > (sb.len() - 4) / 12 {
        return Err(RxgbError::Inconsistent("vertices 计数越出段长"));
    }
    dag.vertices = Vec::with_capacity(n);
    for _ in 0..n {
        let (x, y, z) = (c.f32(), c.f32(), c.f32());
        match (x, y, z) {
            (Some(x), Some(y), Some(z)) => dag.vertices.push([x, y, z]),
            _ => return Err(RxgbError::Truncated("vertices.body")),
        }
    }

    let sb = section_slice(&dir, bytes, SECT_TRI_INDICES)?;
    let mut c = Cursor { bytes: sb, pos: 0 };
    let n = c.u32().ok_or(RxgbError::Truncated("tri_indices.count"))? as usize;
    dag.triangle_indices = c
        .take(n)
        .ok_or(RxgbError::Truncated("tri_indices.body"))?
        .to_vec();

    let sb = section_slice(&dir, bytes, SECT_DAG_NODES)?;
    let mut c = Cursor { bytes: sb, pos: 0 };
    let n = c.u32().ok_or(RxgbError::Truncated("nodes.count"))? as usize;
    if n > (sb.len() - 4) / 16 {
        return Err(RxgbError::Inconsistent("nodes 计数越出段长"));
    }
    dag.nodes = Vec::with_capacity(n);
    for _ in 0..n {
        let (fc, cc, lv, gr) = (c.u32(), c.u32(), c.u32(), c.u32());
        match (fc, cc, lv, gr) {
            (Some(first_child), Some(child_count), Some(level), Some(group)) => {
                dag.nodes.push(DagNode {
                    first_child,
                    child_count,
                    level,
                    group,
                });
            }
            _ => return Err(RxgbError::Truncated("nodes.body")),
        }
    }

    let sb = section_slice(&dir, bytes, SECT_DAG_CHILDREN)?;
    let mut c = Cursor { bytes: sb, pos: 0 };
    let n = c.u32().ok_or(RxgbError::Truncated("children.count"))? as usize;
    if n > (sb.len() - 4) / 4 {
        return Err(RxgbError::Inconsistent("children 计数越出段长"));
    }
    dag.children = Vec::with_capacity(n);
    for _ in 0..n {
        dag.children
            .push(c.u32().ok_or(RxgbError::Truncated("children.body"))?);
    }

    let sb = section_slice(&dir, bytes, SECT_LEVELS)?;
    let mut c = Cursor { bytes: sb, pos: 0 };
    let n = c.u32().ok_or(RxgbError::Truncated("levels.count"))? as usize;
    if n > (sb.len() - 4) / 12 {
        return Err(RxgbError::Inconsistent("levels 计数越出段长"));
    }
    dag.levels = Vec::with_capacity(n);
    for _ in 0..n {
        let (s, c1, t) = (c.u32(), c.u32(), c.u32());
        match (s, c1, t) {
            (Some(record_start), Some(record_count), Some(triangle_count)) => {
                dag.levels.push(DagLevel {
                    record_start,
                    record_count,
                    triangle_count,
                });
            }
            _ => return Err(RxgbError::Truncated("levels.body")),
        }
    }

    // 一致性:节点数 = 记录数;记录偏移/计数落在数据段内。
    if dag.nodes.len() != dag.records.len() {
        return Err(RxgbError::Inconsistent("nodes != records"));
    }
    for r in &dag.records {
        let v_end = r.vertex_offset as usize + r.vertex_count as usize;
        let t_end = r.triangle_offset as usize + 3 * r.triangle_count as usize;
        if v_end > dag.vertices.len() || t_end > dag.triangle_indices.len() {
            return Err(RxgbError::Inconsistent("记录偏移越出数据段"));
        }
    }
    Ok(dag)
}

/// 按 kind 取段体(段目录已校验越界)。
fn section_slice<'a>(
    dir: &[(u32, u32, u32)],
    bytes: &'a [u8],
    kind: u32,
) -> Result<&'a [u8], RxgbError> {
    let &(_, off, len) = dir
        .iter()
        .find(|&&(k, _, _)| k == kind)
        .ok_or(RxgbError::BadDirectory("缺段"))?;
    Ok(&bytes[off as usize..(off + len) as usize])
}

/// 字段序 = 冻结契约 `ClusterRecord` 字段序(64B 定长;手写 LE 不依赖布局)。
fn put_record(b: &mut Vec<u8>, r: &ClusterRecord) {
    let start = b.len();
    for &x in &r.center {
        put_f32(b, x);
    }
    put_f32(b, r.radius);
    for &x in &r.cone_axis {
        put_f32(b, x);
    }
    put_f32(b, r.cone_cutoff);
    put_f32(b, r.error);
    put_f32(b, r.parent_error);
    put_u32(b, r.vertex_offset);
    put_u32(b, r.triangle_offset);
    put_u32(b, r.vertex_count);
    put_u32(b, r.triangle_count);
    put_u32(b, r.page_id);
    put_u32(b, r.reserved);
    debug_assert_eq!(b.len() - start, 64);
}

fn take_record(c: &mut Cursor<'_>) -> Result<ClusterRecord, RxgbError> {
    let f3 = |c: &mut Cursor<'_>| -> Result<[f32; 3], RxgbError> {
        let (x, y, z) = (c.f32(), c.f32(), c.f32());
        match (x, y, z) {
            (Some(x), Some(y), Some(z)) => Ok([x, y, z]),
            _ => Err(RxgbError::Truncated("record.f32x3")),
        }
    };
    let center = f3(c)?;
    let radius = c.f32().ok_or(RxgbError::Truncated("record.radius"))?;
    let cone_axis = f3(c)?;
    let cone_cutoff = c.f32().ok_or(RxgbError::Truncated("record.cone_cutoff"))?;
    let error = c.f32().ok_or(RxgbError::Truncated("record.error"))?;
    let parent_error = c.f32().ok_or(RxgbError::Truncated("record.parent_error"))?;
    let u = |c: &mut Cursor<'_>| c.u32().ok_or(RxgbError::Truncated("record.u32"));
    Ok(ClusterRecord {
        center,
        radius,
        cone_axis,
        cone_cutoff,
        error,
        parent_error,
        vertex_offset: u(c)?,
        triangle_offset: u(c)?,
        vertex_count: u(c)?,
        triangle_count: u(c)?,
        page_id: u(c)?,
        reserved: u(c)?,
    })
}

const fn pad4(v: u32) -> u32 {
    v.div_ceil(4) * 4
}

fn put_u32(b: &mut Vec<u8>, v: u32) {
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

    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }

    fn f32(&mut self) -> Option<f32> {
        Some(f32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::build_dag;
    use crate::mesh::TriMesh;

    fn bits_eq(a: &ClusterDag, b: &ClusterDag) -> bool {
        let rec_eq = a.records.iter().zip(&b.records).all(|(x, y)| {
            x.center.map(f32::to_bits) == y.center.map(f32::to_bits)
                && x.radius.to_bits() == y.radius.to_bits()
                && x.cone_axis.map(f32::to_bits) == y.cone_axis.map(f32::to_bits)
                && x.cone_cutoff.to_bits() == y.cone_cutoff.to_bits()
                && x.error.to_bits() == y.error.to_bits()
                && x.parent_error.to_bits() == y.parent_error.to_bits()
                && x.vertex_offset == y.vertex_offset
                && x.triangle_offset == y.triangle_offset
                && x.vertex_count == y.vertex_count
                && x.triangle_count == y.triangle_count
                && x.page_id == y.page_id
                && x.reserved == y.reserved
        });
        rec_eq
            && a.records.len() == b.records.len()
            && a.nodes == b.nodes
            && a.children == b.children
            && a.vertices
                .iter()
                .map(|v| v.map(f32::to_bits))
                .eq(b.vertices.iter().map(|v| v.map(f32::to_bits)))
            && a.triangle_indices == b.triangle_indices
            && a.levels == b.levels
    }

    #[test]
    fn roundtrip_byte_equal() {
        let dag = build_dag(&TriMesh::uv_sphere(1.0, 12, 12));
        let bytes = write_dag(&dag);
        assert_eq!(&bytes[0..4], b"RXGB");
        let back = read_dag(&bytes).expect("解析失败");
        assert!(bits_eq(&dag, &back), "结构 roundtrip 不一致");
        let bytes2 = write_dag(&back);
        assert_eq!(bytes, bytes2, "字节 roundtrip 不逐字节相等");
    }

    #[test]
    fn rejects_bad_magic_and_truncation() {
        let dag = build_dag(&TriMesh::cube(1.0));
        let bytes = write_dag(&dag);
        let mut bad = bytes.clone();
        bad[0] = b'X';
        assert_eq!(read_dag(&bad), Err(RxgbError::BadMagic));
        let cut = &bytes[..bytes.len() - 7];
        assert!(matches!(
            read_dag(cut),
            Err(RxgbError::BadDirectory(_)) | Err(RxgbError::Truncated(_))
        ));
        assert!(matches!(
            read_dag(&bytes[..8]),
            Err(RxgbError::Truncated(_))
        ));
    }
}
