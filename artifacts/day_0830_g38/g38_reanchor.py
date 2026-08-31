#!/usr/bin/env python3
"""G38 Wave 3 整批重锚执行器(法线 v2 消费切换 [+ lamp-k 提档若 GO] 后收割)。

前置(主 agent 保证):①窗口 bin L7192 normal_dir 字面已切 baked_normals_bin_v2
[②lamp-k 默认字面若 GO 已提档] ③target-night 与 target 两树 release 均已重建
(blocked_probes 门 device 腿吃 target/release,W4 脚本面吃 target-night——侦察提醒①)。

收割链(锁内,任一步 FAIL 即停):
  n1 all-off 8f  == 55e4a92d(负控:off 面零漂)
  n2 bench 160f  == c1d28ad7(负控:bench 面永不动)
  n3 Stage A 单格探针(bistro t100 tsr 160f 对 stage_a 锚;g14_3 无法线臂,理论零影响)
  h1 full19 96f ×2(target-night)→ 新 full 锚收割(双跑位级)
  h2 orbit 64+10 ×2(target-night)+ ×2(target/release)→ 新 RD-045 锚(双二进制同值)
  v1 ris_nee_solo 96f == 851a61ba(不动抽验:off 基无 normal-maps——v2 只动 nrm 面的证明)
  v2 transparency_solo 96f == af1f7264(同上不动抽验)
产物:G38_ANCHORS.json(W4_ANCHORS 结构照抄)+ reanchor_log.jsonl。
blocked_probes 门 L68 字面回写与门复跑归脚本后主 agent 步。
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(r"H:\rurix")
G38 = ROOT / "artifacts" / "day_0830_g38"
EV = G38 / "reanchor" / "ev"
LOG = G38 / "reanchor" / "reanchor_log.jsonl"
WIN_TN = ROOT / "target-night" / "release" / "g31_window_present.exe"
WIN_REL = ROOT / "target" / "release" / "g31_window_present.exe"
PERF = ROOT / "target-night" / "release" / "g14_3_pipeline_perf.exe"

ANCHOR_ALLOFF = "sha256:55e4a92d25be959a91d111140b88eb81e0c7c29fe80d1aeee641aa78d8a86288"
ANCHOR_BENCH = "c1d28ad73783cc3c054ae0ce372b042a23d01df5491394cb38de7725e83b6c02"
ANCHOR_RIS_SOLO = "sha256:851a61baf989733817bc4880e96ba1ededbea428e22e27842d0f4dc995e2b9b2"
ANCHOR_TRANSP = "sha256:af1f7264"  # 前缀比对(W4_ANCHORS 里有全串,判读时读)
STAGE_A = ROOT / "milestones" / "g14" / "g14_3_stage_a_digest_anchor.json"

sys.path.insert(0, str(ROOT / "ci"))
from gpu_device_lock import gpu_device_lock  # noqa: E402

FAILS: list[str] = []
ANCHORS: dict[str, str] = {}
ROWS: list[dict] = []


def rec(step: str, **kw) -> None:
    row = {"t": time.strftime("%H:%M:%S"), "step": step, **kw}
    ROWS.append(row)
    with LOG.open("a", encoding="utf-8") as f:
        f.write(json.dumps(row, ensure_ascii=False) + "\n")
    print(f"[{row['t']}] {step}: {kw.get('status', '')}", flush=True)


def run_win(step: str, exe: Path, extra: list[str], frames: int,
            warmup: int = 2) -> str | None:
    ev_p = EV / f"{step}.json"
    cmd = [str(exe), "--frames", str(frames), "--warmup", str(warmup), "--hidden",
           *extra, "--evidence", str(ev_p)]
    env = dict(os.environ)
    env.pop("RURIX_G18_AMBIENT", None)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    t0 = time.time()
    p = subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                       encoding="utf-8", errors="replace", timeout=1800, env=env)
    d = None
    vuid = None
    frame_ms = None
    if ev_p.exists():
        e = json.loads(ev_p.read_text(encoding="utf-8"))
        d = e.get("digest")
        vuid = e.get("validation_messages", e.get("vuid"))
        frame_ms = e.get("real_render_frame_ms")
    ok = p.returncode == 0 and d is not None
    rec(step, status="OK" if ok else f"FAIL rc={p.returncode}",
        digest=d, wall_s=round(time.time() - t0, 1), vuid=vuid,
        real_render_frame_ms=frame_ms,
        stderr_tail=None if ok else p.stderr[-800:])
    if not ok:
        FAILS.append(step)
    return d


def pair(tag: str, exe: Path, extra: list[str], frames: int,
         warmup: int = 2) -> str | None:
    d1 = run_win(f"{tag}_r1", exe, extra, frames, warmup)
    d2 = run_win(f"{tag}_r2", exe, extra, frames, warmup)
    ok = d1 is not None and d1 == d2
    rec(f"{tag}_pair", status="OK" if ok else "FAIL", double_run_bitexact=ok, digest=d1)
    if not ok:
        FAILS.append(f"{tag}_pair")
    return d1 if ok else None


def retry_v2() -> int:
    """补验模式:只重跑 transparency 不动抽验(正确臂形)并就地修正 G38_ANCHORS.json。"""
    EV.mkdir(parents=True, exist_ok=True)
    w4 = json.loads((ROOT / "artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json")
                    .read_text(encoding="utf-8"))["anchors_harvested"]
    with gpu_device_lock(purpose="G38 Wave3 v2 抽验补跑(臂形修正)"):
        d = run_win("v2_transp_solo_96f_retry", WIN_TN,
                    ["--quality", "off", "--smooth-normals", "on",
                     "--textures", "on", "--transparency", "on"], 96)
    ok = d == w4["transparency_solo_96f"]
    rec("v2_verdict_retry", status="OK" if ok else "FAIL", got=(d or "")[:24],
        expect=w4["transparency_solo_96f"][:24],
        note="首跑 FAIL = 判读器臂形多拼 --ggx on(W4 s04 原形无 ggx),产物无缺陷;本次为正确臂形补验")
    gp = G38 / "reanchor" / "G38_ANCHORS.json"
    g = json.loads(gp.read_text(encoding="utf-8"))
    if ok:
        g["fails"] = max(0, g["fails"] - 1)
        g["verdict"] = "PASS" if g["fails"] == 0 else "FAIL"
        g["v2_correction"] = ("首跑 v2_verdict FAIL = 判读器臂形错(多 --ggx on,W4 s04 原形 = "
                              "off+smooth+textures+transparency);正确臂形补验 digest == W4 "
                              "af1f7264 全串 MATCH——不动锚抽验成立。W4 s09/G38 批次1 B6/B7 判读修正同先例。")
        g["rows"].extend(ROWS)
        gp.write_text(json.dumps(g, ensure_ascii=False, indent=1), encoding="utf-8")
    print("RETRY_V2 " + ("PASS" if ok else "FAIL"))
    return 0 if ok else 1


def main() -> int:
    if "--retry-v2" in sys.argv:
        return retry_v2()
    EV.mkdir(parents=True, exist_ok=True)
    for exe in (WIN_TN, WIN_REL, PERF):
        if not exe.exists():
            print(f"FAIL: 缺 exe {exe}")
            return 2
    w4 = json.loads((ROOT / "artifacts/day_0830_delivery/w4_flip/W4_ANCHORS.json")
                    .read_text(encoding="utf-8"))["anchors_harvested"]
    with gpu_device_lock(purpose="G38 Wave3 整批重锚收割"):
        # n1 all-off 负控
        d = run_win("n1_alloff_8f", WIN_TN, ["--quality", "off"], 8)
        if d != ANCHOR_ALLOFF:
            rec("n1_verdict", status="FAIL", expect=ANCHOR_ALLOFF[:24], got=(d or "")[:24])
            FAILS.append("n1")
        else:
            rec("n1_verdict", status="OK")
        # n2 bench 负控
        out_root = G38 / "reanchor" / "bench_default"
        p = subprocess.run(
            [str(PERF), "--bench", "--scene", "bistro-interior", "--tier", "100",
             "--backend", "tsr_device", "--frames", "160", "--warmup", "10",
             "--out-root", str(out_root)],
            cwd=str(ROOT), capture_output=True, text=True, encoding="utf-8",
            errors="replace", timeout=1800,
            env={**os.environ, "RURIX_REQUIRE_REAL": "1", "RURIX_VK_VALIDATION": "1"})
        rcp = out_root / "bistro-interior" / "tier100" / "tsr_device" / "bench_receipt.json"
        got = json.loads(rcp.read_text(encoding="utf-8"))["last_frame_digest"] if rcp.exists() else None
        ok = p.returncode == 0 and got is not None and got.endswith(ANCHOR_BENCH)
        rec("n2_bench_160f", status="OK" if ok else "FAIL", got=(got or "")[:24])
        if not ok:
            FAILS.append("n2")
        # n3 Stage A 单格探针(同 bench 命令即该格;直接复用 n2 回执对 stage_a 锚)
        sa = json.loads(STAGE_A.read_text(encoding="utf-8"))["anchors"][
            "bistro-interior_t100_tsr_device"]["last_frame_digest"]
        ok = got is not None and got == sa
        rec("n3_stagea_probe", status="OK" if ok else "FAIL",
            note="bench 默认格 == Stage A bistro/t100/tsr 格同一口径回执")
        if not ok:
            FAILS.append("n3")
        if FAILS:
            rec("reanchor", status="ABORT 负控红,不进收割", fails=FAILS)
            return 1
        # h1 full19 新锚
        d = pair("h1_full19_96f", WIN_TN, [], 96)
        if d:
            ANCHORS["full19_default_96f"] = d
        # h2 RD-045 orbit 双二进制
        dn = pair("h2_rd045_orbit_tn", WIN_TN, ["--auto-move", "orbit"], 64, warmup=10)
        dr = pair("h2_rd045_orbit_rel", WIN_REL, ["--auto-move", "orbit"], 64, warmup=10)
        if dn and dr and dn == dr:
            ANCHORS["rd045_orbit_64f_full_default"] = dn
            rec("h2_dual_binary", status="OK", digest=dn)
        else:
            rec("h2_dual_binary", status="FAIL", tn=(dn or "")[:24], rel=(dr or "")[:24])
            FAILS.append("h2_dual_binary")
        # v1/v2 不动抽验(v2 切换只动 nrm 面的机器证明)
        d = run_win("v1_ris_solo_96f", WIN_TN,
                    ["--quality", "off", "--smooth-normals", "on", "--textures", "on",
                     "--gi2", "on", "--gi2-ris", "on", "--gi2-nee", "on"], 96)
        ok = d == ANCHOR_RIS_SOLO == w4["ris_nee_solo_96f"]
        rec("v1_verdict", status="OK" if ok else "FAIL", got=(d or "")[:24])
        if not ok:
            FAILS.append("v1")
        # W4 s04 原臂形 = off+smooth+textures+transparency(无 --ggx;首版判读多拼
        # 了 --ggx on ⇒ 臂形不同 digest 必异,判读器错非产物错,已修)。
        d = run_win("v2_transp_solo_96f", WIN_TN,
                    ["--quality", "off", "--smooth-normals", "on",
                     "--textures", "on", "--transparency", "on"], 96)
        ok = d == w4["transparency_solo_96f"]
        rec("v2_verdict", status="OK" if ok else "FAIL", got=(d or "")[:24],
            expect=w4["transparency_solo_96f"][:24])
        if not ok:
            FAILS.append("v2")
    # 锚表落盘(W4_ANCHORS 结构照抄;不动锚从 W4 表原值誊)
    out = {
        "schema": "rurix.day0830.g38.reanchor.v1",
        "fails": len(FAILS),
        "optional_fails": [],
        "verdict": "PASS" if not FAILS else "FAIL",
        "anchors_harvested": {
            **ANCHORS,
            "transparency_solo_96f": w4["transparency_solo_96f"],
            "lut_neutral_32f": w4["lut_neutral_32f"],
            "lut_warm_32f": w4["lut_warm_32f"],
            "ris_nee_solo_96f": w4["ris_nee_solo_96f"],
            "tex_arm_orbit_64f": w4["tex_arm_orbit_64f"],
        },
        "unaffected_note": "transparency/lut/ris_nee/tex 臂均 --quality off 基(无 normal-maps"
                           "/lamp-lights),v2 切换零影响——v1/v2 抽验为机器证明;原值自 W4 表誊录。",
        "lineage": {"full19": "5db2e7d7(16臂)→7636f72f(19臂+翻转)→本表(法线v2消费)",
                    "rd045": "060e69a8(作废)→ef2b5b19→本表(法线v2消费)"},
        "rows": ROWS,
    }
    (G38 / "reanchor" / "G38_ANCHORS.json").write_text(
        json.dumps(out, ensure_ascii=False, indent=1), encoding="utf-8")
    print(("REANCHOR PASS " if not FAILS else f"REANCHOR FAILS: {FAILS} ")
          + json.dumps(ANCHORS, ensure_ascii=False))
    return 0 if not FAILS else 1


if __name__ == "__main__":
    sys.exit(main())
