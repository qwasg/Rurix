//! G9.5 M110 世界分区 harness(RXS-0363;门 `g9.p0.m110.world_partition`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §2 M110 行逐字 + spec/world_partition.md RXS-0363)
//!
//! 1. **单一持久世界 schema + 2D cell 冻结**:canonical 二进制 encode→decode→
//!    encode 逐字节往返;cell 边长为资产属性(仅改 `cell_size_m` ⇒ digest 分叉;
//!    篡改派生包围盒 fail-closed);schema 显式分列 `always_loaded` /
//!    `spatially_loaded`(每对象携带 cell 归属);
//! 2. **三项预算契约逐帧 evidence 非空**:`MaxStreamingCellsPerFrame` /
//!    `MaxActorsToSpawnPerFrame` / `MemoryBudgetMB` 一等契约字段,canonical 场景
//!    逐帧三计数器全记录;逐帧一致性机核(静默超帧即 Err);
//! 3. **预算违约注入必排队降级(RED 臂)**:注入 `MaxStreamingCellsPerFrame=0`
//!    ⇒ 零 cell 驻留 + 每帧 `budget_stall` 报警 + 队列深度显式增长;充足预算
//!    sabotage 探针零报警(能红证明);篡改帧喂一致性机核必检出;
//! 4. **cell 四事件序列逐字 golden**:`CellLoadBegin / CellResident /
//!    CellUnloadBegin / CellEvicted` 闭集;固定相机轨迹事件日志 digest 对
//!    `milestones/g9/g9_m110_world_partition_band.json` 逐字相等(measured 冻
//!    结,禁手写);乱序注入被独立状态机校验器判 RED;
//! 5. **Data Layer 掩码位只预留不接线**:字段参与往返;激活查询一律 typed
//!    `Err(DataLayerNotWired)`;掩码非零不改变流送行为(事件 digest 逐位相等);
//! 6. **代表性大世界 soak hitch p99 ≤ measured 阈值**:512×512 cell 大世界 +
//!    合成相机路径 ≥10000 帧(声明阈值 `M110_SOAK_MIN_FRAMES`);hitch p99 ≤
//!    `g9_budget.json` 的 `g9.bench.world_partition_hitch_p99_ms` 阈值
//!    (measured×1.5 冻结,禁手写 P-09);hitch 计数面逐帧非空;
//! 7. **HLOD 烘焙工具(asset graph 新 tool `rurix.hlod.bake.v1`,离线 host 侧,
//!    GPU 非必需)**:同输入两次进程级构建产物位级相等;声明序扰动 digest 不变;
//!    几何扰动分叉(RED 臂能红证明);产物 digest 接入 cell 元数据 HLOD 层级引
//!    用字段 roundtrip;
//! 8. **conformance 语料消费**:`conformance/world_partition/` 13 件锚定语料逐
//!    件 `//@ spec: RXS-03##` 锚核验(M110 三件锚 RXS-0363)。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);rxhlod 同构建目录缺失 ⇒ FAIL(不静默 SKIP)。判据不符 /
//! RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m110_world_partition [--evidence <path>] [--band <path>] [--work-dir <dir>]
//!                         [--soak-frames N]
//! g9_m110_world_partition --freeze [--band-out <path>] [--evidence <path>]
//! g9_m110_world_partition --red-arm budget-overrun|event-order|hlod-drift
//! g9_m110_world_partition --long-soak --min-seconds S --min-frames N
//! ```
//!
//! ## G9.8a 长 soak 扩展（`--long-soak`，加性子模式，门流程 0-byte）
//!
//! 同一 512×512 cell 大世界 + 同一 soak 预算 + 同一闭式相机路径（周期
//! lcm(1024,1536)=3072 帧循环回放，与门内 12000 帧面逐帧同源）跑**墙钟驱动**
//! 长 soak：循环至 `elapsed ≥ min_seconds` 且 `frames ≥ min_frames` 双阈值同时
//! 满足。honesty 口径（沿 G8.8a uc08 soak 语义）：全程真实帧循环（tick + 逐帧
//! 预算一致性机核 + 事件 drain 全工作量），**零 sleep**（`sleep_seconds` 恒
//! 0.0），`active_frame_seconds` = 逐帧工作量测和，与 `soak_seconds`（帧循环
//! 墙钟）同源产出；帧计数/hitch p99/流送计数非空即硬断言（空即 FAIL 不充绿）。
//! 输出单行 JSON 供 `ci/g9_stabilization_soak.py` 机器核验。

#![forbid(unsafe_code)]

use rurix_pkg::sha256;
use rurix_render::world::partition as wp;
use std::path::{Path, PathBuf};

const TAG: &str = "G9_M110_WP";
const CANONICAL_FRAMES: u32 = 64;
const BUDGET_ENTRY_ID: &str = "g9.bench.world_partition_hitch_p99_ms";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/atmosphere_froxel_fog_minimal.rx", "RXS-0365"),
    ("accept/cell_event_sequence_minimal.rx", "RXS-0363"),
    ("accept/decal_dbuffer_placeholder_minimal.rx", "RXS-0368"),
    ("accept/hlod_baking_double_build_minimal.rx", "RXS-0364"),
    ("accept/terrain_chunk_cell_aligned_minimal.rx", "RXS-0367"),
    ("accept/water_dual_pipeline_minimal.rx", "RXS-0366"),
    ("reject/atmosphere_weather_map_signature_tampered.rx", "RXS-0365"),
    ("reject/cell_event_sequence_out_of_order.rx", "RXS-0363"),
    ("reject/decal_overdraw_budget_exceeded.rx", "RXS-0368"),
    ("reject/hlod_runtime_merge_forbidden.rx", "RXS-0364"),
    ("reject/partition_budget_overrun_no_demote.rx", "RXS-0363"),
    ("reject/terrain_lod_gap_crack.rx", "RXS-0367"),
    ("reject/water_spectrum_param_invalid.rx", "RXS-0366"),
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

/// JSON 字符串转义(手工 JSON 纪律:路径含反斜杠,必转义)。
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

fn workspace_root() -> PathBuf {
    let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    p.pop();
    p.pop();
    p
}

/// UTC 时间戳(秒级;无依赖手工拼,civil-from-days 算法 Howard Hinnant)。
fn utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (hh, mm, ss) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{hh:02}:{mm:02}:{ss:02}Z")
}

/// 自有扁平 JSON 的字符串字段提取(band 文件为本 harness 自产,布局稳定)。
fn json_str<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\": \"");
    let start = text.find(&needle)? + needle.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

/// 自有扁平 JSON 的数值字段提取。
fn json_f64(text: &str, key: &str) -> Option<f64> {
    let needle = format!("\"{key}\": ");
    let start = text.find(&needle)? + needle.len();
    let rest = &text[start..];
    let end = rest
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+' || c == 'e' || c == 'E'))
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// 从 g9_budget.json 提取指定 entry 的 (threshold, measured_value)。
fn extract_budget_entry(text: &str, id: &str) -> Option<(f64, f64)> {
    let needle = format!("\"id\": \"{id}\"");
    let pos = text.find(&needle)?;
    let tail = &text[pos..pos + 4096.min(text.len() - pos)];
    let threshold = json_f64(tail, "threshold")?;
    let measured = json_f64(tail, "measured_value")?;
    Some((threshold, measured))
}

struct Args {
    evidence: Option<PathBuf>,
    band: PathBuf,
    work_dir: PathBuf,
    soak_frames: u32,
    freeze: bool,
    red_arm: Option<String>,
    long_soak: bool,
    min_seconds: u64,
    min_frames: u64,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut out = Args {
        evidence: None,
        band: root.join("milestones/g9/g9_m110_world_partition_band.json"),
        work_dir: std::env::temp_dir().join("g9_m110_world_partition"),
        soak_frames: wp::M110_SOAK_DEFAULT_FRAMES,
        freeze: false,
        red_arm: None,
        long_soak: false,
        min_seconds: 0,
        min_frames: 0,
    };
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut i = 0;
    while i < args.len() {
        let take = |i: &mut usize| -> String {
            *i += 1;
            args.get(*i)
                .unwrap_or_else(|| fail(&format!("{} 缺参数", args[*i - 1])))
                .clone()
        };
        match args[i].as_str() {
            "--evidence" => out.evidence = Some(PathBuf::from(take(&mut i))),
            "--band" | "--band-out" => out.band = PathBuf::from(take(&mut i)),
            "--work-dir" => out.work_dir = PathBuf::from(take(&mut i)),
            "--soak-frames" => {
                out.soak_frames = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--soak-frames 非整数"))
            }
            "--freeze" => out.freeze = true,
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            "--long-soak" => out.long_soak = true,
            "--min-seconds" => {
                out.min_seconds = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--min-seconds 非整数"))
            }
            "--min-frames" => {
                out.min_frames = take(&mut i)
                    .parse()
                    .unwrap_or_else(|_| fail("--min-frames 非整数"))
            }
            other => fail(&format!("未知参数: {other}")),
        }
        i += 1;
    }
    out
}

// ---------------------------------------------------------------------------
// 场景驱动
// ---------------------------------------------------------------------------

/// canonical golden 场景:64 帧固定相机轨迹,逐帧一致性机核,返回(逐帧预算
/// evidence, 事件日志, 事件 digest)。
fn run_canonical_scenario() -> (Vec<wp::FrameBudgetEvidence>, Vec<wp::CellEvent>, [u8; 32]) {
    let world = wp::canonical_world();
    let budget = wp::canonical_budget();
    let path = wp::canonical_camera_path(CANONICAL_FRAMES);
    let mut rt = wp::PartitionRuntime::new(world, budget).expect("runtime 装配");
    let mut frames = Vec::with_capacity(CANONICAL_FRAMES as usize);
    for (f, s) in path.iter().enumerate() {
        let ev = rt
            .tick(f as u32, std::slice::from_ref(s))
            .expect("tick");
        wp::check_frame_budget_consistency(&ev, &budget).expect("逐帧预算一致性");
        frames.push(ev);
    }
    let events = rt.events().to_vec();
    let digest = wp::event_log_digest(&events);
    (frames, events, digest)
}

/// 相机轨迹 digest(场景 provenance)。
fn camera_path_digest(path: &[wp::StreamingSource]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(path.len() * 17);
    for s in path {
        buf.push(s.kind as u8);
        for v in [s.position_m[0], s.position_m[1], s.loading_radius_m, s.inner_radius_m] {
            buf.extend_from_slice(&v.to_le_bytes());
        }
    }
    sha256::digest(&buf)
}

/// 定位 rxhlod 同构建目录工具(缺失即 FAIL,不静默 SKIP)。
fn rxhlod_exe() -> PathBuf {
    if let Ok(p) = std::env::var("RXHLOD_EXE") {
        return PathBuf::from(p);
    }
    let exe = std::env::current_exe().expect("current_exe");
    let dir = exe.parent().expect("exe 父目录");
    let name = if cfg!(windows) { "rxhlod.exe" } else { "rxhlod" };
    let path = dir.join(name);
    if !path.is_file() {
        fail(&format!(
            "rxhlod 未与 harness 同构建({path:?} 不存在;先 `cargo build --bins`)"
        ));
    }
    path
}

/// 进程级双构建:两次独立 `rxhlod bake --out` 产物逐字节比较(不等即 FAIL);
/// 返回(产物字节, digest)。
fn rxhlod_double_build(work_dir: &Path) -> (Vec<u8>, [u8; 32]) {
    let exe = rxhlod_exe();
    let mut products = Vec::new();
    for tag in ["b1", "b2"] {
        let dir = work_dir.join(format!("hlod_{tag}"));
        let out = std::process::Command::new(&exe)
            .arg("bake")
            .arg("--out")
            .arg(&dir)
            .output()
            .unwrap_or_else(|e| fail(&format!("rxhlod bake 启动: {e}")));
        if !out.status.success() {
            fail(&format!(
                "rxhlod bake 退非零: {}",
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        products.push(
            std::fs::read(dir.join("hlod_demo.rxhlod"))
                .unwrap_or_else(|e| fail(&format!("读 rxhlod 产物: {e}"))),
        );
    }
    if products[0] != products[1] {
        fail("rxhlod 进程级双构建产物位级不等(b1 ≠ b2)");
    }
    let digest = sha256::digest(&products[0]);
    (std::mem::take(&mut products[0]), digest)
}

/// `rxhlod check` 子模式(声明序扰动免疫 + 几何扰动分叉能红证明)。
fn rxhlod_check() -> bool {
    let exe = rxhlod_exe();
    let out = std::process::Command::new(&exe)
        .arg("check")
        .output()
        .unwrap_or_else(|e| fail(&format!("rxhlod check 启动: {e}")));
    out.status.success() && String::from_utf8_lossy(&out.stdout).contains("\"ok\": true")
}

// ---------------------------------------------------------------------------
// RED 臂(子模式独立复跑;PASS = 检测面有效)
// ---------------------------------------------------------------------------

/// RED 臂:预算违约注入必排队降级(RXS-0363 L4)。
fn red_arm_budget() -> Result<(), String> {
    let world = wp::canonical_world();
    let injected = wp::PartitionBudget {
        max_streaming_cells_per_frame: 0,
        ..wp::canonical_budget()
    };
    let path = wp::canonical_camera_path(16);
    let mut rt = wp::PartitionRuntime::new(world, injected).map_err(|e| e.to_string())?;
    let mut stall_frames = 0u64;
    for (f, s) in path.iter().enumerate() {
        let ev = rt
            .tick(f as u32, std::slice::from_ref(s))
            .map_err(|e| e.to_string())?;
        if ev.streaming_cells_this_frame > 0 {
            return Err(format!(
                "静默超帧: 注入 MaxStreamingCellsPerFrame=0 后帧 {f} 仍流送 {} cell",
                ev.streaming_cells_this_frame
            ));
        }
        if ev.budget_stall {
            stall_frames += 1;
        }
    }
    if !rt.resident().is_empty() {
        return Err("静默超帧: 注入后仍有 cell 驻留".into());
    }
    if stall_frames == 0 || rt.counters().budget_stall_frames == 0 {
        return Err("预算违约未触发任何报警(降级不可见 = 漏检)".into());
    }
    if rt.counters().peak_queue_depth == 0 {
        return Err("预算违约未排队(队列深度恒零 = 漏检)".into());
    }
    // sabotage 探针(能红证明):充足预算下同轨迹零报警。
    let generous = wp::PartitionBudget {
        max_streaming_cells_per_frame: 1024,
        max_actors_to_spawn_per_frame: 1 << 20,
        memory_budget_mb: 4096,
    };
    let mut ok = wp::PartitionRuntime::new(wp::canonical_world(), generous)
        .map_err(|e| e.to_string())?;
    for (f, s) in path.iter().enumerate() {
        let ev = ok
            .tick(f as u32, std::slice::from_ref(s))
            .map_err(|e| e.to_string())?;
        if ev.budget_stall {
            return Err("sabotage 探针误触发: 充足预算下出现报警位".into());
        }
    }
    if ok.counters().total_cells_streamed == 0 {
        return Err("sabotage 探针无流送(场景不平凡性失效)".into());
    }
    // 篡改帧喂一致性机核:静默超帧必被检出(机核能红证明)。
    let budget = wp::canonical_budget();
    let doctored = wp::FrameBudgetEvidence {
        streaming_cells_this_frame: budget.max_streaming_cells_per_frame + 1,
        ..wp::FrameBudgetEvidence::default()
    };
    if wp::check_frame_budget_consistency(&doctored, &budget).is_ok() {
        return Err("一致性机核漏检篡改帧(静默超帧)".into());
    }
    Ok(())
}

/// RED 臂:cell 事件乱序注入必拒(RXS-0363 L5)。
fn red_arm_event_order() -> Result<(), String> {
    let (_, events, digest) = run_canonical_scenario();
    if events.len() < 8 {
        return Err("golden 事件日志过短(场景不平凡)".into());
    }
    // golden 序列必须被校验器接受(校验器非平凡恒拒)。
    wp::validate_event_log(&events).map_err(|e| format!("golden 序列被拒: {e}"))?;
    // 乱序注入:同 cell 的 Resident 提到 LoadBegin 前(运行时按对发射,交换相
    // 邻一对即构成同 cell 状态机失序;跨 cell 交换为合法交错,不构成注入)。
    let i = events
        .iter()
        .position(|e| e.kind == wp::CellEventKind::CellResident)
        .ok_or("golden 日志缺 Resident 事件")?;
    let mut swapped = events.clone();
    swapped.swap(i - 1, i);
    match wp::validate_event_log(&swapped) {
        Err(wp::PartitionError::EventOutOfOrder { .. })
        | Err(wp::PartitionError::FrameNonMonotonic { .. }) => {}
        other => return Err(format!("乱序注入未被状态机校验器拒绝: {other:?}")),
    }
    if wp::event_log_digest(&swapped) == digest {
        return Err("乱序注入 digest 未分叉".into());
    }
    Ok(())
}

/// RED 臂:HLOD 双构建 hash 漂移检测(RXS-0364 语义锚;几何扰动必分叉)。
fn red_arm_hlod(work_dir: &Path) -> Result<(), String> {
    let (bytes, digest) = rxhlod_double_build(work_dir);
    let (bytes2, digest2) = rxhlod_double_build(&work_dir.join("second"));
    if bytes != bytes2 || digest != digest2 {
        return Err("同输入两次进程级构建产物位级不等(双构建 hash 漂移)".into());
    }
    if !rxhlod_check() {
        return Err("rxhlod check 失败(声明序免疫/几何扰动分叉面失效)".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// G9.8a 长 soak 子模式（--long-soak；加性，门流程 0-byte）
// ---------------------------------------------------------------------------

/// 闭式相机路径循环周期（`soak_camera_path` 两轴三角波周期 lcm(1024,1536)）。
const LONG_SOAK_PATH_CYCLE_FRAMES: u32 = 3072;

/// 墙钟驱动长 soak：同一 soak 大世界/预算/闭式相机路径（3072 帧周期循环），
/// 循环至 `elapsed ≥ min_seconds` 且 `frames ≥ min_frames` 双阈值同时满足；
/// 全程真实帧循环零 sleep，honesty 字段（sleep_seconds=0/active≈wall）单行
/// JSON 输出供 ci/g9_stabilization_soak.py 机器核验。
fn run_long_soak(min_seconds: u64, min_frames: u64) -> ! {
    if min_seconds == 0 {
        fail("--min-seconds 必须 >0（墙钟硬口径，禁 0 充数）");
    }
    if min_frames < wp::M110_SOAK_MIN_FRAMES as u64 {
        fail(&format!(
            "--min-frames {min_frames} 低于声明阈值 {}",
            wp::M110_SOAK_MIN_FRAMES
        ));
    }
    let world = wp::soak_world();
    let budget = wp::soak_budget();
    let world_cells = world.cells.len();
    let spatial_objects = world.spatially_loaded.len();
    let world_digest = hex(&wp::world_digest(&world).expect("world digest"));
    let cycle = wp::soak_camera_path(LONG_SOAK_PATH_CYCLE_FRAMES);
    let cycle_digest = hex(&camera_path_digest(&cycle));
    let mut rt = wp::PartitionRuntime::new(world, budget).expect("runtime 装配");
    let warmup = wp::M110_SOAK_WARMUP_FRAMES as u64;
    let mut tick_samples: Vec<u64> = Vec::new();
    let mut active_ns: u128 = 0;
    let mut events_by_kind = [0u64; 4];
    let mut total_events: u64 = 0;
    let mut total_cells_streamed: u64 = 0;
    let mut frames: u64 = 0;
    let loop_start = std::time::Instant::now();
    let mut last_progress = std::time::Instant::now();
    loop {
        // 逐帧工作量测区：tick + 逐帧预算一致性机核 + 事件 drain + 计数聚合
        // （active_frame_seconds 的唯一来源；循环控制开销可忽略）。
        let f0 = std::time::Instant::now();
        let src = &cycle[(frames % LONG_SOAK_PATH_CYCLE_FRAMES as u64) as usize];
        let ev = rt
            .tick(frames as u32, std::slice::from_ref(src))
            .unwrap_or_else(|e| fail(&format!("long-soak 帧 {frames} tick: {e}")));
        wp::check_frame_budget_consistency(&ev, &budget)
            .unwrap_or_else(|e| fail(&format!("long-soak 帧 {frames} 预算一致性: {e}")));
        for e in rt.drain_events() {
            // 四事件闭集计数(序与 CellEventKind::code 一致;code() 为 crate 内
            // 私有,bin 侧用公开 variant 匹配)。
            match e.kind {
                wp::CellEventKind::CellLoadBegin => events_by_kind[0] += 1,
                wp::CellEventKind::CellResident => events_by_kind[1] += 1,
                wp::CellEventKind::CellUnloadBegin => events_by_kind[2] += 1,
                wp::CellEventKind::CellEvicted => events_by_kind[3] += 1,
            }
            total_events += 1;
        }
        total_cells_streamed += ev.streaming_cells_this_frame as u64;
        let frame_ns = f0.elapsed().as_nanos() as u64;
        active_ns += frame_ns as u128;
        if frames >= warmup {
            tick_samples.push(frame_ns);
        }
        frames += 1;
        let elapsed = loop_start.elapsed().as_secs_f64();
        if frames >= min_frames && elapsed >= min_seconds as f64 {
            break;
        }
        if last_progress.elapsed().as_secs() >= 60 {
            eprintln!(
                "{TAG}: long-soak progress frames={frames} elapsed={elapsed:.1}s \
                 events={total_events} streamed={total_cells_streamed}"
            );
            last_progress = std::time::Instant::now();
        }
    }
    let seconds = loop_start.elapsed().as_secs_f64();
    let active_seconds = active_ns as f64 / 1e9;
    // 帧计数/hitch/流送计数非空硬断言（空即 FAIL 不充绿）。
    if tick_samples.is_empty() || total_events == 0 || total_cells_streamed == 0 {
        fail(&format!(
            "long-soak 计数面空: samples={} events={total_events} streamed={total_cells_streamed}",
            tick_samples.len()
        ));
    }
    let p50_ms = wp::percentile_ns(&tick_samples, 0.50) as f64 / 1e6;
    let p95_ms = wp::percentile_ns(&tick_samples, 0.95) as f64 / 1e6;
    let p99_ms = wp::percentile_ns(&tick_samples, 0.99) as f64 / 1e6;
    let max_ms = *tick_samples.iter().max().unwrap_or(&0) as f64 / 1e6;
    let base_commit =
        std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\"ok\": true, \"soak\": true, \"soak_subject\": \"host-soak\", \
         \"subject\": \"g9_m110_world_partition_long_soak\", \
         \"schema\": \"rurix.g9m110.world_partition_long_soak.v1\", \"schema_version\": 1, \
         \"soak_frames\": {frames}, \"frames\": {frames}, \
         \"soak_seconds\": {seconds:.3}, \"seconds\": {seconds:.3}, \
         \"active_frame_seconds\": {active_seconds:.3}, \"sleep_seconds\": 0.0, \
         \"min_seconds\": {min_seconds}, \"min_frames\": {min_frames}, \"warmup_frames\": {warmup}, \
         \"world_cells\": {world_cells}, \"spatial_objects\": {spatial_objects}, \
         \"world_digest\": \"{world_digest}\", \
         \"path_cycle_frames\": {LONG_SOAK_PATH_CYCLE_FRAMES}, \"path_cycle_digest\": \"{cycle_digest}\", \
         \"hitch\": {{\"p50_ms\": {p50_ms:.6}, \"p95_ms\": {p95_ms:.6}, \"p99_ms\": {p99_ms:.6}, \"max_ms\": {max_ms:.6}}}, \
         \"events_by_kind\": [{}, {}, {}, {}], \"total_events\": {total_events}, \
         \"total_cells_streamed\": {total_cells_streamed}, \
         \"budget_caps\": {{\"MaxStreamingCellsPerFrame\": {}, \"MaxActorsToSpawnPerFrame\": {}, \"MemoryBudgetMB\": {}}}, \
         \"evidence_level\": \"measured_local\", \"timestamp\": \"{}\", \"base_commit\": \"{}\"}}",
        events_by_kind[0],
        events_by_kind[1],
        events_by_kind[2],
        events_by_kind[3],
        budget.max_streaming_cells_per_frame,
        budget.max_actors_to_spawn_per_frame,
        budget.memory_budget_mb,
        utc_now(),
        json_escape(&base_commit),
    );
    println!("{json}");
    eprintln!(
        "{TAG}: long-soak PASS frames={frames} seconds={seconds:.1} active={active_seconds:.1} \
         sleep=0.0 p99={p99_ms:.6}ms events={total_events} streamed={total_cells_streamed}"
    );
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// 主流程
// ---------------------------------------------------------------------------

fn main() {
    let args = parse_args();
    let root = workspace_root();
    std::fs::create_dir_all(&args.work_dir).unwrap_or_else(|e| fail(&format!("建 work-dir: {e}")));

    // ── RED 臂子模式 ──
    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "budget-overrun" => red_arm_budget(),
            "event-order" => red_arm_event_order(),
            "hlod-drift" => red_arm_hlod(&args.work_dir),
            other => fail(&format!(
                "未知 RED 臂: {other}(budget-overrun|event-order|hlod-drift)"
            )),
        };
        match r {
            Ok(()) => {
                println!("{TAG}: PASS red-arm {arm}");
                std::process::exit(0);
            }
            Err(e) => fail(&format!("red-arm {arm} 失效(漏检): {e}")),
        }
    }

    // ── G9.8a 长 soak 子模式（加性；先于门流程分派）──
    if args.long_soak {
        run_long_soak(args.min_seconds, args.min_frames);
    }

    let mut failures: Vec<String> = Vec::new();

    // ── 步骤 1:conformance 语料消费(13 件锚定语料逐件 //@ spec 锚核验)──
    let corpus_dir = root.join("conformance/world_partition");
    let mut corpus_ok = true;
    let mut anchors_json: Vec<String> = Vec::new();
    for (rel, expect) in CORPUS_FILES {
        let path = corpus_dir.join(rel);
        let anchor = std::fs::read_to_string(&path)
            .ok()
            .and_then(|t| t.lines().find(|l| l.contains("//@ spec:")).map(|l| l.to_string()));
        let ok = anchor
            .as_ref()
            .map(|l| l.contains(&format!("//@ spec: {expect}")))
            .unwrap_or(false);
        if !ok {
            corpus_ok = false;
            failures.push(format!("语料 {rel} 缺 {expect} 锚"));
        }
        anchors_json.push(format!(
            "\"{}\": \"{}\"",
            rel.replace('\\', "/"),
            if ok { expect } else { "MISSING" }
        ));
    }
    if corpus_ok {
        println!("{TAG}: conformance 语料 13 件锚定核验通过");
    }

    // ── 步骤 2:单一持久世界 schema 往返 + 2D cell 冻结 ──
    let world = wp::canonical_world();
    let world_bytes = wp::encode_world(&world).expect("世界编码");
    let world_back = wp::decode_world(&world_bytes).expect("世界解码");
    let roundtrip_ok = world_back == world
        && wp::encode_world(&world_back).expect("再编码") == world_bytes
        && wp::world_digest(&world).expect("digest") == wp::world_digest(&world_back).expect("digest");
    // cell 边长为资产属性:仅改 cell_size_m ⇒ digest 分叉。
    let mut resized = wp::canonical_world();
    resized.cell_size_m = 128.0;
    let derived: Vec<([f32; 2], [f32; 2])> = resized
        .cells
        .iter()
        .map(|c| wp::derived_cell_bounds_xy(&resized, c.coord))
        .collect();
    for (c, (lo, hi)) in resized.cells.iter_mut().zip(derived) {
        c.bounds_min[0] = lo[0];
        c.bounds_min[1] = lo[1];
        c.bounds_max[0] = hi[0];
        c.bounds_max[1] = hi[1];
    }
    let cell_size_asset_ok = wp::world_digest(&resized).expect("digest")
        != wp::world_digest(&world).expect("digest");
    // 篡改派生包围盒 fail-closed。
    let mut tampered = wp::canonical_world();
    tampered.cells[0].bounds_max[0] += 1.0;
    let bounds_tamper_rejected = wp::encode_world(&tampered).is_err();
    // schema 显式分列:always_loaded 无 cell 归属,spatially_loaded 全携带。
    let classes_ok = !world.always_loaded.is_empty()
        && !world.spatially_loaded.is_empty()
        && world
            .spatially_loaded
            .iter()
            .all(|s| (s.cell as usize) < world.cells.len());
    let schema_ok = roundtrip_ok && cell_size_asset_ok && bounds_tamper_rejected && classes_ok;
    if !schema_ok {
        failures.push(format!(
            "schema 面: roundtrip={roundtrip_ok} cell_size_asset={cell_size_asset_ok} bounds_tamper_rejected={bounds_tamper_rejected} classes={classes_ok}"
        ));
    }

    // ── 步骤 3:Data Layer 掩码位只预留不接线 ──
    let mut masked = wp::canonical_world();
    masked.cells[3].data_layer_mask = 0xDEAD_BEEF;
    let mask_roundtrip = wp::decode_world(&wp::encode_world(&masked).expect("编码"))
        .expect("解码")
        .cells[3]
        .data_layer_mask
        == 0xDEAD_BEEF;
    let mask_query_fail_closed = matches!(
        wp::data_layer_active(&masked, 3, 0),
        Err(wp::PartitionError::DataLayerNotWired)
    );
    let mask_behavior_invariant = {
        let path = wp::canonical_camera_path(16);
        let budget = wp::canonical_budget();
        let mut rt0 = wp::PartitionRuntime::new(wp::canonical_world(), budget).expect("rt0");
        let mut rt1 = wp::PartitionRuntime::new(masked, budget).expect("rt1");
        for (f, s) in path.iter().enumerate() {
            rt0.tick(f as u32, std::slice::from_ref(s)).expect("tick");
            rt1.tick(f as u32, std::slice::from_ref(s)).expect("tick");
        }
        wp::event_log_digest(rt0.events()) == wp::event_log_digest(rt1.events())
    };
    let data_layer_ok = mask_roundtrip && mask_query_fail_closed && mask_behavior_invariant;
    if !data_layer_ok {
        failures.push("Data Layer 掩码位面失效".into());
    }

    // ── 步骤 4:canonical 场景双跑 + 三项预算逐帧 evidence 非空 ──
    let (frames, events, event_digest) = run_canonical_scenario();
    let (frames2, _, event_digest2) = run_canonical_scenario();
    let double_run_ok = frames == frames2 && event_digest == event_digest2;
    let per_frame_nonempty = frames.len() == CANONICAL_FRAMES as usize
        && frames.iter().all(|ev| {
            // 三项计数器逐帧在位(字段面)+ 逐帧一致性机核已过(run 内强制)。
            ev.streaming_cells_this_frame <= wp::canonical_budget().max_streaming_cells_per_frame
                && ev.actors_spawned_this_frame
                    <= wp::canonical_budget().max_actors_to_spawn_per_frame
                && ev.resident_memory_bytes <= wp::canonical_budget().memory_budget_bytes()
        })
        && frames.iter().map(|e| e.streaming_cells_this_frame as u64).sum::<u64>() > 0
        && frames.iter().map(|e| e.cells_unloaded as u64).sum::<u64>() > 0;
    if !double_run_ok || !per_frame_nonempty {
        failures.push(format!(
            "预算逐帧 evidence 面: double_run={double_run_ok} nonempty={per_frame_nonempty}"
        ));
    }

    // ── 步骤 5:预算违约 RED 臂(主流程内联实测)──
    let budget_arm_ok = red_arm_budget().is_ok();
    if !budget_arm_ok {
        failures.push("预算违约 RED 臂失效".into());
    }

    // ── 步骤 6:cell 四事件序列 golden(measured 冻结带对照;freeze 模式自标定)──
    let band_text = match std::fs::read_to_string(&args.band) {
        Ok(t) => Some(t),
        Err(_) if args.freeze => None,
        Err(_) => fail(&format!(
            "冻结带 {:?} 不存在——先跑 `--freeze` 产 measured 冻结(禁手写 golden)",
            args.band
        )),
    };
    let golden_digest_hex = match &band_text {
        Some(t) => json_str(t, "event_log_digest")
            .unwrap_or_else(|| fail("冻结带缺 event_log_digest"))
            .to_string(),
        None => hex(&event_digest), // freeze 首跑:以本 run 双跑一致性自标定
    };
    let golden_events_ok = if args.freeze {
        double_run_ok
    } else {
        hex(&event_digest) == golden_digest_hex
    };
    let golden_events_accepted = wp::validate_event_log(&events).is_ok();
    if !golden_events_ok || !golden_events_accepted {
        failures.push(format!(
            "四事件 golden: digest={} golden={} accepted={}",
            hex(&event_digest),
            golden_digest_hex,
            golden_events_accepted
        ));
    }
    let event_arm_ok = red_arm_event_order().is_ok();
    if !event_arm_ok {
        failures.push("事件乱序 RED 臂失效".into());
    }

    // ── 步骤 7:HLOD 工具双构建 + 产物接入 cell 元数据 ──
    let (hlod_bytes, hlod_digest) = rxhlod_double_build(&args.work_dir);
    let (hlod_bytes_b, hlod_digest_b) = rxhlod_double_build(&args.work_dir.join("rerun"));
    let rerun_equal = hlod_bytes == hlod_bytes_b && hlod_digest == hlod_digest_b;
    let band_hlod = match &band_text {
        Some(t) => json_str(t, "hlod_digest")
            .unwrap_or_else(|| fail("冻结带缺 hlod_digest"))
            .to_string(),
        None => hex(&hlod_digest), // freeze 首跑自标定
    };
    let hlod_double_ok = rerun_equal
        && if args.freeze {
            true
        } else {
            hex(&hlod_digest) == band_hlod
        };
    let hlod_check_ok = rxhlod_check();
    // 产物 digest 接入 cell HLOD 层级引用字段并往返。
    let mut wired = wp::canonical_world();
    wired.cells[0].hlod = Some(wp::CellHlodRef {
        digest: hlod_digest,
        levels: 3,
    });
    let hlod_wire_ok = wp::decode_world(&wp::encode_world(&wired).expect("编码"))
        .expect("解码")
        .cells[0]
        .hlod
        == Some(wp::CellHlodRef {
            digest: hlod_digest,
            levels: 3,
        });
    if !hlod_double_ok || !hlod_check_ok || !hlod_wire_ok {
        failures.push(format!(
            "HLOD 面: double={hlod_double_ok} check={hlod_check_ok} wire={hlod_wire_ok}"
        ));
    }

    // ── 步骤 8:大世界 soak hitch p99 ≤ measured 阈值 ──
    if args.soak_frames < wp::M110_SOAK_MIN_FRAMES {
        fail(&format!(
            "soak 帧数 {} 低于声明阈值 {}",
            args.soak_frames,
            wp::M110_SOAK_MIN_FRAMES
        ));
    }
    let soak_world = wp::soak_world();
    let soak_budget = wp::soak_budget();
    let soak_path = wp::soak_camera_path(args.soak_frames);
    let soak_path_digest = camera_path_digest(&soak_path);
    let records = wp::run_soak(&soak_world, soak_budget, &soak_path).expect("soak");
    let warmup = wp::M110_SOAK_WARMUP_FRAMES as usize;
    let samples: Vec<u64> = records.iter().skip(warmup).map(|r| r.tick_ns).collect();
    let p50_ms = wp::percentile_ns(&samples, 0.50) as f64 / 1e6;
    let p95_ms = wp::percentile_ns(&samples, 0.95) as f64 / 1e6;
    let p99_ms = wp::percentile_ns(&samples, 0.99) as f64 / 1e6;
    let max_ms = *samples.iter().max().unwrap_or(&0) as f64 / 1e6;
    let soak_events: u64 = records
        .iter()
        .map(|r| r.events_by_kind.iter().map(|&k| k as u64).sum::<u64>())
        .sum();
    let soak_streamed: u64 = records
        .iter()
        .map(|r| r.budget.streaming_cells_this_frame as u64)
        .sum();
    let soak_counters_nonempty = !samples.is_empty() && soak_events > 0 && soak_streamed > 0;
    // 阈值面:g9_budget.json 实测标定(freeze 模式下自标定,PASS 模式对冻结阈值)。
    let budget_text = std::fs::read_to_string(root.join("milestones/g9/g9_budget.json"))
        .unwrap_or_else(|e| fail(&format!("读 g9_budget.json: {e}")));
    let budget_entry = extract_budget_entry(&budget_text, BUDGET_ENTRY_ID);
    let (threshold_ms, budget_frozen_measured) = match budget_entry {
        Some((t, m)) => (t, Some(m)),
        None => (f64::INFINITY, None),
    };
    let hitch_within = if args.freeze {
        true // freeze 模式自标定,阈值随本 run measured 落盘
    } else {
        budget_entry.is_some() && p99_ms <= threshold_ms
    };
    let budget_provenance_ok = args.freeze
        || band_text
            .as_deref()
            .and_then(|t| json_str(t, "soak_hitch_p99_ms"))
            .and_then(|s| s.parse::<f64>().ok())
            .zip(budget_frozen_measured)
            .map(|(a, b)| (a - b).abs() < 1e-9)
            .unwrap_or(false);
    if !soak_counters_nonempty || !hitch_within || !budget_provenance_ok {
        failures.push(format!(
            "soak 面: nonempty={soak_counters_nonempty} p99={p99_ms}ms threshold={threshold_ms}ms within={hitch_within} provenance={budget_provenance_ok}"
        ));
    }

    // ── 步骤 9:freeze 落盘(measured 冻结 + provenance)──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m110.world_partition_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖,M110 语义面 = 数据模型+调度+预算计数+事件总线;GPU 非必需)\"}},\n  \
             \"freeze_rule\": \"golden digest = canonical 场景实测事件日志 SHA-256(双跑位级一致后冻结,禁手写);hitch 阈值 = measured p99 × {:.1} 落 g9_budget.json 条目 {}(P-09)\",\n  \
             \"spec_anchor\": \"RXS-0363\",\n  \
             \"world_digest\": \"{}\",\n  \
             \"canonical_frames\": {},\n  \
             \"canonical_budget\": {{\"MaxStreamingCellsPerFrame\": {}, \"MaxActorsToSpawnPerFrame\": {}, \"MemoryBudgetMB\": {}}},\n  \
             \"camera_path_digest\": \"{}\",\n  \
             \"event_log_digest\": \"{}\",\n  \
             \"event_count\": {},\n  \
             \"hlod_digest\": \"{}\",\n  \
             \"hlod_bytes\": {},\n  \
             \"hlod_tool\": \"rurix.hlod.bake.v1\",\n  \
             \"soak_frames\": {},\n  \
             \"soak_warmup_frames\": {},\n  \
             \"soak_world_digest\": \"{}\",\n  \
             \"soak_path_digest\": \"{}\",\n  \
             \"soak_hitch_p99_ms\": \"{:.6}\",\n  \
             \"soak_hitch_p99_ns\": {},\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-m110-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            wp::M110_HITCH_THRESHOLD_MARGIN,
            BUDGET_ENTRY_ID,
            hex(&wp::world_digest(&world).expect("digest")),
            CANONICAL_FRAMES,
            wp::canonical_budget().max_streaming_cells_per_frame,
            wp::canonical_budget().max_actors_to_spawn_per_frame,
            wp::canonical_budget().memory_budget_mb,
            hex(&camera_path_digest(&wp::canonical_camera_path(CANONICAL_FRAMES))),
            hex(&event_digest),
            events.len(),
            hex(&hlod_digest),
            hlod_bytes.len(),
            args.soak_frames,
            wp::M110_SOAK_WARMUP_FRAMES,
            hex(&wp::world_digest(&soak_world).expect("digest")),
            hex(&soak_path_digest),
            p99_ms,
            wp::percentile_ns(&samples, 0.99),
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}(measured p99 = {p99_ms:.6} ms)", args.band);
    }

    // ── 步骤 10:evidence(rurix.g9m110.world_partition.v1)──
    let checks: [(&str, bool); 10] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("schema_roundtrip_and_cell_frozen", schema_ok),
        ("data_layer_reserved_unwired", data_layer_ok),
        ("budget_counters_per_frame_nonempty", per_frame_nonempty && double_run_ok),
        ("budget_violation_queued_demote_red_arm", budget_arm_ok),
        ("cell_event_sequence_golden_equal", golden_events_ok && golden_events_accepted),
        ("event_out_of_order_red_arm", event_arm_ok),
        (
            "hlod_double_build_hash_equal",
            hlod_double_ok && hlod_check_ok && hlod_wire_ok,
        ),
        ("soak_hitch_p99_within_measured_threshold", hitch_within && soak_counters_nonempty),
        ("budget_threshold_provenance", budget_provenance_ok),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let frames_json: Vec<String> = frames
        .iter()
        .map(|ev| {
            format!(
                "{{\"frame\": {}, \"target\": {}, \"resident\": {}, \"cells\": {}, \"actors\": {}, \"mem\": {}, \"resident_mem\": {}, \"unloaded\": {}, \"queue\": {}, \"stall\": {}}}",
                ev.frame,
                ev.target_cells,
                ev.resident_cells,
                ev.streaming_cells_this_frame,
                ev.actors_spawned_this_frame,
                ev.memory_bytes_this_frame,
                ev.resident_memory_bytes,
                ev.cells_unloaded,
                ev.queue_depth_end,
                ev.budget_stall
            )
        })
        .collect();
    let soak_ns_json: Vec<String> = records.iter().map(|r| r.tick_ns.to_string()).collect();
    let soak_cells_json: Vec<String> = records
        .iter()
        .map(|r| r.budget.streaming_cells_this_frame.to_string())
        .collect();
    let soak_actors_json: Vec<String> = records
        .iter()
        .map(|r| r.budget.actors_spawned_this_frame.to_string())
        .collect();
    let soak_mem_json: Vec<String> = records
        .iter()
        .map(|r| r.budget.memory_bytes_this_frame.to_string())
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit =
        std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let threshold_json = if threshold_ms.is_finite() {
        format!("{threshold_ms}")
    } else {
        "null".to_string()
    };
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m110.world_partition.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m110_world_partition\",\n  \"spec_anchor\": \"RXS-0363\",\n  \
         \"assertion_id\": \"g9.p0.m110.world_partition\",\n  \"milestone\": \"M110\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;GPU 非必需——M110 语义面 = 数据模型+调度+预算计数+事件总线)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"determinism_protocol\": {{\"rng\": \"LCG(Knuth MMIX)wrapping u64 位级确定\", \"camera_path\": \"闭式整数推进三角波\", \"collections\": \"BTreeSet/Vec 迭代序确定\", \"digest_domain\": \"sha256(canonical LE 编码)\"}},\n  \
         \"budget_contract\": {{\"caps\": {{\"MaxStreamingCellsPerFrame\": {}, \"MaxActorsToSpawnPerFrame\": {}, \"MemoryBudgetMB\": {}}}, \"canonical_frames\": [{}], \
         \"red_arm_budget_overrun\": {{\"injected\": \"MaxStreamingCellsPerFrame=0\", \"queued_demote_visible\": {}, \"sabotage_probe_zero_stall\": true, \"silent_overrun_checker_effective\": true}}}},\n  \
         \"cell_events\": {{\"closed_set\": [\"CellLoadBegin\", \"CellResident\", \"CellUnloadBegin\", \"CellEvicted\"], \"digest\": \"{}\", \"golden_digest\": \"{}\", \"golden_equal\": {}, \"count\": {}, \"out_of_order_red_arm\": {}}},\n  \
         \"data_layer\": {{\"reserved_only\": true, \"wired\": false, \"mask_roundtrip\": {}, \"query_fail_closed\": {}, \"behavior_invariant\": {}}},\n  \
         \"hlod\": {{\"tool\": \"rurix.hlod.bake.v1\", \"gpu_required\": false, \"digest\": \"{}\", \"bytes\": {}, \"double_build_equal\": {}, \"order_invariant_and_drift_detected\": {}, \"wired_into_cell_meta\": {}}},\n  \
         \"soak\": {{\"world_cells\": {}, \"spatial_objects\": {}, \"frames\": {}, \"min_frames_declared\": {}, \"warmup_frames\": {}, \
         \"hitch\": {{\"p50_ms\": {:.6}, \"p95_ms\": {:.6}, \"p99_ms\": {:.6}, \"max_ms\": {:.6}}}, \
         \"threshold_ms\": {}, \"threshold_source\": \"g9_budget.json:{}\", \"within_threshold\": {}, \
         \"budget_caps\": {{\"MaxStreamingCellsPerFrame\": {}, \"MaxActorsToSpawnPerFrame\": {}, \"MemoryBudgetMB\": {}}}, \
         \"total_events\": {}, \"total_cells_streamed\": {}, \
         \"per_frame_tick_ns\": [{}], \"per_frame_cells\": [{}], \"per_frame_actors\": [{}], \"per_frame_mem_bytes\": [{}]}},\n  \
         \"results\": {{\"metrics\": {{\"world_partition_hitch_p99_ms\": {:.6}}}, \"unit\": {{\"world_partition_hitch_p99_ms\": \"ms\"}}}},\n  \
         \"conformance_corpus\": {{\"dir\": \"conformance/world_partition\", \"files\": {}, \"anchors\": {{{}}}}},\n  \
         \"checks\": {{{}}},\n  \
         \"commands\": [{}],\n  \
         \"failures\": [{}]\n}}",
        if args.freeze { "freeze" } else { "pass" },
        utc_now(),
        json_escape(&base_commit),
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::var("RURIX_REQUIRE_REAL").as_deref() == Ok("1"),
        wp::canonical_budget().max_streaming_cells_per_frame,
        wp::canonical_budget().max_actors_to_spawn_per_frame,
        wp::canonical_budget().memory_budget_mb,
        frames_json.join(", "),
        budget_arm_ok,
        hex(&event_digest),
        golden_digest_hex,
        golden_events_ok,
        events.len(),
        event_arm_ok,
        mask_roundtrip,
        mask_query_fail_closed,
        mask_behavior_invariant,
        hex(&hlod_digest),
        hlod_bytes.len(),
        hlod_double_ok,
        hlod_check_ok,
        hlod_wire_ok,
        soak_world.cells.len(),
        soak_world.spatially_loaded.len(),
        args.soak_frames,
        wp::M110_SOAK_MIN_FRAMES,
        wp::M110_SOAK_WARMUP_FRAMES,
        p50_ms,
        p95_ms,
        p99_ms,
        max_ms,
        threshold_json,
        BUDGET_ENTRY_ID,
        hitch_within,
        soak_budget.max_streaming_cells_per_frame,
        soak_budget.max_actors_to_spawn_per_frame,
        soak_budget.memory_budget_mb,
        soak_events,
        soak_streamed,
        soak_ns_json.join(","),
        soak_cells_json.join(","),
        soak_actors_json.join(","),
        soak_mem_json.join(","),
        p99_ms,
        CORPUS_FILES.len(),
        anchors_json.join(", "),
        checks_json.join(", "),
        std::env::args()
            .map(|a| format!("\"{}\"", json_escape(&a)))
            .collect::<Vec<_>>()
            .join(", "),
        failures_json.join(", "),
    );
    if let Some(p) = &args.evidence {
        std::fs::write(p, &json).unwrap_or_else(|e| fail(&format!("写 evidence {p:?}: {e}")));
        println!("{TAG}: evidence 已落盘 {p:?}");
    }
    println!("{json}");
    if failures.is_empty() {
        println!(
            "{TAG}: PASS schema 往返 + 三项预算逐帧 evidence + 预算违约排队降级 RED + 四事件逐字 golden + Data Layer 预留不接线 + HLOD 双构建 + soak hitch p99 {p99_ms:.6}ms ≤ {threshold_json}ms(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
