//! G9.2 M90 cluster_dag_deepening 探针——输出单行 JSON checks 供
//! `ci/g9_cluster_dag_deepening_smoke.py` 消费(RXS-0345;RFC-0022 §4.1)。
//!
//! 用法:
//!   cargo run -p rurix-asset --bin g9_m90_probe -- \
//!     --golden-dir tests/virtual_geometry/golden
//!   cargo run -p rurix-asset --bin g9_m90_probe -- --write-fixtures
//!
//! 腿:双构建 canonical 字节相等 / 逐边单调机器核验 / 破坏单调性 fixture
//! typed Err 拒录 / 蒙皮元数据三字段 roundtrip(含缺字段 RED)/ CLAS 烘焙
//! 输入 roundtrip / 非 M01 替代自检。

#![forbid(unsafe_code)]

use rurix_asset::{concatenate_pages_v2, pack_cluster_dag_v2};
use rurix_geom_build::{
    DagAsset, DagError, SkinWeights, TriMesh, build_asset_dag, canonical_bytes,
    validate_monotonicity,
};
use rurix_geom_pages::{decode_logical_page_v2, encode_logical_page_v2};
use rurix_pkg::sha256;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    let write_fixtures = args.iter().any(|a| a == "--write-fixtures");
    let root = workspace_root();
    let golden_dir = arg_path(&args, "--golden-dir")
        .unwrap_or_else(|| root.join("tests/virtual_geometry/golden"));

    // 固定 mesh 语料(条款:固定 mesh 语料两次独立构建 canonical 字节相等)。
    let mesh = || TriMesh::uv_sphere(1.0, 24, 24);
    let asset_a = DagAsset::static_mesh(mesh());
    let asset_b = DagAsset::static_mesh(mesh());
    let dag_a = build_asset_dag(&asset_a).expect("build a");
    let dag_b = build_asset_dag(&asset_b).expect("build b");
    let canon_a = canonical_bytes(&dag_a.base);
    let canon_b = canonical_bytes(&dag_b.base);
    let double_ok = canon_a == canon_b;
    let canon_digest = sha256::hex_digest(&canon_a);

    // 逐边单调机器核验(全边枚举;核验器自身 = validate_monotonicity 全真)。
    let mut edge_total = 0usize;
    let mut monotonic_ok = validate_monotonicity(&dag_a.base).is_ok();
    for parent in 0..dag_a.base.records.len() as u32 {
        let pe = dag_a.base.record(parent).error;
        for &child in dag_a.base.children_of(parent) {
            edge_total += 1;
            let ce = dag_a.base.record(child).error;
            if !matches!(
                pe.partial_cmp(&ce),
                Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
            ) {
                monotonic_ok = false;
            }
        }
    }

    // 破坏单调性 fixture(真实生成腿):合法 DAG 副本压父 error 到子以下 →
    // builder 核验 typed Err 拒录(fail-closed;非 panic、非静默钳制)。
    let mut broken = dag_a.base.clone();
    let mut broken_edge = None;
    for parent in 0..broken.records.len() as u32 {
        let children = broken.children_of(parent).to_vec();
        if children.is_empty() {
            continue;
        }
        let child = children[0];
        let ce = broken.records[child as usize].error;
        if ce > 0.0 {
            broken.records[parent as usize].error = ce * 0.5;
            broken_edge = Some((parent, child));
            break;
        }
    }
    let reject_ok = match broken_edge {
        Some((p, c)) => matches!(
            validate_monotonicity(&broken),
            Err(DagError::NonMonotonicEdge {
                parent,
                child,
                ..
            }) if parent == p && child == c
        ),
        None => false,
    };

    // 蒙皮元数据 roundtrip(三字段经 v2 页编码→解码逐位回读)。
    let n_verts = mesh().positions.len();
    let influences: Vec<Vec<(u32, f32)>> = (0..n_verts)
        .map(|i| {
            if i % 3 == 0 {
                vec![(0u32, 0.5f32), (2u32, 0.5f32)]
            } else {
                vec![(1u32, 1.0f32)]
            }
        })
        .collect();
    let skinned_asset = DagAsset {
        mesh: mesh(),
        skinned: Some(SkinWeights {
            vertex_influences: influences,
            joint_count: 8,
        }),
    };
    let skinned_dag = build_asset_dag(&skinned_asset).expect("skinned build");
    let skinned_pages = pack_cluster_dag_v2(&skinned_dag).expect("pack skinned");
    let mut skin_roundtrip = true;
    for p in &skinned_pages {
        let bytes = encode_logical_page_v2(p);
        let back = decode_logical_page_v2(&bytes).expect("v2 decode");
        for (c, e) in back.base.clusters.iter().zip(back.ext.iter()) {
            let src = &skinned_dag.skin[c.cluster_id as usize];
            let want_bones = src.bone_indices.clone().unwrap_or_default();
            let want_infl = src.bound_inflation.unwrap_or(0.0);
            if e.max_influences != src.max_influences
                || e.bone_indices != want_bones
                || e.bound_inflation.to_bits() != want_infl.to_bits()
            {
                skin_roundtrip = false;
            }
        }
    }
    // 缺蒙皮字段 RED(权重行缺失 → typed Err)。
    let short = DagAsset {
        mesh: mesh(),
        skinned: Some(SkinWeights {
            vertex_influences: vec![vec![(0u32, 1.0f32)]; n_verts - 1],
            joint_count: 8,
        }),
    };
    let skin_missing_rejected = matches!(
        build_asset_dag(&short),
        Err(DagError::SkinMetadataMissing { .. })
    );
    let skin_ok = skin_roundtrip && skin_missing_rejected;

    // CLAS 烘焙输入 roundtrip(三角形簇几何引用 + 簇级 AABB 经 v2 页回读)。
    let static_pages = pack_cluster_dag_v2(&dag_a).expect("pack static");
    let mut clas_roundtrip = true;
    for p in &static_pages {
        let bytes = encode_logical_page_v2(p);
        let back = decode_logical_page_v2(&bytes).expect("v2 decode clas");
        for (c, e) in back.base.clusters.iter().zip(back.ext.iter()) {
            let src = &dag_a.clas[c.cluster_id as usize];
            if e.aabb_min.map(f32::to_bits) != src.aabb_min.map(f32::to_bits)
                || e.aabb_max.map(f32::to_bits) != src.aabb_max.map(f32::to_bits)
            {
                clas_roundtrip = false;
            }
            // 三角形簇面:簇级 AABB ⊇ 簇顶点(逐位)。
            for v in dag_a.base.cluster_vertices(c.cluster_id) {
                for (k, &x) in v.iter().enumerate() {
                    if !(e.aabb_min[k] <= x && x <= e.aabb_max[k]) {
                        clas_roundtrip = false;
                    }
                }
            }
        }
    }

    // golden 比对(双构建 digest manifest)。
    let manifest_path = golden_dir.join("m90_dag_digest_manifest.json");
    let mut golden_ok = false;
    if manifest_path.is_file() {
        let text = fs::read_to_string(&manifest_path).unwrap();
        golden_ok = text.contains(&canon_digest);
    }

    if write_fixtures {
        fs::create_dir_all(&golden_dir).unwrap();
        let pages_digest = sha256::hex_digest(&concatenate_pages_v2(&static_pages));
        let manifest = format!(
            "{{\n  \"schema_version\": 1,\n  \"input\": \"TriMesh::uv_sphere(1.0,24,24) build_asset_dag(static)\",\n  \"canonical_dag_sha256\": \"{canon_digest}\",\n  \"pages_v2_concat_sha256\": \"{pages_digest}\",\n  \"page_count\": {},\n  \"cluster_count\": {},\n  \"edge_count\": {edge_total}\n}}\n",
            static_pages.len(),
            dag_a.base.records.len()
        );
        fs::write(manifest_path, manifest).unwrap();
        eprintln!("[g9_m90_probe] golden manifest written: {golden_dir:?}");
    }

    // 非 M01 替代自检:M90 面 = typed Err 拒录 + 蒙皮/CLAS 字段;M01 静态 DAG
    // 输出(仅双构建 + 头 golden)不含本面,故本腿显式要求 v2 扩展在场。
    let not_m01 = !static_pages.is_empty()
        && static_pages
            .iter()
            .all(|p| p.ext.len() == p.base.clusters.len())
        && reject_ok;

    let checks: [(&str, bool); 6] = [
        ("double_build_byte_equal", double_ok),
        ("monotonic_edge_check", monotonic_ok),
        ("monotonic_break_fixture_rejected", reject_ok),
        ("skin_metadata_roundtrip", skin_ok),
        ("clas_bake_input_roundtrip", clas_roundtrip),
        ("not_substituted_by_m01", not_m01),
    ];
    let all_ok = checks.iter().all(|(_, v)| *v) && golden_ok;

    print!("{{");
    print!("\"ok\":{all_ok},");
    print!("\"canonical_dag_sha256\":\"{canon_digest}\",");
    print!("\"edge_count\":{edge_total},");
    print!("\"cluster_count\":{},", dag_a.base.records.len());
    print!("\"golden_manifest_match\":{golden_ok},");
    print!("\"skin_missing_rejected\":{skin_missing_rejected},");
    print!("\"checks\":{{");
    for (i, (k, v)) in checks.iter().enumerate() {
        if i > 0 {
            print!(",");
        }
        print!("\"{k}\":{v}");
    }
    print!("}}");
    println!("}}");

    if !all_ok {
        eprintln!("[g9_m90_probe] FAIL checks:");
        for (k, v) in &checks {
            if !*v {
                eprintln!("  - {k}");
            }
        }
        if !golden_ok {
            eprintln!("  - golden_manifest_match(m90_dag_digest_manifest.json)");
        }
        std::process::exit(1);
    }
}

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop(); // src
    p.pop(); // repo root
    p
}

fn arg_path(args: &[String], key: &str) -> Option<PathBuf> {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| Path::new(&w[1]).to_path_buf())
}
