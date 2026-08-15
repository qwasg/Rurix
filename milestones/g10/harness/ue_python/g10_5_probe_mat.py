#!/usr/bin/env python3
"""G10.5 harness — 材质参数/父链/two-sided 核验探针。
Assisted-by: Kimi-K3（G10.5a 波）
"""
import unreal


def log(m):
    unreal.log("MAT: " + str(m))


def main():
    mel = unreal.MaterialEditingLibrary
    for name in ("white", "red", "green"):
        p = "/Game/G10/cornell-box/cornell_box/Materials/{0}.{0}".format(name)
        mi = unreal.EditorAssetLibrary.load_asset(p)
        if mi is None:
            log("MISS " + name)
            continue
        # 父链走到基底 Material
        chain = []
        cur = mi
        for _ in range(8):
            par = cur.get_editor_property("parent") if hasattr(cur, "get_editor_property") else None
            if par is None:
                break
            chain.append((par.get_name(), par.get_class().get_name()))
            if par.get_class().get_name() == "Material":
                break
            cur = par
        log("%s chain: %s" % (name, str(chain)))
        # 向量参数全扫
        try:
            names = mel.get_vector_parameter_names(mi) if hasattr(mel, "get_vector_parameter_names") else None
            log("%s vector params: %s" % (name, str(names)))
        except Exception as e:
            log("%s param scan fail: %r" % (name, e))
    log("MAT DONE")


main()
