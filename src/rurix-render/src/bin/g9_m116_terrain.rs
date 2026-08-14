//! G9.5 M116 地形 harness(RXS-0367;门 `g9.p1.m116.terrain_chunk_cell`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M116 行逐字 + spec/world_partition.md RXS-0367)
//!
//! 1. **chunk ≡ cell 断言**:地形 chunk 与 M110 cell 同一网格族(数量 1:1、coord
//!    同族、边长同一资产属性),出现独立地形分格(第二套网格注入)即 RED;
//! 2. **全 compute LOD/剔除/缝合**:LOD 选择/视锥剔除/邻级缝合全进 compute 产
//!    indirect draw;CPU 侧零逐 chunk 提交断言(计数非零即 RED);
//! 3. **toroidal 更新**:环形窗口滚动复用 ring buffer(复用/加载/占位计数逐帧
//!    evidence),chunk 页迟到 → 父级 LOD 占位;
//! 4. **零 SVT 依赖断言**:SVT/RVT/sampler feedback 依赖注入即 RED;
//! 5. **缝合裂缝 RED 臂**:相邻 chunk LOD 差 >1 注入必须触发缝合路径,出现裂缝
//!    像素即 RED;邻级缝合处顶点位置连续性 golden(裂缝=0);
//! 6. **conformance 语料消费**:`conformance/world_partition/` M116 两件锚定
//!    语料 `//@ spec: RXS-0367` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m116_terrain [--evidence <path>] [--band <path>]
//! g9_m116_terrain --freeze [--band-out <path>] [--evidence <path>]
//! g9_m116_terrain --red-arm second-grid|svt-inject|lod-gap-unstitched|stitch-crack|cpu-submit
//! ```

#![forbid(unsafe_code)]

use rurix_render::world::partition::{CellMeta, CellCoord, PersistentWorld};
use rurix_render::world::terrain::{
    assert_chunk_eq_cell, assert_no_second_grid, assert_zero_cpu_submit,
    assert_zero_svt_dependency, build_chunks_from_cells, build_indirect_draws,
    canonical_chunks, canonical_heightfield, scene_digest, verify_seam, AssetDependencyDesc,
    ForeignGridDesc, IndirectDrawBatch, TerrainError, ToroidalRing,
};
use std::collections::BTreeMap;
use std::path::PathBuf;

const TAG: &str = "G9_M116_TERRAIN";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/terrain_chunk_cell_aligned_minimal.rx", "RXS-0367"),
    ("reject/terrain_lod_gap_crack.rx", "RXS-0367"),
];

/// canonical 视锥六平面(大包围盒,不剔除任何 canonical chunk)。
const PLANES: [[f32; 4]; 6] = [
    [1.0, 0.0, 0.0, 4096.0],
    [-1.0, 0.0, 0.0, 4096.0],
    [0.0, 1.0, 0.0, 4096.0],
    [0.0, -1.0, 0.0, 4096.0],
    [0.0, 0.0, 1.0, 4096.0],
    [0.0, 0.0, -1.0, 4096.0],
];

fn fail(msg: &str) -> ! {
    eprintln!("{TAG}: FAIL {msg}");
    std::process::exit(1)
}

fn hex(d: &[u8; 32]) -> String {
    d.iter().map(|b| format!("{b:02x}")).collect()
}

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

fn json_str<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let needle = format!("\"{key}\": \"");
    let start = text.find(&needle)? + needle.len();
    let end = text[start..].find('"')? + start;
    Some(&text[start..end])
}

struct Args {
    evidence: Option<PathBuf>,
    band: PathBuf,
    freeze: bool,
    red_arm: Option<String>,
}

fn parse_args() -> Args {
    let root = workspace_root();
    let mut out = Args {
        evidence: None,
        band: root.join("milestones/g9/g9_m116_terrain_band.json"),
        freeze: false,
        red_arm: None,
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
            "--freeze" => out.freeze = true,
            "--red-arm" => out.red_arm = Some(take(&mut i)),
            other => fail(&format!("未知参数: {other}")),
        }
        i += 1;
    }
    out
}

/// canonical 3×1 cell 世界(chunk ≡ cell 消费面事实源)。
fn mini_world() -> PersistentWorld {
    let cells = (0..3u32)
        .map(|i| CellMeta {
            coord: CellCoord { x: i as i32, y: 0 },
            bounds_min: [i as f32 * 64.0, 0.0, 0.0],
            bounds_max: [(i as f32 + 1.0) * 64.0, 64.0, 64.0],
            page_refs: vec![],
            hlod: None,
            data_layer_mask: 0,
        })
        .collect();
    PersistentWorld {
        cell_size_m: 64.0,
        grid_min: CellCoord { x: 0, y: 0 },
        grid_max: CellCoord { x: 2, y: 0 },
        cells,
        always_loaded: vec![],
        spatially_loaded: vec![],
    }
}

fn canonical_assets() -> BTreeMap<u32, rurix_render::world::terrain::HeightfieldAsset> {
    let mut m = BTreeMap::new();
    for (i, c) in canonical_chunks().iter().enumerate() {
        m.insert(i as u32, c.heightfield.clone());
    }
    m
}

/// RED 臂:第二套分格注入。
fn red_arm_second_grid() -> Result<(), String> {
    let world = mini_world();
    match assert_no_second_grid(&world, &ForeignGridDesc { cell_size_m: 32.0, origin_m: [0.0, 0.0] }) {
        Err(TerrainError::SecondGridDetected { .. }) => {}
        other => return Err(format!("独立边长分格未拒: {other:?}")),
    }
    match assert_no_second_grid(&world, &ForeignGridDesc { cell_size_m: 64.0, origin_m: [8.0, 0.0] }) {
        Err(TerrainError::SecondGridDetected { .. }) => {}
        other => return Err(format!("偏移原点分格未拒: {other:?}")),
    }
    // sabotage 探针(能红证明):同族网格合法。
    assert_no_second_grid(&world, &ForeignGridDesc { cell_size_m: 64.0, origin_m: [192.0, 0.0] })
        .map_err(|e| format!("同族网格被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:SVT 依赖注入。
fn red_arm_svt_inject() -> Result<(), String> {
    for desc in [
        AssetDependencyDesc { uses_svt: true, ..Default::default() },
        AssetDependencyDesc { uses_rvt: true, ..Default::default() },
        AssetDependencyDesc { uses_sampler_feedback: true, ..Default::default() },
    ] {
        match assert_zero_svt_dependency(&desc) {
            Err(TerrainError::SvtDependencyDetected { .. }) => {}
            other => return Err(format!("SVT 依赖 {desc:?} 未拒: {other:?}")),
        }
    }
    assert_zero_svt_dependency(&AssetDependencyDesc::default())
        .map_err(|e| format!("零依赖被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:邻级 LOD 差 >1 未走缝合路径。
fn red_arm_lod_gap_unstitched() -> Result<(), String> {
    let mut chunks = canonical_chunks();
    chunks[0].lod = 0;
    chunks[1].lod = 2;
    match verify_seam(&chunks[0], &chunks[1], false) {
        Err(TerrainError::LodGapUnstitched { lod_delta: 2 }) => {}
        other => return Err(format!("LOD 差>1 未缝合注入未拒: {other:?}")),
    }
    // sabotage:走缝合路径的同族高度场必须裂缝=0。
    let report = verify_seam(&chunks[0], &chunks[1], true).map_err(|e| format!("合法缝合被误拒: {e}"))?;
    if !report.stitch_invoked || report.crack_pixels != 0 {
        return Err(format!("缝合报告异常: {report:?}"));
    }
    Ok(())
}

/// RED 臂:缝合裂缝像素注入(边界高度篡改)。
fn red_arm_stitch_crack() -> Result<(), String> {
    let mut chunks = canonical_chunks();
    chunks[0].lod = 0;
    chunks[1].lod = 2;
    chunks[1].heightfield = canonical_heightfield(1, 5.0); // 边界列抬高 ⇒ 裂缝
    match verify_seam(&chunks[0], &chunks[1], true) {
        Err(TerrainError::StitchCrackPixels { count }) if count > 0 => Ok(()),
        other => Err(format!("裂缝注入未检出: {other:?}")),
    }
}

/// RED 臂:CPU 侧逐 chunk 提交注入。
fn red_arm_cpu_submit() -> Result<(), String> {
    let bad = IndirectDrawBatch { records: vec![], cpu_per_chunk_submits: 3 };
    match assert_zero_cpu_submit(&bad) {
        Err(TerrainError::CpuPerChunkSubmit { count: 3 }) => {}
        other => return Err(format!("CPU 逐 chunk 提交注入未拒: {other:?}")),
    }
    let good = IndirectDrawBatch { records: vec![], cpu_per_chunk_submits: 0 };
    assert_zero_cpu_submit(&good).map_err(|e| format!("零提交被误拒: {e}"))?;
    Ok(())
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "second-grid" => red_arm_second_grid(),
            "svt-inject" => red_arm_svt_inject(),
            "lod-gap-unstitched" => red_arm_lod_gap_unstitched(),
            "stitch-crack" => red_arm_stitch_crack(),
            "cpu-submit" => red_arm_cpu_submit(),
            other => fail(&format!(
                "未知 RED 臂: {other}(second-grid|svt-inject|lod-gap-unstitched|stitch-crack|cpu-submit)"
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

    let mut failures: Vec<String> = Vec::new();

    // ── 步骤 1:conformance 语料锚定核验 ──
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
        println!("{TAG}: conformance 语料 2 件锚定核验通过");
    }

    // ── 步骤 2:chunk ≡ cell 结构性断言 ──
    let world = mini_world();
    let assets = canonical_assets();
    let chunks = build_chunks_from_cells(&world, &assets)
        .unwrap_or_else(|e| fail(&format!("chunk 构造: {e}")));
    let chunk_eq_cell_ok = assert_chunk_eq_cell(&world, &chunks).is_ok();
    if !chunk_eq_cell_ok {
        failures.push("chunk ≡ cell 结构性断言失败".into());
    }

    // ── 步骤 3:全 compute LOD/剔除/indirect draw + 零 CPU 提交 ──
    let batch = build_indirect_draws(&chunks, [32.0, 32.0, 10.0], &PLANES, world.cell_size_m)
        .unwrap_or_else(|e| fail(&format!("indirect 批次: {e}")));
    let zero_cpu_ok = assert_zero_cpu_submit(&batch).is_ok();
    if !zero_cpu_ok {
        failures.push("CPU 零逐 chunk 提交断言失败".into());
    }
    if batch.records.len() != 3 {
        failures.push(format!("indirect 记录数 {} ≠ 3", batch.records.len()));
    }

    // ── 步骤 4:邻级缝合(LOD 差>1 触发缝合路径,裂缝=0) ──
    let mut seam_chunks = canonical_chunks();
    seam_chunks[0].lod = 0;
    seam_chunks[1].lod = 2;
    let seam = verify_seam(&seam_chunks[0], &seam_chunks[1], true)
        .unwrap_or_else(|e| fail(&format!("缝合校验: {e}")));
    let seam_ok = seam.stitch_invoked && seam.crack_pixels == 0;
    if !seam_ok {
        failures.push(format!("缝合连续性失败: {seam:?}"));
    }
    // 同级对拍(LOD 差 0,不触发缝合但位置仍连续)。
    let seam0 = verify_seam(&seam_chunks[1], &seam_chunks[2], true)
        .unwrap_or_else(|e| fail(&format!("同级缝合校验: {e}")));
    let seam0_ok = seam0.crack_pixels == 0;
    if !seam0_ok {
        failures.push(format!("同级边界连续性失败: {seam0:?}"));
    }

    // ── 步骤 5:toroidal 更新(复用/占位计数) ──
    let mut ring = ToroidalRing::new(CellCoord { x: 0, y: 0 });
    let resident = std::collections::BTreeSet::from([0u32, 1, 2]);
    let r1 = ring
        .recenter(CellCoord { x: 0, y: 0 }, &resident, &world)
        .unwrap_or_else(|e| fail(&format!("toroidal 初帧: {e}")));
    let r2 = ring
        .recenter(CellCoord { x: 1, y: 0 }, &resident, &world)
        .unwrap_or_else(|e| fail(&format!("toroidal 滚动: {e}")));
    let toroidal_ok = r1.loaded == 3 && r2.reused >= 2;
    if !toroidal_ok {
        failures.push(format!("toroidal 复用计数异常: r1={r1:?} r2={r2:?}"));
    }
    // 页迟到 → 父级 LOD 占位。
    let r3 = ring
        .recenter(CellCoord { x: 0, y: 0 }, &std::collections::BTreeSet::from([0u32, 1]), &world)
        .unwrap_or_else(|e| fail(&format!("toroidal 迟到: {e}")));
    let placeholder_ok = r3.placeholders >= 1;
    if !placeholder_ok {
        failures.push("页迟到父级 LOD 占位未发生".into());
    }

    // ── 步骤 6:场景输出双跑位级一致 + golden 带对照 ──
    let seams = [seam.clone(), seam0.clone()];
    let d1 = scene_digest(&batch, &seams);
    let batch2 = build_indirect_draws(&chunks, [32.0, 32.0, 10.0], &PLANES, world.cell_size_m).expect("b2");
    let d2 = scene_digest(&batch2, &seams);
    let double_run_ok = d1 == d2;
    if !double_run_ok {
        failures.push("场景输出双跑位级不一致".into());
    }

    // ── 步骤 7:RED 臂内联实测 ──
    let red_second_grid_ok = red_arm_second_grid().is_ok();
    let red_svt_ok = red_arm_svt_inject().is_ok();
    let red_lod_gap_ok = red_arm_lod_gap_unstitched().is_ok();
    let red_crack_ok = red_arm_stitch_crack().is_ok();
    let red_cpu_ok = red_arm_cpu_submit().is_ok();
    if !red_second_grid_ok {
        failures.push("第二套分格 RED 臂失效".into());
    }
    if !red_svt_ok {
        failures.push("SVT 依赖注入 RED 臂失效".into());
    }
    if !red_lod_gap_ok {
        failures.push("LOD 差>1 未缝合 RED 臂失效".into());
    }
    if !red_crack_ok {
        failures.push("缝合裂缝 RED 臂失效".into());
    }
    if !red_cpu_ok {
        failures.push("CPU 逐 chunk 提交 RED 臂失效".into());
    }

    // ── 步骤 8:golden 带对照(freeze 自标定;PASS 逐字) ──
    let band_text = match std::fs::read_to_string(&args.band) {
        Ok(t) => Some(t),
        Err(_) if args.freeze => None,
        Err(_) => fail(&format!(
            "冻结带 {:?} 不存在——先跑 `--freeze` 产 measured 冻结(禁手写 golden)",
            args.band
        )),
    };
    let mut golden_ok = true;
    if !args.freeze {
        let t = band_text.as_deref().expect("PASS 模式冻结带必读");
        let frozen = json_str(t, "scene_digest").unwrap_or_else(|| fail("冻结带缺 scene_digest"));
        if frozen != hex(&d1) {
            golden_ok = false;
            failures.push("golden 漂移: scene_digest".into());
        }
        let frozen_records = json_str(t, "indirect_records").unwrap_or_else(|| fail("冻结带缺 indirect_records"));
        if frozen_records != batch.records.len().to_string() {
            golden_ok = false;
            failures.push("golden 漂移: indirect_records".into());
        }
    }

    // ── 步骤 9:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m116.terrain_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M116 语义面 = chunk≡cell 数据模型 + compute 批次产出面 + toroidal 复用计数 + 缝合连续性机核)\"}},\n  \
             \"freeze_rule\": \"scene_digest = canonical 场景(3×1 cell 条带 + 同一世界高度函数 heightfield + LOD 差2 邻级缝合)indirect 批次与缝合报告序列 SHA-256(双跑位级一致后冻结,禁手写);indirect_records = 视锥内 chunk 记录数 golden;crack_pixels = 0 为判据本体不入带\",\n  \
             \"spec_anchor\": \"RXS-0367\",\n  \
             \"scene_digest\": \"{}\",\n  \
             \"indirect_records\": \"{}\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-p1b-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&d1),
            batch.records.len(),
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 10:evidence(rurix.g9m116.terrain.v1) ──
    let checks: [(&str, bool); 11] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("chunk_eq_cell_structural", chunk_eq_cell_ok),
        ("full_compute_lod_cull_indirect", batch.records.len() == 3),
        ("cpu_zero_per_chunk_submit", zero_cpu_ok),
        ("stitch_continuity_crack_zero", seam_ok && seam0_ok),
        ("toroidal_reuse_and_placeholder", toroidal_ok && placeholder_ok),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
        ("red_arm_second_grid_and_svt", red_second_grid_ok && red_svt_ok),
        ("red_arm_lod_gap_and_crack", red_lod_gap_ok && red_crack_ok),
        ("red_arm_cpu_submit", red_cpu_ok),
    ];
    let checks_json: Vec<String> = checks.iter().map(|(n, ok)| format!("\"{n}\": {ok}")).collect();
    let failures_json: Vec<String> = failures.iter().map(|f| format!("\"{}\"", json_escape(f))).collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m116.terrain.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m116_terrain\",\n  \"spec_anchor\": \"RXS-0367\",\n  \
         \"assertion_id\": \"g9.p1.m116.terrain_chunk_cell\",\n  \"milestone\": \"M116\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M116 语义面 = chunk≡cell 数据模型 + compute 批次产出面 + 缝合机核)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"scene_digest\": \"{}\", \"indirect_records\": {}, \"freeze_band\": \"{}\"}},\n  \
         \"counters\": {{\"chunks\": {}, \"indirect_records\": {}, \"cpu_per_chunk_submits\": {}, \"toroidal_reused\": {}, \"toroidal_loaded\": {}, \"toroidal_placeholders\": {}, \"seam_crack_pixels\": {}}},\n  \
         \"red_arms\": {{\"second_grid\": {}, \"svt_inject\": {}, \"lod_gap_unstitched\": {}, \"stitch_crack\": {}, \"cpu_submit\": {}}},\n  \
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
        hex(&d1),
        batch.records.len(),
        json_escape(&args.band.display().to_string()),
        chunks.len(),
        batch.records.len(),
        batch.cpu_per_chunk_submits,
        r2.reused,
        r1.loaded,
        r3.placeholders,
        seam.crack_pixels,
        red_second_grid_ok,
        red_svt_ok,
        red_lod_gap_ok,
        red_crack_ok,
        red_cpu_ok,
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
            "{TAG}: PASS chunk≡cell + 全 compute LOD/剔除/缝合 + toroidal 复用 + 零 SVT + 五 RED 臂(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
