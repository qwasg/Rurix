# -*- coding: utf-8 -*-
# Assisted-by: Cursor Grok 4.6（G16.1 第一波）
"""G16 只读探针：读回现采图 G13_CornellBox 上 G13_QuadLight_0 的衰减/强度/单位/源面。

不保存关卡、不改任何资产。由 host 经 UnrealEditor-Cmd -ExecutePythonScript 调用。
环境变量：
  G16_PROBE_OUT=<json 路径>（必填）
"""
from __future__ import annotations

import json
import os

import unreal

OUT = os.environ.get("G16_PROBE_OUT", "")
MAP = "/Game/Maps/G13_CornellBox"
TARGET = "G13_QuadLight_0"


def _prop(comp, name):
    try:
        return comp.get_editor_property(name)
    except Exception as exc:  # UE 反射面缺字段时诚实登记
        return "UNREADABLE:%s" % exc


def main():
    if not OUT:
        raise RuntimeError("env G16_PROBE_OUT 必填")
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    level_subsys.load_level(MAP)
    rec = {
        "map": MAP,
        "target_label": TARGET,
        "found": False,
        "lights": [],
        "note": "readonly probe; no save",
    }
    for actor in unreal.EditorLevelLibrary.get_all_level_actors():
        label = actor.get_actor_label()
        rc = actor.get_component_by_class(unreal.RectLightComponent)
        if rc is None:
            continue
        item = {
            "label": label,
            "class": actor.get_class().get_name(),
            "attenuation_radius": _prop(rc, "attenuation_radius"),
            "intensity": _prop(rc, "intensity"),
            "intensity_units": str(_prop(rc, "intensity_units")),
            "source_width": _prop(rc, "source_width"),
            "source_height": _prop(rc, "source_height"),
            "mobility": str(_prop(rc, "mobility")),
            "cast_shadows": _prop(rc, "cast_shadows"),
            "barn_door_angle": _prop(rc, "barn_door_angle"),
            "barn_door_length": _prop(rc, "barn_door_length"),
            "affects_world": _prop(rc, "affects_world"),
            "hidden_in_game": _prop(actor, "hidden"),
            "location": [float(actor.get_actor_location().x),
                         float(actor.get_actor_location().y),
                         float(actor.get_actor_location().z)],
        }
        rec["lights"].append(item)
        if label == TARGET:
            rec["found"] = True
            rec["target"] = item
    parent = os.path.dirname(OUT)
    if parent:
        os.makedirs(parent, exist_ok=True)
    with open(OUT, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(rec, ensure_ascii=False, indent=2) + "\n")
    unreal.log("G16_PROBE " + json.dumps(rec, ensure_ascii=False))


main()
