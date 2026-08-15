#!/usr/bin/env python3
"""G10.5 harness — UE 5.8.1 Python API 探针 #2（一次性内省件）。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("PROBE2: " + str(m))


# 相机 FOV 面
try:
    all_cam = [m for m in dir(unreal.CameraComponent) if not m.startswith("_")]
    log("CameraComponent ALL: " + str(all_cam))
except Exception as e:
    log("cam FAIL: " + repr(e))
try:
    cine = [m for m in dir(unreal.CineCameraComponent) if "focal" in m.lower() or "filmback" in m.lower() or "fov" in m.lower()]
    log("CineCameraComponent focal/filmback/fov: " + str(cine))
except Exception as e:
    log("cine FAIL: " + repr(e))

# 曝光 method 枚举
for name in ("AutoExposureMethod", "AutoExposureMethodUI", "EAutoExposureMethod"):
    try:
        en = getattr(unreal, name)
        log(name + ": " + str([m for m in dir(en) if not m.startswith("_")]))
    except AttributeError:
        log(name + ": MISSING")

# import_asset / import_scene 签名
try:
    log("import_asset doc: " + str(unreal.InterchangeManager.import_asset.__doc__))
except Exception as e:
    log("import_asset doc FAIL: " + repr(e))
try:
    log("import_scene doc: " + str(unreal.InterchangeManager.import_scene.__doc__))
except Exception as e:
    log("import_scene doc FAIL: " + repr(e))

# InterchangeImportParams 实际类名
try:
    cands = [m for m in dir(unreal) if "Interchange" in m and ("Param" in m or "Import" in m)]
    log("Interchange*Param/Import classes: " + str(cands))
except Exception as e:
    log("cands FAIL: " + repr(e))

# 关卡/actor spawn 面
try:
    eas = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    log("EditorActorSubsystem: " + str([m for m in dir(eas) if "spawn" in m.lower() or "destroy" in m.lower()]))
except Exception as e:
    log("eas FAIL: " + repr(e))

# EditorLevelLibrary 新关卡面
try:
    ell = unreal.EditorLevelLibrary
    log("EditorLevelLibrary level methods: " + str([m for m in dir(ell) if "level" in m.lower() or "new" in m.lower() or "save" in m.lower() or "load" in m.lower()]))
except Exception as e:
    log("ell FAIL: " + repr(e))

# LevelSequenceEditorSubsystem / camera cut 面
try:
    log("MovieSceneCameraCutTrack: " + str([m for m in dir(unreal.MovieSceneCameraCutTrack) if "section" in m.lower() or "add" in m.lower()]))
    log("MovieSceneBindingExtensions: " + str([m for m in dir(unreal.MovieSceneBindingExtensions) if not m.startswith("_")][:30]))
except Exception as e:
    log("cut FAIL: " + repr(e))

# PostProcessVolume
try:
    log("PostProcessVolume: " + str([m for m in dir(unreal.PostProcessVolume) if "settings" in m.lower() or "bound" in m.lower() or "infinite" in m.lower()]))
except Exception as e:
    log("ppv FAIL: " + repr(e))

# texture cube 创建面（sky 常量 cubemap）
try:
    cands = [m for m in dir(unreal) if "TextureCube" in m]
    log("TextureCube classes: " + str(cands))
except Exception as e:
    log("texcube FAIL: " + repr(e))

log("PROBE2 DONE")
