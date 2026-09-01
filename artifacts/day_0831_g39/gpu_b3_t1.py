#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G39 GPU 批 3:T1 --lamp-restir 验收(t1_restir/REPORT.md §六清单执行器)。

段(--only anchors|det|cal|ladder|dolly|judge|all):
  anchors  off==锚两跑(all-off 8f / full19 96f,期望 = 在案锚全串)。
  det      on 臂 EXPLICIT_NOAE(+ris/nee)r1/r2 双跑位级 + 最小组合 r1/r2 + VUID=0。
  cal      ~48 簇 grid 标定(0.10/0.075/0.05 各 4f 短跑抓 stderr 簇统计)。
  ladder   A/B 矩阵六跑:{12(缺省)/26(grid0.15,k48)/~48(标定 grid,k96)} × {off,on};
           每跑 dump-present-raw(every 4)+ profile-json + evidence(单跑,阶梯是量测)。
  dolly    disocclusion 观察臂 240f ×2(digest 双跑位级硬门;观感归登记不判红)。
  judge    零 GPU:ab_metrics noise 四 ROI × 六臂 + render_wall p50 → T1_AB_MATRIX.json。
raw 转储 ~1.2GB 已 gitignore 登记(artifacts/day_0831_g39/t1_restir/ab/**/p.raw*)。
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
T1 = ROOT / "artifacts" / "day_0831_g39" / "t1_restir"
EV = T1 / "ev"
AB = T1 / "ab"
LOG = T1 / "b3_log.jsonl"
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
AB_METRICS_DIR = ROOT / "artifacts" / "day_0829_realism" / "tools"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL19 = "sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1"
BUDGET_MS = 11.11

NOAE = ["--quality", "off", "--smooth-normals", "on", "--ggx", "on",
        "--lamp-lights", "on", "--lamp-gain", "4", "--textures", "on",
        "--bloom", "on", "--dither", "on", "--tsr-quality", "on",
        "--gi2", "on", "--gi2-clamp", "0.01", "--emissive-tex", "on",
        "--metal-f0", "on", "--rt-ao", "on", "--soft-shadows", "on",
        "--soft-shadow-samples", "1", "--rt-reflect", "on", "--gi2-tex", "on",
        "--normal-maps", "on", "--transparency", "on",
        "--gi2-ris", "on", "--gi2-nee", "on"]

CAL_GRIDS = ["0.10", "0.075", "0.05"]
LAMP_RE = re.compile(
    r"lamp-lights 提取 emissive_tris=(\d+) clusters=(\d+) kept=(\d+) dropped=(\d+)")
ROIS = {"wall": "1400,150,480,270", "floor": "1100,800,480,270",
        "dark_arch": "360,0,360,180", "dark_table": "560,560,560,200"}

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

FAILS: list[str] = []
NOTES: list[str] = []


def log(step: str, **kw) -> None:
    rec = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    LOG.parent.mkdir(parents=True, exist_ok=True)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(rec, ensure_ascii=False) + "\n")
    print(f"[{rec['t']}] {step}: {kw.get('status', '')}", flush=True)


def env_of(grid: str | None = None, ambient: bool = False) -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env.pop("RURIX_G31_LAMP_GRID_M", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    if ambient:
        env["RURIX_G18_AMBIENT"] = "0.004"
    if grid is not None:
        env["RURIX_G31_LAMP_GRID_M"] = grid
    return env


def run_win(step: str, extra: list[str], frames: int, warmup: int,
            ev_path: Path, grid: str | None = None, ambient: bool = False,
            keep_stderr: Path | None = None) -> dict:
    ev_path.parent.mkdir(parents=True, exist_ok=True)
    cmd = [str(WIN), "--frames", str(frames), "--warmup", str(warmup), "--hidden",
           *extra, "--evidence", str(ev_path)]
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=1800,
                       env=env_of(grid, ambient))
    wall = round(time.time() - t0, 1)
    if keep_stderr is not None:
        keep_stderr.write_text(p.stderr or "", encoding="utf-8")
    vuid = (p.stderr or "").count("VUID-")
    evd: dict = {}
    digest = None
    if p.returncode == 0 and ev_path.is_file():
        evd = json.loads(ev_path.read_text(encoding="utf-8"))
        digest = evd.get("digest")
    ok = p.returncode == 0 and vuid == 0 and digest is not None
    m = LAMP_RE.search(p.stderr or "")
    row = {"digest": digest, "vuid": vuid, "rc": p.returncode, "wall_s": wall,
           "real_render_frame_ms": evd.get("real_render_frame_ms"),
           "digest_seq_len": len(evd.get("digest_seq") or []),
           "clusters": int(m.group(2)) if m else None,
           "kept": int(m.group(3)) if m else None, "ok": ok}
    log(step, status="OK" if ok else f"FAIL rc={p.returncode} vuid={vuid}",
        **{k: v for k, v in row.items() if k not in ("ok",)},
        stderr_tail=(p.stderr or "").strip().splitlines()[-8:] if not ok else None)
    if not ok:
        FAILS.append(step)
    row["evidence"] = evd
    return row


def seg_anchors() -> None:
    r = run_win("anchors.n1_alloff", ["--quality", "off"], 8, 2, EV / "n1_alloff_8f.json")
    if r["digest"] != ANCHOR_ALLOFF:
        log("anchors.n1_verdict", status="FAIL", got=(r["digest"] or "")[:24])
        FAILS.append("anchors.n1")
    else:
        log("anchors.n1_verdict", status="OK")
    r = run_win("anchors.n4_full19", [], 96, 2, EV / "n4_full19_96f.json")
    if r["digest"] != ANCHOR_FULL19:
        log("anchors.n4_verdict", status="FAIL", got=(r["digest"] or "")[:24])
        FAILS.append("anchors.n4")
    else:
        log("anchors.n4_verdict", status="OK")


def seg_det() -> None:
    pairs = [
        ("on", NOAE + ["--lamp-restir", "on"], True),
        ("onmin", ["--quality", "off", "--smooth-normals", "on", "--textures", "on",
                   "--lamp-lights", "on", "--lamp-restir", "on",
                   "--lamp-restir-mcap", "16"], False),
    ]
    for tag, flags, amb in pairs:
        d1 = run_win(f"det.{tag}_r1", flags, 96, 2, EV / f"{tag}_r1.json", ambient=amb)
        d2 = run_win(f"det.{tag}_r2", flags, 96, 2, EV / f"{tag}_r2.json", ambient=amb)
        bit = d1["ok"] and d2["ok"] and d1["digest"] == d2["digest"]
        log(f"det.{tag}_pair", status="OK" if bit else "FAIL",
            double_run_bitexact=bit, digest=d1["digest"])
        if not bit:
            FAILS.append(f"det.{tag}_pair")


def seg_cal() -> dict:
    """~48 簇档 grid 标定;返回 {grid: kept}。选 kept 最近 48 的档写 cal.json。"""
    kept_of: dict[str, int | None] = {}
    for g in CAL_GRIDS:
        r = run_win(f"cal.g{g}", ["--quality", "full", "--lamp-k", "96"], 4, 0,
                    EV / f"cal_{g}.json", grid=g)
        kept_of[g] = r["kept"]
    valid = {g: k for g, k in kept_of.items() if isinstance(k, int)}
    pick = min(valid, key=lambda g: abs(valid[g] - 48)) if valid else None
    out = {"kept_of": kept_of, "picked_grid": pick,
           "picked_kept": valid.get(pick) if pick else None}
    (EV / "cal.json").write_text(json.dumps(out, ensure_ascii=False, indent=1) + "\n",
                                 encoding="utf-8")
    log("cal.pick", status="OK" if pick else "FAIL", **out)
    if not pick:
        FAILS.append("cal.pick")
    return out


def ladder_arm(tag: str, flags: list[str], grid: str | None) -> None:
    d = AB / tag
    d.mkdir(parents=True, exist_ok=True)
    run_win(f"ladder.{tag}",
            flags + ["--dump-present-raw", str(d / "p.raw"),
                     "--dump-present-every", "4",
                     "--profile-json", str(d / "prof.json")],
            96, 2, d / "ev.json", grid=grid, keep_stderr=d / "stderr.txt")


def seg_ladder(grid48: str | None) -> None:
    ladder_arm("k12_off", [], None)
    ladder_arm("k12_on", ["--lamp-restir", "on"], None)
    ladder_arm("k26_off", ["--quality", "full", "--lamp-k", "48"], "0.15")
    ladder_arm("k26_on", ["--quality", "full", "--lamp-k", "48",
                          "--lamp-restir", "on"], "0.15")
    if grid48:
        ladder_arm("k48_off", ["--quality", "full", "--lamp-k", "96"], grid48)
        ladder_arm("k48_on", ["--quality", "full", "--lamp-k", "96",
                              "--lamp-restir", "on"], grid48)
    else:
        NOTES.append("~48 簇档标定失败,阶梯降为两档如实登记")


def seg_dolly() -> None:
    flags = NOAE + ["--lamp-restir", "on", "--auto-move", "dolly"]
    d1 = run_win("dolly.r1", flags, 240, 2, EV / "dolly_r1.json", ambient=True)
    d2 = run_win("dolly.r2", flags, 240, 2, EV / "dolly_r2.json", ambient=True)
    seq1 = d1["evidence"].get("digest_seq")
    seq2 = d2["evidence"].get("digest_seq")
    bit = d1["ok"] and d2["ok"] and d1["digest"] == d2["digest"] and seq1 == seq2
    log("dolly.pair", status="OK" if bit else "FAIL", double_run_bitexact=bit,
        seq_len=len(seq1 or []))
    if not bit:
        FAILS.append("dolly.pair")


def p50_of(prof_path: Path) -> float | None:
    if not prof_path.is_file():
        return None
    try:
        doc = json.loads(prof_path.read_text(encoding="utf-8"))
    except Exception:
        return None
    fs = doc.get("frame_segments")
    seg = None
    if isinstance(fs, dict):
        seg = fs.get("render_wall")
    elif isinstance(fs, list):
        seg = next((s for s in fs if isinstance(s, dict)
                    and s.get("name", s.get("segment")) == "render_wall"), None)
    if not isinstance(seg, dict):
        return None
    for k in ("p50_ms", "p50", "median_ms"):
        if isinstance(seg.get(k), (int, float)):
            return float(seg[k])
    return None


def seg_judge() -> None:
    """零 GPU 判读:四 ROI 噪声 × 六臂 + 帧时 → T1_AB_MATRIX.json。"""
    sys.path.insert(0, str(AB_METRICS_DIR))
    import ab_metrics  # noqa: E402(只调用不修改)
    ap = ab_metrics.build_parser()

    def noise_of(arm: str) -> dict | None:
        d = AB / arm
        paths = [str(d / f"p.raw.f{64 + 4 * k:04}") for k in range(8)]
        if not all(Path(p).is_file() for p in paths):
            return None
        out = {}
        for label, rect in ROIS.items():
            a = ap.parse_args(["noise", *paths, "--rect", rect, "--label", label])
            r = a.func(a)
            c = r["crops"][label]
            out[label] = {"std_mean": c["temporal_std_mean"],
                          "std_p95": c["temporal_std_p95"]}
        return out

    tiers = {}
    for tier in ("k12", "k26", "k48"):
        row: dict = {}
        for side in ("off", "on"):
            arm = f"{tier}_{side}"
            d = AB / arm
            if not (d / "ev.json").is_file():
                continue
            evd = json.loads((d / "ev.json").read_text(encoding="utf-8"))
            st = (d / "stderr.txt").read_text(encoding="utf-8") if (d / "stderr.txt").is_file() else ""
            m = LAMP_RE.search(st)
            row[side] = {
                "digest": evd.get("digest"),
                "real_render_frame_ms": evd.get("real_render_frame_ms"),
                "render_wall_p50_ms": p50_of(d / "prof.json"),
                "clusters": int(m.group(2)) if m else None,
                "kept": int(m.group(3)) if m else None,
                "noise": noise_of(arm),
            }
        if {"off", "on"} <= row.keys() and row["off"]["noise"] and row["on"]["noise"]:
            shr = {}
            for roi in ("dark_arch", "dark_table"):
                o = row["off"]["noise"][roi]["std_p95"]
                n = row["on"]["noise"][roi]["std_p95"]
                shr[roi] = round((o - n) / o * 100.0, 2) if o else None
            vals = [v for v in shr.values() if v is not None]
            row["shrink_dark_p95_pct"] = shr
            row["shrink_dark_min_pct"] = min(vals) if vals else None
            for side in ("off", "on"):
                p50 = row[side]["render_wall_p50_ms"]
                row[side]["within_budget_p50"] = (p50 <= BUDGET_MS) if p50 else None
        tiers[tier] = row
    out = {"schema": "rurix.day0831.g39.t1_ab_matrix.v1",
           "budget_ms": BUDGET_MS, "rois": ROIS,
           "caliber": "presented u8 尾段 f0064..f0092 恰 8 张;dark ROI p95 shrink;帧时 = profile render_wall p50(fallback real_render_frame_ms)",
           "tiers": tiers, "notes": NOTES}
    (AB / "T1_AB_MATRIX.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    log("judge.matrix", status="OK", tiers=list(tiers.keys()))


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="all",
                    choices=["all", "anchors", "det", "cal", "ladder", "dolly", "judge"])
    a = ap.parse_args()
    if not WIN.is_file():
        print(f"FAIL: 缺 exe {WIN}")
        return 2
    EV.mkdir(parents=True, exist_ok=True)
    grid48 = None
    gpu_segs = [s for s in ("anchors", "det", "cal", "ladder", "dolly")
                if a.only in ("all", s)]
    if gpu_segs:
        with gpu_device_lock(purpose="G39 GPU批3(T1 lamp-restir 验收)", timeout_s=3 * 3600.0):
            for name in gpu_segs:
                log(f"seg.{name}", status="BEGIN")
                n0 = len(FAILS)
                try:
                    if name == "anchors":
                        seg_anchors()
                    elif name == "det":
                        seg_det()
                    elif name == "cal":
                        grid48 = seg_cal().get("picked_grid")
                    elif name == "ladder":
                        if grid48 is None and (EV / "cal.json").is_file():
                            grid48 = json.loads((EV / "cal.json").read_text(
                                encoding="utf-8")).get("picked_grid")
                        seg_ladder(grid48)
                    elif name == "dolly":
                        seg_dolly()
                except Exception as ex:
                    log(f"seg.{name}", status=f"EXC {type(ex).__name__}: {ex}")
                    FAILS.append(f"{name}.exc")
                log(f"seg.{name}", status="END", seg_fails=FAILS[n0:])
    if a.only in ("all", "judge"):
        try:
            seg_judge()
        except Exception as ex:
            log("seg.judge", status=f"EXC {type(ex).__name__}: {ex}")
            FAILS.append("judge.exc")
    (T1 / "B3_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0831.g39.b3.v1",
        "fails": FAILS, "notes": NOTES,
        "verdict": "PASS" if not FAILS else "FAIL",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("B3 PASS" if not FAILS else f"B3 FAILS: {FAILS}")
          + (f" NOTES: {NOTES}" if NOTES else ""))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
