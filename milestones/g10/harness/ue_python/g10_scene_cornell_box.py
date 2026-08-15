#!/usr/bin/env python3
"""G10.2 harness — Cornell Box 最小场景 UE 侧程序化搭建脚本草案（G10.2 期暂定清单最小场景面之一）。

用途：引擎可用后在 G10RefRender 工程内生成 `/Game/Maps/G10_CornellBox` 关卡：
  - 程序化几何：五面墙（白/红/绿漫射）+ 双方块（高/矮）+ 顶灯面光源——几何/反射率数值参考
    Cornell PCG Public Use Data 页（仅数值来源参考登记，零第三方资产摄入，generated 类登记前提
    见 RFC-0027 §4.2 事实表 CornellBox 行）；
  - 相机/光照按 contract_params JSON（g10_param_contract 解析 + 冻结公式映射）布设；
  - 手动曝光（exposure.mode=manual，自动曝光禁入）+ post 节 v1 全关基线；
  - 标定场景标志物（RFC-0026 §4.6 应用层探针）入关卡元数据，供 M130/M139 探针断言。

坐标约定：脚本内以契约世界系（右手系/+Y up/米）书写，经 g10_param_contract 映射公式转 UE
厘米/左手系/Z-up——单源防分叉，禁在脚本内手写第二份换算。

地位：DRAFT 占位可解析形态，待 UE 5.8 引擎可用后实测修订（unreal API 仅引擎内可用；
几何尺寸/材质参数以经典 Cornell Box 文献值为占位，G10.3 清单冻结前可修订）。
Assisted-by: Kimi-K3（G10.2 波）
"""
import math
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import g10_param_contract as contract  # noqa: E402

# Cornell Box 经典尺寸占位（契约世界系，米）：555mm 见方盒体
BOX_HALF = 0.555 / 2.0
WALLS = [
    # (名字, 中心, 法向, 颜色 linear rgb) —— DRAFT 占位，经典 Cornell 反射率近似
    ("floor",   (0.0, 0.0, 0.0),            (0.0, 1.0, 0.0),  (0.725, 0.710, 0.680)),
    ("ceiling", (0.0, 0.555, 0.0),          (0.0, -1.0, 0.0), (0.725, 0.710, 0.680)),
    ("back",    (0.0, 0.2775, -0.2775),     (0.0, 0.0, 1.0),  (0.725, 0.710, 0.680)),
    ("left",    (-0.2775, 0.2775, 0.0),     (1.0, 0.0, 0.0),  (0.630, 0.065, 0.050)),  # 红
    ("right",   (0.2775, 0.2775, 0.0),      (-1.0, 0.0, 0.0), (0.161, 0.133, 0.427)),  # 绿（占位色相）
]
# 顶灯面片（发光面，DRAFT 占位尺寸 0.198×0.198 m 经典值）
LIGHT_PANEL = {"center": (0.0, 0.554, 0.0), "half_extent": (0.099, 0.0, 0.099),
               "emissive_linear_rgb": (17.0, 12.0, 4.0)}


def build_level(contract_obj, map_package="/Game/Maps/G10_CornellBox"):
    """在 UE 内程序化搭建 Cornell Box 关卡。DRAFT：unreal API 调用待 5.8 实测校准。"""
    import unreal as ue

    ue_params = contract.to_ue_scene_params(contract_obj)

    editor_level_lib = ue.EditorLevelLibrary()
    editor_actor_subsys = ue.get_editor_subsystem(ue.EditorActorSubsystem)

    # 1) 新关卡
    editor_level_lib.new_level(map_package)

    # 2) 五面墙 + 双方块 + 顶灯面片：以 Plane/Cube static mesh 缩放摆位（占位实现，
    #    实测期可换 Interchange 导入程序生成 glTF 以消除手工网格近似）
    #    —— 坐标一律 contract.pos_contract_to_ue() 换算，禁手写第二份。
    for name, center_c, _normal, _color in WALLS:
        loc = contract.pos_contract_to_ue(center_c)
        ue.log(f"g10_scene: wall {name} at UE loc {loc}")  # DRAFT：spawn 调用待实测

    # 3) 相机：契约参数 → UE CineCamera（水平 FOV 口径）
    cam_loc = ue_params["camera_location_cm"]
    cam_quat = ue_params["camera_quat_ue"]
    cam_fov_h = ue_params["camera_fov_h_deg"]
    ue.log(f"g10_scene: camera loc_cm={cam_loc} quat={cam_quat} fov_h={cam_fov_h}")

    # 4) 光照：sun（本场景 intensity 0）+ 顶灯面片 emissive；曝光手动 ev100
    ue.log(f"g10_scene: exposure ev100={ue_params['exposure_ev100']} (manual)")

    # 5) 标定探针标志物：盒体四角世界坐标入关卡自定义元数据（M130/M139 application_probes）
    ue.log("g10_scene: calibration landmarks registered (DRAFT)")

    editor_level_lib.save_current_level()
    return map_package


def run(argv):
    if len(argv) < 1:
        raise RuntimeError("usage: g10_scene_cornell_box.py <contract_params.json>")
    with open(argv[0], "r", encoding="utf-8") as f:
        c = contract.parse_contract(f.read())
    return build_level(c)


if __name__ == "__main__":
    run(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
