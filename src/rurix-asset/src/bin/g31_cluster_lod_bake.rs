// Assisted-by: Claude（G31+ #58 簇 DAG LOD 生产接线——离线 bake 步骤 2/3）
//! G31+ #58 簇 DAG LOD 离线 bake（三步文件交接之第 2 步；步骤 1 =
//! `g14_3_pipeline_perf --dump-scene` 产 RXCS 装配 dump，步骤 3 = 生产车道
//! `--cluster-lod leaf|on --cluster-pack` 消费本工具产物）。
//!
//! 职责：RXCS 装配 dump（世界空间三角汤 + 逐三角属性 + 节点段表）→
//! 分块（节点段序贪心合并 ≥ min-block-tris；quad 灯面尾段与 emissive 三角
//! 恒 passthrough 不参与 LOD——光源几何面 0-byte）→ 位置 bits 精确焊接 →
//! `build_asset_dag`（事实源构建器，rurix-geom-build；单调性机核内嵌）→
//! 逐簇继承属性（叶后代面积加权 albedo 均值 + mat 众数；emission 恒 0
//! fail-closed 断言）→ RXCP v1 簇包落盘。
//!
//! 依赖方向说明：生产车道 bin 属 rurix-render，而 rurix-geom-build →
//! rurix-render（契约转引）——车道 bin 不能反向依赖离线构建器，故 DAG 构建
//! 落在本 crate（rurix-asset 依赖两者）。装配语义单源 = RXCS（本工具**不**
//! 复刻 glTF 装配，杜绝跨 bin 位级漂移）。
//!
//! 确定性：块间并行构建（块内单线程确定性 + 输出按块序写）⇒ 同输入同输出；
//! `--double-build` 自校验臂（两次 bake 字节相等，fail-closed）。
//!
//! 病态块降级（诚实登记不静默）：某块 `build_asset_dag` typed Err ⇒ 该块全部
//! 三角转 passthrough（保渲染正确性，放弃该块 LOD 收益）+ 计数进打印面。
//!
//! 用法：
//!   g31_cluster_lod_bake --scene-dump <scene.rxcs> --out <pack.rxcp> \
//!     [--min-block-tris 4096] [--threads N] [--double-build] [--attrs on]
//!
//! G31+ #96 属性臂（--attrs on,默认 off = RXCP v1 字节面逐位不变）：要求
//! RXCS v2（带 UV 段）输入 fail-closed;焊接键扩 (位置,UV) bits（接缝顶点
//! 不误并）→ `build_asset_dag_attrs_params`（属性保持简化,RXGB 冻结面/
//! m90 golden 不涉）→ RXCP v2 = 逐块 vertices 段后追加顶点 UV 平行表
//! （运行时 gather_tri_uv_attrs 按簇局部索引取 corner UV）。

use std::collections::HashMap;
use std::path::Path;

use rurix_geom_build::{
    AttrTriMesh, ClusterDag, DagAsset, DagBuildParams, SimplifyKind, TriMesh, TriMeshAttrs,
    build_asset_dag_attrs_params, build_asset_dag_params,
};

const TAG: &str = "[g31_cluster_lod_bake]";
const RXCS_MAGIC: &[u8; 4] = b"RXCS";
const RXCP_MAGIC: &[u8; 4] = b"RXCP";
/// 无材质/灯面三角哨兵（g14_3_lane_body `SLAB_TRI_NONE` 同字面）。
const MAT_NONE: u32 = u32::MAX;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// RXCS 读取（g14_3_lane_body::dump_scene_rxcs writer 逐字段镜像）
// ---------------------------------------------------------------------------

struct SceneDump {
    gltf_sha256: String,
    /// 9 f32/tri（世界空间，位保真）。
    tris: Vec<[[f32; 3]; 3]>,
    albedo: Vec<[f32; 3]>,
    emission: Vec<[f32; 3]>,
    tri_mat: Vec<u32>,
    /// (tri_offset, tri_count, is_light_tail)。
    groups: Vec<(u32, u32, bool)>,
    /// G31+ #96 RXCS v2:逐三角 corner UV（6 f32/tri 与 tris 同序位保真;
    /// v1 dump = None——属性臂 fail-closed 要求 Some,缺省臂两版都吃）。
    uv: Option<Vec<[f32; 6]>>,
}

struct Cur<'a> {
    b: &'a [u8],
    p: usize,
}

impl<'a> Cur<'a> {
    fn take(&mut self, n: usize) -> &'a [u8] {
        if self.p + n > self.b.len() {
            fail(&format!("RXCS 截断（need {n} at {}）", self.p));
        }
        let s = &self.b[self.p..self.p + n];
        self.p += n;
        s
    }
    fn u32(&mut self) -> u32 {
        let b = self.take(4);
        u32::from_le_bytes([b[0], b[1], b[2], b[3]])
    }
    fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }
    fn f32x3(&mut self) -> [f32; 3] {
        [self.f32(), self.f32(), self.f32()]
    }
}

fn read_scene_dump(path: &Path) -> SceneDump {
    let bytes = std::fs::read(path).unwrap_or_else(|e| fail(&format!("RXCS 读取 {path:?}: {e}")));
    let mut c = Cur { b: &bytes, p: 0 };
    if c.take(4) != RXCS_MAGIC {
        fail("RXCS magic 不符");
    }
    let version = c.u32();
    if version != 1 && version != 2 {
        fail(&format!("RXCS 版本不支持: {version}"));
    }
    let n = c.u32() as usize;
    let g = c.u32() as usize;
    let sha = String::from_utf8(c.take(64).to_vec()).unwrap_or_else(|_| fail("sha 非 utf8"));
    let mut groups = Vec::with_capacity(g);
    for _ in 0..g {
        let off = c.u32();
        let cnt = c.u32();
        let tail = c.u32() != 0;
        groups.push((off, cnt, tail));
    }
    let mut tris = Vec::with_capacity(n);
    for _ in 0..n {
        tris.push([c.f32x3(), c.f32x3(), c.f32x3()]);
    }
    let mut albedo = Vec::with_capacity(n);
    for _ in 0..n {
        albedo.push(c.f32x3());
    }
    let mut emission = Vec::with_capacity(n);
    for _ in 0..n {
        emission.push(c.f32x3());
    }
    let mut tri_mat = Vec::with_capacity(n);
    for _ in 0..n {
        tri_mat.push(c.u32());
    }
    // #96 RXCS v2:UV 尾段（6 f32/tri 与 tris 同序;v1 缺席 = None）。
    let uv = if version == 2 {
        let mut uv = Vec::with_capacity(n);
        for _ in 0..n {
            uv.push([c.f32(), c.f32(), c.f32(), c.f32(), c.f32(), c.f32()]);
        }
        Some(uv)
    } else {
        None
    };
    if c.p != bytes.len() {
        fail(&format!("RXCS 尾部冗余字节（pos {} ≠ len {}）", c.p, bytes.len()));
    }
    // 组表覆盖性：组段升序互斥覆盖 0..n（装配序不变量）。
    let mut cursor = 0u32;
    for &(off, cnt, _) in &groups {
        if off != cursor {
            fail(&format!("组段非连续: offset {off} ≠ cursor {cursor}"));
        }
        cursor += cnt;
    }
    if cursor as usize != n {
        fail(&format!("组段覆盖不全: {cursor} ≠ {n}"));
    }
    SceneDump {
        gltf_sha256: sha,
        tris,
        albedo,
        emission,
        tri_mat,
        groups,
        uv,
    }
}

// ---------------------------------------------------------------------------
// 分块 + 焊接
// ---------------------------------------------------------------------------

/// bake 块：源三角 id 列表（全局）+ 焊接网格（+ #96 属性臂顶点 UV 平行表）。
struct BakeBlock {
    /// 块内三角 i（= TriMesh 三角序）→ 全局源三角 id。
    src: Vec<u32>,
    mesh: TriMesh,
    /// #96 属性臂：与 mesh.positions 等长平行的顶点 UV（(位置,UV) bits
    /// 焊接产物——同位置不同 UV = 接缝拷贝不误并）;缺省臂 = None。
    uv: Option<Vec<[f32; 2]>>,
}

/// 分块：节点段序贪心合并 ≥ min_block_tris；尾段组与 emissive 三角进
/// passthrough。返回 (blocks, passthrough)。
/// #96：`attrs = true` 时焊接键 = (位置,UV) bits(调用方已保证 dump.uv 在场),
/// 缺省臂 = 既有位置 bits 焊接逐字。
fn partition_blocks(
    dump: &SceneDump,
    min_block_tris: usize,
    attrs: bool,
) -> (Vec<BakeBlock>, Vec<u32>) {
    let mut passthrough: Vec<u32> = Vec::new();
    let mut blocks: Vec<Vec<u32>> = Vec::new();
    let mut cur: Vec<u32> = Vec::new();
    for &(off, cnt, is_tail) in &dump.groups {
        if is_tail {
            // quad 灯面尾段：恒 passthrough（光源几何面 0-byte）。
            passthrough.extend(off..off + cnt);
            continue;
        }
        for t in off..off + cnt {
            if dump.emission[t as usize] != [0.0, 0.0, 0.0] {
                passthrough.push(t); // emissive 三角恒 passthrough
            } else {
                cur.push(t);
            }
        }
        if cur.len() >= min_block_tris {
            blocks.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        blocks.push(cur);
    }
    let blocks = blocks
        .into_iter()
        .map(|src| {
            if attrs {
                // #96 属性臂：(位置,UV) bits 精确焊接——接缝顶点(同位置
                // 不同 UV)保持独立 id,简化链按接缝保守锁定(crate 面同律)。
                let dump_uv = dump.uv.as_ref().expect("属性臂须 RXCS v2(调用方已核)");
                let mut weld: HashMap<[u32; 5], u32> = HashMap::new();
                let mut positions: Vec<[f32; 3]> = Vec::new();
                let mut uv: Vec<[f32; 2]> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();
                for &t in &src {
                    let u6 = &dump_uv[t as usize];
                    for (k, v) in dump.tris[t as usize].iter().enumerate() {
                        let uvk = [u6[k * 2], u6[k * 2 + 1]];
                        let key = [
                            v[0].to_bits(),
                            v[1].to_bits(),
                            v[2].to_bits(),
                            uvk[0].to_bits(),
                            uvk[1].to_bits(),
                        ];
                        let next = positions.len() as u32;
                        let id = *weld.entry(key).or_insert_with(|| {
                            positions.push(*v);
                            uv.push(uvk);
                            next
                        });
                        indices.push(id);
                    }
                }
                BakeBlock {
                    src,
                    mesh: TriMesh::new(positions, indices),
                    uv: Some(uv),
                }
            } else {
                // 位置 bits 精确焊接（与 DAG 跨层焊接同口径;「不同 id 同位置」合并
                // 已在 rurix-geom-build lib.rs 声明为已知行为）。
                let mut weld: HashMap<[u32; 3], u32> = HashMap::new();
                let mut positions: Vec<[f32; 3]> = Vec::new();
                let mut indices: Vec<u32> = Vec::new();
                for &t in &src {
                    for v in &dump.tris[t as usize] {
                        let key = v.map(f32::to_bits);
                        let next = positions.len() as u32;
                        let id = *weld.entry(key).or_insert_with(|| {
                            positions.push(*v);
                            next
                        });
                        indices.push(id);
                    }
                }
                BakeBlock {
                    src,
                    mesh: TriMesh::new(positions, indices),
                    uv: None,
                }
            }
        })
        .collect();
    (blocks, passthrough)
}

// ---------------------------------------------------------------------------
// 逐簇继承属性（粗簇三角消费面）
// ---------------------------------------------------------------------------

struct ClusterAttrs {
    albedo: Vec<[f32; 3]>,
    emission: Vec<[f32; 3]>,
    mat: Vec<u32>,
}

// ---------------------------------------------------------------------------
// 组共享 LOD 判定球（G31+ #58/B4）：派生事实源 = rurix-geom-build
// `lod_bounds::derive_lod_bounds`（本 bin 与 device 剔除 harness 共用,禁
// 双世界复刻）;运行时消费面 = rurix-render `select_lod_cut_grouped`。
// ---------------------------------------------------------------------------

fn tri_area(t: &[[f32; 3]; 3]) -> f64 {
    let e1 = [
        (t[1][0] - t[0][0]) as f64,
        (t[1][1] - t[0][1]) as f64,
        (t[1][2] - t[0][2]) as f64,
    ];
    let e2 = [
        (t[2][0] - t[0][0]) as f64,
        (t[2][1] - t[0][1]) as f64,
        (t[2][2] - t[0][2]) as f64,
    ];
    let c = [
        e1[1] * e2[2] - e1[2] * e2[1],
        e1[2] * e2[0] - e1[0] * e2[2],
        e1[0] * e2[1] - e1[1] * e2[0],
    ];
    0.5 * (c[0] * c[0] + c[1] * c[1] + c[2] * c[2]).sqrt()
}

/// 逐簇继承属性：叶后代源三角（sort+dedup——同组多父的组级共享子链接别名
/// 去重）面积加权 albedo 均值 + 三角数众数 mat（tie 取最小 id）。
/// emission fail-closed 断言恒 0（emissive 已剔 passthrough）。
fn derive_cluster_attrs(dag: &ClusterDag, block_src: &[u32], dump: &SceneDump) -> ClusterAttrs {
    let n = dag.records.len();
    let mut albedo = Vec::with_capacity(n);
    let mut emission = Vec::with_capacity(n);
    let mut mat = Vec::with_capacity(n);
    for id in 0..n as u32 {
        let mut leaves = dag.expand_to_leaves(&[id]);
        leaves.sort_unstable();
        leaves.dedup();
        let mut acc = [0.0f64; 3];
        let mut wsum = 0.0f64;
        let mut mats: Vec<u32> = Vec::new();
        for leaf in leaves {
            let r = dag.record(leaf);
            let base = r.triangle_offset as usize / 3;
            for t in 0..r.triangle_count as usize {
                let local = dag.leaf_source_tris[base + t] as usize;
                let src = block_src[local] as usize;
                if dump.emission[src] != [0.0, 0.0, 0.0] {
                    fail(&format!("块内 emissive 三角泄漏（源 {src}）——分块不变量破坏"));
                }
                let w = tri_area(&dump.tris[src]).max(1e-12);
                for k in 0..3 {
                    acc[k] += dump.albedo[src][k] as f64 * w;
                }
                wsum += w;
                mats.push(dump.tri_mat[src]);
            }
        }
        let inv = if wsum > 0.0 { 1.0 / wsum } else { 0.0 };
        albedo.push([
            (acc[0] * inv) as f32,
            (acc[1] * inv) as f32,
            (acc[2] * inv) as f32,
        ]);
        emission.push([0.0f32; 3]);
        // 众数（确定性：值升序计数，同数取最小值）。
        mats.sort_unstable();
        let mut best = (MAT_NONE, 0usize);
        let mut i = 0usize;
        while i < mats.len() {
            let v = mats[i];
            let mut j = i;
            while j < mats.len() && mats[j] == v {
                j += 1;
            }
            if j - i > best.1 {
                best = (v, j - i);
            }
            i = j;
        }
        mat.push(best.0);
    }
    ClusterAttrs {
        albedo,
        emission,
        mat,
    }
}

// ---------------------------------------------------------------------------
// RXCP 写出（g14_3_lane_body::read_cluster_pack reader 逐字段镜像）
// ---------------------------------------------------------------------------

struct BakedBlock {
    dag: ClusterDag,
    /// 叶层三角（DAG 导出序）→ 全局源三角 id。
    leaf_src_global: Vec<u32>,
    attrs: ClusterAttrs,
    /// 逐簇组共享 LOD 判定球（self = 生成组球 [cx,cy,cz,r]）。
    self_lod: Vec<[f32; 4]>,
    /// 逐簇组共享 LOD 判定球（parent = 所属组球）。
    parent_lod: Vec<[f32; 4]>,
    /// #96 属性臂：与 dag.vertices 等长平行的顶点 UV（build_dag_attrs
    /// 产物;RXCP v2 逐块 UV 段数据源）。缺省臂 = None。
    vertex_uv: Option<Vec<[f32; 2]>>,
}

/// RXCP 写出。#96：`attrs_on = true` ⇒ version 2,逐块 vertices 段后紧邻
/// 追加顶点 UV 平行表（2 f32/顶点;g14_3_lane_body read_cluster_pack v2
/// 镜像）;false ⇒ version 1 字节面逐位不变。
fn write_pack(
    dump: &SceneDump,
    passthrough: &[u32],
    baked: &[BakedBlock],
    attrs_on: bool,
    path: &Path,
) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    out.extend_from_slice(RXCP_MAGIC);
    out.extend_from_slice(&(if attrs_on { 2u32 } else { 1u32 }).to_le_bytes());
    out.extend_from_slice(dump.gltf_sha256.as_bytes());
    out.extend_from_slice(&(dump.tris.len() as u32).to_le_bytes());
    out.extend_from_slice(&(passthrough.len() as u32).to_le_bytes());
    for &p in passthrough {
        out.extend_from_slice(&p.to_le_bytes());
    }
    out.extend_from_slice(&(baked.len() as u32).to_le_bytes());
    for b in baked {
        let dag = &b.dag;
        out.extend_from_slice(&(dag.records.len() as u32).to_le_bytes());
        out.extend_from_slice(&(dag.children.len() as u32).to_le_bytes());
        out.extend_from_slice(&(dag.vertices.len() as u32).to_le_bytes());
        out.extend_from_slice(&(dag.triangle_indices.len() as u32).to_le_bytes());
        out.extend_from_slice(&(b.leaf_src_global.len() as u32).to_le_bytes());
        for r in &dag.records {
            for &x in &r.center {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
            out.extend_from_slice(&r.radius.to_bits().to_le_bytes());
            for &x in &r.cone_axis {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
            out.extend_from_slice(&r.cone_cutoff.to_bits().to_le_bytes());
            out.extend_from_slice(&r.error.to_bits().to_le_bytes());
            out.extend_from_slice(&r.parent_error.to_bits().to_le_bytes());
            out.extend_from_slice(&r.vertex_offset.to_le_bytes());
            out.extend_from_slice(&r.triangle_offset.to_le_bytes());
            out.extend_from_slice(&r.vertex_count.to_le_bytes());
            out.extend_from_slice(&r.triangle_count.to_le_bytes());
            out.extend_from_slice(&r.page_id.to_le_bytes());
            out.extend_from_slice(&r.reserved.to_le_bytes());
        }
        for n in &dag.nodes {
            out.extend_from_slice(&n.first_child.to_le_bytes());
            out.extend_from_slice(&n.child_count.to_le_bytes());
            out.extend_from_slice(&n.level.to_le_bytes());
            out.extend_from_slice(&n.group.to_le_bytes());
        }
        for &c in &dag.children {
            out.extend_from_slice(&c.to_le_bytes());
        }
        for v in &dag.vertices {
            for &x in v {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
        }
        // #96 RXCP v2:顶点 UV 平行表(fail-closed:属性臂每块必有且等长)。
        if attrs_on {
            let uv = b
                .vertex_uv
                .as_ref()
                .unwrap_or_else(|| fail("属性臂块缺顶点 UV 表（构建不变量破坏）"));
            if uv.len() != dag.vertices.len() {
                fail(&format!(
                    "属性臂块 UV 表长 {} ≠ 顶点数 {}（平行表不变量破坏）",
                    uv.len(),
                    dag.vertices.len()
                ));
            }
            for u in uv {
                for &x in u {
                    out.extend_from_slice(&x.to_bits().to_le_bytes());
                }
            }
        }
        out.extend_from_slice(&dag.triangle_indices);
        let pad = (4 - dag.triangle_indices.len() % 4) % 4;
        out.extend_from_slice(&[0u8; 3][..pad]);
        for &s in &b.leaf_src_global {
            out.extend_from_slice(&s.to_le_bytes());
        }
        for i in 0..dag.records.len() {
            for &x in &b.attrs.albedo[i] {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
            for &x in &b.attrs.emission[i] {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
            out.extend_from_slice(&b.attrs.mat[i].to_le_bytes());
            out.extend_from_slice(&0u32.to_le_bytes());
            // 组共享 LOD 判定球（self 生成组球 + parent 所属组球;#58/B4）。
            for &x in &b.self_lod[i] {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
            for &x in &b.parent_lod[i] {
                out.extend_from_slice(&x.to_bits().to_le_bytes());
            }
        }
    }
    std::fs::write(path, &out).unwrap_or_else(|e| fail(&format!("RXCP 写盘 {path:?}: {e}")));
    out
}

// ---------------------------------------------------------------------------
// bake 主流程
// ---------------------------------------------------------------------------

struct BakeResult {
    bytes: Vec<u8>,
    blocks: usize,
    degraded_blocks: usize,
    clusters: usize,
    levels_max: usize,
    leaf_tris: usize,
    passthrough: usize,
    root_tris: usize,
    /// 流送页总数（页 0 钉住 + 1..=total 分页;#20–23 驻留压力臂输入面）。
    total_pages: u32,
}

fn bake(
    dump: &SceneDump,
    min_block_tris: usize,
    threads: usize,
    kind: &DagBuildParams,
    attrs_on: bool,
    out: &Path,
) -> BakeResult {
    let (blocks, mut passthrough) = partition_blocks(dump, min_block_tris, attrs_on);
    // 块间并行构建（块内确定性单线程;结果按块序回收——同输入同输出）。
    // #96:结果面 = (DAG, 顶点 UV 平行表);缺省臂 UV 恒 None,构建调用与
    // 既有路径同一事实源(build_asset_dag_params)。
    type BlockResult = Result<(ClusterDag, Option<Vec<[f32; 2]>>), String>;
    let mut results: Vec<Option<BlockResult>> = Vec::new();
    results.resize_with(blocks.len(), || None);
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results_cell: Vec<std::sync::Mutex<Option<BlockResult>>> =
        results.into_iter().map(std::sync::Mutex::new).collect();
    std::thread::scope(|s| {
        for _ in 0..threads.max(1).min(blocks.len().max(1)) {
            s.spawn(|| {
                loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if i >= blocks.len() {
                        break;
                    }
                    let r: BlockResult = if attrs_on {
                        // #96 属性臂：属性资产级构建（panic 转译 + 单调性核验
                        // 与既有入口同律;RXGB 冻结面/m90 golden 不涉——
                        // 属性表为内存直构面,见 rurix-geom-build ClusterDagAttrs）。
                        build_asset_dag_attrs_params(
                            &AttrTriMesh {
                                mesh: blocks[i].mesh.clone(),
                                attrs: TriMeshAttrs {
                                    uv: blocks[i]
                                        .uv
                                        .clone()
                                        .expect("属性臂块必带 UV(partition 已产)"),
                                    normal: None,
                                },
                            },
                            kind,
                        )
                        .map(|a| (a.base, Some(a.vertex_uv)))
                        .map_err(|e| e.to_string())
                    } else {
                        build_asset_dag_params(
                            &DagAsset::static_mesh(blocks[i].mesh.clone()),
                            kind,
                        )
                        .map(|v2| (v2.base, None))
                        .map_err(|e| e.to_string())
                    };
                    *results_cell[i].lock().unwrap() = Some(r);
                }
            });
        }
    });
    let mut baked: Vec<BakedBlock> = Vec::new();
    let mut degraded = 0usize;
    // G31+ #20–23/E:簇 → 流送页分配（页 0 = 各块顶层簇——root 兜底钉住恒
    // 驻留,`apply_page_fallback` 终止性前提;其余簇按 (块序, 簇 id) 每
    // PAGE_CLUSTERS 簇一页,页号全局自 1 递增。写进 64B 记录 page_id 字段
    //（此前恒 0 的预留字段兑现;RXCP 布局 0 改动））。
    const PAGE_CLUSTERS: usize = 64;
    let mut next_page = 1u32;
    let mut in_page = 0usize;
    for (i, cell) in results_cell.into_iter().enumerate() {
        let r = cell
            .into_inner()
            .unwrap()
            .unwrap_or_else(|| fail("块构建结果缺失（调度不变量破坏）"));
        match r {
            Ok((mut dag, vertex_uv)) => {
                // 叶层源 id 全局化 + 叶覆盖完整性（块内恰一次）。
                if dag.leaf_source_tris.len() != blocks[i].src.len() {
                    fail(&format!(
                        "块 {i} 叶三角数 {} ≠ 输入 {}（DAG 叶覆盖破坏）",
                        dag.leaf_source_tris.len(),
                        blocks[i].src.len()
                    ));
                }
                // 页分配（顶层 = 页 0 钉住;其余全局递增分页）。
                let top: std::collections::HashSet<u32> = dag.top_level_ids().collect();
                for id in 0..dag.records.len() as u32 {
                    if top.contains(&id) {
                        dag.records[id as usize].page_id = 0;
                    } else {
                        if in_page == PAGE_CLUSTERS {
                            next_page += 1;
                            in_page = 0;
                        }
                        dag.records[id as usize].page_id = next_page;
                        in_page += 1;
                    }
                }
                let leaf_src_global: Vec<u32> = dag
                    .leaf_source_tris
                    .iter()
                    .map(|&local| blocks[i].src[local as usize])
                    .collect();
                let attrs = derive_cluster_attrs(&dag, &blocks[i].src, dump);
                let (self_lod, parent_lod) =
                    rurix_geom_build::lod_bounds::derive_lod_bounds(&dag)
                        .unwrap_or_else(|e| fail(&format!("块 {i} {e}(拒出包)")));
                baked.push(BakedBlock {
                    dag,
                    leaf_src_global,
                    attrs,
                    self_lod,
                    parent_lod,
                    vertex_uv,
                });
            }
            Err(e) => {
                // 病态块降级：整块 passthrough（保正确性,放弃 LOD 收益;诚实登记）。
                eprintln!("{TAG}: WARN 块 {i} DAG 构建失败,整块 passthrough: {e}");
                passthrough.extend_from_slice(&blocks[i].src);
                degraded += 1;
            }
        }
    }
    passthrough.sort_unstable();
    let bytes = write_pack(dump, &passthrough, &baked, attrs_on, out);
    let clusters: usize = baked.iter().map(|b| b.dag.records.len()).sum();
    let leaf_tris: usize = baked.iter().map(|b| b.leaf_src_global.len()).sum();
    let levels_max = baked.iter().map(|b| b.dag.level_count()).max().unwrap_or(0);
    let root_tris: usize = baked
        .iter()
        .map(|b| {
            b.dag
                .top_level_ids()
                .map(|id| b.dag.record(id).triangle_count as usize)
                .sum::<usize>()
        })
        .sum();
    BakeResult {
        bytes,
        blocks: baked.len(),
        degraded_blocks: degraded,
        clusters,
        levels_max,
        leaf_tris,
        passthrough: passthrough.len(),
        root_tris,
        total_pages: next_page,
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let d = rurix_pkg::sha256::digest(data);
    let mut s = String::with_capacity(64);
    for b in d {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut scene_dump = String::new();
    let mut out_path = String::new();
    let mut min_block_tris: usize = 4096;
    let mut threads: usize = std::thread::available_parallelism().map_or(1, |n| n.get());
    let mut double_build = false;
    // G31+ #66/#98:生产簇包默认质量档(QEM + 8 簇/组 + 边界交替——RXCP 为
    // 新面无 golden;shortest = 既有事实源对照臂(4 簇/组无偏置),m90 golden
    // 锚在 rxcook/build_dag 默认面)。
    let mut simplifier = DagBuildParams::quality();
    // G31+ #96:属性臂(默认 off = 既有 RXCP v1 字节面逐位不变;on = 属性
    // 保持简化 + RXCP v2 簇 UV 表,要求 RXCS v2 输入)。
    let mut attrs_on = false;
    let mut i = 1;
    while i < args.len() {
        let take = |args: &[String], i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--scene-dump" => scene_dump = take(&args, &mut i),
            "--out" => out_path = take(&args, &mut i),
            "--attrs" => {
                attrs_on = match take(&args, &mut i).as_str() {
                    "on" => true,
                    "off" => false,
                    other => fail(&format!("--attrs {other}：只接受 on|off")),
                }
            }
            "--min-block-tris" => {
                min_block_tris = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--min-block-tris 非 usize"))
            }
            "--threads" => {
                threads = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--threads 非 usize"))
            }
            "--double-build" => double_build = true,
            "--simplifier" => {
                simplifier = match take(&args, &mut i).as_str() {
                    "qem" => DagBuildParams::quality(),
                    "shortest" => DagBuildParams::default(),
                    other => fail(&format!("--simplifier {other}：只接受 qem|shortest")),
                }
            }
            "--group-size" => {
                simplifier.group_size = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--group-size 非 usize"));
                if simplifier.group_size < 2 || simplifier.group_size > 32 {
                    fail("--group-size 越闭集(2..=32)");
                }
            }
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if scene_dump.is_empty() || out_path.is_empty() {
        fail("参数闭集缺行（--scene-dump / --out）");
    }
    let dump = read_scene_dump(Path::new(&scene_dump));
    // #96 属性臂 fail-closed:要求 RXCS v2(带 UV 段)输入。
    if attrs_on && dump.uv.is_none() {
        fail(
            "--attrs on 需 RXCS v2（带 UV 段）输入——本 dump 为 v1 无 UV;\
             用 g14_3_pipeline_perf --dump-scene 重产（默认即 v2）",
        );
    }
    eprintln!(
        "{TAG}: RXCS 装载 tris={} groups={} sha={} uv={}",
        dump.tris.len(),
        dump.groups.len(),
        &dump.gltf_sha256[..16],
        dump.uv.is_some(),
    );
    if attrs_on {
        eprintln!("{TAG}: #96 属性臂 on（(位置,UV) 焊接 + build_dag_attrs + RXCP v2 簇 UV 表）");
    }
    let t0 = std::time::Instant::now();
    let r = bake(&dump, min_block_tris, threads, &simplifier, attrs_on, Path::new(&out_path));
    let bake_ms = t0.elapsed().as_secs_f64() * 1e3;
    let stuck = rurix_geom_build::qem::take_stuck_count();
    if double_build {
        let tmp = std::env::temp_dir().join(format!(
            "g31_cluster_lod_bake_double_{}.rxcp",
            std::process::id()
        ));
        let r2 = bake(&dump, min_block_tris, threads, &simplifier, attrs_on, &tmp);
        let equal = r.bytes == r2.bytes;
        let _ = std::fs::remove_file(&tmp);
        let _ = rurix_geom_build::qem::take_stuck_count();
        if !equal {
            fail("double-build 字节不等（确定性破坏）");
        }
        eprintln!("{TAG}: double-build 字节相等 OK");
    }
    println!(
        "{TAG}: bake OK blocks={} (degraded={}) clusters={} levels_max={} leaf_tris={} root_tris={} passthrough={} pages={} bytes={} sha256={} bake_ms={:.1} simplifier={} group_size={} sibling_bias={} qem_stuck_groups={} -> {}",
        r.blocks,
        r.degraded_blocks,
        r.clusters,
        r.levels_max,
        r.leaf_tris,
        r.root_tris,
        r.passthrough,
        r.total_pages,
        r.bytes.len(),
        sha256_hex(&r.bytes),
        bake_ms,
        match simplifier.simplify {
            SimplifyKind::Qem => "qem",
            SimplifyKind::ShortestEdge => "shortest",
        },
        simplifier.group_size,
        simplifier.sibling_bias,
        stuck,
        out_path,
    );
}
