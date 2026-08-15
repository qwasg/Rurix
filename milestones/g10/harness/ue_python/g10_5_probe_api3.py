#!/usr/bin/env python3
"""G10.5 harness — UE 5.8.1 Python API 探针 #3（Interchange 导入参数面）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("PROBE3: " + str(m))


for cls_name in ("InterchangeImportSettings", "InterchangeContentImportSettings"):
    try:
        cls = getattr(unreal, cls_name)
        obj = cls()
        props = [m for m in dir(obj) if not m.startswith("_")]
        log(cls_name + " props: " + str(props))
    except Exception as e:
        log(cls_name + " FAIL: " + repr(e))

# import_asset 最小调用实测（cornell glTF → /Game/G10Probe/）
try:
    mgr = unreal.InterchangeManager.get_interchange_manager_scripted()
    sd = unreal.InterchangeManager.create_source_data(
        "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf"
    )
    params = unreal.InterchangeImportSettings()
    result = mgr.import_asset("/Game/G10Probe", sd, params)
    log("import_asset result type: " + str(type(result)))
    if result:
        for obj in result:
            log("imported: " + str(obj.get_path_name()) + " class=" + str(obj.get_class().get_name()))
    else:
        log("import_asset returned None/empty")
except Exception as e:
    log("import_asset FAIL: " + repr(e))

log("PROBE3 DONE")
