//! G9.2 M91 page_format_v2_abi host 探针(RXS-0344;RFC-0022 §4.5)。
//!
//! 用法:
//!   cargo run -p rurix-asset --bin g9_m91_probe -- \
//!     --golden-dir tests/geom_pages/golden \
//!     --reject-dir conformance/geom_pages/reject
//!   cargo run -p rurix-asset --bin g9_m91_probe -- --write-fixtures ...
//!
//! 腿:v2 ABI id/version 与 v1 不同且冻结 / v1 0-byte 回归 digest /
//! encode→decode 往返逐字节 + canonical records 对 golden / 篡改 digest
//! fail-closed / 未知 major fail-closed / CPU 展开 digest(device 对照源)。

#![forbid(unsafe_code)]

use rurix_asset::{concatenate_pages, concatenate_pages_v2, pack_cluster_dag, pack_cluster_dag_v2};
use rurix_geom_build::{DagAsset, TriMesh, build_asset_dag};
use rurix_geom_pages::logical_v2::{
    HEADER_SIZE_V2, LOGICAL_MAJOR_V2, LOGICAL_MINOR_V2, decode_logical_page_v2, schema_digest_v2,
};
use rurix_geom_pages::{
    HEADER_SIZE, LOGICAL_MAJOR, LOGICAL_MINOR, PageDecodeError, decode_logical_page_any,
    encode_logical_page_v2, expand_logical_page_v2, expand_u32_count_v2, expanded_digest_v2,
};
use rurix_pkg::sha256;
use std::env;
use std::fs;
use std::path::PathBuf;

fn hex32(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let root = workspace_root();
    let mut golden_dir = root.join("tests/geom_pages/golden");
    let mut reject_dir = root.join("conformance/geom_pages/reject");
    let mut write_fixtures = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--golden-dir" => {
                i += 1;
                golden_dir = PathBuf::from(&args[i]);
            }
            "--reject-dir" => {
                i += 1;
                reject_dir = PathBuf::from(&args[i]);
            }
            "--write-fixtures" => write_fixtures = true,
            _ => {}
        }
        i += 1;
    }

    let dag = build_asset_dag(&DagAsset::static_mesh(TriMesh::uv_sphere(1.0, 24, 24)))
        .expect("build dag v2");
    let pages_v2 = pack_cluster_dag_v2(&dag).expect("pack v2");
    let page0 = &pages_v2[0];
    let page0_bytes = encode_logical_page_v2(page0);

    // 1) ABI id/version:v2 与 v1 不同且冻结(字面值 + 双 schema digest 不等)。
    let abi_ok = LOGICAL_MAJOR_V2 == 2
        && LOGICAL_MAJOR_V2 != LOGICAL_MAJOR
        && LOGICAL_MINOR_V2 == 0
        && LOGICAL_MINOR == 0
        && HEADER_SIZE_V2 == 160
        && HEADER_SIZE == 136
        && schema_digest_v2() != rurix_geom_pages::schema_digest()
        && page0_bytes[8..10] == 2u16.to_le_bytes();

    // 2) v1 0-byte 回归:v1 打包/编码路径 digest 与 M01 golden manifest 一致。
    let v1_pages = pack_cluster_dag(&dag.base).expect("pack v1");
    let v1_concat = concatenate_pages(&v1_pages);
    let v1_digest = sha256::hex_digest(&v1_concat);
    let m01_manifest = golden_dir.join("m01_digest_manifest.json");
    let v1_regression_ok = m01_manifest.is_file()
        && fs::read_to_string(&m01_manifest)
            .unwrap()
            .contains(&v1_digest);

    // 3) encode→decode 往返无损 + canonical records 与 golden 逐字节相等。
    let mut roundtrip_ok = true;
    for p in &pages_v2 {
        let b = encode_logical_page_v2(p);
        let back = decode_logical_page_v2(&b).expect("v2 decode");
        if back != *p || encode_logical_page_v2(&back) != b {
            roundtrip_ok = false;
        }
    }
    let golden_page = golden_dir.join("m91_page_v2.rxpl");
    let golden_ok = golden_page.is_file() && fs::read(&golden_page).unwrap() == page0_bytes;

    // 4) 篡改 digest fail-closed(schema/section 两臂;fixture 落 reject 语料)。
    let mut bad_schema = page0_bytes.clone();
    bad_schema[72] ^= 0x01;
    let schema_rej = matches!(
        decode_logical_page_v2(&bad_schema),
        Err(PageDecodeError::DigestMismatch("schema_digest"))
    );
    let mut bad_section = page0_bytes.clone();
    bad_section[104] ^= 0x01;
    let section_rej = matches!(
        decode_logical_page_v2(&bad_section),
        Err(PageDecodeError::DigestMismatch("section_digest"))
    );
    // reject 语料文件(若已落)必须同被 v2 loader 与 any 分发器拒录。
    let mut corpus_rej = true;
    for (name, expect_schema) in [
        ("v2_tampered_schema_digest.rxpl", true),
        ("v2_tampered_section_digest.rxpl", false),
    ] {
        let p = reject_dir.join(name);
        if p.is_file() {
            let b = fs::read(&p).unwrap();
            let r = decode_logical_page_v2(&b);
            let ok = if expect_schema {
                matches!(r, Err(PageDecodeError::DigestMismatch("schema_digest")))
            } else {
                matches!(r, Err(PageDecodeError::DigestMismatch("section_digest")))
            };
            corpus_rej = corpus_rej && ok && decode_logical_page_any(&b).is_err();
        } else {
            corpus_rej = false;
        }
    }
    let corrupt_ok = schema_rej && section_rej && corpus_rej;

    // 5) 未知 major fail-closed(v2 loader + any 分发器双臂)。
    let mut unk = page0_bytes.clone();
    unk[8] = 9;
    unk[9] = 0;
    let unknown_ok = matches!(
        decode_logical_page_v2(&unk),
        Err(PageDecodeError::UnsupportedVersion { major: 9, .. })
    ) && matches!(
        decode_logical_page_any(&unk),
        Err(PageDecodeError::UnsupportedVersion { major: 9, .. })
    ) && decode_logical_page_any(&page0_bytes).is_ok();

    // 6) CPU 展开 digest(device 对照源;smoke 侧与 device 比对)。
    let stream = expand_logical_page_v2(page0);
    let cpu_digest = expanded_digest_v2(page0);
    let cpu_stable = cpu_digest == expanded_digest_v2(page0);
    let expand_n = expand_u32_count_v2(page0);
    debug_assert_eq!(stream.len(), expand_n * 4);

    if write_fixtures {
        fs::create_dir_all(&golden_dir).ok();
        fs::create_dir_all(&reject_dir).ok();
        fs::write(&golden_page, &page0_bytes).expect("write m91_page_v2.rxpl");
        let concat = concatenate_pages_v2(&pages_v2);
        let manifest = format!(
            "{{\n  \"schema_version\": 1,\n  \"input\": \"TriMesh::uv_sphere(1.0,24,24) pack_cluster_dag_v2 page0\",\n  \"v2_schema_digest_hex\": \"{}\",\n  \"expanded_digest_hex\": \"{}\",\n  \"expanded_u32_count\": {},\n  \"page0_sha256\": \"{}\",\n  \"pages_v2_concat_sha256\": \"{}\",\n  \"page0_len\": {},\n  \"page_count\": {},\n  \"v1_concat_sha256\": \"{}\"\n}}\n",
            hex32(&schema_digest_v2()),
            hex32(&cpu_digest),
            expand_n,
            hex32(&sha256::digest(&page0_bytes)),
            hex32(&sha256::digest(&concat)),
            page0_bytes.len(),
            pages_v2.len(),
            v1_digest
        );
        fs::write(golden_dir.join("m91_digest_manifest.json"), manifest).expect("manifest");
        fs::write(
            reject_dir.join("v2_tampered_schema_digest.rxpl"),
            &bad_schema,
        )
        .ok();
        fs::write(
            reject_dir.join("v2_tampered_section_digest.rxpl"),
            &bad_section,
        )
        .ok();
        eprintln!("[g9_m91_probe] fixtures written: {golden_dir:?} / {reject_dir:?}");
    }

    let checks: [(&str, bool); 6] = [
        ("abi_v2_ids_distinct_and_frozen", abi_ok),
        ("v1_abi_zero_byte_regression", v1_regression_ok),
        (
            "encode_decode_roundtrip_byte_equal",
            roundtrip_ok && golden_ok,
        ),
        ("corrupt_v2_digest_fail_closed", corrupt_ok),
        ("unknown_major_fail_closed", unknown_ok),
        ("cpu_decode_digest_stable", cpu_stable),
    ];
    let all_ok = checks.iter().all(|(_, v)| *v);

    print!("{{");
    print!("\"ok\":{all_ok},");
    print!("\"expanded_digest\":\"{}\",", hex32(&cpu_digest));
    print!("\"expanded_u32_count\":{expand_n},");
    print!("\"v2_schema_digest\":\"{}\",", hex32(&schema_digest_v2()));
    print!("\"v1_concat_sha256\":\"{v1_digest}\",");
    print!("\"golden_v2_page_byte_equal\":{golden_ok},");
    print!("\"page0_len\":{},", page0_bytes.len());
    print!("\"page_count\":{},", pages_v2.len());
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
        eprintln!("[g9_m91_probe] FAIL checks:");
        for (k, v) in &checks {
            if !*v {
                eprintln!("  - {k}");
            }
        }
        std::process::exit(1);
    }
}

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}
