#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Day 0829 ptgt「Bistro PT 真值对照器」一键验收器(GPU 锁内,任一步失败停跑)。

用法: py -3 run_ptgt.py [--quick] [--skip-build] [--skip-raster]

序列(战役验收环同律):
  build         cargo build --release -p rurix-render --features vendor-upscale
                --bin g31_pt_gt(CARGO_TARGET_DIR=target-night;GPU 锁外)
  spv           rurixc 双 kernel 编译(fork g31_pt_gt.rx + 冻结母版
                g12_pt_production.rx)→ .tmp/ptgt/spv/ + sha 登记
  equiv         g31_pt_gt --selftest equiv(fork gates 全 off ≡ 母版逐位——
                同场景/同 RNG/同 params 双内核 ProdImage 全字段位级)
  furnace       g31_pt_gt --selftest furnace(GGX 白炉能量界:不造能 +
                metal 粗糙度单调 + dielectric 带)
  render_match  --render --lights match(契约 4 点光 + A1 聚类 12 灯 gain4,
                emissive 只可见——与光栅 full 灯光同源)双跑位级内嵌
  render_area   --render --lights area(44k emissive 灯片真面光,去点光防双计)
  red_seed      同参两种子低分辨率渲染,frame digest 必异(RED 臂)
  raster_legs   g31_window_present 三腿无 AE 显式组合(f0off / f0on /
                f0on+refl;RURIX_G31_DUMP_F32=1 → 1080p 线性 f32 帧)
  compare       compare_pt_raster.py:f0 臂(A vs B)与 refl 臂(B vs C)
                vs PT match 真值(同 scene-linear 未曝光域)
  png           exr2png.py 出可看图(ev100=-4 曝光 + ACES)
  frozen_same   冻结面四文件 sha256 == 本役开工基线快照
  汇总 ACCEPTANCE_SUMMARY.json(fails==0 即 PASS)

口径登记:PT 分辨率默认 960x540 spp64(RNG 流 = w*h*spp*26*4B ≈ 3.4GB host);
raster f32 帧 = post-TSR pre-encode 未曝光 scene-linear(与 PT EXR 同域);
对照组合去 bloom(加性后效 PT 无对应项)、无 AE(红修 #1 教训)、带 ambient
0.004(PT 无 ambient 项,系统性小偏差如实登记)。
"""
from __future__ import annotations

import argparse
import json
import os
import shutil
import subprocess
import sys
import time
from hashlib import sha256
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

DAY = ROOT / "artifacts" / "day_0829_ptgt"
TOOLS = DAY / "tools"
SPV_DIR = ROOT / ".tmp" / "ptgt" / "spv"
RURIXC = ROOT / "target-night" / "release" / "rurixc.exe"
PTGT = ROOT / "target-night" / "release" / "g31_pt_gt.exe"
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
F32_DUMP = ROOT / ".tmp" / "g31_gates" / "hzb" / "last_f32.bin"
RASTER_W, RASTER_H = 1920, 1080

# 冻结面基线(本役开工快照 2026-08-29;.tmp/ptgt/frozen_baseline.txt 同源)。
FROZEN = {
    "src/rurix-render/src/gi/path_trace.rs":
        "278738c84eb95e4d17ede04d18448865f2f42d4b265dc1621fcd5b02d58eccec",
    "src/rurix-render/src/gi/path_trace/prod.rs":
        "494c6ce17a3d32658829754d4cad1c146ad42a4a7074b36c761feef833ac65c5",
    "src/rurix-render/kernels/g12_pt_production.rx":
        "0783b1c61cfc013588bda80e3fe3d17a8a71cbf60e1f84cbf0d563469a074304",
    "src/rurix-render/src/bin/g14_3_lane/g14_3_lane_body.rs":
        "2c7a96a72482d2c6639b8125cf1be1eb328d9a6ce4201e43a5607c9a4bf79c4b",
}

# 无 AE 显式组合(run_arm.py EXPLICIT_NOAE 减 bloom/dither/normal-maps——
# bloom = 加性后效 PT 无对应,dither = encode 域不进 f32 帧,normal-maps =
# PT 无贴图法线;登记面)。
RASTER_BASE = [
    "--smooth-normals", "on", "--ggx", "on",
    "--lamp-lights", "on", "--lamp-gain", "4",
    "--textures", "on", "--tsr-quality", "on",
    "--gi2", "on", "--gi2-clamp", "0.01", "--emissive-tex", "on",
]
RASTER_LEGS = {
    "f0off": [],
    "f0on": ["--metal-f0", "on"],
    "f0refl": ["--metal-f0", "on", "--rt-reflect", "on"],
}


def read_tau() -> float:
    doc = json.loads((ROOT / "milestones" / "g12" / "g12_budget.json").read_text(encoding="utf-8"))

    def walk(v):
        if isinstance(v, dict):
            if v.get("id") == "g12.pt.rr_tau":
                yield v
            for x in v.values():
                yield from walk(x)
        elif isinstance(v, list):
            for x in v:
                yield from walk(x)

    for e in walk(doc):
        return float(e["measured_value"])
    raise SystemExit("g12_budget.json 缺 g12.pt.rr_tau")


def env_gpu(extra: dict | None = None) -> dict:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env.pop("RURIX_G18_AMBIENT", None)
    env.pop("RURIX_G31_DUMP_F32", None)
    if extra:
        env.update(extra)
    return env


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--quick", action="store_true", help="smoke 档:480x270 spp16")
    ap.add_argument("--skip-build", action="store_true")
    ap.add_argument("--skip-raster", action="store_true", help="跳过光栅腿与 compare(仅 PT 面)")
    args = ap.parse_args()
    w, h, spp = (480, 270, 16) if args.quick else (960, 540, 64)
    tau = read_tau()

    for d in ("render", "raster", "png", "ev", "anchors"):
        (DAY / d).mkdir(parents=True, exist_ok=True)
    log = open(DAY / "ptgt_log.jsonl", "a", encoding="utf-8")
    rows: list[dict] = []
    fails = 0

    def rec(row: dict) -> None:
        row["t"] = time.strftime("%H:%M:%S")
        log.write(json.dumps(row, ensure_ascii=False) + "\n")
        log.flush()
        rows.append(row)
        print(json.dumps(row, ensure_ascii=False), flush=True)

    def run(step: str, cmd: list[str], env: dict, timeout: int = 5400) -> tuple[bool, subprocess.CompletedProcess]:
        t0 = time.time()
        r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)
        vuid = (r.stderr or "").count("VUID-")
        ok = r.returncode == 0 and vuid == 0
        row = {"step": step, "rc": r.returncode, "vuid": vuid, "wall_s": round(time.time() - t0, 1), "pass": ok}
        if not ok:
            row["cmd"] = " ".join(cmd[:12])
            row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-10:]
            row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-6:]
        rec(row)
        return ok, r

    # ① build(GPU 锁外)。
    if not args.skip_build:
        env = dict(os.environ)
        env["CARGO_TARGET_DIR"] = str(ROOT / "target-night")
        ok, _ = run("build", ["cargo", "build", "--release", "-p", "rurix-render",
                              "--features", "vendor-upscale", "--bin", "g31_pt_gt"], env, timeout=3600)
        fails += 0 if ok else 1
        if fails:
            return finish(rows, fails)

    # ② SPV 双编译 + sha。
    SPV_DIR.mkdir(parents=True, exist_ok=True)
    spv_fork = SPV_DIR / "g31_pt_gt.spv"
    spv_frozen = SPV_DIR / "g12_pt_production.spv"
    for name, src, out in (("spv_fork", "src/rurix-render/kernels/g31_pt_gt.rx", spv_fork),
                           ("spv_frozen", "src/rurix-render/kernels/g12_pt_production.rx", spv_frozen)):
        ok, _ = run(name, [str(RURIXC), src, "--target", "vulkan", "-o", str(out)], dict(os.environ), timeout=300)
        fails += 0 if ok else 1
    if fails:
        return finish(rows, fails)
    rec({"step": "spv_sha", "fork": sha256(spv_fork.read_bytes()).hexdigest(),
         "frozen": sha256(spv_frozen.read_bytes()).hexdigest(), "pass": True})

    common = ["--tau", repr(tau), "--spv", str(spv_fork)]

    with gpu_device_lock(purpose="day0829 ptgt PT 真值对照器验收", timeout_s=7200.0):
        # ③ equiv(fork off ≡ 母版逐位)。
        ok, _ = run("equiv", [str(PTGT), "--selftest", "equiv", *common,
                              "--spv-frozen", str(spv_frozen)], env_gpu())
        fails += 0 if ok else 1
        # ④ furnace。
        ok, r = run("furnace", [str(PTGT), "--selftest", "furnace", *common], env_gpu())
        fails += 0 if ok else 1
        if ok:
            for ln in (r.stdout or "").splitlines():
                if "furnace" in ln and "{" in ln:
                    rec({"step": "furnace_measured", "line": ln.strip()[:400], "pass": True})
        # ⑤ render match / area。
        digests: dict[str, str] = {}
        for preset in ("match", "area"):
            out_dir = DAY / "render"
            ok, r = run(f"render_{preset}", [str(PTGT), "--render", "--lights", preset,
                                             "--w", str(w), "--h", str(h), "--spp", str(spp),
                                             *common, "--out-dir", str(out_dir)], env_gpu())
            fails += 0 if ok else 1
            if ok:
                rp = out_dir / "pt_receipt.json"
                if rp.is_file():
                    shutil.move(str(rp), str(out_dir / f"pt_receipt_{preset}.json"))
                    d = json.loads((out_dir / f"pt_receipt_{preset}.json").read_text(encoding="utf-8"))
                    digests[preset] = d.get("frame_content_digest", "")
                    rec({"step": f"receipt_{preset}", "digest": digests[preset],
                         "mean_luminance": d.get("mean_luminance"), "lights": d.get("light_counts"),
                         "double_run_bitexact": d.get("double_run_bitexact"), "pass": True})
        # ⑥ RED seed 臂。
        red_dir = DAY / "render" / "red"
        red_dir.mkdir(parents=True, exist_ok=True)
        red_d: list[str] = []
        for sd in ("11", "22"):
            ok, _ = run(f"red_seed_{sd}", [str(PTGT), "--render", "--lights", "match",
                                           "--w", "320", "--h", "180", "--spp", "8", "--seed", sd,
                                           *common, "--out-dir", str(red_dir)], env_gpu())
            fails += 0 if ok else 1
            rp = red_dir / "pt_receipt.json"
            if rp.is_file():
                d = json.loads(rp.read_text(encoding="utf-8"))
                red_d.append(d.get("frame_content_digest", ""))
                shutil.move(str(rp), str(red_dir / f"pt_receipt_seed{sd}.json"))
        red_ok = len(red_d) == 2 and red_d[0] != red_d[1] and all(red_d)
        rec({"step": "red_seed_verdict", "digests": red_d, "pass": red_ok})
        fails += 0 if red_ok else 1

        # ⑦ 光栅三腿(无 AE 显式组合 + f32 线性帧)。
        if not args.skip_raster and fails == 0:
            F32_DUMP.parent.mkdir(parents=True, exist_ok=True)
            for leg, extra in RASTER_LEGS.items():
                if F32_DUMP.is_file():
                    F32_DUMP.unlink()
                ok, _ = run(f"raster_{leg}", [str(WIN), "--frames", "96", "--warmup", "2", "--hidden",
                                              *RASTER_BASE, *extra,
                                              "--evidence", str(DAY / "ev" / f"raster_{leg}.json")],
                            env_gpu({"RURIX_G18_AMBIENT": "0.004", "RURIX_G31_DUMP_F32": "1"}), timeout=1800)
                fails += 0 if ok else 1
                want = RASTER_W * RASTER_H * 3 * 4
                got = F32_DUMP.stat().st_size if F32_DUMP.is_file() else -1
                if ok and got == want:
                    shutil.move(str(F32_DUMP), str(DAY / "raster" / f"{leg}_f32.bin"))
                else:
                    rec({"step": f"raster_{leg}_f32", "bytes": got, "want": want, "pass": False})
                    fails += 1

    # ⑧ compare(GPU 锁外,纯 CPU)。曝光域对齐:光栅 out_color 含契约静态曝光
    # 2^(−ev100)(scene pass 施加,首跑 ≈16× 实证),PT EXR 未曝光 → PT 侧
    # --pt-gain 2^(−ev100) 提到同域(ev100 程序读自 receipt 禁手写)。
    if not args.skip_raster and fails == 0:
        pt_exr = DAY / "render" / f"bistro-interior_ptgt_match_spp{spp}.exr"
        rec_match = json.loads(
            (DAY / "render" / "pt_receipt_match.json").read_text(encoding="utf-8"))
        pt_gain = 2.0 ** (-float(rec_match["ev100"]))
        for arm, off, on in (("f0", "f0off", "f0on"), ("refl", "f0on", "f0refl")):
            ok, r = run(f"compare_{arm}", [sys.executable, str(TOOLS / "compare_pt_raster.py"),
                                           "--pt", str(pt_exr),
                                           "--raster-off", str(DAY / "raster" / f"{off}_f32.bin"),
                                           "--raster-on", str(DAY / "raster" / f"{on}_f32.bin"),
                                           "--raster-w", str(RASTER_W), "--raster-h", str(RASTER_H),
                                           "--arm", arm, "--pt-gain", repr(pt_gain),
                                           "--out", str(DAY / f"compare_{arm}.json")], dict(os.environ), timeout=600)
            fails += 0 if ok else 1
            if ok and (DAY / f"compare_{arm}.json").is_file():
                d = json.loads((DAY / f"compare_{arm}.json").read_text(encoding="utf-8"))
                rec({"step": f"compare_{arm}_verdict", "mask": d.get("mask"), "pass": True})

    # ⑨ PNG 可看图。
    for preset in ("match", "area"):
        exr = DAY / "render" / f"bistro-interior_ptgt_{preset}_spp{spp}.exr"
        if exr.is_file():
            ok, _ = run(f"png_{preset}", [sys.executable, str(TOOLS / "exr2png.py"), str(exr),
                                          str(DAY / "png" / f"ptgt_{preset}.png"), "--ev100", "-4"],
                        dict(os.environ), timeout=600)
            fails += 0 if ok else 1

    # ⑩ 冻结面 SAME 自证。
    frozen_rows = {}
    frozen_ok = True
    for rel, want in FROZEN.items():
        got = sha256((ROOT / rel).read_bytes()).hexdigest()
        same = got == want
        frozen_ok = frozen_ok and same
        frozen_rows[rel] = {"same": same, "sha256": got}
    rec({"step": "frozen_same", "files": frozen_rows, "pass": frozen_ok})
    fails += 0 if frozen_ok else 1

    return finish(rows, fails)


def finish(rows: list[dict], fails: int) -> int:
    (DAY / "ACCEPTANCE_SUMMARY.json").write_text(
        json.dumps({"schema": "rurix.day0829.ptgt.acceptance.v1", "fails": fails,
                    "rows": rows}, ensure_ascii=False, indent=1) + "\n",
        encoding="utf-8")
    print("PTGT-ACCEPT", json.dumps({"fails": fails}))
    return 0 if fails == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
