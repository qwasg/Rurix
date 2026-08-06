//! 整数域展开流与 digest（RXS-0342）。

use crate::memory::MemoryPage;
use rurix_pkg::sha256;

/// 按冻结字段序展开为 `u32` LE 字节流。
pub fn expand_memory_page(page: &MemoryPage) -> Vec<u8> {
    let mut out = Vec::new();
    put_u32(&mut out, page.clusters.len() as u32);
    for c in &page.clusters {
        put_u32(&mut out, c.cluster_id);
        put_u32(&mut out, c.qx as u32);
        put_u32(&mut out, c.qy as u32);
        put_u32(&mut out, c.qz as u32);
        put_u32(&mut out, c.triangle_count);
        let base = c.triangle_offset as usize;
        for t in 0..c.triangle_count as usize {
            let o = base + t * 3;
            let i0 = *page.indices.get(o).unwrap_or(&0) as u32;
            let i1 = *page.indices.get(o + 1).unwrap_or(&0) as u32;
            let i2 = *page.indices.get(o + 2).unwrap_or(&0) as u32;
            put_u32(&mut out, i0);
            put_u32(&mut out, i1);
            put_u32(&mut out, i2);
        }
        put_u32(&mut out, c.vertex_offset);
        put_u32(&mut out, c.triangle_offset);
        put_u32(&mut out, c.vertex_count);
        put_u32(&mut out, c.triangle_count);
        put_u32(&mut out, c.level);
        put_u32(&mut out, c.group);
    }
    for &b in &page.bounds {
        put_u32(&mut out, b.to_bits());
    }
    out
}

/// SHA-256(展开流)。
pub fn expanded_digest(page: &MemoryPage) -> [u8; 32] {
    sha256::digest(&expand_memory_page(page))
}

/// 估算展开流 `u32` 元素个数（供 device out buffer 尺寸）。
pub fn expand_u32_count(page: &MemoryPage) -> usize {
    let mut n = 1; // cluster_count
    for c in &page.clusters {
        n += 4; // id + qx + qy + qz
        n += 1; // triangle_count
        n += c.triangle_count as usize * 3;
        n += 6; // meta fields
    }
    n += 6; // bounds bits
    n
}

fn put_u32(b: &mut Vec<u8>, v: u32) {
    b.extend_from_slice(&v.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logical::{FLAG_ROOT, LogicalPage, PageClusterRecord, quantize_center};
    use crate::memory::from_logical;

    //@ spec: RXS-0342
    #[test]
    fn digest_stable() {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let (qx, qy, qz) = quantize_center([0.0, 0.0, 0.0], bounds);
        let page = from_logical(&LogicalPage {
            page_id: 0,
            flags: FLAG_ROOT,
            lod_level_min: 0,
            lod_level_max: 0,
            bounds,
            clusters: vec![PageClusterRecord {
                cluster_id: 3,
                qx,
                qy,
                qz,
                center: [0.0, 0.0, 0.0],
                radius: 1.0,
                cone_axis: [0.0, 1.0, 0.0],
                cone_cutoff: 0.0,
                error: 0.0,
                parent_error: 1.0,
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
        });
        let d1 = expanded_digest(&page);
        let d2 = expanded_digest(&page);
        assert_eq!(d1, d2);
        assert_eq!(expand_memory_page(&page).len(), expand_u32_count(&page) * 4);
    }
}
