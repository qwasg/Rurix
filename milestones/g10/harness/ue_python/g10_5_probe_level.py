#!/usr/bin/env python3
"""G10.5 harness — 关卡构建结果核验探针（变换/相机/光照读回）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import os

import unreal


def log(m):
    unreal.log("LEVEL: " + str(m))


def main():
    map_path = os.environ.get("G10_5_MAP", "/Game/Maps/G10_CornellBox")
    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level(map_path)
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        label = a.get_actor_label()
        if not label.startswith("G10"):
            continue
        t = a.get_actor_transform()
        loc = t.translation
        rot = t.rotation.rotator()
        log(
            "%s loc=(%.1f,%.1f,%.1f) rot=(p%.2f,y%.2f,r%.2f)"
            % (label, loc.x, loc.y, loc.z, rot.pitch, rot.yaw, rot.roll)
        )
        if label == "G10_ContractCamera":
            cam = a.get_component_by_class(unreal.CameraComponent)
            view = cam.get_camera_view()
            log(
                "camera_view: loc=(%.1f,%.1f,%.1f) rot=(p%.3f,y%.3f,r%.3f) fov=%.4f aspect=%.4f"
                % (
                    view.location.x, view.location.y, view.location.z,
                    view.rotation.pitch, view.rotation.yaw, view.rotation.roll,
                    view.field_of_view, view.aspect_ratio,
                )
            )
    log("LEVEL DONE")


main()
