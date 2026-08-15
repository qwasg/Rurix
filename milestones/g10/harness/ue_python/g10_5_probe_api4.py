#!/usr/bin/env python3
"""G10.5 harness — UE 5.8.1 Python API 探针 #4（ImportAssetParameters + 真导入）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("PROBE4: " + str(m))


try:
    p = unreal.ImportAssetParameters()
    log("ImportAssetParameters props: " + str([m for m in dir(p) if not m.startswith("_")]))
except Exception as e:
    log("ImportAssetParameters FAIL: " + repr(e))

try:
    mgr = unreal.InterchangeManager.get_interchange_manager_scripted()
    sd = unreal.InterchangeManager.create_source_data(
        "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf"
    )
    params = unreal.ImportAssetParameters()
    result = mgr.import_asset("/Game/G10Probe", sd, params)
    log("import_asset result type: " + str(type(result)))
    if result:
        for obj in result:
            log("imported: " + str(obj.get_path_name()) + " class=" + str(obj.get_class().get_name()))
    else:
        log("import_asset returned None/empty")
except Exception as e:
    log("import_asset FAIL: " + repr(e))

log("PROBE4 DONE")
