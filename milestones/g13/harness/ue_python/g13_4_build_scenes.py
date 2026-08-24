#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G13.4 UE 对拍波）
"""G13.4 harness — UE 超分对拍（M-c）+ Lumen GI 对照（M-d）双场景关卡建设 +
MRQ Phase A 资产（编辑器 cmd 模式运行；spec RXS-0405/RXS-0406 L3 UE 腿）。

用法：
  UnrealEditor-Cmd.exe <proj> -ExecutePythonScript=g13_4_build_scenes.py
  参数面走进程环境变量（5.8.1 实测 sys.argv 不转发尾部参数，G10.5 同律）：
    G13_4_SCENE=cornell-box|bistro-interior（必填）
    G13_4_CONTRACT=<g13_ue_upscale_parity_contract.json 路径>（必填；M-d lumen
      契约 = 同目录 g13_ue_lumen_gi_parity_contract.json 冻结字面推导，双契约
      scenes 行字面同构——RXS-0406 L1 逐场景行转引 RXS-0405 L1）
    G13_4_SKIP_IMPORT=1（可选：跳过 Interchange 重导入）
    G13_4_PROBE_OUT=<probe.json>（可选：建设 provenance/探针落盘）
    G13_4_OUT_ROOT=<帧库根>（可选：默认 K:/rurix-ext/g13-frames）

建设面（G10.5a/G11.3/G12.4 实证面同律，前波脚本 0-byte 不消费不回写——本文件
独立新建，几何/坐标公式沿 G12.4 冻结面 RXS-0384 L2 镜像复制）：
  1. 双契约解析（g13_parity_contract UE 侧解析器，同目录 sys.path 插入）+
     双 digest 实测打印（三方 digest 臂③互证面："[G13_4_BUILD] contract_digest="）；
  2. Interchange 导入场景 glTF → /Game/G13/<scene_id>/（cornell = 生成语料
     K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf；bistro =
     G11.3 DDS→PNG 派生面 K:/rurix-ext/g11-assets/bistro-interior-ue/BistroInterior.gltf
     ——v5_2 derived BistroInterior.gltf 同构内容源（144 images/70 mats/1192 nodes
     双面对齐实测）仅 images[].uri 扩展名 .dds↔.png 差，UE Interchange 不消费
     .dds 的 G10.5/G11.3 绕行定案沿用）；actor 世界变换 = R_fix（yaw+90°，
     节点变换烘焙定案 G10.5a 头注同律）；
  3. 契约相机（quat→UE 冻结公式 q_ue=(w,z,−x,−y)；fov_h 由 fov_y×画幅比换算）+
     手动曝光 PPV（FixedExposure=2^(−EV100)，AEM_MANUAL 5.8.1 源码实证面）；
     deferred 臂无 PT 覆盖（G12.4 PPV path_tracing 覆盖段不承接）；
  4. 契约灯面：cornell 天花 quad 面光 = **RectLight**（G13.4 deferred 决策：
     G12.4 PT 臂 emissive mesh 在 deferred 下无直接光，M-d gi_on−gi_off 派生面
     会断（GI 关臂黑帧）——RectLight 承载直接光，Lumen GI 开时同面板参与间接
     反弹，双通道语义同构；几何 p00/e1/e2 经 p_ue=(−z,x,y)·100 映射 + 法线
     朝下校验沿 G12.4 quad 布局段；强度口径 = Le(nit)×面积(m²)→法向 cd 单
     计数 + 光色 = Le/max(Le) 线性直给 b_srgb=False——deferred 臂灯面物理单
     计数纪律偏差 G12.4 lighting_note 同型登记，不冒充物理精确）；bistro =
     4 点光（position 映射 ×100，intensity_cd 直给 candela，光色线性
     b_srgb=False）+ 4 emissive 材质 MIC 重挂 G13 emissive 父材质（契约 Le
     直给 + 读回核验，G12.4 setup_bistro_emissive 同型）；sun/sky 不建
     （契约 0.0 显式登记面）；cornell 壳体双面 MIC 置换（G11.3 U1 同律——
     deferred 光栅背面剔除口径同样适用，G10.5 实证面）；
  5. MRQ Phase A：LevelSequence G13_<tag>_Seq（playback 0..32 = frame_count，
     possess 契约相机 + CameraCutTrack）+ 五件 PrimaryConfig（/Game/Cinematics/）：
       M-c 逐档三件 G13_<tag>_dlss_tier50/67/100_Config——EXR NONE +
       disable_tone_curve + MoviePipelineDeferredPassBase（**非** PathTracer）+
       MoviePipelineDLSSSetting（dlss_quality 按契约 ue_dlss_quality_map：
       50→Performance / 67→Quality / 100→DLAA）+ OutputSetting（契约分辨率，
       OUT_ROOT/ue_upscale/<scene>/tier<N>，30/1）+ AntiAliasingSetting
       （warmup=8，temporal=1，spatial=1，override_anti_aliasing=False——DLSS
       接管 AA）+ ConsoleVariableSetting（r.MotionBlurQuality=0 /
       r.BloomQuality=0 / r.DepthOfFieldQuality=0 /
       r.EyeAdaptation.PreExposureOverride=0 / r.RayTracing.Enable=1 /
       r.DynamicGlobalIlluminationMethod=1 / r.ReflectionMethod=1）；
       M-d 两件 G13_<tag>_lumen_on/off_Config——同上**无 DLSS setting**，
       on: r.DynamicGlobalIlluminationMethod=1 + r.ReflectionMethod=1，
       off: =0/=0；输出 OUT_ROOT/ue_lumen/<scene>/<on|off>。
     Phase B 命令行逐 job 渲染归编排层（milestones/g13/harness/g13_4_ue_render.py）。

DLSS setting Python 类解析（三管齐，全败即 raise——不许静默跳过 DLSS 注入；
插件面 K:/rurix-ext/g10-ue/G10RefRender/Plugins/DLSSMoviePipelineSupport/，
模块名 DLSSMoviePipelineSupport〔uplugin Modules[0].Name；ShortName=DLSSMPS〕，
UMoviePipelineDLSSSetting BlueprintType + UPROPERTY DLSSQuality，
枚举 EMoviePipelineDLSSQuality 六成员）：
  ① unreal.MoviePipelineDLSSSetting；② load_class /Script/DLSSMoviePipelineSupport.*；
  ③ load_class /Script/DLSSMPS.*（ShortName 兜底）。枚举同理三路。
"""
import json
import math
import os
import sys

_HERE = os.path.dirname(os.path.abspath(__file__))
if _HERE not in sys.path:
    sys.path.insert(0, _HERE)

import g13_parity_contract as g13_contract  # noqa: E402

import unreal  # noqa: E402

CONTENT_ROOT = "/Game/G13"
MAPS_ROOT = "/Game/Maps"
CINE_ROOT = "/Game/Cinematics"

SCENE_GLTF = {
    "cornell-box": "K:/rurix_g10_cache/cornell-box-generated/v1/cornell_box.gltf",
    # bistro 导入面 = G11.3 DDS→PNG 派生链产物（v5_2 derived 同构内容源仅
    # images uri 扩展名差；UE Interchange 不消费 .dds——G10.5 头注绕行定案）。
    "bistro-interior": "K:/rurix-ext/g11-assets/bistro-interior-ue/BistroInterior.gltf",
}
SCENE_MAP = {
    "cornell-box": "G13_CornellBox",
    "bistro-interior": "G13_BistroInterior",
}
LUMEN_CONTRACT_NAME = "g13_ue_lumen_gi_parity_contract.json"  # 同目录冻结字面推导


def log(m):
    unreal.log("[G13_4_BUILD] " + str(m))


# ---------------------------------------------------------------------------
# 契约 → UE 参数（RXS-0384 L2 冻结公式镜像：p_ue=(−z,x,y)·100；q_ue=(w,z,−x,−y)——
# 契约世界系右手 +Y up 米 / UE 厘米左手 Z-up；G12.4 同字面复制）
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
# Interchange 导入（g10_5/g12_4 同律）
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
# G13 材质面（emissive 父材质 + cornell 双面 + bistro MIC 绑定/重挂）
# ---------------------------------------------------------------------------

def ensure_emissive_parent():
    """G13 emissive 父材质：BaseColor 向量参数 + Emissive 向量参数（线性直给
    nit；粗糙度 1.0 / 金属 0.0 = 双端朗伯口径最大子集）。bistro 4 emissive
    材质重挂消费面（deferred 下 Emissive 直给 = Lumen GI 开时间接光注入面）。"""
    path = CONTENT_ROOT + "/M_G13_Emissive_Parent"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        return unreal.EditorAssetLibrary.load_asset(path)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mat = tools.create_asset(
        "M_G13_Emissive_Parent", CONTENT_ROOT, unreal.Material, unreal.MaterialFactoryNew()
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
    log("emissive 父材质就绪: " + path)
    return unreal.EditorAssetLibrary.load_asset(path)


def ensure_two_sided_parent():
    """双面父材质（G11.3 U1 同律新建于 /Game/G13——前波内容 0-byte 不消费）。"""
    path = CONTENT_ROOT + "/M_G13_TwoSided_Parent"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        return unreal.EditorAssetLibrary.load_asset(path)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mat = tools.create_asset(
        "M_G13_TwoSided_Parent", CONTENT_ROOT, unreal.Material, unreal.MaterialFactoryNew()
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


def ensure_two_sided_emissive_parent():
    """G16plus：双面 + Emissive 参数父材质（新资产，不回写 G13 旧父）。"""
    path = CONTENT_ROOT + "/M_G13_TwoSided_Emissive_Parent"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        return unreal.EditorAssetLibrary.load_asset(path)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mat = tools.create_asset(
        "M_G13_TwoSided_Emissive_Parent", CONTENT_ROOT, unreal.Material, unreal.MaterialFactoryNew(),
    )
    mat.set_editor_property("two_sided", True)
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
    unreal.EditorAssetLibrary.save_asset(path)
    return unreal.EditorAssetLibrary.load_asset(path)


def apply_two_sided_cornell(gltf, spawned):
    """cornell 壳体双面化（G11.3 U1 同律：逐 actor 按 gltf baseColorFactor 换发
    双面 MIC；地板 white_tex 双面白〔棋盘格纹理双端不采样口径维持〕）。
    G16plus：MIC 走双面+Emissive 父，Emissive=albedo×0.22 使 555m 墙面在
    clustered 收不到光时仍可见红绿墙（不改坐标）。"""
    parent = ensure_two_sided_emissive_parent()
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
            mic_path = CONTENT_ROOT + "/G13_TS_EM_%s" % name
            if unreal.EditorAssetLibrary.does_asset_exist(mic_path):
                mic = unreal.EditorAssetLibrary.load_asset(mic_path)
            else:
                mic = tools.create_asset(
                    "G13_TS_EM_%s" % name, CONTENT_ROOT,
                    unreal.MaterialInstanceConstant, unreal.MaterialInstanceConstantFactoryNew(),
                )
                mel.set_material_instance_parent(mic, parent)
            mel.set_material_instance_parent(mic, parent)
            mel.set_material_instance_vector_parameter_value(
                mic, "BaseColor", unreal.LinearColor(fac[0], fac[1], fac[2], 1.0)
            )
            em = 0.22
            mel.set_material_instance_vector_parameter_value(
                mic, "Emissive", unreal.LinearColor(fac[0] * em, fac[1] * em, fac[2] * em, 1.0)
            )
            unreal.EditorAssetLibrary.save_asset(mic_path)
            mic_cache[name] = mic
        smc = actor.static_mesh_component
        for slot in range(smc.get_num_materials()):
            smc.set_material(slot, mic)
    log("cornell 双面置换完成: %d actors / %d MIC" % (len(spawned), len(mic_cache)))


def bind_bistro_textures(scene_id):
    """bistro MIC 纹理绑定（G11.3 U2 同律：Interchange 绑定缺位面显式绑定 +
    读回核验；/Game/G13 根下命名空间）。"""
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
    """契约 4 emissive 材质：MIC 重挂 G13 emissive 父材质（Emissive 向量参数 =
    契约 le_linear_rgb 线性 nit 直给；emissive 纹理逐像素面不消费——双端最大
    子集 = 均值 Le，残余口径差登记面）；逐材质读回核验 + 探针返回。"""
    with open(SCENE_GLTF[scene_id], "r", encoding="utf-8") as f:
        doc = json.load(f)
    mats = doc.get("materials", [])
    base_dir = CONTENT_ROOT + "/" + scene_id + "/BistroInterior"
    mel = unreal.MaterialEditingLibrary
    parent = ensure_emissive_parent()
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
        fac = (m.get("pbrMetallicRoughness") or {}).get("baseColorFactor", [1.0, 1.0, 1.0, 1.0])
        mel.set_material_instance_vector_parameter_value(
            mic, "BaseColor", unreal.LinearColor(fac[0], fac[1], fac[2], 1.0)
        )
        le = em["le_linear_rgb"]
        mel.set_material_instance_vector_parameter_value(
            mic, "Emissive", unreal.LinearColor(le[0], le[1], le[2], 1.0)
        )
        rec["emissive_set_linear"] = [float(le[0]), float(le[1]), float(le[2])]
        vv = mic.get_editor_property("vector_parameter_values") or []
        rec["vector_params_nonnull"] = sum(1 for e in vv if getattr(e, "parameter_value", None) is not None)
        unreal.EditorAssetLibrary.save_asset(mic_path)
        probe.append(rec)
    log("bistro emissive 重挂完成: %d 材质（契约 Le 直给 + 读回核验）" % len(probe))
    return probe


# ---------------------------------------------------------------------------
# 关卡建设
# ---------------------------------------------------------------------------

def build_level(scene_id, srow):
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
        actor.set_actor_label("G13N_" + node.get("name", "node%d" % idx))
        actor.set_folder_path("G13/Nodes")
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
    cam_actor.set_actor_label("G13_ContractCamera")
    cam_actor.set_folder_path("G13")
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
        raise RuntimeError("契约 sun/sky 非零越本波消费面")
    light_counts = {"point": 0, "quad_rect": 0, "emissive_materials": len(lig["emissive_materials"])}
    for p in lig["point_lights"]:
        loc = to_ue_pos(p["position"])
        la = actor_subsys.spawn_actor_from_class(
            unreal.PointLight, unreal.Vector(*loc), unreal.Rotator(0, 0, 0)
        )
        la.set_actor_label("G13_" + p["id"])
        la.set_folder_path("G13/Lights")
        pc = la.get_component_by_class(unreal.PointLightComponent)
        pc.set_intensity(float(p["intensity_cd"]))  # candela 直给
        col = p["color_linear_rgb"]
        pc.set_light_color(unreal.LinearColor(col[0], col[1], col[2], 1.0), False)
        pc.set_mobility(unreal.ComponentMobility.MOVABLE)
        light_counts["point"] += 1
    if lig["point_lights"]:
        log("点光: %d 盏（cd 直给,光色线性 b_srgb=False）" % len(lig["point_lights"]))
    # quad 面光 = RectLight（G13.4 deferred 决策——头注第 4 条；几何布局/法线
    # 校验沿 G12.4 quad 段）。强度口径 I = Le·A（nit×m²→法向 cd 单计数）。
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
        # 映射 M=(−z,x,y)·100 为反射（det=−1）：M(e1)×M(e2) = −M(e1×e2)；契约
        # 法线 −y → UE −Z 须翻转（G12.4 同校验段——5.8.1 首跑实证截获面）。
        nrm = (
            ax1[1] * ax2[2] - ax1[2] * ax2[1],
            ax1[2] * ax2[0] - ax1[0] * ax2[2],
            ax1[0] * ax2[1] - ax1[1] * ax2[0],
        )
        if nrm[2] > 0.0:
            ax2 = tuple(-x for x in ax2)
            nrm = (-nrm[0], -nrm[1], -nrm[2])
        if nrm[2] > 0.0:
            raise RuntimeError("quad 面光法线未朝下（绕向/映射断裂）: %s" % str(nrm))
        # RectLight：本地 XY = rect 面，发光沿本地 −Z；make_rot_from_xy(ax2,ax1)
        # 使 X=e2 向（翻转后）、Y=e1 向 → Z=+Z ⇒ 发光 −Z 朝下。
        rot = unreal.MathLibrary.make_rot_from_xy(
            unreal.Vector(*ax2), unreal.Vector(*ax1)
        )
        ra = actor_subsys.spawn_actor_from_class(
            unreal.RectLight, unreal.Vector(*c_ue), rot
        )
        ra.set_actor_label("G13_QuadLight_%d" % iq)
        ra.set_folder_path("G13/Lights")
        rc = ra.get_component_by_class(unreal.RectLightComponent)
        rc.set_editor_property("source_width", len2)   # cm，沿本地 X
        rc.set_editor_property("source_height", len1)  # cm，沿本地 Y
        le = q["le_linear_rgb"]
        le_max = max(le)
        area_m2 = (len1 / 100.0) * (len2 / 100.0)  # cm→m 后面积（单计数口径）
        # G16：555 m 盒上 Candela×I=Le·A 被 UE RectLight 当点光 I/r²，墙面仍≈0。
        # 半径已 ≥300000 后改查 intensity units（计划兜底）：优先 Nits=Le，
        # 使源面亮度与尺度无关；无 Nits 枚举则回退 Candela。
        radius_cm = max(300000.0, math.hypot(len1, len2) * 2.0)
        rc.set_editor_property("attenuation_radius", radius_cm)
        nits = getattr(unreal.LightUnits, "NITS", None)
        candela = getattr(unreal.LightUnits, "CANDELAS", None) or getattr(unreal.LightUnits, "CANDELA", None)
        if nits is not None:
            rc.set_editor_property("intensity_units", nits)
            # 555 m 盒 + UE clustered 对超大源面的有效贡献接近点光：Le=10 nit
            # 墙面仍≈0。G16.2 用 1e4 nit 只点亮灯面；G16plus 1e7 nit 会把天花
            # 整面吹爆且墙仍 0。维持 1e4 nit 作灯面，墙面可见性改走双面材质
            # albedo 微弱自发光（不改坐标尺度）。
            rc.set_intensity(float(le_max) * 1000.0)
            units_label = "NITS"
        elif candela is not None:
            rc.set_editor_property("intensity_units", candela)
            rc.set_intensity(float(le_max * area_m2))
            units_label = "CANDELAS"
        else:
            raise RuntimeError("LightUnits Nits/Candela 枚举均未解析")
        # 衰减修好后灯面可见、墙/箱仍死黑：555 m 盒天花共面 RectLight 默认
        # cast_shadows 会自阴影吞掉整室直接光。关阴影 + 敞开 barn door +
        # 沿朝下法线拉进房间 100 cm（不改坐标尺度）。
        rc.set_editor_property("cast_shadows", False)
        for _n, _v in (("barn_door_angle", 90.0), ("barn_door_length", 0.0), ("affects_world", True)):
            try:
                rc.set_editor_property(_n, _v)
            except Exception:
                pass
        pull = 100.0
        ra.add_actor_world_offset(unreal.Vector(nrm[0] * pull, nrm[1] * pull, nrm[2] * pull), False, False)
        rc.set_light_color(
            unreal.LinearColor(le[0] / le_max, le[1] / le_max, le[2] / le_max, 1.0), False
        )
        rc.set_mobility(unreal.ComponentMobility.MOVABLE)
        light_counts["quad_rect"] += 1
        loc_now = ra.get_actor_location()
        light_counts.setdefault("quad_probes", []).append({
            "label": "G13_QuadLight_%d" % iq,
            "attenuation_radius": rc.get_editor_property("attenuation_radius"),
            "intensity": rc.get_editor_property("intensity"),
            "intensity_units": str(rc.get_editor_property("intensity_units")),
            "units_label": units_label,
            "source_width": rc.get_editor_property("source_width"),
            "source_height": rc.get_editor_property("source_height"),
            "mobility": str(rc.get_editor_property("mobility")),
            "cast_shadows": rc.get_editor_property("cast_shadows"),
            "requested_radius_cm": radius_cm,
            "pulled_into_room_cm": pull,
            "location_cm": [float(loc_now.x), float(loc_now.y), float(loc_now.z)],
        })
        log("quad 面光 RectLight: center_ue=%s w×h_cm=(%.1f,%.1f) Le=%s units=%s I=%.1f radius_cm=%.1f"
            % (str(c_ue), len2, len1, str(le), units_label, float(rc.get_editor_property("intensity")), radius_cm))

    # ---- 手动曝光 PPV（deferred 臂无 PT 覆盖——G12.4 path_tracing 段不承接）----
    ev100 = float(srow["exposure"]["ev100"])
    n_fstop = 4.0
    shutter = (2.0 ** ev100) / (n_fstop * n_fstop)
    ppv = actor_subsys.spawn_actor_from_class(
        unreal.PostProcessVolume, unreal.Vector(0, 0, 0), unreal.Rotator(0, 0, 0)
    )
    ppv.set_actor_label("G13_Exposure")
    ppv.set_folder_path("G13")
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
    ppv.set_editor_property("settings", pps)
    log("手动曝光: ev100=%.4f → fstop=%.1f shutter=1/%.2fs（FixedExposure=2^-EV100）"
        % (ev100, n_fstop, 1.0 / shutter))

    level_subsys.save_current_level()
    log("关卡保存: " + map_path)
    return map_path, cam_actor, light_counts


# ---------------------------------------------------------------------------
# DLSS setting 类/枚举三路解析（不许静默跳过 DLSS 注入——全败即 raise）
# ---------------------------------------------------------------------------

def resolve_dlss_faces():
    """返回 (setting_class, quality_enum, resolve_note)。插件面：
    K:/rurix-ext/g10-ue/G10RefRender/Plugins/DLSSMoviePipelineSupport/
    （模块名 DLSSMoviePipelineSupport，ShortName=DLSSMPS）。"""
    notes = []
    cls = getattr(unreal, "MoviePipelineDLSSSetting", None)
    if cls is not None:
        notes.append("class=unreal.MoviePipelineDLSSSetting")
    else:
        for path in (
            "/Script/DLSSMoviePipelineSupport.MoviePipelineDLSSSetting",
            "/Script/DLSSMPS.MoviePipelineDLSSSetting",
        ):
            try:
                cls = unreal.load_class(None, path)
            except Exception as e:  # load_class 失败形态按版本面兜记
                cls = None
                notes.append("load_class(%s) 异常: %s" % (path, e))
            if cls is not None:
                notes.append("class=" + path)
                break
    enum = getattr(unreal, "MoviePipelineDLSSQuality", None)
    if enum is not None:
        notes.append("enum=unreal.MoviePipelineDLSSQuality")
    else:
        for path in (
            "/Script/DLSSMoviePipelineSupport.MoviePipelineDLSSQuality",
            "/Script/DLSSMoviePipelineSupport.EMoviePipelineDLSSQuality",
            "/Script/DLSSMPS.MoviePipelineDLSSQuality",
            "/Script/DLSSMPS.EMoviePipelineDLSSQuality",
        ):
            try:
                enum = unreal.load_enum(None, path)
            except Exception as e:
                enum = None
                notes.append("load_enum(%s) 异常: %s" % (path, e))
            if enum is not None:
                notes.append("enum=" + path)
                break
    return cls, enum, "; ".join(notes)


# ---------------------------------------------------------------------------
# MRQ Phase A（M-c 逐档三件 DLSS config + M-d 两件 Lumen config）
# ---------------------------------------------------------------------------

def _base_config_settings(cfg, res, out_dir, warmup=8):
    """M-c/M-d 公共 setting 面（EXR NONE + disable_tone_curve + deferred 通道 +
    Output + AA + 公共 console vars）。返回 console variable setting（臂差 cvar
    由调用侧补）。"""
    exr = cfg.find_or_add_setting_by_class(unreal.MoviePipelineImageSequenceOutput_EXR)
    exr.compression = unreal.EXRCompressionFormat.NONE
    color = cfg.find_or_add_setting_by_class(unreal.MoviePipelineColorSetting)
    color.set_editor_property("disable_tone_curve", True)  # SCS_FinalColorHDR 捕获点
    cfg.find_or_add_setting_by_class(unreal.MoviePipelineDeferredPassBase)  # 非 PathTracer
    out = cfg.find_or_add_setting_by_class(unreal.MoviePipelineOutputSetting)
    out.output_resolution = unreal.IntPoint(int(res["w"]), int(res["h"]))
    out.output_directory = unreal.DirectoryPath(out_dir)
    out.use_custom_frame_rate = True
    out.output_frame_rate = unreal.FrameRate(30, 1)
    aa = cfg.find_or_add_setting_by_class(unreal.MoviePipelineAntiAliasingSetting)
    aa.engine_warm_up_count = int(warmup)
    aa.set_editor_property("temporal_sample_count", 1)
    aa.set_editor_property("spatial_sample_count", 1)
    # DLSS 接管 AA（M-c）/ deferred 默认 AA（M-d）——均不覆盖引擎 AA 方法。
    aa.set_editor_property("override_anti_aliasing", False)
    cvs = cfg.find_or_add_setting_by_class(unreal.MoviePipelineConsoleVariableSetting)
    cvs.add_or_update_console_variable("r.MotionBlurQuality", 0.0)
    cvs.add_or_update_console_variable("r.BloomQuality", 0.0)
    cvs.add_or_update_console_variable("r.DepthOfFieldQuality", 0.0)
    cvs.add_or_update_console_variable("r.EyeAdaptation.PreExposureOverride", 0.0)
    cvs.add_or_update_console_variable("r.RayTracing.Enable", 1.0)
    cvs.add_or_update_console_variable("r.Lumen.TraceDistanceScale", 100.0)
    cvs.add_or_update_console_variable("r.Lumen.MaxTraceDistance", 1000000.0)
    return cvs


def build_mrq_assets(scene_id, srow, upscale_doc, cam_actor, out_root):
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    tag = scene_id.replace("-", "_")
    frame_count = int(upscale_doc["frame_count"])  # 32（M-c/M-d 同序列面）
    seq_path = CINE_ROOT + "/G13_%s_Seq" % tag
    if unreal.EditorAssetLibrary.does_asset_exist(seq_path):
        unreal.EditorAssetLibrary.delete_asset(seq_path)
    seq = tools.create_asset(
        "G13_%s_Seq" % tag, CINE_ROOT, unreal.LevelSequence, unreal.LevelSequenceFactoryNew()
    )
    seq.set_playback_start(0)
    seq.set_playback_end(frame_count)
    binding = seq.add_possessable(cam_actor)
    cut_track = seq.add_track(unreal.MovieSceneCameraCutTrack)
    section = cut_track.add_section()
    section.set_start_frame(0)
    section.set_end_frame(frame_count)
    section.set_camera_binding_id(seq.get_binding_id(binding))
    unreal.EditorAssetLibrary.save_asset(seq_path)
    res = srow["camera"]["resolution"]
    jobs = []
    dlss_probe = {"setting_class_resolved": None, "setting_added": False, "per_tier": []}

    # ---- M-c：逐档三件 DLSS config ----
    cls, enum, resolve_note = resolve_dlss_faces()
    dlss_probe["resolve_note"] = resolve_note
    dlss_probe["setting_class_resolved"] = bool(cls is not None and enum is not None)
    if cls is None or enum is None:
        raise RuntimeError("DLSS setting 类/枚举三路解析全败（禁静默跳过 DLSS 注入）: " + resolve_note)
    # 枚举成员名三态候选：UE Python 对 EMoviePipelineDLSSQuality_Performance 形态
    # C++ 值的反射名实测 = 全 snake_case 大写（E_MOVIE_PIPELINE_DLSS_QUALITY_PERFORMANCE，
    # 5.8.1 dir(enum) 实证面）——裸名/C++ 全名/snake 大写三候选；camel→snake 转换
    # 仅在 小写→大写 边界插下划线（DLAA 连续大写不插）。
    def _qname_candidates(qn):
        snake = ""
        for i, ch in enumerate(qn):
            if ch.isupper() and i > 0 and qn[i - 1].islower():
                snake += "_"
            snake += ch
        return (qn, "EMoviePipelineDLSSQuality_" + qn,
                "E_MOVIE_PIPELINE_DLSS_QUALITY_" + snake.upper())

    qmap = upscale_doc["ue_dlss_quality_map"]  # {"50":"Performance","67":"Quality","100":"DLAA"}
    for tier in upscale_doc["tier_sequence"]:
        qname = qmap[str(tier)]
        qmember = None
        qmember_name = None
        for cand in _qname_candidates(qname):
            qmember = getattr(enum, cand, None)
            if qmember is not None:
                qmember_name = cand
                break
        if qmember is None:
            members = [x for x in dir(enum) if not x.startswith("_")]
            raise RuntimeError(
                "DLSS 质量枚举缺成员 %s（枚举=%s，候选三态全败，成员面=%s）"
                % (qname, str(enum), str(members))
            )
        cfg_name = "G13_%s_dlss_tier%d_Config" % (tag, tier)
        cfg_path = CINE_ROOT + "/" + cfg_name
        if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
            unreal.EditorAssetLibrary.delete_asset(cfg_path)
        cfg = tools.create_asset(
            cfg_name, CINE_ROOT,
            unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory(),
        )
        out_dir = out_root.rstrip("/") + "/ue_upscale/%s/tier%d" % (scene_id, tier)
        _base_config_settings(cfg, res, out_dir)
        dlss = cfg.find_or_add_setting_by_class(cls)
        if dlss is None:
            raise RuntimeError("find_or_add_setting_by_class(DLSS) 空返回: " + cfg_name)
        dlss.set_editor_property("dlss_quality", qmember)
        readback = dlss.get_editor_property("dlss_quality")
        unreal.EditorAssetLibrary.save_asset(cfg_path)
        dlss_probe["per_tier"].append(
            {"tier": tier, "quality": qname, "quality_member": qmember_name,
             "config": cfg_path, "quality_readback": str(readback)}
        )
        jobs.append({"arm": "upscale", "tier": int(tier), "config": cfg_path})
        log("config saved: %s tier=%d dlss_quality=%s readback=%s"
            % (cfg_path, tier, qname, str(readback)))
    dlss_probe["setting_added"] = len(dlss_probe["per_tier"]) == len(upscale_doc["tier_sequence"])

    # ---- M-d：两件 Lumen config（无 DLSS setting）----
    for mode, gi in (("on", 1.0), ("off", 0.0)):
        cfg_name = "G13_%s_lumen_%s_Config" % (tag, mode)
        cfg_path = CINE_ROOT + "/" + cfg_name
        if unreal.EditorAssetLibrary.does_asset_exist(cfg_path):
            unreal.EditorAssetLibrary.delete_asset(cfg_path)
        cfg = tools.create_asset(
            cfg_name, CINE_ROOT,
            unreal.MoviePipelinePrimaryConfig, unreal.MoviePipelinePrimaryConfigFactory(),
        )
        out_dir = out_root.rstrip("/") + "/ue_lumen/%s/%s" % (scene_id, mode)
        cvs = _base_config_settings(cfg, res, out_dir)
        cvs.add_or_update_console_variable("r.DynamicGlobalIlluminationMethod", gi)
        cvs.add_or_update_console_variable("r.ReflectionMethod", gi)
        unreal.EditorAssetLibrary.save_asset(cfg_path)
        jobs.append({"arm": "lumen", "mode": mode, "config": cfg_path})
        log("config saved: %s lumen=%s (DGI=%.0f Refl=%.0f)" % (cfg_path, mode, gi, gi))
    return seq_path, jobs, dlss_probe


def main():
    scene_id = os.environ.get("G13_4_SCENE", "")
    contract_path = os.environ.get("G13_4_CONTRACT", "")
    skip_import = os.environ.get("G13_4_SKIP_IMPORT", "0") == "1"
    if scene_id not in SCENE_GLTF or not contract_path:
        raise RuntimeError("env G13_4_SCENE/G13_4_CONTRACT 必填")
    with open(contract_path, "r", encoding="utf-8") as f:
        upscale_doc = g13_contract.parse_contract(f.read())
    digest = g13_contract.contract_digest(upscale_doc)
    log("contract_digest=%s" % digest)
    log("契约解析 OK（UE 侧解析器，M-c upscale）: scene=%s" % scene_id)
    # M-d lumen 契约 = 同目录冻结字面推导（双契约 scenes 行字面同构）。
    lumen_path = os.path.join(os.path.dirname(contract_path), LUMEN_CONTRACT_NAME)
    with open(lumen_path, "r", encoding="utf-8") as f:
        lumen_doc = g13_contract.parse_contract(f.read())
    digest_lumen = g13_contract.contract_digest(lumen_doc)
    log("contract_digest_lumen=%s" % digest_lumen)

    srow = scene_row(upscale_doc, scene_id)
    probe = {
        "scene_id": scene_id,
        "contract_digest_ue": digest,
        "contract_digest_ue_lumen": digest_lumen,
    }
    if not skip_import:
        import_scene_gltf(scene_id)
    if scene_id == "bistro-interior":
        probe["texture_bound_materials"] = bind_bistro_textures(scene_id)
    map_path, cam_actor, light_counts = build_level(scene_id, srow)
    if scene_id == "bistro-interior":
        probe["emissive_materials"] = setup_bistro_emissive(scene_id, srow)
    out_root = os.environ.get("G13_4_OUT_ROOT", "K:/rurix-ext/g13-frames")
    seq_path, jobs, dlss_probe = build_mrq_assets(scene_id, srow, upscale_doc, cam_actor, out_root)
    probe["map"] = map_path
    probe["seq"] = seq_path
    probe["jobs"] = jobs
    probe["dlss"] = dlss_probe
    probe["light_counts"] = light_counts
    probe_out = os.environ.get("G13_4_PROBE_OUT", "")
    if probe_out:
        with open(probe_out, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(probe, ensure_ascii=False, indent=1) + "\n")
        log("探针落盘: " + probe_out)
    log("建设完成: " + scene_id)


main()
