#!/usr/bin/env python3
"""G10.5 harness — Interchange glTF 导入轴转换实测（bounds 反推 C 映射）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("AXIS: " + str(m))


def main():
    # part_00_white_tex = floor：glTF 空间 x[0,552.8] y=0 z[0,558.8]
    # part_01_white = ceiling：y=548.8 全张
    # part_03_red = left wall：x=0, y[0,548.8], z[0,558.8]
    for path in (
        "/Game/G10Probe/cornell_box/StaticMeshes/part_00_white_tex.part_00_white_tex",
        "/Game/G10Probe/cornell_box/StaticMeshes/part_01_white.part_01_white",
        "/Game/G10Probe/cornell_box/StaticMeshes/part_03_red.part_03_red",
    ):
        mesh = unreal.EditorAssetLibrary.load_asset(path)
        if mesh is None:
            log("MISS: " + path)
            continue
        b = mesh.get_bounding_box()
        log(
            path.split("/")[-1]
            + " origin=(%.2f,%.2f,%.2f) extent=(%.2f,%.2f,%.2f)"
            % (b.origin.x, b.origin.y, b.origin.z, b.box_extent.x, b.box_extent.y, b.box_extent.z)
        )
    log("AXIS DONE")


main()
