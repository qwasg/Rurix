#!/usr/bin/env python3
"""G10.5 harness — actor bounds/网格指配核验探针。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import os

import unreal


def log(m):
    unreal.log("BOUNDS: " + str(m))


def main():
    map_path = os.environ.get("G10_5_MAP", "/Game/Maps/G10_CornellBox")
    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level(map_path)
    n_mesh_set = 0
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        label = a.get_actor_label()
        if not label.startswith("G10N_"):
            continue
        smc = a.get_component_by_class(unreal.StaticMeshComponent)
        mesh = smc.static_mesh if smc else None
        origin, extent = a.get_actor_bounds(False)
        log(
            "%s mesh=%s origin=(%.0f,%.0f,%.0f) extent=(%.0f,%.0f,%.0f)"
            % (
                label,
                mesh.get_name() if mesh else "NONE",
                origin.x, origin.y, origin.z, extent.x, extent.y, extent.z,
            )
        )
        if mesh:
            n_mesh_set += 1
    log("mesh assigned: %d" % n_mesh_set)
    log("BOUNDS DONE")


main()
