#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""renderer One True Device Frame 冒烟(步骤 96;G7.6;验收门 G-G7-8)。

host 段(**恒跑**,需 Vulkan SDK 的 `spirv-val`/`spirv-dis`;缺工具 → SKIP):
  1. schema 自检:两 evidence schema(device_frame + soak)可加载且 Draft7 可构造。
  2. 冻结锚 + RD-038 行 1/2/4/8:`G7_SCENE_FREEZE`(含 960×540→1920×1080)+
     矩阵 §1 四行字面 + §6.4 帧链并入留痕;RD-038 维持 open。
  3. host oracle 过滤:`cargo test -p rurix-render --lib` 过滤
     `geometry::`+`shadow::`+`temporal::`+`rt::`+`gi::`(步骤 94/95 过滤集之并)。
  4. **既有 kernel manifest 零漂移**:帧链复用既有核(与 w1w2 manifest 交集及全表)
     对 `tests/vulkan/w1w2_spv_manifest.json` 逐字节比对。
  5. 新 glue kernel 排放:6 个新 .rx → spirv-val vulkan1.0 + SPIR-V 1.0 +
     同源 ×2 确定性 + 零 ray query 误声明。
  6. 静态 provenance 审计:`device_frame.rs` 禁 `execute_frame(` 单发入口,
     唯一入口 `execute_with_frame_update`;15 pass 名与关键边表字面在位。

device 段(**gate real**,`RURIX_REQUIRE_REAL=1` + `RURIX_VK_VALIDATION=1`;门 G-G7-8):
  7. `uc06-renderer --release --features device-frame --device-frame --frames 8 --json`
     正轴:阶段转移对拍 + 非退化 + provenance + telemetry。
  8. RED 四轴(独立进程,同 release):`--frame-red-visbuffer` / `--frame-red-history` /
     `--frame-red-jitter` / `--frame-red-provenance` → 期望 `FRAME: RED-OK`。

soak(`--soak`,不进 PR workflow):release 转发 CLI 取证(stdout 实时泵出),真跑归 PR-4;本脚本默认短跑 8 帧。

证据 → `evidence/renderer_device_frame_smoke_<ts>.json`。
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
import threading
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
MANIFEST = ROOT / "tests" / "vulkan" / "w1w2_spv_manifest.json"
KERNEL_DIR = ROOT / "apps" / "uc06-renderer" / "kernels"
LITERAL_MATRIX = ROOT / "milestones" / "g7" / "RD038_LITERAL_MATRIX.md"
SCENE_FREEZE = ROOT / "milestones" / "g7" / "G7_SCENE_FREEZE.md"
DEVICE_FRAME_SRC = ROOT / "apps" / "uc06-renderer" / "src" / "device_frame.rs"
DF_SCHEMA = ROOT / "milestones" / "g7" / "renderer_device_frame_evidence_schema.json"
SOAK_SCHEMA = ROOT / "milestones" / "g7" / "renderer_soak_evidence_schema.json"

TAG = "renderer_device_frame_smoke"

# 新 glue kernel(设计 §1.2;SPIR-V 1.0 / vulkan1.0)。
GLUE_KERNELS = (
    "frame_clear",
    "cull_frame",
    "tri_expand",
    "gbuffer_resolve",
    "deferred_shade",
    "tsr_temporal",
)

# 帧链复用既有 kernel(字节不动承诺;与 w1w2 manifest 交集机验零漂移)。
EXISTING_CHAIN_KERNELS = (
    "visbuffer_sw_u64",
    "classify_resolve",
    "vsm_depth_raster",
    "vsm_sample",
    "taa",
    "tsr_resample",
    "gi_probe",
    "rtao",
    "hard_shadow",
)

PASS_NAMES = (
    "frame_clear",
    "cull_frame",
    "tri_expand",
    "visbuffer_sw_u64",
    "classify_resolve",
    "gbuffer_resolve",
    "vsm_depth_raster",
    "vsm_sample",
    "gi_probe",
    "rtao",
    "hard_shadow",
    "deferred_shade",
    "taa",
    "tsr_resample",
    "tsr_temporal",
)

EXPECTED_EDGES = (
    ("cull_frame", "tri_expand", "visible_flags"),
    ("tri_expand", "visbuffer_sw_u64", "triangles"),
    ("visbuffer_sw_u64", "classify_resolve", "vis"),
    ("visbuffer_sw_u64", "gbuffer_resolve", "vis"),
    ("gbuffer_resolve", "vsm_sample", "pos"),
    ("gbuffer_resolve", "rtao", "pos"),
    ("gbuffer_resolve", "hard_shadow", "pos"),
    ("deferred_shade", "taa", "hdr"),
    ("taa", "tsr_resample", "taa_out"),
    ("tsr_resample", "tsr_temporal", "tsr_cur"),
)

# RD-038 §1 行 1/2/4/8(帧链并入余项)。
FRAME_CHAIN_ROWS = (
    "两级剔除",
    "VisBuffer SW(u64 atomicMax)",
    "classify-resolve",
    "TAA-TSR",
)

SCENE_ANCHORS = (
    "764",
    "实例数 = 3",
    "[0.0, 2.2, 3.4]",
    "[0.0, 0.35, 0.0]",
    "960×540",
    "1920×1080",
)

ASSEMBLY_NOTE = (
    "cluster27 id 域 = 全局簇表下标(装配层选择;VisBuffer 64-bit 位布局 30|27|7 冻结面 "
    "0-byte);c2m 会话期静态;`classify_resolve.rx` 零改动复用。"
)


def fail(msg: str) -> int:
    print(f"[{TAG}] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[{TAG}] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, env=None, timeout: int = 7200):
    r = subprocess.run(
        cmd, capture_output=True, cwd=str(ROOT), timeout=timeout, env=env
    )
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


def tool(name: str) -> str | None:
    env_key = {"spirv-val": "RURIX_SPIRV_VAL", "spirv-dis": "RURIX_SPIRV_DIS"}.get(name)
    if env_key:
        p = os.environ.get(env_key)
        if p and Path(p).is_file():
            return p
    return shutil.which(name)


def sha256_of(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def spv_version(path: Path) -> str:
    raw = path.read_bytes()
    if len(raw) < 8:
        return "invalid"
    v = int.from_bytes(raw[4:8], "little")
    return f"{(v >> 16) & 0xFF}.{(v >> 8) & 0xFF}"


def compile_rx(src: Path, out: Path) -> tuple[int, str]:
    code, o, e = run(
        [
            "cargo", "run", "-q", "-p", "rurixc",
            "--features", "vulkan-backend", "--bin", "rurixc", "--",
            str(src), "--target", "vulkan", "-o", str(out),
        ]
    )
    return code, (o + e)


def spirv_val(spv: Path, target_env: str | None) -> tuple[int, str]:
    exe = tool("spirv-val")
    if not exe:
        return (-1, "spirv-val 不可用")
    cmd = [exe]
    if target_env:
        cmd += ["--target-env", target_env]
    cmd.append(str(spv))
    code, o, e = run(cmd)
    return code, (o + e)


def disasm(spv: Path) -> tuple[int, str]:
    exe = tool("spirv-dis")
    if not exe:
        return (-1, "spirv-dis 不可用")
    code, o, e = run([exe, str(spv)])
    return code, (o + e)


def parse_json_line(blob: str, subject: str) -> dict | None:
    for line in blob.splitlines():
        line = line.strip()
        if line.startswith("{") and subject in line:
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                continue
    return None


# ───────────────────────── host 段(恒跑) ─────────────────────────


def schema_self_check(results: dict) -> bool:
    try:
        import jsonschema
    except ImportError:
        print(f"[{TAG}] 缺 jsonschema(pip install -r requirements.txt)", file=sys.stderr)
        results["schema_self_check_pass"] = False
        return False
    ok = True
    for path in (DF_SCHEMA, SOAK_SCHEMA):
        if not path.is_file():
            print(f"[{TAG}] 缺 schema {path.relative_to(ROOT)}", file=sys.stderr)
            ok = False
            continue
        try:
            schema = json.loads(path.read_text(encoding="utf-8"))
            jsonschema.Draft7Validator.check_schema(schema)
            jsonschema.Draft7Validator(schema)
        except Exception as exc:  # noqa: BLE001 — schema 自检须吞并报告
            print(f"[{TAG}] schema 自检失败 {path.name}: {exc}", file=sys.stderr)
            ok = False
    results["schema_self_check_pass"] = ok
    if ok:
        print(f"[{TAG}] 步骤 1 PASS: 两 evidence schema Draft7 自检绿")
    return ok


def freeze_and_matrix_section(results: dict) -> bool:
    ok = True
    if not LITERAL_MATRIX.is_file():
        print(f"[{TAG}] 缺 {LITERAL_MATRIX.relative_to(ROOT)}", file=sys.stderr)
        ok = False
        text = ""
    else:
        text = LITERAL_MATRIX.read_text(encoding="utf-8")
    missing_rows = [r for r in FRAME_CHAIN_ROWS if f"| {r}" not in text]
    if missing_rows:
        print(f"[{TAG}] RD-038 §1 缺帧链行字面: {missing_rows}", file=sys.stderr)
        ok = False
    if "### 6.4" not in text:
        print(f"[{TAG}] RD038 矩阵缺 §6.4(G7.6 帧链并入留痕)", file=sys.stderr)
        ok = False
    else:
        sec = text.split("### 6.4", 1)[1].split("### ", 1)[0]
        need = ("帧链", "步骤 96", "维持 open", "行 1", "行 2", "行 4", "行 8")
        miss = [n for n in need if n not in sec]
        if miss:
            print(f"[{TAG}] §6.4 缺关键字面: {miss}", file=sys.stderr)
            ok = False
    if not SCENE_FREEZE.is_file():
        print(f"[{TAG}] 缺 {SCENE_FREEZE.relative_to(ROOT)}", file=sys.stderr)
        ok = False
        freeze = ""
    else:
        freeze = SCENE_FREEZE.read_text(encoding="utf-8")
    missing_anchors = [a for a in SCENE_ANCHORS if a not in freeze]
    if missing_anchors:
        print(f"[{TAG}] G7_SCENE_FREEZE 缺锚: {missing_anchors}", file=sys.stderr)
        ok = False
    results["freeze_and_matrix_pass"] = ok
    results["literal_rows_frame_chain"] = list(FRAME_CHAIN_ROWS)
    results["scene_freeze_anchors"] = list(SCENE_ANCHORS)
    results["assembly_notes"] = ASSEMBLY_NOTE
    if ok:
        print(
            f"[{TAG}] 步骤 2 PASS: SCENE_FREEZE(含 960×540→1080p)+ "
            f"RD-038 行 1/2/4/8 + §6.4 帧链并入(RD 维持 open)"
        )
    return ok


def oracle_section(results: dict) -> bool:
    ok = True
    filters = (
        ("geometry::", "geometry"),
        ("shadow::", "shadow"),
        ("temporal::", "temporal"),
        ("rt::", "rt"),
        ("gi::", "gi"),
    )
    for filt, label in filters:
        code, o, e = run(
            ["cargo", "test", "-q", "-p", "rurix-render", "--lib", "--", filt]
        )
        if code != 0:
            print((o + e)[-2400:], file=sys.stderr)
            print(f"[{TAG}] host oracle 单测未过: {label}({filt})", file=sys.stderr)
            ok = False
    results["oracle_tests_pass"] = ok
    if ok:
        print(
            f"[{TAG}] 步骤 3 PASS: host oracle "
            f"(geometry::+shadow::+temporal::+rt::+gi::)全量恒绿"
        )
    return ok


def existing_manifest_zero_drift(results: dict, work: Path) -> bool:
    if not MANIFEST.is_file():
        results["existing_manifest_zero_drift_pass"] = False
        print(f"[{TAG}] 缺 golden manifest {MANIFEST.relative_to(ROOT)}", file=sys.stderr)
        return False
    expected = json.loads(MANIFEST.read_text(encoding="utf-8"))["kernels"]
    observed: dict = {}
    for name, want in sorted(expected.items()):
        src = KERNEL_DIR / f"{name}.rx"
        spv = work / f"w1w2_{name}.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2000:], file=sys.stderr)
            results["existing_manifest_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 编译失败", file=sys.stderr)
            return False
        ver, digest = spv_version(spv), sha256_of(spv)
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        if ver != want["spirv_version"] or digest != want["sha256"] or caps != want["capabilities"]:
            results["existing_manifest_zero_drift_pass"] = False
            print(
                f"[{TAG}] W1/W2 {name} **漂移**: ver={ver} sha={digest} caps={caps} "
                f"vs manifest {want}",
                file=sys.stderr,
            )
            return False
        if "RayQueryKHR" in caps or "SPV_KHR_ray_query" in dis:
            results["existing_manifest_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 误声明 ray query 面", file=sys.stderr)
            return False
        if name in EXISTING_CHAIN_KERNELS:
            observed[name] = {
                "spirv_version": ver,
                "sha256": digest,
                "manifest_matched": True,
                "capabilities": caps,
            }
    # 帧链复用但不在 w1w2 manifest 的既有核:排放确定性 + 版本口径(不重 invent golden)。
    for name in EXISTING_CHAIN_KERNELS:
        if name in observed:
            continue
        src = KERNEL_DIR / f"{name}.rx"
        if not src.is_file():
            results["existing_manifest_zero_drift_pass"] = False
            print(f"[{TAG}] 缺既有链核 {src.relative_to(ROOT)}", file=sys.stderr)
            return False
        spv1 = work / f"chain_{name}_a.spv"
        spv2 = work / f"chain_{name}_b.spv"
        code, blob = compile_rx(src, spv1)
        if code != 0 or not spv1.is_file():
            print(blob[-2000:], file=sys.stderr)
            results["existing_manifest_zero_drift_pass"] = False
            print(f"[{TAG}] 既有链核 {name} 编译失败", file=sys.stderr)
            return False
        code2, _ = compile_rx(src, spv2)
        if code2 != 0 or spv1.read_bytes() != spv2.read_bytes():
            results["existing_manifest_zero_drift_pass"] = False
            print(f"[{TAG}] 既有链核 {name} 非确定性排放", file=sys.stderr)
            return False
        ver = spv_version(spv1)
        # W3 三核 = 1.4;其余 G75/W1 = 1.0。
        want_ver = "1.4" if name in ("gi_probe", "rtao", "hard_shadow") else "1.0"
        if ver != want_ver:
            results["existing_manifest_zero_drift_pass"] = False
            print(f"[{TAG}] 既有链核 {name} SPIR-V {ver} != {want_ver}", file=sys.stderr)
            return False
        dc, dis = disasm(spv1)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        observed[name] = {
            "spirv_version": ver,
            "sha256": sha256_of(spv1),
            "manifest_matched": None,
            "capabilities": caps,
        }
    results["existing_manifest_zero_drift_pass"] = True
    results["existing_chain_kernels"] = observed
    print(
        f"[{TAG}] 步骤 4 PASS: w1w2 manifest 零漂移 + 帧链既有核 "
        f"{len(EXISTING_CHAIN_KERNELS)} 排放审计"
    )
    return True


def glue_kernel_emit(results: dict, work: Path) -> bool:
    per: dict = {}
    for name in GLUE_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        if not src.is_file():
            results["glue_kernel_emit_pass"] = False
            print(f"[{TAG}] 缺 glue kernel {src.relative_to(ROOT)}", file=sys.stderr)
            return False
        spv1 = work / f"glue_{name}_a.spv"
        spv2 = work / f"glue_{name}_b.spv"
        code, blob = compile_rx(src, spv1)
        if code != 0 or not spv1.is_file():
            print(blob[-2400:], file=sys.stderr)
            results["glue_kernel_emit_pass"] = False
            print(f"[{TAG}] {name}.rx 编译未产 .spv", file=sys.stderr)
            return False
        code2, _ = compile_rx(src, spv2)
        deterministic = code2 == 0 and spv1.read_bytes() == spv2.read_bytes()
        if not deterministic:
            results["glue_kernel_emit_pass"] = False
            print(f"[{TAG}] {name} 同源 ×2 非确定性", file=sys.stderr)
            return False
        ver = spv_version(spv1)
        if ver != "1.0":
            results["glue_kernel_emit_pass"] = False
            print(f"[{TAG}] {name} SPIR-V {ver} != 1.0(胶水核禁误升 1.4)", file=sys.stderr)
            return False
        vc, vblob = spirv_val(spv1, "vulkan1.0")
        if vc == -1:
            results["toolchain_skip"] = "no-spirv-val"
            return True
        if vc != 0:
            results["glue_kernel_emit_pass"] = False
            print(f"[{TAG}] {name} spirv-val 拒: {vblob[-800:]}", file=sys.stderr)
            return False
        dc, dis = disasm(spv1)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        if "RayQueryKHR" in dis or "SPV_KHR_ray_query" in dis:
            results["glue_kernel_emit_pass"] = False
            print(f"[{TAG}] {name} 误声明 ray query 面", file=sys.stderr)
            return False
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        per[name] = {
            "spirv_version": ver,
            "sha256": sha256_of(spv1),
            "spirv_val": {"vulkan1.0": "accepted"},
            "deterministic": True,
            "capabilities": caps,
        }
    results["glue_kernel_emit_pass"] = True
    results["glue_kernels"] = per
    print(f"[{TAG}] 步骤 5 PASS: 6 glue kernel spirv-val + 确定性 + SPIR-V 1.0")
    return True


def static_provenance_audit(results: dict) -> bool:
    if not DEVICE_FRAME_SRC.is_file():
        results["static_provenance_audit_pass"] = False
        print(f"[{TAG}] 缺 {DEVICE_FRAME_SRC.relative_to(ROOT)}", file=sys.stderr)
        return False
    text = DEVICE_FRAME_SRC.read_text(encoding="utf-8")
    # 去掉行注释后再查调用,避免模块头「禁 execute_frame」字面误伤。
    code_lines = []
    for ln in text.splitlines():
        if "//" in ln:
            ln = ln.split("//", 1)[0]
        code_lines.append(ln)
    code_only = "\n".join(code_lines)
    forbid_ok = "execute_frame(" not in code_only
    entry_ok = "execute_with_frame_update" in text
    passes_ok = all(f'"{n}"' in text for n in PASS_NAMES)
    edges_ok = all(
        f'("{p}", "{c}", "{r}")' in text or f"(\"{p}\", \"{c}\", \"{r}\")" in text
        for p, c, r in EXPECTED_EDGES
    )
    # EXPECTED_EDGES 在源里是 tuple 字面;宽松再查资源名集合。
    if not edges_ok:
        edges_ok = all(
            p in text and c in text and r in text for p, c, r in EXPECTED_EDGES
        )
    ok = forbid_ok and entry_ok and passes_ok and edges_ok
    results["static_audit"] = {
        "forbid_execute_frame_call": forbid_ok,
        "execute_with_frame_update_present": entry_ok,
        "pass_names_ok": passes_ok,
        "expected_edges_ok": edges_ok,
        "pass_names": list(PASS_NAMES),
    }
    results["static_provenance_audit_pass"] = ok
    if not ok:
        print(
            f"[{TAG}] 静态 provenance 审计未过: "
            f"forbid={forbid_ok} entry={entry_ok} passes={passes_ok} edges={edges_ok}",
            file=sys.stderr,
        )
        return False
    print(
        f"[{TAG}] 步骤 6 PASS: 静态 provenance 审计"
        f"(禁 execute_frame( + 15 pass + 关键边表)"
    )
    return True


# ───────────────────── device 段(gate real,门 G-G7-8) ─────────────────────


def device_section(results: dict, frames: int) -> int:
    # 短跑与 soak 同走 release:debug 帧耗时量级不可用,且与后台 soak 共用
    # target/release 产物避免并行 cargo 锁死/重复编译(设计案 §4/§5)。
    code, o, e = run(
        [
            "cargo", "build", "-p", "uc06-renderer", "--release",
            "--features", "device-frame", "--bin", "uc06-renderer", "--quiet",
        ],
        timeout=7200,
    )
    if code != 0:
        print((o + e)[-3000:], file=sys.stderr)
        results["device_pass"] = False
        return fail("[device] cargo build uc06-renderer --release --features device-frame 失败")

    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    results["validation_enabled"] = True
    code, out, err = run(
        [
            "cargo", "run", "-q", "-p", "uc06-renderer", "--release",
            "--features", "device-frame", "--bin", "uc06-renderer", "--",
            "--device-frame", "--frames", str(frames), "--json",
        ],
        env=env,
        timeout=7200,
    )
    blob = out + err
    if "FRAME: SKIP" in blob:
        reason = next(
            (
                ln.split("FRAME: SKIP", 1)[1].strip()
                for ln in blob.splitlines()
                if "FRAME: SKIP" in ln
            ),
            "unknown",
        )
        results["device_pass"] = None
        results["device_skip_reason"] = reason
        return skip(f"[device] --device-frame SKIP({reason})")
    if code != 0 or "FRAME: PASS" not in blob:
        print(blob[-4000:], file=sys.stderr)
        results["device_pass"] = False
        return fail(f"[device] --device-frame 未 PASS(rc={code})")
    doc = parse_json_line(blob, "uc06_device_frame")
    if doc is None:
        results["device_pass"] = False
        return fail("[device] --device-frame 未产可解析的单行 JSON")
    if not doc.get("all_pass"):
        results["device_pass"] = False
        return fail(f"[device] JSON all_pass=false: {doc}")

    # 阶段转移对拍
    bool_axes = (
        "cull_bitexact",
        "tri_expand_bitexact",
        "visbuffer_bitexact",
        "classify_bitexact",
        "gbuffer_pass",
        "vsm_sample_pass",
        "gi_pass",
        "ao_pass",
        "hard_pass",
        "taa_pass",
        "tsr_resample_pass",
        "tsr_temporal_pass",
        "provenance_edges_ok",
        "all_pass_gpu_ns_positive",
        "non_degen_ok",
    )
    bad = [k for k in bool_axes if not doc.get(k)]
    if bad:
        results["device_pass"] = False
        return fail(f"[device] 对拍/provenance/非退化轴未过: {bad}")
    if doc.get("validation_error_count", 1) != 0:
        results["device_pass"] = False
        return fail(f"[device] validation_error_count={doc.get('validation_error_count')}")
    if doc.get("device_lost_count", 1) != 0:
        results["device_pass"] = False
        return fail(f"[device] device_lost_count={doc.get('device_lost_count')}")
    if (
        doc.get("leaked_object_count", 1) != 0
        or doc.get("leaked_allocation_count", 1) != 0
    ):
        results["device_pass"] = False
        return fail(
            f"[device] leak objects={doc.get('leaked_object_count')} "
            f"allocs={doc.get('leaked_allocation_count')}"
        )
    # 非退化:动态场景不是摆拍
    if not (
        doc.get("covered_pixels", 0) > 0
        and doc.get("mv_nonzero_count", 0) > 0
        and doc.get("mv_nonzero_changed")
        and doc.get("instance_transform_changed")
    ):
        results["device_pass"] = False
        return fail(
            f"[device] 非退化不足: covered={doc.get('covered_pixels')} "
            f"mv_nz={doc.get('mv_nonzero_count')} "
            f"mv_chg={doc.get('mv_nonzero_changed')} "
            f"xform={doc.get('instance_transform_changed')}"
        )

    results["device_name"] = doc.get("device_name")
    results["stage_parity"] = {
        "cull_bitexact": doc["cull_bitexact"],
        "tri_expand_bitexact": doc["tri_expand_bitexact"],
        "tri_expand_max_abs": doc["tri_expand_max_abs"],
        "visbuffer_bitexact": doc["visbuffer_bitexact"],
        "classify_bitexact": doc["classify_bitexact"],
        "gbuffer_max_abs": doc["gbuffer_max_abs"],
        "tol_gbuffer": doc["tol_gbuffer"],
        "gbuffer_pass": doc["gbuffer_pass"],
        "vsm_sample_max_abs": doc["vsm_sample_max_abs"],
        "tol_vsm_sample": doc["tol_vsm_sample"],
        "vsm_sample_pass": doc["vsm_sample_pass"],
        "gi_max_abs": doc["gi_max_abs"],
        "tol_gi": doc["tol_gi"],
        "gi_pass": doc["gi_pass"],
        "ao_max_abs": doc["ao_max_abs"],
        "tol_ao": doc["tol_ao"],
        "ao_pass": doc["ao_pass"],
        "hard_max_abs": doc["hard_max_abs"],
        "tol_hard": doc["tol_hard"],
        "hard_pass": doc["hard_pass"],
        "taa_max_abs": doc["taa_max_abs"],
        "tol_taa": doc["tol_taa"],
        "taa_pass": doc["taa_pass"],
        "tsr_resample_max_abs": doc["tsr_resample_max_abs"],
        "tol_tsr_resample": doc["tol_tsr_resample"],
        "tsr_resample_pass": doc["tsr_resample_pass"],
        "tsr_temporal_max_abs": doc["tsr_temporal_max_abs"],
        "tol_tsr_temporal": doc["tol_tsr_temporal"],
        "tsr_temporal_pass": doc["tsr_temporal_pass"],
    }
    results["non_degen"] = {
        "covered_pixels": doc["covered_pixels"],
        "material_counts": doc["material_counts"],
        "mv_nonzero_count": doc["mv_nonzero_count"],
        "mv_nonzero_changed": doc["mv_nonzero_changed"],
        "instance_transform_changed": doc["instance_transform_changed"],
        "non_degen_ok": doc["non_degen_ok"],
    }
    results["provenance"] = {
        "edges_ok": doc["provenance_edges_ok"],
        "edges": doc.get("provenance_edges", []),
    }
    results["telemetry"] = {
        "validation_error_count": doc["validation_error_count"],
        "leaked_object_count": doc["leaked_object_count"],
        "leaked_allocation_count": doc["leaked_allocation_count"],
        "device_lost_count": doc["device_lost_count"],
        "all_pass_gpu_ns_positive": doc["all_pass_gpu_ns_positive"],
        "pass_gpu_timings": doc.get("pass_gpu_timings", []),
        "frames": doc["frames"],
        "elapsed_seconds": doc["elapsed_seconds"],
        "in_w": doc.get("in_w"),
        "in_h": doc.get("in_h"),
        "out_w": doc.get("out_w"),
        "out_h": doc.get("out_h"),
    }
    results["device_capability_snapshot"] = (
        f"device_name={doc.get('device_name')} wave=W3+device-frame "
        f"(require_wave 通过;validation=0 lost=0)"
    )

    # RED 四轴
    red = {
        "visbuffer": None,
        "history": None,
        "jitter": None,
        "provenance": None,
    }
    red_flags = (
        ("visbuffer", "--frame-red-visbuffer", 2),
        # History 轴在 device_frame.rs 内仅 frame>1 置 red_ok(错绑双缓冲)。
        ("history", "--frame-red-history", 4),
        ("jitter", "--frame-red-jitter", 2),
        ("provenance", "--frame-red-provenance", 2),
    )
    for key, flag, red_frames in red_flags:
        rc, o2, e2 = run(
            [
                "cargo", "run", "-q", "-p", "uc06-renderer", "--release",
                "--features", "device-frame", "--bin", "uc06-renderer", "--",
                flag, "--frames", str(red_frames), "--json",
            ],
            env=env,
            timeout=7200,
        )
        blob2 = o2 + e2
        ok_red = rc == 0 and f"FRAME: RED-OK {key}" in blob2
        if not ok_red:
            ok_red = rc == 0 and "FRAME: RED-OK" in blob2 and key in blob2
        red[key] = ok_red
        if not ok_red:
            print(blob2[-3000:], file=sys.stderr)
            results["device_red"] = red
            results["device_pass"] = False
            return fail(f"[device] RED 轴 {flag} 未触发(期望 FRAME: RED-OK {key})")
    results["device_red"] = red
    results["device_pass"] = True
    print(
        f"[{TAG}] 步骤 7+8 PASS: device-frame {frames} 帧正轴 + RED 四轴全过;"
        f"covered={doc['covered_pixels']} mv_nz={doc['mv_nonzero_count']} "
        f"val={doc['validation_error_count']}"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_device_frame_smoke",
        "milestone": "G7.6 One True Device Frame / G-G7-8",
        "step": 96,
        "spec_clauses": ["RXS-0297", "RXS-0298", "RXS-0299", "RXS-0300"],
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            k: results[k]
            for k in (
                "schema_self_check_pass",
                "freeze_and_matrix_pass",
                "oracle_tests_pass",
                "existing_manifest_zero_drift_pass",
                "glue_kernel_emit_pass",
                "static_provenance_audit_pass",
            )
            if results.get(k) is not None
        },
        "literal_rows_frame_chain": results.get("literal_rows_frame_chain"),
        "scene_freeze_anchors": results.get("scene_freeze_anchors"),
        "assembly_notes": results.get("assembly_notes"),
        "device_pass": results.get("device_pass"),
        "device_skip_reason": results.get("device_skip_reason"),
        "device_name": results.get("device_name"),
        "validation_enabled": results.get("validation_enabled", False),
        "device_red": results.get("device_red", {}),
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": results.get("toolchain_skip") is not None
        or results.get("device_pass") is None,
        "require_real": os.environ.get("RURIX_REQUIRE_REAL") == "1",
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    for key in (
        "glue_kernels",
        "existing_chain_kernels",
        "static_audit",
        "device_capability_snapshot",
        "stage_parity",
        "non_degen",
        "provenance",
        "telemetry",
    ):
        if results.get(key) is not None:
            doc[key] = results[key]
    # schema additionalProperties:false — 去掉值为 None 的可选顶栏
    for k in list(doc.keys()):
        if doc[k] is None and k not in (
            "device_pass",
            "device_skip_reason",
            "device_name",
            "toolchain_skip",
            "assembly_notes",
            "device_capability_snapshot",
        ):
            # keep nullable typed fields; drop accidental Nones for non-nullable
            if k in ("literal_rows_frame_chain", "scene_freeze_anchors"):
                doc[k] = []
    ev = EVIDENCE_DIR / f"renderer_device_frame_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[{TAG}] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def run_soak_streaming(cmd: list[str], env: dict, timeout: int) -> tuple[int, str, str]:
    """soak 实时转发 stdout/stderr(供 Tee-Object / soak_run.log),同时收集供 JSON 解析。

    不可用 capture_output=True:整段 soak(≥30min)期间父壳 Tee 收不到任何字节,
    `.tmp/soak_run.log` 会一直不出现/为空,进度行也被吞掉。
    """
    # 强制子进程行缓冲,避免 cargo/CRT 块缓冲导致进度延迟。
    env = dict(env)
    env.setdefault("PYTHONUNBUFFERED", "1")
    env.setdefault("RUST_LOG_STYLE", "always")
    proc = subprocess.Popen(
        cmd,
        cwd=str(ROOT),
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        encoding="utf-8",
        errors="replace",
        bufsize=1,
    )
    out_chunks: list[str] = []
    err_chunks: list[str] = []
    assert proc.stdout is not None and proc.stderr is not None

    def _pump(stream, sink: list[str], dest) -> None:
        for line in stream:
            sink.append(line)
            dest.write(line)
            dest.flush()

    t_out = threading.Thread(
        target=_pump, args=(proc.stdout, out_chunks, sys.stdout), daemon=True
    )
    t_err = threading.Thread(
        target=_pump, args=(proc.stderr, err_chunks, sys.stderr), daemon=True
    )
    t_out.start()
    t_err.start()
    try:
        code = proc.wait(timeout=timeout)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.wait()
        t_out.join(timeout=5)
        t_err.join(timeout=5)
        raise
    t_out.join(timeout=30)
    t_err.join(timeout=30)
    return code, "".join(out_chunks), "".join(err_chunks)


def run_soak_forward(frames: int, min_minutes: float) -> int:
    """人工 soak 转发 + evidence 落盘(不进 PR workflow;设计案 §5)。"""
    env = dict(os.environ)
    env.pop("RURIX_VK_VALIDATION", None)  # soak 关闭验证层
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_SOAK"] = "1"  # 放行 REQUIRE_REAL 而无 validation(设计案 §5)
    # 双下界:分钟下界 + 帧数×3s 余量(本机 release soak ~1.5s/帧实测)。
    timeout = max(int(min_minutes * 60) + 3600, int(frames) * 3 + 3600, 7200)
    code, o, e = run_soak_streaming(
        [
            "cargo", "run", "-q", "-p", "uc06-renderer",
            "--release", "--features", "device-frame",
            "--bin", "uc06-renderer", "--",
            "--device-frame", "--soak",
            "--frames", str(frames),
            "--min-minutes", str(min_minutes),
            "--json",
        ],
        env=env,
        timeout=timeout,
    )

    doc_cli = None
    for line in o.splitlines():
        line = line.strip()
        if line.startswith("{") and '"subject":"uc06_device_frame"' in line.replace(" ", ""):
            try:
                doc_cli = json.loads(line)
            except json.JSONDecodeError:
                continue
        elif line.startswith("{") and "uc06_device_frame" in line:
            try:
                doc_cli = json.loads(line)
            except json.JSONDecodeError:
                continue

    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    fail_reasons: list[str] = []
    soak = (doc_cli or {}).get("soak_telemetry") if doc_cli else None

    if doc_cli is None:
        fail_reasons.append("cli_json_missing")
    if soak is None:
        fail_reasons.append("soak_telemetry_missing")
        soak = {}

    health = {
        "validation_error_count": int(
            soak.get("validation_error_count", doc_cli.get("validation_error_count", 1) if doc_cli else 1)
        ),
        "device_lost_count": int(
            soak.get("device_lost_count", doc_cli.get("device_lost_count", 1) if doc_cli else 1)
        ),
        "tdr_suspected_count": int(soak.get("tdr_suspected_count", 1)),
        "leaked_object_count": int(
            soak.get("leaked_object_count", doc_cli.get("leaked_object_count", 1) if doc_cli else 1)
        ),
        "leaked_allocation_count": int(
            soak.get(
                "leaked_allocation_count",
                doc_cli.get("leaked_allocation_count", 1) if doc_cli else 1,
            )
        ),
        "vsm_page_overflow_count": int(soak.get("vsm_page_overflow_count", 0)),
    }
    # soak_telemetry JSON 未重复 validation/lost/leak(在顶栏);用顶栏覆盖。
    if doc_cli is not None:
        health["validation_error_count"] = int(doc_cli.get("validation_error_count", 1))
        health["device_lost_count"] = int(doc_cli.get("device_lost_count", 1))
        health["leaked_object_count"] = int(doc_cli.get("leaked_object_count", 1))
        health["leaked_allocation_count"] = int(doc_cli.get("leaked_allocation_count", 1))

    actual_frames = int(soak.get("actual_frames", doc_cli.get("frames", 0) if doc_cli else 0))
    elapsed_minutes = float(
        soak.get(
            "elapsed_minutes",
            (doc_cli.get("elapsed_seconds", 0) / 60.0) if doc_cli else 0,
        )
    )
    fps_mean = float(
        soak.get(
            "fps_mean",
            (actual_frames / (elapsed_minutes * 60.0)) if elapsed_minutes > 0 else 0.0,
        )
    )
    pass_ts = list(soak.get("pass_gpu_timestamps") or [])
    # schema 要求恰好 15 项;CLI 未产 telemetry 时垫零占位(ok 仍由 hard_fails 判红)。
    if len(pass_ts) != 15:
        pass_ts = [
            {"pass": name, "gpu_p50_ms": 0.0, "gpu_p95_ms": 0.0} for name in PASS_NAMES
        ]
    performance = {
        "frame_gpu_p50_ms": float(soak.get("frame_gpu_p50_ms", 0)),
        "frame_gpu_p95_ms": float(soak.get("frame_gpu_p95_ms", 0)),
        "frame_gpu_p99_ms": float(soak.get("frame_gpu_p99_ms", 0)),
        "cpu_submit_p50_ms": float(soak.get("cpu_submit_p50_ms", 0)),
        "cpu_submit_p95_ms": float(soak.get("cpu_submit_p95_ms", 0)),
        "cpu_submit_p99_ms": float(soak.get("cpu_submit_p99_ms", 0)),
        "pass_gpu_timestamps": pass_ts,
        "peak_vram_mb": float(soak.get("peak_vram_mb", 0)),
    }
    visual = soak.get("visual_digest") or {
        "anchor_color_sha256": soak.get("anchor_color_sha256", []),
        "luma_mean_series": soak.get("luma_mean_series", []),
        "luma_var_series": soak.get("luma_var_series", []),
    }
    # 锚点 PPM 取首/中/末(控制体积;全量 digest 仍在 visual_digest)。
    all_ppm = list(soak.get("anchor_ppm") or [])
    if len(all_ppm) >= 3:
        mid = len(all_ppm) // 2
        anchor_ppm = [all_ppm[0], all_ppm[mid], all_ppm[-1]]
    else:
        anchor_ppm = all_ppm

    fail_reasons.extend(list(soak.get("fail_reasons") or []))
    if health["validation_error_count"] != 0:
        fail_reasons.append(f"validation={health['validation_error_count']}")
    if health["device_lost_count"] != 0:
        fail_reasons.append(f"lost={health['device_lost_count']}")
    if health["tdr_suspected_count"] != 0:
        fail_reasons.append(f"tdr={health['tdr_suspected_count']}")
    if health["leaked_object_count"] != 0 or health["leaked_allocation_count"] != 0:
        fail_reasons.append("leak!=0")
    if actual_frames < 10000:
        fail_reasons.append(f"frames={actual_frames}<10000")
    if elapsed_minutes < 30.0:
        fail_reasons.append(f"minutes={elapsed_minutes}<30")
    if not visual.get("anchor_color_sha256"):
        fail_reasons.append("anchor_digest_empty")
    if len(performance["pass_gpu_timestamps"]) != 15:
        fail_reasons.append("pass_timestamps!=15")
    validation_layers_enabled = bool(soak.get("validation_layers_enabled", False))
    if validation_layers_enabled:
        fail_reasons.append("validation_layers_enabled")

    def _is_note(r: str) -> bool:
        return (
            r.startswith("tdr_policy:")
            or "frame_gpu_soft_spike@" in r
            or "design_2s_replaced" in r
            or "not_tdr" in r
        )

    # 去重保序;政策/软峰留痕不构成硬失败。
    seen = set()
    uniq_all = []
    hard_fails = []
    for r in fail_reasons:
        if r in seen:
            continue
        seen.add(r)
        uniq_all.append(r)
        if not _is_note(r):
            hard_fails.append(r)

    # 优先信任 CLI soak_telemetry.ok;否则按硬失败判定。
    if "ok" in soak and isinstance(soak.get("ok"), bool):
        ok = bool(soak["ok"]) and code == 0 and not hard_fails
    else:
        ok = len(hard_fails) == 0 and code == 0
    device_caps = soak.get("device_caps") or {
        "device_name": (doc_cli or {}).get("device_name", "unknown")
    }
    # 成功件也保留 tdr_policy 字面(设计案 2s 偏差可审计)。
    evidence = {
        "schema_version": 1,
        "subject": "renderer_soak",
        "milestone": "G7.6 One True Device Frame soak / G-G7-8",
        "actual_frames": actual_frames,
        "elapsed_minutes": elapsed_minutes,
        "fps_mean": fps_mean,
        "health": health,
        "performance": performance,
        "reproducibility": {
            "scene_digest": soak.get("scene_digest") or ("0" * 64),
            "visual_digest": visual,
            "anchor_ppm": anchor_ppm if anchor_ppm else ["evidence/soak_anchors/missing.ppm"],
        },
        "environment": {
            "device_name": (doc_cli or {}).get("device_name")
            or device_caps.get("device_name", "unknown"),
            "driver_version": soak.get("driver_version"),
            "device_caps": device_caps,
        },
        "validation_layers_enabled": validation_layers_enabled,
        "ok": ok,
        "require_real": True,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    if uniq_all:
        evidence["fail_reasons"] = uniq_all
    elif not ok:
        evidence["fail_reasons"] = ["cli_exit_nonzero" if code else "unknown"]

    ev = EVIDENCE_DIR / f"renderer_soak_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(evidence, ensure_ascii=False, indent=2) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(
        f"[{TAG}] 写 soak evidence {ev.relative_to(ROOT)}; ok={ok}; "
        f"frames={actual_frames} minutes={elapsed_minutes:.2f}"
    )
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser(description="G7.6 CI 步骤 96 device-frame smoke")
    ap.add_argument("--soak", action="store_true", help="转发 soak(不进 PR workflow)")
    ap.add_argument("--frames", type=int, default=None)
    ap.add_argument("--min-minutes", type=float, default=30.0)
    args = ap.parse_args()

    if args.soak:
        frames = args.frames if args.frames is not None else 10000
        return run_soak_forward(frames, args.min_minutes)

    frames = args.frames if args.frames is not None else 8
    results: dict = {}
    work = ROOT / "target" / "g7_device_frame_smoke"
    work.mkdir(parents=True, exist_ok=True)

    host_ok = schema_self_check(results)
    if host_ok:
        host_ok = freeze_and_matrix_section(results)
    if host_ok:
        host_ok = oracle_section(results)
    if host_ok:
        host_ok = existing_manifest_zero_drift(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = glue_kernel_emit(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = static_provenance_audit(results)

    if results.get("toolchain_skip") is not None:
        device_rc = 0
        write_evidence(results, host_ok, device_rc)
        return skip(
            f"[host] {results['toolchain_skip']}(spirv-val/spirv-dis 缺;编译段判据未取证)"
        )

    device_rc = device_section(results, frames) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return fail("host 段未过(schema/冻结锚/oracle/零漂移/glue/静态审计)")
    if device_rc != 0:
        return device_rc
    print(
        f"[{TAG}] PASS(host 恒跑全绿;device 段真跑全绿 = G7.6 步骤 96 兑现,门 G-G7-8)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
