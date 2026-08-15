#!/usr/bin/env python3
"""G10.5 harness — 应用层探针（RXS-0390）+ UE 进程内 digest（RXS-0384 L4 双端核验期载体）。

UE 内嵌 CPython 运行（editor cmd：-ExecutePythonScript；参数面走进程环境变量，
5.8.1 实测 sys.argv 不转发尾部参数）：
  G10_5_SCENE    = cornell-box | bistro-interior
  G10_5_CONTRACT = 契约参数 JSON 路径（milestones/g10/corpus/contract_params_<scene>.json）

探针面（双端各自管线语义）：
  1. UE 进程内解析契约 → param_digest_ue5（g10_param_contract 单源，RXS-0384 L3 布局）；
  2. 加载已建关卡 /Game/Maps/G10_<Scene>，读回 G10_ContractCamera actor 实际位姿
     （as-built：get_actor_transform 平移 f64 LWC + 旋转四元数 f32 读回，
     fov/aspect 经 CameraComponent field_of_view / aspect_ratio 属性读回——
     5.8.1 实测 get_camera_view 需 delta_time 参数，属性读回为稳定面）；
  3. 冻结标志物（RXS-0390 L2 逐值，本脚本内嵌同字面 + host 侧门脚本对账）经
     pos_contract_to_ue 映射到 UE 世界（f64），以 as-built 相机三轴（fwd/right/up）
     按 UE 视/投影链投影为像素：view = (rel·right, rel·up, rel·fwd)，
     ndc.x = view.x/(view.z·tan(hfov/2))、ndc.y = view.y/(view.z·tan(vfov/2))、
     px = (ndc.x/2+0.5)·w、py = (0.5−ndc.y/2)·h（UE 5.8 源树一手锚定：
     GameplayStatics.cpp CalculateViewProjectionMatricesFromMinimalView +
     PerspectiveMatrix.h FReversedZPerspectiveMatrix + SceneView.cpp
     ProjectWorldToScreen 像素映射）；
  4. 输出 G10_5_PROBE JSON 行：digest + 相机读回 + 逐点像素。

输出契约（host 门脚本解析）：最后一个 "G10_5_PROBE " 前缀行 = JSON。
Assisted-by: Kimi-K3（G10.5a 波续）
"""
import json
import math
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import g10_param_contract as contract  # noqa: E402

import unreal  # noqa: E402

# RXS-0390 L2 冻结标志物集（逐值字面；host 门脚本对账同字面，漂移即 RED）。
LANDMARKS = {
    "cornell-box": [
        [0.0, 0.0, 558.8],
        [552.8, 0.0, 558.8],
        [552.8, 548.8, 558.8],
        [0.0, 548.8, 558.8],
        [276.4, 274.4, 558.8],
    ],
    "bistro-interior": [
        [2.0375248420941845, 1.3697032820278594, -1.6595583445401449],
        [2.1463398736291461, 1.6862064060565474, -0.82191749619001619],
        [1.9521887623639345, 1.6862064214520678, -2.4999157956664435],
        [2.1228609218244348, 1.053200142603651, -0.81920089341384617],
        [1.9287098105592226, 1.0532001579991714, -2.4971991928902737],
    ],
}

SCENE_MAP = {
    "cornell-box": "/Game/Maps/G10_CornellBox",
    "bistro-interior": "/Game/Maps/G10_BistroInterior",
}


def log(m):
    unreal.log("G10_5_PROBE: " + str(m))


def main():
    scene_id = os.environ.get("G10_5_SCENE", "")
    contract_path = os.environ.get("G10_5_CONTRACT", "")
    if scene_id not in LANDMARKS or not contract_path:
        raise RuntimeError("env G10_5_SCENE / G10_5_CONTRACT 必填")

    with open(contract_path, "r", encoding="utf-8") as f:
        c = contract.parse_contract(f.read())
    digest = contract.param_digest(c)

    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level(SCENE_MAP[scene_id])
    cam_actor = None
    for a in unreal.EditorLevelLibrary.get_all_level_actors():
        if a.get_actor_label() == "G10_ContractCamera":
            cam_actor = a
            break
    if cam_actor is None:
        raise RuntimeError("关卡缺 G10_ContractCamera（先跑 g10_5_build_scenes.py）")

    t = cam_actor.get_actor_transform()
    loc = t.translation  # f64 LWC
    rot_q = t.rotation  # unreal.Quat（f32 读回）
    fwd_q = rot_q.rotate_vector(unreal.Vector(1.0, 0.0, 0.0))
    right_q = rot_q.rotate_vector(unreal.Vector(0.0, 1.0, 0.0))
    up_q = rot_q.rotate_vector(unreal.Vector(0.0, 0.0, 1.0))
    fwd = (fwd_q.x, fwd_q.y, fwd_q.z)
    right = (right_q.x, right_q.y, right_q.z)
    up = (up_q.x, up_q.y, up_q.z)

    cam_comp = cam_actor.get_component_by_class(unreal.CameraComponent)
    fov_h = float(cam_comp.get_editor_property("field_of_view"))
    # aspect 口径：constrain_aspect_ratio=False 时 UE 投影 aspect = 视口（MRQ 输出
    # 分辨率）宽高比，即契约 resolution——CameraComponent.aspect_ratio 属性不参与
    # （5.8.1 FMinimalViewInfo::CalculateProjectionMatrix 分支语义；实证：读回
    # 属性值 1.7778 与 512×512 契约不符，按契约 w/h 后探针像素与 Rurix 端逐位对账）。
    w = int(c["camera"]["resolution"]["w"])
    h = int(c["camera"]["resolution"]["h"])
    aspect = w / h
    tan_h = math.tan(math.radians(fov_h) / 2.0)
    tan_v = tan_h / aspect

    pixels = []
    for lm in LANDMARKS[scene_id]:
        p_ue = contract.pos_contract_to_ue(lm)
        rel = (p_ue[0] - loc.x, p_ue[1] - loc.y, p_ue[2] - loc.z)
        vz = sum(rel[i] * fwd[i] for i in range(3))
        vx = sum(rel[i] * right[i] for i in range(3))
        vy = sum(rel[i] * up[i] for i in range(3))
        if vz <= 0.0:
            pixels.append(None)
            continue
        ndc_x = vx / (vz * tan_h)
        ndc_y = vy / (vz * tan_v)
        px = (ndc_x / 2.0 + 0.5) * w
        py = (0.5 - ndc_y / 2.0) * h
        pixels.append([px, py])

    out = {
        "scene_id": scene_id,
        "param_digest_ue5_inprocess": digest,
        "camera_readback": {
            "loc_cm": [loc.x, loc.y, loc.z],
            "fwd": list(fwd),
            "right": list(right),
            "up": list(up),
            "fov_h_deg": fov_h,
            "aspect": aspect,
            "aspect_component_property": float(cam_comp.get_editor_property("aspect_ratio")),
        },
        "pixels_ue5": pixels,
    }
    # 输出双通道：单行 JSON 落 UE log + 文件落盘（5.8.1 实测 unreal.log 不进
    # cmd stdout——文件面为门脚本权威解析源，G10_5_PROBE_OUT 环境变量给路径）。
    line = json.dumps(out, separators=(",", ":"))
    unreal.log("G10_5_PROBE " + line)
    out_path = os.environ.get("G10_5_PROBE_OUT", "")
    if out_path:
        with open(out_path, "w", encoding="utf-8", newline="\n") as f:
            f.write(line + "\n")
    log("DONE scene=%s" % scene_id)


main()
