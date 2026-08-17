#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G12.4 harness — UE PT 对标双场景关卡建设 + MRQ Phase A 资产（编辑器 cmd 模式运行；
spec/visual_comparison.md RXS-0403 L1/L3；RFC-0029 §4.6 L1/L4）。

用法：
  UnrealEditor-Cmd.exe <proj> -ExecutePythonScript=g12_4_build_pt_scenes.py
  参数面走进程环境变量（5.8.1 实测 sys.argv 不转发尾部参数，G10.5 同律）：
    G12_4_SCENE=cornell-box|bistro-interior（必填）
    G12_4_CONTRACT=<g12_ue_pt_parity_contract.json 路径>（必填）
    G12_4_SKIP_IMPORT=1（可选：跳过 Interchange 重导入）
    G12_4_PROBE_OUT=<probe.json>（可选：建设 provenance/探针落盘）

建设面（G10.5a/G11.3 实证面同律，G10 脚本 0-byte 不消费不回写——本文件独立新建）：
  1. 契约解析（g12_pt_contract UE 侧解析器）+ digest 实测打印（三向互证面）；
  2. Interchange 导入场景 glTF → /Game/G12/<scene_id>/（cornell = 生成语料
     K:/rurix_g10_cache/cornell-box-generated/v1/；bistro = G11.3 DDS→PNG 转码
     派生面 K:/rurix-ext/g11-assets/bistro-interior-ue/）；actor 世界变换 =
     R_fix（yaw+90°——节点变换烘焙定案 G10.5a 头注同律）；
  3. 契约相机（quat→UE 冻结公式 q_ue=(w,z,−x,−y)；方形画幅 fov_h=fov_y）+
     手动曝光 PPV（FixedExposure=2^(−EV100)，AEM_MANUAL 5.8.1 源码实证面）+
     PT 覆盖（PathTracingMaxBounces=4 双端同深度 + EmissiveMaterials 开）；
  4. 契约灯面：cornell = 天花 quad 面光（/Engine/BasicShapes/Plane + 自研
     emissive 父材质 M_G12_PT_Emissive_Parent〔Emissive 向量参数直给 Le
     线性 nit〕，p00/e1/e2 经 p_ue=(−z,x,y)·100 映射）；bistro = 4 点光
     （position 映射 ×100，intensity_cd 直给 candela，光色线性 b_srgb=False）
     + emissive 材质面光（4 契约 emissive 材质 MIC 重挂 emissive 父材质 +
     emissive 纹理绑定，逐材质读回核验 + 探针登记）；sun/sky 不建（契约
     0.0 显式登记面）；cornell 壳体双面 MIC 置换（G11.3 U1 同律）；
  5. MRQ Phase A：LevelSequence（possess 契约相机 + CameraCutTrack）+ 逐 spp
     PrimaryConfig（EXR NONE + disable_tone_curve〔SCS_FinalColorHDR 捕获点〕+
     MoviePipelineDeferredPass_PathTracer + temporal_sample_count=spp〔PT 采样
     数 = temporal×spatial，MovieGraphPathTracerPass.cpp:39 语义面〕+ 契约
     分辨率 + console vars r.PathTracing=1/FilterWidth=0/MaxBounces=4 等）——
     Phase B 命令行逐 job 渲染归编排层（milestones/g12/harness/g12_4_ue_render.py）。

Assisted-by: Kimi-K3（G12.4 UE PT 对标波）
"""
import json
import math
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import g12_pt_contract as pt_contract  # noqa: E402

import unreal  # noqa: E402

CONTENT_ROOT = "/Game/G12"
MAPS_ROOT = "/Game/Maps"
CINE_ROOT = "/Game/Cinematics"

SCENE_GLTF = {
    # G12.4 PT 双绕向派生语料（harness 侧对齐面——Interchange 导入反射映射
    # (x,z,y)·100 det=−1 翻转绕向 + UE PT RT 遍历背面剔除实测（G12.4 探针
    # 取证：双面材质 MIC/父材质/网格槽位重挂三路不接线;引擎原生网格 PT 正常）
    # ⇒ 内容恒等双绕向派生承载,milestones/g12/harness/g12_4_make_pt2sided.py
    # 产,派生报告 milestones/g12/g12_4_pt2sided_derivation.json;Rurix 臂续
    # 消费 M133 原语料——同一表面集,场景恒等）。
    "cornell-box": "K:/rurix-ext/g12-assets/cornell-box-pt2sided/cornell_box.gltf",
    "bistro-interior": "K:/rurix-ext/g12-assets/bistro-interior-ue-pt2sided/BistroInterior.gltf",
}
SCENE_MAP = {
    "cornell-box": "G12_PTCornellBox",
    "bistro-interior": "G12_PTBistroInterior",
}


def log(m):
    unreal.log("G12_4_BUILD: " + str(m))


# ---------------------------------------------------------------------------
# 契约 → UE 参数（RXS-0384 L2 冻结公式镜像：p_ue=(−z,x,y)·100；q_ue=(w,z,−x,−y)；
# 方形画幅 fov_h=fov_y——契约世界系右手 +Y up 米 / UE 厘米左手 Z-up）
# ---------------------------------------------------------------------------

def to_ue_pos(p):
    return (-p[2] * 100.0, p[0] * 100.0, p[1] * 100.0)


def to_ue_quat(q):
    # (w,x,y,z) → (w,z,−x,−y)（v1.1 errata 修订式）
    return (q[0], q[3], -q[1], -q[2])


def scene_row(contract, scene_id):
    for s in contract["scenes"]:
        if s["scene_id"] == scene_id:
            return s
    raise RuntimeError("契约缺场景行: " + scene_id)


# ---------------------------------------------------------------------------
# Interchange 导入（g10_5 同律）
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
    n_mesh = sum(1 for o in result if o.get_class().get_name() == "StaticMesh")
    log("import 完成: %s meshes=%d total_assets=%d" % (scene_id, n_mesh, len(result)))


# ---------------------------------------------------------------------------
# G12 PT 材质面（emissive 父材质 + bistro emissive MIC 重挂/绑定）
# ---------------------------------------------------------------------------

def ensure_pt_emissive_parent():
    """PT 对标父材质：BaseColor 向量参数 + Emissive 向量参数（线性直给 nit;
    粗糙度 1.0 / 金属 0.0 = 双端朗伯口径最大子集;单面——绕向法线口径）。"""
    path = CONTENT_ROOT + "/M_G12_PT_Emissive_Parent"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        return unreal.EditorAssetLibrary.load_asset(path)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mat = tools.create_asset(
        "M_G12_PT_Emissive_Parent", CONTENT_ROOT, unreal.Material, unreal.MaterialFactoryNew()
    )
    mel = unreal.MaterialEditingLibrary
    vp = mel.create_material_expression(mat, unreal.MaterialExpressionVectorParameter, -700, 0)
    vp.set_editor_property("parameter_name", "BaseColor")
    vp.set_editor_property("default_value", unreal.LinearColor(1.0, 1.0, 1.0, 1.0))
    mel.connect_material_property(vp, "RGB", unreal.MaterialProperty.MP_BASE_COLOR)
    ve = mel.create_material_expression(mat, unreal.MaterialExpressionVectorParameter, -700, 220)
    ve.set_editor_property("parameter_name", "Emissive")
    ve.set_editor_property("default_value", unreal.LinearColor(0.0, 0.0, 0.0, 1.0))
    mel.connect_material_property(ve, "RGB", unreal.MaterialProperty.MP_EMISSIVE_COLOR)
    sp = mel.create_material_expression(mat, unreal.MaterialExpressionScalarParameter, -700, 440)
    sp.set_editor_property("parameter_name", "Roughness")
    sp.set_editor_property("default_value", 1.0)
    mel.connect_material_property(sp, "", unreal.MaterialProperty.MP_ROUGHNESS)
    sp2 = mel.create_material_expression(mat, unreal.MaterialExpressionScalarParameter, -700, 600)
    sp2.set_editor_property("parameter_name", "Metallic")
    sp2.set_editor_property("default_value", 0.0)
    mel.connect_material_property(sp2, "", unreal.MaterialProperty.MP_METALLIC)
    unreal.EditorAssetLibrary.save_asset(path)
    log("PT emissive 父材质就绪: " + path)
    return unreal.EditorAssetLibrary.load_asset(path)


def ensure_two_sided_parent():
    """双面父材质（G11.3 U1 同律新建于 /Game/G12——G10 内容 0-byte 不消费）。"""
    path = CONTENT_ROOT + "/M_G12_TwoSided_Parent"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        return unreal.EditorAssetLibrary.load_asset(path)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mat = tools.create_asset(
        "M_G12_TwoSided_Parent", CONTENT_ROOT, unreal.Material, unreal.MaterialFactoryNew()
    )
    mat.set_editor_property("two_sided", True)
    mel = unreal.MaterialEditingLibrary
    vp = mel.create_material_expression(mat, unreal.MaterialExpressionVectorParameter, -600, 0)
    vp.set_editor_property("parameter_name", "BaseColor")
    vp.set_editor_property("default_value", unreal.LinearColor(1.0, 1.0, 1.0, 1.0))
    mel.connect_material_property(vp, "RGB", unreal.MaterialProperty.MP_BASE_COLOR)
    sp = mel.create_material_expression(mat, unreal.MaterialExpressionScalarParameter, -600, 160)
    sp.set_editor_property("parameter_name", "Roughness")
    sp.set_editor_property("default_value", 1.0)
    mel.connect_material_property(sp, "", unreal.MaterialProperty.MP_ROUGHNESS)
    sp2 = mel.create_material_expression(mat, unreal.MaterialExpressionScalarParameter, -600, 320)
    sp2.set_editor_property("parameter_name", "Metallic")
    sp2.set_editor_property("default_value", 0.0)
    mel.connect_material_property(sp2, "", unreal.MaterialProperty.MP_METALLIC)
    unreal.EditorAssetLibrary.save_asset(path)
    chk = unreal.EditorAssetLibrary.load_asset(path)
    if not chk.get_editor_property("two_sided"):
        raise RuntimeError("双面父材质 two_sided 读回失败")
    log("双面父材质就绪: " + path)
    return chk


def apply_two_sided_cornell(gltf, spawned):
    """cornell 壳体双面化（G11.3 U1 同律：逐 actor 按 gltf baseColorFactor 换发
    双面 MIC；地板 white_tex 双面白〔棋盘格纹理双端不采样口径维持〕）。"""
    parent = ensure_two_sided_parent()
    mel = unreal.MaterialEditingLibrary
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mats = gltf.get("materials", [])
    mic_cache = {}
    for actor, mat_idx in spawned:
        if mat_idx is None or mat_idx >= len(mats):
            raise RuntimeError("cornell 图元材质索引缺失: %s" % mat_idx)
        m = mats[mat_idx]
        name = m.get("name", "mat%d" % mat_idx)
        fac = m.get("pbrMetallicRoughness", {}).get("baseColorFactor", [1.0, 1.0, 1.0, 1.0])
        mic = mic_cache.get(name)
        if mic is None:
            mic_path = CONTENT_ROOT + "/G12_TS_%s" % name
            if unreal.EditorAssetLibrary.does_asset_exist(mic_path):
                mic = unreal.EditorAssetLibrary.load_asset(mic_path)
            else:
                mic = tools.create_asset(
                    "G12_TS_%s" % name, CONTENT_ROOT,
                    unreal.MaterialInstanceConstant, unreal.MaterialInstanceConstantFactoryNew(),
                )
                mel.set_material_instance_parent(mic, parent)
            mel.set_material_instance_vector_parameter_value(
                mic, "BaseColor", unreal.LinearColor(fac[0], fac[1], fac[2], 1.0)
            )
            unreal.EditorAssetLibrary.save_asset(mic_path)
            mic_cache[name] = mic
        smc = actor.static_mesh_component
        for slot in range(smc.get_num_materials()):
            smc.set_material(slot, mic)
    log("cornell 双面置换完成: %d actors / %d MIC" % (len(spawned), len(mic_cache)))


def bind_bistro_textures(scene_id):
    """bistro MIC 纹理绑定（G11.3 U2 同律：Interchange 绑定缺位面显式绑定 +
    读回核验；/Game/G12 根下命名空间）。"""
    with open(SCENE_GLTF[scene_id], "r", encoding="utf-8") as f:
        doc = json.load(f)
    images = doc.get("images", [])
    textures = doc.get("textures", [])
    mats = doc.get("materials", [])
    base_dir = CONTENT_ROOT + "/" + scene_id + "/BistroInterior"
    mel = unreal.MaterialEditingLibrary
    bound = 0
    problems = []
    for m in mats:
        name = m.get("name")
        if not name:
            problems.append({"material": None, "reason": "材质缺 name"})
            continue
        safe = "".join(ch if (ch.isalnum() or ch == "_") else "_" for ch in name)
        mic_path = base_dir + "/Materials/%s.%s" % (safe, safe)
        if not unreal.EditorAssetLibrary.does_asset_exist(mic_path):
            problems.append({"material": name, "reason": "MIC 资产缺失: %s" % safe})
            continue
        mic = unreal.EditorAssetLibrary.load_asset(mic_path)
        nb = 0
        for param, ref in (
            ("BaseColorTexture", (m.get("pbrMetallicRoughness") or {}).get("baseColorTexture")),
            ("NormalTexture", m.get("normalTexture")),
        ):
            if not ref:
                continue
            ti = ref.get("index")
            src = textures[ti].get("source")
            stem = images[src].get("uri", "").rsplit("/", 1)[-1].rsplit(".", 1)[0]
            tex_path = base_dir + "/Textures/%s.%s" % (stem, stem)
            if not unreal.EditorAssetLibrary.does_asset_exist(tex_path):
                problems.append({"material": name, "reason": "纹理资产缺失: %s" % stem})
                continue
            mel.set_material_instance_texture_parameter_value(
                mic, param, unreal.EditorAssetLibrary.load_asset(tex_path)
            )
            nb += 1
        if nb:
            tpv = mic.get_editor_property("texture_parameter_values") or []
            nonnull = sum(1 for e in tpv if getattr(e, "parameter_value", None) is not None)
            if nonnull < nb:
                raise RuntimeError("MIC 纹理绑定读回失败: %s" % name)
            unreal.EditorAssetLibrary.save_asset(mic_path)
            bound += 1
    log("bistro MIC 纹理绑定: %d 材质（problems=%d）" % (bound, len(problems)))
    if problems:
        raise RuntimeError("纹理绑定面缺行（禁静默）: %s" % json.dumps(problems[:4], ensure_ascii=False))
    return bound


def setup_bistro_emissive(scene_id, srow):
    """契约 4 emissive 材质：MIC 重挂 PT emissive 父材质 + emissive 纹理绑定
    （emissiveTexture → PNG 派生面），Le = 契约 le_linear_rgb × emissive 纹理
    均值口径——UE 侧消费 = emissiveFactor×纹理（同源,残余口径差进差距登记）；
    逐材质读回核验 + 探针返回。"""
    with open(SCENE_GLTF[scene_id], "r", encoding="utf-8") as f:
        doc = json.load(f)
    images = doc.get("images", [])
    textures = doc.get("textures", [])
    mats = doc.get("materials", [])
    base_dir = CONTENT_ROOT + "/" + scene_id + "/BistroInterior"
    mel = unreal.MaterialEditingLibrary
    parent = ensure_pt_emissive_parent()
    probe = []
    for em in srow["lighting"]["emissive_materials"]:
        name = em["material_name"]
        mi = em["material_index"]
        m = mats[mi]
        if m.get("name") != name:
            raise RuntimeError("emissive 材质索引/名称断裂: %s vs idx %d" % (name, mi))
        safe = "".join(ch if (ch.isalnum() or ch == "_") else "_" for ch in name)
        mic_path = base_dir + "/Materials/%s.%s" % (safe, safe)
        if not unreal.EditorAssetLibrary.does_asset_exist(mic_path):
            raise RuntimeError("emissive MIC 资产缺失: " + safe)
        mic = unreal.EditorAssetLibrary.load_asset(mic_path)
        mel.set_material_instance_parent(mic, parent)
        rec = {"material": name, "le_contract": em["le_linear_rgb"]}
        # BaseColor 参数（factor 直给,纹理由 U2 面绑定）。
        fac = (m.get("pbrMetallicRoughness") or {}).get("baseColorFactor", [1.0, 1.0, 1.0, 1.0])
        mel.set_material_instance_vector_parameter_value(
            mic, "BaseColor", unreal.LinearColor(fac[0], fac[1], fac[2], 1.0)
        )
        # Emissive = 契约 Le 直给（向量参数;emissive 纹理逐像素面不消费——
        # 双端最大子集 = 均值 Le,残余口径差登记面）。
        le = em["le_linear_rgb"]
        mel.set_material_instance_vector_parameter_value(
            mic, "Emissive", unreal.LinearColor(le[0], le[1], le[2], 1.0)
        )
        rec["emissive_set_linear"] = [float(le[0]), float(le[1]), float(le[2])]
        # 读回核验（禁静默）。
        sv = mic.get_editor_property("scalar_parameter_values") or []
        vv = mic.get_editor_property("vector_parameter_values") or []
        rec["vector_params_nonnull"] = sum(1 for e in vv if getattr(e, "parameter_value", None) is not None)
        rec["scalar_params"] = len(sv)
        unreal.EditorAssetLibrary.save_asset(mic_path)
        probe.append(rec)
    log("bistro emissive 重挂完成: %d 材质（契约 Le 直给 + 读回核验）" % len(probe))
    return probe


# ---------------------------------------------------------------------------
# 关卡建设
# ---------------------------------------------------------------------------

def build_level(scene_id, contract):
    srow = scene_row(contract, scene_id)
    cam = srow["camera"]
    level_subsys = unreal.get_editor_subsystem(unreal.LevelEditorSubsystem)
    actor_subsys = unreal.get_editor_subsystem(unreal.EditorActorSubsystem)
    map_path = MAPS_ROOT + "/" + SCENE_MAP[scene_id]
    if unreal.EditorAssetLibrary.does_asset_exist(map_path):
        level_subsys.load_level(map_path)
        for a in unreal.EditorLevelLibrary.get_all_level_actors():
            actor_subsys.destroy_actor(a)
    else:
        level_subsys.new_level(map_path)

    fix = unreal.Transform(rotation=unreal.Rotator(pitch=0.0, yaw=90.0, roll=0.0))
    with open(SCENE_GLTF[scene_id], "r", encoding="utf-8") as f:
        gltf = json.load(f)
    nodes = gltf.get("nodes", [])
    mesh_ref = {}
    for n in nodes:
        if "mesh" in n:
            mesh_ref[n["mesh"]] = mesh_ref.get(n["mesh"], 0) + 1
    if any(c > 1 for c in mesh_ref.values()):
        raise RuntimeError("网格多引用不在消费面（语料实测应为零）")
    mesh_assets = {}
    mesh_dir = CONTENT_ROOT + "/" + scene_id
    for sub in unreal.EditorAssetLibrary.list_assets(mesh_dir, recursive=True):
        name = sub.split("/")[-1].split(".")[0]
        norm = "".join(ch for ch in name.lower() if ch.isalnum())
        mesh_assets[norm] = sub
    cornell_spawned = []
    n_spawned = 0
    for idx, node in enumerate(nodes):
        if "mesh" not in node:
            continue
        actor = actor_subsys.spawn_actor_from_class(
            unreal.StaticMeshActor, fix.translation, fix.rotation.rotator()
        )
        actor.set_actor_label("G12N_" + node.get("name", "node%d" % idx))
        actor.set_folder_path("G12/Nodes")
        norm = "".join(ch for ch in node.get("name", "").lower() if ch.isalnum())
        asset_path = mesh_assets.get(norm)
        if asset_path is None:
            raise RuntimeError("网格资产未匹配节点: " + node.get("name", "?"))
        smc = actor.static_mesh_component
        smc.set_static_mesh(unreal.EditorAssetLibrary.load_asset(asset_path))
        smc.set_mobility(unreal.ComponentMobility.STATIC)
        if scene_id == "cornell-box":
            prims = gltf.get("meshes", [])[node["mesh"]].get("primitives", [])
            cornell_spawned.append((actor, prims[0].get("material") if prims else None))
        n_spawned += 1
    log("节点 spawn: %d mesh actors" % n_spawned)
    if scene_id == "cornell-box":
        apply_two_sided_cornell(gltf, cornell_spawned)

    # ---- 契约相机 ----
    cam_loc = to_ue_pos(cam["position"])
    cam_q = to_ue_quat(cam["orientation_quat"])
    fov_h = math.degrees(2.0 * math.atan(math.tan(math.radians(cam["fov_y_deg"]) / 2.0)
                                        * (cam["resolution"]["w"] / cam["resolution"]["h"])))
    cam_actor = actor_subsys.spawn_actor_from_class(
        unreal.CameraActor, unreal.Vector(*cam_loc), unreal.Rotator(0, 0, 0)
    )
    cam_actor.set_actor_label("G12_ContractCamera")
    cam_actor.set_folder_path("G12")
    quat = unreal.Quat(cam_q[1], cam_q[2], cam_q[3], cam_q[0])  # unreal.Quat(x,y,z,w)
    cam_actor.set_actor_rotation(quat.rotator(), False)
    cam_comp = cam_actor.camera_component
    cam_comp.set_field_of_view(fov_h)
    cam_comp.set_constraint_aspect_ratio(False)
    cam_comp.set_editor_property("constrain_aspect_ratio", False)
    log("相机: loc_cm=%s fov_h=%.6f" % (str(cam_loc), fov_h))

    # ---- 契约灯面（sun/sky 不建——契约 0.0 显式登记）----
    lig = srow["lighting"]
    if lig["sun_intensity_lux"] != 0.0 or lig["sky_intensity"] != 0.0:
        raise RuntimeError("契约 sun/sky 非零越 PT 起步范围面")
    for p in lig["point_lights"]:
        loc = to_ue_pos(p["position"])
        la = actor_subsys.spawn_actor_from_class(
            unreal.PointLight, unreal.Vector(*loc), unreal.Rotator(0, 0, 0)
        )
        la.set_actor_label("G12_" + p["id"])
        la.set_folder_path("G12/Lights")
        pc = la.get_component_by_class(unreal.PointLightComponent)
        pc.set_intensity(float(p["intensity_cd"]))  # candela 直给
        col = p["color_linear_rgb"]
        pc.set_light_color(unreal.LinearColor(col[0], col[1], col[2], 1.0), False)
        pc.set_mobility(unreal.ComponentMobility.MOVABLE)
    if lig["point_lights"]:
        log("点光: %d 盏（cd 直给,光色线性 b_srgb=False）" % len(lig["point_lights"]))
    # quad 面光 = emissive plane（/Engine/BasicShapes/Plane 100×100cm +Z 面）。
    for iq, q in enumerate(lig["quad_lights"]):
        p00, e1, e2 = q["p00"], q["e1"], q["e2"]
        center = [p00[i] + (e1[i] + e2[i]) / 2.0 for i in range(3)]
        c_ue = to_ue_pos(center)
        e1_ue = to_ue_pos(e1)
        e2_ue = to_ue_pos(e2)
        len1 = math.sqrt(sum(x * x for x in e1_ue))
        len2 = math.sqrt(sum(x * x for x in e2_ue))
        ax1 = tuple(x / len1 for x in e1_ue)
        ax2 = tuple(x / len2 for x in e2_ue)
        # 平面法线 = ax1×ax2。映射 M=(−z,x,y)·100 为反射（det=−1）——映射后
        # 绕向叉积变号:M(e1)×M(e2) = −M(e1×e2);契约法线 −y → UE −Z 须以
        # −ax2 为 Y 轴翻转平面朝向（覆盖域关于中心对称不变;5.8.1 实证面:
        # 首跑法线 (0,0,+1) 被本校验截获）。
        nrm = (
            ax1[1] * ax2[2] - ax1[2] * ax2[1],
            ax1[2] * ax2[0] - ax1[0] * ax2[2],
            ax1[0] * ax2[1] - ax1[1] * ax2[0],
        )
        if nrm[2] > 0.0:
            # 反射映射面:翻转到朝下（−Z_ue）= 契约 −y 发光法线。
            ax2 = tuple(-x for x in ax2)
            nrm = (-nrm[0], -nrm[1], -nrm[2])
        rot = unreal.MathLibrary.make_rot_from_xy(
            unreal.Vector(*ax1), unreal.Vector(*ax2)
        )
        plane_mesh = unreal.EditorAssetLibrary.load_asset("/Engine/BasicShapes/Plane")
        pa = actor_subsys.spawn_actor_from_class(
            unreal.StaticMeshActor, unreal.Vector(*c_ue), rot
        )
        pa.set_actor_label("G12_QuadLight_%d" % iq)
        pa.set_folder_path("G12/Lights")
        pa.set_actor_scale3d(unreal.Vector(len1 / 100.0, len2 / 100.0, 1.0))
        psmc = pa.static_mesh_component
        psmc.set_static_mesh(plane_mesh)
        psmc.set_mobility(unreal.ComponentMobility.STATIC)
        parent = ensure_pt_emissive_parent()
        tools = unreal.AssetToolsHelpers.get_asset_tools()
        mic_path = CONTENT_ROOT + "/G12_QuadLightMI_%d" % iq
        if unreal.EditorAssetLibrary.does_asset_exist(mic_path):
            mic = unreal.EditorAssetLibrary.load_asset(mic_path)
        else:
            mic = tools.create_asset(
                "G12_QuadLightMI_%d" % iq, CONTENT_ROOT,
                unreal.MaterialInstanceConstant, unreal.MaterialInstanceConstantFactoryNew(),
            )
            unreal.MaterialEditingLibrary.set_material_instance_parent(mic, parent)
        le = q["le_linear_rgb"]
        mel = unreal.MaterialEditingLibrary
        mel.set_material_instance_vector_parameter_value(
            mic, "BaseColor", unreal.LinearColor(0.0, 0.0, 0.0, 1.0)
        )
        mel.set_material_instance_vector_parameter_value(
            mic, "Emissive", unreal.LinearColor(le[0], le[1], le[2], 1.0)
        )
        unreal.EditorAssetLibrary.save_asset(mic_path)
        for slot in range(psmc.get_num_materials()):
            psmc.set_material(slot, mic)
        if nrm[2] > 0.0:
            raise RuntimeError("quad 面光法线未朝下（绕向/映射断裂）: %s" % str(nrm))
        log("quad 面光: center_ue=%s scale=(%.3f,%.3f) Le=%s" % (str(c_ue), len1 / 100.0, len2 / 100.0, str(le)))

    # ---- 手动曝光 + PT 覆盖 PPV ----
    ev100 = float(srow["exposure"]["ev100"])
    n_fstop = 4.0
    shutter = (2.0 ** ev100) / (n_fstop * n_fstop)
    ppv = actor_subsys.spawn_actor_from_class(
        unreal.PostProcessVolume, unreal.Vector(0, 0, 0), unreal.Rotator(0, 0, 0)
    )
    ppv.set_actor_label("G12_Exposure_PT")
    ppv.set_folder_path("G12")
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
    pps.set_editor_property("override_bloom_intensity", True)
    pps.set_editor_property("bloom_intensity", 0.0)
    pps.set_editor_property("override_vignette_intensity", True)
    pps.set_editor_property("vignette_intensity", 0.0)
    pps.set_editor_property("override_motion_blur_amount", True)
    pps.set_editor_property("motion_blur_amount", 0.0)
    # PT 覆盖（双端同深度 + emissive 开;spp 由 MRQ AA 设置驱动不锁 PPV）。
    pps.set_editor_property("override_path_tracing_max_bounces", True)
    pps.set_editor_property("path_tracing_max_bounces", int(contract["max_bounces"]))
    pps.set_editor_property("override_path_tracing_enable_emissive_materials", True)
    pps.set_editor_property("path_tracing_enable_emissive_materials", True)
    ppv.set_editor_property("settings", pps)
    log("手动曝光 + PT 覆盖: ev100=%.4f bounces=%d emissive=on" % (ev100, int(contract["max_bounces"])))

    level_subsys.save_current_level()
    log("关卡保存: " + map_path)
    return map_path, cam_actor


# ---------------------------------------------------------------------------
# MRQ Phase A（逐 spp PrimaryConfig;PT 通道 + AA temporal samples = spp）
# ---------------------------------------------------------------------------

def build_mrq_assets(scene_id, contract, cam_actor, out_root):
    srow = scene_row(contract, scene_id)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    tag = scene_id.replace("-", "_")
    seq_path = CINE_ROOT + "/G12_%s_Seq" % tag
    if unreal.EditorAssetLibrary.does_asset_exist(seq_path):
        unreal.EditorAssetLibrary.delete_asset(seq_path)
    seq = tools.create_asset(
        "G12_%s_Seq" % tag, CINE_ROOT, unreal.LevelSequence, unreal.LevelSequenceFactoryNew()
    )
    seq.set_playback_start(0)
    seq.set_playback_end(4)
    binding = seq.add_possessable(cam_actor)
    cut_track = seq.add_track(unreal.MovieSceneCameraCutTrack)
    section = cut_track.add_section()
    section.set_start_frame(0)
    section.set_end_frame(4)
    section.set_camera_binding_id(seq.get_binding_id(binding))
    unreal.EditorAssetLibrary.save_asset(seq_path)
    res = srow["camera"]["resolution"]
    pol = contract["rendering_policy"]
    jobs = []
    for spp in contract["spp_sequence"]:
        cfg_name = "G12_%s_spp%d_Config" % (tag, spp)
        cfg_path = CINE_ROOT + "/" + cfg_name
        if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
            unreal.EditorAssetLibrary.delete_asset(cfg_path)
        cfg = tools.create_asset(
            cfg_name, CINE_ROOT,
            unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory(),
        )
        exr = cfg.find_or_add_setting_by_class(unreal.MoviePipelineImageSequenceOutput_EXR)
        exr.compression = unreal.EXRCompressionFormat.NONE
        color = cfg.find_or_add_setting_by_class(unreal.MoviePipelineColorSetting)
        color.set_editor_property("disable_tone_curve", True)
        # PT 渲染通道（5.8.1:MoviePipelineDeferredPass_PathTracer;采样数 =
        # AA temporal×spatial——MovieGraphPathTracerPass.cpp:39 语义面）。
        cfg.find_or_add_setting_by_class(unreal.MoviePipelineDeferredPass_PathTracer)
        out = cfg.find_or_add_setting_by_class(unreal.MoviePipelineOutputSetting)
        out.output_resolution = unreal.IntPoint(int(res["w"]), int(res["h"]))
        out.output_directory = unreal.DirectoryPath(
            out_root.rstrip("/") + "/" + scene_id + "/spp%d" % spp
        )
        out.use_custom_frame_rate = True
        out.output_frame_rate = unreal.FrameRate(30, 1)
        aa = cfg.find_or_add_setting_by_class(unreal.MoviePipelineAntiAliasingSetting)
        aa.engine_warm_up_count = 8
        aa.set_editor_property("temporal_sample_count", int(spp))
        aa.set_editor_property("spatial_sample_count", 1)
        aa.set_editor_property("override_anti_aliasing", True)
        cvs = cfg.find_or_add_setting_by_class(unreal.MoviePipelineConsoleVariableSetting)
        # RT 运行期使能（Dynamic 模式 r.RayTracing.EnableOnDemand=1 默认面:
        # r.RayTracing=1 项目设置 + 运行期 r.RayTracing.Enable=1 翻面——
        # 5.8.1 首跑实证 "Path Tracer is not enabled by this project" 截获面）。
        cvs.add_or_update_console_variable("r.RayTracing.Enable", 1.0)
        cvs.add_or_update_console_variable("r.PathTracing", 1.0)
        cvs.add_or_update_console_variable(
            "r.PathTracing.FilterWidth", float(pol["filter_width"])
        )
        cvs.add_or_update_console_variable(
            "r.PathTracing.MaxBounces", float(pol["max_bounces"])
        )
        cvs.add_or_update_console_variable("r.PathTracing.MISMode", float(pol["mis_mode"]))
        cvs.add_or_update_console_variable("r.MotionBlurQuality", 0.0)
        cvs.add_or_update_console_variable("r.BloomQuality", 0.0)
        cvs.add_or_update_console_variable("r.DepthOfFieldQuality", 0.0)
        cvs.add_or_update_console_variable("r.EyeAdaptation.PreExposureOverride", 0.0)
        unreal.EditorAssetLibrary.save_asset(cfg_path)
        jobs.append({"spp": spp, "config": cfg_path})
        log("config saved: %s spp=%d" % (cfg_path, spp))
    return seq_path, jobs


def main():
    scene_id = os.environ.get("G12_4_SCENE", "")
    contract_path = os.environ.get("G12_4_CONTRACT", "")
    skip_import = os.environ.get("G12_4_SKIP_IMPORT", "0") == "1"
    if scene_id not in SCENE_GLTF or not contract_path:
        raise RuntimeError("env G12_4_SCENE/G12_4_CONTRACT 必填")
    with open(contract_path, "r", encoding="utf-8") as f:
        contract = pt_contract.parse_contract(f.read())
    digest = pt_contract.contract_digest(contract)
    log("契约解析 OK（UE 侧解析器）: scene=%s digest=%s" % (scene_id, digest))

    probe = {"scene_id": scene_id, "contract_digest_ue": digest}
    if not skip_import:
        import_scene_gltf(scene_id)
    if scene_id == "bistro-interior":
        probe["texture_bound_materials"] = bind_bistro_textures(scene_id)
    map_path, cam_actor = build_level(scene_id, contract)
    if scene_id == "bistro-interior":
        probe["emissive_materials"] = setup_bistro_emissive(scene_id, scene_row(contract, scene_id))
    out_root = os.environ.get("G12_4_OUT_ROOT", "K:/rurix-ext/g12-frames/ue_pt")
    seq_path, jobs = build_mrq_assets(scene_id, contract, cam_actor, out_root)
    probe["map"] = map_path
    probe["seq"] = seq_path
    probe["jobs"] = jobs
    probe_out = os.environ.get("G12_4_PROBE_OUT", "")
    if probe_out:
        with open(probe_out, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(probe, ensure_ascii=False, indent=1) + "\n")
        log("探针落盘: " + probe_out)
    log("建设完成: " + scene_id)


main()
