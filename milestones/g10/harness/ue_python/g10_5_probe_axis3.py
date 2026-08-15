#!/usr/bin/env python3
"""G10.5 harness — 轴转换实测探针 #3（Box 字段校准）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("AXIS3: " + str(m))


def main():
    for name in ("part_00_white_tex", "part_01_white", "part_03_red"):
        path = "/Game/G10Probe/cornell_box/StaticMeshes/{0}.{0}".format(name)
        mesh = unreal.EditorAssetLibrary.load_asset(path)
        if mesh is None:
            log("MISS: " + path)
            continue
        b = mesh.get_bounding_box()
        log(name + " box dir: " + str([m for m in dir(b) if not m.startswith("_")]))
        try:
            log(
                "%s min=(%.2f,%.2f,%.2f) max=(%.2f,%.2f,%.2f)"
                % (name, b.min.x, b.min.y, b.min.z, b.max.x, b.max.y, b.max.z)
            )
        except Exception as e:
            log("bounds FAIL: " + repr(e))
        break
    log("AXIS3 DONE")


main()
