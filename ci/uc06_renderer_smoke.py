#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-06 全管线渲染器冒烟(步骤 87;G5.4;RFC-0016 §1 管线图;验收门 G-G5-8)。

host 段(**恒跑**,无 GPU):
  1. `cargo run -p uc06-renderer -- --frames 4 --size 128x72 --json`——host 全管线
     (meshlet→剔除→VisBuffer→材质→VSM+GI+RTAO→TAA/TSR),解析单行 JSON:
     exit 0 + asserts 全 true + pso 告警 0 + alias_peak < no_alias_peak + fence_count ≥ 1。

device 段(**gate real**:Vulkan 在位;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP=dev-env-degrade):
  2. `RURIX_REQUIRE_REAL=1 cargo run -p uc06-renderer --features vulkan -- --device --json`——
     render_exec 真多 pass 后依次跑五个 W1/W2 内核 host 对拍，device 字段记录能力快照、
     分波汇总、逐内核 pass 与关键统计；对拍失败始终硬红。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"


def fail(msg: str) -> int:
    print(f"[uc06_renderer_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "(RURIX_REQUIRE_REAL=1 不许 SKIP)")
    print(f"[uc06_renderer_smoke] SKIP {msg}(dev-env-degrade,退出 0)")
    return 0


def run(cmd, cwd: Path = ROOT, timeout: int = 1800, env_extra: dict | None = None):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout, env=env)
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


def parse_demo_json(out: str) -> dict | None:
    for line in out.splitlines():
        line = line.strip()
        if line.startswith("{") and line.endswith("}") and '"subject":"uc06_renderer"' in line:
            try:
                return json.loads(line)
            except json.JSONDecodeError:
                return None
    return None


def host_section(results: dict) -> bool:
    code, out, err = run(["cargo", "run", "-q", "-p", "uc06-renderer", "--bin", "uc06-renderer", "--",
                          "--frames", "4", "--size", "128x72", "--json"], timeout=1800)
    doc = parse_demo_json(out)
    if code != 0 or doc is None:
        print((out + err)[-2400:], file=sys.stderr)
        results["host_pipeline_pass"] = False
        return fail(f"host 段: uc06-renderer host 管线未过(rc={code}, JSON 解析失败)")
    results["host_pipeline_pass"] = True
    results["host_exit_ok"] = doc.get("exit_ok") is True
    if not results["host_exit_ok"]:
        return fail("host 段: demo exit_ok != true(断言未全过)")
    asserts = doc.get("asserts", {})
    all_true = all(v is True for v in asserts.values()) and len(asserts) >= 8
    results["host_asserts_all_true"] = all_true
    if not all_true:
        return fail(f"host 段: asserts 未全 true({asserts})")
    results["pso_warnings"] = doc.get("pso_runtime_compile_warnings")
    if doc.get("pso_runtime_compile_warnings") != 0:
        return fail(f"host 段: PSO 运行时编译告警 {doc.get('pso_runtime_compile_warnings')} != 0(G-G5-7)")
    g = doc.get("graph", {})
    alias_ok = g.get("alias_peak", 0) < g.get("no_alias_peak", 0)
    fence_ok = g.get("fence_count", 0) >= 1
    results["graph_alias_saves"] = alias_ok
    results["graph_fences_nonempty"] = fence_ok
    if not alias_ok or not fence_ok:
        return fail(f"host 段: graph 结构不符(alias={g.get('alias_peak')}≥{g.get('no_alias_peak')} 或 fence={g.get('fence_count')}<1)")
    stages = doc.get("stages", [])
    results["host_stage_count"] = len(stages)
    print(f"[uc06_renderer_smoke] host 步骤 1 PASS: uc06-renderer host 管线 exit_ok, {len(stages)} 阶段")
    return True


def device_section(results: dict) -> int:
    code, out, err = run(
        ["cargo", "run", "-q", "-p", "uc06-renderer", "--bin", "uc06-renderer", "--features", "vulkan",
         "--", "--device", "--frames", "2", "--size", "64x64", "--json"],
        env_extra={"RURIX_REQUIRE_REAL": "1"}, timeout=1800,
    )
    doc = parse_demo_json(out)
    if code != 0 or doc is None:
        blob = out + err
        if "no-vulkan" in blob.lower() or "vulkan loader" in blob.lower() or "SKIP" in blob:
            results["device_pass"] = "SKIP"
            results["toolchain_skip"] = "no-vulkan"
            return skip("device 段:无 Vulkan loader(device 真跑归 gate real;host 段已恒跑)")
        print((out + err)[-2400:], file=sys.stderr)
        results["device_pass"] = False
        return fail(f"device 段: uc06-renderer --device 未过(rc={code})")
    dev = doc.get("device")
    results["device_pass"] = doc.get("exit_ok") is True and dev is not None
    if not results["device_pass"]:
        return fail(f"device 段: exit_ok != true 或 device 字段空({doc.get('device')})")
    results["device_summary"] = dev
    results["device_name"] = dev.get("device_name")
    results["device_atomic_int64"] = dev.get("atomic_int64")
    results["device_triangle_pixels"] = dev.get("triangle_pixels")
    results["device_compute_write_ok"] = dev.get("compute_write_ok")
    results["device_mixed_pass_ok"] = dev.get("mixed_pass_ok")
    for field in (
        "wave_w1_pass", "wave_w2_pass", "cull_pass", "visbuffer_pass",
        "classify_resolve_pass", "vsm_page_mark_pass", "taa_pass",
    ):
        if field in dev and dev[field] is not True:
            return fail(f"device 段: {field} 出现但不为 true(对拍回归，禁止降级)")
        if field in dev:
            results[f"device_{field}"] = dev[field]
    for field in (
        "shader_int64", "ray_query", "acceleration_structure",
        "buffer_device_address", "descriptor_indexing", "deferred_host_operations",
        "cull_visible_clusters", "visbuffer_matched_words", "classify_matched_pixels",
        "vsm_marked_pages", "taa_max_err",
    ):
        if field in dev:
            results[f"device_{field}"] = dev[field]
    print(
        f"[uc06_renderer_smoke] device 步骤 2 PASS: {dev.get('device_name')} "
        f"真多 pass + W1={dev.get('wave_w1_pass')} W2={dev.get('wave_w2_pass')}; "
        f"vis_words={dev.get('visbuffer_matched_words')} taa_max_err={dev.get('taa_max_err')}"
    )
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("device_pass") == "SKIP" or results.get("toolchain_skip") is not None
    doc = {
        "schema_version": 1,
        "subject": "uc06_renderer_smoke",
        "milestone": "G5.4 / G-G5-8 (RFC-0016 §1 管线图)",
        "step": 87,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": {k: results.get(k) for k in (
            "host_pipeline_pass", "host_exit_ok", "host_asserts_all_true", "pso_warnings",
            "graph_alias_saves", "graph_fences_nonempty", "host_stage_count",
            "device_pass", "device_name", "device_atomic_int64", "device_triangle_pixels",
            "device_compute_write_ok", "device_mixed_pass_ok",
            "device_wave_w1_pass", "device_wave_w2_pass", "device_cull_pass",
            "device_visbuffer_pass", "device_classify_resolve_pass",
            "device_vsm_page_mark_pass", "device_taa_pass", "device_shader_int64",
            "device_ray_query", "device_acceleration_structure", "device_buffer_device_address",
            "device_descriptor_indexing", "device_deferred_host_operations",
            "device_cull_visible_clusters", "device_visbuffer_matched_words",
            "device_classify_matched_pixels", "device_vsm_marked_pages", "device_taa_max_err",
        ) if results.get(k) is not None},
        "device": results.get("device_summary"),
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"uc06_renderer_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n")
    print(f"[uc06_renderer_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    results: dict = {}
    host_ok = host_section(results)
    device_rc = device_section(results) if host_ok else 1
    write_evidence(results, host_ok, device_rc)
    if not host_ok:
        return 1
    if device_rc != 0:
        return device_rc
    print("[uc06_renderer_smoke] PASS(host 恒跑 + device gate real)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
