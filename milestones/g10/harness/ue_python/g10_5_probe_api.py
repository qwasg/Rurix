#!/usr/bin/env python3
"""G10.5 harness — UE 5.8.1 Python API 探针（一次性内省件，引擎内运行）。

用法（编辑器 cmd 模式）：
  UnrealEditor-Cmd.exe <proj> -ExecutePythonScript=<this> -unattended -log
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("PROBE: " + str(m))


try:
    mgr = unreal.InterchangeManager.get_interchange_manager_scripted()
    log("mgr ok: " + str(type(mgr)))
    log("mgr import methods: " + str([m for m in dir(mgr) if "import" in m.lower()]))
except Exception as e:
    log("mgr FAIL: " + repr(e))

try:
    sd = unreal.InterchangeManager.create_source_data(
        "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf"
    )
    log("source_data ok: " + str(sd))
except Exception as e:
    log("source_data FAIL: " + repr(e))

try:
    p = unreal.InterchangeImportParams()
    log("params props: " + str([m for m in dir(p) if not m.startswith("_")][:40]))
except Exception as e:
    log("params FAIL: " + repr(e))

try:
    log(
        "CameraComponent fov/exposure: "
        + str(
            [
                m
                for m in dir(unreal.CameraComponent)
                if "fov" in m.lower() or "exposure" in m.lower() or "aspect" in m.lower()
            ]
        )
    )
except Exception as e:
    log("cam FAIL: " + repr(e))

try:
    log(
        "LevelSequence binding methods: "
        + str(
            [
                m
                for m in dir(unreal.LevelSequence)
                if "possess" in m.lower()
                or "spawn" in m.lower()
                or "track" in m.lower()
                or "binding" in m.lower()
            ]
        )
    )
    log(
        "CameraCutSection: "
        + str(
            [
                m
                for m in dir(unreal.MovieSceneCameraCutSection)
                if "camera" in m.lower() or "binding" in m.lower()
            ]
        )
    )
except Exception as e:
    log("seq FAIL: " + repr(e))

try:
    log(
        "SkyLightComponent: "
        + str(
            [
                m
                for m in dir(unreal.SkyLightComponent)
                if "source" in m.lower() or "cubemap" in m.lower() or "intensity" in m.lower()
            ]
        )
    )
    log(
        "SkyLightSourceType: "
        + str([m for m in dir(unreal.SkyLightSourceType) if not m.startswith("_")])
    )
except Exception as e:
    log("sky FAIL: " + repr(e))

try:
    log(
        "PostProcessSettings exposure-related: "
        + str(
            [
                m
                for m in dir(unreal.PostProcessSettings)
                if "exposure" in m.lower()
                or "metering" in m.lower()
                or "iso" in m.lower()
                or "aperture" in m.lower()
                or "shutter" in m.lower()
            ]
        )
    )
    log(
        "AutoExposureMeteringMode: "
        + str([m for m in dir(unreal.AutoExposureMeteringMode) if not m.startswith("_")])
    )
except Exception as e:
    log("pp FAIL: " + repr(e))

try:
    cvs = unreal.MoviePipelineConsoleVariableSetting()
    log(
        "ConsoleVariableSetting props: "
        + str([m for m in dir(cvs) if "console" in m.lower() or "variable" in m.lower()])
    )
except Exception as e:
    log("cvs FAIL: " + repr(e))

log("PROBE DONE")
