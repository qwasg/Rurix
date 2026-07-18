#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# SPIKE(RD-027):campaign.jsonl → 最终取证 evidence JSON 蒸馏器。
"""把 build/spike-rd027/campaign.jsonl(逐 run 增量日志)蒸馏为
evidence/rd027_pt_poison_spike_<date>.json(过 milestones/g3/rd027_spike_evidence_schema.json)。

机械事实自动蒸馏;归因判断(attribution/minimized_repro/workaround)从
spike/rd027-pt-poison/attribution.json 读入(判断与机械分离,归因文件由
spike 结论时人工/主循环落笔)。

用法:py -3 spike/rd027-pt-poison/make_evidence.py [--date YYYYMMDD]
"""
from __future__ import annotations

import datetime
import json
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(ROOT))
CAMPAIGN = ROOT / "build" / "spike-rd027" / "campaign.jsonl"
ATTRIBUTION = HERE / "attribution.json"


def fail(msg: str) -> None:
    print(f"[make_evidence] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def ptxas_version() -> str:
    import os
    cuda = os.environ.get("CUDA_PATH")
    if not cuda:
        return "unavailable"
    try:
        r = subprocess.run([str(Path(cuda) / "bin" / "ptxas.exe"), "--version"],
                           capture_output=True, timeout=30)
        for ln in r.stdout.decode("utf-8", errors="replace").splitlines():
            if "release" in ln or ln.strip().startswith("Cuda"):
                return ln.strip()
        return r.stdout.decode("utf-8", errors="replace").strip()[:120]
    except Exception:
        return "unavailable"


def main() -> int:
    date = datetime.date.today().strftime("%Y%m%d")
    args = sys.argv[1:]
    if "--date" in args:
        date = args[args.index("--date") + 1]
    if not CAMPAIGN.is_file():
        fail(f"campaign 日志缺失: {CAMPAIGN}")
    if not ATTRIBUTION.is_file():
        fail(f"归因文件缺失: {ATTRIBUTION}(spike 结论时落笔)")
    recs = [json.loads(l) for l in CAMPAIGN.read_bytes().decode("utf-8").splitlines()]
    attribution = json.loads(ATTRIBUTION.read_bytes().decode("utf-8"))

    env = next((r["environment"] for r in recs if r.get("kind") == "header"), {})
    e0a = next((r for r in recs if r.get("kind") == "e0a_summary"), None)
    if e0a is None:
        fail("缺 e0a_summary")
    sac = next((r for r in recs if r.get("kind") == "single_artifact_check"), {})
    aot_ptx = next((r["ptx_sha256"] for r in recs
                    if r.get("kind") == "build" and r.get("name") == "ctrl_b2"), None)
    jit_ptx = next((r["ptx_sha256"] for r in recs
                    if r.get("kind") == "build" and r.get("name") == "jit_ctrl_b2"), None)

    exp_of = {"ctrl": "E0a", "poison": "E0a", "jit_": "E0b", "aotO": "E1",
              "e7b_": "E7b", "canary": "E0a"}
    experiments = []
    for r in recs:
        if r.get("kind") == "run" and r.get("name") != "canary":
            name = r["name"]
            exp = ("E0b" if name.startswith("jit_") else
                   "E1" if name.startswith("aotO") else
                   "E7b" if name.startswith("e7b_") else "E0a")
            loader = ("driver_jit(ptx78)" if name.startswith("jit_") else
                      f"ptxas_aot_O{name[4]}" if name.startswith("aotO") else
                      "ptxas_aot_default(O3)")
            experiments.append({
                "experiment": exp, "name": name, "loader_path": loader,
                "classification": r["classification"], "exit_code": r["exit_code"],
                "wall_s": r["wall_s"], "timeout_s": r["timeout_s"],
            })
        elif r.get("kind") == "memcheck":
            experiments.append({
                "experiment": "E7a", "name": f"memcheck_{r['name']}",
                "loader_path": "ptxas_aot_default(O3)+compute-sanitizer",
                "classification": ("hang_timeout" if r["timed_out"]
                                   else ("completed" if r["exit_code"] == 0 else "error")),
                "exit_code": r["exit_code"], "wall_s": -1.0, "timeout_s": 300,
                "detail": r.get("error_summary", ""),
            })
        elif r.get("kind") == "ppm_compare":
            experiments.append({
                "experiment": "E7b", "name": f"ppm_compare_{r['name']}",
                "classification": "completed",
                "detail": f"match_o0_reference={r['match_o0_reference']}",
            })

    sig = None
    for r in recs:
        if (r.get("kind") == "run" and r.get("name", "").startswith("poison")
                and r.get("gpu_during")):
            s = r["gpu_during"][len(r["gpu_during"]) // 2]
            sig = {"util_pct": s["util_pct"], "power_w": s["power_w"],
                   "sm_clock_mhz": s["sm_clock_mhz"]}
            break

    canaries = [r for r in recs if r.get("kind") == "canary_verdict"]
    quarantined = sum(len(r.get("quarantined", [])) for r in recs if r.get("kind") == "run")

    doc = {
        "schema_version": 1,
        "subject": "rd027_pt_poison_spike",
        "status": "measured_local",
        "rd_ref": "RD-027",
        "gate_ref": "G3_CONTRACT G-G3-1",
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "environment": {
            "gpu_name": env.get("gpu_name", "unavailable"),
            "driver_version": env.get("driver_version", "unavailable"),
            "cuda_driver_version": env.get("cuda_driver_version", "unavailable"),
            "ptxas_version": ptxas_version(),
            "os_build": env.get("os_build", "unavailable"),
            "compute_capability": env.get("compute_capability", "unavailable"),
            "tdr": env.get("tdr"),
            "hags_enabled": env.get("hags_enabled"),
        },
        "baseline": {
            "reproduced": bool(e0a.get("reproduced")),
            "results": e0a.get("results", {}),
        },
        "single_artifact": {
            "distinct_ptx_digests": sac.get("distinct_ptx_digests", -1),
            "confirmed": bool(sac.get("confirmed_single_artifact")),
            "ptx_sha256": aot_ptx,
            "jit_leg_ptx_sha256": jit_ptx,
        },
        "experiments": experiments,
        "gpu_hang_signature": sig,
        "attribution": attribution["attribution"],
        "minimized_repro": attribution.get("minimized_repro",
                                           {"status": "not_attempted"}),
        "workaround": attribution.get("workaround"),
        "discipline": {
            "proc_guard_all_runs": True,
            "canary_gates_passed": sum(1 for c in canaries if c.get("ok")),
            "canary_gates_failed": sum(1 for c in canaries if not c.get("ok")),
            "quarantined_exes": quarantined,
            "notes": "全 GPU 运行经 bench/proc_guard guarded_run(120s 判定线/杀树/隔离);"
                     "毒径判定后金丝雀门;实验窗与 CI runner 错峰;首轮 E7a 因隔离区收走"
                     " exe 判定作废并重测(campaign correction 记录在案)",
        },
    }
    out = ROOT / "evidence" / f"rd027_pt_poison_spike_{date}.json"
    with open(out, "wb") as f:
        f.write((json.dumps(doc, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
    print(f"[make_evidence] → {out.relative_to(ROOT)} ({len(experiments)} experiments)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
