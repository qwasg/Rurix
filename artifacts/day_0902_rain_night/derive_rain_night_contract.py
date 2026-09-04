#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""day_0902_rain_night：由 exterior_scene_facts.json 派生「BistroExterior 雨夜街景」借壳展示契约。

借壳口径（共享体 parse_contract 闭集：scenes 恰二行且 scene_id ∈ {bistro-interior, cornell-box}）：
  - 克隆冻结契约 milestones/g13/g13_ue_upscale_parity_contract.json，根级字段与 cornell-box 行一字不改；
  - 只替换 bistro-interior 行的 camera / exposure / lighting / material_policy / gltf_product_digest，
    scene_id 字面保留 "bistro-interior"（借壳），provenance（不入 digest）如实登记；
  - 灯面：point_lights = 机位视锥内可见路灯（按距离 top-K）+ 可见吊灯笼（g35 车道 emissive 只可见不投光，
    照明全靠点光；点光位取 facts 的逃逸测试最优位，位于闭合灯罩外侧）；emissive_materials = 10 条，
    Le = 目标 display 线性亮度 × 2^ev100（随 ev100 自适应，色向取 emissive DDS 线性均值方向）；
  - digest 由 milestones/g13/harness/ue_python/g13_parity_contract.py 离线算出（= 渲染 bin 的 --expect-digest）。

用法：
  py -3 -B derive_rain_night_contract.py --camera C1 [--ev100 -7] [--lamp-cd 0.01] [--lamp-k 14]
        [--lantern-cd-ratio 0.35] [--lamp-color 1.0,0.72,0.42] [--le-scale 1.0] [--fov-y-deg 52]
        [--gltf <path>] [--gltf-sha256 sha256:<hex>] [--out <json>] [--tag <name>]
  py -3 -B derive_rain_night_contract.py --selftest   # look-at→quat 手性自检（室内 corpus ↔ 冻结契约）
  py -3 -B derive_rain_night_contract.py --camera C1 --write-corpus   # 另写 g10_corpus/ 三件（借壳登记）

fail-closed：facts / 冻结契约 / harness 缺失、字段缺失、digest 计算失败 ⇒ 中文原因 + 非 0 退出。
"""
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import subprocess
import sys
import time
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent.parent
sys.path.insert(0, str(HERE))
from analyze_exterior_scene import camera_basis, lookat_quat_wxyz  # noqa: E402  同目录、同一套相机数学

FROZEN_CONTRACT = ROOT / "milestones" / "g13" / "g13_ue_upscale_parity_contract.json"
HARNESS = ROOT / "milestones" / "g13" / "harness" / "ue_python" / "g13_parity_contract.py"
CORPUS_DIR = ROOT / "milestones" / "g10" / "corpus"
DEFAULT_FACTS = HERE / "exterior_scene_facts.json"
DEFAULT_GLTF = Path(r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroExterior\BistroExterior.gltf")
INDEX_PATH = HERE / "contracts_index.json"
CORPUS_OUT = HERE / "g10_corpus"

# 室内冻结行的标定参照（等地面照度换算起点）：4 点光 0.0182 cd @ ev100 −4，灯离地约 3.0 m
INT_LAMP_CD = 0.0182327231840096
INT_LAMP_HEIGHT_M = 3.0
INT_EV100 = -4.0

# emissive 目标 display 线性亮度（ACES 前，1.0 ≈ 显示白；Le = 该值 × 2^ev100 × le_scale）
EMISSIVE_DISPLAY_TARGET = {
    12: 4.0,   # Emissive_StreetLight（路灯玻璃 / 吊灯笼发光体）：过曝成白 = 光源观感
    13: 2.0,   # Spotlight_Glass_Emissive
    1: 2.5, 2: 2.5, 3: 2.5, 4: 2.5, 5: 2.5, 6: 2.5,   # 彩灯串六色：保留色相不全白
    38: 0.5, 39: 0.5,   # 店招（首探 2.0 过曝成白盘、1.0 仍平白，降至 0.5 保留色相；无贴图采样只能是常量色盘）
}
# DDS 均值不可得时的兜底色向（线性 RGB 方向）
FALLBACK_COLOR = {
    12: (1.0, 0.72, 0.42), 13: (1.0, 1.0, 1.0),
    1: (1.0, 0.45, 0.05), 2: (0.1, 0.3, 1.0), 3: (1.0, 1.0, 1.0), 4: (1.0, 0.2, 0.6), 5: (1.0, 0.05, 0.02), 6: (0.1, 1.0, 0.1),
    38: (0.3, 1.0, 0.3), 39: (1.0, 0.5, 0.5),
}


def fail(msg: str, code: int = 2) -> None:
    print(f"FAIL: {msg}", file=sys.stderr)
    sys.exit(code)


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    with open(p, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 22), b""):
            h.update(chunk)
    return "sha256:" + h.hexdigest()


def parse_vec3(s: str, name: str) -> list[float]:
    parts = s.split(",")
    if len(parts) != 3:
        fail(f"{name} 须为 r,g,b 三元组：{s}")
    try:
        v = [float(x) for x in parts]
    except ValueError:
        fail(f"{name} 非数值：{s}")
    if any(x < 0 for x in v):
        fail(f"{name} 分量须 ≥ 0：{s}")
    return v


def harness_digest(contract_path: Path) -> str:
    """调用 g13_parity_contract.py 取 canonical digest（与 Rust prelude 同表）。"""
    if not HARNESS.is_file():
        fail(f"digest harness 缺失：{HARNESS}")
    r = subprocess.run([sys.executable, "-B", str(HARNESS), str(contract_path)], cwd=ROOT,
                       capture_output=True, text=True, encoding="utf-8", errors="replace")
    if r.returncode != 0:
        fail(f"g13_parity_contract.py 拒绝契约（rc={r.returncode}）：\n{(r.stdout + r.stderr)[-1200:]}")
    lines = [ln.strip() for ln in r.stdout.splitlines() if ln.strip()]
    dig = next((ln for ln in reversed(lines) if ln.startswith("sha256:") and len(ln) == 71), None)
    if dig is None:
        fail(f"harness 输出无 digest 行：{r.stdout[-400:]}")
    return dig


def selftest() -> int:
    """室内 corpus 的 eye/target/up → quat 应复现冻结契约室内行 orientation_quat（q 与 −q 等价，容差 1e-6）。"""
    cam = json.loads((CORPUS_DIR / "camera_bistro_interior.json").read_text(encoding="utf-8"))
    frozen = json.loads(FROZEN_CONTRACT.read_text(encoding="utf-8"))
    row = next(s for s in frozen["scenes"] if s["scene_id"] == "bistro-interior")
    q_ref = np.asarray(row["camera"]["orientation_quat"], dtype=np.float64)
    q = lookat_quat_wxyz(cam["eye"], cam["target"], cam["up"])
    if np.dot(q, q_ref) < 0:
        q = -q
    err = float(np.max(np.abs(q - q_ref)))
    r, u, f = camera_basis(q_ref)
    f_ref = np.asarray(cam["target"]) - np.asarray(cam["eye"])
    f_ref /= np.linalg.norm(f_ref)
    ferr = float(np.max(np.abs(f - f_ref)))
    ok = err < 1e-6 and ferr < 1e-6
    print(f"[selftest] lookat→quat 与冻结契约室内行 max|Δq|={err:.2e}，forward 复现 max|Δf|={ferr:.2e} ⇒ {'PASS' if ok else 'FAIL'}")
    print(f"[selftest] 约定：orientation_quat 序 w,x,y,z；forward = q·(0,0,−1)；up = q·(0,1,0)；右手系（det=+1）")
    return 0 if ok else 1


def emissive_rows(facts: dict, ev100: float, le_scale: float) -> list[dict]:
    rows = []
    exposure_inv = 2.0 ** ev100  # display = L × 2^(−ev100) ⇒ L = display × 2^ev100
    for em in facts["emissive_materials"]:
        mi = int(em["material_index"])
        mean = (em.get("emissive_dds") or {}).get("mean_linear_rgb")
        if mean and max(mean) > 1e-6:
            col = np.asarray(mean, dtype=np.float64)
            src = "emissive DDS 线性均值方向"
        else:
            col = np.asarray(FALLBACK_COLOR.get(mi, (1.0, 1.0, 1.0)), dtype=np.float64)
            src = "兜底色表（DDS 均值不可得）"
        col = col / col.max()
        # 店招 DDS 底色偏暗（发光图案占比小），色向做轻微去饱和以免纯色块
        if mi in (38, 39):
            col = 0.7 * col + 0.3
            col = col / col.max()
        target = EMISSIVE_DISPLAY_TARGET.get(mi, 2.0)
        le = (col * target * exposure_inv * le_scale).tolist()
        area = float(em["area_m2"])
        if not area > 0:
            fail(f"材质 {mi} area_m2 非正：{area}")
        rows.append({
            "material_name": em["name"],
            "material_index": mi,
            "le_linear_rgb": [float(v) for v in le],
            "area_m2": area,
            "_note": f"{src}；目标 display 线性 {target} × 2^({ev100:g}) × le_scale {le_scale:g}",
        })
    return rows


def pick_lights(facts: dict, cand: dict, lamp_k: int, lamp_cd: float, lantern_ratio: float, lamp_color: list[float]) -> tuple[list[dict], list[dict]]:
    lamps = {r["id"]: r for r in facts["streetlights"]["lamps"]}
    lanterns = {r["id"]: r for r in facts["lanterns"]["items"]}
    chosen, notes = [], []
    n_sl = 0
    for v in cand["lamps_visible_sorted"]:
        if v["kind"] == "streetlight":
            if n_sl >= lamp_k:
                continue
            r = lamps.get(v["id"])
            if r is None:
                fail(f"机位可见路灯 {v['id']} 不在路灯表")
            chosen.append({
                "id": v["id"],
                "position": [float(x) for x in r["point_light_pos"]],
                "color_linear_rgb": [float(c) for c in lamp_color],
                "intensity_cd": float(lamp_cd),
            })
            notes.append({"id": v["id"], "kind": "streetlight", "distance_m": v["distance_m"], "rule": r.get("point_light_rule"),
                          "escape": r.get("point_light_escape"), "glass_centroid": r["glass_centroid"]})
            n_sl += 1
        elif v["kind"] == "lantern":
            r = lanterns.get(v["id"])
            if r is None:
                fail(f"机位可见吊灯笼 {v['id']} 不在灯笼表")
            chosen.append({
                "id": v["id"],
                "position": [float(x) for x in r["point_light_pos"]],
                "color_linear_rgb": [float(c) for c in lamp_color],
                "intensity_cd": float(lamp_cd * lantern_ratio),
            })
            notes.append({"id": v["id"], "kind": "lantern", "distance_m": v["distance_m"], "rule": r.get("point_light_rule"),
                          "escape": r.get("point_light_escape"), "glass_centroid": r["glass_centroid"]})
    if not chosen:
        fail(f"机位 {cand['id']} 视锥内无可见灯，无法构造夜景点光")
    return chosen, notes


def default_lamp_cd(cand_lamps: list[dict], facts: dict, ev100: float) -> tuple[float, str]:
    """等地面照度换算：I_ext = I_int × (h_ext/h_int)² × 2^(ev_ext − ev_int)。h_ext 取选中路灯离地高均值。"""
    lamps = {r["id"]: r for r in facts["streetlights"]["lamps"]}
    hs = [float(lamps[n["id"]]["height_above_ground_m"]) for n in cand_lamps if n["kind"] == "streetlight" and n["id"] in lamps]
    h_ext = float(np.mean(hs)) if hs else 6.3
    cd = INT_LAMP_CD * (h_ext / INT_LAMP_HEIGHT_M) ** 2 * (2.0 ** (ev100 - INT_EV100))
    note = (f"I_ext = {INT_LAMP_CD:.4f} × ({h_ext:.2f}/{INT_LAMP_HEIGHT_M:.1f})² × 2^({ev100:g} − ({INT_EV100:g})) = {cd:.5f} cd"
            f"（室内 4 点光 0.0182 cd @ ev100 −4、灯高 3.0 m 等地面照度换算；仅探针起点）")
    return cd, note


def write_corpus(cand: dict, row: dict, notes: list[dict], gltf: str, camera_id: str) -> None:
    CORPUS_OUT.mkdir(exist_ok=True)
    common_note = ("借壳登记：文件名按 scene_id 派生规则沿用 bistro_interior（共享体场景闭集所留），实体 = BistroExterior "
                   f"雨夜展示面（day_0902_rain_night，机位 {camera_id}）；本文件只被渲染 bin 以 sha 形式登记进 evidence g10_provenance，不裁决")
    cam = {
        "schema": "rurix.g10.camera_params.v1", "scene_id": "bistro-interior",
        "eye": [float(v) for v in cand["eye"]], "target": [float(v) for v in cand["target"]], "up": [float(v) for v in cand["up"]],
        "fov_y_deg": float(row["camera"]["fov_y_deg"]), "resolution": [row["camera"]["resolution"]["w"], row["camera"]["resolution"]["h"]],
        "note": common_note + f"；机位描述：{cand['desc']}；world = 1.6 × glTF 局部（根节点缩放）",
    }
    (CORPUS_OUT / "camera_bistro_interior.json").write_text(json.dumps(cam, ensure_ascii=False, indent=1), encoding="utf-8")
    lighting = {
        "schema": "rurix.g10.lighting_params.v1", "scene_id": "bistro-interior",
        "lights": [],
        "note": common_note + "；无日光/无天光（sun/sky = 0，g35 车道不消费），照明 = 契约 point_lights（路灯/吊灯笼派生位）+ emissive 常量",
        "point_lights": [dict(pl, **{"derived_from": f"{n['kind']} {n['id']}：facts 逃逸测试最优位（规则 {n['rule']}，逃逸比 {n['escape']}），距机位 {n['distance_m']} m"})
                         for pl, n in zip(row["lighting"]["point_lights"], notes)],
        "emissive_materials": row["lighting"]["emissive_materials"],
    }
    (CORPUS_OUT / "lighting_bistro_interior.json").write_text(json.dumps(lighting, ensure_ascii=False, indent=1), encoding="utf-8")
    params = {
        "camera": copy.deepcopy(row["camera"]),
        "lighting": {
            "sun": {"direction": [0.0, -1.0, 0.0], "intensity_lux": 0.0, "color_linear_rgb": [1.0, 1.0, 1.0]},
            "sky": {"intensity": 0.0, "cubemap_id": None},
            "exposure": copy.deepcopy(row["exposure"]),
        },
        "time": {"fixed_dt_s": 1.0 / 60.0, "warmup_frames": 10, "capture_frame_index": -1, "random_seed": 42,
                 "jitter": {"sequence": "halton_2_3", "index_base": 0, "scale": 1.0}},
        "post": {"view_transform": "aces13", "bloom": False, "vignette": False, "motion_blur": False, "dof": False},
        "gltf": gltf,
        "note": common_note,
    }
    (CORPUS_OUT / "contract_params_bistro_interior.json").write_text(json.dumps(params, ensure_ascii=False, indent=1), encoding="utf-8")
    print(f"[corpus] 写出 {CORPUS_OUT}/ 三件（借壳登记）")


def main() -> int:
    ap = argparse.ArgumentParser(description="派生 BistroExterior 雨夜借壳契约")
    ap.add_argument("--facts", type=Path, default=DEFAULT_FACTS)
    ap.add_argument("--camera", default="C1", help="facts.camera_candidates[].id（C0/C1/C2/C3）")
    ap.add_argument("--ev100", type=float, default=-7.0)
    ap.add_argument("--lamp-cd", type=float, default=None, help="路灯点光 intensity_cd（缺省 = 等地面照度换算起点）")
    ap.add_argument("--lamp-k", type=int, default=14, help="视锥内可见路灯按距离取前 K 盏")
    ap.add_argument("--lantern-cd-ratio", type=float, default=0.35, help="吊灯笼点光 = 路灯 × 该比")
    ap.add_argument("--lamp-color", default="1.0,0.72,0.42", help="点光线性色（暖白）")
    ap.add_argument("--le-scale", type=float, default=1.0)
    ap.add_argument("--fov-y-deg", type=float, default=None, help="覆写机位 fov_y_deg")
    ap.add_argument("--gltf", type=Path, default=DEFAULT_GLTF)
    ap.add_argument("--gltf-sha256", default=None, help="gltf_product_digest（缺省对 --gltf 实算）")
    ap.add_argument("--out", type=Path, default=None)
    ap.add_argument("--tag", default=None, help="契约标签（缺省 = camera id）；输出名 contract_rain_night_<tag>.json")
    ap.add_argument("--write-corpus", action="store_true", help="另写 g10_corpus/ 三件")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()

    if args.selftest:
        return selftest()

    if not args.facts.is_file():
        fail(f"facts 缺失：{args.facts}")
    if not FROZEN_CONTRACT.is_file():
        fail(f"冻结契约缺失：{FROZEN_CONTRACT}")
    facts = json.loads(args.facts.read_text(encoding="utf-8"))
    cand = next((c for c in facts["camera_candidates"] if c["id"] == args.camera), None)
    if cand is None:
        fail(f"机位 {args.camera} 不在 facts（可选：{[c['id'] for c in facts['camera_candidates']]}）")
    if not cand["clearance"]["pass"]:
        fail(f"机位 {args.camera} 视点净空检查未过：{cand['clearance']}")
    lamp_color = parse_vec3(args.lamp_color, "--lamp-color")
    if not (-16.0 <= args.ev100 <= 16.0):
        fail("--ev100 须 ∈ [−16, 16]")
    if args.lamp_k < 1:
        fail("--lamp-k 须 ≥ 1")
    if args.gltf_sha256:
        if not (args.gltf_sha256.startswith("sha256:") and len(args.gltf_sha256) == 71):
            fail("--gltf-sha256 形态须 sha256:<64hex>")
        gltf_digest = args.gltf_sha256
    else:
        if not args.gltf.is_file():
            fail(f"glTF 不存在且未给 --gltf-sha256：{args.gltf}")
        gltf_digest = sha256_file(args.gltf)

    frozen = json.loads(FROZEN_CONTRACT.read_text(encoding="utf-8"))
    contract = copy.deepcopy(frozen)
    idx = next((i for i, s in enumerate(contract["scenes"]) if s["scene_id"] == "bistro-interior"), None)
    if idx is None:
        fail("冻结契约无 bistro-interior 行")
    row = contract["scenes"][idx]

    # 相机（look-at → quat，与 facts 同一套数学；全精度）
    eye = [float(v) for v in cand["eye"]]
    target = [float(v) for v in cand["target"]]
    up = [float(v) for v in cand["up"]]
    q = lookat_quat_wxyz(eye, target, up)
    q = q / np.linalg.norm(q)
    fov = float(args.fov_y_deg if args.fov_y_deg is not None else cand["fov_y_deg"])
    if not (0.0 < fov < 180.0):
        fail("fov_y_deg 须 ∈ (0, 180)")
    row["camera"] = {
        "position": eye,
        "orientation_quat": [float(v) for v in q],
        "fov_y_deg": fov,
        "near": float(cand["near"]),
        "far": float(cand["far"]),
        "resolution": {"w": int(cand["resolution"]["w"]), "h": int(cand["resolution"]["h"])},
    }
    row["exposure"] = {"mode": "manual", "ev100": float(args.ev100)}

    # 灯面
    visible_notes_all = cand["lamps_visible_sorted"]
    lamp_cd_note = None
    if args.lamp_cd is None:
        args.lamp_cd, lamp_cd_note = default_lamp_cd(visible_notes_all, facts, args.ev100)
        print(f"[lamp-cd] 缺省换算：{lamp_cd_note}")
    if args.lamp_cd < 0:
        fail("--lamp-cd 须 ≥ 0")
    points, notes = pick_lights(facts, cand, args.lamp_k, args.lamp_cd, args.lantern_cd_ratio, lamp_color)
    emis = emissive_rows(facts, args.ev100, args.le_scale)
    emis_notes = [e.pop("_note") for e in emis]
    row["lighting"] = {
        "quad_lights": [],
        "point_lights": points,
        "emissive_materials": emis,
        "sun_intensity_lux": 0.0,
        "sky_intensity": 0.0,
    }
    row["material_policy"] = {"texture_mean_albedo": True, "white_tex_to_white": False}
    row["gltf_product_digest"] = gltf_digest
    # m133_manifest_digest 保持冻结值（M133 清单 digest 语义未变；借壳在 provenance 说明）

    tag = args.tag or args.camera
    out = args.out or (HERE / f"contract_rain_night_{tag}.json")
    contract["provenance"]["showcase_note"] = (
        "day_0902_rain_night 借壳展示契约：几何 = ORCA BistroExterior（CC-BY-4.0）经 FBX2glTF v0.9.7 无纹理臂 + "
        "fix_exterior_textures.py URI 回接派生（K:\\rurix_g10_cache\\bistro-orca\\v5_2\\derived\\BistroExterior\\）；"
        "scene_id 字面 bistro-interior 仅为共享体 parse_contract 场景闭集所留的借壳，evidence 的 scene 标签与 G10 语料文件名因此失真；"
        "m133_manifest_digest 沿冻结值（清单语义未变）；相机 / 点光 / emissive 由 derive_rain_night_contract.py 从 "
        "exterior_scene_facts.json 派生（世界坐标 = 1.6 × glTF 局部）；无日光/无天光（sun/sky = 0，车道不消费）；"
        "点光位 = 灯罩闭合盒外侧逃逸测试最优位（radius 恒 0、阴影射线无半径截断）；emissive 只可见不投光。2026-09-03。"
    )
    contract["provenance"]["showcase_params"] = {
        "camera_id": args.camera, "camera_desc": cand["desc"], "eye": eye, "target": target, "up": up, "fov_y_deg": fov,
        "ev100": args.ev100, "lamp_cd": args.lamp_cd, "lamp_cd_derivation": lamp_cd_note, "lamp_k": args.lamp_k,
        "lantern_cd_ratio": args.lantern_cd_ratio, "lamp_color_linear_rgb": lamp_color, "le_scale": args.le_scale,
        "emissive_le_rule": "Le = display 目标亮度 × 2^ev100 × le_scale；色向 = emissive DDS 线性均值方向（店招轻度去饱和）",
        "emissive_notes": emis_notes, "point_light_sources": notes,
        "gltf": str(args.gltf).replace("\\", "/"), "facts": str(args.facts.name),
        "emitter_suggestion_cmdline": cand["emitter"]["cmdline"],
        "emitter_life_s": cand["emitter"]["emitter_life_s"],
    }
    out.write_text(json.dumps(contract, ensure_ascii=False, indent=1), encoding="utf-8")
    digest = harness_digest(out)
    print(f"[契约] {out.name}  camera={args.camera} ev100={args.ev100:g} lamp_cd={args.lamp_cd:.5f} lamp_k={args.lamp_k} "
          f"point_lights={len(points)}（路灯 {sum(1 for n in notes if n['kind']=='streetlight')} + 灯笼 {sum(1 for n in notes if n['kind']=='lantern')}）")
    print(f"[digest] {digest}")

    # 索引（幂等：同路径覆盖）
    index = []
    if INDEX_PATH.is_file():
        index = json.loads(INDEX_PATH.read_text(encoding="utf-8"))
    rel = str(out.relative_to(HERE)).replace("\\", "/") if out.is_relative_to(HERE) else str(out)
    rec = {
        "contract": rel, "expect_digest": digest, "tag": tag, "camera": args.camera, "ev100": args.ev100,
        "lamp_cd": args.lamp_cd, "lamp_k": args.lamp_k, "lantern_cd_ratio": args.lantern_cd_ratio, "le_scale": args.le_scale,
        "fov_y_deg": fov, "point_lights": len(points), "gltf_product_digest": gltf_digest,
        "generated_at": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
    }
    index = [r for r in index if r.get("contract") != rel] + [rec]
    INDEX_PATH.write_text(json.dumps(index, ensure_ascii=False, indent=1), encoding="utf-8")

    if args.write_corpus:
        write_corpus(cand, row, notes, str(args.gltf).replace("\\", "/"), args.camera)

    print(f"[命令建议] --contract artifacts\\day_0902_rain_night\\{rel.replace('/', chr(92))} --expect-digest {digest} --gltf {args.gltf} "
          f"--g10-dir artifacts\\day_0902_rain_night\\g10_corpus --particles on --rain-shutter 1.0 " + cand["emitter"]["cmdline"])
    return 0


if __name__ == "__main__":
    sys.exit(main())
