#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G14.2 修订与测量波）
"""G14.2 UE 侧建设步（编辑器内 Python，经 UnrealEditor-Cmd -ExecutePythonScript 调用）：
benchmark 臂契约相机 auto-activation 对齐。

G13 双图 G13_ContractCamera 经 MRQ LevelSequence possess（G13.4 面 0-byte 不动）；
-game benchmark 臂的视口 = 相机 auto_player_activation=Player0 自动激活面（UE
CameraComponent 语义）。本步幂等设置该属性并落盘，使 benchmark 臂视口 == 契约
相机位（M-d 双端 A/B 同视点硬约束）。

环境变量：G14_2_SCENE（cornell-box|bistro-interior）；探针 JSON 落
G14_2_PROBE_OUT（读回核验：auto_player_activation == Player0 字面）。
"""
import json
import os

import unreal

SCENE_MAP = {
    "cornell-box": "/Game/Maps/G13_CornellBox",
    "bistro-interior": "/Game/Maps/G13_BistroInterior",
}


def log(msg):
    unreal.log(f"[g14_2_camera_align] {msg}")


def main():
    scene = os.environ["G14_2_SCENE"]
    probe_out = os.environ["G14_2_PROBE_OUT"]
    map_path = SCENE_MAP[scene]
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    level_subsys.load_level(map_path)
    found = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        if a.get_actor_label() == "G13_ContractCamera":
            found = a
            break
    if found is None:
        raise RuntimeError(f"G13_ContractCamera 未找到: {map_path}")
    # AutoActivateForPlayer 挂在 CameraActor（非 CameraComponent——UE5.8 UPROPERTY 面实证）。
    found.set_editor_property("auto_activate_for_player", unreal.AutoReceiveInput.PLAYER0)
    got = found.get_editor_property("auto_activate_for_player")
    ok = got == unreal.AutoReceiveInput.PLAYER0
    level_subsys.save_current_level()
    log(f"{scene}: camera={found.get_name()} auto_player_activation={got} ok={ok} saved")
    with open(probe_out, "w", encoding="utf-8") as f:
        json.dump({
            "scene": scene,
            "map": map_path,
            "camera": found.get_name(),
            "auto_player_activation": str(got),
            "aligned": bool(ok),
        }, f, ensure_ascii=False, indent=1)
    if not ok:
        raise RuntimeError("auto_player_activation 读回非 Player0")


main()
