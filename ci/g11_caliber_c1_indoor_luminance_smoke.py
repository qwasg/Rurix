#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.2 波）
"""G11.2 M144 C1 室内亮度口径对齐闭环门（P0，步骤 196；
g11.p0.m144.caliber_c1_indoor_luminance；G11_CONTRACT §4.2 M144 行判据逐字 /
G-G11-4；G11_ACCEPTANCE_MAP §1 M144 行；CI_GATES §4；RFC-0028 §4.5；
spec/visual_comparison.md RXS-0392/RXS-0393）。

host 纯 host 门（device_section_state=not_applicable——双端复跑出帧由
milestones/g11/harness/g11_2_ab_rerun.py 先行真跑，本门为机核面不重跑 UE/渲染）。
判据（契约 §4.2 M144 行字面）：

1. **GI/天光遮蔽口径差 + 太阳 lux→辐射度链差逐行对齐**：
   - 太阳链（RXS-0392 L3）：UE 臂光色**线性直给**（harness g10_5_build_scenes.py
     `set_light_color(..., False)` 修复面在树——G10.5a b_srgb=True sRGB 二次转换
     口径偏差 G−2.5%/B−6.3% 修复）+ lux→辐射度双端同构式 provenance（UE
     DirectionalLight lux → L=ρ·E·(n·l)/π；Rurix sun_color=rgb·lux →
     direct=·ndl·albedo/π）；
   - 天光链（RXS-0392 L2）：UE SkyLight 指定 cubemap（白色常量资产逐像素值
     =1.0 uniform 实测 + sha256 digest 登记）× intensity vs Rurix 常量天光
     辐射度同单位链；采样档位登记；
   - 曝光链（C2 对齐面消费）：Rurix 臂曝光尺度管线内烘焙 2^(−EV100) 与 UE 臂
     pipe 内 FixedExposure 同域；LDR 派生尺度双端统一 ×1.0。
2. **对齐后残余口径差显式登记**（RXS-0392 L4）：
   milestones/g11/g11_2_residual_caliber_registry.json 逐环节非空行（灯种子集
   结构差→R3 承接锚 / GI 结构差→R4 承接锚 / 镜面 IBL 结构差 / 源位深量化差
   →C3 承接面），每行处置锚非空。
3. **对齐前后口径参数 provenance 齐备**（caliber_chain 闭集块逐环节
   contract_value/ue_applied/rurix_applied + before/after 字面）。
4. **修复前后度量 delta 对拍**（measured）：复测 delta 自 G11.2 帧区独立重算
   （bistro HDR 亮度中位 / cornell HDR p90），与 G10.8b 锁定基线 delta 对拍
   登记（原域 + 域统一换算双面）；复测 delta 与登记残余一致（残余登记
   measured_impact 与门内独立重算逐位一致）。
5. **契约 digest 三面绑定 0-byte**（RXS-0393 L4）：双场景当次重算 ==
   G10.5 锁定值；Rurix 侧全部帧 capture_params_digest == 锁定值互证。

RED 臂（契约判据字面）：未对齐口径消费复测 delta 即 RED
（red_unaligned_consumption）；拟合冒充对齐即 RED（red_fitting_masquerade——
篡改契约参数后 digest ≠ 锁定值必检出）；残余口径差未登记即 RED
（red_residual_unregistered——缺 R3/R4 承接锚的伪造登记必检出）。

用法：
  py -3 ci/g11_caliber_c1_indoor_luminance_smoke.py --gate g11.p0.m144.caliber_c1_indoor_luminance
  py -3 ci/g11_caliber_c1_indoor_luminance_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
SCHEMA_PATH = ROOT / "milestones" / "g11" / "g11_m144_caliber_c1_indoor_luminance_evidence_schema.json"

sys.path.insert(0, str(ROOT / "ci"))

import g11_2_caliber_lib as cl  # noqa: E402
import g11_wave_exit_lib as wel  # noqa: E402

GATE_KEY = "g11.p0.m144.caliber_c1_indoor_luminance"
NUMERIC_STEP = 196
SOURCE_REF = (
    "G11_CONTRACT §4.2 M144 + G-G11-4;G11_ACCEPTANCE_MAP §1 M144;CI_GATES §4;"
    "RFC-0028 §4.5;spec/visual_comparison.md RXS-0392/RXS-0393"
)
TAG = "g11_m144"
SUBJECT = "g11_m144_caliber_c1_indoor_luminance"
MATRIX_ROW = "M144"

CHECK_KEYS = [
    "contract_digest_locked_unchanged",
    "sun_chain_aligned_provenance",
    "sky_chain_aligned_provenance",
    "exposure_chain_unified_provenance",
    "residual_registry_complete",
    "retest_delta_measured_consistent",
    "aligned_chains_provenance_before_after",
    "red_fitting_masquerade_detected",
    "red_unaligned_consumption_detected",
    "red_residual_unregistered_detected",
]

FAILURES: list[str] = []
NOTES: list[str] = []
COMMANDS: list[dict] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def run_selftest() -> int:
    check(False, "selftest 合成失败（证明 check() 能红）")
    if len(FAILURES) != 1:
        print(f"[{TAG}] selftest FAIL: check() 未记录合成失败", file=sys.stderr)
        return 1
    FAILURES.clear()
    # 绿臂①：真树残余登记校验零 problems。
    reg = cl.load_residual_registry()
    if validate_residual_arm(reg):
        print(f"[{TAG}] selftest FAIL: 真树登记误判", file=sys.stderr)
        return 1
    # 绿臂②：schema checks.required 与 CHECK_KEYS 闭集精确互核。
    schema = cl.load_json(SCHEMA_PATH) if SCHEMA_PATH.is_file() else {}
    req = set(schema.get("properties", {}).get("checks", {}).get("required", []))
    if req != set(CHECK_KEYS):
        print(f"[{TAG}] selftest FAIL: schema required 与 CHECK_KEYS 闭集不等 {req ^ set(CHECK_KEYS)}", file=sys.stderr)
        return 1
    # 红臂①：伪造登记缺 exposure_scale 对齐环节（未对齐口径消费）必检出。
    import copy

    forged = copy.deepcopy(reg)
    forged["aligned_chains"] = [c for c in forged["aligned_chains"] if c.get("chain") != "exposure_scale"]
    if not validate_residual_arm(forged):
        print(f"[{TAG}] selftest FAIL: 未对齐口径消费未检出", file=sys.stderr)
        return 1
    # 红臂②：伪造登记缺 R3/R4 承接锚（残余未登记）必检出。
    forged2 = copy.deepcopy(reg)
    forged2["items"] = [i for i in forged2["items"] if "m153" in str(i.get("disposition_anchor", ""))]
    forged2["items"] = [dict(i, disposition_anchor="显式留档") for i in forged2["items"]]
    if not validate_residual_arm(forged2):
        print(f"[{TAG}] selftest FAIL: 残余未登记未检出", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} (2 RED + 2 GREEN)")
    return 0


def validate_residual_arm(doc: dict) -> list[str]:
    return cl.validate_residual_registry(doc)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    checks: dict[str, bool] = {k: False for k in CHECK_KEYS}
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")

    # ① 契约 digest 三面绑定 0-byte（当次重算 == 锁定值；Rurix 侧帧 metadata 互证）。
    digest_rows: dict[str, str] = {}
    digest_drift: list[str] = []
    for scene_id in cl.SCENES:
        got = cl.contract_digest_rust(scene_id)
        digest_rows[scene_id] = got
        COMMANDS.append({
            "seq": len(COMMANDS) + 1,
            "command": f"{cl.RUST_RELEASE_BIN} --contract-digest milestones/g10/corpus/contract_params_{scene_id.replace('-', '_')}.json",
            "exit_code": 0,
        })
        if got != cl.LOCKED_DIGEST[scene_id]:
            digest_drift.append(f"{scene_id}: {got} ≠ 锁定值 {cl.LOCKED_DIGEST[scene_id]}")
    frame_digest_bad: list[str] = []
    for scene_id in cl.SCENES:
        d = cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix")
        if d["metadata"].get("rurix:capture_params_digest") != cl.LOCKED_DIGEST[scene_id]:
            frame_digest_bad.append(f"{scene_id} Rurix HDR capture_params_digest 漂移")
        for end in ("rurix", "ue5"):
            ld = cl.decode(cl.ldr_frame(scene_id, end), "rurix")
            if ld["metadata"].get("rurix:capture_params_digest") != cl.LOCKED_DIGEST[scene_id]:
                frame_digest_bad.append(f"{scene_id}/{end} LDR capture_params_digest 漂移")
    checks["contract_digest_locked_unchanged"] = not digest_drift and not frame_digest_bad
    check(not digest_drift, f"契约 digest 漂移（修复动契约参数即 RED）: {digest_drift}")
    check(not frame_digest_bad, f"帧 metadata digest 漂移: {frame_digest_bad}")
    note(f"契约 digest 三面绑定复核: {digest_rows}（== G10.5 锁定值）")

    # ② 太阳链对齐 provenance（b_srgb=False 修复面 + 双端同构式 + 契约值回显）。
    bs_text = cl.BUILD_SCENES_PY.read_text(encoding="utf-8")
    rs_text = cl.SCENE_RENDER_RS.read_text(encoding="utf-8")
    sun_fix_in_tree = "set_light_color(unreal.LinearColor(rgb[0], rgb[1], rgb[2], 1.0), False)" in bs_text
    rurix_sun_form = ("c.sun_color[0] * c.sun_intensity_lux" in rs_text
                      and "albedo[ch] * inv_pi" in rs_text)
    contracts = {
        s: cl.load_json(cl.CORPUS / f"contract_params_{s.replace('-', '_')}.json") for s in cl.SCENES
    }
    sun_provenance = {
        s: {
            "contract_intensity_lux": contracts[s]["lighting"]["sun"]["intensity_lux"],
            "contract_color_linear_rgb": contracts[s]["lighting"]["sun"]["color_linear_rgb"],
            "ue_applied": "DirectionalLight.intensity=契约 lux；set_light_color(b_srgb=False) 线性直给（修复面在树）",
            "rurix_applied": "sun_color=color_linear_rgb×intensity_lux；direct=sun_color·ndl·albedo/π（g10_5_scene_render.rs 同构式）",
        }
        for s in cl.SCENES
    }
    checks["sun_chain_aligned_provenance"] = sun_fix_in_tree and rurix_sun_form
    check(sun_fix_in_tree, "UE 臂光色线性直给修复面（b_srgb=False）不在树——未对齐口径")
    check(rurix_sun_form, "Rurix 侧太阳链同构式源码面缺失")

    # ③ 天光链对齐 provenance（白色 cubemap 逐像素 =1.0 uniform + digest 登记）。
    sky_ok = False
    white_digest = ""
    white_note = ""
    try:
        px, white_digest = cl.parse_white_hdr(cl.WHITE_HDR)
        uniform = all(abs(v - 1.0) == 0.0 for v in px)
        sky_ok = uniform and len(px) == 6
        white_note = f"white_2x1.hdr 2×1 逐像素 = 1.0 uniform（{len(px)//3} px）digest={white_digest[:24]}…"
    except (OSError, ValueError) as e:
        white_note = f"白色 cubemap 解析失败: {e}"
    checks["sky_chain_aligned_provenance"] = sky_ok
    check(sky_ok, f"天光链 cubemap 核验失败: {white_note}")
    note(white_note)

    # ④ 曝光链统一 provenance（C2 对齐面：Rurix 管线内 2^(−EV100) + 派生 ×1.0 双端）。
    report = cl.load_report()
    results = report.get("results", {})
    exposure_bad: list[str] = []
    for scene_id, s in cl.SCENES.items():
        want = 2.0 ** (-s["ev100"])
        got = (results.get("rurix", {}).get(scene_id) or {}).get("exposure_scale_in_pipe")
        if got != want:
            exposure_bad.append(f"{scene_id} Rurix 管线内曝光尺度 {got!r} ≠ 2^(−EV100)={want!r}")
        for end in ("rurix", "ue5"):
            hs = (results.get("derive", {}).get(f"{scene_id}:{end}") or {}).get("exposure_scale_host")
            if hs != 1.0:
                exposure_bad.append(f"{scene_id}/{end} LDR 派生尺度 {hs!r} ≠ 1.0")
        # 派生链元数据互证回归：LDR source_frame_digest == HDR 内容 digest（独立重算）。
        hd = cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix")
        hd_digest = cl.exr.frame_content_digest(hd["width"], hd["height"], 3, hd["pixels"])
        ld = cl.decode(cl.ldr_frame(scene_id, "rurix"), "rurix")
        if ld["metadata"].get("rurix:source_frame_digest") != hd_digest:
            exposure_bad.append(f"{scene_id}/rurix LDR 派生链互证断裂")
        hu = cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5")
        hu_digest = cl.exr.frame_content_digest(hu["width"], hu["height"], 3, hu["pixels"])
        lu = cl.decode(cl.ldr_frame(scene_id, "ue5"), "rurix")
        if lu["metadata"].get("rurix:source_frame_digest") != hu_digest:
            exposure_bad.append(f"{scene_id}/ue5 LDR 派生链互证断裂")
    checks["exposure_chain_unified_provenance"] = not exposure_bad
    check(not exposure_bad, f"曝光链未统一（C2 未对齐即消费）: {exposure_bad[:3]}")

    # ⑤ 残余口径差显式登记（RXS-0392 L4 机核面）。
    registry = cl.load_residual_registry()
    reg_problems = validate_residual_arm(registry)
    checks["residual_registry_complete"] = not reg_problems
    check(not reg_problems, f"残余口径差登记异常（残余未登记即 RED）: {reg_problems[:3]}")

    # ⑥ 修复前后度量 delta 对拍（独立重算；与登记残余一致机核）。
    c1_row = cl.gap_row("C1")
    baseline = {m["metric"]: m for m in c1_row["measured_delta"]}
    retest: dict = {}
    consistency_bad: list[str] = []
    reg_impacts: dict[str, float] = {}
    for it in registry.get("items", []):
        mi = it.get("measured_impact") or {}
        for k, v in mi.items():
            reg_impacts[f"{it['residual_id']}:{k}"] = v
    for scene_id in cl.SCENES:
        arr_r = cl.pixels_of(cl.decode(cl.hdr_frame(scene_id, "rurix"), "rurix"))
        arr_u = cl.pixels_of(cl.decode(cl.hdr_frame(scene_id, "ue5"), "ue5"))
        sr, su = cl.lum_stats(arr_r), cl.lum_stats(arr_u)
        retest[scene_id] = {
            "rurix": sr, "ue5": su,
            "median_delta": su["median"] - sr["median"],
            "p90_delta": su["p90"] - sr["p90"],
        }
    # 与登记 measured_impact 逐位一致（残余 delta 全额归属登记项）。
    impact_map = {
        "c1_light_seed_subset_r3:hdr_luminance_median_delta_post_alignment": retest["bistro-interior"]["median_delta"],
        "c1_light_seed_subset_r3:hdr_luminance_p90_delta_post_alignment": retest["bistro-interior"]["p90_delta"],
        "c1_gi_structure_multibounce_r4:hdr_luminance_p90_delta_post_alignment": retest["bistro-interior"]["p90_delta"],
        "c1_cornell_gi_structure_r4:hdr_luminance_p90_delta_post_alignment": retest["cornell-box"]["p90_delta"],
    }
    for k, v in impact_map.items():
        rv = reg_impacts.get(k)
        if rv is None or rv != v:
            consistency_bad.append(f"{k}: 登记 {rv!r} ≠ 门内重算 {v!r}")
    # 域统一换算对拍（原域基线 → 双端同域基线 → 复测同域实测）。
    bistro_base = baseline["hdr_luminance_median@bistro-interior"]
    cornell_base = baseline["hdr_luminance_p90@cornell-box(rurix×2^-EV100)"]
    domain_unified_bistro_baseline = bistro_base["b_value"] - bistro_base["a_value"] * (2.0 ** (-1.0))
    delta_decomposition = {
        "bistro_median": {
            "baseline_delta_original_domain": bistro_base["delta"],
            "baseline_a_rurix_unexposed": bistro_base["a_value"],
            "baseline_b_ue5_exposed": bistro_base["b_value"],
            "baseline_delta_domain_unified": domain_unified_bistro_baseline,
            "retest_delta_aligned_domain": retest["bistro-interior"]["median_delta"],
            "note": "原域基线 = Rurix 未施曝光 vs UE 已施曝光域混测；C2 对齐后双端同域（曝光已施 scene-linear）——域统一换算基线 = b − a×2^(−EV100)；复测 delta 在同域实测，与登记残余一致",
        },
        "cornell_p90": {
            "baseline_delta_domain_unified": cornell_base["delta"],
            "baseline_a_rurix_scaled": cornell_base["a_value"],
            "baseline_b_ue5_exposed": cornell_base["b_value"],
            "retest_delta_aligned_domain": retest["cornell-box"]["p90_delta"],
            "note": "cornell 行基线本为域统一口径（a 含 ×2^(−EV100) 派生尺度）；复测同域实测",
        },
    }
    checks["retest_delta_measured_consistent"] = not consistency_bad
    check(not consistency_bad, f"复测 delta 与登记残余不一致: {consistency_bad[:3]}")
    note(
        f"修复前后 delta 对拍（bistro 中位）: 基线原域 {bistro_base['delta']:.6f} → 域统一 {domain_unified_bistro_baseline:.6f} → 复测 {retest['bistro-interior']['median_delta']:.6f}；"
        f"cornell p90: 基线 {cornell_base['delta']:.6f} → 复测 {retest['cornell-box']['p90_delta']:.6f}"
    )

    # ⑦ 对齐前后口径参数 provenance（caliber_chain 闭集块）。
    caliber_chain = []
    for c in registry.get("aligned_chains", []):
        caliber_chain.append({
            "chain": c.get("chain"),
            "scene_id": c.get("scene_id"),
            "contract_value": "见 contracts 回显（sun/sky/exposure 节）",
            "ue_applied": c.get("after", ""),
            "rurix_applied": c.get("after", ""),
            "before": c.get("before", ""),
            "aligned": c.get("status") in ("aligned_fixed", "aligned_verified"),
            "residual_note": "",
        })
    checks["aligned_chains_provenance_before_after"] = (
        len(caliber_chain) >= 4 and all(r["aligned"] for r in caliber_chain)
    )
    check(checks["aligned_chains_provenance_before_after"], "对齐前后口径参数 provenance 不齐备")

    # ⑧ RED 臂①：拟合冒充对齐必检出（篡改契约参数 → digest ≠ 锁定值）。
    red_fitting = False
    with tempfile.TemporaryDirectory(prefix="g11_m144_red_") as td:
        tampered = dict(contracts["cornell-box"])
        tampered["lighting"] = dict(tampered["lighting"])
        tampered["lighting"]["sun"] = dict(tampered["lighting"]["sun"])
        tampered["lighting"]["sun"]["intensity_lux"] = tampered["lighting"]["sun"]["intensity_lux"] * 0.5
        tp = Path(td) / "tampered.json"
        tp.write_text(json.dumps(tampered), encoding="utf-8")
        r = subprocess.run([str(cl.RUST_RELEASE_BIN), "--contract-digest", str(tp)],
                           cwd=ROOT, capture_output=True, text=True)
        line = [l for l in r.stdout.splitlines() if "param_digest_rust" in l]
        if line:
            got = "sha256:" + line[-1].split("=")[-1].strip()
            red_fitting = got != cl.LOCKED_DIGEST["cornell-box"]
    checks["red_fitting_masquerade_detected"] = red_fitting
    check(red_fitting, "拟合冒充（契约参数篡改）未检出——digest 机核失效")

    # ⑨ RED 臂②：未对齐口径消费复测 delta 必检出（缺 exposure_scale 对齐环节）。
    import copy

    forged = copy.deepcopy(registry)
    forged["aligned_chains"] = [c for c in forged["aligned_chains"] if c.get("chain") != "exposure_scale"]
    checks["red_unaligned_consumption_detected"] = bool(validate_residual_arm(forged))
    check(checks["red_unaligned_consumption_detected"], "未对齐口径消费伪造登记未检出")

    # ⑩ RED 臂③：残余口径差未登记必检出（缺 R3/R4 承接锚）。
    forged2 = copy.deepcopy(registry)
    forged2["items"] = [dict(i, disposition_anchor="显式留档") for i in forged2["items"]]
    checks["red_residual_unregistered_detected"] = bool(validate_residual_arm(forged2))
    check(checks["red_residual_unregistered_detected"], "残余未登记伪造登记未检出")

    host_pass = all(checks.values())
    all_pass = host_pass and not FAILURES

    evidence = {
        "schema_version": 1,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": MATRIX_ROW,
        "milestone": MATRIX_ROW,
        "assertion_id": GATE_KEY,
        "status": "pass" if all_pass else "fail",
        "wave": "G11.2",
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
        "host_section_pass": host_pass,
        "device_section_state": "not_applicable",
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "commands": COMMANDS,
        "caliber_chain": caliber_chain,
        "sun_provenance": sun_provenance,
        "sky_provenance": {
            "white_cubemap_digest": white_digest,
            "white_cubemap_uniform": white_note,
            "contract_sky_intensity": {s: contracts[s]["lighting"]["sky"]["intensity"] for s in cl.SCENES},
            "sampling_tier": "Rurix 屏幕探针单反弹（probe cell/rays 档位沿 G9 M99 host 参考管线默认；GiParams seed=契约 random_seed）",
        },
        "closure": {
            "gap_row_id": c1_row["gap_id"],
            "baseline_delta": bistro_base["delta"],
            "retest_delta": retest["bistro-interior"]["median_delta"],
            "converged": bool(all_pass and not consistency_bad),
            "threshold_provenance": (
                "caliber_diff 行闭环语义（RXS-0393 L2 口径款）：口径对齐完成 + 残余显式登记 + 复测 delta 与登记残余一致"
                "——收敛判定不经统计阈值；对齐链 provenance 见 caliber_chain/aligned_chains；"
                "残余登记 = milestones/g11/g11_2_residual_caliber_registry.json"
            ),
            "contract_digest_unchanged": bool(checks["contract_digest_locked_unchanged"]),
        },
        "delta_decomposition": delta_decomposition,
        "retest_measured": retest,
        "residual_registry_path": "milestones/g11/g11_2_residual_caliber_registry.json",
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": wel.collect_environment(),
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"{SUBJECT}_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device=not_applicable")
    if all_pass and not FAILURES:
        print(
            f"[{TAG}] PASS（C1 口径差逐行对齐闭环：太阳/天光/曝光链 provenance 齐备 + "
            f"残余口径差逐环节显式登记 + 复测 delta 与登记残余一致 + 契约 digest 0-byte + RED 三臂全检出）"
        )
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
