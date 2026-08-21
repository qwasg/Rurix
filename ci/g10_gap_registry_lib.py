#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G10.5b 波）
"""G10.5b 差距清单共享判定层（spec/visual_comparison.md RXS-0391 单一事实源；
RFC-0026 §4.5 + §3.3；G10_ACCEPTANCE_MAP §1 M140 行判据逐字）。

消费面（RXS-0391 IR2 载体纪律：禁第二份手写）：
  - M139 落盘侧 ci/g10_ab_comparison_smoke.py（清单生成 + 落盘前自检）；
  - M140 门侧 ci/g10_gap_registry_smoke.py（登记核验 + RED 臂注入面）。

内容面（UE5 模块归属枚举闭集 / 字段闭集 / gap_id 冻结字节派生规则 /
校验器）全部单源在本文件；差距项内容（title/description/kind/模块归属/
建议 P 级/g11_anchor 与 measured_delta 数值面）由 M139 门按当次度量
evidence 实测装配（数字必须来自命令输出，禁记忆/推断）。
"""
from __future__ import annotations

import hashlib
from typing import Any

# ---------------------------------------------------------------------------
# UE5 模块归属枚举闭集（RXS-0391 L5；公共前缀 + 目录级 23 + 文件级 57 + Other）
# ---------------------------------------------------------------------------

MODULE_PREFIX = "Engine/Source/Runtime/Renderer/Private/"

DIR_MODULES: tuple[str, ...] = (
    "CompositionLighting", "Froxel", "HairStrands", "HeterogeneousVolumes",
    "InstanceCulling", "Lumen", "MaterialCache", "MegaLights", "Nanite",
    "OIT", "PostProcess", "RayTracing", "Renderer", "SceneCulling",
    "Shadows", "Skinning", "SparseVolumeTexture", "StateStream",
    "StochasticLighting", "Substrate", "VariableRateShading",
    "VirtualShadowMaps", "VT",
)

FILE_MODULES: tuple[str, ...] = (
    "PathTracing.cpp", "PathTracingSpatialTemporalDenoising.cpp",
    "SceneCaptureRendering.cpp", "SkyAtmosphereRendering.cpp",
    "SkyPassRendering.cpp", "VolumetricCloudRendering.cpp",
    "VolumetricFog.cpp", "SingleLayerWaterRendering.cpp",
    "WaterInfoTextureRendering.cpp", "SubsurfaceTiles.cpp",
    "DBufferTextures.cpp", "TranslucentRendering.cpp",
    "TranslucentLighting.cpp", "FrontLayerTranslucency.cpp",
    "ShadowRendering.cpp", "ShadowSetup.cpp", "ShadowDepthRendering.cpp",
    "CapsuleShadowRendering.cpp", "DistanceFieldAmbientOcclusion.cpp",
    "DistanceFieldShadowing.cpp", "DistanceFieldScreenGridLighting.cpp",
    "DistanceFieldLightingPost.cpp", "GlobalDistanceField.cpp",
    "ReflectionEnvironment.cpp", "ReflectionEnvironmentCapture.cpp",
    "ReflectionEnvironmentDiffuseIrradiance.cpp",
    "ReflectionEnvironmentRealTimeCapture.cpp",
    "PlanarReflectionRendering.cpp", "ScreenSpaceReflectionTiles.cpp",
    "ScreenSpaceRayTracing.cpp", "ScreenSpaceDenoise.cpp",
    "FogRendering.cpp", "LocalFogVolumeRendering.cpp",
    "LightRendering.cpp", "IndirectLightRendering.cpp",
    "LightShaftRendering.cpp", "BasePassRendering.cpp", "DepthRendering.cpp",
    "VelocityRendering.cpp", "AnisotropyRendering.cpp",
    "DecalRenderingShared.cpp", "GPUScene.cpp", "HZB.cpp",
    "SceneVisibility.cpp", "DeferredShadingRenderer.cpp", "Renderer.cpp",
    "HaltonUtilities.cpp", "BlueNoise.cpp", "HdrCustomResolveShaders.cpp",
    "GPUBenchmark.cpp", "ShadingEnergyConservation.cpp",
    "IESTextureManager.cpp", "RectLightTextureManager.cpp",
    "LightFunctionRendering.cpp", "VolumeLighting.cpp",
    "HeightfieldLighting.cpp", "DistortionRendering.cpp",
)

OTHER_MODULE = MODULE_PREFIX + "Other"

UE5_MODULE_ENUM: frozenset[str] = frozenset(
    [MODULE_PREFIX + d for d in DIR_MODULES]
    + [MODULE_PREFIX + f for f in FILE_MODULES]
    + [OTHER_MODULE]
)

KINDS: tuple[str, ...] = ("quality_gap", "caliber_diff")
PRIORITIES: tuple[str, ...] = ("P0", "P1", "P2")
DOMAINS: tuple[str, ...] = ("scene-linear-hdr", "display-referred-ldr")

TOP_KEYS: frozenset[str] = frozenset(
    {"schema_version", "registry", "generated_by", "scene_set", "items",
     "scene_summary", "not_ready_scenes"}
)
ITEM_KEYS: frozenset[str] = frozenset(
    {"gap_id", "scene_id", "camera_id", "domain", "kind",
     "ue5_module_primary", "ue5_module_secondary", "measured_delta",
     "suggested_priority", "g11_anchor", "title", "description", "attachments"}
)
ITEM_OPTIONAL_KEYS: frozenset[str] = frozenset({"attribution_note"})
DELTA_KEYS: frozenset[str] = frozenset(
    {"metric", "a_value", "b_value", "delta", "evidence_digest"}
)
DELTA_OPTIONAL_KEYS: frozenset[str] = frozenset({"region_ref"})
SUMMARY_KEYS: frozenset[str] = frozenset({"scene_id", "gap_count", "no_gap_explicit"})

REGISTRY_NAME = "g10_gap_registry"
SCHEMA_VERSION = 1


def derive_gap_id(scene_id: str, camera_id: str, ue5_module_primary: str,
                  kind: str, title: str) -> str:
    """gap_id 派生（RXS-0391 L3 冻结字节规则）：utf8 五节 0x00 分隔拼接
    sha256 全小写 hex 前 16 字符。"""
    buf = b"\x00".join(
        s.encode("utf-8")
        for s in (scene_id, camera_id, ue5_module_primary, kind, title)
    )
    return hashlib.sha256(buf).hexdigest()[:16]


def _is_num(v: Any) -> bool:
    return isinstance(v, (int, float)) and not isinstance(v, bool)


def _is_sha256(v: Any) -> bool:
    return (
        isinstance(v, str)
        and v.startswith("sha256:")
        and len(v) == len("sha256:") + 64
        and all(c in "0123456789abcdef" for c in v[len("sha256:"):])
    )


def validate_registry(doc: Any, scene_set: list[str] | None = None,
                      registry_name: str | None = None) -> list[str]:
    """差距清单校验器（RXS-0391 L1~L9；返回错误列表，空 = 通过）。

    scene_set 参数给定时机核与清单 scene_set 精确全等（M140 门传 M133
    冻结清单行集）；None 时只做清单自洽核验。
    registry_name 为 G13.4 起加性可选参数（G13 波登记表自带命名，
    缺省 None = REGISTRY_NAME 既有行为 0-byte，G10 消费面不变）。"""
    want_name = registry_name if registry_name is not None else REGISTRY_NAME
    errs: list[str] = []
    if not isinstance(doc, dict):
        return ["清单顶层非 object"]
    extra = set(doc) - TOP_KEYS
    missing = TOP_KEYS - set(doc)
    if extra or missing:
        errs.append(f"顶层闭集漂移: extra={sorted(extra)} missing={sorted(missing)}")
        return errs
    if doc.get("schema_version") != SCHEMA_VERSION:
        errs.append(f"schema_version ≠ {SCHEMA_VERSION}: {doc.get('schema_version')!r}")
    if doc.get("registry") != want_name:
        errs.append(f"registry ≠ {want_name!r}: {doc.get('registry')!r}")
    if not isinstance(doc.get("generated_by"), str) or not doc["generated_by"].strip():
        errs.append("generated_by 空/非字符串")
    scenes = doc.get("scene_set")
    if not isinstance(scenes, list) or not scenes or any(
        not isinstance(s, str) or not s.strip() for s in scenes
    ):
        errs.append("scene_set 空/含非字符串")
        scenes = []
    if scene_set is not None and scenes != list(scene_set):
        errs.append(f"scene_set 与给定行集不全等: {scenes} vs {list(scene_set)}")
    items = doc.get("items")
    if not isinstance(items, list):
        errs.append("items 非数组")
        items = []
    summary = doc.get("scene_summary")
    if not isinstance(summary, list):
        errs.append("scene_summary 非数组")
        summary = []
    nrs = doc.get("not_ready_scenes")
    if not isinstance(nrs, list) or any(not isinstance(s, str) for s in (nrs or [])):
        errs.append("not_ready_scenes 非字符串数组（键必须存在，可空集）")
        nrs = []
    for s in nrs:
        if scenes and s not in scenes:
            errs.append(f"not_ready_scenes 含 scene_set 外场景: {s!r}")

    seen_ids: set[str] = set()
    per_scene_count: dict[str, int] = {s: 0 for s in scenes}
    other_count = 0
    for idx, it in enumerate(items):
        tag = f"items[{idx}]"
        if not isinstance(it, dict):
            errs.append(f"{tag} 非 object")
            continue
        allowed = ITEM_KEYS | ITEM_OPTIONAL_KEYS
        iextra = set(it) - allowed
        imissing = ITEM_KEYS - set(it)
        if iextra or imissing:
            errs.append(f"{tag} 字段闭集漂移: extra={sorted(iextra)} missing={sorted(imissing)}")
            continue
        scene_id = it["scene_id"]
        if scenes and scene_id not in per_scene_count:
            errs.append(f"{tag}.scene_id 不在 scene_set: {scene_id!r}")
        else:
            per_scene_count[scene_id] = per_scene_count.get(scene_id, 0) + 1
        if not isinstance(it["camera_id"], str) or not it["camera_id"].strip():
            errs.append(f"{tag}.camera_id 空")
        if it["domain"] not in DOMAINS:
            errs.append(f"{tag}.domain 闭集外: {it['domain']!r}")
        if it["kind"] not in KINDS:
            errs.append(f"{tag}.kind 闭集外: {it['kind']!r}")
        prim = it["ue5_module_primary"]
        if prim not in UE5_MODULE_ENUM:
            errs.append(f"{tag}.ue5_module_primary 枚举闭集外: {prim!r}")
        is_other = prim == OTHER_MODULE
        if is_other:
            other_count += 1
            note = it.get("attribution_note")
            if not isinstance(note, str) or not note.strip():
                errs.append(f"{tag} Other 终值缺 attribution_note（L2/L5）")
        elif "attribution_note" in it:
            errs.append(f"{tag} 非 Other 项携带 attribution_note（L2 当且仅当）")
        sec = it["ue5_module_secondary"]
        if not isinstance(sec, list) or any(s not in UE5_MODULE_ENUM for s in sec):
            errs.append(f"{tag}.ue5_module_secondary 含闭集外/非数组")
        if it["suggested_priority"] not in PRIORITIES:
            errs.append(f"{tag}.suggested_priority 闭集外: {it['suggested_priority']!r}")
        if not isinstance(it["g11_anchor"], str) or not it["g11_anchor"].strip():
            errs.append(f"{tag}.g11_anchor 空（缺承接锚行即 RED）")
        if not isinstance(it["title"], str) or not it["title"].strip():
            errs.append(f"{tag}.title 空")
        if not isinstance(it["description"], str) or not it["description"].strip():
            errs.append(f"{tag}.description 空")
        att = it["attachments"]
        if not isinstance(att, list) or any(not _is_sha256(a) for a in att):
            errs.append(f"{tag}.attachments 含非 sha256 digest 引用")
        md = it["measured_delta"]
        if not isinstance(md, list) or not md:
            errs.append(f"{tag}.measured_delta 空（非 measured 叙述充差距即 RED）")
            md = []
        for j, d in enumerate(md):
            tj = f"{tag}.measured_delta[{j}]"
            if not isinstance(d, dict):
                errs.append(f"{tj} 非 object")
                continue
            dallowed = DELTA_KEYS | DELTA_OPTIONAL_KEYS
            dextra = set(d) - dallowed
            dmissing = DELTA_KEYS - set(d)
            if dextra or dmissing:
                errs.append(f"{tj} 字段闭集漂移: extra={sorted(dextra)} missing={sorted(dmissing)}")
                continue
            if not isinstance(d["metric"], str) or not d["metric"].strip():
                errs.append(f"{tj}.metric 空")
            if not (_is_num(d["a_value"]) and _is_num(d["b_value"]) and _is_num(d["delta"])):
                errs.append(f"{tj} a_value/b_value/delta 非数值")
            elif float(d["b_value"]) - float(d["a_value"]) != float(d["delta"]):
                errs.append(
                    f"{tj} delta ≠ b−a（f64 精确重算不等）: "
                    f"{d['b_value']}−{d['a_value']}={float(d['b_value']) - float(d['a_value'])!r} vs {d['delta']!r}"
                )
            if not _is_sha256(d["evidence_digest"]):
                errs.append(f"{tj}.evidence_digest 非 sha256 形态: {d['evidence_digest']!r}")
            if "region_ref" in d and not isinstance(d["region_ref"], str):
                errs.append(f"{tj}.region_ref 非字符串")
        want = derive_gap_id(
            str(it["scene_id"]), str(it["camera_id"]), str(it["ue5_module_primary"]),
            str(it["kind"]), str(it["title"]),
        )
        if it["gap_id"] != want:
            errs.append(f"{tag}.gap_id 重算不等: {it['gap_id']!r} vs {want!r}")
        if it["gap_id"] in seen_ids:
            errs.append(f"{tag}.gap_id 重复: {it['gap_id']}")
        seen_ids.add(it["gap_id"])

    sum_scenes: list[str] = []
    for k, row in enumerate(summary):
        ts = f"scene_summary[{k}]"
        if not isinstance(row, dict) or set(row) != SUMMARY_KEYS:
            errs.append(f"{ts} 字段闭集漂移: {sorted(set(row) ^ SUMMARY_KEYS) if isinstance(row, dict) else '非 object'}")
            continue
        sum_scenes.append(row["scene_id"])
        gc = row["gap_count"]
        actual = per_scene_count.get(row["scene_id"], 0)
        if not isinstance(gc, int) or isinstance(gc, bool) or gc != actual:
            errs.append(f"{ts}.gap_count={gc!r} ≠ 实计 {actual}")
        nge = row["no_gap_explicit"]
        if not isinstance(nge, bool) or nge != (actual == 0):
            errs.append(f"{ts}.no_gap_explicit={nge!r} 与 gap_count 矛盾（L8）")
    if scenes and sorted(sum_scenes) != sorted(scenes):
        errs.append(
            f"scene_summary 行集与 scene_set 不全等（缺场景行即 RED）: "
            f"{sorted(sum_scenes)} vs {sorted(scenes)}"
        )
    return errs


# ---------------------------------------------------------------------------
# G14 M-a 加性面：登记表结构化对账（UE 厂商随机运行间方差带吸收，G13 §8.7 承接锚）
# ---------------------------------------------------------------------------

# provenance 三值闭集：classify(metric, field, old_value) 返回值语义——
#   "ue"         = UE 侧测量值（厂商随机运行间方差面，方差带吸收）；
#   "rurix"      = Rurix 侧测量值（固定 seed 位级确定性面，位级一致硬门）；
#   "structural" = 结构常量（0.0/1.0 等参照面常量，位级一致硬门）。
PROVENANCE_UE = "ue"
PROVENANCE_RURIX = "rurix"
PROVENANCE_STRUCTURAL = "structural"

# 行级身份字段闭集（measured_delta 数值三面之外的逐字节面）。
_ITEM_IDENTITY_KEYS = (
    "gap_id", "scene_id", "camera_id", "domain", "kind",
    "ue5_module_primary", "ue5_module_secondary", "suggested_priority",
    "g11_anchor", "title", "description", "attachments",
)


def reconcile_registry_structured(old_doc: Any, new_doc: Any, ue_band_rel: float,
                                  classify: Any, ue_band_rel_map: Any = None) -> list[str]:
    """在树冻结登记表 vs 当次重算登记表的结构化对账（返回错误列表，空 = 通过）。

    G13 §8.7 承接锚字面兑现——三面：
    ① 身份面逐字节：顶层 registry/generated_by/scene_set/scene_summary/
       not_ready_scenes/schema_version + 行集序与行级身份字段 + measured_delta
       的 metric 名与 evidence_digest（Rurix 侧 digest 位级稳定面）；
    ② Rurix 侧与结构常量数值位级一致（classify 判 "rurix"/"structural" 的
       a_value/b_value 位，== 比对）；
    ③ UE 侧数值程序产方差带内（classify 判 "ue" 的位：|new−old| ≤
       ue_band_rel × max(|old|, 1e-30)；ue_band_rel 由门内 UE 探针格双跑
       方差底 ×headroom 程序产，本函数只消费不产阈——P-09 禁手写；band==0.0
       时 UE 侧面退化为位级一致〔最严〕）。
       **G14.5a 后事件加性（G14-N1 重判条件命中兑现——只追加程序重判对账语义面）**：
       可选 `ue_band_rel_map`（{f"{gap_id}|{metric}|{field}" → band_rel}）逐位带
       覆盖标量带——跨会话厂商随机方差包络面（同会话探针带对跨会话漂移欠覆盖的
       实证事件：indirect_ssim@bistro 跨会话 ±4.4% vs 同会话探针带 0.15%），
       样本级联登记面 = g14_ue_variance_samples.json（只追加），带 = 历史样本
       极差率 × headroom 与同会话探针带取 max（双程序产面取严，不拟合当次差值——
       RXS-0392/P-09 维持）。
    双面 delta == b−a（f64 精确）关系核验维持（构造不变式）。
    """
    errs: list[str] = []
    if not isinstance(old_doc, dict) or not isinstance(new_doc, dict):
        return ["结构化对账输入非 object"]
    for key in ("schema_version", "registry", "generated_by", "scene_set",
                "scene_summary", "not_ready_scenes"):
        if old_doc.get(key) != new_doc.get(key):
            errs.append(f"顶层身份面漂移 {key}: {old_doc.get(key)!r} vs {new_doc.get(key)!r}")
    old_items = old_doc.get("items") or []
    new_items = new_doc.get("items") or []
    if len(old_items) != len(new_items):
        errs.append(f"行集行数漂移: {len(old_items)} vs {len(new_items)}")
        return errs
    if not _is_num(ue_band_rel) or ue_band_rel < 0.0:
        errs.append(f"ue_band_rel 非法（非数值/负值）: {ue_band_rel!r}")
        return errs
    for idx, (oi, ni) in enumerate(zip(old_items, new_items)):
        tag = f"items[{idx}]"
        for key in _ITEM_IDENTITY_KEYS:
            if oi.get(key) != ni.get(key):
                errs.append(f"{tag}.{key} 身份面漂移: {oi.get(key)!r} vs {ni.get(key)!r}")
        omd = oi.get("measured_delta") or []
        nmd = ni.get("measured_delta") or []
        if len(omd) != len(nmd):
            errs.append(f"{tag}.measured_delta 行数漂移: {len(omd)} vs {len(nmd)}")
            continue
        for j, (od, nd) in enumerate(zip(omd, nmd)):
            tj = f"{tag}.measured_delta[{j}]"
            if od.get("metric") != nd.get("metric"):
                errs.append(f"{tj}.metric 身份面漂移: {od.get('metric')!r} vs {nd.get('metric')!r}")
                continue
            if od.get("evidence_digest") != nd.get("evidence_digest"):
                errs.append(f"{tj}.evidence_digest 位级漂移（Rurix 侧 digest 面）")
            metric = str(od.get("metric"))
            for field in ("a_value", "b_value"):
                ov = od.get(field)
                nv = nd.get(field)
                if not (_is_num(ov) and _is_num(nv)):
                    errs.append(f"{tj}.{field} 非数值: {ov!r} vs {nv!r}")
                    continue
                side = classify(metric, field, float(ov))
                if side == PROVENANCE_UE:
                    band_rel_eff = ue_band_rel
                    if ue_band_rel_map is not None:
                        _mk = f"{oi.get('gap_id')}|{metric}|{field}"
                        _mv = (ue_band_rel_map.get(_mk) if hasattr(ue_band_rel_map, "get") else None)
                        if _mv is not None and _is_num(_mv):
                            band_rel_eff = max(float(_mv), float(ue_band_rel))
                    band = band_rel_eff * max(abs(float(ov)), 1e-30)
                    if abs(float(nv) - float(ov)) > band:
                        errs.append(
                            f"{tj}.{field} UE 侧超方差带（{metric}）: "
                            f"|{nv!r}−{ov!r}|={abs(float(nv) - float(ov))!r} > {band!r}"
                            f"（band_rel={band_rel_eff!r} 程序产）"
                        )
                elif side in (PROVENANCE_RURIX, PROVENANCE_STRUCTURAL):
                    if float(nv) != float(ov):
                        errs.append(
                            f"{tj}.{field} {side} 面位级漂移（{metric}）: {ov!r} vs {nv!r}"
                        )
                else:
                    errs.append(f"{tj}.{field} provenance 闭集外: {side!r}")
            # delta == b−a 构造不变式双面核验（f64 精确，沿 validate_registry 口径）
            for lbl, dd in (("在树", od), ("当次", nd)):
                if _is_num(dd.get("a_value")) and _is_num(dd.get("b_value")) and _is_num(dd.get("delta")):
                    if float(dd["b_value"]) - float(dd["a_value"]) != float(dd["delta"]):
                        errs.append(f"{tj}（{lbl}）delta ≠ b−a 构造不变式破坏")
    return errs


# ---------------------------------------------------------------------------
# G14.5a 后事件加性面：UE 侧跨会话方差样本级联登记（G14-N1 重判条件命中兑现）
# ---------------------------------------------------------------------------
# 语义：同会话探针带（max 两两相对差 ×2.0）只覆盖同窗口运行间方差；跨会话
# （跨日/跨 gate 运行窗口）厂商随机漂移实证可达 ±4.4%（2026-08-21 indirect_ssim@
# bistro-interior 三样本 {0.0065669, 0.0065253, 0.0068552}，evidence 034829Z/
# 071403Z 在档）——带面须从**历史样本级联**派生：逐 UE 位（gap_id|metric|field）
# 登记每次门运行的 fresh 测量值（只追加），带 = 历史样本极差率 × headroom 与
# 当次同会话探针带取 max（双程序产面取严；不带入当次差值——不拟合，RXS-0392/
# P-09 维持）。样本面永不回写 G13/G12 冻结登记表本体（0-byte 纪律维持）。

def ue_samples_load(path: Any) -> dict:
    """读取跨会话样本登记面（缺文件 → 空集骨架）。"""
    import json as _json
    from pathlib import Path as _P
    p = _P(str(path))
    if not p.is_file():
        return {"schema": "rurix.g14.ue_variance_samples.v1", "entries": []}
    return _json.loads(p.read_text(encoding="utf-8"))


def ue_samples_append(path: Any, rows: list, *, source: str, timestamp: str) -> None:
    """追加 UE 位测量样本（rows = [{gap_id, metric, field, value}]；幂等键 =
    (gap_id|metric|field, source, timestamp)——同源同戳重放不重复登记）。"""
    import json as _json
    from pathlib import Path as _P
    p = _P(str(path))
    doc = ue_samples_load(p)
    entries = doc.setdefault("entries", [])
    for row in rows:
        key = f"{row.get('gap_id')}|{row.get('metric')}|{row.get('field')}"
        hit = None
        for e in entries:
            if e.get("key") == key:
                hit = e
                break
        if hit is None:
            hit = {"key": key, "gap_id": row.get("gap_id"), "metric": row.get("metric"),
                   "field": row.get("field"), "values": []}
            entries.append(hit)
        dup = any(v.get("source") == source and v.get("timestamp") == timestamp
                  for v in hit["values"])
        if not dup:
            hit["values"].append({
                "value": float(row["value"]),
                "source": source,
                "timestamp": timestamp,
            })
    p.parent.mkdir(parents=True, exist_ok=True)
    p.write_text(_json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")


def ue_cross_session_band(path: Any, gap_id: str, metric: str, field: str,
                          old_value: float, *, headroom: float = 2.0) -> float:
    """逐位跨会话带：样本级联（冻结在树值为首样本 + 历次 fresh 登记值）极差率
    × headroom——样本 < 2 点时返回 0.0（调用方与同会话探针带取 max 兜住）。"""
    doc = ue_samples_load(path)
    key = f"{gap_id}|{metric}|{field}"
    series = [float(old_value)]
    for e in doc.get("entries") or []:
        if e.get("key") == key:
            series += [float(v["value"]) for v in e.get("values") or [] if _is_num(v.get("value"))]
            break
    if len(series) < 2:
        return 0.0
    lo, hi = min(series), max(series)
    return (hi - lo) / max(abs(lo), 1e-30) * headroom


def selftest() -> int:
    """红绿两臂自检（不依赖树上文件）。"""
    good_delta = {"metric": "m", "a_value": 1.0, "b_value": 2.5,
                  "delta": 1.5, "evidence_digest": "sha256:" + "0" * 64}
    good_item = {
        "gap_id": "", "scene_id": "s", "camera_id": "c",
        "domain": "scene-linear-hdr", "kind": "quality_gap",
        "ue5_module_primary": MODULE_PREFIX + "Lumen",
        "ue5_module_secondary": [], "measured_delta": [good_delta],
        "suggested_priority": "P1", "g11_anchor": "G11 承接锚",
        "title": "t", "description": "d", "attachments": [],
    }
    good_item["gap_id"] = derive_gap_id("s", "c", good_item["ue5_module_primary"],
                                        "quality_gap", "t")
    good = {
        "schema_version": 1, "registry": REGISTRY_NAME,
        "generated_by": "selftest", "scene_set": ["s"],
        "items": [good_item],
        "scene_summary": [{"scene_id": "s", "gap_count": 1, "no_gap_explicit": False}],
        "not_ready_scenes": [],
    }
    failures = 0

    def red(name: str, mutate) -> None:
        nonlocal failures
        import copy

        doc = copy.deepcopy(good)
        mutate(doc)
        errs = validate_registry(doc, scene_set=["s"])
        if errs:
            print(f"  RED ok   — {name}（{errs[0][:80]}）")
        else:
            print(f"  RED MISS — {name}：负样本过检")
            failures += 1

    red("缺归属", lambda d: d["items"][0].pop("ue5_module_primary"))
    red("闭集外模块", lambda d: d["items"][0].update(ue5_module_primary="Evil/Module.cpp"))
    red("Other 无 note", lambda d: d["items"][0].update(
        ue5_module_primary=OTHER_MODULE,
        gap_id=derive_gap_id("s", "c", OTHER_MODULE, "quality_gap", "t")))
    red("缺承接锚", lambda d: d["items"][0].update(g11_anchor=""))
    red("measured_delta 空", lambda d: d["items"][0].update(measured_delta=[]))
    red("delta ≠ b−a", lambda d: d["items"][0]["measured_delta"][0].update(delta=9.0))
    red("场景缺行", lambda d: d.update(scene_summary=[]))
    red("no_gap 矛盾", lambda d: d["scene_summary"][0].update(no_gap_explicit=True))
    red("gap_id 漂移", lambda d: d["items"][0].update(gap_id="0" * 16))
    red("顶层闭集外字段", lambda d: d.update(evil=1))

    errs = validate_registry(good, scene_set=["s"])
    if errs:
        print(f"  GREEN MISS — 合形清单被误拒: {errs}")
        failures += 1
    else:
        print("  GREEN ok — 合形清单过检")

    # ── G14 M-a 结构化对账面（2 GREEN + 5 RED）──
    import copy as _copy

    def _cls(metric: str, field: str, value: float) -> str:
        return PROVENANCE_UE if field == "a_value" else PROVENANCE_RURIX

    base = _copy.deepcopy(good)
    red2_cases = []

    def red2(name: str, mutate, band: float = 0.01) -> None:
        nonlocal failures
        doc = _copy.deepcopy(base)
        mutate(doc)
        errs2 = reconcile_registry_structured(base, doc, band, _cls)
        if errs2:
            print(f"  RED ok   — {name}（{errs2[0][:80]}）")
        else:
            print(f"  RED MISS — {name}：负样本过检")
            failures += 1

    red2("UE 侧超带", lambda d: d["items"][0]["measured_delta"][0].update(
        a_value=1.0 * 1.05, delta=2.5 * 1.0 - 1.0 * 1.05), band=0.01)
    red2("Rurix 侧 1 ulp 级漂移", lambda d: d["items"][0]["measured_delta"][0].update(
        b_value=2.5 + 1e-12, delta=1.5 + 1e-12), band=0.5)
    red2("身份面 gap_id 漂移", lambda d: d["items"][0].update(gap_id="f" * 16))
    red2("metric 名漂移", lambda d: d["items"][0]["measured_delta"][0].update(metric="m2"))
    red2("行数漂移", lambda d: d["items"].append(_copy.deepcopy(d["items"][0])))
    green_small = _copy.deepcopy(base)
    green_small["items"][0]["measured_delta"][0].update(
        a_value=1.0 * 1.005, delta=2.5 - 1.0 * 1.005)
    errs3 = reconcile_registry_structured(base, green_small, 0.01, _cls)
    if errs3:
        print(f"  GREEN MISS — UE 侧带内小方差被误拒: {errs3}")
        failures += 1
    else:
        print("  GREEN ok — UE 侧带内小方差吸收")
    green_zero_band = _copy.deepcopy(base)
    errs4 = reconcile_registry_structured(base, green_zero_band, 0.0, _cls)
    if errs4:
        print(f"  GREEN MISS — 恒等面零带被误拒: {errs4}")
        failures += 1
    else:
        print("  GREEN ok — band=0 恒等位级面过检")

    # ── G14.5a 后事件加性面（逐位带 map + 跨会话样本级联；2 RED + 2 GREEN）──
    import tempfile as _tmp
    gid = base["items"][0]["gap_id"]
    map_key = f"{gid}|m|a_value"
    # RED③：逐位带面大漂移（×1.5）超 map 带（0.20）必检出
    big_map = _copy.deepcopy(base)
    big_map["items"][0]["measured_delta"][0].update(a_value=1.5, delta=2.5 - 1.5)
    errs5 = reconcile_registry_structured(base, big_map, 0.01, _cls,
                                          ue_band_rel_map={map_key: 0.20})
    if not errs5:
        print("  RED MISS — 逐位带面大漂移未检出")
        failures += 1
    else:
        print("  RED ok   — 逐位带面大漂移（×1.5 > map 带 0.20）检出")
    # GREEN③：逐位带内（×1.10 ≤ map 带 0.20）吸收（标量带 0.01 本拒——map 生效面）
    in_map = _copy.deepcopy(base)
    in_map["items"][0]["measured_delta"][0].update(a_value=1.10, delta=2.5 - 1.10)
    errs6 = reconcile_registry_structured(base, in_map, 0.01, _cls,
                                          ue_band_rel_map={map_key: 0.20})
    if errs6:
        print(f"  GREEN MISS — 逐位带内吸收被误拒: {errs6}")
        failures += 1
    else:
        print("  GREEN ok — 逐位带内（×1.10 ≤ map 带 0.20）吸收（map 覆盖标量带生效）")
    # 样本级联面：append 幂等 + 跨会话带派生（series {1.00, 1.04} → 极差率 4% × 2.0 = 0.08）
    with _tmp.TemporaryDirectory(prefix="gaplib_g14_5a_") as td:
        sp = f"{td}/samples.json"
        ue_samples_append(sp, [{"gap_id": gid, "metric": "m", "field": "a_value", "value": 1.04}],
                          source="selftest", timestamp="20260821T000000Z")
        ue_samples_append(sp, [{"gap_id": gid, "metric": "m", "field": "a_value", "value": 1.04}],
                          source="selftest", timestamp="20260821T000000Z")  # 重放幂等
        band_cs = ue_cross_session_band(sp, gid, "m", "a_value", 1.00)
        n_vals = 0
        for e in ue_samples_load(sp).get("entries") or []:
            if e.get("key") == map_key:
                n_vals = len(e.get("values") or [])
        if n_vals != 1:
            print(f"  RED MISS — 样本级联幂等面破坏（values={n_vals} ≠ 1）")
            failures += 1
        elif abs(band_cs - 0.08) > 1e-12:
            print(f"  RED MISS — 跨会话带派生漂移（{band_cs} ≠ 0.08）")
            failures += 1
        else:
            print("  RED ok   — 样本级联幂等 + 跨会话带派生（series {1.00,1.04} → 0.08）")
    # GREEN④：单点样本带 0.0 退化面（由调用方与探针带取 max 兜住字面）
    with _tmp.TemporaryDirectory(prefix="gaplib_g14_5b_") as td:
        band_one = ue_cross_session_band(f"{td}/none.json", gid, "m", "a_value", 1.00)
    if band_one != 0.0:
        print(f"  GREEN MISS — 单点样本带非零退化: {band_one}")
        failures += 1
    else:
        print("  GREEN ok — 样本 < 2 点带 0.0 退化（调用方 max 探针带兜住）")

    if failures:
        print(f"[g10_gap_registry_lib] SELFTEST FAIL ({failures})")
        return 1
    print(f"[g10_gap_registry_lib] SELFTEST PASS（枚举闭集 {len(UE5_MODULE_ENUM)} 值；10 RED + 1 GREEN + 结构化对账 5 RED + 2 GREEN + 跨会话带面 2 RED + 2 GREEN）")
    return 0


if __name__ == "__main__":
    import sys

    if "--selftest" in sys.argv:
        sys.exit(selftest())
    print(__doc__)
    sys.exit(0)
