#!/usr/bin/env python3
"""G10.5 harness — Bistro 嵌套节点旋转读回定案探针。Assisted-by: Kimi-K3（G10.5a 波续）"""
import unreal


def log(m):
    unreal.log("ROT: " + str(m))


def main():
    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level("/Game/Maps/G10_BistroInterior")
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        label = a.get_actor_label()
        if label in (
            "G10N_Bistro_Research_Interior_CashRegister_01A_Mesh_1134",
            "G10N_Bistro_Research_Interior_paris_building_01_interior_2326",
        ):
            t = a.get_actor_transform()
            q = t.rotation
            r = q.rotator()
            loc = t.translation
            log(
                label
                + " loc=(%.1f,%.1f,%.1f) quat=(%.6f,%.6f,%.6f,%.6f) rot=(p%.4f,y%.4f,r%.4f)"
                % (loc.x, loc.y, loc.z, float(q.x), float(q.y), float(q.z), float(q.w), float(r.pitch), float(r.yaw), float(r.roll))
            )
            fv = q.rotate_vector(unreal.Vector(1.0, 0.0, 0.0))
            rv = q.rotate_vector(unreal.Vector(0.0, 1.0, 0.0))
            uv = q.rotate_vector(unreal.Vector(0.0, 0.0, 1.0))
            log(
                "  fwd=(%.4f,%.4f,%.4f) right=(%.4f,%.4f,%.4f) up=(%.4f,%.4f,%.4f)"
                % (fv.x, fv.y, fv.z, rv.x, rv.y, rv.z, uv.x, uv.y, uv.z)
            )
    log("ROT DONE")


main()
