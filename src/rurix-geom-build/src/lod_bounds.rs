//! 组共享 LOD 判定球派生（G31+ #58/B4;Nanite "same input → same output"
//! 语义——SIGGRAPH 2021 深潜:同一判定的全部消费方共享逐位相同的
//! (LOD 球, 误差),组内判定必然一致,无需簇间通信）。
//!
//! 事实源纪律:本模块为 bake（rurix-asset `g31_cluster_lod_bake`）与 device
//! 剔除对拍 harness 的**唯一**派生实现（禁双世界复刻）;运行时消费面 =
//! rurix-render `geometry::visible_cluster_set::select_lod_cut_grouped`。
//!
//! 数学不变量（运行时球面最近点投影单调 ⇒ 每条 root→leaf 链判定恰翻转
//! 一次 ⇒ cut 无洞无重叠）:
//! - `lod[c]`:叶 = 几何球(64B 记录 center/radius);非叶 = 生成组球 =
//!   孩子 lod 球并集(**升序**成员序——与 parent 侧同源逐位一致);
//! - `self_lod[c] = lod[c]`;`parent_lod[c]` = 所属组([`crate::dag::DagNode`]
//!   `group`)成员 lod 球并集(根组 parent_error = +∞ 恒不过,球给自身占位);
//! - 球沿链并集嵌套 + 误差单调(builder 机核在案)⇒ 投影单调。
//!
//! 嵌套性由 [`verify_lod_bounds_nesting`] fail-closed 机核（两遍法并集的
//! f32 舍入边缘破坏即拒,不静默出包）。

use std::collections::HashMap;

use crate::dag::ClusterDag;

/// 成员球并集（两遍法,确定性）:第一遍 f64 增量闭式估球心,第二遍 f32 域
/// 精确覆盖半径 `R = max(dist(center, ci) + ri)`——f32 语义下必然包含全部
/// 成员球（保守方向;运行时投影单调性的球嵌套前提）。
pub fn sphere_union(members: &[([f32; 3], f32)]) -> ([f32; 3], f32) {
    if members.is_empty() {
        return ([0.0; 3], 0.0);
    }
    let mut c = [
        members[0].0[0] as f64,
        members[0].0[1] as f64,
        members[0].0[2] as f64,
    ];
    let mut r = members[0].1 as f64;
    for m in &members[1..] {
        let mc = [m.0[0] as f64, m.0[1] as f64, m.0[2] as f64];
        let mr = m.1 as f64;
        let d = ((mc[0] - c[0]).powi(2) + (mc[1] - c[1]).powi(2) + (mc[2] - c[2]).powi(2)).sqrt();
        if d + mr <= r {
            continue; // 已含
        }
        if d + r <= mr {
            c = mc;
            r = mr;
            continue; // 被含
        }
        let nr = (d + r + mr) * 0.5;
        let t = (nr - r) / d.max(1e-30);
        c = [
            c[0] + (mc[0] - c[0]) * t,
            c[1] + (mc[1] - c[1]) * t,
            c[2] + (mc[2] - c[2]) * t,
        ];
        r = nr;
    }
    let center = [c[0] as f32, c[1] as f32, c[2] as f32];
    // 第二遍:f32 域精确覆盖（含 f64→f32 舍入的兜底）。
    let mut radius = 0.0f32;
    for m in members {
        let d = ((m.0[0] - center[0]).powi(2)
            + (m.0[1] - center[1]).powi(2)
            + (m.0[2] - center[2]).powi(2))
        .sqrt();
        radius = radius.max(d + m.1);
    }
    (center, radius)
}

/// 逐簇组共享 LOD 判定球派生（self 生成组球 + parent 所属组球,各
/// `[cx, cy, cz, r]`）。派生后嵌套机核 fail-closed（typed Err）。
pub fn derive_lod_bounds(dag: &ClusterDag) -> Result<(Vec<[f32; 4]>, Vec<[f32; 4]>), String> {
    let n = dag.records.len();
    let mut lod: Vec<([f32; 3], f32)> = vec![([0.0; 3], 0.0); n];
    // 层升序（叶层先）:非叶 = 孩子 lod 球并集（孩子恒在下层,已算）。
    for l in &dag.levels {
        for id in l.record_start..l.record_start + l.record_count {
            let node = dag.node(id);
            if node.child_count == 0 {
                let r = dag.record(id);
                lod[id as usize] = (r.center, r.radius);
            } else {
                let mut ch: Vec<u32> = dag.children_of(id).to_vec();
                ch.sort_unstable();
                let members: Vec<([f32; 3], f32)> = ch.iter().map(|&c| lod[c as usize]).collect();
                lod[id as usize] = sphere_union(&members);
            }
        }
    }
    // 组分桶（组号全局唯一;成员升序 = self 侧 children 排序后同一序列 ⇒
    // parent 球与产物 self 球逐位同源）。
    let mut group_members: HashMap<u32, Vec<u32>> = HashMap::new();
    for id in 0..n as u32 {
        group_members
            .entry(dag.node(id).group)
            .or_default()
            .push(id);
    }
    let mut group_sphere: HashMap<u32, ([f32; 3], f32)> = HashMap::new();
    for (&g, members) in &group_members {
        let mut ms = members.clone();
        ms.sort_unstable();
        let spheres: Vec<([f32; 3], f32)> = ms.iter().map(|&c| lod[c as usize]).collect();
        group_sphere.insert(g, sphere_union(&spheres));
    }
    let mut self_out = Vec::with_capacity(n);
    let mut parent_out = Vec::with_capacity(n);
    for id in 0..n as u32 {
        let s = lod[id as usize];
        self_out.push([s.0[0], s.0[1], s.0[2], s.1]);
        let p = group_sphere[&dag.node(id).group];
        parent_out.push([p.0[0], p.0[1], p.0[2], p.1]);
    }
    verify_lod_bounds_nesting(dag, &lod, &self_out, &parent_out)?;
    Ok((self_out, parent_out))
}

/// LOD 球嵌套机核（fail-closed;运行时投影单调 ⇒ cut 无洞无重叠的球面
/// 前提）:① 非叶簇 self 球 ⊇ 每个孩子 LOD 球;② 每簇 parent 球 ⊇ 自身
/// LOD 球。包含判定与两遍法半径同式（f32 域 + 1 ulp 级容差）。
pub fn verify_lod_bounds_nesting(
    dag: &ClusterDag,
    lod: &[([f32; 3], f32)],
    self_out: &[[f32; 4]],
    parent_out: &[[f32; 4]],
) -> Result<(), String> {
    let contains = |outer: &[f32; 4], inner: &([f32; 3], f32)| -> bool {
        let d = ((inner.0[0] - outer[0]).powi(2)
            + (inner.0[1] - outer[1]).powi(2)
            + (inner.0[2] - outer[2]).powi(2))
        .sqrt();
        d + inner.1 <= outer[3] * (1.0 + 1e-6) + 1e-6
    };
    for id in 0..dag.records.len() as u32 {
        for &ch in dag.children_of(id) {
            if !contains(&self_out[id as usize], &lod[ch as usize]) {
                return Err(format!(
                    "LOD 球嵌套破坏: 簇 {id} self 球未包含孩子 {ch}(单调投影前提破坏)"
                ));
            }
        }
        if !contains(&parent_out[id as usize], &lod[id as usize]) {
            return Err(format!("LOD 球嵌套破坏: 簇 {id} parent 球未包含自身 LOD 球"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::{DagBuildParams, build_dag_params};
    use crate::mesh::TriMesh;

    #[test]
    fn union_contains_members_and_deterministic() {
        let members = vec![
            ([0.0f32, 0.0, -100.0], 1.0f32),
            ([0.0, 0.0, -400.0], 1.0),
            ([3.0, -2.0, -250.0], 5.0),
        ];
        let (c, r) = sphere_union(&members);
        for m in &members {
            let d = ((m.0[0] - c[0]).powi(2) + (m.0[1] - c[1]).powi(2) + (m.0[2] - c[2]).powi(2))
                .sqrt();
            assert!(d + m.1 <= r * (1.0 + 1e-6) + 1e-6, "并集未包含成员");
        }
        let (c2, r2) = sphere_union(&members);
        assert_eq!(c.map(f32::to_bits), c2.map(f32::to_bits));
        assert_eq!(r.to_bits(), r2.to_bits());
    }

    #[test]
    fn derive_bounds_nesting_holds_for_quality_dag() {
        let mesh = TriMesh::uv_sphere(1.0, 24, 24);
        let dag = build_dag_params(&mesh, &DagBuildParams::quality());
        let (self_b, parent_b) = derive_lod_bounds(&dag).expect("嵌套机核过");
        assert_eq!(self_b.len(), dag.records.len());
        assert_eq!(parent_b.len(), dag.records.len());
        // 同组产物 self 球逐位一致（"same input → same output" 的产物面）。
        for id in 0..dag.records.len() as u32 {
            for &ch in dag.children_of(id) {
                // 孩子的 parent 球 == 本簇 self 球（相邻层判定同源）。
                let pb = parent_b[ch as usize];
                let sb = self_b[id as usize];
                assert_eq!(
                    pb.map(f32::to_bits),
                    sb.map(f32::to_bits),
                    "簇 {id} 孩子 {ch} 判定球不同源"
                );
            }
        }
    }
}
