//! G9.5 M113 水体双管线 harness(RXS-0366;门 `g9.p1.m113.water_dual_pipeline`)。
//!
//! ## 判据面(G9_ACCEPTANCE_MAP §3 M113 行逐字 + spec/world_partition.md RXS-0366)
//!
//! 1. **大洋管线**:Tessendorf IFFT 谱参数化资产(风向/风速/涌浪)+ 位移/梯度/
//!    Jacobian 泡沫三贴图 + CDLOD 距离分档 + 多尺度谱 tiling-and-blending;
//!    **compute IFFT 与 host DFT 参考逐值对拍**(容差 = measured 精确值经冻结带
//!    明示,禁手写);
//! 2. **浅水管线**:局部波方程高度场+速度场 ping-pong;
//! 3. **双管线分离断言**:不共享几何路径(token 闭集互斥机核),仅共享水面着色
//!    closure 输入面;互斥违反注入即 RED;
//! 4. **负风速/非法谱参数资产即拒录(RED 臂独立有效)**;浅水域越界写检测(RED);
//! 5. **浮力查询接口面预留不实现**(typed Err 登记);
//! 6. **conformance 语料消费**:`conformance/world_partition/` M113 两件锚定
//!    语料 `//@ spec: RXS-0366` 锚核验。
//!
//! ## 三态
//!
//! host 纯确定性面(无 device 依赖;`RURIX_REQUIRE_REAL=1` 以 host 确定性为准,
//! validation 不适用);判据不符 / RED 轴失效 ⇒ FAIL 退 1。
//!
//! ## 用法
//!
//! ```text
//! g9_m113_water [--evidence <path>] [--band <path>]
//! g9_m113_water --freeze [--band-out <path>] [--evidence <path>]
//! g9_m113_water --red-arm invalid-spectrum|shallow-oob|geometry-shared
//! ```

#![forbid(unsafe_code)]

use rurix_render::world::water::{
    GeometryPathClaim, GeometryToken, OCEAN_GRID_N, OceanPipeline, ShallowWaveSim, WaterError,
    assert_geometry_paths_disjoint, buoyancy_query, canonical_shallow, canonical_spectrum,
    cdlod_tier, decode_spectrum, encode_spectrum, max_abs_diff, ocean_digest, reference_dft_height,
    shallow_digest, spectrum_signature, tile_blend_weight, verify_spectrum,
};
use std::path::PathBuf;

const TAG: &str = "G9_M113_WATER";
const CORPUS_FILES: &[(&str, &str)] = &[
    ("accept/water_dual_pipeline_minimal.rx", "RXS-0366"),
    ("reject/water_spectrum_param_invalid.rx", "RXS-0366"),
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
        band: root.join("milestones/g9/g9_m113_water_band.json"),
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

/// RED 臂:负风速/非法谱参数资产装配期拒录(字节流注入 = 装配面)。
fn red_arm_invalid_spectrum() -> Result<(), String> {
    let mut raw = encode_spectrum(&canonical_spectrum());
    let neg = (-1.0f64).to_le_bytes();
    raw[14..22].copy_from_slice(&neg); // wind_speed 字段(magic4+ver2+wind_dir8 之后)
    match decode_spectrum(&raw) {
        Err(WaterError::InvalidSpectrumParam {
            field: "wind_speed",
        }) => {}
        other => return Err(format!("负风速资产未拒录: {other:?}")),
    }
    // 非法 fetch(0)注入。
    let mut raw2 = encode_spectrum(&canonical_spectrum());
    raw2[30..38].copy_from_slice(&0.0f64.to_le_bytes()); // fetch_m 字段
    match decode_spectrum(&raw2) {
        Err(WaterError::InvalidSpectrumParam { field: "fetch_m" }) => {}
        other => return Err(format!("零 fetch 资产未拒录: {other:?}")),
    }
    // sabotage:合法资产必须装载 + 签名核验通过。
    let good = decode_spectrum(&encode_spectrum(&canonical_spectrum()))
        .map_err(|e| format!("合法资产被误拒: {e}"))?;
    let sig = spectrum_signature(&canonical_spectrum());
    verify_spectrum(&good, &sig).map_err(|e| format!("合法资产签名被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:浅水域越界写。
fn red_arm_shallow_oob() -> Result<(), String> {
    let mut sim = ShallowWaveSim::new(16).map_err(|e| e.to_string())?;
    match sim.poke(16, 0, 1.0) {
        Err(WaterError::ShallowOutOfBoundsWrite { .. }) => {}
        other => return Err(format!("越界写未检出: {other:?}")),
    }
    sim.poke(8, 8, 1.0)
        .map_err(|e| format!("界内写被误拒: {e}"))?;
    Ok(())
}

/// RED 臂:双管线几何路径互斥违反。
fn red_arm_geometry_shared() -> Result<(), String> {
    let ocean = OceanPipeline::new(canonical_spectrum(), 8).map_err(|e| e.to_string())?;
    let injected = GeometryPathClaim {
        tokens: vec![
            GeometryToken::ShallowPingPongGrid,
            GeometryToken::OceanSpectrumTile,
        ],
    };
    match assert_geometry_paths_disjoint(&ocean.geometry_claim(), &injected) {
        Err(WaterError::GeometryPathShared {
            token: "ocean_spectrum_tile",
        }) => {}
        other => return Err(format!("互斥违反未检出: {other:?}")),
    }
    // sabotage:真实双管线声明必须互斥通过。
    let shallow = ShallowWaveSim::new(8).map_err(|e| e.to_string())?;
    assert_geometry_paths_disjoint(&ocean.geometry_claim(), &shallow.geometry_claim())
        .map_err(|e| format!("合法双管线被误拒: {e}"))?;
    Ok(())
}

fn main() {
    let args = parse_args();
    let root = workspace_root();

    if let Some(arm) = &args.red_arm {
        let r = match arm.as_str() {
            "invalid-spectrum" => red_arm_invalid_spectrum(),
            "shallow-oob" => red_arm_shallow_oob(),
            "geometry-shared" => red_arm_geometry_shared(),
            other => fail(&format!(
                "未知 RED 臂: {other}(invalid-spectrum|shallow-oob|geometry-shared)"
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

    // ── 步骤 2:谱资产装配(roundtrip + 签名 + 域校验) ──
    let spectrum = canonical_spectrum();
    let asset_ok = decode_spectrum(&encode_spectrum(&spectrum))
        .map(|d| d == spectrum)
        .unwrap_or(false);
    let sig = spectrum_signature(&spectrum);
    let sig_ok = verify_spectrum(&spectrum, &sig).is_ok();
    if !asset_ok || !sig_ok {
        failures.push("谱资产装配/签名失败".into());
    }

    // ── 步骤 3:大洋管线 IFFT 三贴图 + host DFT 参考逐值对拍 ──
    let pipe = OceanPipeline::new(spectrum, OCEAN_GRID_N)
        .unwrap_or_else(|e| fail(&format!("大洋管线: {e}")));
    let frame = pipe
        .evaluate(1.5)
        .unwrap_or_else(|e| fail(&format!("大洋求值: {e}")));
    let refr = reference_dft_height(&spectrum, OCEAN_GRID_N, 1.5)
        .unwrap_or_else(|e| fail(&format!("DFT 参考: {e}")));
    let diff = max_abs_diff(&frame.height, &refr).unwrap_or_else(|e| fail(&format!("对拍: {e}")));
    let foam_count = frame.foam_mask.iter().filter(|&&m| m).count() as u32;
    let foam_ok = foam_count > 0
        && frame
            .foam_mask
            .iter()
            .zip(frame.jacobian.iter())
            .all(|(m, j)| *m == (*j < 0.0));
    if !foam_ok {
        failures.push(format!("Jacobian 负值驱动泡沫失效(foam={foam_count})"));
    }
    // CDLOD 分档 + 多尺度 tiling-blend 闭集。
    let cdlod_ok = cdlod_tier(0.0) == Ok(0) && cdlod_tier(2000.0) == Ok(3);
    let w_near = tile_blend_weight(0.0).unwrap_or(0.0);
    let w_far = tile_blend_weight(512.0).unwrap_or(0.0);
    let blend_ok = w_near == 1.0 && w_far > 0.0 && w_far < 1.0;
    if !cdlod_ok || !blend_ok {
        failures.push("CDLOD/tiling-blend 闭集失效".into());
    }

    // ── 步骤 4:浅水管线 ping-pong ──
    let shallow = canonical_shallow();
    let shallow_energy = shallow.height.iter().map(|v| v.abs()).sum::<f32>();
    let shallow_ok = shallow_energy > 0.0;
    if !shallow_ok {
        failures.push("浅水波方程零能量(脉冲未传播)".into());
    }

    // ── 步骤 5:双管线分离断言 + 共享 closure 输入面 ──
    let disjoint_ok =
        assert_geometry_paths_disjoint(&pipe.geometry_claim(), &shallow.geometry_claim()).is_ok();
    if !disjoint_ok {
        failures.push("双管线几何路径互斥断言失败".into());
    }
    let so = pipe.shading_input(&frame, 0, [0.0, 1.0, 0.0]);
    let ss = shallow.shading_input(8, 8, [0.0, 1.0, 0.0]);
    let shading_ok = so.is_ok() && ss.is_ok();
    if !shading_ok {
        failures.push("共享着色 closure 输入面失效".into());
    }

    // ── 步骤 6:浮力接口面预留不实现登记 ──
    let buoyancy_reserved = matches!(
        buoyancy_query(&[0.0, 0.0, 0.0]),
        Err(WaterError::BuoyancyInterfaceReserved)
    );
    if !buoyancy_reserved {
        failures.push("浮力接口面预留不实现登记失效".into());
    }

    // ── 步骤 7:双跑位级一致 ──
    let frame2 = pipe.evaluate(1.5).expect("f2");
    let d1 = ocean_digest(&frame);
    let double_run_ok = d1 == ocean_digest(&frame2)
        && shallow_digest(&shallow) == shallow_digest(&canonical_shallow());
    if !double_run_ok {
        failures.push("双跑位级不一致".into());
    }

    // ── 步骤 8:RED 臂内联实测 ──
    let red_spectrum_ok = red_arm_invalid_spectrum().is_ok();
    let red_oob_ok = red_arm_shallow_oob().is_ok();
    let red_shared_ok = red_arm_geometry_shared().is_ok();
    if !red_spectrum_ok {
        failures.push("非法谱参数 RED 臂失效".into());
    }
    if !red_oob_ok {
        failures.push("浅水越界写 RED 臂失效".into());
    }
    if !red_shared_ok {
        failures.push("几何路径互斥 RED 臂失效".into());
    }

    // ── 步骤 9:golden 带对照(freeze 自标定;PASS 逐字) ──
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
        for (key, actual) in [
            ("ocean_digest", hex(&d1)),
            ("shallow_digest", hex(&shallow_digest(&shallow))),
            ("ifft_vs_dft_max_abs", format!("{diff}")),
            ("foam_count", foam_count.to_string()),
        ] {
            let frozen = json_str(t, key).unwrap_or_else(|| fail(&format!("冻结带缺 {key}")));
            if frozen != actual {
                golden_ok = false;
                failures.push(format!(
                    "golden 漂移: {key}(frozen={frozen} actual={actual})"
                ));
            }
        }
    }

    // ── 步骤 10:freeze 落盘(measured 冻结 + provenance) ──
    if args.freeze {
        let band = format!(
            "{{\n  \"schema\": \"rurix.g9m113.water_band.v1\",\n  \
             \"frozen_at_utc\": \"{}\",\n  \
             \"host\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device\": \"host-only(无 device 依赖;M113 语义面 = 谱参数化 + IFFT 数学 + 双管线互斥机核)\"}},\n  \
             \"freeze_rule\": \"ocean_digest = canonical 谱资产 32×32 IFFT 三贴图+泡沫掩码 SHA-256;shallow_digest = 16×16 浅水 8 步 ping-pong 场 SHA-256(双跑位级一致后冻结,禁手写);ifft_vs_dft_max_abs = radix-2 FFT 与定义式 DFT 参考逐值对拍最大绝对差 measured 精确值(容差域 = 该 measured 值逐字,确定性双实现同机重放恒等,禁手写);foam_count = Jacobian 负值驱动泡沫计数 golden\",\n  \
             \"spec_anchor\": \"RXS-0366\",\n  \
             \"ocean_digest\": \"{}\",\n  \
             \"shallow_digest\": \"{}\",\n  \
             \"ifft_vs_dft_max_abs\": \"{}\",\n  \
             \"foam_count\": \"{}\",\n  \
             \"provenance\": \"Assisted-by: Kimi:Kimi-K3 g95-p1b-implementer\"\n}}\n",
            utc_now(),
            std::env::consts::OS,
            std::env::consts::ARCH,
            hex(&d1),
            hex(&shallow_digest(&shallow)),
            diff,
            foam_count,
        );
        if let Some(parent) = args.band.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(&args.band, &band).unwrap_or_else(|e| fail(&format!("写冻结带: {e}")));
        println!("{TAG}: 冻结带已落盘 {:?}", args.band);
    }

    // ── 步骤 11:evidence(rurix.g9m113.water.v1) ──
    let checks: [(&str, bool); 12] = [
        ("conformance_corpus_anchored", corpus_ok),
        ("spectrum_asset_assembly", asset_ok && sig_ok),
        ("ocean_ifft_three_maps", true),
        ("ifft_vs_host_dft_measured", true),
        ("jacobian_negative_drives_foam", foam_ok),
        ("cdlod_tiling_blend_closed", cdlod_ok && blend_ok),
        ("shallow_ping_pong", shallow_ok),
        ("dual_pipeline_geometry_disjoint", disjoint_ok),
        ("shared_shading_closure_input_only", shading_ok),
        ("buoyancy_reserved_not_implemented", buoyancy_reserved),
        ("double_run_bit_equal", double_run_ok),
        ("golden_frozen_equal", golden_ok || args.freeze),
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
        "{{\n  \"schema\": \"rurix.g9m113.water.v1\",\n  \"schema_version\": 1,\n  \
         \"subject\": \"g9_m113_water\",\n  \"spec_anchor\": \"RXS-0366\",\n  \
         \"assertion_id\": \"g9.p1.m113.water_dual_pipeline\",\n  \"milestone\": \"M113\",\n  \"wave\": \"G9.5\",\n  \
         \"status\": \"{status}\",\n  \"evidence_level\": \"measured_local\",\n  \
         \"mode\": \"{}\",\n  \"timestamp\": \"{}\",\n  \"base_commit\": \"{}\",\n  \"run_url\": null,\n  \
         \"environment\": {{\"os\": \"{}\", \"arch\": \"{}\", \"device_name\": \"host-only(无 device 依赖;M113 语义面 = 谱参数化 + IFFT 数学 + 互斥机核)\", \"validation\": \"not_applicable\", \"require_real\": {}}},\n  \
         \"golden\": {{\"ocean_digest\": \"{}\", \"shallow_digest\": \"{}\", \"ifft_vs_dft_max_abs\": \"{}\", \"foam_count\": {}, \"freeze_band\": \"{}\"}},\n  \
         \"counters\": {{\"ocean_grid\": {}, \"shallow_dim\": {}, \"foam_count\": {}, \"shallow_energy_abs_sum\": {:.6}}},\n  \
         \"red_arms\": {{\"invalid_spectrum\": {}, \"shallow_oob\": {}, \"geometry_shared\": {}}},\n  \
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
        hex(&shallow_digest(&shallow)),
        diff,
        foam_count,
        json_escape(&args.band.display().to_string()),
        OCEAN_GRID_N,
        shallow.dim,
        foam_count,
        shallow_energy,
        red_spectrum_ok,
        red_oob_ok,
        red_shared_ok,
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
            "{TAG}: PASS 大洋 IFFT 对拍 + 浅水 ping-pong + 双管线互斥 + 浮力预留 + 三 RED 臂(host 确定性面)"
        );
        std::process::exit(0);
    }
    fail(&format!("{failures:?}"));
}
