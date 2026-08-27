// Assisted-by: Claude（G31+ #95/#68/#67 WP cell HLOD 离线 bake——三步文件交接之第 2 步）
//! G31+ #95/#68 World Partition cell HLOD 离线 bake（三步文件交接之第 2 步；
//! 步骤 1 = `g14_3_pipeline_perf --dump-scene` 产 RXCS 装配 dump，步骤 3 =
//! 生产车道 `--wp-hlod full|on --wp-pack` 消费本工具产物）。
//!
//! 职责：RXCS 装配 dump → XZ 平面正方形 cell 网格（边长 = 资产属性
//! `--cell-size`，RXS-0363 L1 字面；三角按质心归属恰一 cell）→ 逐 cell
//! **跨组件（节点段）先合并再 QEM 简化**（#67/#97 字面：`bake_hlod_merged`
//! 事实源直调——L0 = 逐 Component 全量，L≥1 = 合并 + 逐层 QEM 减半）→
//! RXHL v1 资产字节 + digest（RXS-0364 产物即资产，digest 寻址）→ RXWH v1
//! cell 包落盘。quad 灯面尾段与 emissive 三角恒 passthrough 不参与 cell
//! 归属（光源几何面 0-byte——与 g31_cluster_lod_bake 同律）。
//!
//! 依赖方向说明：WP/HLOD 运行时机核（`PartitionRuntime`/`HlodRuntime`）在
//! rurix-render，本 crate 与其互不依赖——交接经 RXWH 文件（cell 表 +
//! RXHL 资产字节），运行时侧自构 `PersistentWorld` 并以 sha256(RXHL 字节)
//! 对 digest 核验（`canon::digest_bytes` 与 rurix-pkg sha256 同为标准
//! SHA-256，跨 crate 位级同源）。
//!
//! 确定性：cell 序 = (cy, cx) 升序（partition canonical 序同律）；cell 内
//! Component 序/三角序经 `bake_hlod_merged` canonical 化（声明序扰动免疫）；
//! `--double-build` 自校验臂（两次 bake 字节相等，fail-closed）。
//!
//! 用法：
//!   g31_wp_hlod_bake --scene-dump <scene.rxcs> --out <pack.rxwh> \
//!     [--cell-size 8.0] [--levels 4] [--double-build]

use std::path::Path;

use rurix_asset::hlod::{
    ComponentGeometry, HlodBakeInput, bake_hlod_merged, encode_hlod_asset, hlod_asset_digest,
};

const TAG: &str = "[g31_wp_hlod_bake]";
const RXCS_MAGIC: &[u8; 4] = b"RXCS";
const RXWH_MAGIC: &[u8; 4] = b"RXWH";
/// 无材质三角哨兵（g14_3_lane_body `SLAB_TRI_NONE` 同字面）。
const MAT_NONE: u32 = u32::MAX;

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

// ---------------------------------------------------------------------------
// RXCS 读取（g14_3_lane_body::dump_scene_rxcs writer 逐字段镜像；与
// g31_cluster_lod_bake 同一读取面——bin-local 各自持有，交接格式为界）
// ---------------------------------------------------------------------------

struct SceneDump {
    gltf_sha256: String,
    tris: Vec<[[f32; 3]; 3]>,
    albedo: Vec<[f32; 3]>,
    emission: Vec<[f32; 3]>,
    tri_mat: Vec<u32>,
    /// (tri_offset, tri_count, is_light_tail)。
    groups: Vec<(u32, u32, bool)>,
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
    if version != 1 {
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
    if c.p != bytes.len() {
        fail(&format!("RXCS 尾部冗余字节（pos {} ≠ len {}）", c.p, bytes.len()));
    }
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
    }
}

// ---------------------------------------------------------------------------
// cell 归属（XZ 网格；质心 f64 精算 floor 归格——确定性）
// ---------------------------------------------------------------------------

/// 三角质心 XZ（f64 精算——归格确定性）。
fn tri_centroid_xz(t: &[[f32; 3]; 3]) -> (f64, f64) {
    let cx = (t[0][0] as f64 + t[1][0] as f64 + t[2][0] as f64) / 3.0;
    let cz = (t[0][2] as f64 + t[1][2] as f64 + t[2][2] as f64) / 3.0;
    (cx, cz)
}

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

/// 单 cell bake 产物。
struct CellBake {
    /// 源三角 id（升序；Full 内容重建面）。
    src: Vec<u32>,
    /// 世界 y 高度范围（cell 包围盒第三维；partition bounds z 位）。
    y_min: f32,
    y_max: f32,
    /// cell 代理属性（面积加权 albedo 均值 + mat 众数；emission 恒 0 断言）。
    albedo: [f32; 3],
    mat: u32,
    /// RXHL v1 资产字节（`encode_hlod_asset` 产物；L0 全量 + L≥1 合并 QEM 链）。
    rxhl: Vec<u8>,
    /// `hlod_asset_digest`（运行时 `register_loaded_asset` 核验面）。
    digest: [u8; 32],
}

struct BakeResult {
    bytes: Vec<u8>,
    grid: (i32, i32, i32, i32),
    cells_total: usize,
    cells_nonempty: usize,
    cell_tris_min: usize,
    cell_tris_max: usize,
    passthrough: usize,
    /// 逐层代理三角总数（L1..levels-1；远景收益登记面）。
    proxy_tris_per_level: Vec<usize>,
}

fn bake(dump: &SceneDump, cell_size_m: f64, levels: u32, out: &Path) -> BakeResult {
    // ── 三角归属：passthrough（尾段 + emissive）与 cell 集合 ──
    let n = dump.tris.len();
    let mut passthrough: Vec<u32> = Vec::new();
    let mut cell_of: Vec<Option<(i32, i32)>> = vec![None; n];
    for &(off, cnt, is_tail) in &dump.groups {
        for t in off..off + cnt {
            if is_tail || dump.emission[t as usize] != [0.0, 0.0, 0.0] {
                passthrough.push(t);
            } else {
                let (cx, cz) = tri_centroid_xz(&dump.tris[t as usize]);
                let gx = (cx / cell_size_m).floor() as i32;
                let gy = (cz / cell_size_m).floor() as i32;
                cell_of[t as usize] = Some((gx, gy));
            }
        }
    }
    passthrough.sort_unstable();
    // ── 网格范围（稠密矩形;partition validate_world 同律）──
    let occupied: Vec<(i32, i32)> = cell_of.iter().flatten().copied().collect();
    if occupied.is_empty() {
        fail("全部三角 passthrough——cell 网格空（--cell-size 或场景异常）");
    }
    let gx0 = occupied.iter().map(|c| c.0).min().unwrap();
    let gx1 = occupied.iter().map(|c| c.0).max().unwrap();
    let gy0 = occupied.iter().map(|c| c.1).min().unwrap();
    let gy1 = occupied.iter().map(|c| c.1).max().unwrap();
    let ex = (gx1 - gx0) as i64 + 1;
    let ey = (gy1 - gy0) as i64 + 1;
    let cells_total = (ex * ey) as usize;
    if cells_total > 65536 {
        fail(&format!("cell 数 {cells_total} 超上界 65536（--cell-size 过小）"));
    }
    // ── 逐 cell 三角集合（(cy,cx) 升序 = partition canonical 序）──
    let mut cell_tris: Vec<Vec<u32>> = vec![Vec::new(); cells_total];
    for (t, c) in cell_of.iter().enumerate() {
        if let Some((gx, gy)) = c {
            let idx = ((gy - gy0) as i64 * ex + (gx - gx0) as i64) as usize;
            cell_tris[idx].push(t as u32);
        }
    }
    // ── 逐 cell bake（跨组件合并 + QEM 链;确定性 cell 序单线程——bistro
    //    规模 ~百万三角 QEM 分摊到 cell,壁钟可接受;并行归性能窗）──
    let mut cells: Vec<Option<CellBake>> = Vec::with_capacity(cells_total);
    let mut nonempty = 0usize;
    let mut tris_min = usize::MAX;
    let mut tris_max = 0usize;
    let mut proxy_tris_per_level = vec![0usize; levels as usize];
    for tris in &cell_tris {
        if tris.is_empty() {
            cells.push(None);
            continue;
        }
        nonempty += 1;
        tris_min = tris_min.min(tris.len());
        tris_max = tris_max.max(tris.len());
        // Component 划分：cell 内按节点段（#97「cell 内多 Actor 先合并再简化」
        // 的组件粒度 = 装配节点段）。段号经二分定位（组段升序连续已核验）。
        let seg_of = |t: u32| -> usize {
            dump.groups
                .partition_point(|&(off, _, _)| off <= t)
                .saturating_sub(1)
        };
        let mut comp_map: std::collections::BTreeMap<usize, Vec<[f32; 9]>> =
            std::collections::BTreeMap::new();
        let mut y_min = f32::INFINITY;
        let mut y_max = f32::NEG_INFINITY;
        let mut acc = [0.0f64; 3];
        let mut wsum = 0.0f64;
        let mut mats: Vec<u32> = Vec::new();
        for &t in tris {
            let tri = &dump.tris[t as usize];
            let mut flat = [0.0f32; 9];
            for k in 0..3 {
                flat[k * 3..k * 3 + 3].copy_from_slice(&tri[k]);
                y_min = y_min.min(tri[k][1]);
                y_max = y_max.max(tri[k][1]);
            }
            comp_map.entry(seg_of(t)).or_default().push(flat);
            if dump.emission[t as usize] != [0.0, 0.0, 0.0] {
                fail(&format!("cell 内 emissive 三角泄漏（源 {t}）——归属不变量破坏"));
            }
            let w = tri_area(tri).max(1e-12);
            for k in 0..3 {
                acc[k] += dump.albedo[t as usize][k] as f64 * w;
            }
            wsum += w;
            mats.push(dump.tri_mat[t as usize]);
        }
        let inv = if wsum > 0.0 { 1.0 / wsum } else { 0.0 };
        let albedo = [
            (acc[0] * inv) as f32,
            (acc[1] * inv) as f32,
            (acc[2] * inv) as f32,
        ];
        // mat 众数（值升序计数,同数取最小——g31_cluster_lod_bake 同律）。
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
        let input = HlodBakeInput {
            cell_name: format!("cell_{}", cells.len()),
            levels,
            components: comp_map
                .into_iter()
                .map(|(seg, triangles)| ComponentGeometry {
                    name: format!("seg_{seg}"),
                    triangles,
                })
                .collect(),
        };
        let asset =
            bake_hlod_merged(&input).unwrap_or_else(|e| fail(&format!("cell bake: {e}")));
        for (li, lv) in asset.levels.iter().enumerate() {
            proxy_tris_per_level[li] +=
                lv.proxies.iter().map(|p| p.proxy_triangles.len()).sum::<usize>();
        }
        cells.push(Some(CellBake {
            src: tris.clone(),
            y_min,
            y_max,
            albedo,
            mat: best.0,
            rxhl: encode_hlod_asset(&asset),
            digest: hlod_asset_digest(&asset),
        }));
    }
    // ── RXWH v1 写出（reader 逐字段镜像在 g14_3_lane_body）──
    let mut out_bytes: Vec<u8> = Vec::new();
    out_bytes.extend_from_slice(RXWH_MAGIC);
    out_bytes.extend_from_slice(&1u32.to_le_bytes());
    out_bytes.extend_from_slice(dump.gltf_sha256.as_bytes());
    out_bytes.extend_from_slice(&cell_size_m.to_bits().to_le_bytes());
    out_bytes.extend_from_slice(&gx0.to_le_bytes());
    out_bytes.extend_from_slice(&gy0.to_le_bytes());
    out_bytes.extend_from_slice(&gx1.to_le_bytes());
    out_bytes.extend_from_slice(&gy1.to_le_bytes());
    out_bytes.extend_from_slice(&levels.to_le_bytes());
    out_bytes.extend_from_slice(&(passthrough.len() as u32).to_le_bytes());
    for &p in &passthrough {
        out_bytes.extend_from_slice(&p.to_le_bytes());
    }
    out_bytes.extend_from_slice(&(cells_total as u32).to_le_bytes());
    for c in &cells {
        match c {
            None => out_bytes.extend_from_slice(&0u32.to_le_bytes()),
            Some(cb) => {
                out_bytes.extend_from_slice(&(cb.src.len() as u32).to_le_bytes());
                for &s in &cb.src {
                    out_bytes.extend_from_slice(&s.to_le_bytes());
                }
                out_bytes.extend_from_slice(&cb.y_min.to_bits().to_le_bytes());
                out_bytes.extend_from_slice(&cb.y_max.to_bits().to_le_bytes());
                for &x in &cb.albedo {
                    out_bytes.extend_from_slice(&x.to_bits().to_le_bytes());
                }
                out_bytes.extend_from_slice(&cb.mat.to_le_bytes());
                out_bytes.extend_from_slice(&cb.digest);
                out_bytes.extend_from_slice(&(cb.rxhl.len() as u32).to_le_bytes());
                out_bytes.extend_from_slice(&cb.rxhl);
            }
        }
    }
    std::fs::write(out, &out_bytes).unwrap_or_else(|e| fail(&format!("RXWH 写盘 {out:?}: {e}")));
    BakeResult {
        bytes: out_bytes,
        grid: (gx0, gy0, gx1, gy1),
        cells_total,
        cells_nonempty: nonempty,
        cell_tris_min: tris_min,
        cell_tris_max: tris_max,
        passthrough: passthrough.len(),
        proxy_tris_per_level,
    }
}

fn sha256_hex(data: &[u8]) -> String {
    let d = rurix_asset::canon::digest_bytes(data);
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
    let mut cell_size_m: f64 = 8.0;
    let mut levels: u32 = 4;
    let mut double_build = false;
    let mut i = 1;
    while i < args.len() {
        let take = |args: &[String], i: &mut usize| -> String {
            *i += 1;
            args.get(*i).unwrap_or_else(|| fail("缺参数值")).clone()
        };
        match args[i].as_str() {
            "--scene-dump" => scene_dump = take(&args, &mut i),
            "--out" => out_path = take(&args, &mut i),
            "--cell-size" => {
                cell_size_m = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--cell-size 非 f64"));
                if !(cell_size_m.is_finite() && cell_size_m > 0.0) {
                    fail("--cell-size 必须有限正");
                }
            }
            "--levels" => {
                levels = take(&args, &mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--levels 非 u32"));
                if !(2..=8).contains(&levels) {
                    fail("--levels 越闭集(2..=8——L0 全量 + 至少一层代理)");
                }
            }
            "--double-build" => double_build = true,
            other => fail(&format!("未知参数 {other}")),
        }
        i += 1;
    }
    if scene_dump.is_empty() || out_path.is_empty() {
        fail("参数闭集缺行（--scene-dump / --out）");
    }
    let dump = read_scene_dump(Path::new(&scene_dump));
    eprintln!(
        "{TAG}: RXCS 装载 tris={} groups={} sha={}",
        dump.tris.len(),
        dump.groups.len(),
        &dump.gltf_sha256[..16],
    );
    let t0 = std::time::Instant::now();
    let r = bake(&dump, cell_size_m, levels, Path::new(&out_path));
    let bake_ms = t0.elapsed().as_secs_f64() * 1e3;
    if double_build {
        let tmp = std::env::temp_dir().join(format!(
            "g31_wp_hlod_bake_double_{}.rxwh",
            std::process::id()
        ));
        let r2 = bake(&dump, cell_size_m, levels, &tmp);
        let equal = r.bytes == r2.bytes;
        let _ = std::fs::remove_file(&tmp);
        if !equal {
            fail("double-build 字节不等（确定性破坏）");
        }
        eprintln!("{TAG}: double-build 字节相等 OK");
    }
    println!(
        "{TAG}: bake OK grid=[{},{}]..[{},{}] cells={}/{} cell_size_m={} levels={} cell_tris=[{},{}] passthrough={} proxy_tris={:?} bytes={} sha256={} bake_ms={:.1} -> {}",
        r.grid.0,
        r.grid.1,
        r.grid.2,
        r.grid.3,
        r.cells_nonempty,
        r.cells_total,
        cell_size_m,
        levels,
        r.cell_tris_min,
        r.cell_tris_max,
        r.passthrough,
        r.proxy_tris_per_level,
        r.bytes.len(),
        sha256_hex(&r.bytes),
        bake_ms,
        out_path,
    );
}
