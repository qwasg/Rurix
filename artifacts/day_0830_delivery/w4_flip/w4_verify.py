#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W4:默认翻转后全臂验收 + 锚整批重收割(GPU 锁内,主链任一步失败停跑)。

前置:①窗口 bin 已翻转(--quality 缺省 = full 十九臂)②target-night 全 bin 已重建
③CI 调用面 --quality off 补扫完成。

主链序列:
  s01 alloff        显式 --quality off 8f == 55e4a92d(off 面跨重建稳定锚)
  s02 full_r1/r2    默认(无 --quality)96f 双跑位级 → **新 full 十九臂锚收割**
  s03 bench         bench 默认 160f == c1d28ad7(bench 面永不动)
  s04 transp_r1/r2  transparency 单开双跑位级(装配日志 mat7 130792 tris)
  s05 lut_n/w       LUT neutral/warm 各双跑位级 + n≠w≠off + warm 暖移方向
  s06 ris_r1/r2     RIS+NEE 单开双跑位级 + ≠off(装配日志 灯片表 44024)
  s07 pso_storm     full × storm3 + --pso-report → pso_runtime_creates==0
  s08 visbuffer     独立冒烟 wiring PASS + 窗口臂 sidecar(g36 rxcp 复用)
  s09 framecut      probe 判档 PASS + 窗口臂 + 加性回归(不带旗标 == 对照)
  s10 rd045         orbit 64+10 无 --quality 双跑 → **RD-045 新锚收割**
  s11 texarm        heap tex 臂 orbit 64+10 双跑 → **tex 臂锚收割**(回填判读器)
可选段(失败登记不停跑):
  s12 fgcombo       accept_fg_combo.py --execute
锚收割结果落 W4_ANCHORS.json;字面回写(L63/判读器/HANDOVER)归主线人工步。
"""
from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
BENCH = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"
VBW = ROOT / "target-night" / "release" / "g31_visbuffer_wiring.exe"
FCP = ROOT / "target-night" / "release" / "g31_frame_cut_probe.exe"
W4 = ROOT / "artifacts" / "day_0830_delivery" / "w4_flip"
EV = W4 / "ev"
RXCP = ROOT / ".tmp" / "g36_gates" / "wave1_geo_composition" / "bistro.rxcp"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_BENCH = "sha256:c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"

LOG = open(W4 / "w4_log.jsonl", "a", encoding="utf-8")
RESULTS: list[dict] = []
ANCHORS: dict[str, str | None] = {}
FAILS = 0
OPTIONAL_FAILS: list[str] = []


def env_of() -> dict:
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def rec(row: dict) -> None:
    row["t"] = time.strftime("%H:%M:%S")
    LOG.write(json.dumps(row, ensure_ascii=False) + "\n")
    LOG.flush()
    RESULTS.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)


def run_win(tag: str, extra: list[str], frames: int, warmup: int = 2,
            expect_log: str | None = None) -> tuple[bool, str | None, dict]:
    ev = EV / f"{tag}.json"
    cmd = [str(WIN), "--frames", str(frames), "--warmup", str(warmup), "--hidden",
           *extra, "--evidence", str(ev)]
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       timeout=2400, env=env_of())
    wall = time.time() - t0
    got = None
    evd: dict = {}
    if r.returncode == 0 and ev.is_file():
        evd = json.loads(ev.read_text(encoding="utf-8"))
        got = evd.get("digest")
    vuid = (r.stderr or "").count("VUID-")
    log_ok = True
    if expect_log is not None:
        log_ok = expect_log in (r.stderr or "") or expect_log in (r.stdout or "")
    ok = r.returncode == 0 and vuid == 0 and got is not None and log_ok
    row = {"step": tag, "rc": r.returncode, "digest": got, "vuid": vuid,
           "wall_s": round(wall, 1), "log_ok": log_ok,
           "real_render_frame_ms": evd.get("real_render_frame_ms")}
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-10:]
    return ok, got, row


def pair(tag: str, extra: list[str], frames: int, warmup: int = 2,
         expect_log: str | None = None) -> str | None:
    """双跑位级;返回收割 digest(失败返回 None 并计 FAILS)。"""
    global FAILS
    ok1, d1, row1 = run_win(f"{tag}_r1", extra, frames, warmup, expect_log)
    rec(row1)
    ok2, d2, row2 = run_win(f"{tag}_r2", extra, frames, warmup)
    row2["double_run_bitexact"] = (d1 == d2 and d1 is not None)
    rec(row2)
    if not (ok1 and ok2 and d1 == d2 and d1 is not None):
        FAILS += 1
        rec({"step": f"{tag}_verdict", "pass": False})
        return None
    rec({"step": f"{tag}_verdict", "pass": True, "digest": d1})
    return d1


def raw_channel_means(path: Path) -> tuple[float, float, float]:
    """present raw = 8B(w,h u32)头 + BGRA8;返回 (B,G,R) 均值。"""
    b = path.read_bytes()
    px = b[8:]
    n = len(px) // 4
    sb = sg = sr = 0
    for i in range(0, n * 4, 4):
        sb += px[i]
        sg += px[i + 1]
        sr += px[i + 2]
    return sb / n, sg / n, sr / n


def main() -> int:
    global FAILS
    EV.mkdir(parents=True, exist_ok=True)
    if not RXCP.is_file():
        print(f"缺 RXCP 资产 {RXCP}(g36 门产物)——s08/s09 将失败", flush=True)
    with gpu_device_lock(purpose="G37 W4 flip verify + reanchor", timeout_s=14400.0):
        # s01 all-off 显式回退档
        ok, got, row = run_win("s01_alloff", ["--quality", "off"], 8)
        row["expect"] = ANCHOR_ALLOFF
        row["pass"] = ok and got == ANCHOR_ALLOFF
        FAILS += 0 if row["pass"] else 1
        rec(row)
        # s02 full 默认新锚(无 --quality 参数 = 翻转后默认)
        if FAILS == 0:
            d = pair("s02_full19", [], 96)
            ANCHORS["full19_default_96f"] = d
        # s03 bench 面不动
        if FAILS == 0:
            t0 = time.time()
            rp = subprocess.run(
                [str(BENCH), "--bench", "--scene", "bistro-interior", "--tier", "100",
                 "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
                 "--out-root", str(W4 / "bench_default")],
                cwd=ROOT, capture_output=True, text=True, timeout=1800, env=env_of())
            receipt = (W4 / "bench_default" / "bistro-interior" / "tier100"
                       / "tsr_device" / "bench_receipt.json")
            got = None
            if rp.returncode == 0 and receipt.is_file():
                got = json.loads(receipt.read_text(encoding="utf-8")).get("last_frame_digest")
            row = {"step": "s03_bench", "rc": rp.returncode, "digest": got,
                   "expect": ANCHOR_BENCH, "wall_s": round(time.time() - t0, 1),
                   "pass": got == ANCHOR_BENCH}
            FAILS += 0 if row["pass"] else 1
            rec(row)
        # s04 transparency 单开
        if FAILS == 0:
            d = pair("s04_transp",
                     ["--quality", "off", "--smooth-normals", "on", "--textures", "on",
                      "--transparency", "on"], 96,
                     expect_log="TransparentGlass")
            ANCHORS["transparency_solo_96f"] = d
        # s05 LUT neutral / warm
        if FAILS == 0:
            dn = pair("s05_lut_neutral", ["--quality", "off", "--lut", "neutral"], 32)
            ANCHORS["lut_neutral_32f"] = dn
            dw_raw = EV / "s05_lut_warm.raw"
            dw = pair("s05_lut_warm",
                      ["--quality", "off", "--lut", "warm",
                       "--dump-present-raw", str(dw_raw), "--dump-present-every", "31"], 32)
            ANCHORS["lut_warm_32f"] = dw
            doff_raw = EV / "s05_lut_off.raw"
            ok, doff, row = run_win("s05_lut_off",
                                    ["--quality", "off",
                                     "--dump-present-raw", str(doff_raw),
                                     "--dump-present-every", "31"], 32)
            rec(row)
            warm_shift = None
            if dw_raw.is_file() and doff_raw.is_file():
                bw, _, rw = raw_channel_means(dw_raw)
                bo, _, ro = raw_channel_means(doff_raw)
                warm_shift = (rw - ro) > 0 and (bw - bo) < 0
            distinct = dn is not None and dw is not None and dn != dw and dw != doff
            row = {"step": "s05_verdict", "distinct": distinct,
                   "warm_shift_r_up_b_down": warm_shift,
                   "pass": bool(distinct and warm_shift)}
            FAILS += 0 if row["pass"] else 1
            rec(row)
        # s06 RIS + NEE 单开
        if FAILS == 0:
            d = pair("s06_ris_nee",
                     ["--quality", "off", "--smooth-normals", "on", "--textures", "on",
                      "--gi2", "on", "--gi2-ris", "on", "--gi2-nee", "on"], 96,
                     expect_log="44024")
            ANCHORS["ris_nee_solo_96f"] = d
            # ≠ 同参数无 ris/nee 对照
            ok, dbase, row = run_win("s06_base",
                                     ["--quality", "off", "--smooth-normals", "on",
                                      "--textures", "on", "--gi2", "on"], 96)
            rec(row)
            row = {"step": "s06_verdict", "on_neq_off": d is not None and d != dbase,
                   "pass": bool(d is not None and dbase is not None and d != dbase)}
            FAILS += 0 if row["pass"] else 1
            rec(row)
        # s07 full × storm3 + PSO 账本
        if FAILS == 0:
            pso_rep = EV / "s07_pso_report.json"
            ok, got, row = run_win("s07_full_storm3",
                                   ["--window-storm", "3", "--pso-report", str(pso_rep)],
                                   30)
            prc = None
            sessions = None
            if pso_rep.is_file():
                pd = json.loads(pso_rep.read_text(encoding="utf-8"))
                prc = pd.get("pso_runtime_creates")
                sessions = pd.get("sessions")
            row["pso_runtime_creates"] = prc
            row["sessions"] = sessions
            row["pass"] = ok and prc == 0
            FAILS += 0 if row["pass"] else 1
            rec(row)
        # s08 VisBuffer:独立冒烟 + 窗口臂
        if FAILS == 0:
            t0 = time.time()
            r = subprocess.run([str(VBW), "--cluster-pack", str(RXCP),
                                "--error-px", "2.0", "--samples", "3",
                                "--evidence", str(EV / "s08_vbw.json")],
                               cwd=ROOT, capture_output=True, text=True,
                               timeout=1800, env=env_of())
            row = {"step": "s08_vbw_smoke", "rc": r.returncode,
                   "wall_s": round(time.time() - t0, 1), "pass": r.returncode == 0}
            if r.returncode != 0:
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-10:]
            FAILS += 0 if row["pass"] else 1
            rec(row)
        if FAILS == 0:
            vb_out = EV / "s08_vb_sidecar.json"
            ok, got, row = run_win("s08_vb_window",
                                   ["--quality", "off", "--headless-smoke",
                                    "--auto-move", "dolly", "--tier", "100",
                                    "--cluster-lod", "on", "--cluster-error-px", "2.0",
                                    "--cluster-pack", str(RXCP),
                                    "--visbuffer", "on", "--visbuffer-out", str(vb_out)],
                                   24)
            row["sidecar"] = vb_out.is_file()
            row["pass"] = ok and vb_out.is_file()
            FAILS += 0 if row["pass"] else 1
            rec(row)
        # s09 frame-cut:probe 判档 + 窗口臂 + 加性回归
        if FAILS == 0:
            t0 = time.time()
            r = subprocess.run([str(FCP), "--cluster-pack", str(RXCP),
                                "--error-px", "2.0", "--frames", "16",
                                "--step-m", "0.15", "--res", "96x54",
                                "--evidence", str(EV / "s09_fcp.json")],
                               cwd=ROOT, capture_output=True, text=True,
                               timeout=2400, env=env_of())
            ok_ev = False
            evp = EV / "s09_fcp.json"
            if evp.is_file():
                ok_ev = bool(json.loads(evp.read_text(encoding="utf-8")).get("pass"))
            row = {"step": "s09_fcp_probe", "rc": r.returncode, "evidence_pass": ok_ev,
                   "wall_s": round(time.time() - t0, 1),
                   "pass": r.returncode == 0 and ok_ev}
            if not row["pass"]:
                row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-10:]
            FAILS += 0 if row["pass"] else 1
            rec(row)
        if FAILS == 0:
            fc_out = EV / "s09_fc_sidecar.json"
            base_args = ["--quality", "off", "--headless-smoke", "--auto-move", "dolly",
                         "--tier", "100", "--cluster-lod", "on",
                         "--cluster-error-px", "2.0", "--cluster-pack", str(RXCP)]
            ok1, d_on, row1 = run_win("s09_fc_window",
                                      base_args + ["--cluster-per-frame-cut", "on",
                                                   "--frame-cut-out", str(fc_out)], 24)
            row1["sidecar"] = fc_out.is_file()
            rec(row1)
            ok2, d_base, row2 = run_win("s09_fc_base", base_args, 24)
            row2["additive_regression"] = (d_on == d_base and d_on is not None)
            rec(row2)
            row = {"step": "s09_verdict",
                   "pass": bool(ok1 and ok2 and fc_out.is_file() and d_on == d_base)}
            FAILS += 0 if row["pass"] else 1
            rec(row)
        # s10 RD-045 P02 腿新锚(默认臂语义 = 翻转后 full)
        if FAILS == 0:
            d = pair("s10_rd045_orbit", ["--auto-move", "orbit"], 64, warmup=10)
            ANCHORS["rd045_orbit_64f_full_default"] = d
        # s11 heap tex 臂锚收割(判读器 PENDING_W4_REHARVEST 回填值)
        if FAILS == 0:
            d = pair("s11_texarm",
                     ["--quality", "off", "--auto-move", "orbit", "--textures", "on",
                      "--spv-texture", ".tmp/night_0828/spv/g31_texture_gi_v2.spv",
                      "--spv-texture-probe", ".tmp/night_0828/spv/g31_texture_probe_v2.spv"],
                     64, warmup=10)
            ANCHORS["tex_arm_orbit_64f"] = d
    # s12 可选:fg combo 验收(自管 GPU 锁)
    if FAILS == 0:
        t0 = time.time()
        r = subprocess.run([sys.executable,
                            str(ROOT / "artifacts/day_0830_delivery/w3_deep/fg_combo/accept_fg_combo.py"),
                            "--execute"],
                           cwd=ROOT, capture_output=True, text=True,
                           timeout=7200, env=env_of())
        row = {"step": "s12_fgcombo", "rc": r.returncode,
               "wall_s": round(time.time() - t0, 1), "pass": r.returncode == 0,
               "optional": True}
        if r.returncode != 0:
            row["tail"] = ((r.stdout or "") + (r.stderr or "")).strip().splitlines()[-12:]
            OPTIONAL_FAILS.append("s12_fgcombo")
        rec(row)

    doc = {
        "schema": "rurix.day0830.delivery.w4_flip_verify.v1",
        "fails": FAILS,
        "optional_fails": OPTIONAL_FAILS,
        "verdict": "PASS" if FAILS == 0 else "FAIL",
        "anchors_harvested": ANCHORS,
        "notes": "锚字面回写(blocked_probes L63/texsampling 判读器/HANDOVER)归主线人工步;"
                 "s12 可选段失败不阻塞主链(独立处置)。",
        "rows": RESULTS,
    }
    (W4 / "W4_ANCHORS.json").write_text(
        json.dumps(doc, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print("W4", json.dumps({"fails": FAILS, "optional_fails": OPTIONAL_FAILS,
                            "anchors": ANCHORS}, ensure_ascii=False))
    return 0 if FAILS == 0 else 1


if __name__ == "__main__":
    sys.exit(main())
