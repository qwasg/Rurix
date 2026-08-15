#!/usr/bin/env python3
"""G10.5 harness — 双场景 UE 关卡建设 + MRQ Phase A 资产（编辑器 cmd 模式运行）。

用法：
  UnrealEditor-Cmd.exe <proj> -ExecutePythonScript=g10_5_build_scenes.py -- \
      --scene cornell-box|bistro-interior --contract <contract_params.json> [--skip-import]

建设面（RFC-0026 §4.6 契约应用 + RFC-0027 §4.1 出图编排边界）：
  1. Interchange 导入场景 glTF → /Game/G10/<scene_id>/（导入设置 is_automated +
     replace_existing；单位/轴转换实测登记：glTF (x,y,z) → UE (x,z,y)·100，
     经 part_00 bounds 实测反推验证，见 g10_5_ab_preview.md）；
  2. 关卡 /Game/Maps/G10_<Scene>：网格 actor 世界变换 = R_fix（yaw+90° 修正
     旋转 = M∘C⁻¹，契约 M 映射 p_ue=(−z,x,y)·100 与导入 C 映射之差）。
     **5.8.1 实证（G10.5a 波续定案）：Interchange import_asset 把 glTF 节点
     世界变换烘进网格顶点**（bistro 全网格节点 −90°X 旋转实测：actor 再施加
     节点旋转 = 双重旋转，场景倾倒出全黑帧；网格局部顶点 = C·R_node·v·100
     实测反推成立）——故 actor 只挂 R_fix，节点 TRS 不重复施加；语料实测
     双场景网格节点全部扁平单引用（bistro 1186/1186 深度 1、零多引用），
     嵌套/多实例情形不在本 harness 消费面（出现即报错不静默）；
  3. 契约相机（CameraActor，水平 FOV = fov_y→fov_h 冻结公式换算，
     constrain_aspect_ratio=False）+ 契约光照（DirectionalLight lux 直给 +
     SkyLight 指定白色 cubemap × 契约 sky.intensity）+ 手动曝光
     （PostProcessVolume unbound：AEM_MANUAL + 物理相机曝光 N²·S=2^EV100——
     UE 5.8.1 源码实证 FixedExposure=2^(−EV100)，PostProcessEyeAdaptation.cpp
     CalculateManualAutoExposure/GetEyeAdaptationFixedExposure）；
  4. MRQ Phase A：LevelSequence（possess 关卡相机 + CameraCutTrack）+
     PrimaryConfig（EXR NONE + DeferredPass + 契约分辨率 + warmup + 固定帧率
     + 后处理全关 console vars）；Phase B 命令行渲染归编排层。
  5. CornellBox 地板棋盘格纹理按双端最大子集口径降为白材质（Rurix 侧纹理
     不采样——诚实边界对齐，登记进 preview）；Bistro 动画 Take 001 / glTF
     相机节点不引用（动画剥离登记）。

坐标/单位一律经 g10_param_contract 冻结公式换算，禁脚本内手写第二份。
Assisted-by: Kimi-K3（G10.5a 波）
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

CONTENT_ROOT = "/Game/G10"
MAPS_ROOT = "/Game/Maps"
CINE_ROOT = "/Game/Cinematics"

SCENE_GLTF = {
    "cornell-box": "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf",
    "bistro-interior": "K:/rurix_g10_cache/bistro-orca/v5_2/derived/BistroInterior/BistroInterior.gltf",
}
SCENE_MAP = {
    "cornell-box": "G10_CornellBox",
    "bistro-interior": "G10_BistroInterior",
}
WHITE_HDR = "K:/rurix-ext/g10-ue/harness_assets/white_2x1.hdr"


def log(m):
    unreal.log("G10_5_BUILD: " + str(m))


# ---------------------------------------------------------------------------
# 1) 白色 lat-long HDR → TextureCube（SkyLight 指定 cubemap 常量天光）
# ---------------------------------------------------------------------------

def ensure_white_cubemap():
    path = CONTENT_ROOT + "/white_cube"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        obj = unreal.EditorAssetLibrary.load_asset(path)
        log("white cubemap 已在树: " + obj.get_class().get_name())
        return obj
    task = unreal.AssetImportTask()
    task.filename = WHITE_HDR
    task.destination_path = CONTENT_ROOT
    task.automated = True
    task.replace_existing = True
    task.save = True
    unreal.AssetToolsHelpers.get_asset_tools().import_asset_tasks([task])
    # 期望产物 TextureCube（lat-long 2:1 HDR 自动识别）；登记实际类名
    found = None
    for name in ("white_2x1", "white_cube"):
        p = CONTENT_ROOT + "/" + name
        if unreal.EditorAssetLibrary.does_asset_exist(p):
            obj = unreal.EditorAssetLibrary.load_asset(p)
            log("hdr 导入产物: " + p + " class=" + obj.get_class().get_name())
            found = obj
            break
    if found is None:
        raise RuntimeError("white hdr 导入失败（无产物）")
    if found.get_class().get_name() != "TextureCube":
        raise RuntimeError("white hdr 导入产物非 TextureCube: " + found.get_class().get_name())
    return found


# ---------------------------------------------------------------------------
# 2) Interchange 导入
# ---------------------------------------------------------------------------

def import_scene_gltf(scene_id):
    dest = CONTENT_ROOT + "/" + scene_id
    mgr = unreal.InterchangeManager.get_interchange_manager_scripted()
    sd = unreal.InterchangeManager.create_source_data(SCENE_GLTF[scene_id])
    params = unreal.ImportAssetParameters()
    params.is_automated = True
    params.replace_existing = True
    result = mgr.import_asset(dest, sd, params)
    if not result:
        raise RuntimeError("import_asset 空返回: " + scene_id)
    unreal.EditorAssetLibrary.save_directory(dest, only_if_is_dirty=False)
    n_mesh = 0
    names = []
    for obj in result:
        cls = obj.get_class().get_name()
        if cls == "StaticMesh":
            n_mesh += 1
            names.append(obj.get_name())
    log("import 完成: %s meshes=%d total_assets=%d" % (scene_id, n_mesh, len(result)))
    return names


# ---------------------------------------------------------------------------
# 3) glTF 网格资产 → 关卡 actors（actor 世界变换 = R_fix；节点变换烘焙定案见头注）
# ---------------------------------------------------------------------------

def build_level(scene_id, contract_obj):
    ue_params = contract.to_ue_scene_params(contract_obj)
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    actor_subsys = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)

    map_name = SCENE_MAP[scene_id]
    map_path = MAPS_ROOT + "/" + map_name
    if unreal.EditorAssetLibrary.does_asset_exist(map_path):
        level_subsys.load_level(map_path)
        # 清空既有 actors（幂等重建）
        for a in unreal.EditorLevelLibrary.get_all_level_actors():
            actor_subsys.destroy_actor(a)
    else:
        level_subsys.new_level(map_path)

    # 网格 actor 世界变换 = R_fix（头注第 2 条实证定案：Interchange 导入把
    # glTF 节点世界变换烘进网格顶点——bistro 全网格节点 −90°X 旋转双重施加
    # 实测：actor 旋转正确时顶点已含节点旋转，actor 只挂 R_fix 落位正确；
    # 扁平单引用前提逐节点核验，违反即报错不静默）。
    fix = unreal.Transform(rotation=unreal.Rotator(pitch=0.0, yaw=90.0, roll=0.0))

    with open(SCENE_GLTF[scene_id], "r", encoding="utf-8") as f:
        gltf = json.load(f)
    nodes = gltf.get("nodes", [])
    mesh_ref = {}
    for n in nodes:
        if "mesh" in n:
            mesh_ref[n["mesh"]] = mesh_ref.get(n["mesh"], 0) + 1
    if any(c > 1 for c in mesh_ref.values()):
        raise RuntimeError("网格多引用不在本 harness 消费面（烘焙口径需逐实例复制），语料实测应为零")

    mesh_assets = {}  # 归一化名 → 资产路径
    mesh_dir = CONTENT_ROOT + "/" + scene_id
    for sub in unreal.EditorAssetLibrary.list_assets(mesh_dir, recursive=True):
        obj_path = sub
        name = obj_path.split("/")[-1].split(".")[0]
        norm = "".join(ch for ch in name.lower() if ch.isalnum())
        mesh_assets[norm] = obj_path

    n_spawned = 0
    for idx, node in enumerate(nodes):
        if "mesh" not in node:
            continue
        actor = actor_subsys.spawn_actor_from_class(
            unreal.StaticMeshActor, fix.translation, fix.rotation.rotator()
        )
        actor.set_actor_label("G10N_" + node.get("name", "node%d" % idx))
        actor.set_folder_path("G10/Nodes")
        norm = "".join(ch for ch in node.get("name", "").lower() if ch.isalnum())
        asset_path = mesh_assets.get(norm)
        if asset_path is None:
            raise RuntimeError("网格资产未匹配节点: " + node.get("name", "?"))
        mesh = unreal.EditorAssetLibrary.load_asset(asset_path)
        smc = actor.static_mesh_component
        smc.set_static_mesh(mesh)
        smc.set_mobility(unreal.ComponentMobility.STATIC)
        if scene_id == "cornell-box" and "white_tex" in node.get("name", ""):
            # 地板棋盘格降为白材质（双端最大子集口径对齐）
            white = unreal.EditorAssetLibrary.load_asset(
                CONTENT_ROOT + "/cornell-box/cornell_box/Materials/white.white"
            )
            if white is not None:
                smc.set_material(0, white)
                log("地板材质降为 white（checker 纹理双端不采样口径）")
        n_spawned += 1
    log("节点 spawn 完成: %d mesh actors" % n_spawned)

    # ---- 契约相机 ----
    cam_loc = ue_params["camera_location_cm"]
    cam_q = ue_params["camera_quat_ue"]  # (w,x,y,z)
    cam_fov_h = ue_params["camera_fov_h_deg"]
    cam_actor = actor_subsys.spawn_actor_from_class(
        unreal.CameraActor, unreal.Vector(*cam_loc), unreal.Rotator(0, 0, 0)
    )
    cam_actor.set_actor_label("G10_ContractCamera")
    cam_actor.set_folder_path("G10")
    quat = unreal.Quat(cam_q[1], cam_q[2], cam_q[3], cam_q[0])  # unreal.Quat(x,y,z,w)
    cam_actor.set_actor_rotation(quat.rotator(), False)
    cam_comp = cam_actor.camera_component
    cam_comp.set_field_of_view(cam_fov_h)
    cam_comp.set_constraint_aspect_ratio(False)
    cam_comp.set_editor_property("constrain_aspect_ratio", False)
    log("相机: loc_cm=%s fov_h=%.6f" % (str(cam_loc), cam_fov_h))

    # ---- 契约光照 ----
    sun_dir_ue = ue_params["sun_direction_ue"]  # 传播方向（UE 系）
    sun_actor = actor_subsys.spawn_actor_from_class(unreal.DirectionalLight, unreal.Vector(0, 0, 1000), unreal.Rotator(0, 0, 0))
    sun_actor.set_actor_label("G10_Sun")
    sun_actor.set_folder_path("G10")
    look = unreal.MathLibrary.find_look_at_rotation(
        unreal.Vector(0, 0, 0), unreal.Vector(*sun_dir_ue)
    )
    sun_actor.set_actor_rotation(look, False)
    dl = sun_actor.get_component_by_class(unreal.DirectionalLightComponent)
    dl.set_intensity(float(ue_params["sun_intensity_lux"]))
    rgb = ue_params["sun_color_linear_rgb"]
    dl.set_light_color(unreal.LinearColor(rgb[0], rgb[1], rgb[2], 1.0), True)
    dl.set_mobility(unreal.ComponentMobility.MOVABLE)
    log("太阳光: dir_ue=%s lux=%s" % (str(sun_dir_ue), str(ue_params["sun_intensity_lux"])))

    white_cube = ensure_white_cubemap()
    sky_actor = actor_subsys.spawn_actor_from_class(unreal.SkyLight, unreal.Vector(0, 0, 0), unreal.Rotator(0, 0, 0))
    sky_actor.set_actor_label("G10_Sky")
    sky_actor.set_folder_path("G10")
    sl = sky_actor.get_component_by_class(unreal.SkyLightComponent)
    sl.set_editor_property("source_type", unreal.SkyLightSourceType.SLS_SPECIFIED_CUBEMAP)
    sl.set_cubemap(white_cube)
    sl.set_intensity(float(ue_params["sky_intensity"]))
    sl.set_editor_property("real_time_capture", False)
    sl.set_mobility(unreal.ComponentMobility.MOVABLE)
    log("天光: 指定白色 cubemap × intensity=%s" % str(ue_params["sky_intensity"]))

    # ---- 手动曝光（FixedExposure=2^(−EV100) 源码实证公式；N²·S=2^EV100 @ISO100）----
    ev100 = float(ue_params["exposure_ev100"])
    n_fstop = 4.0
    shutter = (2.0 ** ev100) / (n_fstop * n_fstop)
    ppv = actor_subsys.spawn_actor_from_class(unreal.PostProcessVolume, unreal.Vector(0, 0, 0), unreal.Rotator(0, 0, 0))
    ppv.set_actor_label("G10_Exposure")
    ppv.set_folder_path("G10")
    ppv.set_editor_property("unbound", True)
    pps = unreal.PostProcessSettings()
    pps.set_editor_property("override_auto_exposure_method", True)
    pps.set_editor_property("auto_exposure_method", unreal.AutoExposureMethod.AEM_MANUAL)
    pps.set_editor_property("override_auto_exposure_apply_physical_camera_exposure", True)
    pps.set_editor_property("auto_exposure_apply_physical_camera_exposure", True)
    pps.set_editor_property("override_camera_iso", True)
    pps.set_editor_property("camera_iso", 100.0)
    pps.set_editor_property("override_camera_shutter_speed", True)
    pps.set_editor_property("camera_shutter_speed", shutter)
    pps.set_editor_property("override_depth_of_field_fstop", True)
    pps.set_editor_property("depth_of_field_fstop", n_fstop)
    # post 节 v1 全关基线
    pps.set_editor_property("override_bloom_intensity", True)
    pps.set_editor_property("bloom_intensity", 0.0)
    pps.set_editor_property("override_vignette_intensity", True)
    pps.set_editor_property("vignette_intensity", 0.0)
    pps.set_editor_property("override_motion_blur_amount", True)
    pps.set_editor_property("motion_blur_amount", 0.0)
    ppv.set_editor_property("settings", pps)
    log("手动曝光: ev100=%.4f → fstop=%.1f shutter=1/%.2fs（FixedExposure=2^-EV100）" % (ev100, n_fstop, 1.0 / shutter))

    level_subsys.save_current_level()
    log("关卡保存: " + map_path)
    return map_path, cam_actor


# ---------------------------------------------------------------------------
# 4) MRQ Phase A 资产（LevelSequence + PrimaryConfig）
# ---------------------------------------------------------------------------

def build_mrq_assets(scene_id, contract_obj, cam_actor, out_dir):
    ue_params = contract.to_ue_scene_params(contract_obj)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    tag = scene_id.replace("-", "_")

    seq_path = CINE_ROOT + "/G10_%sSeq" % tag
    if unreal.EditorAssetLibrary.does_asset_exist(seq_path):
        unreal.EditorAssetLibrary.delete_asset(seq_path)
    seq = tools.create_asset("G10_%sSeq" % tag, CINE_ROOT, unreal.LevelSequence, unreal.LevelSequenceFactoryNew())
    seq.set_playback_start(0)
    seq.set_playback_end(4)
    binding = seq.add_possessable(cam_actor)
    cut_track = seq.add_track(unreal.MovieSceneCameraCutTrack)
    section = cut_track.add_section()
    section.set_start_frame(0)
    section.set_end_frame(4)
    bid = seq.get_binding_id(binding)
    section.set_camera_binding_id(bid)
    unreal.EditorAssetLibrary.save_asset(seq_path)
    log("sequence ready: " + seq_path)

    cfg_path = CINE_ROOT + "/G10_%sConfig" % tag
    if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
        unreal.EditorAssetLibrary.delete_asset(cfg_path)
    cfg = tools.create_asset(
        "G10_%sConfig" % tag, CINE_ROOT,
        unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory(),
    )
    exr = cfg.find_or_add_setting_by_class(unreal.MoviePipelineImageSequenceOutput_EXR)
    exr.compression = unreal.EXRCompressionFormat.NONE
    # HDR 臂捕获点 = tonemap 前 scene-linear（RXS-0386 L1）：关 tone curve——
    # 5.8 源树 MoviePipelineEXROutput.cpp: bDisableToneCurve → SCS_FinalColorHDR
    # （否则 SCS_FinalToneCurveHDR 压缩进 ~[0,1]，实证：修复前 UE 帧 max≈1.03）。
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
    unreal.EditorAssetLibrary.save_asset(cfg_path)
    log("config saved: " + cfg_path + " res=%dx%d warmup=%d -> %s" % (res["w"], res["h"], int(t["warmup_frames"]), out_dir))
    return seq_path, cfg_path


def main():
    # UE 内嵌 CPython 不转发命令行尾部参数（5.8.1 实测 sys.argv 仅脚本路径）——
    # 参数面走进程环境变量（G10_5_SCENE / G10_5_CONTRACT / G10_5_SKIP_IMPORT）。
    scene_id = os.environ.get("G10_5_SCENE", "")
    contract_path = os.environ.get("G10_5_CONTRACT", "")
    skip_import = os.environ.get("G10_5_SKIP_IMPORT", "0") == "1"
    if scene_id not in SCENE_GLTF or not contract_path:
        raise RuntimeError(
            "env G10_5_SCENE=cornell-box|bistro-interior G10_5_CONTRACT=<params.json> [G10_5_SKIP_IMPORT=1] 必填"
        )

    with open(contract_path, "r", encoding="utf-8") as f:
        c = contract.parse_contract(f.read())
    log("契约解析 OK（UE 侧解析器）: scene=%s" % scene_id)

    if not skip_import:
        import_scene_gltf(scene_id)
    else:
        log("跳过导入（--skip-import）")
    map_path, cam_actor = build_level(scene_id, c)
    out_dir = "K:/rurix-ext/g10-frames/g10_5/ue/" + scene_id
    seq_path, cfg_path = build_mrq_assets(scene_id, c, cam_actor, out_dir)
    log("BUILD DONE scene=%s map=%s seq=%s cfg=%s" % (scene_id, map_path, seq_path, cfg_path))


main()
