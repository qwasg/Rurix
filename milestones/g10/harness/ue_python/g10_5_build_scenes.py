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
    # G11.3 U2 修复面：bistro 导入面 = G11.3 派生链转码产物（DDS→PNG，UE
    # Interchange 不消费 .dds 的绕行面——G10-N7 承接锚兑现；派生 gltf 仅
    # images[].uri 扩展名改写，buffer.bin 逐字节复制，G10 语料 0-byte 只读；
    # 产物登记 milestones/g11/g11_3_dds_transcode_manifest.json）。
    "bistro-interior": "K:/rurix-ext/g11-assets/bistro-interior-ue/BistroInterior.gltf",
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

def ensure_two_sided_parent():
    """G11.3 U1 修复面：双面父材质（壳体内表面参与光照——语料单面片外向绕向
    × UE 背面剔除口径的 harness 侧对齐；颜色经 BaseColor 向量参数走 MIC）。
    粗糙度 1.0 / 金属 0.0 = cornell 语料 pbr 因子同值（双端朗伯口径对齐）。"""
    path = CONTENT_ROOT + "/M_G11_TwoSided_Parent"
    if unreal.EditorAssetLibrary.does_asset_exist(path):
        return unreal.EditorAssetLibrary.load_asset(path)
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mat = tools.create_asset(
        "M_G11_TwoSided_Parent", CONTENT_ROOT, unreal.Material, unreal.MaterialFactoryNew()
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
    # 读回核验（双面属性未生效即报错不静默）
    chk = unreal.EditorAssetLibrary.load_asset(path)
    if not chk.get_editor_property("two_sided"):
        raise RuntimeError("双面父材质 two_sided 读回失败")
    log("双面父材质就绪: %s（two_sided=True 读回核验）" % path)
    return chk


def apply_two_sided_cornell(gltf, spawned):
    """cornell 壳体（墙/顶/地板）双面化：逐 actor 以 gltf 材质 baseColorFactor
    换发双面 MIC（父材质 two_sided=True）；地板 white_tex 按其 factor [1,1,1]
    双面白材质（棋盘格纹理双端不采样口径维持——Rurix 侧 PNG 容器显式登记
    不消费面，G10.5a 降级登记面演进）。返回逐 actor 置换 provenance 列表。"""
    parent = ensure_two_sided_parent()
    mel = unreal.MaterialEditingLibrary
    tools = unreal.AssetToolsHelpers.get_asset_tools()
    mats = gltf.get("materials", [])
    mic_cache = {}
    prov = []
    for actor, mat_idx in spawned:
        if mat_idx is None or mat_idx >= len(mats):
            raise RuntimeError("cornell 图元材质索引缺失（置换映射断裂）: %s" % mat_idx)
        m = mats[mat_idx]
        name = m.get("name", "mat%d" % mat_idx)
        fac = m.get("pbrMetallicRoughness", {}).get("baseColorFactor", [1.0, 1.0, 1.0, 1.0])
        key = name
        mic = mic_cache.get(key)
        if mic is None:
            mic_path = CONTENT_ROOT + "/G11_TS_%s" % name
            if unreal.EditorAssetLibrary.does_asset_exist(mic_path):
                mic = unreal.EditorAssetLibrary.load_asset(mic_path)
            else:
                mic = tools.create_asset(
                    "G11_TS_%s" % name, CONTENT_ROOT,
                    unreal.MaterialInstanceConstant, unreal.MaterialInstanceConstantFactoryNew(),
                )
                mel.set_material_instance_parent(mic, parent)
            mel.set_material_instance_vector_parameter_value(
                mic, "BaseColor", unreal.LinearColor(fac[0], fac[1], fac[2], 1.0)
            )
            unreal.EditorAssetLibrary.save_asset(mic_path)
            mic_cache[key] = mic
        smc = actor.static_mesh_component
        for slot in range(smc.get_num_materials()):
            smc.set_material(slot, mic)
        prov.append({
            "actor": actor.get_actor_label(),
            "material": name,
            "base_color_factor": [float(fac[0]), float(fac[1]), float(fac[2])],
            "two_sided_readback": bool(parent.get_editor_property("two_sided")),
        })
    log("cornell 双面置换完成: %d actors / %d MIC（壳体内表面参与光照）" % (len(prov), len(mic_cache)))
    return prov


def bind_bistro_material_textures(scene_id):
    """G11.3 U2 修复面：MIC 纹理参数显式绑定（UE 5.8.1 实测：派生链 PNG 纹理
    资产导入成功但 Interchange 建成的 70 个 MIC texture_parameter_values 全空——
    父材质九参数面〔BaseColorTexture/NormalTexture/…〕在树而绑定缺位；本函数按
    派生 gltf 材质→纹理映射显式绑定 + 读回核验，禁静默）。返回 provenance 块。"""
    with open(SCENE_GLTF[scene_id], "r", encoding="utf-8") as f:
        doc = json.load(f)
    images = doc.get("images", [])
    textures = doc.get("textures", [])
    mats = doc.get("materials", [])
    base_dir = CONTENT_ROOT + "/" + scene_id + "/BistroInterior"
    mel = unreal.MaterialEditingLibrary
    bound = []
    problems = []
    for m in mats:
        name = m.get("name")
        if not name:
            problems.append({"material": None, "reason": "材质缺 name"})
            continue
        # UE 对象名净化（gltf 材质名 '.' 等非法字符 → '_'，5.8.1 实测
        # TransparentGlass.DoubleSided → TransparentGlass_DoubleSided.uasset）。
        safe = "".join(ch if (ch.isalnum() or ch == "_") else "_" for ch in name)
        mic_path = base_dir + "/Materials/%s.%s" % (safe, safe)
        if not unreal.EditorAssetLibrary.does_asset_exist(mic_path):
            problems.append({"material": name, "reason": "MIC 资产缺失: %s" % safe})
            continue
        mic = unreal.EditorAssetLibrary.load_asset(mic_path)
        rec = {"material": name, "bound": []}
        refs = (
            ("BaseColorTexture", (m.get("pbrMetallicRoughness") or {}).get("baseColorTexture")),
            ("NormalTexture", m.get("normalTexture")),
        )
        for param, ref in refs:
            if not ref:
                continue
            ti = ref.get("index")
            try:
                src = textures[ti].get("source")
                stem = images[src].get("uri", "").rsplit("/", 1)[-1].rsplit(".", 1)[0]
            except (IndexError, AttributeError):
                problems.append({"material": name, "reason": "纹理引用链断裂: %s" % param})
                continue
            tex_path = base_dir + "/Textures/%s.%s" % (stem, stem)
            if not unreal.EditorAssetLibrary.does_asset_exist(tex_path):
                problems.append({"material": name, "reason": "纹理资产缺失: %s" % stem})
                continue
            tex = unreal.EditorAssetLibrary.load_asset(tex_path)
            mel.set_material_instance_texture_parameter_value(mic, param, tex)
            rec["bound"].append(param + "=" + stem)
        if rec["bound"]:
            # 读回核验（绑定未生效即报错不静默）
            tpv = mic.get_editor_property("texture_parameter_values") or []
            nonnull = sum(1 for e in tpv if getattr(e, "parameter_value", None) is not None)
            if nonnull < len(rec["bound"]):
                raise RuntimeError("MIC 纹理绑定读回核验失败: %s（bound=%d readback=%d）" % (name, len(rec["bound"]), nonnull))
            rec["readback_nonnull"] = int(nonnull)
            unreal.EditorAssetLibrary.save_asset(mic_path)
            bound.append(rec)
    log("bistro MIC 纹理绑定完成: %d 材质（problems=%d）" % (len(bound), len(problems)))
    if problems:
        raise RuntimeError("U2 纹理绑定面存在缺行（禁静默）: %s" % json.dumps(problems[:4], ensure_ascii=False))
    return {
        "bound_materials": len(bound),
        "bound_texture_params": sum(len(r["bound"]) for r in bound),
        "detail_head": bound[:6],
    }


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
    cornell_spawned = []  # (actor, gltf 材质索引) —— U1 双面置换映射面
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
        if scene_id == "cornell-box":
            # G11.3 U1 修复面：cornell 壳体双面化（单面片外向绕向 × UE 背面剔除
            # 口径对齐——壳体内表面参与光照）。语料 0-byte（不走 M133 修订——
            # 双端着色口径对齐面）；地板 white_tex 维持双端最大子集口径（棋盘格
            # 纹理双端不采样，材质 = white_tex factor [1,1,1] 双面白——G10.5a
            # 「降为 white」登记面演进为双面置换面，逐 actor baseColorFactor 置换）。
            mesh_def = gltf.get("meshes", [])[node["mesh"]]
            prims = mesh_def.get("primitives", [])
            mat_idx = prims[0].get("material") if prims else None
            cornell_spawned.append((actor, mat_idx))
        n_spawned += 1
    log("节点 spawn 完成: %d mesh actors" % n_spawned)
    g11_3_probe = {}
    if scene_id == "cornell-box":
        prov = apply_two_sided_cornell(gltf, cornell_spawned)
        g11_3_probe["two_sided_replacement"] = prov
        g11_3_probe["two_sided_actor_count"] = len(prov)

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
    # G11.2 C1 口径对齐修复（RXS-0392 L3）：契约 color_linear_rgb 本为线性域——
    # set_light_color 第二参 b_srgb=True 会被 UE 按 sRGB 二次转线性（bistro 太阳色
    # [1,0.98,0.95] 实测偏差 G −2.5% / B −6.3%）；b_srgb=False = 线性直给口径。
    dl.set_light_color(unreal.LinearColor(rgb[0], rgb[1], rgb[2], 1.0), False)
    dl.set_mobility(unreal.ComponentMobility.MOVABLE)
    log("太阳光: dir_ue=%s lux=%s（光色线性直给 b_srgb=False，G11.2 C1 口径对齐）" % (str(sun_dir_ue), str(ue_params["sun_intensity_lux"])))

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

    # ---- G11.4 R3 灯种子集（spec/global_illumination.md RXS-0394 L2：光源参数
    # 唯一事实源 = 契约光照参数面 corpus/lighting_*.json，双端同消费；bistro 包内
    # pointLight1~4 派生点光源逐盏 spawn + provenance 读回入探针；cornell 契约
    # sun+sky 灯面 0-byte——本段对 cornell 不触发）----
    if scene_id == "bistro-interior":
        import pathlib
        _light_json = (
            pathlib.Path(os.environ.get("G10_5_CONTRACT", "")).parent
            / "lighting_bistro_interior.json"
        )
        _ldoc = json.loads(_light_json.read_text(encoding="utf-8"))
        _pls = _ldoc.get("point_lights", [])
        if len(_pls) < 4:
            raise RuntimeError("契约光照 JSON point_lights < 4（R3 承接锚字面 4+ 盏）: %s" % _light_json)
        _pl_probe = []
        for pl in _pls:
            p_ue = contract.pos_contract_to_ue(pl["position"])
            pa = actor_subsys.spawn_actor_from_class(
                unreal.PointLight, unreal.Vector(*p_ue), unreal.Rotator(0, 0, 0)
            )
            pa.set_actor_label("G11_4_" + str(pl["id"]))
            pa.set_folder_path("G10")
            pc = pa.get_component_by_class(unreal.PointLightComponent)
            pc.set_intensity(float(pl["intensity_cd"]))  # candela（UE 点光默认单位）
            pc.set_light_color(
                unreal.LinearColor(pl["color_linear_rgb"][0], pl["color_linear_rgb"][1], pl["color_linear_rgb"][2], 1.0),
                False,
            )  # 线性直给（G11.2 C1 同口径）
            pc.set_mobility(unreal.ComponentMobility.MOVABLE)
            _pl_probe.append({
                "id": pl["id"],
                "position_ue_cm": [float(v) for v in p_ue],
                "intensity_cd_readback": float(pc.get_editor_property("intensity")),
                "emit_direction_contract": pl["emit_direction"],
                "derived_from": pl["derived_from"],
            })
        g11_3_probe["g11_4_point_lights"] = _pl_probe
        g11_3_probe["g11_4_point_lights_count"] = len(_pl_probe)
        g11_3_probe["g11_4_lighting_json_digest_note"] = "契约光照面单通道消费（RXS-0394 L2；UE 侧朗伯轴向简化为全向点光，强度微小实测面见 A/B 报告）"
        log("G11.4 点光源: %d 盏逐盏 spawn（契约光照 JSON 单通道）" % len(_pl_probe))

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
    return map_path, cam_actor, g11_3_probe


def probe_texture_params(scene_id):
    """G11.3 U2 修复面探针：导入后材质实例 texture_parameter_values 非空回归
    （bistro 派生链转码纹理绑定机核面；空值冒充修复即 RED 的计数目）。"""
    mesh_dir = CONTENT_ROOT + "/" + scene_id
    total = 0
    with_tex = 0
    detail = []
    for sub in unreal.EditorAssetLibrary.list_assets(mesh_dir, recursive=True):
        cls_path = unreal.EditorAssetLibrary.find_asset_data(sub).get_class().get_path_name()
        if "Material" not in cls_path:
            continue
        obj = unreal.EditorAssetLibrary.load_asset(sub)
        tpv = []
        try:
            tpv = list(obj.get_editor_property("texture_parameter_values") or [])
        except Exception:
            tpv = []
        nonnull = 0
        for e in tpv:
            try:
                if e.parameter_value is not None:
                    nonnull += 1
            except Exception:
                pass
        total += 1
        if nonnull > 0:
            with_tex += 1
        detail.append({"asset": sub.split("/")[-1], "class": cls_path.split("/")[-1],
                       "texture_params": len(tpv), "texture_params_nonnull": nonnull})
    log("材质纹理参数探针: %s materials=%d with_textures=%d" % (scene_id, total, with_tex))
    return {"materials_total": total, "materials_with_textures": with_tex,
            "texture_parameter_detail_head": detail[:8]}


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
    g11_3_binding = None
    if scene_id == "bistro-interior":
        # G11.3 U2 修复面：MIC 纹理参数显式绑定（Interchange 导入纹理资产成功但
        # 绑定缺位——绑定后 probe_texture_params 非空回归方可成立）。
        g11_3_binding = bind_bistro_material_textures(scene_id)
    map_path, cam_actor, g11_3_probe = build_level(scene_id, c)
    if scene_id == "bistro-interior":
        # G11.3 U2 修复面探针：材质实例 texture_parameter_values 非空回归。
        if g11_3_binding is not None:
            g11_3_probe["texture_binding"] = g11_3_binding
        g11_3_probe["texture_params"] = probe_texture_params(scene_id)
    # G11.3 探针输出（文件面为门脚本权威解析源——同 G10_5_PROBE_OUT 体例）。
    probe_out = os.environ.get("G11_3_PROBE_OUT", "")
    if probe_out and g11_3_probe:
        g11_3_probe["scene_id"] = scene_id
        with open(probe_out, "w", encoding="utf-8", newline="\n") as f:
            f.write(json.dumps(g11_3_probe, ensure_ascii=False, separators=(",", ":")) + "\n")
        log("G11.3 探针落盘: " + probe_out)
    # 出帧根目录：默认 G10.5 帧库面（G10 门序既有字面不动）；G11.2/G11.3 复测批经
    # G11_2_OUT_ROOT 环境变量指向对应波次帧区（G10 帧库只读纪律，K: 盘分区隔离）。
    out_root = os.environ.get("G11_2_OUT_ROOT", "K:/rurix-ext/g10-frames/g10_5/ue")
    out_dir = out_root.rstrip("/") + "/" + scene_id
    seq_path, cfg_path = build_mrq_assets(scene_id, c, cam_actor, out_dir)
    log("BUILD DONE scene=%s map=%s seq=%s cfg=%s" % (scene_id, map_path, seq_path, cfg_path))


main()

