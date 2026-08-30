//! 输入三角网格模型与内置生成器。
//!
//! 报告1 §3.1 口径:离线构建输入为「索引 + 位置」,P0 不引入法线/UV 属性
//! (属性在 P2 顶点获取路径再进入)。G31+ #96 起,UV/法线经 [`TriMeshAttrs`]
//! **加性伴随结构**进入属性保持简化链(新入口 [`TriMesh::with_attrs`];
//! `TriMesh` 本体与既有构建链 0-byte 不动)。生成器产出的网格顶点已按索引共享
//! (焊接),供簇化邻接生长与 DAG 组边界判定直接使用;跨层顶点一致性靠
//! 「端点收缩不改变存活顶点坐标」+ 精确位置焊接维持(见 [`crate::dag`])。

use std::collections::HashMap;

/// 三角网格(位置 + 三角形索引;索引数为 3 的倍数)。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriMesh {
    pub positions: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

/// 顶点属性平行表(G31+ #96 属性保持简化输入面)。
///
/// meshopt `simplifyWithAttributes` 顶点属性口径:属性与 [`TriMesh::positions`]
/// 等长平行;UV 接缝由上游按「位置 + 属性」预拆分顶点承载(同位置不同 UV =
/// 不同顶点 id,简化链按接缝顶点保守锁定处理,见 [`crate::dag::build_dag_attrs`])。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TriMeshAttrs {
    /// 逐顶点 UV(#96 最小属性面,必备;与 positions 等长)。
    pub uv: Vec<[f32; 2]>,
    /// 逐顶点法线(可选;与 positions 等长。插值端点逐位拷贝原值,内点
    /// 插值后重归一化——非单位输入不拒)。
    pub normal: Option<Vec<[f32; 3]>>,
}

/// 带属性三角网格(TriMesh + 平行属性表的伴随结构;经 [`TriMesh::with_attrs`]
/// 校验构造)。字段公开维持仓内离线工具惯例——绕过构造的输入由消费入口
/// ([`crate::dag::build_dag_attrs`])再次 fail-closed 校验。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AttrTriMesh {
    pub mesh: TriMesh,
    pub attrs: TriMeshAttrs,
}

/// 属性网格退化输入 typed 错误(G31+ #96 纪律:属性链新入口一律 typed Err,
/// 既有构造函数的 panic 面不动)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttrMeshError {
    /// 零三角形(空网格无简化/烘焙语义)。
    EmptyMesh,
    /// 索引数非 3 的倍数。
    IndicesNotTriples { len: usize },
    /// 三角形索引越界。
    IndexOutOfBounds { index: u32, vertex_count: usize },
    /// UV 表与顶点数不齐。
    UvLengthMismatch { vertices: usize, uv: usize },
    /// 法线表与顶点数不齐。
    NormalLengthMismatch { vertices: usize, normals: usize },
    /// 非有限数值(表名 + 顶点下标)。
    NonFinite { table: &'static str, index: usize },
}

impl std::fmt::Display for AttrMeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttrMeshError::EmptyMesh => write!(f, "属性网格为空(零三角形)"),
            AttrMeshError::IndicesNotTriples { len } => {
                write!(f, "三角形索引数 {len} 非 3 的倍数")
            }
            AttrMeshError::IndexOutOfBounds {
                index,
                vertex_count,
            } => {
                write!(f, "三角形索引 {index} 越界(顶点数 {vertex_count})")
            }
            AttrMeshError::UvLengthMismatch { vertices, uv } => {
                write!(f, "UV 表长度 {uv} 与顶点数 {vertices} 不齐")
            }
            AttrMeshError::NormalLengthMismatch { vertices, normals } => {
                write!(f, "法线表长度 {normals} 与顶点数 {vertices} 不齐")
            }
            AttrMeshError::NonFinite { table, index } => {
                write!(f, "{table}[{index}] 含非有限数值")
            }
        }
    }
}

impl std::error::Error for AttrMeshError {}

/// 属性网格输入统一校验(#96 三入口共用单一事实源:[`TriMesh::with_attrs`] /
/// [`crate::dag::build_dag_attrs`] / [`crate::qem::simplify_free_mesh_attrs`];
/// fail-closed typed Err,不 panic、不静默钳制)。
pub(crate) fn validate_attr_input(
    positions: &[[f32; 3]],
    indices: &[u32],
    uv: &[[f32; 2]],
    normal: Option<&[[f32; 3]]>,
) -> Result<(), AttrMeshError> {
    if indices.is_empty() {
        return Err(AttrMeshError::EmptyMesh);
    }
    if !indices.len().is_multiple_of(3) {
        return Err(AttrMeshError::IndicesNotTriples {
            len: indices.len(),
        });
    }
    let n = positions.len();
    for &i in indices {
        if i as usize >= n {
            return Err(AttrMeshError::IndexOutOfBounds {
                index: i,
                vertex_count: n,
            });
        }
    }
    if uv.len() != n {
        return Err(AttrMeshError::UvLengthMismatch {
            vertices: n,
            uv: uv.len(),
        });
    }
    if let Some(nm) = normal
        && nm.len() != n
    {
        return Err(AttrMeshError::NormalLengthMismatch {
            vertices: n,
            normals: nm.len(),
        });
    }
    for (i, p) in positions.iter().enumerate() {
        if !p.iter().all(|v| v.is_finite()) {
            return Err(AttrMeshError::NonFinite {
                table: "positions",
                index: i,
            });
        }
    }
    for (i, t) in uv.iter().enumerate() {
        if !t.iter().all(|v| v.is_finite()) {
            return Err(AttrMeshError::NonFinite {
                table: "uv",
                index: i,
            });
        }
    }
    if let Some(nm) = normal {
        for (i, t) in nm.iter().enumerate() {
            if !t.iter().all(|v| v.is_finite()) {
                return Err(AttrMeshError::NonFinite {
                    table: "normal",
                    index: i,
                });
            }
        }
    }
    Ok(())
}

impl TriMesh {
    /// 构造并校验(索引数 3 的倍数且不越界;离线工具,坏输入即panic)。
    pub fn new(positions: Vec<[f32; 3]>, indices: Vec<u32>) -> Self {
        assert!(indices.len().is_multiple_of(3), "三角形索引数须为 3 的倍数");
        let n = positions.len() as u32;
        assert!(indices.iter().all(|&i| i < n), "三角形索引越界");
        Self { positions, indices }
    }

    /// 附着顶点属性 → 带属性网格(G31+ #96 新入口;既有构造函数签名不动)。
    /// 退化输入(空网格/索引非 3 倍数/越界/属性长度不齐/非有限)= typed Err。
    pub fn with_attrs(self, attrs: TriMeshAttrs) -> Result<AttrTriMesh, AttrMeshError> {
        validate_attr_input(
            &self.positions,
            &self.indices,
            &attrs.uv,
            attrs.normal.as_deref(),
        )?;
        Ok(AttrTriMesh { mesh: self, attrs })
    }

    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    pub fn triangle(&self, f: usize) -> [u32; 3] {
        [
            self.indices[3 * f],
            self.indices[3 * f + 1],
            self.indices[3 * f + 2],
        ]
    }

    pub fn triangles(&self) -> Vec<[u32; 3]> {
        self.indices
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect()
    }

    /// 立方体(8 顶点 12 三角形,三面交点共享顶点;`half_extent` 半边长)。
    /// 绕序外法线向外(单测校验),供背面锥剔除对拍。
    pub fn cube(half_extent: f32) -> Self {
        let h = half_extent;
        let positions = vec![
            [-h, -h, -h],
            [h, -h, -h],
            [h, h, -h],
            [-h, h, -h],
            [-h, -h, h],
            [h, -h, h],
            [h, h, h],
            [-h, h, h],
        ];
        #[rustfmt::skip]
        let indices: Vec<u32> = vec![
            0, 2, 1, 0, 3, 2, // -z
            4, 5, 6, 4, 6, 7, // +z
            0, 4, 7, 0, 7, 3, // -x
            1, 2, 6, 1, 6, 5, // +x
            0, 1, 5, 0, 5, 4, // -y
            3, 7, 6, 3, 6, 2, // +y
        ];
        Self::new(positions, indices)
    }

    /// UV 球(报告1 §6.4 压力测试替代物;参数化经纬分段)。
    ///
    /// 极点为单顶点共享扇(避免「不同 id 同位置」顶点被 DAG 层级焊接误并)。
    /// `segments` 经向分段 ≥3,`rings` 纬向分段 ≥2;64×64 ≈ 8k 三角形。
    pub fn uv_sphere(radius: f32, segments: u32, rings: u32) -> Self {
        assert!(segments >= 3, "经向分段 ≥3");
        assert!(rings >= 2, "纬向分段 ≥2");
        use std::f32::consts::PI;
        let mut positions = Vec::with_capacity(2 + (rings - 1) as usize * segments as usize);
        positions.push([0.0, radius, 0.0]); // 顶极 = 顶点 0
        for i in 1..rings {
            let phi = PI * i as f32 / rings as f32; // 自顶向下
            let (sp, cp) = phi.sin_cos();
            for j in 0..segments {
                let theta = 2.0 * PI * j as f32 / segments as f32;
                let (st, ct) = theta.sin_cos();
                positions.push([radius * sp * ct, radius * cp, radius * sp * st]);
            }
        }
        positions.push([0.0, -radius, 0.0]); // 底极 = 末顶点
        let bottom = positions.len() as u32 - 1;
        let ring = |i: u32, j: u32| 1 + (i - 1) * segments + (j % segments);
        let mut indices = Vec::new();
        for j in 0..segments {
            indices.extend_from_slice(&[0, ring(1, j + 1), ring(1, j)]); // 顶扇
        }
        for i in 1..rings - 1 {
            for j in 0..segments {
                let (a, b) = (ring(i, j), ring(i + 1, j));
                let (a2, b2) = (ring(i, j + 1), ring(i + 1, j + 1));
                indices.extend_from_slice(&[a, b2, b]);
                indices.extend_from_slice(&[a, a2, b2]);
            }
        }
        for j in 0..segments {
            indices.extend_from_slice(&[bottom, ring(rings - 1, j), ring(rings - 1, j + 1)]); // 底扇
        }
        Self::new(positions, indices)
    }

    /// 平面网格(n×n 方格 = 2n² 三角形,z=0 法线 +z,覆盖 `[-half, half]²`)。
    pub fn plane_grid(n: u32, half_extent: f32) -> Self {
        assert!(n >= 1, "至少 1×1 方格");
        let mut positions = Vec::with_capacity((n as usize + 1) * (n as usize + 1));
        for i in 0..=n {
            for j in 0..=n {
                let x = -half_extent + 2.0 * half_extent * j as f32 / n as f32;
                let y = -half_extent + 2.0 * half_extent * i as f32 / n as f32;
                positions.push([x, y, 0.0]);
            }
        }
        let vid = |i: u32, j: u32| i * (n + 1) + j;
        let mut indices = Vec::with_capacity(n as usize * n as usize * 6);
        for i in 0..n {
            for j in 0..n {
                let (v00, v10, v11, v01) =
                    (vid(i, j), vid(i, j + 1), vid(i + 1, j + 1), vid(i + 1, j));
                indices.extend_from_slice(&[v00, v10, v11]);
                indices.extend_from_slice(&[v00, v11, v01]);
            }
        }
        Self::new(positions, indices)
    }
}

/// 共享边邻接(每条无向边 → 面列表,面两两互邻)。
///
/// 簇化贪心生长(`cluster.rs`)与 DAG 组边界判定(`dag.rs`)的公共输入。
/// 流形边(≤2 面)为常态;非流形边全部记录,不静默丢弃。
pub fn build_face_adjacency(tris: &[[u32; 3]]) -> Vec<Vec<u32>> {
    let mut edge_map: HashMap<(u32, u32), Vec<u32>> = HashMap::new();
    for (f, t) in tris.iter().enumerate() {
        for e in 0..3 {
            let (a, b) = (t[e], t[(e + 1) % 3]);
            edge_map
                .entry((a.min(b), a.max(b)))
                .or_default()
                .push(f as u32);
        }
    }
    let mut adj: Vec<Vec<u32>> = vec![Vec::new(); tris.len()];
    // 按边键排序遍历,避免 HashMap 迭代序导致跨进程非确定性
    // (邻接序影响簇化前沿序,同分决胜依赖前沿先入者)。
    let mut edges: Vec<((u32, u32), Vec<u32>)> = edge_map.into_iter().collect();
    edges.sort_unstable_by_key(|(e, _)| *e);
    for (_, faces) in edges {
        for &f in &faces {
            for &g in &faces {
                if f != g && !adj[f as usize].contains(&g) {
                    adj[f as usize].push(g);
                }
            }
        }
    }
    for nbrs in &mut adj {
        nbrs.sort_unstable();
    }
    adj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vecmath::{vcross, vdot, vsub};

    fn face_normal_centroid(m: &TriMesh, f: usize) -> ([f32; 3], [f32; 3]) {
        let t = m.triangle(f);
        let (a, b, c) = (
            m.positions[t[0] as usize],
            m.positions[t[1] as usize],
            m.positions[t[2] as usize],
        );
        let n = vcross(vsub(b, a), vsub(c, a));
        let c0 = [
            (a[0] + b[0] + c[0]) / 3.0,
            (a[1] + b[1] + c[1]) / 3.0,
            (a[2] + b[2] + c[2]) / 3.0,
        ];
        (n, c0)
    }

    #[test]
    fn cube_counts_and_outward_normals() {
        let m = TriMesh::cube(0.5);
        assert_eq!(m.positions.len(), 8);
        assert_eq!(m.triangle_count(), 12);
        // 外法线:法线与质心(相对原点)同向。
        for f in 0..m.triangle_count() {
            let (n, c) = face_normal_centroid(&m, f);
            assert!(vdot(n, c) > 0.0, "三角形 {f} 法线朝内");
        }
    }

    #[test]
    fn uv_sphere_counts_outward_and_welded() {
        let (seg, rings) = (64, 64);
        let m = TriMesh::uv_sphere(1.0, seg, rings);
        assert_eq!(m.positions.len(), 2 + (rings as usize - 1) * seg as usize);
        assert_eq!(
            m.triangle_count(),
            (2 * seg + (rings - 2) * seg * 2) as usize
        );
        // 外法线 + 极点焊接(无重复位置,保证层级焊接安全)。
        let mut seen = std::collections::HashSet::new();
        for p in &m.positions {
            assert!(seen.insert(p.map(f32::to_bits)), "重复位置 {p:?}");
        }
        for f in 0..m.triangle_count() {
            let (n, c) = face_normal_centroid(&m, f);
            assert!(vdot(n, c) > 0.0, "三角形 {f} 法线朝内");
        }
    }

    #[test]
    fn plane_grid_counts_and_normals() {
        let m = TriMesh::plane_grid(4, 1.0);
        assert_eq!(m.positions.len(), 25);
        assert_eq!(m.triangle_count(), 32);
        for f in 0..m.triangle_count() {
            let (n, _) = face_normal_centroid(&m, f);
            assert!(n[2] > 0.0, "三角形 {f} 法线非 +z");
        }
    }

    /// G31+ #96:属性扩面新入口——合法附着 + 退化输入 typed Err 五臂
    /// (空网格/索引越界/UV 不齐/法线不齐/非有限),既有构造 panic 面不动。
    #[test]
    fn with_attrs_valid_and_degenerate_typed_err() {
        let mesh = TriMesh::plane_grid(2, 1.0);
        let n = mesh.positions.len();
        let uv: Vec<[f32; 2]> = mesh
            .positions
            .iter()
            .map(|p| [(p[0] + 1.0) * 0.5, (p[1] + 1.0) * 0.5])
            .collect();
        // 合法:仅 UV / UV+法线 两态。
        let am = mesh
            .clone()
            .with_attrs(TriMeshAttrs {
                uv: uv.clone(),
                normal: None,
            })
            .expect("合法 UV 附着");
        assert_eq!(am.attrs.uv.len(), n);
        mesh.clone()
            .with_attrs(TriMeshAttrs {
                uv: uv.clone(),
                normal: Some(vec![[0.0, 0.0, 1.0]; n]),
            })
            .expect("合法 UV+法线附着");
        // 空网格。
        let empty = TriMesh::default().with_attrs(TriMeshAttrs::default());
        assert_eq!(empty.unwrap_err(), AttrMeshError::EmptyMesh);
        // 索引越界(字面构造绕过 TriMesh::new 校验——本入口必须自校)。
        let oob = TriMesh {
            positions: vec![[0.0; 3]],
            indices: vec![0, 1, 2],
        }
        .with_attrs(TriMeshAttrs {
            uv: vec![[0.0; 2]],
            normal: None,
        });
        assert!(matches!(
            oob.unwrap_err(),
            AttrMeshError::IndexOutOfBounds { index: 1, .. }
        ));
        // 索引数非 3 倍数。
        let not3 = TriMesh {
            positions: vec![[0.0; 3]; 2],
            indices: vec![0, 1],
        }
        .with_attrs(TriMeshAttrs {
            uv: vec![[0.0; 2]; 2],
            normal: None,
        });
        assert!(matches!(
            not3.unwrap_err(),
            AttrMeshError::IndicesNotTriples { len: 2 }
        ));
        // UV 表不齐。
        let short_uv = mesh.clone().with_attrs(TriMeshAttrs {
            uv: uv[..n - 1].to_vec(),
            normal: None,
        });
        assert!(matches!(
            short_uv.unwrap_err(),
            AttrMeshError::UvLengthMismatch { .. }
        ));
        // 法线表不齐。
        let short_nm = mesh.clone().with_attrs(TriMeshAttrs {
            uv: uv.clone(),
            normal: Some(vec![[0.0, 0.0, 1.0]; n - 1]),
        });
        assert!(matches!(
            short_nm.unwrap_err(),
            AttrMeshError::NormalLengthMismatch { .. }
        ));
        // 非有限 UV。
        let mut nan_uv = uv;
        nan_uv[0][0] = f32::NAN;
        let nan = mesh.with_attrs(TriMeshAttrs {
            uv: nan_uv,
            normal: None,
        });
        assert!(matches!(
            nan.unwrap_err(),
            AttrMeshError::NonFinite { table: "uv", index: 0 }
        ));
    }

    #[test]
    fn adjacency_plane_and_cube() {
        // 1×1 平面:两个三角形互邻。
        let m = TriMesh::plane_grid(1, 1.0);
        let tris = m.triangles();
        let adj = build_face_adjacency(&tris);
        assert_eq!(adj.len(), 2);
        assert_eq!(adj[0], vec![1]);
        assert_eq!(adj[1], vec![0]);
        // 立方体:闭合流形,每条边恰 2 面 → 邻接条目总数 = 2 × 共享边数。
        let m = TriMesh::cube(1.0);
        let tris = m.triangles();
        let adj = build_face_adjacency(&tris);
        let entries: usize = adj.iter().map(Vec::len).sum();
        assert_eq!(entries, 2 * 18); // 18 条共享边(立方体 12 边 + 6 面对角线)
        for faces in &adj {
            assert_eq!(faces.len(), 3); // 每三角形 3 个边邻居
        }
    }
}
