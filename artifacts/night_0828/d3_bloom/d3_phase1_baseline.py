#!/usr/bin/env python3
"""D3 bloom 接线前基线（phase 1）：持 GPU 锁构建现行源码（pre-bloom,含 D1+D2
加性面默认 off）→ target-night,跑 off 基线;再跑 target/release 23:37 无
dither 无 bloom 二进制同命令基线。两 digest 必须位级一致（D1/D2 默认面零漂移
交叉核验 + bloom 改前锚）。
"""
import json
import os
import subprocess
import sys

sys.path.insert(0, "ci")
from gpu_device_lock import gpu_device_lock

ROOT = r"H:\rurix"
OUT = os.path.join(ROOT, "artifacts", "night_0828", "d3_bloom")
FRAMES = ["--frames", "8", "--warmup", "2", "--hidden"]


def run(exe, evidence, extra=None):
    env = dict(os.environ)
    env["RURIX_VK_VALIDATION"] = "1"
    argv = [exe, *FRAMES, "--evidence", evidence] + (extra or [])
    r = subprocess.run(argv, cwd=ROOT, env=env, capture_output=True, text=True, timeout=1200)
    out = r.stdout + r.stderr
    rec = {"exe": exe, "argv": argv[1:], "rc": r.returncode}
    if r.returncode == 0 and os.path.isfile(evidence):
        ev = json.load(open(evidence, encoding="utf-8"))
        rec["digest"] = ev["digest"]
        rec["render_digest"] = ev["render_digest"]
        rec["real_render_frame_ms"] = ev["real_render_frame_ms"]
        rec["encode_gpu_ms"] = ev["stats"]["encode_gpu_ms"]
    rec["validation_silent"] = ("VUID-" not in out) and ("Validation Error" not in out)
    rec["tail"] = out.strip()[-400:]
    return rec


def main():
    os.makedirs(OUT, exist_ok=True)
    env = dict(os.environ)
    env["CARGO_TARGET_DIR"] = r"H:\rurix\target-night"
    with gpu_device_lock(purpose="d3 bloom phase1 pre-bloom baseline build+run"):
        b = subprocess.run(
            ["cargo", "build", "--release", "-p", "rurix-render", "--features",
             "vendor-upscale", "--bin", "g31_window_present"],
            cwd=ROOT, env=env, capture_output=True, text=True, timeout=3600)
        print("build rc=", b.returncode)
        if b.returncode != 0:
            print((b.stdout + b.stderr)[-3000:])
            return 1
        res = {
            "night_pre": run(os.path.join(ROOT, "target-night", "release", "g31_window_present.exe"),
                             os.path.join(OUT, "pre_off_night.json")),
            "rel_2337": run(os.path.join(ROOT, "target", "release", "g31_window_present.exe"),
                            os.path.join(OUT, "pre_off_2337.json")),
        }
    with open(os.path.join(OUT, "d3_phase1_baseline.json"), "w", encoding="utf-8") as f:
        json.dump(res, f, indent=2, ensure_ascii=False)
    for k, v in res.items():
        print(k, "rc=", v["rc"], "digest=", v.get("digest"), "render=", v.get("render_digest"),
              "val_silent=", v["validation_silent"])
        if v["rc"] != 0:
            print(v["tail"])
    same = (res["night_pre"].get("digest") == res["rel_2337"].get("digest")
            and res["night_pre"].get("render_digest") == res["rel_2337"].get("render_digest"))
    print("BASELINE_MATCH:", same)
    return 0 if same else 1


if __name__ == "__main__":
    sys.exit(main())
