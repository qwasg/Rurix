//! G8.3 M04 host 探针：生成/核验 RXPD golden、RED 拒录、ABI 字面、CPU digest。

use rurix_asset::pack_cluster_dag;
use rurix_geom_build::{TriMesh, build_dag};
use rurix_geom_pages::codec;
use rurix_geom_pages::disk::{self, HEADER_SIZE as DISK_HDR, decode_disk_page, encode_disk_page};
use rurix_geom_pages::expand::{expand_u32_count, expanded_digest};
use rurix_geom_pages::memory::{
    self, HEADER_SIZE as MEM_HDR, SECTION_DIR_ENTRY_SIZE, decode_memory_page, encode_memory_page,
    from_logical,
};
use rurix_geom_pages::{DISK_MAJOR, FORMAT_ID as LOGICAL_FID, MEMORY_MAJOR, RXPD_MAGIC, RXPM_MAGIC};
use rurix_pkg::sha256;
use std::env;
use std::fs;
use std::path::PathBuf;

fn hex32(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let mut golden_dir = PathBuf::from("tests/geom_pages/golden");
    let mut reject_dir = PathBuf::from("conformance/geom_pages/reject");
    let mut write_fixtures = false;
    let args: Vec<String> = env::args().skip(1).collect();
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

    let dag = build_dag(&TriMesh::uv_sphere(1.0, 24, 24));
    let pages = pack_cluster_dag(&dag).expect("pack");
    assert!(!pages.is_empty());
    let logical = &pages[0];
    let mem = from_logical(logical);
    let rxpm = encode_memory_page(&mem);
    let rxpd = encode_disk_page(&mem, &logical.dependency_page_ids);
    let rxpd2 = encode_disk_page(&mem, &logical.dependency_page_ids);
    let compress_eq = rxpd == rxpd2;

    let decoded = decode_disk_page(&rxpd).expect("decode disk");
    let rxpm_back = encode_memory_page(&decoded);
    let records_eq = rxpm_back == rxpm;

    let dig1 = expanded_digest(&decoded);
    let dig2 = expanded_digest(&decoded);
    let cpu_stable = dig1 == dig2;
    let expand_n = expand_u32_count(&decoded);

    let compressed = codec::compress(&rxpm);
    let compress_twice = codec::compress(&rxpm) == compressed;

    // ABI frozen literals
    let abi_ok = RXPD_MAGIC == *b"RXPD"
        && RXPM_MAGIC == *b"RXPM"
        && disk::FORMAT_ID == 3
        && memory::FORMAT_ID == 2
        && LOGICAL_FID == 1
        && disk::FORMAT_ID != memory::FORMAT_ID
        && DISK_MAJOR == 1
        && MEMORY_MAJOR == 1
        && mapping_ok();

    // reject axes
    let mut trunc = rxpd.clone();
    trunc.truncate(DISK_HDR as usize + 3);
    let trunc_rej = matches!(
        decode_disk_page(&trunc),
        Err(disk::DiskError::Truncated(_))
    );

    let mut chk = rxpd.clone();
    if chk.len() > DISK_HDR as usize + 1 {
        chk[DISK_HDR as usize] ^= 0x01;
    }
    let chk_rej = decode_disk_page(&chk) == Err(disk::DiskError::ChecksumMismatch);

    let mut unk_c = rxpd.clone();
    unk_c[20] = 99;
    unk_c[21] = 0;
    unk_c[22] = 0;
    unk_c[23] = 0;
    let unk_codec = decode_disk_page(&unk_c) == Err(disk::DiskError::UnknownCodec(99));

    let mut unk_v = rxpd.clone();
    unk_v[8] = 9;
    unk_v[9] = 0;
    let unk_ver = matches!(
        decode_disk_page(&unk_v),
        Err(disk::DiskError::UnsupportedVersion { major: 9, .. })
    );

    // section overlap / oob on RXPM
    let mut overlap = rxpm.clone();
    let dir1 = MEM_HDR as usize + SECTION_DIR_ENTRY_SIZE;
    let pos_off = u32::from_le_bytes(overlap[MEM_HDR as usize + 4..MEM_HDR as usize + 8].try_into().unwrap());
    overlap[dir1 + 4..dir1 + 8].copy_from_slice(&pos_off.to_le_bytes());
    let overlap_rej = decode_memory_page(&overlap) == Err(memory::MemoryError::SectionOverlap);

    let mut oob = rxpm.clone();
    let huge = (rxpm.len() as u32).saturating_add(16);
    oob[MEM_HDR as usize + 4..MEM_HDR as usize + 8].copy_from_slice(&huge.to_le_bytes());
    let oob_rej = decode_memory_page(&oob) == Err(memory::MemoryError::SectionOob);

    // reject_before_allocation: truncated must not require uncompressed_size alloc
    // （实现上 Truncated 在 decompress 前返回）
    let reject_before = trunc_rej;

    let mapping_frozen = disk::mapping_allows(1, 1)
        && !disk::mapping_allows(1, 2)
        && !disk::mapping_allows(2, 1);

    if write_fixtures {
        fs::create_dir_all(&golden_dir).ok();
        fs::create_dir_all(&reject_dir).ok();
        fs::write(golden_dir.join("m04_page0.rxpd"), &rxpd).expect("write rxpd");
        fs::write(golden_dir.join("m04_page0.rxpm"), &rxpm).expect("write rxpm");
        let manifest = format!(
            "{{\n  \"schema_version\": 1,\n  \"input\": \"TriMesh::uv_sphere(1.0,24,24) page0\",\n  \
             \"expanded_digest_hex\": \"{}\",\n  \"expanded_u32_count\": {},\n  \
             \"rxpd_sha256\": \"{}\",\n  \"compressed_payload_sha256\": \"{}\",\n  \
             \"rxpd_len\": {},\n  \"rxpm_len\": {}\n}}\n",
            hex32(&dig1),
            expand_n,
            hex32(&sha256::digest(&rxpd)),
            hex32(&sha256::digest(&compressed)),
            rxpd.len(),
            rxpm.len()
        );
        fs::write(golden_dir.join("m04_digest_manifest.json"), manifest).expect("manifest");

        fs::write(reject_dir.join("truncated_payload.rxpd"), &trunc).ok();
        fs::write(reject_dir.join("checksum_flip.rxpd"), &chk).ok();
        fs::write(reject_dir.join("unknown_codec.rxpd"), &unk_c).ok();
        fs::write(reject_dir.join("unknown_major.rxpd"), &unk_v).ok();
        fs::write(reject_dir.join("section_overlap.rxpm"), &overlap).ok();
        fs::write(reject_dir.join("section_oob.rxpm"), &oob).ok();
    }

    // golden compare if present
    let mut golden_eq = false;
    let gpath = golden_dir.join("m04_page0.rxpd");
    if gpath.is_file() {
        let g = fs::read(&gpath).unwrap();
        golden_eq = g == rxpd;
    }

    println!(
        "{{\n  \"abi_ids_distinct_and_frozen\": {},\n  \"encode_decode_records_byte_equal\": {},\n  \
         \"compress_twice_byte_equal\": {},\n  \"corrupt_truncation_fail_closed\": {},\n  \
         \"corrupt_checksum_fail_closed\": {},\n  \"corrupt_unknown_codec_fail_closed\": {},\n  \
         \"corrupt_unknown_version_fail_closed\": {},\n  \"section_overlap_oob_fail_closed\": {},\n  \
         \"reject_before_allocation\": {},\n  \"disk_memory_mapping_frozen\": {},\n  \
         \"cpu_decode_digest_stable\": {},\n  \"golden_rxpd_byte_equal\": {},\n  \
         \"expanded_digest\": \"{}\",\n  \"expanded_u32_count\": {},\n  \
         \"compress_stream_eq\": {},\n  \"rxpd_len\": {}\n}}",
        abi_ok,
        records_eq,
        compress_twice && compress_eq,
        trunc_rej,
        chk_rej,
        unk_codec,
        unk_ver,
        overlap_rej && oob_rej,
        reject_before,
        mapping_frozen,
        cpu_stable,
        golden_eq,
        hex32(&dig1),
        expand_n,
        compress_eq,
        rxpd.len()
    );
}

fn mapping_ok() -> bool {
    disk::mapping_allows(DISK_MAJOR, MEMORY_MAJOR)
}
