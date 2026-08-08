//! G8.5a M19 host probe:跑 16 帧 fixture,stdout 单行 JSON + 可选写 golden。
//!
//! `--red-wrong-eviction` = A2 RED 驱逐轴:同脚本把池预算 6→7,驱逐决策必变,
//! 事件序列 sha 必与基线/golden 不同(证明序列判据非空转)。

use std::env;
use std::fs;
use std::path::PathBuf;

use rurix_render::shadow::page_cache::{
    result_to_json_value, run_m19_fixture, run_m19_fixture_pool, RED_EVICT_POOL,
};

fn main() {
    let mut golden_dir: Option<PathBuf> = None;
    let mut write_golden = false;
    let mut red_wrong_eviction = false;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--golden-dir" => {
                i += 1;
                golden_dir = Some(PathBuf::from(args.get(i).expect("--golden-dir path")));
            }
            "--write-golden" => write_golden = true,
            "--red-wrong-eviction" => red_wrong_eviction = true,
            other => {
                eprintln!("unknown arg {other}");
                std::process::exit(2);
            }
        }
        i += 1;
    }

    let r = run_m19_fixture();
    let golden_sha = golden_dir.as_ref().and_then(|d| {
        let p = d.join("m19_events.sha256");
        fs::read_to_string(p).ok().map(|s| s.trim().to_owned())
    });

    if red_wrong_eviction {
        let red = run_m19_fixture_pool(RED_EVICT_POOL);
        let sha_red = golden_sha
            .as_deref()
            .map(|g| g != red.events_sha256)
            .unwrap_or(red.events_sha256 != r.events_sha256);
        let evict_red = red.evict_count != r.evict_count && red.evict_count > 0;
        println!(
            "{{\"subject\":\"g8_m19_red_wrong_eviction\",\
             \"base_pool_pages\":{},\"red_pool_pages\":{},\
             \"base_evict_count\":{},\"red_evict_count\":{},\
             \"base_events_sha256\":\"{}\",\"red_events_sha256\":\"{}\",\
             \"evict_order_changed\":{},\"event_sequence_red\":{},\"red_ok\":{}}}",
            r.pool_pages,
            red.pool_pages,
            r.evict_count,
            red.evict_count,
            r.events_sha256,
            red.events_sha256,
            evict_red,
            sha_red,
            evict_red && sha_red && r.evict_count > 0
        );
        return;
    }

    if write_golden {
        let dir = golden_dir.expect("--write-golden 需要 --golden-dir");
        fs::create_dir_all(&dir).expect("mkdir golden");
        fs::write(dir.join("m19_events.jsonl"), &r.canonical_json).expect("write events");
        fs::write(dir.join("m19_events.sha256"), format!("{}\n", r.events_sha256))
            .expect("write sha");
        let dig = serde_like_digests(&r);
        fs::write(dir.join("m19_digests.json"), dig).expect("write digests");
        eprintln!("[g8_m19_probe] wrote golden → {}", dir.display());
    }

    println!("{}", result_to_json_value(&r, golden_sha.as_deref()));
}

fn serde_like_digests(r: &rurix_render::shadow::page_cache::M19RunResult) -> String {
    let mut s = String::from("{\n  \"frames\": [\n");
    for (i, d) in r.digests.iter().enumerate() {
        if i > 0 {
            s.push_str(",\n");
        }
        s.push_str(&format!(
            "    {{\"frame\":{},\"page_table\":\"{}\",\"depth_pool\":\"{}\",\"sample\":\"{}\",\"dirty_depth\":\"{}\"}}",
            d.frame, d.page_table, d.depth_pool, d.sample, d.dirty_depth
        ));
    }
    s.push_str("\n  ]\n}\n");
    s
}
