#!/usr/bin/env python3
"""G10.2 harness — RFC-0026 §4.6 双端确定性契约参数 schema 的 UE 侧解析脚本（UE 内嵌 CPython 载体）。

字节布局已按 spec/visual_comparison.md RXS-0384 L3 单源冻结（G10.2 spec-first，2026-08-15）：
SPEC_BYTE_LAYOUT 为冻结字面，禁改；Rurix 侧骨架参考解析器（ci/g10_dual_determinism_contract_smoke.py）
按同字面独立实现，双端 digest 对拍由 M130 门机核。此前 DRAFT_BYTE_LAYOUT 占位块已随 spec 冻结失效替换
（环境日志 §5 示例 digest c6ebe3f6… 为 DRAFT 布局产物，冻结后失效重算，如实留痕）。

schema 四节闭集（全字段必填；schema 外字段注入即拒；null 仅 sky.cubemap_id 合法）：
  camera   : position f64×3 · orientation_quat f64×4 (w,x,y,z) · fov_y_deg · near · far · resolution{w,h}
  lighting : sun{direction,intensity_lux,color_linear_rgb} · sky{intensity,cubemap_id} · exposure{mode:"manual",ev100}
  time     : fixed_dt_s · warmup_frames · capture_frame_index · random_seed · jitter{sequence:"halton_2_3",index_base,scale}
  post     : view_transform "aces13" · bloom/vignette/motion_blur/dof 全 false

值约定（RFC-0026 §4.6 冻结公式 + v1.1 章 E errata；spec/visual_comparison.md RXS-0384 L2
+ 修订记录 v1.1 勘误行）：契约世界系右手系/+Y up/米；UE 厘米/左手系/Z-up/水平 FOV。
  p_ue = (−z, x, y)·100 ；四元数共轭按 errata 修订式 q_ue = (w, z, −x, −y)（反射 M det=−1
  共轭 R(M·axis, −θ)：向量部 −M·v、标量部不变；原文「向量部同 M、转角保持」勘误不复用）；
  fov_h_ue = 2·atan(tan(fov_y/2)·aspect)；sun.direction 同 M（无单位换算）。
  unit-norm 判定式 |‖v‖²−1| ≤ 2^-40（schema 合法性谓词常量，非 measured）。

Assisted-by: Kimi-K3（G10.2 波）
"""
import hashlib
import json
import math
import struct

# ---------------------------------------------------------------------------
# SPEC_BYTE_LAYOUT —— spec/visual_comparison.md RXS-0384 L3 冻结字节布局（单源）。
# 版本前缀 = ASCII "G10DCP-1" + NUL（47 31 30 44 43 50 2D 31 00，9 字节）；
# 类型标签字节值如下；整数宽度 schema 驱动（u64 字段值 < 2^32 仍 8 字节，禁值域分派）；
# NaN/±Inf 禁入。改动 = spec 违例，M130 门 byte_layout_matches_spec 机器核验。
SPEC_BYTE_LAYOUT = {
    "version_prefix": b"G10DCP-1\x00",  # RXS-0384 L3 冻结字面
    "tag_f64": b"\x01",
    "tag_u32": b"\x02",
    "tag_u64": b"\x03",
    "tag_str": b"\x04",
    "tag_bool": b"\x05",
    "tag_null": b"\x06",
    "tag_obj_begin": b"\x07",
    "tag_obj_end": b"\x08",
    "tag_arr_begin": b"\x09",
    "tag_arr_end": b"\x0a",
}
SPEC_VERSION_PREFIX_HEX = "4731304443502d3100"  # RXS-0384 L3 版本前缀字节序列（hex 旁证）
# ---------------------------------------------------------------------------

UNIT_NORM_TOL = 2.0 ** -40  # schema 合法性谓词常量（RFC-0026 §4.6 冻结判定式，不走 g10_budget）

SCHEMA = {
    "camera": {
        "position": ("arr_f64", 3),
        "orientation_quat": ("arr_f64", 4),
        "fov_y_deg": "f64",
        "near": "f64",
        "far": "f64",
        "resolution": {"w": "u32", "h": "u32"},
    },
    "lighting": {
        "sun": {
            "direction": ("arr_f64", 3),
            "intensity_lux": "f64",
            "color_linear_rgb": ("arr_f64", 3),
        },
        "sky": {"intensity": "f64", "cubemap_id": "str_or_null"},
        "exposure": {"mode": ("enum", ("manual",)), "ev100": "f64"},
    },
    "time": {
        "fixed_dt_s": "f64",
        "warmup_frames": "u32",
        "capture_frame_index": "u32",
        "random_seed": "u64",
        "jitter": {
            "sequence": ("enum", ("halton_2_3",)),
            "index_base": "u32",
            "scale": "f64",
        },
    },
    "post": {
        "view_transform": ("enum", ("aces13",)),
        "bloom": ("const", False),
        "vignette": ("const", False),
        "motion_blur": ("const", False),
        "dof": ("const", False),
    },
}


class ContractError(ValueError):
    """fail-closed：任何 schema 违例即抛出。"""


def _check_f64(name, v):
    if isinstance(v, bool) or not isinstance(v, (int, float)):
        raise ContractError(f"{name}: expected f64, got {type(v).__name__}")
    v = float(v)
    if math.isnan(v) or math.isinf(v):
        raise ContractError(f"{name}: NaN/Inf forbidden")
    return v


def _check_uint(name, v, bits):
    if isinstance(v, bool) or not isinstance(v, int):
        raise ContractError(f"{name}: expected u{bits}, got {type(v).__name__}")
    if v < 0 or v >= 2 ** bits:
        raise ContractError(f"{name}: out of u{bits} range")
    return v


def _validate(name, spec, value):
    if isinstance(spec, dict):
        if not isinstance(value, dict):
            raise ContractError(f"{name}: expected object")
        extra = set(value) - set(spec)
        if extra:
            raise ContractError(f"{name}: unknown fields {sorted(extra)}")
        missing = set(spec) - set(value)
        if missing:
            raise ContractError(f"{name}: missing fields {sorted(missing)}")
        return {k: _validate(f"{name}.{k}", s, value[k]) for k, s in spec.items()}
    if isinstance(spec, tuple):
        kind = spec[0]
        if kind == "arr_f64":
            n = spec[1]
            if not isinstance(value, list) or len(value) != n:
                raise ContractError(f"{name}: expected f64[{n}]")
            return [_check_f64(f"{name}[{i}]", x) for i, x in enumerate(value)]
        if kind == "enum":
            if value not in spec[1]:
                raise ContractError(f"{name}: value {value!r} not in {spec[1]}")
            return value
        if kind == "const":
            if value is not spec[1] and value != spec[1]:
                raise ContractError(f"{name}: must be {spec[1]!r}")
            return value
        raise ContractError(f"{name}: bad spec {spec!r}")
    if spec == "f64":
        return _check_f64(name, value)
    if spec == "u32":
        return _check_uint(name, value, 32)
    if spec == "u64":
        return _check_uint(name, value, 64)
    if spec == "str_or_null":
        if value is not None and not isinstance(value, str):
            raise ContractError(f"{name}: expected string|null")
        return value
    raise ContractError(f"{name}: bad spec {spec!r}")


def parse_contract(text):
    """解析并校验参数 JSON 文本（strict fail-closed）。返回规范化后的 dict。"""
    data = json.loads(text)  # CPython json 浮点解析 correctly-rounded（RFC-0026 §4.6 口径）
    if not isinstance(data, dict):
        raise ContractError("top level must be object")
    extra = set(data) - set(SCHEMA)
    if extra:
        raise ContractError(f"unknown sections {sorted(extra)}")
    missing = set(SCHEMA) - set(data)
    if missing:
        raise ContractError(f"missing sections {sorted(missing)}")
    out = {k: _validate(k, SCHEMA[k], data[k]) for k in SCHEMA}
    q = out["camera"]["orientation_quat"]
    if abs(sum(x * x for x in q) - 1.0) > UNIT_NORM_TOL:
        raise ContractError("camera.orientation_quat: unit-norm violated (|q²−1| > 2^-40)")
    d = out["lighting"]["sun"]["direction"]
    if abs(sum(x * x for x in d) - 1.0) > UNIT_NORM_TOL:
        raise ContractError("lighting.sun.direction: unit-norm violated (|d²−1| > 2^-40)")
    return out


# ---------------------------------------------------------------------------
# canonical preimage（SPEC 冻结布局，RXS-0384 L3）+ SHA-256
# 编码与 SCHEMA 并行行走（整数宽度 schema 驱动，禁值域分派）。
# ---------------------------------------------------------------------------

def _enc_value(buf, spec, value):
    L = SPEC_BYTE_LAYOUT
    if isinstance(spec, dict):
        buf += L["tag_obj_begin"]
        for k in sorted(spec, key=lambda s: [ord(c) for c in s]):  # Unicode code point 序
            kb = k.encode("utf-8")
            buf += struct.pack("<I", len(kb)) + kb
            buf = _enc_value(buf, spec[k], value[k])
        buf += L["tag_obj_end"]
        return buf
    if isinstance(spec, tuple):
        kind = spec[0]
        if kind == "arr_f64":
            buf += L["tag_arr_begin"]
            for x in value:
                buf += L["tag_f64"] + struct.pack("<d", x)
            buf += L["tag_arr_end"]
            return buf
        if kind == "enum":
            sb = value.encode("utf-8")
            return buf + L["tag_str"] + struct.pack("<I", len(sb)) + sb
        if kind == "const":
            return buf + L["tag_bool"] + (b"\x01" if value else b"\x00")
        raise ContractError(f"bad spec {spec!r}")
    if spec == "f64":
        return buf + L["tag_f64"] + struct.pack("<d", value)
    if spec == "u32":
        return buf + L["tag_u32"] + struct.pack("<I", value)
    if spec == "u64":
        return buf + L["tag_u64"] + struct.pack("<Q", value)
    if spec == "str_or_null":
        if value is None:
            return buf + L["tag_null"]
        sb = value.encode("utf-8")
        return buf + L["tag_str"] + struct.pack("<I", len(sb)) + sb
    raise ContractError(f"bad spec {spec!r}")


def canonical_preimage(contract):
    """参数 dict → canonical preimage bytes（SPEC 冻结布局，RXS-0384 L3）。"""
    return _enc_value(SPEC_BYTE_LAYOUT["version_prefix"], SCHEMA, contract)


def param_digest(contract):
    """SHA-256(canonical_preimage)。双端 digest 相等 ⟺ 双端解析一致（RFC-0026 §4.6）。"""
    return hashlib.sha256(canonical_preimage(contract)).hexdigest()


def section_param_digest(contract, section):
    """单节 canonical digest（provenance camera_params_digest / lighting_params_digest 用）：
    SHA-256(version_prefix ‖ enc(section, SCHEMA[section]))。"""
    if section not in SCHEMA:
        raise ContractError(f"unknown section {section!r}")
    pre = _enc_value(SPEC_BYTE_LAYOUT["version_prefix"], SCHEMA[section], contract[section])
    return "sha256:" + hashlib.sha256(pre).hexdigest()


# ---------------------------------------------------------------------------
# 值约定映射：契约世界系（右手系/+Y up/米）→ UE（厘米/左手系/Z-up/水平 FOV）
# 冻结公式（RFC-0026 §4.6 + v1.1 章 E errata；RXS-0384 L2 + v1.1 勘误行）：
# p_ue = (−z, x, y)·100；四元数共轭 q_ue = (w, z, −x, −y)（errata 修订式——M 为
# 反射 det=−1，共轭 R_ue = M·R(axis,θ)·M⁻¹ = R(M·axis, −θ)，向量部 −M·v、标量部
# 不变；勘误前实现 (w, −z, x, y) = R(M·axis, +θ) 镜像朝向，tests/
# test_g10_param_contract.py 共轭恒等式对拍 RED→GREEN 实证）；
# fov_h_ue = 2·atan(tan(fov_y/2)·aspect)；sun.direction 同 M。
# ---------------------------------------------------------------------------

def pos_contract_to_ue(p):
    x, y, z = p
    return (-z * 100.0, x * 100.0, y * 100.0)


def quat_contract_to_ue(q):
    w, x, y, z = q
    return (w, z, -x, -y)


def fov_y_to_ue_horizontal(fov_y_deg, aspect):
    return math.degrees(2.0 * math.atan(math.tan(math.radians(fov_y_deg) / 2.0) * aspect))


def dir_contract_to_ue(d):
    x, y, z = d
    return (-z, x, y)


def to_ue_scene_params(contract):
    """契约参数 → UE 侧应用参数（UE 单位/坐标/FOV 口径）。"""
    cam = contract["camera"]
    aspect = cam["resolution"]["w"] / cam["resolution"]["h"]
    return {
        "camera_location_cm": pos_contract_to_ue(cam["position"]),
        "camera_quat_ue": quat_contract_to_ue(cam["orientation_quat"]),
        "camera_fov_h_deg": fov_y_to_ue_horizontal(cam["fov_y_deg"], aspect),
        "resolution": dict(cam["resolution"]),
        "sun_direction_ue": dir_contract_to_ue(contract["lighting"]["sun"]["direction"]),
        "sun_intensity_lux": contract["lighting"]["sun"]["intensity_lux"],
        "sun_color_linear_rgb": list(contract["lighting"]["sun"]["color_linear_rgb"]),
        "sky_intensity": contract["lighting"]["sky"]["intensity"],
        "exposure_ev100": contract["lighting"]["exposure"]["ev100"],
        "time": dict(contract["time"]),
        "post": dict(contract["post"]),
    }


if __name__ == "__main__":
    import sys

    with open(sys.argv[1], "r", encoding="utf-8") as f:
        c = parse_contract(f.read())
    print("param_digest_ue5 =", param_digest(c))
