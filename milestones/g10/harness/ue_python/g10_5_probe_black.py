#!/usr/bin/env python3
"""G10.5 harness — Bistro 全黑帧归因探针（几何在位性 + 材质纹理引用 + 光照读回）。
Assisted-by: Kimi-K3（G10.5a 波续）
"""
import unreal


def log(m):
    unreal.log("BLACK: " + str(m))


def main():
    unreal.get_editor_subsystem(unreal.LevelEditorSubsystem).load_level("/Game/Maps/G10_BistroInterior")
    actors = unreal.EditorLevelLibrary.get_all_level_actors()
    cam = None
    meshes = []
    for a in actors:
        label = a.get_actor_label()
        if label == "G10_ContractCamera":
            cam = a
        elif label.startswith("G10N_"):
            meshes.append(a)
    log("actors total=%d mesh_actors=%d" % (len(actors), len(meshes)))
    ct = cam.get_actor_transform()
    log("cam loc=(%.1f,%.1f,%.1f)" % (ct.translation.x, ct.translation.y, ct.translation.z))
    # 抽样 5 个 mesh actor 的 bounds 与相机距离/相对位置
    for a in meshes[:5] + meshes[len(meshes) // 2 : len(meshes) // 2 + 3]:
        o, e = a.get_actor_bounds(False)
        log(
            "%s bounds origin=(%.0f,%.0f,%.0f) extent=(%.0f,%.0f,%.0f)"
            % (a.get_actor_label(), o.x, o.y, o.z, e.x, e.y, e.z)
        )
    # 材质实例抽样：base color 纹理引用（贴图缺失 → 黑）
    mats = set()
    for a in meshes[:40]:
        smc = a.static_mesh_component
        if smc is None:
            continue
        m = smc.get_material(0)
        if m is not None:
            mats.add(m.get_path_name())
    for mp in sorted(mats)[:8]:
        mi = unreal.EditorAssetLibrary.load_asset(mp.split(".")[0])
        if mi is None:
            log("mat MISS " + mp)
            continue
        cls = mi.get_class().get_name()
        textures = []
        try:
            textures = [t.get_name() for t in mi.get_editor_property("texture_parameter_values")] if cls == "MaterialInstanceConstant" else []
        except Exception as e:
            textures = ["<scan fail %r>" % e]
        log("mat %s class=%s textures=%s" % (mp.split("/")[-1], cls, str(textures)))
    # 光照读回
    for a in actors:
        label = a.get_actor_label()
        if label in ("G10_Sun", "G10_Sky", "G10_Exposure"):
            log("light actor %s at %s" % (label, str(a.get_actor_location())))
    log("BLACK DONE")


main()
