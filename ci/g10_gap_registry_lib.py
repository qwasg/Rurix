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


def validate_registry(doc: Any, scene_set: list[str] | None = None) -> list[str]:
    """差距清单校验器（RXS-0391 L1~L9；返回错误列表，空 = 通过）。

    scene_set 参数给定时机核与清单 scene_set 精确全等（M140 门传 M133
    冻结清单行集）；None 时只做清单自洽核验。"""
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
    if doc.get("registry") != REGISTRY_NAME:
        errs.append(f"registry ≠ {REGISTRY_NAME!r}: {doc.get('registry')!r}")
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
    if failures:
        print(f"[g10_gap_registry_lib] SELFTEST FAIL ({failures})")
        return 1
    print(f"[g10_gap_registry_lib] SELFTEST PASS（枚举闭集 {len(UE5_MODULE_ENUM)} 值；10 RED + 1 GREEN）")
    return 0


if __name__ == "__main__":
    import sys

    if "--selftest" in sys.argv:
        sys.exit(selftest())
    print(__doc__)
    sys.exit(0)
