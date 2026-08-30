#!/usr/bin/env python3
"""D3 bloom 接线自验（phase 2）：持 GPU 锁,用 target-night 新二进制跑
off×2 / on×2（RURIX_VK_VALIDATION=1）,核验:
  ① off 双跑 digest 位级一致;② off == 接线前基线(phase1 锚)零漂移;
  ③ on 双跑位级一致;④ on ≠ off（接线真实生效）;⑤ 全臂 validation 静默;
  ⑥ 帧时对照 real_render_frame_ms / encode_gpu_ms + bloom 四 pass GPU 增量
    （--profile-json 逐 pass 分解,off/on 各一臂）。
汇总 verdict JSON 落本目录。
"""
import json
import os
import subprocess
import sys

sys.path.insert(0, "ci")
from gpu_device_lock import gpu_device_lock

ROOT = r"H:\rurix"
OUT = os.path.join(ROOT, "artifacts", "night_0828", "d3_bloom")
EXE = os.path.join(ROOT, "target-night", "release", "g31_window_present.exe")
FRAMES = ["--frames", "8", "--warmup", "2", "--hidden"]


def run(tag, extra):
    env = dict(os.environ)
    env["RURIX_VK_VALIDATION"] = "1"
    evidence = os.path.join(OUT, f"{tag}.json")
    argv = [EXE, *FRAMES, "--evidence", evidence] + extra
    r = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True, text=True, timeout=1200)
    out = r.stdout + r.stderr
    rec = {"argv": argv[1:], "rc": r.returncode}
    if r.returncode == 0 and os.path.isfile(evidence):
        ev = json.load(open(evidence, encoding="utf-8"))
        rec["digest"] = ev["digest"]
        rec["render_digest"] = ev["render_digest"]
        rec["real_render_frame_ms"] = ev["real_render_frame_ms"]
        rec["encode_gpu_ms"] = ev["stats"]["encode_gpu_ms"]
    rec["validation_silent"] = ("VUID-" not in out) and ("Validation Error" not in out)
    rec["tail"] = out.strip()[-500:]
    return rec


def profile_bloom_ms(tag):
    pj = os.path.join(OUT, f"{tag}_profile.json")
    if not os.path.isfile(pj):
        return None
    doc = json.load(open(pj, encoding="utf-8"))
    passes = doc.get("gpu_passes", [])
    per_pass = {p["name"]: p["mean_ms"] for p in passes}
    bloom = {k: v for k, v in per_pass.items() if k.startswith("g31_bloom_")}
    return {
        "n_frames": doc.get("frames_measured"),
        "bloom_per_pass_ms": bloom,
        "bloom_total_ms": round(sum(bloom.values()), 6),
        "all_pass_ms": per_pass,
    }


def main():
    base = json.load(open(os.path.join(OUT, "d3_phase1_baseline.json"), encoding="utf-8"))
    anchor_digest = base["night_pre"]["digest"]
    anchor_render = base["night_pre"]["render_digest"]
    res = {}
    with gpu_device_lock(purpose="d3 bloom phase2 verify runs"):
        res["off1"] = run("off_run1", ["--bloom", "off"])
        res["off2"] = run("off_run2", ["--bloom", "off", "--profile-json",
                                       os.path.join(OUT, "off_run2_profile.json")])
        res["on1"] = run("on_run1", ["--bloom", "on"])
        res["on2"] = run("on_run2", ["--bloom", "on", "--profile-json",
                                     os.path.join(OUT, "on_run2_profile.json")])
    checks = {}
    checks["off_double_run_identical"] = (
        res["off1"].get("digest") == res["off2"].get("digest")
        and res["off1"].get("render_digest") == res["off2"].get("render_digest")
        and res["off1"]["rc"] == 0 and res["off2"]["rc"] == 0
    )
    checks["off_matches_pre_bloom_baseline"] = (
        res["off1"].get("digest") == anchor_digest
        and res["off1"].get("render_digest") == anchor_render
    )
    checks["on_double_run_identical"] = (
        res["on1"].get("digest") == res["on2"].get("digest")
        and res["on1"].get("render_digest") == res["on2"].get("render_digest")
        and res["on1"]["rc"] == 0 and res["on2"]["rc"] == 0
    )
    # on≠off 判据 = 呈现面 BGRA8 digest 相异（bloom 改变最终显示帧 = 接线生效）。
    # render_digest = TSR f32 输出（bloom 上游）,on==off 恒等 = bloom 不回污染
    # 渲染车道的结构证明（合成写独立缓冲,TSR out_color 只读）。
    checks["on_differs_from_off"] = res["on1"].get("digest") != res["off1"].get("digest")
    checks["on_render_digest_unpolluted"] = (
        res["on1"].get("render_digest") == res["off1"].get("render_digest")
    )
    checks["validation_silent_all"] = all(v["validation_silent"] for v in res.values())
    checks["all_rc_zero"] = all(v["rc"] == 0 for v in res.values())
    prof_on = profile_bloom_ms("on_run2")
    prof_off = profile_bloom_ms("off_run2")
    verdict = {
        "schema": "rurix.night0828.d3_bloom_wiring_verdict.v1",
        "baseline_anchor": {"digest": anchor_digest, "render_digest": anchor_render,
                            "source": "d3_phase1_baseline.json (pre-bloom target-night == target/release 23:37)"},
        "arms": res,
        "timing": {
            "off": {"real_render_frame_ms": res["off1"].get("real_render_frame_ms"),
                    "encode_gpu_ms": res["off1"].get("encode_gpu_ms"),
                    "per_pass": prof_off},
            "on": {"real_render_frame_ms": res["on1"].get("real_render_frame_ms"),
                   "encode_gpu_ms": res["on1"].get("encode_gpu_ms"),
                   "per_pass": prof_on},
        },
        "checks": checks,
        "verdict": "PASS" if all(checks.values()) else "FAIL",
    }
    with open(os.path.join(OUT, "d3_verdict.json"), "w", encoding="utf-8") as f:
        json.dump(verdict, f, indent=2, ensure_ascii=False)
    for k, v in res.items():
        print(k, "rc=", v["rc"], "digest=", v.get("digest"), "val_silent=", v["validation_silent"])
        if v["rc"] != 0:
            print(v["tail"])
    print("checks:", json.dumps(checks, indent=2))
    if prof_on:
        print("bloom per-pass ms:", json.dumps(prof_on["bloom_per_pass_ms"]),
              "total:", prof_on["bloom_total_ms"])
    print("VERDICT:", verdict["verdict"])
    return 0 if verdict["verdict"] == "PASS" else 1


if __name__ == "__main__":
    sys.exit(main())
