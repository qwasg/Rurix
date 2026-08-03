#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""compute RayQuery codegen 冒烟(步骤 93;G7.2 W3a;RFC-0018 章 A/B;验收门 G-G7-4)。

host / compile 段(**恒跑**,需 Vulkan SDK 的 `spirv-val`/`spirv-dis`;缺工具 → SKIP):
  1. RED/accept 语料批跑:`conformance/rayquery/` accept 零诊断 + reject 全拦截
     (RXS-0297~0299;`src/rurixc/tests/rayquery_corpus.rs`)。
  2. codegen 锚定单测:1.4 分叉 / capability+extension 按需 / 反汇编 golden 最小集 /
     initialize 操作数序与冻结 flags / interface 全量 / 1.0 零漂移锚点 /
     `spirv-val` 双口径(`vulkan_codegen::tests::ray_query*`,RXS-0300)。
  3. 真实 `.rx` → `.spv`:两件 accept 语料经 `rurixc --target vulkan` 产物,
     `spirv-val --target-env vulkan1.2` **与** `spv1.4` 双口径皆 accept(退出码判定,
     不 grep stdout;承 RXS-0212/0247)。
  4. 反汇编 golden 锚定(G-G7-4 逐字):`spirv-dis` 输出须含 1.4 header +
     `RayQueryKHR` + `SPV_KHR_ray_query` + `OpTypeRayQueryKHR` /
     `OpRayQueryInitializeKHR` / `OpRayQueryProceedKHR` / `OpRayQueryTerminateKHR` +
     committed 查询族。
  5. **W1/W2 零漂移门**:五 kernel(cull/visbuffer_sw_u64/classify_resolve/
     vsm_page_mark/taa)逐件对 `tests/vulkan/w1w2_spv_manifest.json` 的
     sha256 + SPIR-V 版本 + capability 集合**逐字节/逐项**比对,且不得出现
     `RayQueryKHR`/`SPV_KHR_ray_query`(能力声明零回归)。
  6. RED 反证:篡改 `.spv` 单字节 → `spirv-val` 必拒(退出码非 0),证校验轴真在生效。

device 段(**gate real**):最小 hit/miss/属性查询 kernel 真跑。
  当前 **blocked-honest**:compute AS descriptor / TLAS 导入通道属 G7.3(W3b)交付面,
  尚未在树 → 记 `device_blocked="G7.2-W3b-pending"` 并 SKIP;`RURIX_REQUIRE_REAL=1`
  翻硬红(不以 host 段绿冒充 device 绿,RFC-0016 §4.E3 / §9.1 R-3 纪律)。
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
ACCEPT_DIR = ROOT / "conformance" / "rayquery" / "accept"
W1W2_KERNELS = ROOT / "apps" / "uc06-renderer" / "kernels"

TAG = "ray_query_codegen_smoke"

# 反汇编 golden 最小集(G-G7-4 逐字 + RXS-0300「反汇编 golden 锚定(最小集)」)。
#
# 分两层,因 RXS-0298 明定 `terminate` 为**可选早退**(「SPIR-V 语义不要求终结;
# 未 terminate 的 RayQuery 随 function 作用域结束自然消亡」),committed 查询族亦
# 「**按真实使用**」声明 —— 故最小集是**golden 套件整体**的属性,不是每件语料的属性:
#   · PER_FILE:任何 compute RayQuery 模块必然出现的指令(构造 + 遍历推进);
#   · CORPUS:最小集其余项,由语料集**并集**覆盖(全流程语料 ray_query_basic 承载)。
GOLDEN_PER_FILE = (
    "OpTypeAccelerationStructureKHR",
    "OpTypeRayQueryKHR",
    "OpRayQueryInitializeKHR",
    "OpRayQueryProceedKHR",
)
GOLDEN_CORPUS_UNION = (
    "OpRayQueryTerminateKHR",
    "OpRayQueryGetIntersectionTypeKHR",
)
# committed 查询族(按真实使用;全流程语料应全覆盖)。
GOLDEN_COMMITTED = (
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


def run(cmd, cwd: Path = ROOT, timeout: int = 1800):
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout)
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
    """`.spv` header 第 2 字 = 版本(小端);→ "major.minor"。"""
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


# ───────────────────────── host / compile 段 ─────────────────────────


def corpus_section(results: dict) -> bool:
    """步骤 1:RED/accept 语料批跑(RXS-0297~0299)。"""
    code, o, e = run(
        ["cargo", "test", "-q", "-p", "rurixc", "--features", "vulkan-backend",
         "--test", "rayquery_corpus"]
    )
    blob = o + e
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        results["corpus_pass"] = False
        return False
    results["corpus_pass"] = True
    print(f"[{TAG}] 步骤 1 PASS: rayquery 语料 accept 零诊断 + reject 全拦截")
    return True


def codegen_unit_section(results: dict) -> bool:
    """步骤 2:codegen 锚定单测(RXS-0300)。"""
    code, o, e = run(
        ["cargo", "test", "-q", "-p", "rurixc", "--features", "vulkan-backend",
         "--lib", "vulkan_codegen::tests"]
    )
    blob = o + e
    if code != 0:
        print(blob[-2400:], file=sys.stderr)
        results["codegen_unit_pass"] = False
        return False
    results["codegen_unit_pass"] = True
    print(f"[{TAG}] 步骤 2 PASS: vulkan_codegen 锚定单测(1.4 分叉/golden/双口径)")
    return True


def emit_and_validate_section(results: dict, work: Path) -> bool:
    """步骤 3+4:真实 .rx → .spv,双口径 spirv-val + 反汇编 golden 锚定。"""
    corpora = sorted(ACCEPT_DIR.glob("*.rx"))
    if not corpora:
        results["emit_pass"] = False
        print(f"[{TAG}] accept 语料集为空", file=sys.stderr)
        return False
    per_file = {}
    for src in corpora:
        spv = work / (src.stem + ".spv")
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2400:], file=sys.stderr)
            results["emit_pass"] = False
            print(f"[{TAG}] {src.name} 编译未产 .spv", file=sys.stderr)
            return False
        ver = spv_version(spv)
        if ver != "1.4":
            results["emit_pass"] = False
            print(f"[{TAG}] {src.name} 版本 {ver} != 1.4(RXS-0300 升版判定)", file=sys.stderr)
            return False
        # 双口径 spirv-val(退出码判定)。
        envs = {}
        for env in ("vulkan1.2", "spv1.4"):
            vc, vblob = spirv_val(spv, env)
            if vc == -1:
                results["toolchain_skip"] = "no-spirv-val"
                return True  # 交由 main 走 SKIP 路径
            if vc != 0:
                results["emit_pass"] = False
                print(f"[{TAG}] {src.name} spirv-val --target-env {env} 拒: {vblob[-800:]}",
                      file=sys.stderr)
                return False
            envs[env] = "accepted"
        # 反汇编 golden 锚定。
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        if dc != 0:
            results["emit_pass"] = False
            print(f"[{TAG}] {src.name} spirv-dis 失败: {dis[-800:]}", file=sys.stderr)
            return False
        missing = [m for m in GOLDEN_PER_FILE if m not in dis]
        if missing:
            results["emit_pass"] = False
            print(f"[{TAG}] {src.name} 反汇编 golden(per-file 必含)缺: {missing}",
                  file=sys.stderr)
            return False
        if "RayQueryKHR" not in dis or "SPV_KHR_ray_query" not in dis:
            results["emit_pass"] = False
            print(f"[{TAG}] {src.name} 缺 RayQueryKHR capability / SPV_KHR_ray_query",
                  file=sys.stderr)
            return False
        per_file[src.name] = {
            "spirv_version": ver,
            "spirv_val": envs,
            "sha256": sha256_of(spv),
            "committed_queries": sorted(m for m in GOLDEN_COMMITTED if m in dis),
            "golden_mnemonics": sorted(
                m
                for m in GOLDEN_PER_FILE + GOLDEN_CORPUS_UNION + GOLDEN_COMMITTED
                if m in dis
            ),
        }
    # golden 最小集的**并集**判据:语料集整体须覆盖 terminate + intersection-type +
    # committed 五查询族(= lowering 覆盖 RFC-0018 冻结子集的机器判据)。
    union = set()
    for info in per_file.values():
        union |= set(info["golden_mnemonics"])
    required = set(GOLDEN_PER_FILE) | set(GOLDEN_CORPUS_UNION) | set(GOLDEN_COMMITTED)
    if not required <= union:
        results["emit_pass"] = False
        print(f"[{TAG}] golden 最小集并集覆盖不全: 缺 {sorted(required - union)}",
              file=sys.stderr)
        return False
    results["golden_union"] = sorted(union)
    results["emit_pass"] = True
    results["modules"] = per_file
    print(f"[{TAG}] 步骤 3+4 PASS: {len(per_file)} 件语料 1.4 + spirv-val 双口径 + golden 锚定")
    return True


def w1w2_zero_drift_section(results: dict, work: Path) -> bool:
    """步骤 5:W1/W2 五 kernel 逐字节 + capability 声明零漂移(G-G7-4 逐字)。"""
    if not MANIFEST.is_file():
        results["w1w2_zero_drift_pass"] = False
        print(f"[{TAG}] 缺 golden manifest {MANIFEST.relative_to(ROOT)}", file=sys.stderr)
        return False
    manifest = json.loads(MANIFEST.read_text(encoding="utf-8"))
    expected = manifest["kernels"]
    observed = {}
    for name, want in sorted(expected.items()):
        src = W1W2_KERNELS / f"{name}.rx"
        if not src.is_file():
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] 缺 W1/W2 kernel 源 {src.relative_to(ROOT)}", file=sys.stderr)
            return False
        spv = work / f"w1w2_{name}.spv"
        code, blob = compile_rx(src, spv)
        if code != 0 or not spv.is_file():
            print(blob[-2000:], file=sys.stderr)
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 编译失败", file=sys.stderr)
            return False
        ver = spv_version(spv)
        digest = sha256_of(spv)
        dc, dis = disasm(spv)
        if dc == -1:
            results["toolchain_skip"] = "no-spirv-dis"
            return True
        caps = sorted(
            line.split("OpCapability", 1)[1].strip()
            for line in dis.splitlines()
            if "OpCapability" in line
        )
        observed[name] = {"spirv_version": ver, "sha256": digest, "capabilities": caps}
        if ver != want["spirv_version"]:
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 版本漂移: {ver} != {want['spirv_version']}",
                  file=sys.stderr)
            return False
        if digest != want["sha256"]:
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} **字节漂移**: {digest} != {want['sha256']}"
                  f"(RXS-0300 零漂移门:既有 golden 不重 bless)", file=sys.stderr)
            return False
        if caps != want["capabilities"]:
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} capability 声明漂移: {caps} != {want['capabilities']}",
                  file=sys.stderr)
            return False
        if "RayQueryKHR" in caps or "SPV_KHR_ray_query" in dis:
            results["w1w2_zero_drift_pass"] = False
            print(f"[{TAG}] W1/W2 {name} 误声明 ray query 面(capability 只按真实使用声明)",
                  file=sys.stderr)
            return False
    results["w1w2_zero_drift_pass"] = True
    results["w1w2_observed"] = observed
    print(f"[{TAG}] 步骤 5 PASS: W1/W2 {len(observed)} kernel 字节 + 版本 + capability 零漂移")
    return True


def red_tamper_section(results: dict, work: Path) -> bool:
    """步骤 6:RED 反证——篡改 .spv 单字节 → spirv-val 必拒(校验轴真在生效)。"""
    spv = work / "ray_query_basic.spv"
    if not spv.is_file():
        results["red_tamper_pass"] = False
        print(f"[{TAG}] 缺 {spv.name},无法做 RED 反证", file=sys.stderr)
        return False
    raw = bytearray(spv.read_bytes())
    # 篡改 header 的 magic 尾字节(必然使 spirv-val 拒;不依赖具体指令布局)。
    raw[0] ^= 0xFF
    bad = work / "ray_query_basic_tampered.spv"
    bad.write_bytes(bytes(raw))
    code, blob = spirv_val(bad, None)
    if code == -1:
        results["toolchain_skip"] = "no-spirv-val"
        return True
    if code == 0:
        results["red_tamper_pass"] = False
        print(f"[{TAG}] 篡改后的 .spv 仍被 spirv-val 接受(校验轴失效)", file=sys.stderr)
        return False
    results["red_tamper_pass"] = True
    print(f"[{TAG}] 步骤 6 PASS: 篡改 .spv → spirv-val 拒(退出码 {code}),校验轴生效")
    return True


# ───────────────────────── device 段(gate real) ─────────────────────────


def device_section(results: dict) -> int:
    """最小 hit/miss/属性查询 kernel device 真跑。

    **blocked-honest**:compute AS descriptor / 真实 TLAS 导入通道是 G7.3(W3b)的
    交付面(G-G7-5),尚未在树 —— 没有它,ray query kernel 无法在设备上取得可遍历的
    加速结构。此处如实记 blocked 并 SKIP,**不**以 host/compile 段绿冒充 device 绿
    (RFC-0016 §4.E3 条件臂 / §9.1 R-3;G7 契约 guardrail「mock/host substitution/
    isolated nonzero 不得满足 device 门」)。
    """
    results["device_blocked"] = "G7.2-W3b-pending"
    results["device_probe_note"] = (
        "compute AS descriptor / TLAS 导入通道属 G7.3(W3b,门 G-G7-5)交付面,尚未在树;"
        "ray query kernel 的 device 真跑以其为硬前置。host/compile 段(1~6)恒跑且全绿,"
        "但不构成 device 见证。"
    )
    return skip("[device] 最小 hit/miss kernel 真跑硬前置 G7.3 W3b(compute AS descriptor)未在树")


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "ray_query_codegen_smoke",
        "milestone": "G7.2 W3a / G-G7-4 (RFC-0018 章 A/B)",
        "step": 93,
        "spec_clauses": ["RXS-0297", "RXS-0298", "RXS-0299", "RXS-0300"],
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "device_blocked": results.get("device_blocked"),
        "device_probe_note": results.get("device_probe_note"),
        "checks": {
            k: results.get(k)
            for k in (
                "corpus_pass",
                "codegen_unit_pass",
                "emit_pass",
                "w1w2_zero_drift_pass",
                "red_tamper_pass",
            )
            if results.get(k) is not None
        },
        "modules": results.get("modules", {}),
        "golden_union": results.get("golden_union", []),
        "w1w2_observed": results.get("w1w2_observed", {}),
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": results.get("toolchain_skip") is not None,
        "require_real": os.environ.get("RURIX_REQUIRE_REAL") == "1",
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"ray_query_codegen_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[{TAG}] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    work = ROOT / "target" / "g7_rq_smoke"
    work.mkdir(parents=True, exist_ok=True)

    host_ok = corpus_section(results)
    if host_ok:
        host_ok = codegen_unit_section(results)
    if host_ok:
        host_ok = emit_and_validate_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = w1w2_zero_drift_section(results, work)
    if host_ok and results.get("toolchain_skip") is None:
        host_ok = red_tamper_section(results, work)

    if results.get("toolchain_skip") is not None:
        # 缺 Vulkan SDK 工具:dev-env degrade(RXS-0212 三态),不 fake pass。
        device_rc = 0
        write_evidence(results, host_ok, device_rc)
        return skip(f"[host] {results['toolchain_skip']}(spirv-val/spirv-dis 缺;编译段判据未取证)")

    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return fail("host/compile 段未过(G-G7-4 编译门)")
    if device_rc != 0:
        return device_rc
    print(f"[{TAG}] PASS(host/compile 恒跑全绿;device 段 blocked-honest = G7.3 W3b 前置)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
