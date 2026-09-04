#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G40 GPU 批 3:T1 画质补窗验收(三件合一;G39 gpu_b3_t1.py 同型扩段)。

段(--only anchors|det|verify|ladder|dolly|storm|judge|all):
  anchors  off==锚两跑(终态树;all-off 8f / full19 96f = 在案锚全串,硬门)。
  det      on 臂 EXPLICIT_NOAE r1/r2 双跑位级 + 最小组合 r1/r2(digest 换代
           如实登记——kernel stride/T1b/T1c 进链,on 臂无锚)。
  verify   T1a 镜像对拍批:NOAE+on+--lamp-restir-verify 16(22f warmup2,
           覆盖首帧无历史 + merge 路)r1/r2 双跑位级 + GREEN 行 + ×clamp 4
           正交(钳制不扰 reservoir 链的机核)+ red-arm phase/resv 必红。
  ladder   A/B 阶梯八跑:{12 缺省 / 26=grid0.15+k48 / 38=grid0.10+k96} ×
           {off,on} + T1c 对照双臂(k26_on_clamp4 / k26_on_mcap4);
           每跑 dump-present-raw(every 4)+ profile-json。
  dolly    D-4 消解:rej 缺省臂 240f r1/r2(digest_seq 双跑位级硬门)+
           norej 臂(两拒 0 = G39 v1 语言形)各带 raw 转储 → 边缘噪声对照。
  storm    风暴组合首验:--lamp-restir on × --window-storm 3(30f dolly;
           rc=0 + resize_eras≥1 + VUID=0 + exit=frames_done,e4_storm 口径;
           era 重建首帧 has_history=0 语义由 params[74] 门承载)。
  judge    零 GPU:四 ROI 噪声矩阵 + dolly 边缘 ROI 对照 + render_wall p50
           (26 on ≤ 11.11ms 硬门)→ T1_AB_MATRIX.json。
raw 转储 ~2GB gitignore 登记(artifacts/day_0901_g40/t1_restir2/ab/**/p.raw*)。
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
T1 = ROOT / "artifacts" / "day_0901_g40" / "t1_restir2"
EV = T1 / "ev"
AB = T1 / "ab"
LOG = T1 / "b3_log.jsonl"
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
AB_METRICS_DIR = ROOT / "artifacts" / "day_0829_realism" / "tools"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_FULL19 = "sha256:a5521e4708a814e364fd3bf95b18f0ab69b6646efd039246e8686533c30e4fb1"
BUDGET_MS = 11.11
GRID_26 = "0.15"
GRID_38 = "0.10"

NOAE = ["--quality", "off", "--smooth-normals", "on", "--ggx", "on",
        "--lamp-lights", "on", "--lamp-gain", "4", "--textures", "on",
        "--bloom", "on", "--dither", "on", "--tsr-quality", "on",
        "--gi2", "on", "--gi2-clamp", "0.01", "--emissive-tex", "on",
        "--metal-f0", "on", "--rt-ao", "on", "--soft-shadows", "on",
        "--soft-shadow-samples", "1", "--rt-reflect", "on", "--gi2-tex", "on",
        "--normal-maps", "on", "--transparency", "on",
        "--gi2-ris", "on", "--gi2-nee", "on"]

LAMP_RE = re.compile(
    r"lamp-lights 提取 emissive_tris=(\d+) clusters=(\d+) kept=(\d+) dropped=(\d+)")
# T1a 两级判据 GREEN 行(run3 起):边界事件计数 + margin p100 measured 登记。
GREEN_RE = re.compile(
    r"T1a 镜像对拍 GREEN frames=(\d+) pixels=(\d+) hit_pixels=(\d+) merged=(\d+) "
    r"y_mismatch=(\d+) y_attributed=(\d+) y_unattributed=0 margin_abs_p100=([0-9.e+-]+) "
    r"ulp_bound=([0-9.e+-]+) m_mismatch=(\d+) wsum_absdiff_p100=([0-9.e+-]+) "
    r"w_absdiff_p100=([0-9.e+-]+)")
ROIS = {"wall": "1400,150,480,270", "floor": "1100,800,480,270",
        "dark_arch": "360,0,360,180", "dark_table": "560,560,560,200"}
DOLLY_ROIS = {"edge_l": "0,0,192,1080", "edge_r": "1728,0,192,1080",
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
    try:
        print(f"[{rec['t']}] {step}: {kw.get('status', '')}", flush=True)
    except OSError:
        # 首跑 09-01 20:18 在 det.onmin_r2 起跑时因宿主终端关闭致 stdout 失效
        # (OSError Errno 22)整批中断;日志文件已落盘,控制台不可写不应中断 GPU 批。
        pass


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
            keep_stderr: Path | None = None, expect_rc0: bool = True,
            stderr_must: list[str] | None = None) -> dict:
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
    rc_ok = (p.returncode == 0 and digest is not None) if expect_rc0 else (p.returncode != 0)
    miss = [s for s in (stderr_must or []) if s not in (p.stderr or "")]
    ok = rc_ok and not miss and (vuid == 0 if expect_rc0 else True)
    m = LAMP_RE.search(p.stderr or "")
    row = {"digest": digest, "vuid": vuid, "rc": p.returncode, "wall_s": wall,
           "real_render_frame_ms": evd.get("real_render_frame_ms"),
           "clusters": int(m.group(2)) if m else None,
           "kept": int(m.group(3)) if m else None, "ok": ok}
    g = GREEN_RE.search(p.stderr or "")
    if g:
        row["verify_green"] = {
            "frames": int(g.group(1)), "pixels": int(g.group(2)),
            "hit_pixels": int(g.group(3)), "merged": int(g.group(4)),
            "y_mismatch": int(g.group(5)), "y_attributed": int(g.group(6)),
            "margin_abs_p100": float(g.group(7)), "ulp_bound": float(g.group(8)),
            "m_mismatch": int(g.group(9)), "wsum_absdiff_p100": float(g.group(10)),
            "w_absdiff_p100": float(g.group(11))}
    log(step, status="OK" if ok else f"FAIL rc={p.returncode} vuid={vuid} miss={miss}",
        **{k: v for k, v in row.items() if k != "ok"},
        stderr_tail=(p.stderr or "").strip().splitlines()[-10:] if not ok else None)
    if not ok:
        FAILS.append(step)
    row["evidence"] = evd
    row["stderr"] = p.stderr or ""
    return row


def seg_anchors() -> None:
    r = run_win("anchors.n1_alloff", ["--quality", "off"], 8, 2, EV / "n1_alloff_8f.json")
    ok = r["digest"] == ANCHOR_ALLOFF
    log("anchors.n1_verdict", status="OK" if ok else "FAIL", got=(r["digest"] or "")[:24])
    if not ok:
        FAILS.append("anchors.n1")
    r = run_win("anchors.n4_full19", [], 96, 2, EV / "n4_full19_96f.json")
    ok = r["digest"] == ANCHOR_FULL19
    log("anchors.n4_verdict", status="OK" if ok else "FAIL", got=(r["digest"] or "")[:24])
    if not ok:
        FAILS.append("anchors.n4")


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
    # on 臂 digest 换代登记(G39 在案 on_r1 bec12f48…;kernel stride/T1b 进链
    # 预期不等,如实登记不判读)。
    g39 = ROOT / "artifacts/day_0831_g39/t1_restir/ev/on_r1.json"
    if g39.is_file() and (EV / "on_r1.json").is_file():
        d39 = json.loads(g39.read_text(encoding="utf-8")).get("digest")
        d40 = json.loads((EV / "on_r1.json").read_text(encoding="utf-8")).get("digest")
        NOTES.append(f"on 臂 digest 换代:G39 {str(d39)[:24]} → G40 {str(d40)[:24]}(SPV 8ac52dc4 + stride12 + T1b 缺省拒,如实登记)")


def seg_verify() -> None:
    base = NOAE + ["--lamp-restir", "on", "--lamp-restir-verify", "16"]
    v1 = run_win("verify.r1", base, 22, 2, EV / "verify_r1.json", ambient=True,
                 keep_stderr=EV / "verify_r1_stderr.txt",
                 stderr_must=["T1a 镜像对拍 GREEN"])
    v2 = run_win("verify.r2", base, 22, 2, EV / "verify_r2.json", ambient=True,
                 stderr_must=["T1a 镜像对拍 GREEN"])
    bit = v1["ok"] and v2["ok"] and v1["digest"] == v2["digest"]
    log("verify.pair", status="OK" if bit else "FAIL", double_run_bitexact=bit)
    if not bit:
        FAILS.append("verify.pair")
    # 两级判据 measured 登记(硬门 = GREEN 行存在 ⇔ y_unattributed=0;边界事件
    # 计数/margin p100 如实登记不判读;r1/r2 统计位级同值 = 归因确定性自证)。
    g1, g2 = v1.get("verify_green"), v2.get("verify_green")
    if g1 and g2:
        NOTES.append(
            f"T1a 边界事件 measured:r1 y_attributed={g1['y_attributed']}/pixels={g1['pixels']} "
            f"margin_abs_p100={g1['margin_abs_p100']:.3e}(ulp_bound {g1['ulp_bound']:.3e}) "
            f"wsum_p100={g1['wsum_absdiff_p100']:.3e} w_p100={g1['w_absdiff_p100']:.3e};"
            f"r1==r2 统计同值={g1 == g2}")
    # 正交:×clamp 4(钳制只动输出权重不动 reservoir 写回 ⇒ 镜像必须仍绿)。
    c4 = run_win("verify.clamp4", base + ["--lamp-restir-clamp", "4"], 22, 2,
                 EV / "verify_clamp4.json", ambient=True,
                 stderr_must=["T1a 镜像对拍 GREEN"])
    g4 = c4.get("verify_green")
    if g1 and g4:
        NOTES.append(f"T1c 正交:clamp4 镜像统计 == 无钳 r1 = {g4 == g1}(钳制不扰 reservoir 链)")
    # red-arm 双臂(必红;fail-closed 消费路径机核)。
    run_win("verify.red_phase", base + ["--lamp-restir-verify-red", "phase"], 22, 2,
            EV / "verify_red_phase.json", ambient=True, expect_rc0=False,
            stderr_must=["T1a y-mismatch", "镜像对拍红"])
    run_win("verify.red_resv", base + ["--lamp-restir-verify-red", "resv"], 22, 2,
            EV / "verify_red_resv.json", ambient=True, expect_rc0=False,
            stderr_must=["red-arm resv 篡改", "镜像对拍红"])


def ladder_arm(tag: str, flags: list[str], grid: str | None) -> None:
    d = AB / tag
    d.mkdir(parents=True, exist_ok=True)
    run_win(f"ladder.{tag}",
            flags + ["--dump-present-raw", str(d / "p.raw"),
                     "--dump-present-every", "4",
                     "--profile-json", str(d / "prof.json")],
            96, 2, d / "ev.json", grid=grid, keep_stderr=d / "stderr.txt")


def seg_ladder() -> None:
    ladder_arm("k12_off", [], None)
    ladder_arm("k12_on", ["--lamp-restir", "on"], None)
    ladder_arm("k26_off", ["--quality", "full", "--lamp-k", "48"], GRID_26)
    ladder_arm("k26_on", ["--quality", "full", "--lamp-k", "48",
                          "--lamp-restir", "on"], GRID_26)
    ladder_arm("k38_off", ["--quality", "full", "--lamp-k", "96"], GRID_38)
    ladder_arm("k38_on", ["--quality", "full", "--lamp-k", "96",
                          "--lamp-restir", "on"], GRID_38)
    # T1c 缺省裁决对照双臂(26 簇交付档;clamp 4 vs 第一旋钮 mcap 4)。
    ladder_arm("k26_on_clamp4", ["--quality", "full", "--lamp-k", "48",
                                 "--lamp-restir", "on",
                                 "--lamp-restir-clamp", "4"], GRID_26)
    ladder_arm("k26_on_mcap4", ["--quality", "full", "--lamp-k", "48",
                                "--lamp-restir", "on",
                                "--lamp-restir-mcap", "4"], GRID_26)
    # k12_off == full19 锚(阶梯基线自证)。
    evp = AB / "k12_off" / "ev.json"
    if evp.is_file():
        d = json.loads(evp.read_text(encoding="utf-8")).get("digest")
        ok = d == ANCHOR_FULL19
        log("ladder.k12_off_anchor", status="OK" if ok else "FAIL", got=(d or "")[:24])
        if not ok:
            FAILS.append("ladder.k12_off_anchor")


def dolly_arm(tag: str, extra: list[str], dump: bool) -> dict:
    d = AB / tag
    d.mkdir(parents=True, exist_ok=True)
    flags = NOAE + ["--lamp-restir", "on", "--auto-move", "dolly"] + extra
    if dump:
        flags += ["--dump-present-raw", str(d / "p.raw"), "--dump-present-every", "4"]
    return run_win(f"dolly.{tag}", flags, 240, 2, d / "ev.json", ambient=True)


def seg_dolly() -> None:
    # rej 缺省臂(0.10/0.80)双跑位级 + raw;norej 臂(两拒 0 = G39 v1 语言形)
    # 单跑 + raw = 边缘噪声对照(D-4 消解 measured)。
    d1 = dolly_arm("dolly_rej", [], True)
    d2 = dolly_arm("dolly_rej_r2", [], False)
    seq1 = d1["evidence"].get("digest_seq")
    seq2 = d2["evidence"].get("digest_seq")
    bit = d1["ok"] and d2["ok"] and d1["digest"] == d2["digest"] and seq1 == seq2
    log("dolly.pair", status="OK" if bit else "FAIL", double_run_bitexact=bit,
        seq_len=len(seq1 or []))
    if not bit:
        FAILS.append("dolly.pair")
    dolly_arm("dolly_norej", ["--lamp-restir-depth-rej", "0",
                              "--lamp-restir-nrm-rej", "0"], True)
    g39 = ROOT / "artifacts/day_0831_g39/t1_restir/ev/dolly_r1.json"
    if g39.is_file():
        s39 = json.loads(g39.read_text(encoding="utf-8")).get("digest_seq")
        NOTES.append(f"dolly digest_seq 对 G39 基线:MATCH={seq1 == s39}(预期 False = digest 换代如实登记;确定性门 = 本役双跑位级)")


def seg_storm() -> None:
    # e4_storm 口径:30f/warmup4/dolly + 爆发 resize 3;era 重建首帧
    # has_history=0(params[74] 门,车道重建 reset 语义)。
    r = run_win("storm.restir", ["--auto-move", "dolly", "--lamp-restir", "on",
                                 "--window-storm", "3"], 30, 4,
                EV / "storm_restir.json")
    evd = r["evidence"]
    eras = evd.get("resize_eras")
    exitr = evd.get("exit_reason")
    ok = r["ok"] and (eras or 0) >= 1 and exitr == "frames_done"
    log("storm.judge", status="OK" if ok else "FAIL", resize_eras=eras,
        exit_reason=exitr, vuid=r["vuid"])
    if not ok:
        FAILS.append("storm.judge")


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
    """零 GPU 判读:阶梯噪声矩阵 + T1c 对照 + dolly 边缘噪声 + 预算门。"""
    sys.path.insert(0, str(AB_METRICS_DIR))
    import ab_metrics  # noqa: E402(只调用不修改)
    ap = ab_metrics.build_parser()

    def noise_of(arm: str, rois: dict[str, str]) -> dict | None:
        d = AB / arm
        paths = [str(d / f"p.raw.f{64 + 4 * k:04}") for k in range(8)]
        if not all(Path(p).is_file() for p in paths):
            return None
        out = {}
        for label, rect in rois.items():
            a = ap.parse_args(["noise", *paths, "--rect", rect, "--label", label])
            r = a.func(a)
            c = r["crops"][label]
            out[label] = {"std_mean": c["temporal_std_mean"],
                          "std_p95": c["temporal_std_p95"]}
        return out

    def arm_row(arm: str, rois: dict[str, str]) -> dict | None:
        d = AB / arm
        if not (d / "ev.json").is_file():
            return None
        evd = json.loads((d / "ev.json").read_text(encoding="utf-8"))
        st = (d / "stderr.txt").read_text(encoding="utf-8") if (d / "stderr.txt").is_file() else ""
        m = LAMP_RE.search(st)
        return {"digest": evd.get("digest"),
                "real_render_frame_ms": evd.get("real_render_frame_ms"),
                "render_wall_p50_ms": p50_of(d / "prof.json"),
                "clusters": int(m.group(2)) if m else None,
                "kept": int(m.group(3)) if m else None,
                "noise": noise_of(arm, rois)}

    tiers: dict = {}
    for tier in ("k12", "k26", "k38"):
        row: dict = {}
        for side in ("off", "on"):
            r = arm_row(f"{tier}_{side}", ROIS)
            if r:
                row[side] = r
        if {"off", "on"} <= row.keys() and row["off"]["noise"] and row["on"]["noise"]:
            shr = {}
            for roi in ("dark_arch", "dark_table"):
                o = row["off"]["noise"][roi]["std_p95"]
                n = row["on"]["noise"][roi]["std_p95"]
                shr[roi] = round((o - n) / o * 100.0, 2) if o else None
            vals = [v for v in shr.values() if v is not None]
            row["shrink_dark_p95_pct"] = shr
            row["shrink_dark_min_pct"] = min(vals) if vals else None
        tiers[tier] = row
    # T1c 对照双臂(26 簇档;缺省裁决素材)。
    t1c = {arm: arm_row(arm, ROIS) for arm in ("k26_on_clamp4", "k26_on_mcap4")}
    # 26 on 预算硬门。
    k26on = tiers.get("k26", {}).get("on") or {}
    p50 = k26on.get("render_wall_p50_ms") or k26on.get("real_render_frame_ms")
    ok = isinstance(p50, (int, float)) and p50 <= BUDGET_MS
    log("judge.k26on_budget", status="OK" if ok else "FAIL", p50=p50, budget=BUDGET_MS)
    if not ok:
        FAILS.append("judge.k26on_budget")
    # dolly 边缘噪声对照(rej vs norej;D-4 消解 measured,登记不判红)。
    dolly = {"rej": noise_of("dolly_rej", DOLLY_ROIS),
             "norej": noise_of("dolly_norej", DOLLY_ROIS)}
    dolly_improve = {}
    if dolly["rej"] and dolly["norej"]:
        for roi in DOLLY_ROIS:
            o = dolly["norej"][roi]["std_p95"]
            n = dolly["rej"][roi]["std_p95"]
            dolly_improve[roi] = round((o - n) / o * 100.0, 2) if o else None
    out = {"schema": "rurix.day0901.g40.t1_ab_matrix.v1",
           "budget_ms": BUDGET_MS, "rois": ROIS, "dolly_rois": DOLLY_ROIS,
           "caliber": "presented u8 尾段 f0064..f0092 恰 8 张(G39 REPORT §六 3b 字面);dark ROI p95 shrink;帧时 = profile render_wall p50;dolly 对照 = rej 缺省(0.10/0.80) vs norej(两拒 0 = G39 v1 语言形)同轨迹同切片",
           "tiers": tiers, "t1c_arms": t1c,
           "dolly_noise": dolly, "dolly_rej_improve_pct": dolly_improve,
           "notes": NOTES}
    (AB / "T1_AB_MATRIX.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    log("judge.matrix", status="OK", tiers=list(tiers.keys()),
        dolly_improve=dolly_improve)


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--only", default="all",
                    help="all 或逗号分隔段名子集(anchors,det,verify,ladder,dolly,storm,judge)")
    a = ap.parse_args()
    all_segs = ("anchors", "det", "verify", "ladder", "dolly", "storm", "judge")
    only = set(all_segs) if a.only == "all" else {s.strip() for s in a.only.split(",")}
    bad = only - set(all_segs)
    if bad:
        print(f"FAIL: 未知段 {sorted(bad)}")
        return 2
    if not WIN.is_file():
        print(f"FAIL: 缺 exe {WIN}")
        return 2
    EV.mkdir(parents=True, exist_ok=True)
    gpu_segs = [s for s in ("anchors", "det", "verify", "ladder", "dolly", "storm")
                if s in only]
    segf = {"anchors": seg_anchors, "det": seg_det, "verify": seg_verify,
            "ladder": seg_ladder, "dolly": seg_dolly, "storm": seg_storm}
    if gpu_segs:
        with gpu_device_lock(purpose="G40 GPU批3(T1 画质补窗验收)", timeout_s=4 * 3600.0):
            for name in gpu_segs:
                log(f"seg.{name}", status="BEGIN")
                n0 = len(FAILS)
                try:
                    segf[name]()
                except Exception as ex:
                    log(f"seg.{name}", status=f"EXC {type(ex).__name__}: {ex}")
                    FAILS.append(f"{name}.exc")
                log(f"seg.{name}", status="END", seg_fails=FAILS[n0:])
    if "judge" in only:
        try:
            seg_judge()
        except Exception as ex:
            log("seg.judge", status=f"EXC {type(ex).__name__}: {ex}")
            FAILS.append("judge.exc")
    (T1 / "B3_SUMMARY.json").write_text(json.dumps({
        "schema": "rurix.day0901.g40.b3.v1",
        "segments": [s for s in all_segs if s in only],
        "fails": FAILS, "notes": NOTES,
        "verdict": "PASS" if not FAILS else "FAIL",
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(("B3 PASS" if not FAILS else f"B3 FAILS: {FAILS}")
          + (f" NOTES: {NOTES}" if NOTES else ""))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
