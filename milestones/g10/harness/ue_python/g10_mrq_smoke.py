#!/usr/bin/env python3
"""G10.2 harness — MRQ 冒烟 Phase A：生成 LevelSequence + PrimaryConfig 资产（编辑器模式一次性准备）。

Phase B（官方命令行模式 1，spike 实证形态）：
  UnrealEditor-Cmd.exe <proj> <map> -game -LevelSequence="/Game/Cinematics/G10_SmokeSeq"
    -MoviePipelineConfig="/Game/Cinematics/G10_SmokeConfig" -windowed -resx=1920 -resy=1080
    -log -notexturestreaming -Unattended -FixedSeed

实测校准记录（UE 5.8.1-56057345 Launcher 版）：
  - EXR 输出类 = unreal.MoviePipelineImageSequenceOutput_EXR（EXRCompressionFormat.NONE 设置成功）；
  - MoviePipelineQueueSubsystem 为 EditorSubsystem（get_editor_subsystem）；
  - 5.8.1 无 MoviePipelineQueueFactoryNew → 队列资产不可脚本建；改走 PrimaryConfig 路线
    （MoviePipelinePrimaryConfigFactory 存在，实测在类清单）；
  - -ExecutePythonScript 仅编辑器模式可用（-game 下报错拒绝，5.8.1 实证）；
  - 编辑器 cmd 模式脚本结束后进程自动退出 → 异步 InProcessExecutor 无法在脚本态完成渲染，
    必须走 Phase B 命令行渲染模式。
地位：DRAFT 冒烟件；正式臂模板 g10_mrq_render.py 合入同批校准。
Assisted-by: Kimi-K3（G10.2 波）
"""
import unreal


def log(m):
    unreal.log("G10MRQ-A: " + str(m))


def main():
    tools = unreal.AssetToolsHelpers.get_asset_tools()

    seq_path = "/Game/Cinematics/G10_SmokeSeq"
    if unreal.EditorAssetLibrary.does_asset_exist(seq_path):
        seq = unreal.EditorAssetLibrary.load_asset(seq_path)
    else:
        seq = tools.create_asset("G10_SmokeSeq", "/Game/Cinematics",
                                 unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
    seq.set_playback_start(0)
    seq.set_playback_end(4)
    unreal.EditorAssetLibrary.save_asset(seq_path)
    log("sequence ready: " + str(seq))

    cfg_path = "/Game/Cinematics/G10_SmokeConfig"
    if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
        unreal.EditorAssetLibrary.delete_asset(cfg_path)
    cfg = tools.create_asset("G10_SmokeConfig", "/Game/Cinematics",
                             unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory())
    log("config created: " + str(cfg))

    exr = cfg.find_or_add_setting_by_class(unreal.MoviePipelineImageSequenceOutput_EXR)
    exr.compression = unreal.EXRCompressionFormat.NONE
    log("exr compression NONE ok")

    # 5.8.1 实测：无渲染通道则 "Shot has 0 Passes" 零输出——Deferred 通道提供图像数据
    cfg.find_or_add_setting_by_class(unreal.MoviePipelineDeferredPassBase)
    log("deferred pass added")

    out = cfg.find_or_add_setting_by_class(unreal.MoviePipelineOutputSetting)
    out.output_resolution = unreal.IntPoint(1920, 1080)
    out.output_directory = unreal.DirectoryPath("K:/rurix-ext/g10-frames/smoke")
    out.use_custom_frame_rate = True
    out.output_frame_rate = unreal.FrameRate(30, 1)
    log("output 1920x1080 @30 -> K:/rurix-ext/g10-frames/smoke")

    aa = cfg.find_or_add_setting_by_class(unreal.MoviePipelineAntiAliasingSetting)
    aa.engine_warm_up_count = 64
    log("warmup 64 ok")

    unreal.EditorAssetLibrary.save_asset(cfg_path)
    log("config saved")


main()
