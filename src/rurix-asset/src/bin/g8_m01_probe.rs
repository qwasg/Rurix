//! G8.3 M01 meshlet_page_builder 探针——输出 JSON checks 供 smoke 消费。
//!
//! 用法:
//!   cargo run -p rurix-asset --bin g8_m01_probe -- \
//!     --golden-dir tests/geom_pages/golden \
//!     --reject-dir conformance/geom_pages/reject
//!   cargo run -p rurix-asset --bin g8_m01_probe -- --write-fixtures

#![forbid(unsafe_code)]

use rurix_asset::{concatenate_pages, pack_cluster_dag, rxgb_to_pages};
use rurix_geom_build::{TriMesh, build_dag, write_dag};
use rurix_geom_pages::{
    HEADER_SIZE, LOGICAL_MAJOR, PageDecodeError, STREAM_PAGE_SIZE, decode_logical_page,
    encode_logical_page, schema_digest,
};
use rurix_pkg::sha256;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    let write_fixtures = args.iter().any(|a| a == "--write-fixtures");
    let root = workspace_root();
    let golden_dir = arg_path(&args, "--golden-dir")
        .unwrap_or_else(|| root.join("tests/geom_pages/golden"));
    let reject_dir = arg_path(&args, "--reject-dir")
        .unwrap_or_else(|| root.join("conformance/geom_pages/reject"));

    let dag = build_dag(&TriMesh::uv_sphere(1.0, 24, 24));
    let pages_a = pack_cluster_dag(&dag).expect("pack a");
    let pages_b = pack_cluster_dag(&dag).expect("pack b");
    let bytes_a = concatenate_pages(&pages_a);
    let bytes_b = concatenate_pages(&pages_b);

    let header = &encode_logical_page(&pages_a[0])[..HEADER_SIZE as usize];
    let schema = schema_digest();
    let schema_hex = sha256::hex(&schema);
    let pages_digest = sha256::hex_digest(&bytes_a);

    if write_fixtures {
        fs::create_dir_all(&golden_dir).unwrap();
        fs::create_dir_all(&reject_dir).unwrap();
        fs::write(golden_dir.join("m01_header.bin"), header).unwrap();
        let manifest = format!(
            "{{\n  \"schema_version\": 1,\n  \"input\": \"TriMesh::uv_sphere(1.0,24,24)\",\n  \"header_size\": {HEADER_SIZE},\n  \"schema_digest_hex\": \"{schema_hex}\",\n  \"pages_concat_sha256\": \"{pages_digest}\",\n  \"page_count\": {},\n  \"stream_page_size\": {STREAM_PAGE_SIZE}\n}}\n",
            pages_a.len()
        );
        fs::write(golden_dir.join("m01_digest_manifest.json"), manifest).unwrap();

        // reject fixtures from a valid encoded page
        let mut valid = encode_logical_page(&pages_a[0]);
        let mut unknown = valid.clone();
        unknown[8] = 9;
        unknown[9] = 0;
        // 故意不重算 digest——decode 必须在段消费前因 major 拒录
        fs::write(reject_dir.join("unknown_version.rxpl"), &unknown).unwrap();

        let mut bad_magic = valid.clone();
        bad_magic[0] = b'B';
        bad_magic[1] = b'A';
        bad_magic[2] = b'D';
        bad_magic[3] = b'!';
        fs::write(reject_dir.join("bad_magic.rxpl"), &bad_magic).unwrap();

        let trunc = &valid[..40.min(valid.len())];
        fs::write(reject_dir.join("truncated.rxpl"), trunc).unwrap();

        // 防止 unused
        let _ = &mut valid;
        eprintln!("[g8_m01_probe] fixtures written under {golden_dir:?} / {reject_dir:?}");
    }

    let mut checks: Vec<(&str, bool)> = Vec::new();

    // 1 double_build
    checks.push(("builder_double_run_byte_equal", bytes_a == bytes_b));

    // 2 header golden
    let golden_header_path = golden_dir.join("m01_header.bin");
    let header_ok = if golden_header_path.is_file() {
        let g = fs::read(&golden_header_path).unwrap();
        g == header
    } else {
        false
    };
    checks.push(("header_magic_version_golden", header_ok));

    // 3 schema digest golden
    let manifest_path = golden_dir.join("m01_digest_manifest.json");
    let schema_ok = if manifest_path.is_file() {
        let text = fs::read_to_string(&manifest_path).unwrap();
        text.contains(&schema_hex)
            && header[72..104] == schema
    } else {
        false
    };
    checks.push(("header_schema_digest_golden", schema_ok));

    // 4-7 decode reference
    let (nodes_ok, edges_ok, bounds_ok, lod_ok) = compare_to_dag(&dag, &pages_a);
    checks.push(("decoded_dag_nodes_equal_reference", nodes_ok));
    checks.push(("decoded_dag_edges_equal_reference", edges_ok));
    checks.push(("decoded_bounds_equal_reference", bounds_ok));
    checks.push(("decoded_lod_parent_equal_reference", lod_ok));

    // 8 page size
    let size_ok = pages_a.iter().all(|p| {
        encode_logical_page(p).len() <= STREAM_PAGE_SIZE as usize
    });
    checks.push(("page_size_within_contract", size_ok));

    // 9 unknown version rejected pre-consume
    let unknown_path = reject_dir.join("unknown_version.rxpl");
    let unknown_ok = if unknown_path.is_file() {
        let b = fs::read(&unknown_path).unwrap();
        matches!(
            decode_logical_page(&b),
            Err(PageDecodeError::UnsupportedVersion { major: 9, .. })
        )
    } else {
        false
    };
    checks.push(("unknown_version_rejected_pre_consume", unknown_ok));

    // also bad_magic + truncated for corpus completeness (folded into unknown leg notes)
    let bad_magic_ok = reject_dir.join("bad_magic.rxpl").is_file()
        && matches!(
            decode_logical_page(&fs::read(reject_dir.join("bad_magic.rxpl")).unwrap()),
            Err(PageDecodeError::BadMagic)
        );
    let trunc_ok = reject_dir.join("truncated.rxpl").is_file()
        && matches!(
            decode_logical_page(&fs::read(reject_dir.join("truncated.rxpl")).unwrap()),
            Err(PageDecodeError::Truncated(_)) | Err(PageDecodeError::BadMagic)
        );

    // 10 rxgb converter
    let rxgb = write_dag(&dag);
    let conv = rxgb_to_pages(&rxgb);
    let mut bad = rxgb.clone();
    bad[0] = b'X';
    let conv_bad = rxgb_to_pages(&bad);
    let converter_ok = conv.is_ok()
        && conv.as_ref().unwrap().len() == pages_a.len()
        && concatenate_pages(conv.as_ref().unwrap()) == bytes_a
        && conv_bad.is_err();
    checks.push(("rxgb_converter_explicit", converter_ok));

    // 11 rxgb reader zero-byte: 由 smoke 跑 geom-build 单测；探针侧验证 roundtrip 仍可用
    let round = rurix_geom_build::read_dag(&rxgb).expect("rxgb roundtrip");
    let rxgb2 = write_dag(&round);
    checks.push(("rxgb_reader_zero_byte", rxgb == rxgb2));

    // 12 not substituted by M04
    let not_m04 = !bytes_a.windows(4).any(|w| w == b"RXPD" || w == b"RXPM")
        && pages_a.iter().all(|p| {
            let b = encode_logical_page(p);
            &b[0..4] == b"RXPL" && u16::from_le_bytes([b[8], b[9]]) == LOGICAL_MAJOR
        });
    checks.push(("not_substituted_by_m04", not_m04));

    let all_ok = checks.iter().all(|(_, v)| *v) && bad_magic_ok && trunc_ok;

    // JSON stdout
    print!("{{");
    print!("\"ok\":{},", all_ok);
    print!("\"page_count\":{},", pages_a.len());
    print!("\"schema_digest_hex\":\"{schema_hex}\",");
    print!("\"pages_concat_sha256\":\"{pages_digest}\",");
    print!("\"bad_magic_rejected\":{},", bad_magic_ok);
    print!("\"truncated_rejected\":{},", trunc_ok);
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
        eprintln!("[g8_m01_probe] FAIL checks:");
        for (k, v) in &checks {
            if !*v {
                eprintln!("  - {k}");
            }
        }
        if !bad_magic_ok {
            eprintln!("  - bad_magic_rejected");
        }
        if !trunc_ok {
            eprintln!("  - truncated_rejected");
        }
        std::process::exit(1);
    }
}

fn compare_to_dag(
    dag: &rurix_geom_build::ClusterDag,
    pages: &[rurix_geom_pages::LogicalPage],
) -> (bool, bool, bool, bool) {
    let mut nodes = HashMap::new();
    let mut edges = HashSet::new();
    let mut bounds = HashMap::new();
    for p in pages {
        let d = decode_logical_page(&encode_logical_page(p)).unwrap();
        for c in &d.clusters {
            nodes.insert(c.cluster_id, (c.level, c.group));
            bounds.insert(
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
    let nodes_ok = nodes.len() == dag.records.len()
        && (0..dag.records.len() as u32).all(|id| {
            let n = dag.node(id);
            nodes.get(&id) == Some(&(n.level, n.group))
        });
    let bounds_ok = (0..dag.records.len() as u32).all(|id| {
        let r = dag.record(id);
        bounds.get(&id)
            == Some(&(
                r.center.map(f32::to_bits),
                r.radius.to_bits(),
                r.cone_axis.map(f32::to_bits),
                r.cone_cutoff.to_bits(),
                r.error.to_bits(),
                r.parent_error.to_bits(),
            ))
    });
    let mut ref_edges = HashSet::new();
    let mut lod_parents: HashMap<u32, HashSet<u32>> = HashMap::new();
    for parent in 0..dag.records.len() as u32 {
        for &child in dag.children_of(parent) {
            ref_edges.insert((parent, child));
            lod_parents.entry(child).or_default().insert(parent);
        }
    }
    let edges_ok = edges == ref_edges;
    let mut got_parents: HashMap<u32, HashSet<u32>> = HashMap::new();
    for &(p, c) in &edges {
        got_parents.entry(c).or_default().insert(p);
    }
    let lod_ok = got_parents == lod_parents;
    (nodes_ok, edges_ok, bounds_ok, lod_ok)
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
