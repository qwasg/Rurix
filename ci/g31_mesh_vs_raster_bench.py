#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Kimi-K3（G31+ 波 C Task C16 重判窗批量执行 M61 ③ measured 面）
"""G31+ 波 C Task C16:M61 ③ mesh shader HW 路径 vs 现 VS 光栅路径 measured
对照门冒烟(g31.waveC.meshbench;RFC-0034 重判表三项闭集之③;TODO §3.1 #24)。

链路:GLSL 四源(内嵌本文件;bench_common 段与 harness bin `tri_vert_ndc`
逐字同源)经 glslangValidator 现编 → harness `g31_mesh_vs_raster_bench`
(src/rurix-rt/src/bin/,vk_g31_mesh_bench.rs 单会话三臂底座)真跑 → 三臂
(vs_fetch 取数 / vs_procedural / mesh_procedural)同一确定性三角形集像素
digest 对拍 + GPU timestamp 逐帧 measured(median/mean/min/max 如实登记,
不设通过线——G6 无硬门纪律)。

判据闭集(milestones/g31/g31_mesh_vs_raster_bench_evidence_schema.json 描述段):
1. shader_compile:四 SPV 经 glslangValidator 现编 rc=0(缺工具 → 三态降级)。
2. harness_build:cargo build release rc=0(host 编译红 = FAIL 非 SKIP)。
3. device_run_digest_parity:harness 真跑 PASS——三臂像素 digest 位级全等
   (同一三角形集真上屏结构证据;无深度/无混合+恒色 fragment ⇒ 重叠写序
   不影响终图,digest 可比)。
4. measured_sane:逐臂 gpu_ms median/mean/min/max 有限正数 + samples==frames
   + timestamp_period_ns>0(measured 登记面健全,非阈门)。
5. determinism_double_run:双跑逐臂像素 digest 位级一致(影像确定性;
   时序数字双跑各自如实登记,不要求位级——真实 GPU 时序噪声面)。
6. validation_silent:RURIX_VK_VALIDATION=1 下 validation 零报错。

三态:无 glslang/无 Vulkan loader/mesh feature 缺失 → DEV_ENV_DEGRADE 退 0
(不冒充 PASS);RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL。

evidence 纪律:PASS-only schema 面——PASS 才落
evidence/g31_mesh_vs_raster_bench_<ts>.json(check_schemas 前缀路由
g31_mesh_vs_raster_);FAIL 诊断件落 .tmp/g31_gates/meshbench/ 不污染
evidence/ 路由面。

用法:
  py -3 ci/g31_mesh_vs_raster_bench.py --selftest
  py -3 ci/g31_mesh_vs_raster_bench.py --gate g31.waveC.meshbench [--triangles 262144] [--frames 60]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import io
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g31.waveC.meshbench"
SUBJECT = "g31_mesh_vs_raster_bench"
TAG = "g31_mesh_vs_raster_bench"
SCHEMA_PATH = ROOT / "milestones" / "g31" / "g31_mesh_vs_raster_bench_evidence_schema.json"
SCHEMA_ID = "rurix.g31.mesh_vs_raster_bench_gate_evidence.v1"
WORK = ROOT / ".tmp" / "g31_gates" / "meshbench"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "release" / f"g31_mesh_vs_raster_bench{EXE_SUFFIX}"

FAILURES: list[str] = []
FACT_IDS = [
    "shader_compile",
    "harness_build",
    "device_run_digest_parity",
    "measured_sane",
    "determinism_double_run",
    "validation_silent",
]

# ── GLSL 四源(bench_common 段与 src/rurix-rt/src/bin/g31_mesh_vs_raster_bench.rs
#    tri_vert_ndc/pcg_hash 逐字同源——任一侧改动 = 对拍面红)──
GLSL_COMMON = """uint pcg_hash(uint v) {
    uint state = v * 747796405u + 2891336453u;
    uint word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

vec2 tri_vert_px(uint tri, uint slot) {
    uint r0 = pcg_hash(tri);
    uint r1 = pcg_hash(tri ^ 0x9E3779B9u);
    uint cx = r0 % pc.gw;
    uint cy = r1 % pc.gh;
    uint ox = (r0 >> 8u) & 7u;
    uint oy = (r1 >> 8u) & 7u;
    uint flip = (r0 >> 16u) & 1u;
    float bx = float(cx * pc.cell + ox);
    float by = float(cy * pc.cell + oy);
    float s = float(pc.cell);
    vec2 p;
    if (flip == 0u) {
        p = slot == 0u ? vec2(bx, by) : (slot == 1u ? vec2(bx + s, by) : vec2(bx, by + s));
    } else {
        p = slot == 0u ? vec2(bx + s, by) : (slot == 1u ? vec2(bx + s, by + s) : vec2(bx, by + s));
    }
    return p;
}

vec2 px_to_ndc(vec2 p) {
    float w = float(pc.gw * pc.cell);
    float h = float(pc.gh * pc.cell);
    precise float nx = (p.x / w) * 2.0 - 1.0;
    precise float ny = (p.y / h) * 2.0 - 1.0;
    return vec2(nx, ny);
}
"""

GLSL_MESH = """#version 460
#extension GL_EXT_mesh_shader : require
#extension GL_GOOGLE_include_directive : require

layout(local_size_x = 64) in;
layout(triangles, max_vertices = 192, max_primitives = 64) out;

layout(push_constant) uniform PushC {
    uint gw;
    uint gh;
    uint cell;
    uint nt;
} pc;

#include "bench_common.glsl"

void main() {
    uint lane = gl_LocalInvocationIndex;
    if (lane == 0u) {
        SetMeshOutputsEXT(192u, 64u);
    }
    uint tri = gl_WorkGroupID.x * 64u + lane;
    vec2 p0 = px_to_ndc(tri_vert_px(tri, 0u));
    vec2 p1 = px_to_ndc(tri_vert_px(tri, 1u));
    vec2 p2 = px_to_ndc(tri_vert_px(tri, 2u));
    gl_MeshVerticesEXT[lane * 3u + 0u].gl_Position = vec4(p0, 0.0, 1.0);
    gl_MeshVerticesEXT[lane * 3u + 1u].gl_Position = vec4(p1, 0.0, 1.0);
    gl_MeshVerticesEXT[lane * 3u + 2u].gl_Position = vec4(p2, 0.0, 1.0);
    gl_PrimitiveTriangleIndicesEXT[lane] = uvec3(lane * 3u + 0u, lane * 3u + 1u, lane * 3u + 2u);
}
"""

GLSL_VS_FETCH = """#version 460
layout(location = 0) in vec4 pos;
void main() { gl_Position = pos; }
"""

GLSL_VS_PROC = """#version 460
#extension GL_GOOGLE_include_directive : require

layout(push_constant) uniform PushC {
    uint gw;
    uint gh;
    uint cell;
    uint nt;
} pc;

#include "bench_common.glsl"

void main() {
    uint vid = uint(gl_VertexIndex);
    uint tri = vid / 3u;
    uint slot = vid - tri * 3u;
    vec2 p = px_to_ndc(tri_vert_px(tri, slot));
    gl_Position = vec4(p, 0.0, 1.0);
}
"""

GLSL_FRAG = """#version 460
layout(location = 0) out vec4 o;
void main() { o = vec4(0.85, 0.35, 0.12, 1.0); }
"""

SHADER_SOURCES = [
    ("bench_common.glsl", GLSL_COMMON, None),
    ("bench.mesh", GLSL_MESH, "mesh"),
    ("bench_fetch.vert", GLSL_VS_FETCH, "vert"),
    ("bench_proc.vert", GLSL_VS_PROC, "vert"),
    ("bench.frag", GLSL_FRAG, "frag"),
]

ARM_IDS = ["vs_fetch", "vs_procedural", "mesh_procedural"]
DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def locate_glslang() -> str | None:
    """env 绝对路径 > Vulkan SDK Bin > PATH 名定位(meshrt_probe_smoke 同律)。"""
    v = os.environ.get("RURIX_GLSLANG")
    if v and Path(v).is_file():
        return v
    sdk = os.environ.get("VULKAN_SDK")
    if sdk:
        for n in ("glslangValidator", "glslang"):
            for ext in ("", ".exe"):
                p = Path(sdk) / "Bin" / (n + ext)
                if p.is_file():
                    return str(p)
    for n in ("glslangValidator", "glslang"):
        p = shutil.which(n)
        if p:
            return p
    return None


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面)
# ---------------------------------------------------------------------------


def bench_doc_judge(doc: dict, frames: int) -> list[str]:
    """harness JSON 判读(对拍 + measured 健全;selftest 红绿臂消费)。"""
    fails: list[str] = []
    if not isinstance(doc, dict):
        return ["harness JSON 非 object"]
    if doc.get("schema") != "rurix.g31.mesh_vs_raster_bench.v1":
        fails.append(f"schema 漂移: {doc.get('schema')!r}")
    if doc.get("digest_all_equal") is not True:
        fails.append("digest_all_equal ≠ true(三臂终图分叉 = 对拍面破坏)")
    arms = doc.get("arms")
    if not isinstance(arms, list) or [a.get("arm") for a in arms if isinstance(a, dict)] != ARM_IDS:
        fails.append(f"arms 闭集破: {arms!r}"[:120])
        return fails
    digests: list[str] = []
    for a in arms:
        d = a.get("pixel_digest", "")
        if not DIGEST_RE.match(d):
            fails.append(f"{a.get('arm')} pixel_digest 形态非法: {str(d)[:40]!r}")
        digests.append(d)
        for k in ("gpu_ms_median", "gpu_ms_mean", "gpu_ms_min", "gpu_ms_max"):
            v = a.get(k)
            if not isinstance(v, (int, float)) or isinstance(v, bool) or not (v == v and v > 0):
                fails.append(f"{a.get('arm')}.{k} 非有限正数: {v!r}")
        if a.get("samples") != frames:
            fails.append(f"{a.get('arm')}.samples {a.get('samples')} ≠ frames {frames}")
    if len(set(digests)) != 1:
        fails.append(f"三臂 digest 未全等: {digests!r}"[:160])
    tp = doc.get("timestamp_period_ns")
    if not isinstance(tp, (int, float)) or isinstance(tp, bool) or not (tp == tp and tp > 0):
        fails.append(f"timestamp_period_ns 非正: {tp!r}")
    for k in ("triangles", "frames", "warmup", "width", "height"):
        v = doc.get(k)
        if not isinstance(v, int) or isinstance(v, bool) or v < 1:
            fails.append(f"{k} 非正整数: {v!r}")
    return fails


def degrade_exit_code(degrade: list[str], require_real: bool) -> int | None:
    if not degrade:
        return None
    return 1 if require_real else 0


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def run_gate(triangles: int, frames: int, warmup: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not SCHEMA_PATH.is_file():
        fail(f"门 schema 缺失: {SCHEMA_PATH}")
        return 1
    WORK.mkdir(parents=True, exist_ok=True)
    degrade: list[str] = []

    # ── ① GLSL 四源落盘 + glslang 现编 ──
    for name, src, _stage in SHADER_SOURCES:
        (WORK / name).write_text(src, encoding="utf-8", newline="\n")
    glslang = locate_glslang()
    spvs: dict[str, Path] = {}
    compile_ok = True
    if not glslang:
        degrade.append("glslangValidator 定位失败(env/VULKAN_SDK/PATH 三路零命中)")
        compile_ok = False
    else:
        for name, _src, stage in SHADER_SOURCES:
            if stage is None:
                continue
            # 命名 = 词干_阶段(bench.mesh vs bench.frag 词干碰撞教训——唯一命名防覆盖)。
            out_spv = WORK / (Path(name).stem + "_" + stage + ".spv")
            r = run(
                [glslang, "-V", "--target-env", "vulkan1.2", "-I.", "-S", stage,
                 "-o", str(out_spv), str(WORK / name)],
                timeout=600,
            )
            if r.returncode != 0 or not out_spv.is_file():
                fail(f"glslang 编译 {name} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}")
                compile_ok = False
            else:
                spvs[stage + "_" + Path(name).stem] = out_spv
    set_fact(
        "shader_compile",
        compile_ok and len(spvs) == 4,
        f"GLSL 四源经 glslang 现编: {sorted(p.name for p in spvs.values())}"
        + ("" if compile_ok else "(失败/降级见上)"),
    )

    # ── ② harness 构建 ──
    build_ok = False
    if compile_ok:
        r = run(
            ["cargo", "build", "--release", "-p", "rurix-rt", "--features", "vulkan",
             "--bin", "g31_mesh_vs_raster_bench", "--quiet"],
            timeout=7200,
        )
        build_ok = r.returncode == 0 and BIN.is_file()
        if not build_ok:
            fail(f"cargo build harness 失败(host 编译红,非 SKIP 事项): {(r.stdout + r.stderr)[-400:]}")
    set_fact("harness_build", build_ok, f"cargo build --release g31_mesh_vs_raster_bench rc={'0' if build_ok else '≠0'}")

    code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
    if code is not None:
        doc = {"schema": "rurix.g31.mesh_vs_raster_bench.skip.v1", "state": "DEV_ENV_DEGRADE", "reasons": degrade}
        print(json.dumps(doc, ensure_ascii=False))
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0
    if not (compile_ok and build_ok):
        return 1

    # ── ③④⑤⑥ device 双跑(gpu_device_lock 串行)──
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    argv = [
        str(BIN),
        str(spvs["mesh_bench"]),
        str(spvs["vert_bench_fetch"]),
        str(spvs["vert_bench_proc"]),
        str(spvs["frag_bench"]),
        "--triangles", str(triangles),
        "--frames", str(frames),
        "--warmup", str(warmup),
    ]
    run_docs: list[dict | None] = []
    run_outs: list[str] = []
    skipped = False
    with gpu_device_lock(purpose=f"{TAG} harness 双跑(device 真跑 + 确定性复跑)"):
        for _rep in range(2):
            r = run(argv, timeout=3600, env=env)
            out = (r.stdout or "") + (r.stderr or "")
            run_outs.append(out)
            if "MESH_BENCH: SKIP" in out:
                skipped = True
                break
            m = re.search(r"MESH_BENCH_JSON: (\{.*\})", out)
            doc = None
            if r.returncode == 0 and m:
                try:
                    doc = json.loads(m.group(1))
                except json.JSONDecodeError:
                    doc = None
            run_docs.append(doc)
    if skipped:
        degrade.append("harness 报 SKIP(无 Vulkan 设备 / mesh feature 缺失)")
        code = degrade_exit_code(degrade, os.environ.get("RURIX_REQUIRE_REAL") == "1")
        for d in degrade:
            note(f"DEV_ENV_DEGRADE {d}")
        if code == 1:
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    doc1, doc2 = (run_docs + [None, None])[:2]
    judge1 = bench_doc_judge(doc1, frames) if doc1 else ["跑 1 harness JSON 缺失/解析失败"]
    for f_ in judge1:
        fail(f"跑 1: {f_}")
    parity_ok = not judge1 and "MESH_BENCH: PASS" in run_outs[0]
    set_fact(
        "device_run_digest_parity",
        parity_ok,
        f"harness 真跑: digest_all_equal={(doc1 or {}).get('digest_all_equal')} "
        f"digest={(((doc1 or {}).get('arms') or [{}])[0].get('pixel_digest') or '')[:26]}…"
        + ("" if parity_ok else f";红 {judge1[:2]}"),
    )
    sane_ok = not judge1
    arm_stats = {}
    if doc1:
        for a in doc1.get("arms", []):
            arm_stats[a["arm"]] = {
                "gpu_ms_median": a["gpu_ms_median"],
                "gpu_ms_mean": a["gpu_ms_mean"],
                "gpu_ms_min": a["gpu_ms_min"],
                "gpu_ms_max": a["gpu_ms_max"],
                "wall_ms_median": a.get("wall_ms_median"),
                "samples": a.get("samples"),
                "pixel_digest": a.get("pixel_digest"),
            }
    set_fact(
        "measured_sane",
        sane_ok,
        "逐臂 GPU ms(median/mean/min/max): "
        + "; ".join(
            f"{k}={v['gpu_ms_median']:.4f}/{v['gpu_ms_mean']:.4f}/{v['gpu_ms_min']:.4f}/{v['gpu_ms_max']:.4f}"
            for k, v in arm_stats.items()
        )
        + f"(samples={frames} 各;如实登记不设通过线)"
        if arm_stats else "measured 面缺失",
    )
    judge2 = bench_doc_judge(doc2, frames) if doc2 else ["跑 2 harness JSON 缺失/解析失败"]
    det_ok = False
    if not judge2 and doc1 and doc2:
        d1 = [a.get("pixel_digest") for a in doc1.get("arms", [])]
        d2 = [a.get("pixel_digest") for a in doc2.get("arms", [])]
        det_ok = d1 == d2 and len(set(d1)) == 1
    else:
        for f_ in judge2:
            fail(f"跑 2: {f_}")
    set_fact(
        "determinism_double_run",
        det_ok,
        f"双跑逐臂像素 digest 位级一致={det_ok}(影像确定性;时序双跑各自如实登记)",
    )
    val_silent = all("Validation Error" not in o and "VUID-" not in o for o in run_outs)
    set_fact("validation_silent", val_silent, f"RURIX_VK_VALIDATION=1 双跑 validation 静默={val_silent}")

    # ── 门裁决 + evidence(PASS-only 面)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    env_info = {
        "gpu": (doc1 or {}).get("device_name", "unknown"),
        "driver_version": (doc1 or {}).get("driver_version", 0),
        "vendor_id": (doc1 or {}).get("vendor_id", 0),
        "api_version": (doc1 or {}).get("api_version", 0),
        "timestamp_period_ns": (doc1 or {}).get("timestamp_period_ns", 0.0),
        "os": "windows" if sys.platform == "win32" else sys.platform,
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    d_run2 = {}
    if doc2:
        for a in doc2.get("arms", []):
            d_run2[a["arm"]] = {
                "gpu_ms_median": a["gpu_ms_median"],
                "gpu_ms_mean": a["gpu_ms_mean"],
                "gpu_ms_min": a["gpu_ms_min"],
                "gpu_ms_max": a["gpu_ms_max"],
                "pixel_digest": a.get("pixel_digest"),
            }
    gate_doc = {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": "G31+.C",
        "workload": {
            "width": (doc1 or {}).get("width", 0),
            "height": (doc1 or {}).get("height", 0),
            "triangles": (doc1 or {}).get("triangles", 0),
            "frames": (doc1 or {}).get("frames", 0),
            "warmup": (doc1 or {}).get("warmup", 0),
            "triangle_regime": "确定性 PCG 哈希散布小三角形(cell=8px 右三角;Nanite 域小三角形档;重叠恒色写序无关)",
        },
        "arms": arm_stats,
        "arms_run2": d_run2,
        "digest_all_equal": (doc1 or {}).get("digest_all_equal") is True,
        "double_run_digest_bitexact": det_ok,
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G31+ 波 C Task C16 M61 ③ measured 对照(RFC-0034 重判表三项闭集之③):单会话三臂——"
            "vs_fetch(device-local vertex buffer 取数 = 现光栅路径形态)/ vs_procedural(同管线形态"
            "隔离取数成本,解释性臂)/ mesh_procedural(mesh 阶段 64 lane/wg 每 lane 1 三角形);"
            "同一确定性三角形集 ⇒ 三臂像素 digest 位级全等对拍(结构证据)+ GPU timestamp 逐帧 "
            "measured(TOP/BOTTOM 包 render pass;median/mean/min/max 如实登记不设通过线——G6 无硬门纪律)。"
            "主对照 = vs_fetch vs mesh_procedural。判据:①glslang 现编 ②host 构建 ③真跑+对拍 "
            f"④measured 健全 ⑤双跑 digest 位级={det_ok} ⑥validation 静默。"
            f"facts: {'; '.join(f['id'] + '=' + f['status'] for f in fact_rows)}"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED;PASS-only 闭集面)

    errs = list(jsonschema.Draft7Validator(
        json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        for e in errs[:5]:
            fail("gate evidence schema 自校验红: " + "/".join(str(p) for p in e.path) + f": {e.message}")
        all_pass = False
    if all_pass:
        gate_path = ROOT / "evidence" / f"g31_mesh_vs_raster_bench_{ts}.json"
    else:
        gate_path = WORK / f"gate_fail_{ts}.json"
    with io.open(gate_path, "w", encoding="utf-8", newline="\n") as f:
        f.write(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n")
    note(f"evidence: {gate_path.relative_to(ROOT)}")
    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿两臂,无 GPU/无构建依赖)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    dg = "sha256:" + "ab" * 32
    good_arm = lambda name: {  # noqa: E731
        "arm": name,
        "gpu_ms_median": 1.234, "gpu_ms_mean": 1.345, "gpu_ms_min": 0.9, "gpu_ms_max": 2.1,
        "wall_ms_median": 2.0, "wall_ms_mean": 2.1, "samples": 60, "pixel_digest": dg,
    }
    good_doc = {
        "schema": "rurix.g31.mesh_vs_raster_bench.v1",
        "subject": "g31_mesh_vs_raster_bench",
        "width": 1920, "height": 1080, "triangles": 262144, "frames": 60, "warmup": 10,
        "timestamp_period_ns": 1.0, "device_name": "RTX 4070 Ti",
        "driver_version": 1, "vendor_id": 4318, "api_version": 4206882,
        "arms": [good_arm(a) for a in ARM_IDS],
        "digest_all_equal": True,
    }
    expect(bench_doc_judge(good_doc, 60) == [], "GREEN:正例")
    expect(bench_doc_judge(None, 60) != [], "RED:非 object 必红")
    bad = json.loads(json.dumps(good_doc))
    bad["digest_all_equal"] = False
    expect(bench_doc_judge(bad, 60) != [], "RED:digest 未全等必红")
    bad = json.loads(json.dumps(good_doc))
    bad["arms"][1]["pixel_digest"] = "sha256:" + "cd" * 32
    expect(bench_doc_judge(bad, 60) != [], "RED:单臂 digest 分叉必红")
    bad = json.loads(json.dumps(good_doc))
    bad["arms"][0]["gpu_ms_median"] = 0.0
    expect(bench_doc_judge(bad, 60) != [], "RED:0ms 必红")
    bad = json.loads(json.dumps(good_doc))
    bad["arms"][0]["gpu_ms_mean"] = "zz"
    expect(bench_doc_judge(bad, 60) != [], "RED:非数值必红")
    bad = json.loads(json.dumps(good_doc))
    bad["arms"][2]["samples"] = 59
    expect(bench_doc_judge(bad, 60) != [], "RED:samples 漂移必红")
    bad = json.loads(json.dumps(good_doc))
    bad["arms"] = bad["arms"][:2]
    expect(bench_doc_judge(bad, 60) != [], "RED:臂闭集缺件必红")
    bad = json.loads(json.dumps(good_doc))
    bad["timestamp_period_ns"] = 0.0
    expect(bench_doc_judge(bad, 60) != [], "RED:timestampPeriod 非正必红")
    bad = json.loads(json.dumps(good_doc))
    bad["arms"][0]["pixel_digest"] = "zz"
    expect(bench_doc_judge(bad, 60) != [], "RED:digest 形态非法必红")
    expect(degrade_exit_code([], False) is None, "GREEN:无降级续跑")
    expect(degrade_exit_code(["x"], False) == 0, "GREEN:降级 SKIP 退 0")
    expect(degrade_exit_code(["x"], True) == 1, "RED:REQUIRE_REAL 下降级翻硬红")
    # 源同源互核:GLSL 公共段关键字面 == harness bin 锚字面(漂移即红)。
    bin_src = (ROOT / "src" / "rurix-rt" / "src" / "bin" / "g31_mesh_vs_raster_bench.rs").read_text(encoding="utf-8")
    for token in ("747_796_405", "2_891_336_453", "277_803_737", "0x9E37_79B9"):
        expect(token in bin_src, f"bin 哈希字面 {token} 在案")
    for token in ("747796405u", "2891336453u", "277803737u", "0x9E3779B9u"):
        expect(token in GLSL_COMMON, f"GLSL 哈希字面 {token} 在案")
    expect("precise float nx" in GLSL_COMMON, "GLSL precise 禁 FMA 字面在案")
    expect(SCHEMA_PATH.is_file(), "门 schema 在树")
    if SCHEMA_PATH.is_file():
        gs = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "schema const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "subject const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate key const 互核")
    expect(len(FACT_IDS) == 6, "facts 闭集 = 6")
    expect(len(ARM_IDS) == 3, "臂闭集 = 3")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=6;红臂 10 + 正例 + 同源互核 + schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--triangles", type=int, default=262144)
    ap.add_argument("--frames", type=int, default=60)
    ap.add_argument("--warmup", type=int, default=10)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.triangles % 64 != 0 or args.triangles < 64:
            print(f"[{TAG}] FAIL: --triangles 须为 64 非零整数倍", file=sys.stderr)
            return 1
        if args.frames < 8:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 8", file=sys.stderr)
            return 1
        return run_gate(args.triangles, args.frames, args.warmup)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
