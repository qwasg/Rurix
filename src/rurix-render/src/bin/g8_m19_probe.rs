//! G8.5a M19 host probe:跑 16 帧 fixture,stdout 单行 JSON + 可选写 golden。

use std::env;
use std::fs;
use std::path::PathBuf;

use rurix_render::shadow::page_cache::{result_to_json_value, run_m19_fixture};

fn main() {
    let mut golden_dir: Option<PathBuf> = None;
    let mut write_golden = false;
    let args: Vec<String> = env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--golden-dir" => {
                i += 1;
                golden_dir = Some(PathBuf::from(args.get(i).expect("--golden-dir path")));
            }
            "--write-golden" => write_golden = true,
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
