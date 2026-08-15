#!/usr/bin/env python3
"""G10.5 harness — Interchange 导入 + 保存 + 轴转换实测一体探针。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("AXIS2: " + str(m))


def main():
    mgr = unreal.InterchangeManager.get_interchange_manager_scripted()
    sd = unreal.InterchangeManager.create_source_data(
        "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf"
    )
    params = unreal.ImportAssetParameters()
    params.is_automated = True
    params.replace_existing = True
    result = mgr.import_asset("/Game/G10Probe", sd, params)
    if not result:
        log("import empty")
        return
    # 保存导入产物
    saved = unreal.EditorAssetLibrary.save_directory("/Game/G10Probe", only_if_is_dirty=False)
    log("save_directory -> " + str(saved))
    for name in ("part_00_white_tex", "part_01_white", "part_03_red"):
        path = "/Game/G10Probe/cornell_box/StaticMeshes/{0}.{0}".format(name)
        mesh = unreal.EditorAssetLibrary.load_asset(path)
        if mesh is None:
            log("MISS: " + path)
            continue
        b = mesh.get_bounding_box()
        log(
            "%s origin=(%.2f,%.2f,%.2f) extent=(%.2f,%.2f,%.2f)"
            % (name, b.origin.x, b.origin.y, b.origin.z, b.box_extent.x, b.box_extent.y, b.box_extent.z)
        )
    log("AXIS2 DONE")


main()
