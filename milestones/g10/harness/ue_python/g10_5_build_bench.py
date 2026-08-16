#!/usr/bin/env python3
"""G10.5b harness — UE 侧 benchmark MRQ Phase A 资产（编辑器 cmd 模式运行）。

M141 帧率基线面（14 §5 采样协议 / BENCH_PROTOCOL §3；spike 实证命令形态
g10_ue5_harness_spike.md）：与 A/B 出图同一关卡/相机/光照（g10_5_build_scenes.py
建设面幂等产物复用，本脚本零导入零关卡改动），仅新建 benchmark 用
LevelSequence（playback 0..N-1，N = G10_5_BENCH_FRAMES，缺省 160 = 10
warmup 弃计 + 150 timed = 3 trial 块 × 50）+ PrimaryConfig（EXR NONE +
DeferredPass + disable_tone_curve + 契约分辨率 + engine_warm_up_count=64 +
固定帧率 1/fixed_dt_s + 后处理全关 console vars），输出目录
K:/rurix-ext/g10-frames/g10_5/bench/ue/<scene>/。

逐帧墙钟来源（5.8 源树实证）：MRQ 逐帧 RenderTimeFrameStatistics
（MoviePipelineRendering.cpp FindOrAdd StartTime/EndTime）经
ResolveFilenameFormatArguments 写 EXR 头 unreal/frameRenderStartTimeUTC /
frameRenderEndTimeUTC / frameRenderDuration（MoviePipeline.cpp:1614-1623 →
MoviePipelineEXROutput.cpp AddFileMetadata → Imf::Header）——门侧
ci/g10_perf_baseline_smoke.py 解析 EXR 头字符串属性取逐帧时长，不解析日志。

用法（经 milestones/g10/harness/g10_5_ue_run.py 执行）：
  env G10_5_SCENE=cornell-box|bistro-interior G10_5_CONTRACT=<params.json>
      [G10_5_BENCH_FRAMES=160]

坐标/单位一律经 g10_param_contract 冻结公式换算，禁脚本内手写第二份。
Assisted-by: Kimi-K3（G10.5b 波）
"""
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import g10_param_contract as contract  # noqa: E402

import unreal  # noqa: E402

MAPS_ROOT = "/Game/Maps"
CINE_ROOT = "/Game/Cinematics"

SCENE_MAP = {
    "cornell-box": "G10_CornellBox",
    "bistro-interior": "G10_BistroInterior",
}


def log(m):
    unreal.log("G10_5_BENCH: " + str(m))


def main():
    scene_id = os.environ.get("G10_5_SCENE", "")
    contract_path = os.environ.get("G10_5_CONTRACT", "")
    n_frames = int(os.environ.get("G10_5_BENCH_FRAMES", "160"))
    if scene_id not in SCENE_MAP or not contract_path:
        raise RuntimeError("env G10_5_SCENE/G10_5_CONTRACT 必填")
    if n_frames < 60:
        raise RuntimeError("G10_5_BENCH_FRAMES 须 ≥60（warmup 10 + timed 50 下限）")

    with open(contract_path, "r", encoding="utf-8") as f:
        c = contract.parse_contract(f.read())
    ue_params = contract.to_ue_scene_params(c)
    log("契约解析 OK（UE 侧解析器）: scene=%s bench_frames=%d" % (scene_id, n_frames))

    # 复用 A/B 关卡（幂等产物；找不到即 fail-closed——先跑 build_scenes）。
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    map_path = MAPS_ROOT + "/" + SCENE_MAP[scene_id]
    if not unreal.EditorAssetLibrary.does_asset_exist(map_path):
        raise RuntimeError("关卡不存在（先跑 g10_5_build_scenes.py）: " + map_path)
    level_subsys.load_level(map_path)
    cam_actor = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        if a.get_actor_label() == "G10_ContractCamera":
            cam_actor = a
            break
    if cam_actor is None:
        raise RuntimeError("关卡缺 G10_ContractCamera（先跑 g10_5_build_scenes.py）")

    tools = unreal.AssetToolsHelpers.get_asset_tools()
    tag = scene_id.replace("-", "_")

    seq_path = CINE_ROOT + "/G10_%sBenchSeq" % tag
    if unreal.EditorAssetLibrary.does_asset_exist(seq_path):
        unreal.EditorAssetLibrary.delete_asset(seq_path)
    seq = tools.create_asset("G10_%sBenchSeq" % tag, CINE_ROOT, unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
    seq.set_playback_start(0)
    seq.set_playback_end(n_frames)
    binding = seq.add_possessable(cam_actor)
    cut_track = seq.add_track(unreal.MovieSceneCameraCutTrack)
    section = cut_track.add_section()
    section.set_start_frame(0)
    section.set_end_frame(n_frames)
    bid = seq.get_binding_id(binding)
    section.set_camera_binding_id(bid)
    unreal.EditorAssetLibrary.save_asset(seq_path)
    log("bench sequence ready: %s frames=%d" % (seq_path, n_frames))

    cfg_path = CINE_ROOT + "/G10_%sBenchConfig" % tag
    if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
        unreal.EditorAssetLibrary.delete_asset(cfg_path)
    cfg = tools.create_asset(
        "G10_%sBenchConfig" % tag, CINE_ROOT,
        unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory(),
    )
    exr = cfg.find_or_add_setting_by_class(unreal.MoviePipelineImageSequenceOutput_EXR)
    exr.compression = unreal.EXRCompressionFormat.NONE
    color = cfg.find_or_add_setting_by_class(unreal.MoviePipelineColorSetting)
    color.set_editor_property("disable_tone_curve", True)
    cfg.find_or_add_setting_by_class(unreal.MoviePipelineDeferredPassBase)
    out = cfg.find_or_add_setting_by_class(unreal.MoviePipelineOutputSetting)
    res = ue_params["resolution"]
    out.output_resolution = unreal.IntPoint(res["w"], res["h"])
    out.output_directory = unreal.DirectoryPath("K:/rurix-ext/g10-frames/g10_5/bench/ue/" + scene_id)
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
    unreal.EditorAssetLibrary.save_asset(cfg_path)
    log("bench config saved: %s res=%dx%d warmup=%d" % (cfg_path, res["w"], res["h"], int(t["warmup_frames"])))
    log("BENCH BUILD DONE scene=%s" % scene_id)


main()
