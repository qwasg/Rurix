#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G13.4 harness — UE 超分对拍 / Lumen GI 对照双契约（RXS-0405 / RXS-0406 L1/L2）
UE 侧解析脚本（UE 内嵌 CPython 载体；host 侧门脚本 ci/g13_ue_upscale_parity_smoke.py
与 ci/g13_ue_lumen_gi_parity_smoke.py 内嵌独立第三实现，Rurix Rust harness
g13_4_ue_upscale_parity_render --contract-digest 为第二实现——三方 digest 全等机核）。

schema = rurix.g13.ue_upscale_parity_contract.v1 / rurix.g13.ue_lumen_gi_parity_contract.v1
（字段闭集，schema 外字段注入即拒；null 禁入；NaN/±Inf 禁入；unit-norm 谓词常量
2^-40 沿 RXS-0384 L2）。canonical 字节布局（RXS-0405 L2 / RXS-0406 L2）：

  版本前缀 = ASCII "G13USP-1" + NUL（47 31 33 55 53 50 2D 31 00，9 字节；M-c）
           | ASCII "G13LGP-1" + NUL（47 31 33 4C 47 50 2D 31 00，9 字节；M-d）；
  类型标签：f64=0x01 / u32=0x02 / u64=0x03 / str=0x04 / bool=0x05 /
           obj 0x07..0x08 / arr 0x09..0x0A；
  键 = u32 length-prefix UTF-8，对象键按 Unicode code point 升序；
  f64 = binary64 小端；u32/u64 宽度 schema 驱动（禁值域分派）；
  digest 域 = 全字段闭集除 provenance 块；digest = SHA-256(preimage)。

用法：py -3 g13_parity_contract.py <contract.json>  # 打印 contract digest

Assisted-by: Kimi-K3（G13.4 UE 对拍波）
"""
import hashlib
import json
import math
import struct

UNIT_NORM_TOL = 2.0 ** -40  # RXS-0384 L2 谓词常量继承

SCENE_SET = ("cornell-box", "bistro-interior")  # 场景闭集（M133 清单转引）

F64 = "f64"
U32 = "u32"
U64 = "u64"
STR = "str"
BOOL = "bool"


class ContractError(ValueError):
    """fail-closed：任何 schema 违例即抛出。"""


# ---------------------------------------------------------------------------
# 逐契约冻结面（RXS-0405 L1 / RXS-0406 L1 字段闭集）
# ---------------------------------------------------------------------------

UPSCALE_SCHEMA_ID = "rurix.g13.ue_upscale_parity_contract.v1"
UPSCALE_VERSION_PREFIX = b"G13USP-1\x00"  # RXS-0405 L2 冻结字面
UPSCALE_TIERS = (50, 67, 100)  # tier_sequence 闭集
UPSCALE_BACKENDS = ("dlss_sr", "fsr_3_1_5", "tsr_device")  # 值域闭集（code point 升序）
UPSCALE_DLSS_QUALITY = ("Balanced", "DLAA", "Performance", "Quality", "UltraPerformance", "UltraQuality")
UPSCALE_QUALITY_MAP = {"50": "Performance", "67": "Quality", "100": "DLAA"}  # 契约冻结映射

LUMEN_SCHEMA_ID = "rurix.g13.ue_lumen_gi_parity_contract.v1"
LUMEN_VERSION_PREFIX = b"G13LGP-1\x00"  # RXS-0406 L2 冻结字面
LUMEN_GI_SURFACE = ("multibounce_chain", "screen_probe_near_field", "world_cache_far_field")  # 三锚闭集


def _f64(name, v):
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise ContractError(f"{name}: expected f64, got {type(v).__name__}")
    v = float(v)
    if math.isnan(v) or math.isinf(v):
        raise ContractError(f"{name}: NaN/Inf forbidden")
    return v


def _u32(name, v):
    if isinstance(v, bool) or not isinstance(v, int):
        raise ContractError(f"{name}: expected u32, got {type(v).__name__}")
    if not (0 <= v <= 0xFFFFFFFF):
        raise ContractError(f"{name}: u32 越域 {v}")
    return v


def _u64(name, v):
    if isinstance(v, bool) or not isinstance(v, int):
        raise ContractError(f"{name}: expected u64, got {type(v).__name__}")
    if not (0 <= v <= 0xFFFFFFFFFFFFFFFF):
        raise ContractError(f"{name}: u64 越域 {v}")
    return v


def _str(name, v):
    if not isinstance(v, str) or not v:
        raise ContractError(f"{name}: expected non-empty str")
    return v


def _bool(name, v):
    if not isinstance(v, bool):
        raise ContractError(f"{name}: expected bool")
    return v


def _f64v(name, v, n):
    if not isinstance(v, list) or len(v) != n:
        raise ContractError(f"{name}: expected f64×{n}")
    return [_f64(f"{name}[{i}]", x) for i, x in enumerate(v)]


def _unit_norm(name, v):
    q = _f64v(name, v, 4)
    n2 = sum(x * x for x in q)
    if abs(n2 - 1.0) > UNIT_NORM_TOL:
        raise ContractError(f"{name}: 非单位四元数（‖q‖²−1 = {n2 - 1.0}）")
    return q


def _closed(name, obj, keys):
    if not isinstance(obj, dict):
        raise ContractError(f"{name}: expected obj")
    extra = set(obj) - set(keys)
    if extra:
        raise ContractError(f"{name}: schema 外字段注入 {sorted(extra)}")
    missing = [k for k in keys if k not in obj]
    if missing:
        raise ContractError(f"{name}: 缺字段 {missing}")
    return obj


def parse_scene(idx, s):
    """逐场景行解析（双契约共用——RXS-0406 L1 逐场景行转引 RXS-0405 L1）。"""
    _closed(f"scenes[{idx}]", s, (
        "scene_id", "m133_manifest_digest", "gltf_product_digest",
        "camera", "exposure", "lighting", "material_policy",
    ))
    sid = _str("scene_id", s["scene_id"])
    if sid not in SCENE_SET:
        raise ContractError(f"scene_id {sid} 越场景闭集 {SCENE_SET}")
    cam = _closed("camera", s["camera"], (
        "position", "orientation_quat", "fov_y_deg", "near", "far", "resolution",
    ))
    _f64v("camera.position", cam["position"], 3)
    _unit_norm("camera.orientation_quat", cam["orientation_quat"])
    fov = _f64("camera.fov_y_deg", cam["fov_y_deg"])
    if not (0.0 < fov < 180.0):
        raise ContractError("fov_y_deg 越域")
    near = _f64("camera.near", cam["near"])
    far = _f64("camera.far", cam["far"])
    if not (0.0 < near < far):
        raise ContractError("near/far 越域")
    res = _closed("camera.resolution", cam["resolution"], ("w", "h"))
    _u32("camera.resolution.w", res["w"])
    _u32("camera.resolution.h", res["h"])
    exp = _closed("exposure", s["exposure"], ("mode", "ev100"))
    if exp["mode"] != "manual":
        raise ContractError("exposure.mode 仅 manual")
    _f64("exposure.ev100", exp["ev100"])
    lig = _closed("lighting", s["lighting"], (
        "quad_lights", "point_lights", "emissive_materials",
        "sun_intensity_lux", "sky_intensity",
    ))
    _f64("lighting.sun_intensity_lux", lig["sun_intensity_lux"])
    _f64("lighting.sky_intensity", lig["sky_intensity"])
    if not isinstance(lig["quad_lights"], list):
        raise ContractError("quad_lights: expected array")
    for i, q in enumerate(lig["quad_lights"]):
        _closed(f"quad_lights[{i}]", q, ("p00", "e1", "e2", "le_linear_rgb"))
        _f64v(f"quad_lights[{i}].p00", q["p00"], 3)
        _f64v(f"quad_lights[{i}].e1", q["e1"], 3)
        _f64v(f"quad_lights[{i}].e2", q["e2"], 3)
        le = _f64v(f"quad_lights[{i}].le_linear_rgb", q["le_linear_rgb"], 3)
        if any(c < 0.0 for c in le):
            raise ContractError(f"quad_lights[{i}].le 负值")
    if not isinstance(lig["point_lights"], list):
        raise ContractError("point_lights: expected array")
    for i, p in enumerate(lig["point_lights"]):
        _closed(f"point_lights[{i}]", p, ("id", "position", "color_linear_rgb", "intensity_cd"))
        _str(f"point_lights[{i}].id", p["id"])
        _f64v(f"point_lights[{i}].position", p["position"], 3)
        col = _f64v(f"point_lights[{i}].color_linear_rgb", p["color_linear_rgb"], 3)
        if any(c < 0.0 for c in col):
            raise ContractError(f"point_lights[{i}].color 负值")
        if _f64(f"point_lights[{i}].intensity_cd", p["intensity_cd"]) < 0.0:
            raise ContractError(f"point_lights[{i}].intensity_cd 负值")
    if not isinstance(lig["emissive_materials"], list):
        raise ContractError("emissive_materials: expected array")
    for i, m in enumerate(lig["emissive_materials"]):
        _closed(f"emissive_materials[{i}]", m, ("material_name", "material_index", "le_linear_rgb", "area_m2"))
        _str(f"emissive_materials[{i}].material_name", m["material_name"])
        _u32(f"emissive_materials[{i}].material_index", m["material_index"])
        le = _f64v(f"emissive_materials[{i}].le_linear_rgb", m["le_linear_rgb"], 3)
        if any(c < 0.0 for c in le):
            raise ContractError(f"emissive_materials[{i}].le 负值")
        if _f64(f"emissive_materials[{i}].area_m2", m["area_m2"]) <= 0.0:
            raise ContractError(f"emissive_materials[{i}].area_m2 非正")
    pol = _closed("material_policy", s["material_policy"], ("texture_mean_albedo", "white_tex_to_white"))
    _bool("material_policy.texture_mean_albedo", pol["texture_mean_albedo"])
    _bool("material_policy.white_tex_to_white", pol["white_tex_to_white"])
    _str("m133_manifest_digest", s["m133_manifest_digest"])
    _str("gltf_product_digest", s["gltf_product_digest"])
    return s


def _parse_scenes(doc):
    scenes = doc["scenes"]
    if not isinstance(scenes, list) or len(scenes) != 2:
        raise ContractError("scenes 须恰二行")
    ids = set()
    for i, s in enumerate(scenes):
        parse_scene(i, s)
        ids.add(s["scene_id"])
    if ids != set(SCENE_SET):
        raise ContractError(f"场景闭集不全等: {ids}")
    if not isinstance(doc.get("provenance"), dict):
        raise ContractError("provenance 须为 obj（不入 digest）")


def parse_upscale_contract(text):
    """M-c 契约 fail-closed 解析（RXS-0405 L1 字段闭集 + 约束谓词）。"""
    try:
        doc = json.loads(text)
    except json.JSONDecodeError as e:
        raise ContractError(f"JSON 解析失败: {e}") from e
    _closed("root", doc, (
        "schema", "contract_id", "version", "tier_sequence", "frame_count",
        "seed", "calibration_seed", "noise_probe_tier", "rurix_backends",
        "ue_dlss_quality_map", "rendering_policy", "scenes", "provenance",
    ))
    if doc["schema"] != UPSCALE_SCHEMA_ID:
        raise ContractError(f"schema 字面不符: {doc['schema']}")
    _str("contract_id", doc["contract_id"])
    _u32("version", doc["version"])
    tiers = doc["tier_sequence"]
    if not isinstance(tiers, list) or not tiers:
        raise ContractError("tier_sequence 空/非数组")
    tiers = [_u32(f"tier_sequence[{i}]", v) for i, v in enumerate(tiers)]
    if any(tiers[i] >= tiers[i + 1] for i in range(len(tiers) - 1)):
        raise ContractError("tier_sequence 非严格递增")
    if tuple(tiers) != UPSCALE_TIERS:
        raise ContractError(f"tier_sequence 越闭集 {UPSCALE_TIERS}: {tiers}")
    _u32("frame_count", doc["frame_count"])
    seed = _u64("seed", doc["seed"])
    cal_seed = _u64("calibration_seed", doc["calibration_seed"])
    if cal_seed == seed:
        raise ContractError("calibration_seed == seed（标定腿须异 seed）")
    probe = _u32("noise_probe_tier", doc["noise_probe_tier"])
    if probe not in tiers:
        raise ContractError("noise_probe_tier 越 tier_sequence")
    backends = doc["rurix_backends"]
    if not isinstance(backends, list) or not backends:
        raise ContractError("rurix_backends 空/非数组")
    backends = [_str(f"rurix_backends[{i}]", v) for i, v in enumerate(backends)]
    if set(backends) != set(UPSCALE_BACKENDS) or len(backends) != len(UPSCALE_BACKENDS):
        raise ContractError(f"rurix_backends 越闭集 {UPSCALE_BACKENDS}: {backends}")
    qm = _closed("ue_dlss_quality_map", doc["ue_dlss_quality_map"], tuple(UPSCALE_QUALITY_MAP.keys()))
    for t, q in qm.items():
        _str(f"ue_dlss_quality_map[{t}]", q)
        if q not in UPSCALE_DLSS_QUALITY:
            raise ContractError(f"ue_dlss_quality_map[{t}] 越 DLSS 质量枚举: {q}")
        if UPSCALE_QUALITY_MAP.get(t) != q:
            raise ContractError(f"ue_dlss_quality_map[{t}] 离冻结映射 {UPSCALE_QUALITY_MAP[t]}: {q}")
    pol = _closed("rendering_policy", doc["rendering_policy"], (
        "tonemap", "denoiser", "ue_temporal_upscaler", "jitter",
    ))
    if pol["tonemap"] != "off" or pol["denoiser"] != "off":
        raise ContractError("rendering_policy tonemap/denoiser 须 const off")
    if pol["ue_temporal_upscaler"] != "dlss_plugin":
        raise ContractError("rendering_policy.ue_temporal_upscaler 须 const dlss_plugin")
    if pol["jitter"] != "halton_static":
        raise ContractError("rendering_policy.jitter 须 const halton_static")
    _parse_scenes(doc)
    return doc


def parse_lumen_contract(text):
    """M-d 契约 fail-closed 解析（RXS-0406 L1 字段闭集 + 约束谓词）。"""
    try:
        doc = json.loads(text)
    except json.JSONDecodeError as e:
        raise ContractError(f"JSON 解析失败: {e}") from e
    _closed("root", doc, (
        "schema", "contract_id", "version", "seed", "calibration_seed",
        "rendering_policy", "rurix_gi_surface", "scenes", "provenance",
    ))
    if doc["schema"] != LUMEN_SCHEMA_ID:
        raise ContractError(f"schema 字面不符: {doc['schema']}")
    _str("contract_id", doc["contract_id"])
    _u32("version", doc["version"])
    seed = _u64("seed", doc["seed"])
    cal_seed = _u64("calibration_seed", doc["calibration_seed"])
    if cal_seed == seed:
        raise ContractError("calibration_seed == seed（标定腿须异 seed）")
    pol = _closed("rendering_policy", doc["rendering_policy"], (
        "ue_gi_method", "ue_reflection_method", "tonemap", "denoiser",
        "indirect_derivation",
    ))
    if pol["ue_gi_method"] != "lumen" or pol["ue_reflection_method"] != "lumen":
        raise ContractError("rendering_policy ue_gi_method/ue_reflection_method 须 const lumen")
    if pol["tonemap"] != "off" or pol["denoiser"] != "off":
        raise ContractError("rendering_policy tonemap/denoiser 须 const off")
    if pol["indirect_derivation"] != "gi_on_minus_gi_off":
        raise ContractError("rendering_policy.indirect_derivation 须 const gi_on_minus_gi_off")
    surf = doc["rurix_gi_surface"]
    if not isinstance(surf, list) or not surf:
        raise ContractError("rurix_gi_surface 空/非数组")
    surf = [_str(f"rurix_gi_surface[{i}]", v) for i, v in enumerate(surf)]
    if set(surf) != set(LUMEN_GI_SURFACE) or len(surf) != len(LUMEN_GI_SURFACE):
        raise ContractError(f"rurix_gi_surface 越三锚闭集 {LUMEN_GI_SURFACE}: {surf}")
    _parse_scenes(doc)
    return doc


def parse_contract(text):
    """按 schema 字面分派双契约解析（fail-closed）。"""
    doc = json.loads(text)
    schema = doc.get("schema") if isinstance(doc, dict) else None
    if schema == UPSCALE_SCHEMA_ID:
        return parse_upscale_contract(text)
    if schema == LUMEN_SCHEMA_ID:
        return parse_lumen_contract(text)
    raise ContractError(f"未知 schema 字面: {schema}")


def version_prefix(doc):
    if doc["schema"] == UPSCALE_SCHEMA_ID:
        return UPSCALE_VERSION_PREFIX
    if doc["schema"] == LUMEN_SCHEMA_ID:
        return LUMEN_VERSION_PREFIX
    raise ContractError(f"未知 schema 字面: {doc['schema']}")


# ---------------------------------------------------------------------------
# canonical preimage（RXS-0405 L2 / RXS-0406 L2；digest 域 = 字段闭集除 provenance）
# ---------------------------------------------------------------------------

_TAG = {F64: b"\x01", U32: b"\x02", U64: b"\x03", STR: b"\x04", BOOL: b"\x05"}


def _enc_key(buf, k):
    b = k.encode("utf-8")
    buf += struct.pack("<I", len(b))
    buf += b
    return buf


def _enc_scalar(buf, ty, v):
    buf += _TAG[ty]
    if ty == F64:
        buf += struct.pack("<d", float(v))
    elif ty == U32:
        buf += struct.pack("<I", int(v))
    elif ty == U64:
        buf += struct.pack("<Q", int(v))
    elif ty == STR:
        b = v.encode("utf-8")
        buf += struct.pack("<I", len(b))
        buf += b
    elif ty == BOOL:
        buf += b"\x01" if v else b"\x00"
    return buf


# 字段类型表（digest 域；键序由 code point 升序通用律承载，本表只钉类型）。
UPSCALE_ROOT_TYPES = {
    "calibration_seed": U64, "contract_id": STR, "frame_count": U32,
    "noise_probe_tier": U32, "rendering_policy": "obj_upscale_policy",
    "rurix_backends": "arr_str", "schema": STR, "scenes": "arr",
    "seed": U64, "tier_sequence": "arr_u32", "ue_dlss_quality_map": "obj_strmap",
    "version": U32,
}
UPSCALE_POLICY_TYPES = {
    "denoiser": STR, "jitter": STR, "tonemap": STR, "ue_temporal_upscaler": STR,
}
LUMEN_ROOT_TYPES = {
    "calibration_seed": U64, "contract_id": STR,
    "rendering_policy": "obj_lumen_policy", "rurix_gi_surface": "arr_str",
    "schema": STR, "scenes": "arr", "seed": U64, "version": U32,
}
LUMEN_POLICY_TYPES = {
    "denoiser": STR, "indirect_derivation": STR, "tonemap": STR,
    "ue_gi_method": STR, "ue_reflection_method": STR,
}
CAMERA_TYPES = {
    "far": F64, "fov_y_deg": F64, "near": F64, "orientation_quat": "arr3_f64",
    "position": "arr3_f64", "resolution": "obj",
}
RES_TYPES = {"h": U32, "w": U32}
EXPOSURE_TYPES = {"ev100": F64, "mode": STR}
LIGHTING_TYPES = {
    "emissive_materials": "arr_emissive", "point_lights": "arr_point",
    "quad_lights": "arr_quad", "sky_intensity": F64, "sun_intensity_lux": F64,
}
QUAD_TYPES = {"e1": "arr3_f64", "e2": "arr3_f64", "le_linear_rgb": "arr3_f64", "p00": "arr3_f64"}
POINT_TYPES = {"color_linear_rgb": "arr3_f64", "id": STR, "intensity_cd": F64, "position": "arr3_f64"}
EMISSIVE_TYPES = {"area_m2": F64, "le_linear_rgb": "arr3_f64", "material_index": U32, "material_name": STR}
SCENE_TYPES = {
    "camera": "obj_camera", "exposure": "obj_exposure",
    "gltf_product_digest": STR, "lighting": "obj_lighting",
    "m133_manifest_digest": STR, "material_policy": "obj_matpol", "scene_id": STR,
}
MATPOL_TYPES = {"texture_mean_albedo": BOOL, "white_tex_to_white": BOOL}


def _enc_typed(buf, ty, v):
    """schema 驱动类型编码（对象键 code point 升序；数组保序）。"""
    if ty in _TAG:
        return _enc_scalar(buf, ty, v)
    if ty == "arr_u32":
        buf += b"\x09"
        for x in v:
            buf = _enc_scalar(buf, U32, x)
        return buf + b"\x0a"
    if ty == "arr_str":
        buf += b"\x09"
        for x in v:
            buf = _enc_scalar(buf, STR, x)
        return buf + b"\x0a"
    if ty == "arr3_f64":
        buf += b"\x09"
        for x in v:
            buf = _enc_scalar(buf, F64, x)
        return buf + b"\x0a"
    if ty == "obj_upscale_policy":
        return _enc_obj(buf, v, UPSCALE_POLICY_TYPES)
    if ty == "obj_lumen_policy":
        return _enc_obj(buf, v, LUMEN_POLICY_TYPES)
    if ty == "obj_strmap":
        return _enc_obj(buf, v, {k: STR for k in v})
    if ty == "obj_camera":
        return _enc_obj(buf, v, CAMERA_TYPES)
    if ty == "obj_exposure":
        return _enc_obj(buf, v, EXPOSURE_TYPES)
    if ty == "obj_lighting":
        return _enc_obj(buf, v, LIGHTING_TYPES)
    if ty == "obj_matpol":
        return _enc_obj(buf, v, MATPOL_TYPES)
    if ty == "arr_quad":
        buf += b"\x09"
        for q in v:
            buf = _enc_obj(buf, q, QUAD_TYPES)
        return buf + b"\x0a"
    if ty == "arr_point":
        buf += b"\x09"
        for p in v:
            buf = _enc_obj(buf, p, POINT_TYPES)
        return buf + b"\x0a"
    if ty == "arr_emissive":
        buf += b"\x09"
        for m in v:
            buf = _enc_obj(buf, m, EMISSIVE_TYPES)
        return buf + b"\x0a"
    if ty == "arr":
        buf += b"\x09"
        for s in v:
            buf = _enc_obj(buf, s, SCENE_TYPES)
        return buf + b"\x0a"
    raise ContractError(f"未知类型标签 {ty}")


def _enc_obj(buf, obj, types):
    buf += b"\x07"
    for k in sorted(obj, key=lambda s: [ord(c) for c in s]):
        if k not in types:
            raise ContractError(f"digest 域外字段 {k}")
        buf = _enc_key(buf, k)
        if types[k] == "obj" and k == "resolution":
            buf = _enc_obj(buf, obj[k], RES_TYPES)
        else:
            buf = _enc_typed(buf, types[k], obj[k])
    return buf + b"\x08"


def canonical_preimage(doc):
    """digest 域 = 根字段闭集除 provenance（RXS-0405 L2 / RXS-0406 L2）。"""
    buf = bytearray(version_prefix(doc))
    body = {k: v for k, v in doc.items() if k != "provenance"}
    if doc["schema"] == UPSCALE_SCHEMA_ID:
        return bytes(_enc_obj(buf, body, UPSCALE_ROOT_TYPES))
    if doc["schema"] == LUMEN_SCHEMA_ID:
        return bytes(_enc_obj(buf, body, LUMEN_ROOT_TYPES))
    raise ContractError(f"未知 schema 字面: {doc['schema']}")


def contract_digest(doc):
    return "sha256:" + hashlib.sha256(canonical_preimage(doc)).hexdigest()


def main():
    import sys
    if len(sys.argv) < 2:
        print("usage: g13_parity_contract.py <contract.json>  # 打印 contract digest", file=sys.stderr)
        return 2
    text = open(sys.argv[1], "r", encoding="utf-8").read()
    doc = parse_contract(text)
    print(contract_digest(doc))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
