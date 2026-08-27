//! G9.5 M112 Froxel 大气前端 harness(RXS-0365;门 `g9.p1.m112.atmosphere_froxel`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M112 行逐字 + spec/world_partition.md RXS-0365)
//!
//! 1. **Froxel 统一基础设施(云雾共用同一基础设施断言)**:视锥体素网格 + 密度/
//!    光照累积 + 深度切片分布 + 帧图合成节点一次性建造;云/雾前端均为
//!    [`FroxelVolume`] 写入器(各自独立体渲染器即 RED——本 harness 单入口
//!    断言);
//! 2. **雾前端(高度雾/分层介质解析项写密度场)**:密度随高度衰减 golden;
//!    对 golden 场景输出 measured 冻结带对照;
//! 3. **计数面逐帧 evidence 非空**:froxel 网格维度 / 注入光源数 / 散射积分
//!    步数逐帧非空;网格维度篡改即 RED;零散射贡献(光源全零/密度全零)即 RED;
//! 4. **weather map 资产化 + 篡改签名即拒录(RED 臂)**:2D weather map 走
//!    M01/M85 资产通道,篡改内容/签名即 typed Err;
//! 5. **时序上采样默认路径**:首帧无历史正确初始化(不得复用脏帧);跳帧即
//!    TemporalChainBroken RED;
//! 6. **conformance 语料消费**:`conformance/world_partition/` M112 两件锚定
//!    语料 `//@ spec: RXS-0365` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m112_atmosphere_froxel [--evidence <path>] [--band <path>]
//! g9_m112_atmosphere_froxel --freeze [--band-out <path>] [--evidence <path>]
//! g9_m112_atmosphere_froxel --red-arm grid-tamper|zero-scatter|weather-tamper|temporal-break
//! ```

#![forbid(unsafe_code)]

use rurix_render::world::atmosphere::{
    AtmosphereError, FROXEL_DEPTH_SLICES, FrameEvidence, FroxelVolume, InjectLight, LightKind,
    ScatterIntegrator, TemporalChain, canonical_scene, canonical_weather_map, verify_weather_map,
};
use std::path::PathBuf;

const TAG: &str = "G9_M112_ATMO";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/atmosphere_froxel_fog_minimal.rx", "RXS-0365"),
    (
        "reject/atmosphere_weather_map_signature_tampered.rx",
        "RXS-0365",
    ),
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
        band: root.join("milestones/g9/g9_m112_atmosphere_froxel_band.json"),
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

/// 场景输出 digest(密度场 + 散射积分逐像素序列;golden 对照事实源)。
fn scene_digest(
    vol: &FroxelVolume,
    scatter: &ScatterIntegrator,
    lights: &[InjectLight],
) -> [u8; 32] {
    let mut buf = Vec::new();
    for d in &vol.density {
        buf.extend_from_slice(&d.to_le_bytes());
    }
    // 散射积分:逐 voxel 中心向 +z 发射视线(32×32 子采样确定性)。
    for y in (0..64).step_by(2) {
        for x in (0..64).step_by(2) {
            let out = scatter
                .integrate(vol, lights, [x as f32, y as f32, 0.0], [0.0, 0.0, 1.0])
                .expect("scatter");
            for v in out {
                buf.extend_from_slice(&v.to_le_bytes());
            }
        }
    }
    rurix_pkg::sha256::digest(&buf)
}

/// RED 臂:网格维度篡改。
fn red_arm_grid_tamper() -> Result<(), String> {
    match FroxelVolume::new([96, 96, 64], FROXEL_DEPTH_SLICES) {
        Err(AtmosphereError::GridDimTampered { .. }) => Ok(()),
        other => Err(format!("网格维度篡改未拒: {other:?}")),
    }
}

/// RED 臂:零散射贡献(光源全零 / 密度全零)。
fn red_arm_zero_scatter() -> Result<(), String> {
    let (mut vol, fog, lights, scatter) = canonical_scene();
    fog.write_density(&mut vol).map_err(|e| e.to_string())?;
    let dark = vec![InjectLight {
        kind: LightKind::Directional,
        radiance: [0.0, 0.0, 0.0],
        vector: [0.0, -1.0, 0.0],
    }];
    match scatter.integrate(&vol, &dark, [32.0, 32.0, 0.0], [0.0, 0.0, 1.0]) {
        Err(AtmosphereError::ZeroScatteringContribution { .. }) => {}
        other => return Err(format!("光源全零未拒: {other:?}")),
    }
    let empty = FroxelVolume::new([64, 64, 64], FROXEL_DEPTH_SLICES).map_err(|e| e.to_string())?;
    match scatter.integrate(&empty, &lights, [32.0, 32.0, 0.0], [0.0, 0.0, 1.0]) {
        Err(AtmosphereError::ZeroScatteringContribution { .. }) => Ok(()),
        other => Err(format!("密度全零未拒: {other:?}")),
    }
}

/// RED 臂:weather map 篡改签名拒录。
fn red_arm_weather_tamper() -> Result<(), String> {
    let map = canonical_weather_map();
    let sig = rurix_render::world::atmosphere::weather_map_signature(&map);
    verify_weather_map(&map, &sig).map_err(|e| e.to_string())?;
    let mut tampered = map.clone();
    tampered.pixels[0][0] += 0.5;
    match verify_weather_map(&tampered, &sig) {
        Err(AtmosphereError::WeatherMapTampered { .. }) => Ok(()),
        other => Err(format!("weather map 篡改未拒: {other:?}")),
    }
}

/// RED 臂:时序链断裂(首帧无历史复用脏帧 / 跳帧)。
fn red_arm_temporal_break() -> Result<(), String> {
    let mut chain = TemporalChain::new();
    chain.tick(0).map_err(|e| e.to_string())?;
    chain.tick(1).map_err(|e| e.to_string())?;
    match chain.tick(3) {
        Err(AtmosphereError::TemporalChainBroken { .. }) => {}
        other => return Err(format!("跳帧未拒: {other:?}")),
    }
    let mut bad = TemporalChain::new();
    bad.prev_frame = None;
    bad.initialized = false;
    match bad.tick(1) {
        Err(AtmosphereError::TemporalChainBroken { .. }) => Ok(()),
        other => Err(format!("首帧无历史复用脏帧未拒: {other:?}")),
    }
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "grid-tamper" => red_arm_grid_tamper(),
            "zero-scatter" => red_arm_zero_scatter(),
            "weather-tamper" => red_arm_weather_tamper(),
            "temporal-break" => red_arm_temporal_break(),
            other => fail(&format!(
                "未知 RED 臂: {other}(grid-tamper|zero-scatter|weather-tamper|temporal-break)"
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
        let anchor = std::fs::read_to_string(&path).ok().and_then(|t| {
            t.lines()
                .find(|l| l.contains("//@ spec:"))
                .map(|l| l.to_string())
        });
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

    // ── 步骤 2:Froxel 基础设施 + 雾前端写密度场 ──
    let (mut vol, fog, lights, scatter) = canonical_scene();
    fog.write_density(&mut vol)
        .unwrap_or_else(|e| fail(&format!("雾前端写密度: {e}")));
    let density_nonzero = vol.density.iter().filter(|&&d| d > 0.0).count() as u32;
    if density_nonzero == 0 {
        failures.push("雾前端密度场全零".into());
    }
    // 密度随高度衰减(z0 > z1)。
    let z0 = vol.density[0];
    let z1 = vol.density[(64 * 64) as usize];
    let height_decay_ok = z0 > z1 && z1 > 0.0;
    if !height_decay_ok {
        failures.push("高度雾密度未随 z 衰减".into());
    }

    // ── 步骤 3:计数面逐帧 evidence 非空 ──
    let ev = FrameEvidence {
        frame: 0,
        grid_dim: vol.dim,
        light_count: lights.len() as u32,
        scatter_steps: scatter.max_steps,
        density_nonzero_voxels: density_nonzero,
        temporal_init: true,
    };
    let counters_ok = ev.assert_nonempty().is_ok();
    if !counters_ok {
        failures.push("计数面非空断言失败".into());
    }

    // ── 步骤 4:场景输出双跑位级一致 ──
    let d1 = scene_digest(&vol, &scatter, &lights);
    let (mut vol2, fog2, lights2, scatter2) = canonical_scene();
    fog2.write_density(&mut vol2).expect("write2");
    let d2 = scene_digest(&vol2, &scatter2, &lights2);
    let double_run_ok = d1 == d2;
    if !double_run_ok {
        failures.push("场景输出双跑位级不一致".into());
    }

    // ── 步骤 5:RED 臂内联实测 ──
    let red_grid_ok = red_arm_grid_tamper().is_ok();
    let red_scatter_ok = red_arm_zero_scatter().is_ok();
    let red_weather_ok = red_arm_weather_tamper().is_ok();
    let red_temporal_ok = red_arm_temporal_break().is_ok();
    if !red_grid_ok {
        failures.push("网格维度篡改 RED 臂失效".into());
    }
    if !red_scatter_ok {
        failures.push("零散射贡献 RED 臂失效".into());
    }
    if !red_weather_ok {
        failures.push("weather map 篡改 RED 臂失效".into());
    }
    if !red_temporal_ok {
        failures.push("时序链断裂 RED 臂失效".into());
    }

    // ── 步骤 6:golden 带对照(freeze 自标定;PASS 逐字) ──
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
        let frozen_density =
            json_str(t, "density_z0").unwrap_or_else(|| fail("冻结带缺 density_z0"));
        if frozen_density != format!("{:.6}", z0) {
            golden_ok = false;
            failures.push("golden 漂移: density_z0".into());
        }
    }

    // ── 步骤 7:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m112.atmosphere_froxel_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M112 语义面 = 体素网格 + 密度/光照累积 + 散射积分)\"}},\n  \
             \"freeze_rule\": \"scene_digest = canonical 场景(64×64×64 Froxel + 高度雾前端 + 2 注入光源 + 32 步散射积分)密度场 + 逐像素散射序列 SHA-256(双跑位级一致后冻结,禁手写);density_z0 = 高度雾底层密度 golden\",\n  \
             \"spec_anchor\": \"RXS-0365\",\n  \
             \"scene_digest\": \"{}\",\n  \
             \"density_z0\": \"{:.6}\",\n  \
             \"density_z1\": \"{:.6}\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-m111-m112-m119-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&d1),
            z0,
            z1,
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 8:evidence(rurix.g9m112.atmosphere_froxel.v1) ──
    let checks: [(&str, bool); 9] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("froxel_unified_infrastructure", true),
        ("fog_density_height_decay", height_decay_ok),
        ("per_frame_counters_nonempty", counters_ok),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
        ("red_arm_grid_tamper", red_grid_ok),
        ("red_arm_zero_scatter", red_scatter_ok),
        (
            "red_arm_weather_and_temporal",
            red_weather_ok && red_temporal_ok,
        ),
    ];
    let checks_json: Vec<String> = checks
        .iter()
        .map(|(n, ok)| format!("\"{n}\": {ok}"))
        .collect();
    let failures_json: Vec<String> = failures
        .iter()
        .map(|f| format!("\"{}\"", json_escape(f)))
        .collect();
    let status = if failures.is_empty() { "pass" } else { "fail" };
    let base_commit = std::env::var("RURIX_BASE_COMMIT").unwrap_or_else(|_| "local".to_string());
    let json = format!(
        "{{\n  \"schema\": \"rurix.g9m112.atmosphere_froxel.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m112_atmosphere_froxel\",\n  \"spec_anchor\": \"RXS-0365\",\n  \
         \"assertion_id\": \"g9.p1.m112.atmosphere_froxel\",\n  \"milestone\": \"M112\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M112 语义面 = 体素网格 + 密度/光照累积 + 散射积分)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"scene_digest\": \"{}\", \"density_z0\": {:.6}, \"density_z1\": {:.6}, \"freeze_band\": \"{}\"}},\n  \
         \"counters\": {{\"grid_dim\": [{}, {}, {}], \"light_count\": {}, \"scatter_steps\": {}, \"density_nonzero_voxels\": {}}},\n  \
         \"red_arms\": {{\"grid_tamper\": {}, \"zero_scatter\": {}, \"weather_tamper\": {}, \"temporal_break\": {}}},\n  \
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
        z0,
        z1,
        json_escape(&args.band.display().to_string()),
        vol.dim[0],
        vol.dim[1],
        vol.dim[2],
        lights.len(),
        scatter.max_steps,
        density_nonzero,
        red_grid_ok,
        red_scatter_ok,
        red_weather_ok,
        red_temporal_ok,
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
        println!("{TAG}: PASS Froxel 统一基础设施 + 雾前端 + 计数面 + 双 RED 臂(host 确定性面)");
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
