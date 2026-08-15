#!/usr/bin/env python3
"""G10.5 harness — 导入网格 winding/法线/材质 实测定案探针。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("MESH: " + str(m))


def main():
    # 1) 材质 two-sided 状态
    for name in ("white", "red", "green", "white_tex"):
        p = "/Game/G10/cornell-box/cornell_box/Materials/{0}.{0}".format(name)
        mi = unreal.EditorAssetLibrary.load_asset(p)
        if mi is None:
            log("mat MISS " + name)
            continue
        parent = mi.get_editor_property("parent")
        ts = parent.get_editor_property("two_sided") if parent else None
        log("mat %s parent=%s two_sided=%s" % (name, parent.get_name() if parent else "?", ts))
        # 读 base color 参数当前值
        try:
            v = unreal.MaterialEditingLibrary.get_material_instance_vector_parameter_value(mi, "BaseColor")
            log("  %s BaseColor=(%.3f,%.3f,%.3f,%.3f)" % (name, v.r, v.g, v.b, v.a))
        except Exception as e:
            log("  %s BaseColor read fail: %r" % (name, e))

    # 2) 网格截面 winding/法线（EditorStaticMeshLibrary 面）
    lib = unreal.EditorStaticMeshLibrary
    mesh = unreal.EditorAssetLibrary.load_asset(
        "/Game/G10/cornell-box/cornell_box/StaticMeshes/part_02_white.part_02_white"
    )
    log("mesh lib methods: " + str([m for m in dir(lib) if "vertex" in m.lower() or "index" in m.lower() or "lod" in m.lower()][:20]))
    log("MESH DONE")


main()
