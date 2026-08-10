//! ClusterDagV2 → RXPL major=2 逻辑页确定性装箱(RXS-0344 消费面;G9.2 M91)。
//!
//! 与 v1 [`crate::geom_build::pack_cluster_dag`] 同算法(页边界/依赖/边集逐字节
//! 同构),v2 差异仅在:record 面带误差/包围球(v1 96B 记录已有)+ 每簇 v2 扩展
//! (蒙皮元数据 + CLAS 输入段,RXS-0345 schema)。v1 打包路径 0-byte 不动。

use rurix_geom_build::{ClusterDagV2, ClusterSkinMeta};
use rurix_geom_pages::logical_v2::{LogicalPageV2, V2ClusterExt};
use rurix_geom_pages::{LogicalPage, STREAM_PAGE_SIZE, encode_logical_page_v2};

use crate::geom_build::{PackError, pack_cluster_dag};

/// `ClusterDagV2 → Vec<LogicalPageV2>`(packing_algo_id=1 沿用;RXS-0344 §1)。
///
/// 先按 v1 记录面装箱(页成员资格/依赖/边 = v1 语义),再把 v1 页内每簇的
/// v2 扩展(蒙皮三字段 + CLAS AABB)按 cluster_id 回填;骨骼资产缺三字段
/// 任一面在 builder 期已被 `build_asset_dag` typed `Err` 拒录,本层不再重演。
pub fn pack_cluster_dag_v2(dag: &ClusterDagV2) -> Result<Vec<LogicalPageV2>, PackError> {
    let v1_pages = pack_cluster_dag(&dag.base)?;
    let mut out = Vec::with_capacity(v1_pages.len());
    for p in v1_pages {
        let mut ext = Vec::with_capacity(p.clusters.len());
        for c in &p.clusters {
            let id = c.cluster_id as usize;
            let skin: &ClusterSkinMeta = &dag.skin[id];
            let clas = &dag.clas[id];
            ext.push(V2ClusterExt {
                max_influences: skin.max_influences,
                bone_indices: skin.bone_indices.clone().unwrap_or_default(),
                bound_inflation: skin.bound_inflation.unwrap_or(0.0),
                aabb_min: clas.aabb_min,
                aabb_max: clas.aabb_max,
            });
        }
        out.push(LogicalPageV2 { base: p, ext });
    }
    Ok(out)
}

/// v2 全页串接(双构建比对用)。
pub fn concatenate_pages_v2(pages: &[LogicalPageV2]) -> Vec<u8> {
    let mut out = Vec::new();
    for p in pages {
        out.extend_from_slice(&encode_logical_page_v2(p));
    }
    out
}

/// v2 页尺寸契约(沿 v1 STREAM_PAGE_SIZE;装箱估算复用 v1 上界 + v2 段增量,
/// 由本函数在装箱后机器核验,超出即 `PackError::ClusterExceedsPage`)。
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
        // v1/v2 共存律:v2 页的 v1 段面(簇成员/依赖/边)与纯 v1 打包逐字节同构。
        let dag = fixture();
        let v1 = pack_cluster_dag(&dag.base).unwrap();
        let v2 = pack_cluster_dag_v2(&dag).unwrap();
        assert_eq!(v1.len(), v2.len());
        for (a, b) in v1.iter().zip(v2.iter()) {
            assert_eq!(a, &b.base);
        }
    }
}
