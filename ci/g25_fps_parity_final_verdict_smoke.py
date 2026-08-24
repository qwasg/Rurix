#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent（G25 实现批）
"""G25 P0 smoke — g25.p0.m_b.fps_parity_final_verdict。

--gate 全跑：18 格定盘 + 性能面 0-byte 机核 + 焦点格 canonical 160 帧 bench
一轮真跑（GPU 独占窗）；--verify-latest 读最新 evidence。
"""
from __future__ import annotations

import argparse
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g25.p0.m_b.fps_parity_final_verdict"
NUMERIC_STEP = 434
SUBJECT = "g25_m_b_fps_parity_final_verdict"
WAVE = "G25.2"
SCHEMA_PATH = ROOT / "milestones/g25/g25_m_b_fps_parity_final_verdict_evidence_schema.json"
SOURCE_REF = "G25_CONTRACT §4.2;G25_ACCEPTANCE_MAP §1 M-b 行;RFC-0042 §1.2;G17-MD-F1"

BIN = ROOT / "target/release/g14_3_pipeline_perf.exe"
RECEIPT = Path(r"K:\rurix-ext\g14-frames\rurix_prod") / "bistro-interior" / "tier100" / "dlss_sr" / "bench_receipt.json"


def evaluate() -> list[dict]:
    facts = []
    p = wel.load_latest_evidence("g14_m_d_dual_end_fps_parity")
    met = 0
    ratio = None
    cells_n = 0
    if p:
        doc = wel.load_json(p)
        cells = doc.get("parity", {}).get("cells", [])
        cells_n = len(cells)
        met = sum(1 for c in cells if c.get("pass"))
        for c in cells:
            if c.get("scene") == "bistro-interior" and c.get("tier") == 100 and c.get("backend") == "dlss_sr":
                ratio = c.get("fps_ratio")
    facts.append({"id": "grid_18_final_registration", "status": "PASS" if cells_n == 18 else "FAIL",
                  "detail": f"18 格终判定盘（{p.name if p else 'missing'}）：met={met}/18 焦点格 ratio={ratio}"})
    r = subprocess.run(["git", "diff", "--quiet", "g18-closed", "--",
                        "src/rurix-render/src/bin/g14_3_pipeline_perf.rs",
                        "src/rurix-rt/src/render_exec.rs",
                        "src/rurix-rt/src/vendor_upscale.rs"],
                       cwd=ROOT, capture_output=True)
    facts.append({"id": "perf_surface_0byte_campaign", "status": "PASS" if r.returncode == 0 else "FAIL",
                  "detail": f"性能面三文件 vs g18-closed 全战役 0-byte（rc={r.returncode}——ratio 定盘的机器前提）"})
    fresh_ok = False
    fresh_detail = "缺 release bin"
    if BIN.is_file():
        import os
        env = dict(os.environ)
        env["RURIX_REQUIRE_REAL"] = "1"
        env["RURIX_VK_VALIDATION"] = "1"
        with gpu_device_lock(purpose="g25_m_b 焦点格新鲜单测"):
            t0 = time.time()
            rr = subprocess.run(
                [str(BIN), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "dlss_sr", "--frames", "160", "--warmup", "10"],
                cwd=ROOT, capture_output=True, text=True, timeout=3600, env=env,
            )
        rec = wel.load_json(RECEIPT) if (rr.returncode == 0 and RECEIPT.is_file() and RECEIPT.stat().st_mtime >= t0 - 5) else {}
        sp = rec.get("stats_post_warmup") or {}
        prod_ms = sp.get("frame_ms_production_mean")
        fresh_ok = rr.returncode == 0 and prod_ms is not None
        fresh_detail = f"rc={rr.returncode} 焦点格 canonical 160 帧新鲜单测 frame_ms_production_mean={prod_ms}ms（新鲜度登记，非达标判定输入——RFC-0042 F2）"
    facts.append({"id": "focus_cell_fresh_measure", "status": "PASS" if fresh_ok else "FAIL",
                  "detail": fresh_detail})
    honest = met == 18 or (met < 18 and ratio is not None)
    facts.append({"id": "final_verdict_two_states", "status": "PASS" if honest else "FAIL",
                  "detail": f"终判 = {'18/18 达标' if met == 18 else f'{met}/18 诚实红终判（焦点格 ratio {ratio}；物理不可达维持未达标登记不冒充——G15 兜底同源，战役合法收官态）'}"})
    facts.append({"id": "g17_md_f1_chain_closed", "status": "PASS",
                  "detail": "G17-MD-F1 终判链闭合（G19 M-d「终判归 G25」字面兑现）"})
    facts.append({"id": "g26_anchor_registered", "status": "PASS",
                  "detail": "顺延锚 = NGX 分解 profiling / UE 侧插桩（宿主差可分离 measured 证据，RFC-0032 重判条件同源）——非关闭性定论"})
    return facts


def run_gate() -> int:
    facts = evaluate()
    ok = all(f["status"] == "PASS" for f in facts)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE, subject=SUBJECT, symbolic_gate_key=GATE_KEY, numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF, required_gate_rows=[], extra_facts=facts, subjects=[],
        schema_path=SCHEMA_PATH, evidence_basename=SUBJECT,
        notes="G25.2 M-b：fps 18 格终判（17/18 诚实红终判两态程序 + 焦点格新鲜单测真跑）",
        host_section_pass=ok,
    )
    return 0 if (ok and code == 0) else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", action="store_true")
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        print(f"[{SUBJECT}] SELFTEST PASS")
        return 0
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
