//! RXPL major=2 整数域展开流与 digest(RXS-0342 字段序体例扩至 v2;RXS-0344)。
//!
//! 与 v1 展开流同构:全 `u32` 整数域(量化参数 f32 按 `to_bits` 搬运,kernel 内零浮点
//! 运算)。v2 追加字段序(LE `u32` 序列,接 v1 展开流后):
//!
//! ```text
//! for c in 0..cluster_count:
//!   radius_bits, center_bits×3, cone_axis_bits×3, cone_cutoff_bits,
//!   error_bits, parent_error_bits          # 簇误差/包围球(10 字)
//!   max_influences, bone_count, bound_inflation_bits
//!   for b in 0..bone_count: bone_index          # 自骨骼索引集段按序取
//!   aabb_min_bits×3, aabb_max_bits×3
//! ```
//!
//! device kernel(`geom_page_decode_v2.rx`)消费 **RXPL major=2 页字节** 直接展开;
//! host 侧经 [`expand_logical_page_v2`] 自解码后的结构展开,两侧 digest 必须逐位等。

use rurix_pkg::sha256;

use crate::logical_v2::LogicalPageV2;

/// v2 展开流(RXS-0342 v1 字段序 + 上列 v2 追加序)。
///
/// **v2 追加序(冻结)**:center_bits×3, radius_bits, cone_axis_bits×3,
/// cone_cutoff_bits, error_bits, parent_error_bits(包围球 = center+radius,
/// 共 10 字),再三字段蒙皮(max_influences, bone_count, bound_inflation_bits)+
/// 变长骨骼索引 + CLAS AABB×6。
pub fn expand_logical_page_v2(page: &LogicalPageV2) -> Vec<u8> {
    let base = &page.base;
    let mut out = Vec::new();
    put_u32(&mut out, base.clusters.len() as u32);
    for (c, e) in base.clusters.iter().zip(page.ext.iter()) {
        put_u32(&mut out, c.cluster_id);
        put_u32(&mut out, c.qx as u32);
        put_u32(&mut out, c.qy as u32);
        put_u32(&mut out, c.qz as u32);
        put_u32(&mut out, c.triangle_count);
        let tbase = c.triangle_offset as usize;
        for t in 0..c.triangle_count as usize {
            let o = tbase + t * 3;
            let i0 = *base.indices.get(o).unwrap_or(&0) as u32;
            let i1 = *base.indices.get(o + 1).unwrap_or(&0) as u32;
            let i2 = *base.indices.get(o + 2).unwrap_or(&0) as u32;
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
        // 簇误差/包围球(RXS-0344 §1「簇误差/包围球段」展开面;center 走量化 qx/qy/qz 不重发)。
        put_u32(&mut out, c.center[0].to_bits());
        put_u32(&mut out, c.center[1].to_bits());
        put_u32(&mut out, c.center[2].to_bits());
        put_u32(&mut out, c.radius.to_bits());
        put_u32(&mut out, c.cone_axis[0].to_bits());
        put_u32(&mut out, c.cone_axis[1].to_bits());
        put_u32(&mut out, c.cone_axis[2].to_bits());
        put_u32(&mut out, c.cone_cutoff.to_bits());
        put_u32(&mut out, c.error.to_bits());
        put_u32(&mut out, c.parent_error.to_bits());
        put_u32(&mut out, e.max_influences);
        put_u32(&mut out, e.bone_indices.len() as u32);
        put_u32(&mut out, e.bound_inflation.to_bits());
        for &b in &e.bone_indices {
            put_u32(&mut out, b);
        }
        put_u32(&mut out, e.aabb_min[0].to_bits());
        put_u32(&mut out, e.aabb_min[1].to_bits());
        put_u32(&mut out, e.aabb_min[2].to_bits());
        put_u32(&mut out, e.aabb_max[0].to_bits());
        put_u32(&mut out, e.aabb_max[1].to_bits());
        put_u32(&mut out, e.aabb_max[2].to_bits());
    }
    for &b in &base.bounds {
        put_u32(&mut out, b.to_bits());
    }
    out
}

/// SHA-256(v2 展开流)。
pub fn expanded_digest_v2(page: &LogicalPageV2) -> [u8; 32] {
    sha256::digest(&expand_logical_page_v2(page))
}

/// v2 展开流 `u32` 元素个数(device out buffer 尺寸)。
pub fn expand_u32_count_v2(page: &LogicalPageV2) -> usize {
    let mut n = 1; // cluster_count
    for (c, e) in page.base.clusters.iter().zip(page.ext.iter()) {
        n += 4; // id + qx + qy + qz
        n += 1; // triangle_count
        n += c.triangle_count as usize * 3;
        n += 6; // meta fields
        n += 10; // center×3 + radius + cone×3 + cone_cutoff + error + parent_error
        n += 3; // max_influences/bone_count/bound_inflation
        n += e.bone_indices.len();
        n += 6; // aabb
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
    use crate::logical_v2::V2ClusterExt;

    fn sample() -> LogicalPageV2 {
        let bounds = [-1.0, -1.0, -1.0, 1.0, 1.0, 1.0];
        let (qx, qy, qz) = quantize_center([0.0, 0.0, 0.0], bounds);
        LogicalPageV2 {
            base: LogicalPage {
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
            },
            ext: vec![V2ClusterExt {
                max_influences: 2,
                bone_indices: vec![5, 7],
                bound_inflation: 0.5,
                aabb_min: [-1.0, -1.0, -1.0],
                aabb_max: [1.0, 1.0, 1.0],
            }],
        }
    }

    //@ spec: RXS-0342
    //@ spec: RXS-0344
    #[test]
    fn digest_stable_and_count_matches() {
        let page = sample();
        let d1 = expanded_digest_v2(&page);
        let d2 = expanded_digest_v2(&page);
        assert_eq!(d1, d2);
        assert_eq!(
            expand_logical_page_v2(&page).len(),
            expand_u32_count_v2(&page) * 4
        );
        // 骨骼索引集进入展开流:改一枚骨骼 id 必翻 digest。
        let mut other = sample();
        other.ext[0].bone_indices[1] = 8;
        assert_ne!(expanded_digest_v2(&other), d1);
    }
}
