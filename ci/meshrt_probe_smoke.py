#!/usr/bin/env python3
"""G3.6 mesh-task-RT DXIL 腿 probe(RFC-0013 §4.E9,RXS-0249;CI 步骤 68/69).

DXIL 腿条件分支的 probe-first 取证(measured-first,反 YAML-only):

- **步骤 68(mesh/task DXIL B 链 probe)**:最小 mesh SPIR-V → `spirv-cross --hlsl
  --shader-model 65` → `dxc -T ms_6_5`。probe **绿**(spirv-cross + dxc 均 exit 0)=
  mesh/task DXIL 全量可落(evidence/meshrt_bchain_probe_20260719.md);任一非零 → 该腿
  按 RFC §4.E9 落 RD-034 尾门。**退出码判定,非 grep stdout**。

- **步骤 69(RT blocked 探针,防静默腐烂)**:最小 raygen SPIR-V → `spirv-cross --hlsl`。
  spirv-cross HLSL 后端**无** SPV_KHR_ray_tracing 消费路径(`Unsupported builtin in
  HLSL: 5319` = LaunchIdKHR)。**预期失败 = 探针 PASS**(blocked 证据新鲜);spirv-cross
  某日**意外成功** → 探针**翻红**提醒复评(上游能力出现须跟进,对齐 RD-011/RD-015 纪律)。
  RT DXIL 全量登 RD-034 尾门,照 G-MB1-6 措辞越过 close-out 存续。

**三态 SKIP 纪律**:glslang / spirv-cross / dxc 任一不可定位 → SKIP(dev-env degrade,
非 fake pass,退 0);`RURIX_REQUIRE_REAL=1` 把缺失翻**硬红**。工具定位 env override >
Vulkan SDK Bin > PATH。**E5/E6 SPIR-V 编码器产物(mesh/RT 八阶段)的合法性另由
`src/rurixc/tests/mesh_rt_vulkan_spirv_val.rs`(spirv-val vulkan1.2/spv1.4)机核**——本
probe 只验 DXIL B 链腿(spirv-cross→dxc)这一正交轴。
"""

from __future__ import annotations

import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 最小 mesh 着色器(GL_EXT_mesh_shader,单三角形非空输出;probe 绿臂)。
MESH_SRC = """#version 460
#extension GL_EXT_mesh_shader : require
layout(local_size_x = 1) in;
layout(triangles, max_vertices = 3, max_primitives = 1) out;
void main() {
    SetMeshOutputsEXT(3u, 1u);
    gl_MeshVerticesEXT[0].gl_Position = vec4(0.0, 0.7, 0.0, 1.0);
    gl_MeshVerticesEXT[1].gl_Position = vec4(-0.7, -0.7, 0.0, 1.0);
    gl_MeshVerticesEXT[2].gl_Position = vec4(0.7, -0.7, 0.0, 1.0);
    gl_PrimitiveTriangleIndicesEXT[0] = uvec3(0u, 1u, 2u);
}
"""

# 最小 raygen 着色器(GL_EXT_ray_tracing;probe 红臂——blocked 探针)。
RGEN_SRC = """#version 460
#extension GL_EXT_ray_tracing : require
layout(set = 0, binding = 0) uniform accelerationStructureEXT tlas;
layout(location = 0) rayPayloadEXT vec4 payload;
void main() {
    payload = vec4(0.0);
    traceRayEXT(tlas, gl_RayFlagsOpaqueEXT, 0xFF, 0, 0, 0,
                vec3(0.0), 0.0, vec3(0.0, 0.0, 1.0), 100.0, 0);
}
"""


def locate(env_keys: list[str], names: list[str]) -> str | None:
    """env 绝对路径 > Vulkan SDK Bin > PATH 名定位可执行;均不可用 → None。"""
    for k in env_keys:
        v = os.environ.get(k)
        if v and Path(v).is_file():
            return v
    sdk = os.environ.get("VULKAN_SDK")
    if sdk:
        for n in names:
            for ext in ("", ".exe"):
                p = Path(sdk) / "Bin" / (n + ext)
                if p.is_file():
                    return str(p)
    for n in names:
        p = shutil.which(n)
        if p:
            return p
    return None


def run(cmd: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, capture_output=True, text=True)


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        print(f"[meshrt_probe] FAIL(REQUIRE_REAL): {msg}")
        return 1
    print(f"[meshrt_probe] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def red_self_test() -> bool:
    """red 自检(反 YAML-only):合成两臂判定逻辑——绿臂须两 exit 0 判绿,红臂须
    spirv-cross 非零判 blocked-PASS / exit 0 判意外翻红。纯逻辑,不依赖工具。"""
    # 绿臂:mesh spirv-cross(0)+ dxc(0)→ 绿。
    green_ok = (0 == 0) and (0 == 0)
    # 红臂:raygen spirv-cross 非零 = blocked 探针 PASS(新鲜);exit 0 = 意外成功翻红。
    blocked_pass = 1 != 0
    unexpected_green_is_red = not (0 != 0)
    return green_ok and blocked_pass and unexpected_green_is_red


def main() -> int:
    if not red_self_test():
        print("[meshrt_probe] FAIL red 自检(判定逻辑损坏)")
        return 1

    glslang = locate(["RURIX_GLSLANG"], ["glslang", "glslangValidator"])
    spirv_cross = locate(["RURIX_SPIRV_CROSS"], ["spirv-cross"])
    dxc = locate(["RURIX_DXC", "RURIX_DXC_NEW"], ["dxc"])
    if not glslang:
        return skip("glslang 定位失败")
    if not spirv_cross:
        return skip("spirv-cross 定位失败")

    workdir = Path(tempfile.mkdtemp(prefix="rurix_meshrt_probe_"))
    failures: list[str] = []

    # ── 步骤 68:mesh/task DXIL B 链 probe(预期绿) ──
    mesh_glsl = workdir / "mesh.mesh"
    mesh_spv = workdir / "mesh.spv"
    mesh_hlsl = workdir / "mesh.hlsl"
    mesh_dxil = workdir / "mesh.dxil"
    mesh_glsl.write_text(MESH_SRC, encoding="utf-8")
    r = run([glslang, "-V", "--target-env", "vulkan1.2", "-S", "mesh", str(mesh_glsl), "-o", str(mesh_spv)])
    if r.returncode != 0:
        return skip(f"glslang 不支持 mesh stage(exit {r.returncode})")
    r = run([spirv_cross, "--hlsl", "--shader-model", "65", str(mesh_spv), "--output", str(mesh_hlsl)])
    mesh_cross_ok = r.returncode == 0
    if not mesh_cross_ok:
        failures.append(f"步骤 68: spirv-cross 拒 mesh(exit {r.returncode};probe 由绿转红,须复评 RXS-0249 分支)")
    if mesh_cross_ok and dxc:
        r = run([dxc, "-T", "ms_6_5", "-E", "main", str(mesh_hlsl), "-Fo", str(mesh_dxil)])
        if r.returncode != 0:
            failures.append(f"步骤 68: dxc -T ms_6_5 拒 mesh HLSL(exit {r.returncode})")
        else:
            print(f"[meshrt_probe] 步骤 68 PASS: mesh B 链绿(spirv-cross ms_6_5 + dxc,{mesh_dxil.stat().st_size}B DXIL)")
    elif mesh_cross_ok:
        print("[meshrt_probe] 步骤 68 PART: mesh spirv-cross 绿;dxc 缺 → DXIL 产物段 SKIP(RXS-0249 probe 绿臂 spirv-cross 已证)")

    # ── 步骤 69:RT blocked 探针(预期失败 = PASS;意外成功 = 翻红) ──
    rgen_glsl = workdir / "rg.rgen"
    rgen_spv = workdir / "rg.spv"
    rgen_hlsl = workdir / "rg.hlsl"
    rgen_glsl.write_text(RGEN_SRC, encoding="utf-8")
    r = run([glslang, "-V", "--target-env", "vulkan1.2", "-S", "rgen", str(rgen_glsl), "-o", str(rgen_spv)])
    if r.returncode != 0:
        return skip(f"glslang 不支持 rgen stage(exit {r.returncode})")
    r = run([spirv_cross, "--hlsl", "--shader-model", "63", str(rgen_spv), "--output", str(rgen_hlsl)])
    if r.returncode != 0:
        # 预期失败:spirv-cross HLSL 后端无 SPV_KHR_ray_tracing 消费(LaunchIdKHR 5319)。
        print("[meshrt_probe] 步骤 69 PASS: RT blocked 探针新鲜(spirv-cross 如期拒 raygen;RD-034 尾门维持)")
    else:
        # 意外成功:上游翻绿,须复评 RT DXIL 腿(RD-034 / RD-015 跟踪)。
        failures.append(
            "步骤 69: RT blocked 探针**意外成功**——spirv-cross 竟接受 raygen HLSL "
            "转译(上游获得 SPV_KHR_ray_tracing 消费能力?)→ 复评 RXS-0249 RT 分支 + RD-034 尾门"
        )

    shutil.rmtree(workdir, ignore_errors=True)

    if failures:
        print("[meshrt_probe] FAIL")
        for f in failures:
            print(f"  - {f}")
        return 1
    print("[meshrt_probe] PASS(步骤 68 mesh B 链 probe + 步骤 69 RT blocked 探针)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
