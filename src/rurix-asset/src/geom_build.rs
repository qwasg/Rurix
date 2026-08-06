//! ClusterDag → 逻辑页确定性装箱 + RXGB→pages converter（RXS-0329~0331）。

use rurix_geom_build::{ClusterDag, RxgbError, read_dag};
use rurix_geom_pages::{
    FLAG_ROOT, HEADER_SIZE, LogicalPage, PageClusterRecord, RECORD_SIZE, STREAM_PAGE_SIZE,
    encode_logical_page, quantize_center,
};
use rurix_render::graph::types::STREAM_PAGE_SIZE as RENDER_STREAM_PAGE_SIZE;

/// 装箱错误。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackError {
    EmptyDag,
    ClusterExceedsPage(u32),
    Rxgb(RxgbError),
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackError::EmptyDag => write!(f, "ClusterDag 为空"),
            PackError::ClusterExceedsPage(id) => {
                write!(f, "单簇 {id} 编码超过 STREAM_PAGE_SIZE")
            }
            PackError::Rxgb(e) => write!(f, "RXGB:{e}"),
        }
    }
}

impl std::error::Error for PackError {}

impl From<RxgbError> for PackError {
    fn from(e: RxgbError) -> Self {
        PackError::Rxgb(e)
    }
}

/// 编译期/运行期锚定：本 crate 复述的页上限 = render 冻结契约单源。
const _: () = assert!(STREAM_PAGE_SIZE == RENDER_STREAM_PAGE_SIZE);

/// `ClusterDag → Vec<LogicalPage>`（packing_algo_id=1；RXS-0329）。
pub fn pack_cluster_dag(dag: &ClusterDag) -> Result<Vec<LogicalPage>, PackError> {
    if dag.records.is_empty() {
        return Err(PackError::EmptyDag);
    }

    // 父映射：child → parent（由 children 表导出）。
    let mut parent_of = vec![u32::MAX; dag.records.len()];
    let mut all_edges: Vec<(u32, u32)> = Vec::new();
    for parent in 0..dag.records.len() as u32 {
        for &child in dag.children_of(parent) {
            parent_of[child as usize] = parent;
            all_edges.push((parent, child));
        }
    }
    all_edges.sort_unstable();
    all_edges.dedup();

    // 按 level 升序、level 内 stable id 升序。
    let mut order: Vec<u32> = (0..dag.records.len() as u32).collect();
    order.sort_by_key(|&id| (dag.nodes[id as usize].level, id));

    let top: std::collections::HashSet<u32> = dag.top_level_ids().collect();

    let mut pages: Vec<PageBuilder> = Vec::new();
    let mut current = PageBuilder::new();

    for &id in &order {
        if current.cluster_ids.is_empty() {
            current.push(id, dag);
            let est = current.estimate_encoded_len(dag, &all_edges, &[]);
            if est > STREAM_PAGE_SIZE as usize {
                return Err(PackError::ClusterExceedsPage(id));
            }
            continue;
        }
        let mut trial = current.clone();
        trial.push(id, dag);
        let est = trial.estimate_encoded_len(dag, &all_edges, &[]);
        if est > STREAM_PAGE_SIZE as usize {
            pages.push(current);
            current = PageBuilder::new();
            current.push(id, dag);
            let est2 = current.estimate_encoded_len(dag, &all_edges, &[]);
            if est2 > STREAM_PAGE_SIZE as usize {
                return Err(PackError::ClusterExceedsPage(id));
            }
        } else {
            current = trial;
        }
    }
    if !current.cluster_ids.is_empty() {
        pages.push(current);
    }

    // 簇 → page_id
    let mut cluster_page = vec![u64::MAX; dag.records.len()];
    for (pi, pb) in pages.iter().enumerate() {
        for &cid in &pb.cluster_ids {
            cluster_page[cid as usize] = pi as u64;
        }
    }

    let mut out = Vec::with_capacity(pages.len());
    for (pi, pb) in pages.into_iter().enumerate() {
        let page_id = pi as u64;
        let mut is_root = false;
        for &cid in &pb.cluster_ids {
            if top.contains(&cid) {
                is_root = true;
                break;
            }
        }

        // 依赖：跨页边 → child 页依赖 parent 页
        let mut deps: Vec<u64> = Vec::new();
        let mut links: Vec<(u32, u32)> = Vec::new();
        let in_page: std::collections::HashSet<u32> = pb.cluster_ids.iter().copied().collect();
        for &(p, c) in &all_edges {
            let p_here = in_page.contains(&p);
            let c_here = in_page.contains(&c);
            if p_here || c_here {
                links.push((p, c));
            }
            if c_here && !p_here {
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

        let page = pb.finish(dag, page_id, is_root, deps, links);
        debug_assert!(page.encoded_len() <= STREAM_PAGE_SIZE as usize);
        out.push(page);
    }
    Ok(out)
}

/// 显式 RXGB v1 → 逻辑页（RXS-0331）。
pub fn rxgb_to_pages(bytes: &[u8]) -> Result<Vec<LogicalPage>, PackError> {
    let dag = read_dag(bytes)?;
    pack_cluster_dag(&dag)
}

/// 全页串接（双构建比对用）。
pub fn concatenate_pages(pages: &[LogicalPage]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in pages {
        out.extend_from_slice(&encode_logical_page(p));
    }
    out
}

#[derive(Clone)]
struct PageBuilder {
    cluster_ids: Vec<u32>,
}

impl PageBuilder {
    fn new() -> Self {
        Self {
            cluster_ids: Vec::new(),
        }
    }

    fn push(&mut self, id: u32, _dag: &ClusterDag) {
        self.cluster_ids.push(id);
    }

    /// 估算编码长度。依赖/边在封页前用占位上界：最坏每簇一条跨页依赖 + 全部相关边。
    /// 装箱决策用保守上界，保证最终页 ≤ STREAM_PAGE_SIZE。
    fn estimate_encoded_len(
        &self,
        dag: &ClusterDag,
        all_edges: &[(u32, u32)],
        _deps_hint: &[u64],
    ) -> usize {
        let mut vert = 0usize;
        let mut idx = 0usize;
        for &id in &self.cluster_ids {
            let r = &dag.records[id as usize];
            vert += r.vertex_count as usize;
            idx += (r.triangle_count as usize) * 3;
        }
        let in_page: std::collections::HashSet<u32> = self.cluster_ids.iter().copied().collect();
        let mut link_n = 0usize;
        let mut dep_n = 0usize;
        let mut dep_seen = std::collections::HashSet::new();
        for &(p, c) in all_edges {
            let p_here = in_page.contains(&p);
            let c_here = in_page.contains(&c);
            if p_here || c_here {
                link_n += 1;
            }
            if c_here && !p_here {
                // 上界：每条跨页边算一个唯一 dep（实际会 dedup，估大无妨）
                if dep_seen.insert(p) {
                    dep_n += 1;
                }
            }
        }
        HEADER_SIZE as usize
            + self.cluster_ids.len() * RECORD_SIZE as usize
            + vert * 12
            + idx
            + dep_n * 8
            + link_n * 8
    }

    fn finish(
        self,
        dag: &ClusterDag,
        page_id: u64,
        is_root: bool,
        deps: Vec<u64>,
        links: Vec<(u32, u32)>,
    ) -> LogicalPage {
        let mut bounds = [
            f32::INFINITY,
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        ];
        for &id in &self.cluster_ids {
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
        let mut clusters = Vec::with_capacity(self.cluster_ids.len());
        let mut lod_min = u16::MAX;
        let mut lod_max = 0u16;

        for &id in &self.cluster_ids {
            let r = &dag.records[id as usize];
            let n = &dag.nodes[id as usize];
            lod_min = lod_min.min(n.level as u16);
            lod_max = lod_max.max(n.level as u16);

            let v_off = vertices.len() as u32;
            let t_off = indices.len() as u32;
            vertices.extend_from_slice(dag.cluster_vertices(id));
            for t in 0..r.triangle_count {
                let tri = dag.cluster_triangle(id, t);
                indices.extend_from_slice(&tri);
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use rurix_geom_build::{TriMesh, build_dag, write_dag};
    use rurix_geom_pages::{decode_logical_page, schema_digest};
    use rurix_pkg::sha256;
    use std::collections::{HashMap, HashSet};

    fn fixture_dag() -> ClusterDag {
        build_dag(&TriMesh::uv_sphere(1.0, 24, 24))
    }

    //@ spec: RXS-0329
    #[test]
    fn double_pack_byte_equal() {
        let dag = fixture_dag();
        let a = pack_cluster_dag(&dag).unwrap();
        let b = pack_cluster_dag(&dag).unwrap();
        assert_eq!(concatenate_pages(&a), concatenate_pages(&b));
    }

    //@ spec: RXS-0329
    #[test]
    fn page_size_within_contract() {
        let pages = pack_cluster_dag(&fixture_dag()).unwrap();
        for p in &pages {
            let n = encode_logical_page(p).len();
            assert!(n <= STREAM_PAGE_SIZE as usize, "page {} len {n}", p.page_id);
        }
        assert!(pages.iter().any(|p| p.is_root()), "missing root page");
    }

    //@ spec: RXS-0330
    #[test]
    fn decode_equals_cpu_reference() {
        let dag = fixture_dag();
        let pages = pack_cluster_dag(&dag).unwrap();
        let mut nodes: HashMap<u32, (u32, u32)> = HashMap::new();
        let mut edges: HashSet<(u32, u32)> = HashSet::new();
        let mut bounds_map: HashMap<u32, ([u32; 3], u32, [u32; 3], u32, u32, u32)> = HashMap::new();

        for p in &pages {
            let bytes = encode_logical_page(p);
            let d = decode_logical_page(&bytes).unwrap();
            for c in &d.clusters {
                nodes.insert(c.cluster_id, (c.level, c.group));
                bounds_map.insert(
                    c.cluster_id,
                    (
                        c.center.map(f32::to_bits),
                        c.radius.to_bits(),
                        c.cone_axis.map(f32::to_bits),
                        c.cone_cutoff.to_bits(),
                        c.error.to_bits(),
                        c.parent_error.to_bits(),
                    ),
                );
            }
            for &e in &d.dag_links {
                edges.insert(e);
            }
        }

        assert_eq!(nodes.len(), dag.records.len());
        for id in 0..dag.records.len() as u32 {
            let n = dag.node(id);
            assert_eq!(nodes.get(&id), Some(&(n.level, n.group)));
            let r = dag.record(id);
            let got = bounds_map.get(&id).unwrap();
            assert_eq!(got.0, r.center.map(f32::to_bits));
            assert_eq!(got.1, r.radius.to_bits());
            assert_eq!(got.2, r.cone_axis.map(f32::to_bits));
            assert_eq!(got.3, r.cone_cutoff.to_bits());
            assert_eq!(got.4, r.error.to_bits());
            assert_eq!(got.5, r.parent_error.to_bits());
        }

        let mut ref_edges = HashSet::new();
        let mut lod_parents: HashMap<u32, HashSet<u32>> = HashMap::new();
        for parent in 0..dag.records.len() as u32 {
            for &child in dag.children_of(parent) {
                ref_edges.insert((parent, child));
                lod_parents.entry(child).or_default().insert(parent);
            }
        }
        assert_eq!(edges, ref_edges);

        // LOD parent：由边集导出（多父：同组多个父簇共享孩子）
        let mut got_parents: HashMap<u32, HashSet<u32>> = HashMap::new();
        for &(p, c) in &edges {
            got_parents.entry(c).or_default().insert(p);
        }
        assert_eq!(got_parents, lod_parents);
    }

    //@ spec: RXS-0331
    #[test]
    fn rxgb_converter_explicit() {
        let dag = fixture_dag();
        let rxgb = write_dag(&dag);
        let pages = rxgb_to_pages(&rxgb).unwrap();
        assert!(!pages.is_empty());
        // 坏 magic → 传播 RxgbError
        let mut bad = rxgb.clone();
        bad[0] = b'X';
        assert!(matches!(
            rxgb_to_pages(&bad),
            Err(PackError::Rxgb(RxgbError::BadMagic))
        ));
        // 未知 version
        let mut ver = rxgb;
        ver[4] = 9;
        assert!(matches!(
            rxgb_to_pages(&ver),
            Err(PackError::Rxgb(RxgbError::UnsupportedVersion(9)))
        ));
    }

    //@ spec: RXS-0328
    #[test]
    fn schema_digest_matches_header() {
        let pages = pack_cluster_dag(&fixture_dag()).unwrap();
        let bytes = encode_logical_page(&pages[0]);
        assert_eq!(&bytes[72..104], &schema_digest());
        let _ = sha256::hex(&schema_digest());
    }

    //@ spec: RXS-0329
    //@ spec: RXS-0330
    #[test]
    fn multi_page_pack_has_cross_page_deps() {
        // 更大网格迫使多页,覆盖跨页依赖边(24×24 球通常单页)。
        let dag = build_dag(&TriMesh::uv_sphere(1.0, 48, 48));
        let pages = pack_cluster_dag(&dag).unwrap();
        assert!(pages.len() >= 2, "期望多页,实测 {}", pages.len());
        for p in &pages {
            assert!(encode_logical_page(p).len() <= STREAM_PAGE_SIZE as usize);
        }
        assert!(pages.iter().any(|p| p.is_root()));
        // 跨页边 ⇒ child 页依赖表非空(至少一页)
        let has_deps = pages.iter().any(|p| !p.dependency_page_ids.is_empty());
        assert!(has_deps, "多页 DAG 应产生跨页依赖");
        // 解码合并仍全等
        let mut edges = HashSet::new();
        let mut nodes = HashSet::new();
        for p in &pages {
            let d = decode_logical_page(&encode_logical_page(p)).unwrap();
            for c in &d.clusters {
                nodes.insert(c.cluster_id);
            }
            for &e in &d.dag_links {
                edges.insert(e);
            }
        }
        assert_eq!(nodes.len(), dag.records.len());
        let mut ref_edges = HashSet::new();
        for parent in 0..dag.records.len() as u32 {
            for &child in dag.children_of(parent) {
                ref_edges.insert((parent, child));
            }
        }
        assert_eq!(edges, ref_edges);
    }
}
