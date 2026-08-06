//! G8.2 M30 PSO cache **device 门 harness**(RXS-0314~0316;RFC-0019 §4.1.4;门
//! g8.p0.m30.pso_cache;沿 `bin/vk_rt`/`bin/vk_mesh` device 真跑 / SKIP 三态体例)。
//!
//! ## 四模式(各自**独立进程**——「全新进程 warm run」判据字面)
//! - `--collector-only`:纯 host 输出固定场景 collector 的 PSO key 集合 JSON(RXS-0314
//!   字段位),不触 GPU;golden `tests/pso/pso_keys.golden.json` 的生成/比对源。
//! - `--cold <dir>`:precache 构建——逐 pso_key 恰好创建一次 pipeline 并捕获落盘
//!   (RXS-0316),写 `<dir>/rurix_pso_cache.bin`(RXS-0315)。
//! - `--warm <dir> [--drop-key N]`:全新进程 warm——装载核验 → 逐 key 重建,全部 create
//!   带 FAIL_ON_PIPELINE_COMPILE_REQUIRED;`--drop-key N` = 能红反证腿(warm 前物理删
//!   单条 key 持久化数据 → 该 key 必记 stall)。
//! - `--tamper <schema|version|driver_uuid|keyset> <dir>`:篡改 store header 对应段后
//!   尝试 warm——必须 fail-closed 全量重建(rebuild_reason 正确)且输出仍正确
//!   (no_false_hit,RXS-0315 IR)。
//!
//! 输出 JSON 到 stdout(供 `ci/g8_pso_cache_smoke.py` 消费;人类日志走 stderr)。
//! validation:`RURIX_VK_VALIDATION=1` 时装载 VK_LAYER_KHRONOS_validation + debug
//! utils messenger fail-closed,任何 ERROR → 非零退出。**三态**:无 loader/无 GPU/RT 扩展
//! 或 pipelineCreationCacheControl 缺位 → `device_state=skipped_dev_env` 退 0(dev-env
//! degrade,非 fake pass);`RURIX_REQUIRE_REAL=1` 翻硬红归 smoke 脚本层裁决。

use std::path::PathBuf;

use rurix_rt::pso_cache::{
    self, PsoCacheManager, PsoRunOutcome, collector_json, collect_records, pso_fixtures,
};

/// 无设备 / provisioning 缺失(SKIP)信号(镜像 bin/vk_rt NO_DEVICE_KEYS + M30 特性轴)。
const NO_DEVICE_KEYS: &[&str] = &[
    "vulkan loader",
    "vulkan-1.dll",
    "libvulkan",
    "物理设备",
    "graphics/compute queue",
    "vkCreateInstance",
    "缺扩展",
    "RT feature",
    "vkGetPhysicalDeviceFeatures2",
    "vkEnumerateDeviceExtensionProperties",
    "pipelineCreationCacheControl 特性缺位",
];

fn is_no_device(e: &str) -> bool {
    NO_DEVICE_KEYS.iter().any(|k| e.contains(k))
}

fn hex_of(b: &[u8]) -> String {
    let mut s = String::with_capacity(b.len() * 2);
    for x in b {
        s.push_str(&format!("{x:02x}"));
    }
    s
}

fn outcome_json(
    mode: &str,
    o: &PsoRunOutcome,
    drop_key: Option<usize>,
    red_leg_scope: Option<&str>,
    tamper_axis: Option<&str>,
) -> String {
    let branch = if o.branch == rurix_rt::vk::PSO_BRANCH_BINARY {
        "binary"
    } else {
        "cache"
    };
    let per_key = o
        .per_key
        .iter()
        .map(|k| {
            format!(
                "    {{\"name\": \"{}\", \"hit\": {}, \"stalled\": {}, \"built\": {}}}",
                k.name, k.hit, k.stalled, k.built
            )
        })
        .collect::<Vec<_>>()
        .join(",\n");
    let hits = o.per_key.iter().filter(|k| k.hit).count();
    format!(
        "{{\n  \"mode\": \"{mode}\",\n  \"device_state\": \"executed\",\n  \
         \"branch\": \"{branch}\",\n  \
         \"pipeline_binary_capability\": {},\n  \
         \"pipeline_creation_cache_control\": {},\n  \
         \"device\": {{\n    \"name\": \"{}\",\n    \"vendor_id\": {},\n    \
         \"device_id\": {},\n    \"driver_version\": {},\n    \"api_version\": {},\n    \
         \"pipeline_cache_uuid\": \"{}\"\n  }},\n  \
         \"keyset_digest\": \"{}\",\n  \
         \"rebuild_reason\": \"{}\",\n  \"rebuilt\": {},\n  \"false_hits\": {},\n  \
         \"precache_build_count\": {},\n  \"runtime_compile_stalls\": {},\n  \
         \"hits\": {},\n  \
         \"per_key\": [\n{per_key}\n  ],\n  \
         \"validation_errors\": {},\n  \
         \"drop_key\": {},\n  \"red_leg_scope\": {},\n  \"tamper_axis\": {}\n}}",
        o.pipeline_binary_capability,
        o.pipeline_creation_cache_control,
        o.identity.device_name,
        o.identity.vendor_id,
        o.identity.device_id,
        o.identity.driver_version,
        o.identity.api_version,
        hex_of(&o.identity.pipeline_cache_uuid),
        hex_of(&o.keyset_digest),
        o.rebuild_reason.as_str(),
        o.rebuilt,
        o.false_hits,
        o.precache_build_count,
        o.runtime_compile_stalls,
        hits,
        if o.validation_error { 1 } else { 0 },
        drop_key.map_or("null".to_owned(), |n| n.to_string()),
        red_leg_scope.map_or("null".to_owned(), |s| format!("\"{s}\"")),
        tamper_axis.map_or("null".to_owned(), |s| format!("\"{s}\"")),
    )
}

fn skip_json(reason: &str) -> String {
    let esc = reason.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
    format!(
        "{{\n  \"device_state\": \"skipped_dev_env\",\n  \"reason\": \"{esc}\"\n}}"
    )
}

fn fail(msg: &str) -> ! {
    eprintln!("PSO: FAIL {msg}");
    std::process::exit(1)
}

fn usage() -> ! {
    eprintln!(
        "用法: vk_pso_cache --collector-only | --cold <dir> | --warm <dir> [--drop-key N] | \
         --tamper <schema|version|driver_uuid|keyset> <dir>"
    );
    std::process::exit(2)
}

/// device 模式统一执行/分类(SKIP 三态 + validation fail-closed + JSON 落 stdout)。
fn run_device<F>(mode: &str, drop_key: Option<usize>, tamper_axis: Option<&str>, f: F) -> !
where
    F: FnOnce() -> Result<(PsoRunOutcome, Option<String>), String>,
{
    match f() {
        Ok((o, scope)) => {
            println!(
                "{}",
                outcome_json(mode, &o, drop_key, scope.as_deref(), tamper_axis)
            );
            std::process::exit(0)
        }
        Err(e) if is_no_device(&e) => {
            eprintln!("PSO: SKIP 无 Vulkan 设备 / provisioning 缺失({})", e.trim());
            println!("{}", skip_json(e.trim()));
            std::process::exit(0)
        }
        Err(e) if e.contains("VK_LAYER_KHRONOS_validation") => {
            eprintln!("PSO: FAIL validation ERROR(fail-closed): {e}");
            std::process::exit(2)
        }
        Err(e) => fail(&format!("{mode} 会话: {e}")),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    eprintln!("[vk_pso_cache] G8.2 M30 PSO cache device 门 harness(RFC-0019 §4.1.4,RXS-0314~0316)");

    let fixtures = match pso_fixtures() {
        Ok(f) => f,
        Err(e) => fail(&format!("fixture 集: {e}")),
    };

    match args[1].as_str() {
        // ── 纯 host collector(不触 GPU)──
        "--collector-only" => {
            let records = collect_records(&fixtures);
            print!("{}", collector_json(&records));
        }
        "--cold" => {
            let dir = args.get(2).map(PathBuf::from).unwrap_or_else(|| usage());
            run_device("cold", None, None, move || {
                let mut mgr = PsoCacheManager::new();
                let o = mgr.cold(&dir, &fixtures)?;
                Ok((o, None))
            });
        }
        "--warm" => {
            let dir = args.get(2).map(PathBuf::from).unwrap_or_else(|| usage());
            let drop_key: Option<usize> = args
                .iter()
                .position(|a| a == "--drop-key")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse().ok());
            run_device("warm", drop_key, None, move || {
                let mut mgr = PsoCacheManager::new();
                let scope = match drop_key {
                    Some(n) => Some(mgr.drop_key_blob(&dir, n)?),
                    None => None,
                };
                let o = mgr.warm(&dir, &fixtures)?;
                Ok((o, scope))
            });
        }
        "--tamper" => {
            let axis = args.get(2).cloned().unwrap_or_else(|| usage());
            if !matches!(
                axis.as_str(),
                "schema" | "version" | "driver_uuid" | "keyset"
            ) {
                usage();
            }
            let dir = args.get(3).map(PathBuf::from).unwrap_or_else(|| usage());
            let axis_l = axis.clone();
            run_device("warm", None, Some(&axis_l), move || {
                let mut mgr = PsoCacheManager::new();
                let o = mgr.tamper(&dir, &fixtures, &axis)?;
                Ok((o, None))
            });
        }
        _ => usage(),
    }
}

// pso_cache 模块面引用(防 unused 告警;manager 计数器公开字段契约锚)。
#[allow(dead_code)]
fn _anchors(m: &PsoCacheManager) -> (usize, usize) {
    let _ = pso_cache::STORE_FILE_NAME;
    (m.precache_build_count, m.runtime_compile_stalls)
}
