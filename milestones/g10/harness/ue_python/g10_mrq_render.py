#!/usr/bin/env python3
"""G10.2 harness — MRQ 批量出图臂（臂 A 主路）UE 侧执行脚本草案。

形态：UE 内嵌 CPython（PythonScriptPlugin）经
  UnrealEditor-Cmd.exe <proj>.uproject <map> -game -ExecutePythonScript=g10_mrq_render.py -- <job.json>
调用（臂 A/C 复合：MRQ 主路出图 + Python 编排回调退出，命令面闭集见 RFC-0027 §4.1.3）。

职责：
  1. 读取 job JSON（scene_id / map / 契约参数 JSON 路径 / 输出目录 / 分辨率）；
  2. 经 g10_param_contract 解析契约参数 → UE 场景参数（坐标/单位/FOV 冻结公式映射）；
  3. 程序化搭建 MRQ 队列：EXR 输出（scene-linear，tone curve 关闭——HDR 臂捕获点 tonemap 前）、
     分辨率字段化、warm-up 帧（time.warmup_frames，TSR/时域累积收敛协议）、确定性 seed；
  4. 执行渲染，逐帧落盘 EXR，写 provenance 七元组 JSON 行
     （scene_id / camera_params_digest / lighting_params_digest / ue_build_id /
       gpu_driver_version / clock_lock_state / capture_arm —— RFC-0027 §4.1.4 闭集）。

地位：DRAFT 占位可解析形态，待 UE 5.8 引擎可用后实测修订（unreal API 仅引擎内可用）。
Assisted-by: Kimi-K3（G10.2 波）
"""
import json
import os
import sys

# UE 进程外仅做语法可解析；unreal 导入延迟到 run() 内（host 侧可 import 本模块做静态检查）。
unreal = None  # noqa: F841 — set inside run()

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import g10_param_contract as contract  # noqa: E402


def build_mrq_job(job):
    """程序化构建 MRQ 队列作业。job 字段闭集：
    scene_id / map_package / contract_params_path / output_dir / capture_arm。
    返回 (queue, job_handle)。DRAFT：MRQ API 调用待 5.8 实测校准。"""
    import unreal as ue  # UE 内嵌 CPython 内方可用

    with open(job["contract_params_path"], "r", encoding="utf-8") as f:
        c = contract.parse_contract(f.read())
    ue_params = contract.to_ue_scene_params(c)

    subsystem = ue.get_editor_subsystem(ue.MoviePipelineQueueSubsystem)  # 5.8.1 实测：EditorSubsystem
    queue = subsystem.get_queue()

    mrq_job = queue.allocate_new_job(ue.MoviePipelineExecutorJob)
    mrq_job.job_name = f"g10_{job['scene_id']}"
    mrq_job.map = ue.SoftObjectPath(job["map_package"])
    mrq_job.author = "g10-harness"

    config = mrq_job.get_configuration()

    # EXR 输出：HDR 臂 scene-linear（tonemap 前捕获点）；压缩配置收窄 {NONE, ZIP} 归 M134 spec 面
    # 5.8.1 实测类名：MoviePipelineImageSequenceOutput_EXR + EXRCompressionFormat
    exr_setting = config.find_or_add_setting_by_class(ue.MoviePipelineImageSequenceOutput_EXR)
    exr_setting.compression = ue.EXRCompressionFormat.NONE  # DRAFT：实测后按 spec 冻结值收窄登记

    # 5.8.1 实测：无渲染通道则 "Shot has 0 Passes" 零输出——Deferred 通道提供图像数据
    config.find_or_add_setting_by_class(ue.MoviePipelineDeferredPassBase)

    # 分辨率/帧参数字段化
    out_setting = config.find_or_add_setting_by_class(ue.MoviePipelineOutputSetting)
    res = ue_params["resolution"]
    out_setting.output_resolution = ue.IntPoint(res["w"], res["h"])
    out_setting.use_custom_frame_rate = True
    t = ue_params["time"]
    out_setting.output_frame_rate = ue.FrameRate(int(round(1.0 / t["fixed_dt_s"])), 1)
    out_setting.override_frame_padding = True

    # warm-up：时域累积收敛协议（warmup_frames 后第 capture_frame_index 帧捕获）
    # 5.8.1 实测：warmup 由 AntiAliasingSetting.engine_warm_up_count 承载
    aa_setting = config.find_or_add_setting_by_class(ue.MoviePipelineAntiAliasingSetting)
    aa_setting.engine_warm_up_count = int(t["warmup_frames"])

    # 调试/控制台覆盖：手动曝光 + 后处理全关基线（post 节 v1 最小闭集）
    # 5.8.1 实测：MoviePipelineConsoleVariableSetting 无 .console_variables 属性（冒烟期报错），
    # 正确字段名待引擎内 introspect 复核——保守保留本块并标注。
    console_setting = config.find_or_add_setting_by_class(ue.MoviePipelineConsoleVariableSetting)
    console_setting.console_variables = {
        "r.MotionBlurQuality": 0.0,
        "r.BloomQuality": 0.0,
        "r.Vignette": 0.0,
        "r.DepthOfFieldQuality": 0.0,
        "r.EyeAdaptation.EditorOnly": 0.0,  # 自动曝光禁入（exposure.mode=manual）
    }

    return queue, mrq_job, c, ue_params


def emit_provenance(job, contract_obj, frame_paths, out_path):
    """provenance 七元组闭集逐帧登记（RFC-0027 §4.1.4；缺行即 RED 由门侧求值）。
    时间戳/主机绝对路径/用户名不得入签名面——digest 字段只含参数 digest 与 build/驱动文本。"""
    import unreal as ue

    cam = dict(contract_obj["camera"])
    light = dict(contract_obj["lighting"])
    rows = []
    for fp in frame_paths:
        rows.append({
            "scene_id": job["scene_id"],
            "camera_params_digest": contract.param_digest({"camera": cam}),
            "lighting_params_digest": contract.param_digest({"lighting": light}),
            "ue_build_id": ue.SystemLibrary.get_engine_version(),  # 版本号+CL 字符串文本（非目录哈希）
            "gpu_driver_version": "PENDING_RUNTIME_QUERY",  # DRAFT：UE 侧 RHI 查询接口待实测
            "clock_lock_state": "PENDING_HOST_QUERY",        # DRAFT：锁频状态由 host 侧 nvidia-smi 登记补写
            "capture_arm": job["capture_arm"],                # 臂 id + 命令面/queue 配置 digest 由编排层拼接
            "frame_file": os.path.basename(fp),
        })
    with open(out_path, "w", encoding="utf-8", newline="\n") as f:
        json.dump({"frames": rows}, f, ensure_ascii=False, indent=2)
        f.write("\n")
    return rows


def run(argv):
    import unreal as ue

    if len(argv) < 1:
        raise RuntimeError("usage: g10_mrq_render.py <job.json>")
    with open(argv[0], "r", encoding="utf-8") as f:
        job = json.load(f)

    queue, mrq_job, contract_obj, ue_params = build_mrq_job(job)

    subsystem = ue.get_editor_subsystem(ue.MoviePipelineQueueSubsystem)
    executor = ue.MoviePipelineInProcessExecutor()
    # 5.8.1 实测：编辑器 cmd 模式脚本结束即 QUIT_EDITOR，异步 executor 无法完成渲染——
    # 出图必须走 Phase B 命令行模式（-LevelSequence + -MoviePipelineConfig），见 g10_mrq_smoke.py。
    # 完成/失败回调 → 写 provenance 并退出编辑器（Python 编排臂形态）
    # DRAFT：executor 回调签名与 quit_editor 时序待 5.8 实测校准。
    subsystem.render_queue_with_executor_instance(executor)
    ue.log(f"g10_mrq_render: job submitted for scene {job['scene_id']}")


if __name__ == "__main__":
    run(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
