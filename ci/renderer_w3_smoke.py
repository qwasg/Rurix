#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""renderer W3 三效果核冒烟(步骤 94;G7.4 W3c;RFC-0018 章 A/C/D;验收门 G-G7-6)。

host 段(**恒跑**,需 Vulkan SDK 的 `spirv-val`/`spirv-dis`;缺工具 → SKIP):
  1. host BVH/reference 三效果 oracle 单测:`rurix-render` 的 `rt::`(bvh/ref_tracer/
     effects/as_manager/denoise)与 `gi::`(pipeline/probe/sh/tracer/interpolate/
     filter/temporal)全量恒绿 —— **oracle 数值语义 0-byte** 的回归网。
  2. AS/lifetime 审计(RFC-0018 §C1/C4):`rt::as_manager` 单测 + `rurix-render` 的
     `#![forbid(unsafe_code)]` 在位 + `unsafe-audit/rurix-rt.md` 的 U30(AS/SBT/
     device-address)边界登记在位(新 unsafe 优先复用 U30,不另开边界)。
  3. 三 kernel 真实 `.rx` → `.spv`:`gi_probe`/`rtao`/`hard_shadow` 经
     `rurixc --target vulkan` 产物须 SPIR-V **1.4**,`spirv-val --target-env
     vulkan1.2` **与** `spv1.4` 双口径皆 accept(退出码判定)。
  4. 反汇编 golden 锚定:三 kernel 并集须覆盖 `OpTypeAccelerationStructureKHR` /
     `OpTypeRayQueryKHR` / `OpRayQueryInitializeKHR` / `OpRayQueryProceedKHR` /
     `OpRayQueryGetIntersectionTypeKHR` + committed 五查询族(含 G7.4 路 A 实现兑现的
     `OpRayQueryGetIntersectionBarycentricsKHR` **分量真实消费**)。
  5. 单 TLAS 纪律静态审计(RXS-0297):每个 kernel 签名 `AccelStruct` 形参**恰好一个**。
  6. W1/W2 零漂移门:五 kernel 逐件对 `tests/vulkan/w1w2_spv_manifest.json` 的
     sha256 + SPIR-V 版本 + capability 集合比对,且不得出现 ray query 能力声明。
  7. RED 反证:篡改三 kernel 之一的 `.spv` 单字节 → `spirv-val` 必拒。

device 段(**gate real**,`RURIX_REQUIRE_REAL=1`;门 G-G7-6):
  8. `uc06-renderer --w3-effects`(feature vulkan,`RURIX_VK_VALIDATION=1`):
     三 kernel 在**一次** `VkAsManager` 建面(3 BLAS × 3 实例 = 冻结场景 764 三角形)
     + 一条 command buffer + 单次提交中依次 dispatch,**同一个 TLAS 句柄**写入三个
     descriptor set(identity 机验);逐量对拍 host oracle:
       · hit/miss、instance index、primitive index、geometryIndex → **零容差**;
       · committed_t、barycentric 分量、GI 辐射度、RTAO AO、硬阴影可见性 →
         measured 与**冻结**容差成对机验(measured ≤ tol)。
  9. RED 三轴:`--w3-red-tamper`(篡改 device 侧场景顶点 → 对拍必红的**数据流反证**)
     + 注入式 stale-tlas / wrong-barrier(validation VUID 拦截 → fail-closed)。
  无 Vulkan 设备/能力链缺失 → SKIP=dev-env degrade;`RURIX_REQUIRE_REAL=1` 翻硬红
  (不以 host 段绿冒充 device 绿,RFC-0016 §4.E3 / §9.1 R-3 纪律)。
"""
from __future__ import annotations

import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
MANIFEST = ROOT / "tests" / "vulkan" / "w1w2_spv_manifest.json"
KERNEL_DIR = ROOT / "apps" / "uc06-renderer" / "kernels"
UNSAFE_AUDIT = ROOT / "unsafe-audit" / "rurix-rt.md"
RENDER_LIB = ROOT / "src" / "rurix-render" / "src" / "lib.rs"

TAG = "renderer_w3_smoke"

# W3c 三效果核(共用同一真实 TLAS)。
W3_KERNELS = ("gi_probe", "rtao", "hard_shadow")

# 反汇编 golden:每件 compute RayQuery 模块必然出现的指令。
GOLDEN_PER_FILE = (
    "OpTypeAccelerationStructureKHR",
    "OpTypeRayQueryKHR",
    "OpRayQueryInitializeKHR",
    "OpRayQueryProceedKHR",
    "OpRayQueryGetIntersectionTypeKHR",
)
# committed 查询族:按真实使用,由三 kernel **并集**覆盖(gi_probe 承载全五项)。
GOLDEN_CORPUS_UNION = (
    "OpRayQueryGetIntersectionTKHR",
    "OpRayQueryGetIntersectionBarycentricsKHR",
    "OpRayQueryGetIntersectionInstanceIdKHR",
    "OpRayQueryGetIntersectionPrimitiveIndexKHR",
    "OpRayQueryGetIntersectionGeometryIndexKHR",
)


def fail(msg: str) -> int:
    print(f"[{TAG}] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[{TAG}] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, env=None, timeout: int = 3600):
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
    """定位序沿 RXS-0212:env 覆盖(绝对路径)> PATH。"""
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


# ───────────────────────── host 段(恒跑) ─────────────────────────


def oracle_section(results: dict) -> bool:
    """步骤 1:host BVH/reference 三效果 oracle 单测(数值语义 0-byte 回归网)。"""
    ok = True
    for filt, label in (("rt::", "rt(bvh/ref_tracer/effects/as_manager/denoise)"),
                        ("gi::", "gi(pipeline/probe/sh/tracer/interpolate/filter/temporal)")):
        code, o, e = run(
            ["cargo", "test", "-q", "-p", "rurix-render", "--lib", "--", filt]
        )
        if code != 0:
            print((o + e)[-2400:], file=sys.stderr)
            print(f"[{TAG}] host oracle 单测未过: {label}", file=sys.stderr)
            ok = False
    results["oracle_tests_pass"] = ok
    if ok:
        print(f"[{TAG}] 步骤 1 PASS: host 三效果 oracle(rt:: + gi::)全量恒绿")
    return ok


def as_lifetime_section(results: dict) -> bool:
    """步骤 2:AS 生命周期/所有权审计(RFC-0018 §C1/C4)。"""
    code, o, e = run(
        ["cargo", "test", "-q", "-p", "rurix-render", "--lib", "--", "rt::as_manager"]
    )
    ok = code == 0
    if not ok:
        print((o + e)[-2000:], file=sys.stderr)
        print(f"[{TAG}] rt::as_manager 单测未过(AS 策略单源)", file=sys.stderr)
    # rurix-render 维持 forbid(unsafe_code)(host oracle crate 零 unsafe)。
    if not RENDER_LIB.is_file() or "#![forbid(unsafe_code)]" not in RENDER_LIB.read_text(
        encoding="utf-8"
    ):
        print(f"[{TAG}] rurix-render 缺 #![forbid(unsafe_code)](冻结面)", file=sys.stderr)
        ok = False
    # 新 unsafe 优先复用 U30(AS/SBT/device-address)边界:登记须在位。
    if not UNSAFE_AUDIT.is_file() or "U30" not in UNSAFE_AUDIT.read_text(encoding="utf-8"):
        print(f"[{TAG}] unsafe-audit 缺 U30 边界登记(compute AS descriptor 消费臂扩注)",
              file=sys.stderr)
        ok = False
    results["as_lifetime_audit_pass"] = ok
    if ok:
        print(f"[{TAG}] 步骤 2 PASS: AS/lifetime 审计(as_manager 单源 + forbid(unsafe) + U30 登记)")
    return ok


def kernel_emit_section(results: dict, work: Path) -> bool:
    """步骤 3+4+5:三 kernel → SPIR-V 1.4 + 双口径 spirv-val + golden + 单 TLAS 纪律。"""
    per_kernel: dict = {}
    union: set[str] = set()
    for name in W3_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        if not src.is_file():
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] 缺 W3 kernel 源 {src.relative_to(ROOT)}", file=sys.stderr)
            return False
        # 单 TLAS 纪律静态审计(RXS-0297:签名 AccelStruct 形参恰好一个)。
        text = src.read_text(encoding="utf-8")
        sig = text.split("kernel fn", 1)[1].split(")", 1)[0] if "kernel fn" in text else ""
        accel_params = sig.count("AccelStruct")
        spv = work / f"{name}.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2400:], file=sys.stderr)
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name}.rx 编译未产 .spv", file=sys.stderr)
            return False
        ver = spv_version(spv)
        if ver != "1.4":
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} SPIR-V 版本 {ver} != 1.4(RXS-0300 升版判定)",
                  file=sys.stderr)
            return False
        envs = {}
        for env in ("vulkan1.2", "spv1.4"):
            vc, vblob = spirv_val(spv, env)
            if vc == -1:
                results["toolchain_skip"] = "no-spirv-val"
                return True
            if vc != 0:
                results["kernel_emit_pass"] = False
                print(f"[{TAG}] {name} spirv-val --target-env {env} 拒: {vblob[-800:]}",
                      file=sys.stderr)
                return False
            envs[env] = "accepted"
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        if dc != 0:
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} spirv-dis 失败: {dis[-800:]}", file=sys.stderr)
            return False
        missing = [m for m in GOLDEN_PER_FILE if m not in dis]
        if missing:
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} 反汇编 golden(per-file 必含)缺: {missing}", file=sys.stderr)
            return False
        if "RayQueryKHR" not in dis or "SPV_KHR_ray_query" not in dis:
            results["kernel_emit_pass"] = False
            print(f"[{TAG}] {name} 缺 RayQueryKHR capability / SPV_KHR_ray_query",
                  file=sys.stderr)
            return False
        found = sorted(m for m in GOLDEN_PER_FILE + GOLDEN_CORPUS_UNION if m in dis)
        union |= set(found)
        per_kernel[name] = {
            "spirv_version": ver,
            "sha256": sha256_of(spv),
            "spirv_val": envs,
            "accel_struct_params": accel_params,
            "golden_mnemonics": found,
        }
    # 单 TLAS 纪律:每 kernel 恰好一个 AccelStruct 形参。
    bad = {k: v["accel_struct_params"] for k, v in per_kernel.items()
           if v["accel_struct_params"] != 1}
    results["single_tlas_discipline_pass"] = not bad
    if bad:
        results["kernel_emit_pass"] = False
        print(f"[{TAG}] 单 TLAS 纪律违例(AccelStruct 形参数 != 1): {bad}", file=sys.stderr)
        return False
    required = set(GOLDEN_PER_FILE) | set(GOLDEN_CORPUS_UNION)
    if not required <= union:
        results["kernel_emit_pass"] = False
        print(f"[{TAG}] golden 最小集并集覆盖不全: 缺 {sorted(required - union)}",
              file=sys.stderr)
        return False
    results["kernel_emit_pass"] = True
    results["kernels"] = per_kernel
    print(f"[{TAG}] 步骤 3+4+5 PASS: 三核 1.4 + spirv-val 双口径 + golden 并集 + 单 TLAS 纪律")
    return True


def w1w2_zero_drift_section(results: dict, work: Path) -> bool:
    """步骤 6:W1/W2 五 kernel 逐字节 + capability 声明零漂移(既有判据只增不改)。"""
    if not MANIFEST.is_file():
        results["w1w2_zero_drift_pass"] = False
        print(f"[{TAG}] 缺 golden manifest {MANIFEST.relative_to(ROOT)}", file=sys.stderr)
        return False
    expected = json.loads(MANIFEST.read_text(encoding="utf-8"))["kernels"]
    for name, want in sorted(expected.items()):
        src = KERNEL_DIR / f"{name}.rx"
        spv = work / f"w1w2_{name}.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2000:], file=sys.stderr)
            results["w1w2_zero_drift_pass"] = False
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
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} **漂移**: ver={ver} sha={digest} caps={caps} "
                  f"vs manifest {want}(零漂移门:既有 golden 不重 bless)", file=sys.stderr)
            return False
        if "RayQueryKHR" in caps or "SPV_KHR_ray_query" in dis:
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 误声明 ray query 面", file=sys.stderr)
            return False
    results["w1w2_zero_drift_pass"] = True
    print(f"[{TAG}] 步骤 6 PASS: W1/W2 {len(expected)} kernel 字节 + 版本 + capability 零漂移")
    return True


def red_tamper_spv_section(results: dict, work: Path) -> bool:
    """步骤 7:篡改 .spv 单字节 → spirv-val 必拒(编译校验轴生效反证)。"""
    spv = work / "gi_probe.spv"
    if not spv.is_file():
        results["red_tamper_spv_pass"] = False
        print(f"[{TAG}] 缺 {spv.name},无法做 RED 反证", file=sys.stderr)
        return False
    raw = bytearray(spv.read_bytes())
    raw[0] ^= 0xFF
    bad = work / "gi_probe_tampered.spv"
    bad.write_bytes(bytes(raw))
    code, _ = spirv_val(bad, None)
    if code == -1:
        results["toolchain_skip"] = "no-spirv-val"
        return True
    if code == 0:
        results["red_tamper_spv_pass"] = False
        print(f"[{TAG}] 篡改后的 .spv 仍被 spirv-val 接受(校验轴失效)", file=sys.stderr)
        return False
    results["red_tamper_spv_pass"] = True
    print(f"[{TAG}] 步骤 7 PASS: 篡改 .spv → spirv-val 拒(退出码 {code}),校验轴生效")
    return True


# ───────────────────── device 段(gate real,门 G-G7-6) ─────────────────────


def device_section(results: dict) -> int:
    """步骤 8+9:三核共用同一真实 TLAS device 真跑 + 对拍 + RED 三轴。"""
    code, o, e = run(
        ["cargo", "build", "-p", "uc06-renderer", "--features", "vulkan",
         "--bin", "uc06-renderer", "--quiet"]
    )
    if code != 0:
        print((o + e)[-2400:], file=sys.stderr)
        results["device_pass"] = False
        return fail("[device] cargo build uc06-renderer --features vulkan 失败(host 编译红)")

    # validation 开启真跑(G-G7-6「validation 零错误」;messenger ERROR → fail-closed Err)。
    env = dict(os.environ, RURIX_VK_VALIDATION="1")
    results["validation_enabled"] = True
    code, out, err = run(
        ["cargo", "run", "-q", "-p", "uc06-renderer", "--features", "vulkan",
         "--bin", "uc06-renderer", "--", "--w3-effects"],
        env=env,
    )
    blob = out + err
    red = {
        "tamper_geometry": None,
        "stale_tlas": "W3: RED-OK stale-tlas" in blob,
        "wrong_barrier": "W3: RED-OK wrong-barrier" in blob,
        "missing_capability": None,
    }
    results["device_red"] = red
    if "W3: SKIP" in blob:
        reason = next(
            (ln.split("W3: SKIP", 1)[1].strip() for ln in blob.splitlines() if "W3: SKIP" in ln),
            "unknown",
        )
        results["device_pass"] = None
        results["device_skip_reason"] = reason
        return skip(f"[device] uc06-renderer --w3-effects SKIP({reason})")
    if code != 0 or "W3: PASS" not in blob:
        print(blob[-3000:], file=sys.stderr)
        results["device_pass"] = False
        return fail(f"[device] --w3-effects 未 PASS(rc={code})")
    doc = None
    for line in blob.splitlines():
        line = line.strip()
        if line.startswith("{") and "uc06_w3_effects" in line:
            try:
                doc = json.loads(line)
            except json.JSONDecodeError:
                doc = None
    if doc is None:
        results["device_pass"] = False
        return fail("[device] --w3-effects 未产可解析的单行 JSON")

    # ── 机验:共用同一 TLAS + 零容差量 + measured ≤ 冻结容差 ──
    if not doc.get("shared_tlas"):
        results["device_pass"] = False
        return fail(f"[device] 三 dispatch 未共用同一 TLAS: {doc.get('dispatch_tlas')}")
    zero_tol = {
        "geom_hit_mismatches": doc.get("geom_hit_mismatches"),
        "geom_instance_mismatches": doc.get("geom_instance_mismatches"),
        "geom_primitive_mismatches": doc.get("geom_primitive_mismatches"),
        "geom_geometry_nonzero": doc.get("geom_geometry_nonzero"),
    }
    bad_zero = {k: v for k, v in zero_tol.items() if v != 0}
    if bad_zero:
        results["device_pass"] = False
        return fail(f"[device] 零容差量非零(hit/miss 与索引类): {bad_zero}")
    pairs = (
        ("measured_t_max_abs", "tol_t"),
        ("measured_bary_max_abs", "tol_bary"),
        ("measured_radiance_max_abs", "tol_radiance"),
        ("measured_ao_max_abs", "tol_ao"),
        ("measured_visibility_max_abs", "tol_visibility"),
    )
    for m, t in pairs:
        if doc.get(m) is None or doc.get(t) is None:
            results["device_pass"] = False
            return fail(f"[device] evidence 缺 measured/tol 成对字段: {m}/{t}")
        if float(doc[m]) > float(doc[t]):
            results["device_pass"] = False
            return fail(f"[device] {m}={doc[m]} > {t}={doc[t]}(冻结容差外)")
    if not (doc.get("gi_probe_pass") and doc.get("rtao_pass") and doc.get("hard_shadow_pass")):
        results["device_pass"] = False
        return fail(f"[device] 逐核判定未全过: {doc}")

    results["device_name"] = doc.get("device_name")
    results["shared_tlas"] = {
        "tlas_identity": doc["tlas_identity"],
        "dispatch_tlas": doc["dispatch_tlas"],
        "identical": True,
    }
    results["scene"] = {
        "blas_count": doc["blas_count"],
        "instance_count": doc["instance_count"],
        "triangle_count": doc["triangle_count"],
        "probe_rays": doc["probe_rays"],
        "gbuffer_pixels": doc["gbuffer_pixels"],
    }
    results["geometry_parity"] = {
        "hit_mismatches": doc["geom_hit_mismatches"],
        "instance_mismatches": doc["geom_instance_mismatches"],
        "primitive_mismatches": doc["geom_primitive_mismatches"],
        "geometry_index_nonzero": doc["geom_geometry_nonzero"],
        "measured_t_max_abs": doc["measured_t_max_abs"],
        "tol_t": doc["tol_t"],
        "measured_bary_max_abs": doc["measured_bary_max_abs"],
        "tol_bary": doc["tol_bary"],
    }
    results["effect_parity"] = {
        "measured_radiance_max_abs": doc["measured_radiance_max_abs"],
        "tol_radiance": doc["tol_radiance"],
        "measured_ao_max_abs": doc["measured_ao_max_abs"],
        "tol_ao": doc["tol_ao"],
        "measured_visibility_max_abs": doc["measured_visibility_max_abs"],
        "tol_visibility": doc["tol_visibility"],
        "gi_probe_pass": doc["gi_probe_pass"],
        "rtao_pass": doc["rtao_pass"],
        "hard_shadow_pass": doc["hard_shadow_pass"],
    }
    results["effect_stats"] = {
        "ao_mean_device": doc["ao_mean_device"],
        "ao_occluded_pixels": doc["ao_occluded_pixels"],
        "shadowed_ratio_device": doc["shadowed_ratio_device"],
        "radiance_nonzero_ratio_device": doc["radiance_nonzero_ratio_device"],
    }
    results["input_provenance"] = {
        "rtao_dirs": doc["rtao_dirs_provenance"],
        "rtao_seed": "0x5255525855430006(uc06 冻结默认种子)",
        "rtao_samples_per_pixel": 8,
        "rtao_radius": 2.0,
        "gbuffer_source": "冻结相机 64x36 针孔探针网格对冻结 TLAS 求交后的命中点压实"
                          "(位置 + 世界法线;全有效)",
        "invalid_pixel_arm": "不在 device kernel 表达(NaN/inf 位置、零长法线/光方向由 host "
                            "oracle 单测覆盖);miss 轴由探针网格未命中光线在 device 真实覆盖",
    }
    # capability snapshot(与步骤 93 同源探测面)。
    results["device_capability_snapshot"] = (
        f"device_name={doc.get('device_name')} wave=W3(七能力链 require_wave 通过)"
    )

    # ── RED 轴 ①:篡改 device 侧几何 → 对拍必红(数据流反证)──
    code, out2, err2 = run(
        ["cargo", "run", "-q", "-p", "uc06-renderer", "--features", "vulkan",
         "--bin", "uc06-renderer", "--", "--w3-red-tamper"],
        env=env,
    )
    blob2 = out2 + err2
    red["tamper_geometry"] = "W3: RED-OK tamper-geometry" in blob2
    if code != 0 or not red["tamper_geometry"]:
        print(blob2[-2400:], file=sys.stderr)
        results["device_pass"] = False
        return fail("[device] RED-tamper-geometry 失效(篡改 device 顶点后对拍仍通过)")
    if not (red["stale_tlas"] and red["wrong_barrier"]):
        results["device_pass"] = False
        return fail(f"[device] 注入式 RED 轴不全: {red}")
    results["device_pass"] = True
    print(
        f"[{TAG}] 步骤 8+9 PASS: 三核共用同一真实 TLAS device 真跑 "
        f"(t={doc['measured_t_max_abs']:.3e} bary={doc['measured_bary_max_abs']:.3e} "
        f"radiance={doc['measured_radiance_max_abs']:.3e} ao={doc['measured_ao_max_abs']:.3e} "
        f"visibility={doc['measured_visibility_max_abs']:.3e})+ RED 三轴(篡改几何/过期 TLAS/"
        f"错误 barrier)全过;validation 零错误"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "renderer_w3_smoke",
        "milestone": "G7.4 W3c / G-G7-6 (RFC-0018 章 A/C/D)",
        "step": 94,
        "spec_clauses": ["RXS-0297", "RXS-0298", "RXS-0299", "RXS-0300"],
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {
            k: results[k]
            for k in (
                "oracle_tests_pass",
                "as_lifetime_audit_pass",
                "kernel_emit_pass",
                "single_tlas_discipline_pass",
                "w1w2_zero_drift_pass",
                "red_tamper_spv_pass",
            )
            if results.get(k) is not None
        },
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
        "kernels",
        "device_capability_snapshot",
        "shared_tlas",
        "scene",
        "geometry_parity",
        "effect_parity",
        "effect_stats",
        "input_provenance",
    ):
        if results.get(key) is not None:
            doc[key] = results[key]
    ev = EVIDENCE_DIR / f"renderer_w3_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[{TAG}] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    work = ROOT / "target" / "g7_w3c_smoke"
    work.mkdir(parents=True, exist_ok=True)

    host_ok = oracle_section(results)
    if host_ok:
        host_ok = as_lifetime_section(results)
    if host_ok:
        host_ok = kernel_emit_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = w1w2_zero_drift_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = red_tamper_spv_section(results, work)

    if results.get("toolchain_skip") is not None:
        device_rc = 0
        write_evidence(results, host_ok, device_rc)
        return skip(f"[host] {results['toolchain_skip']}(spirv-val/spirv-dis 缺;编译段判据未取证)")

    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return fail("host 段未过(oracle/编译/零漂移/RED 反证)")
    if device_rc != 0:
        return device_rc
    print(f"[{TAG}] PASS(host 恒跑全绿;device 段真跑全绿 = G7.4 W3c 兑现,门 G-G7-6)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
