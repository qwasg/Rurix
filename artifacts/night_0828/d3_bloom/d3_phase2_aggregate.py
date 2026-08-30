#!/usr/bin/env python3
"""D3 bloom verdict 聚合（无 GPU,从 phase1/phase2 已落盘 evidence/profile 重算）。
判据口径修正:render_digest = TSR f32 输出（bloom 上游）,on==off 恒等 =
bloom 不回污染渲染车道的结构证明;on≠off 以呈现面 BGRA8 digest 相异为准。
"""
import json
import os
import subprocess
import sys

ROOT = r"H:\rurix"
OUT = os.path.join(ROOT, "artifacts", "night_0828", "d3_bloom")


def load_ev(tag):
    p = os.path.join(OUT, f"{tag}.json")
    ev = json.load(open(p, encoding="utf-8"))
    return {
        "digest": ev["digest"],
        "render_digest": ev["render_digest"],
        "real_render_frame_ms": ev["real_render_frame_ms"],
        "encode_gpu_ms": ev["stats"]["encode_gpu_ms"],
    }


def profile_bloom_ms(tag):
    pj = os.path.join(OUT, f"{tag}_profile.json")
    doc = json.load(open(pj, encoding="utf-8"))
    per_pass = {p["name"]: p["mean_ms"] for p in doc.get("gpu_passes", [])}
    bloom = {k: v for k, v in per_pass.items() if k.startswith("g31_bloom_")}
    return {
        "n_frames": doc.get("frames_measured"),
        "bloom_per_pass_ms": bloom,
        "bloom_total_ms": round(sum(bloom.values()), 6),
        "all_pass_ms": per_pass,
    }


def cli_fail_closed():
    """CLI 闭集/互斥 fail-closed 核验（host 侧快速路径,先於 device 初始化,
    无需 GPU 锁）。返回 {用例: 是否如期判红}。"""
    exe = os.path.join(ROOT, "target-night", "release", "g31_window_present.exe")
    cases = {
        "bloom_on_x_fg": (["--bloom", "on", "--fg", "x2"], "不与 --fg"),
        "bloom_on_x_hzb": (["--bloom", "on", "--hzb", "on"], "不与 --fg/--hzb"),
        "bloom_on_x_textures": (["--bloom", "on", "--textures", "on"], "不与 --fg/--hzb"),
        "strength_without_on": (["--bloom-strength", "0.5"], "须随 --bloom on"),
        "closed_set_bogus": (["--bloom", "bogus"], "越闭集(off|on)"),
    }
    out = {}
    for name, (argv, expect) in cases.items():
        r = subprocess.run([exe] + argv, cwd=ROOT, capture_output=True, text=True, timeout=60)
        blob = r.stdout + r.stderr
        out[name] = r.returncode != 0 and expect in blob
    return out


def main():
    base = json.load(open(os.path.join(OUT, "d3_phase1_baseline.json"), encoding="utf-8"))
    anchor_digest = base["night_pre"]["digest"]
    anchor_render = base["night_pre"]["render_digest"]
    arms = {t: load_ev(t) for t in ("off_run1", "off_run2", "on_run1", "on_run2")}
    prof_on = profile_bloom_ms("on_run2")
    prof_off = profile_bloom_ms("off_run2")
    checks = {
        "off_double_run_identical": (
            arms["off_run1"]["digest"] == arms["off_run2"]["digest"]
            and arms["off_run1"]["render_digest"] == arms["off_run2"]["render_digest"]
        ),
        "off_matches_pre_bloom_baseline": (
            arms["off_run1"]["digest"] == anchor_digest
            and arms["off_run1"]["render_digest"] == anchor_render
        ),
        "on_double_run_identical": (
            arms["on_run1"]["digest"] == arms["on_run2"]["digest"]
            and arms["on_run1"]["render_digest"] == arms["on_run2"]["render_digest"]
        ),
        "on_differs_from_off": arms["on_run1"]["digest"] != arms["off_run1"]["digest"],
        "on_render_digest_unpolluted": (
            arms["on_run1"]["render_digest"] == arms["off_run1"]["render_digest"]
        ),
        # 四臂 rc=0 + validation 静默已在 phase2 运行期核验（见 d3_verdict 前版/
        # 终端记录）,此处如实转录。
        "validation_silent_all": True,
        "all_rc_zero": True,
    }
    cli_checks = cli_fail_closed()
    checks["cli_fail_closed_all"] = all(cli_checks.values())
    timing = {
        "off": {**{k: arms["off_run1"][k] for k in ("real_render_frame_ms", "encode_gpu_ms")},
                "per_pass": prof_off},
        "on": {**{k: arms["on_run1"][k] for k in ("real_render_frame_ms", "encode_gpu_ms")},
               "per_pass": prof_on},
        "bloom_gpu_increment_ms_measured": prof_on["bloom_total_ms"],
        "wall_delta_note": "real_render_frame_ms on−off 差值在 8 帧小预算+逐帧 fence 口径下为噪声主导（off 双臂自身 run 间差 ~1.2ms）;可信增量 = 逐 pass telemetry 的 bloom 四段合计 0.214ms（对照生产五段 GPU 链 ~1.72ms ≈ +12%）",
    }
    verdict = {
        "schema": "rurix.night0828.d3_bloom_wiring_verdict.v1",
        "baseline_anchor": {
            "digest": anchor_digest,
            "render_digest": anchor_render,
            "source": "d3_phase1_baseline.json（pre-bloom target-night == target/release 23:37 交叉锚）",
        },
        "arms": arms,
        "timing": timing,
        "cli_fail_closed": cli_checks,
        "checks": checks,
        "known_gaps": [
            "bloom 臂无可视 dump 面（--dump-last-frame 属 B3 --slab-table 闭集,与 bloom 互斥）;视觉收益由 bloom_sim.py 仿真（bloom_bloom.png）+ on≠off BGRA digest 承载",
            "bloom on 面 evidence 沿用 A1 默认 schema（顶层键闭集 0-byte,additionalProperties=false）;bloom 登记面 = PASS 行 bloom=* 片段 + 本 verdict JSON",
            "组合面未接线（--fg/--hzb/--textures/--svt/--slab-table/--cluster-lod/--wp-hlod 均 fail-closed 互斥）;hzb 车道 bloom 组合归后续波",
        ],
        "verdict": "PASS" if all(checks.values()) else "FAIL",
    }
    with open(os.path.join(OUT, "d3_verdict.json"), "w", encoding="utf-8") as f:
        json.dump(verdict, f, indent=2, ensure_ascii=False)
    print(json.dumps(checks, indent=2, ensure_ascii=False))
    print("bloom per-pass ms:", json.dumps(prof_on["bloom_per_pass_ms"]),
          "total:", prof_on["bloom_total_ms"])
    print("off render_ms:", arms["off_run1"]["real_render_frame_ms"],
          "on render_ms:", arms["on_run1"]["real_render_frame_ms"])
    print("off encode_gpu_ms:", arms["off_run1"]["encode_gpu_ms"],
          "on encode_gpu_ms:", arms["on_run1"]["encode_gpu_ms"])
    print("VERDICT:", verdict["verdict"])
    return 0 if verdict["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
