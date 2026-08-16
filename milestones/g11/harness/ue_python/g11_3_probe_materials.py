#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.3 波）
"""G11.3 U2 诊断探针：bistro 材质实例纹理绑定深挖（UE 内嵌 CPython）。

输出（G10_5_PROBE_OUT 文件面，单行 JSON）：
- 抽样 MIC 的 parent 材质链 / texture_parameter_values 计数与非空计数
- 对应 Texture2D 资产尺寸读回（纹理资产真实性）
- MIC 父材质图的纹理表达式节点计数（材质图面）
"""
import json
import os

import unreal

CONTENT_ROOT = "/Game/G10"
PROBE_OUT = os.environ.get("G10_5_PROBE_OUT", "")


def log(m):
    unreal.log("G11_3_PROBE: " + str(m))


def main():
    mel = unreal.MaterialEditingLibrary
    mat_dir = CONTENT_ROOT + "/bistro-interior/BistroInterior/Materials"
    tex_dir = CONTENT_ROOT + "/bistro-interior/BistroInterior/Textures"
    out = {"mic_samples": [], "texture_samples": []}

    mics = [p for p in unreal.EditorAssetLibrary.list_assets(mat_dir, recursive=False)]
    log("MIC count=%d" % len(mics))
    for path in sorted(mics)[:6]:
        mic = unreal.EditorAssetLibrary.load_asset(path)
        rec = {"asset": path.split("/")[-1], "class": mic.get_class().get_name()}
        try:
            parent = mel.get_material_instance_parent(mic)
            rec["parent"] = parent.get_path_name() if parent else None
        except Exception as e:
            rec["parent_error"] = str(e)
        try:
            tpv = mic.get_editor_property("texture_parameter_values")
            rec["tpv_len"] = len(tpv)
            rec["tpv"] = [
                {"name": str(e.parameter_info.name), "nonnull": e.parameter_value is not None}
                for e in list(tpv)[:8]
            ]
        except Exception as e:
            rec["tpv_error"] = str(e)
        # 静态接口面（5.8 MaterialEditingLibrary 备选通道）
        try:
            names = mel.get_texture_parameter_names(mic) if hasattr(mel, "get_texture_parameter_names") else None
            rec["param_names_iface"] = [str(n) for n in names] if names else None
        except Exception as e:
            rec["param_names_error"] = str(e)
        # 标量参数面（metallic/roughness 消费口径实证——U2/R1 双端材质口径对账）
        try:
            spv = mic.get_editor_property("scalar_parameter_values")
            rec["scalar_params"] = [
                {"name": str(e.parameter_info.name), "value": float(e.parameter_value)}
                for e in list(spv)[:12]
            ]
        except Exception as e:
            rec["scalar_params_error"] = str(e)
        out["mic_samples"].append(rec)
        log(json.dumps(rec, ensure_ascii=False))

    texs = [p for p in unreal.EditorAssetLibrary.list_assets(tex_dir, recursive=False)]
    out["texture_count"] = len(texs)
    for path in sorted(texs)[:6]:
        t = unreal.EditorAssetLibrary.load_asset(path)
        rec = {"asset": path.split("/")[-1], "class": t.get_class().get_name()}
        try:
            rec["size"] = [int(t.blueprint_get_size_x()), int(t.blueprint_get_size_y())]
        except Exception as e:
            rec["size_error"] = str(e)
        out["texture_samples"].append(rec)
        log(json.dumps(rec, ensure_ascii=False))

    # 父材质图面：首个 MIC 的 parent 链追到根，数纹理表达式节点
    if out["mic_samples"] and out["mic_samples"][0].get("parent"):
        try:
            root_parent = unreal.load_asset(out["mic_samples"][0]["parent"])
            chain = [root_parent.get_path_name()]
            exprs = unreal.MaterialEditingLibrary.get_material_expressions(root_parent) if hasattr(unreal.MaterialEditingLibrary, "get_material_expressions") else []
            tex_exprs = [e for e in exprs if "Texture" in e.get_class().get_name()]
            out["parent_graph"] = {
                "chain_head": chain[0],
                "expressions": len(exprs),
                "texture_expressions": len(tex_exprs),
            }
        except Exception as e:
            out["parent_graph_error"] = str(e)

    if PROBE_OUT:
        with open(PROBE_OUT, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(out, ensure_ascii=False) + "\n")
        log("probe -> " + PROBE_OUT)


main()
