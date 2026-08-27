//! ClusterDagV2 → RXPL major=2 逻辑页确定性装箱(RXS-0344 消费面;G9.2 M91)。
//!
//! 与 v1 [`crate::geom_build::pack_cluster_dag`] 同算法(层级/stable id 序贪心
//! 装箱、依赖/边集同语义),差异:v2 扩展段(蒙皮元数据 + CLAS 输入段,
//! RXS-0345 schema)随每簇回填,且**装箱估算 v2 段感知**——v1 估算只计 v1
//! 段字节,v2 页编码另含 skin_hdr(12B/簇)+ 骨骼索引(4B/骨)+ CLAS(32B/簇)
//! + 4B 对齐填充 + header 160B,近 128KB 边界的页在 v1 估算下会实际超页
//! (P4-1 bistro 实证);v2 装箱按精确 v2 编码上界封页,`check_v2_pages_
//! within_contract` 机核兜底。v1 打包路径(geom_build.rs)0-byte 不动;
//! v1/v2 页成员在 v2 段开销不触边界的 DAG 上逐字节同构(单测锚定)。

use rurix_geom_build::{ClusterDagV2, ClusterSkinMeta};
use rurix_geom_pages::logical::RECORD_SIZE;
use rurix_geom_pages::logical_v2::{
    CLAS_RECORD_SIZE, HEADER_SIZE_V2, LogicalPageV2, SKIN_RECORD_SIZE, V2ClusterExt,
};
use rurix_geom_pages::{LogicalPage, STREAM_PAGE_SIZE, encode_logical_page_v2};

use crate::geom_build::PackError;

/// v2 感知编码长度估算(与 v1 `PageBuilder::estimate_encoded_len` 同构 +
/// v2 段增量精确计入;依赖/边用与 v1 相同的封页前占位上界)。
///
/// 返回 (整页编码字节, v1 段含填充字节)——**双约束**:整页 ≤ STREAM_PAGE_SIZE
/// ∧ v1 段 ≤ u16::MAX(RXPL v2 冻结布局 `v1_section_bytes:u16` 的结构性
/// 上限;bistro 实证单页 v1 段可破 64KB,P4-1 装箱必须同守两界)。
fn estimate_encoded_len_v2(
    dag: &rurix_geom_build::ClusterDag,
    skin: &[ClusterSkinMeta],
    cluster_ids: &[u32],
    all_edges: &[(u32, u32)],
) -> (usize, usize) {
    let mut vert = 0usize;
    let mut idx = 0usize;
    let mut bones = 0usize;
    for &id in cluster_ids {
        let r = &dag.records[id as usize];
        vert += r.vertex_count as usize;
        idx += (r.triangle_count as usize) * 3;
        bones += skin[id as usize]
            .bone_indices
            .as_ref()
            .map_or(0, |b| b.len());
    }
    let in_page: std::collections::HashSet<u32> = cluster_ids.iter().copied().collect();
    let mut link_n = 0usize;
    let mut dep_n = 0usize;
    let mut dep_seen = std::collections::HashSet::new();
    for &(p, c) in all_edges {
        if in_page.contains(&p) || in_page.contains(&c) {
            link_n += 1;
        }
        if in_page.contains(&c) && !in_page.contains(&p) && dep_seen.insert(p) {
            dep_n += 1;
        }
    }
    let n = cluster_ids.len();
    let v1_section = n * RECORD_SIZE as usize + vert * 12 + idx + dep_n * 8 + link_n * 8;
    let pad = (4 - v1_section % 4) % 4;
    let v1_padded = v1_section + pad;
    let total = HEADER_SIZE_V2 as usize
        + v1_padded
        + n * SKIN_RECORD_SIZE
        + bones * 4
        + n * CLAS_RECORD_SIZE;
    (total, v1_padded)
}

/// 封页判据(双约束同守)。
fn v2_page_fits(est: (usize, usize)) -> bool {
    est.0 <= STREAM_PAGE_SIZE as usize && est.1 <= u16::MAX as usize
}

/// `ClusterDagV2 → Vec<LogicalPageV2>`(packing_algo_id=1 沿用;RXS-0344 §1)。
///
/// 与 v1 同算法(父映射/全边集/层级+stable id 升序贪心封页/跨页依赖与边
/// 集同语义),估算面 v2 段感知;页成员确定后按 v1 段面编码(v1 记录面
/// 逐字节同构)再回填 v2 扩展;骨骼资产缺三字段任一面在 builder 期已被
/// `build_asset_dag` typed `Err` 拒录,本层不再重演。
pub fn pack_cluster_dag_v2(dag: &ClusterDagV2) -> Result<Vec<LogicalPageV2>, PackError> {
    let base = &dag.base;
    if base.records.is_empty() {
        return Err(PackError::EmptyDag);
    }
    // 父映射与全边集(v1 同律)。
    let mut all_edges: Vec<(u32, u32)> = Vec::new();
    for parent in 0..base.records.len() as u32 {
        for &child in base.children_of(parent) {
            all_edges.push((parent, child));
        }
    }
    all_edges.sort_unstable();
    all_edges.dedup();
    // 层级升序、level 内 stable id 升序(v1 同律)。
    let mut order: Vec<u32> = (0..base.records.len() as u32).collect();
    order.sort_by_key(|&id| (base.nodes[id as usize].level, id));
    let top: std::collections::HashSet<u32> = base.top_level_ids().collect();

    // v2 段感知贪心封页(v1 同算法,估算面换装;双约束同守)。
    let mut pages: Vec<Vec<u32>> = Vec::new();
    let mut current: Vec<u32> = Vec::new();
    for &id in &order {
        if current.is_empty() {
            current.push(id);
            if !v2_page_fits(estimate_encoded_len_v2(base, &dag.skin, &current, &all_edges)) {
                return Err(PackError::ClusterExceedsPage(id));
            }
            continue;
        }
        let mut trial = current.clone();
        trial.push(id);
        if !v2_page_fits(estimate_encoded_len_v2(base, &dag.skin, &trial, &all_edges)) {
            pages.push(current);
            current = vec![id];
            if !v2_page_fits(estimate_encoded_len_v2(base, &dag.skin, &current, &all_edges)) {
                return Err(PackError::ClusterExceedsPage(id));
            }
        } else {
            current = trial;
        }
    }
    if !current.is_empty() {
        pages.push(current);
    }

    // 簇 → page_id + 依赖/边集(v1 同律)。
    let mut cluster_page = vec![u64::MAX; base.records.len()];
    for (pi, ids) in pages.iter().enumerate() {
        for &cid in ids {
            cluster_page[cid as usize] = pi as u64;
        }
    }
    let mut out = Vec::with_capacity(pages.len());
    for (pi, ids) in pages.into_iter().enumerate() {
        let is_root = ids.iter().any(|cid| top.contains(cid));
        let in_page: std::collections::HashSet<u32> = ids.iter().copied().collect();
        let mut deps: Vec<u64> = Vec::new();
        let mut links: Vec<(u32, u32)> = Vec::new();
        for &(p, c) in &all_edges {
            if in_page.contains(&p) || in_page.contains(&c) {
                links.push((p, c));
            }
            if in_page.contains(&c) && !in_page.contains(&p) {
                let pp = cluster_page[p as usize];
                if pp != u64::MAX {
                    deps.push(pp);
                }
            }
        }
        deps.sort_unstable();
        deps.dedup();
        links.sort_unstable();
        links.dedup();
        // v1 段面编码(与纯 v1 打包逐字节同构——v1 记录面编码律单源)。
        let v1_page = finish_v1_page(base, pi as u64, is_root, &ids, deps, links);
        debug_assert!(v1_page.encoded_len() <= STREAM_PAGE_SIZE as usize);
        let mut ext = Vec::with_capacity(ids.len());
        for &cid in &ids {
            let skin: &ClusterSkinMeta = &dag.skin[cid as usize];
            let clas = &dag.clas[cid as usize];
            ext.push(V2ClusterExt {
                max_influences: skin.max_influences,
                bone_indices: skin.bone_indices.clone().unwrap_or_default(),
                bound_inflation: skin.bound_inflation.unwrap_or(0.0),
                aabb_min: clas.aabb_min,
                aabb_max: clas.aabb_max,
            });
        }
        out.push(LogicalPageV2 { base: v1_page, ext });
    }
    Ok(out)
}

/// 页成员 → v1 逻辑页(与 v1 `PageBuilder::finish` 逐字同构:bounds 先行
/// 聚合(含非有限回退 [0;6])→ 记录逐簇(v/t 段拼接序、量化、bounds、
/// lod 范围、FLAGS)同序同律——v1 段面编码律单源互证)。
fn finish_v1_page(
    dag: &rurix_geom_build::ClusterDag,
    page_id: u64,
    is_root: bool,
    ids: &[u32],
    deps: Vec<u64>,
    links: Vec<(u32, u32)>,
) -> LogicalPage {
    use rurix_geom_pages::{FLAG_ROOT, PageClusterRecord, quantize_center};
    let mut bounds = [
        f32::INFINITY,
        f32::INFINITY,
        f32::INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
        f32::NEG_INFINITY,
    ];
    for &id in ids {
        let r = &dag.records[id as usize];
        for k in 0..3 {
            let lo = r.center[k] - r.radius;
            let hi = r.center[k] + r.radius;
            bounds[k] = bounds[k].min(lo);
            bounds[k + 3] = bounds[k + 3].max(hi);
        }
    }
    if !bounds[0].is_finite() {
        bounds = [0.0; 6];
    }

    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let mut clusters = Vec::with_capacity(ids.len());
    let mut lod_min = u16::MAX;
    let mut lod_max = 0u16;
    for &id in ids {
        let r = &dag.records[id as usize];
        let n = &dag.nodes[id as usize];
        lod_min = lod_min.min(n.level as u16);
        lod_max = lod_max.max(n.level as u16);
        let v_off = vertices.len() as u32;
        let t_off = indices.len() as u32;
        vertices.extend_from_slice(dag.cluster_vertices(id));
        for t in 0..r.triangle_count {
            indices.extend_from_slice(&dag.cluster_triangle(id, t));
        }
        let (qx, qy, qz) = quantize_center(r.center, bounds);
        clusters.push(PageClusterRecord {
            cluster_id: id,
            qx,
            qy,
            qz,
            center: r.center,
            radius: r.radius,
            cone_axis: r.cone_axis,
            cone_cutoff: r.cone_cutoff,
            error: r.error,
            parent_error: r.parent_error,
            vertex_offset: v_off,
            triangle_offset: t_off,
            vertex_count: r.vertex_count,
            triangle_count: r.triangle_count,
            level: n.level,
            group: n.group,
        });
    }
    if lod_min == u16::MAX {
        lod_min = 0;
    }
    LogicalPage {
        page_id,
        flags: if is_root { FLAG_ROOT } else { 0 },
        lod_level_min: lod_min,
        lod_level_max: lod_max,
        bounds,
        clusters,
        vertices,
        indices,
        dependency_page_ids: deps,
        dag_links: links,
    }
}

/// v2 全页串接(双构建比对用)。
pub fn concatenate_pages_v2(pages: &[LogicalPageV2]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in pages {
        out.extend_from_slice(&encode_logical_page_v2(p));
    }
    out
}

/// v2 页尺寸契约(沿 v1 STREAM_PAGE_SIZE;v2 段感知估算封页后由本函数
/// 机器核验兜底,超出即 `PackError::ClusterExceedsPage`)。
pub fn check_v2_pages_within_contract(pages: &[LogicalPageV2]) -> Result<(), PackError> {
    for p in pages {
        if encode_logical_page_v2(p).len() > STREAM_PAGE_SIZE as usize {
            return Err(PackError::ClusterExceedsPage(p.base.page_id as u32));
        }
    }
    Ok(())
}

/// 自 v1 逻辑页 + v2 平行扩展装配 v2 页(rxcook/探针直通路径用)。
pub fn logical_v2_from_parts(base: LogicalPage, ext: Vec<V2ClusterExt>) -> LogicalPageV2 {
    debug_assert_eq!(base.clusters.len(), ext.len());
    LogicalPageV2 { base, ext }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom_build::pack_cluster_dag;
    use rurix_geom_build::{DagAsset, TriMesh, build_asset_dag};
    use rurix_geom_pages::decode_logical_page_v2;

    fn fixture() -> ClusterDagV2 {
        build_asset_dag(&DagAsset::static_mesh(TriMesh::uv_sphere(1.0, 24, 24))).unwrap()
    }

    //@ spec: RXS-0344
    #[test]
    fn v2_pack_roundtrip_byte_equal() {
        let dag = fixture();
        let a = pack_cluster_dag_v2(&dag).unwrap();
        let b = pack_cluster_dag_v2(&dag).unwrap();
        assert_eq!(concatenate_pages_v2(&a), concatenate_pages_v2(&b));
        check_v2_pages_within_contract(&a).unwrap();
        for p in &a {
            let bytes = encode_logical_page_v2(p);
            let back = decode_logical_page_v2(&bytes).unwrap();
            assert_eq!(&back, p);
            assert_eq!(encode_logical_page_v2(&back), bytes);
        }
    }

    //@ spec: RXS-0344
    #[test]
    fn v2_pages_share_v1_membership() {
        // v1/v2 共存律:v2 段开销不触 128KB 边界的 DAG 上,v2 页成员/v1 段面
        // 与纯 v1 打包逐字节同构(触边界时 v2 感知装箱按契约提前封页——
        // 成员可不同,v1 段面编码律仍单源同构,由 roundtrip/契约测试锚定)。
        let dag = fixture();
        let v1 = pack_cluster_dag(&dag.base).unwrap();
        let v2 = pack_cluster_dag_v2(&dag).unwrap();
        assert_eq!(v1.len(), v2.len());
        for (a, b) in v1.iter().zip(v2.iter()) {
            assert_eq!(a, &b.base);
        }
    }

    //@ spec: RXS-0344
    #[test]
    fn v2_packing_contract_holds_on_larger_dag() {
        // 较大 DAG(多页):v2 感知装箱全页 ≤ 契约(v1 估算面在此规模可触
        // 边界——本测试为 v2 段感知封页的机器实证)。
        let dag = build_asset_dag(&DagAsset::static_mesh(TriMesh::uv_sphere(1.0, 48, 48))).unwrap();
        let pages = pack_cluster_dag_v2(&dag).unwrap();
        assert!(pages.len() >= 1);
        check_v2_pages_within_contract(&pages).unwrap();
        for p in &pages {
            let n = encode_logical_page_v2(p).len();
            assert!(n <= STREAM_PAGE_SIZE as usize, "page {} len {n}", p.base.page_id);
        }
    }
}
