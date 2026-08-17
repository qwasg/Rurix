#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G11.5b 波）
"""G11.5b UE 侧诊断场景构建/探针（编辑器 cmd 模式运行，g10_5_ue_run.py 驱动）。

诊断面（G11.5 A/B 帧库与契约场景资产 0-byte——诊断变体独立地图/配置资产，输出
落 G11.5b 诊断帧区；契约参数 0-byte）：

- 模式 probe（G11_5B_DIAG_MODE=probe）：读回 G10_BistroInterior 关卡现状——
  SkyLight 组件属性（source_type/intensity/cubemap/cast_shadows/lower_hemisphere
  /mobility）+ 渲染管线 cvar 实测值（Lumen GI/反射/距离场/SSR 开关）+ bistro 70
  MIC 父材质 blend_mode/two_sided 读回（UE 侧玻璃透明口径实测取证，禁假设）——
  JSON 落 G11_5B_PROBE_OUT；
- 模式 sky0：复制地图 G11_DiagBistroSky0 + SkyLight intensity=0（其余逐字同契约
  建设面）+ 新 seq/config（输出 K:/rurix-ext/g11-frames/g11_5b/ue_diag/
  bistro-interior-sky0）——SkyLight 总贡献分离臂；
- 模式 nospec：复用契约地图与既有 seq，仅新 MRQ config（追加 cvar
  r.Lumen.Reflections.Allow=0 + r.SSR.Quality=0，输出 …/bistro-interior-nospec）
  ——镜面（Lumen 反射/SSR）路径贡献分离臂。

用法：
  UnrealEditor-Cmd.exe <proj> -ExecutePythonScript=g11_5b_diag_scenes.py
  环境变量：G11_5B_DIAG_MODE=probe|sky0|nospec；G10_5_CONTRACT=<contract.json>；
            G11_5B_PROBE_OUT=<probe.json>（probe 模式）。
"""
import json
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
_G10_UE_PYTHON = os.path.normpath(os.path.join(_HERE, "..", "..", "..", "g10", "harness", "ue_python"))
for _p in (_HERE, _G10_UE_PYTHON):
    if _p not in sys.path:
        sys.path.insert(0, _p)

import g10_param_contract as contract  # noqa: E402

import unreal  # noqa: E402

CONTENT_ROOT = "/Game/G10"
MAPS_ROOT = "/Game/Maps"
CINE_ROOT = "/Game/Cinematics"
BASE_MAP = MAPS_ROOT + "/G10_BistroInterior"
BASE_SEQ = CINE_ROOT + "/G10_bistro_interiorSeq"
DIAG_OUT_ROOT = "K:/rurix-ext/g11-frames/g11_5b/ue_diag"


def log(m):
    unreal.log("G11_5B_DIAG: " + str(m))


def _get(obj, prop):
    try:
        return obj.get_editor_property(prop)
    except Exception as e:  # noqa: BLE001
        return "<unreadable:%s>" % type(e).__name__


def _cvar_int(name):
    try:
        return unreal.SystemLibrary.get_console_variable_int_value(name)
    except Exception:  # noqa: BLE001
        try:
            return unreal.SystemLibrary.get_console_variable_float_value(name)
        except Exception as e:  # noqa: BLE001
            return "<unreadable:%s>" % type(e).__name__


# ---------------------------------------------------------------------------
# probe：SkyLight/管线 cvar/材质 blend mode 实测读回
# ---------------------------------------------------------------------------

def run_probe():
    out_path = os.environ.get("G11_5B_PROBE_OUT", "")
    if not out_path:
        raise RuntimeError("probe 模式缺 G11_5B_PROBE_OUT")
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    level_subsys.load_level(BASE_MAP)
    probe = {"scene_id": "bistro-interior", "map": BASE_MAP}

    sky = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        if a.get_actor_label() == "G10_Sky":
            sky = a
            break
    if sky is None:
        raise RuntimeError("G10_Sky actor 缺失（探针面断裂）")
    sl = sky.get_component_by_class(unreal.SkyLightComponent)
    probe["skylight"] = {
        "source_type": str(_get(sl, "source_type")),
        "intensity": _get(sl, "intensity"),
        "cubemap": str(_get(sl, "cubemap")),
        "cast_shadows": _get(sl, "cast_shadows"),
        "lower_hemisphere_is_black": _get(sl, "lower_hemisphere_is_black"),
        "mobility": str(_get(sl, "mobility")),
        "real_time_capture": _get(sl, "real_time_capture"),
        "affect_indirect_lighting": _get(sl, "affect_indirect_lighting"),
        "b_affect_indirect_lighting": _get(sl, "b_affect_indirect_lighting"),
    }
    probe["cv_toggles"] = {
        n: _cvar_int(n)
        for n in (
            "r.DynamicGlobalIlluminationMethod",
            "r.ReflectionMethod",
            "r.GenerateMeshDistanceFields",
            "r.Lumen.DiffuseIndirect.Allow",
            "r.Lumen.Reflections.Allow",
            "r.Lumen.ScreenProbeGather.Allow",
            "r.SSR.Quality",
            "r.SkyLight.RealTimeReflectionCapture",
            "r.DiffuseIndirectDenoiser",
        )
    }

    mel = unreal.MaterialEditingLibrary
    mats = []
    mesh_dir = CONTENT_ROOT + "/bistro-interior"
    for sub in unreal.EditorAssetLibrary.list_assets(mesh_dir, recursive=True):
        cls_path = unreal.EditorAssetLibrary.find_asset_data(sub).get_class().get_path_name()
        if "MaterialInstanceConstant" not in cls_path:
            continue
        mic = unreal.EditorAssetLibrary.load_asset(sub)
        rec = {"mic": sub.split("/")[-1]}
        try:
            parent = mel.get_material_instance_parent(mic)
        except Exception:  # noqa: BLE001
            parent = None
        if parent is None:
            rec["parent"] = None
        else:
            rec["parent"] = parent.get_name()
            rec["parent_class"] = parent.get_class().get_name()
            rec["blend_mode"] = str(_get(parent, "blend_mode"))
            rec["two_sided"] = _get(parent, "two_sided")
            rec["shading_models"] = str(_get(parent, "shading_models"))
        mats.append(rec)
    probe["materials"] = mats
    probe["materials_count"] = len(mats)
    blend_hist = {}
    for rec in mats:
        blend_hist[rec.get("blend_mode", "<none>")] = blend_hist.get(rec.get("blend_mode", "<none>"), 0) + 1
    probe["blend_mode_histogram"] = blend_hist

    with open(out_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(probe, ensure_ascii=False, indent=1) + "\n")
    log("probe 落盘: %s（materials=%d blend_hist=%s）" % (out_path, len(mats), json.dumps(blend_hist)))


# ---------------------------------------------------------------------------
# MRQ 诊断配置（与 g10_5_build_scenes.build_mrq_assets 同字面设置 + 诊断增量）
# ---------------------------------------------------------------------------

def build_diag_config(cfg_name, contract_obj, out_dir, extra_cvars):
    ue_params = contract.to_ue_scene_params(contract_obj)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    cfg_path = CINE_ROOT + "/" + cfg_name
    if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
        unreal.EditorAssetLibrary.delete_asset(cfg_path)
    cfg = tools.create_asset(
        cfg_name, CINE_ROOT,
        unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory(),
    )
    exr = cfg.find_or_add_setting_by_class(unreal.MoviePipelineImageSequenceOutput_EXR)
    exr.compression = unreal.EXRCompressionFormat.NONE
    # HDR 臂捕获点 = tonemap 前 scene-linear（RXS-0386 L1；disable_tone_curve 同字面）。
    color = cfg.find_or_add_setting_by_class(unreal.MoviePipelineColorSetting)
    color.set_editor_property("disable_tone_curve", True)
    cfg.find_or_add_setting_by_class(unreal.MoviePipelineDeferredPassBase)
    out = cfg.find_or_add_setting_by_class(unreal.MoviePipelineOutputSetting)
    res = ue_params["resolution"]
    out.output_resolution = unreal.IntPoint(res["w"], res["h"])
    out.output_directory = unreal.DirectoryPath(out_dir)
    out.use_custom_frame_rate = True
    t = ue_params["time"]
    out.output_frame_rate = unreal.FrameRate(int(round(1.0 / t["fixed_dt_s"])), 1)
    aa = cfg.find_or_add_setting_by_class(unreal.MoviePipelineAntiAliasingSetting)
    aa.engine_warm_up_count = int(t["warmup_frames"])
    cvs = cfg.find_or_add_setting_by_class(unreal.MoviePipelineConsoleVariableSetting)
    cvs.add_or_update_console_variable("r.MotionBlurQuality", 0.0)
    cvs.add_or_update_console_variable("r.BloomQuality", 0.0)
    cvs.add_or_update_console_variable("r.DepthOfFieldQuality", 0.0)
    cvs.add_or_update_console_variable("r.EyeAdaptation.PreExposureOverride", 0.0)
    for name, val in extra_cvars:
        cvs.add_or_update_console_variable(name, float(val))
        log("诊断 cvar: %s = %s" % (name, val))
    unreal.EditorAssetLibrary.save_asset(cfg_path)
    log("config saved: " + cfg_path + " -> " + out_dir)
    return cfg_path


def run_sky0(contract_obj):
    # 诊断地图 = host 侧 .umap 磁盘字节复制（G10_BistroInterior 0-byte；进程内
    # duplicate_asset + load_level 实测触发 World Memory Leaks fatal——5.8.1 实证，
    # 故走磁盘复制 + 全新进程装载面）。
    diag_map = MAPS_ROOT + "/G11_DiagBistroSky0"
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    level_subsys.load_level(diag_map)
    sky = None
    cam = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        label = a.get_actor_label()
        if label == "G10_Sky":
            sky = a
        elif label == "G10_ContractCamera":
            cam = a
    if sky is None or cam is None:
        raise RuntimeError("诊断地图 actor 缺失（sky=%s cam=%s）" % (sky is not None, cam is not None))
    sl = sky.get_component_by_class(unreal.SkyLightComponent)
    sl.set_intensity(0.0)
    log("sky0: SkyLight intensity 读回 = %s" % str(_get(sl, "intensity")))
    level_subsys.save_current_level()

    tools = unreal.AssetToolsHelpers.get_asset_tools()
    seq_name = "G11_DiagBistroSky0Seq"
    seq_path = CINE_ROOT + "/" + seq_name
    if unreal.EditorAssetLibrary.does_asset_exist(seq_path):
        unreal.EditorAssetLibrary.delete_asset(seq_path)
    seq = tools.create_asset(seq_name, CINE_ROOT, unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
    seq.set_playback_start(0)
    seq.set_playback_end(4)
    binding = seq.add_possessable(cam)
    cut_track = seq.add_track(unreal.MovieSceneCameraCutTrack)
    section = cut_track.add_section()
    section.set_start_frame(0)
    section.set_end_frame(4)
    bid = seq.get_binding_id(binding)
    section.set_camera_binding_id(bid)
    unreal.EditorAssetLibrary.save_asset(seq_path)
    cfg = build_diag_config(
        "G11_DiagBistroSky0Config", contract_obj,
        DIAG_OUT_ROOT + "/bistro-interior-sky0", [],
    )
    log("sky0 BUILD DONE map=%s seq=%s cfg=%s" % (diag_map, seq_path, cfg))


def run_nospec(contract_obj):
    cfg = build_diag_config(
        "G11_DiagBistroNoSpecConfig", contract_obj,
        DIAG_OUT_ROOT + "/bistro-interior-nospec",
        [("r.Lumen.Reflections.Allow", 0.0), ("r.SSR.Quality", 0.0)],
    )
    log("nospec BUILD DONE map=%s seq=%s cfg=%s（复用契约地图与 seq）" % (BASE_MAP, BASE_SEQ, cfg))


def main():
    mode = os.environ.get("G11_5B_DIAG_MODE", "")
    contract_path = os.environ.get("G10_5_CONTRACT", "")
    if not contract_path:
        raise RuntimeError("env G10_5_CONTRACT=<params.json> 必填")
    with open(contract_path, "r", encoding="utf-8") as f:
        c = contract.parse_contract(f.read())
    if mode == "probe":
        run_probe()
    elif mode == "sky0":
        run_sky0(c)
    elif mode == "nospec":
        run_nospec(c)
    else:
        raise RuntimeError("G11_5B_DIAG_MODE 闭集外: %r（probe|sky0|nospec）" % mode)


main()
