#!/usr/bin/env python3
"""G10.5 harness — 相机视线 line trace 探针（定位平帧成因）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import os

import unreal


def log(m):
    unreal.log("TRACE: " + str(m))


def main():
    map_path = os.environ.get("G10_5_MAP", "/Game/Maps/G10_CornellBox")
    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level(map_path)
    world = unreal.get_editor_subsystem(unreal.UnrealEditorSubsystem).get_editor_world()
    cam = unreal.Vector(80000.0, 27800.0, 27300.0)
    targets = {
        "forward_-X_2000m": unreal.Vector(80000.0 - 200000.0, 27800.0, 27300.0),
        "box_center": unreal.Vector(-27940.0, 27640.0, 27440.0),
        "down_-Z": unreal.Vector(80000.0, 27800.0, 0.0),
    }
    for name, end in targets.items():
        hit = unreal.SystemLibrary.line_trace_single(
            world, cam, end,
            unreal.TraceTypeQuery.TRACE_TYPE_QUERY1,
            False, [], unreal.DrawDebugTrace.NONE, True,
        )
        if hit and hit.hit_actor:
            log(
                "%s -> HIT %s at (%.0f,%.0f,%.0f)"
                % (name, hit.hit_actor.get_actor_label(), hit.impact_point.x, hit.impact_point.y, hit.impact_point.z)
            )
        else:
            log("%s -> NO HIT" % name)
    log("TRACE DONE")


main()
