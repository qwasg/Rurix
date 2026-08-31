#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G38 T5:lamp-k 阶梯跑批(六档,主 agent 批次 2 GPU 锁内执行)。

背景(EVAL_RESTIR §9.4「改参数不改算法」):
  --lamp-k 已在(默认 12,须随 --lamp-lights on;--lamp-k 不进 full dup 表
  = 可与 --quality full 组合微调),但聚类网格 GRID_M=0.6 为 lane_body
  extract_lamp_lights 内 const(bistro 44,024 emissive 三角在 0.6m 网格
  26 邻域 union-find 下只产 13 簇,keep_n=min(len,max_k) ⇒ 现网格下
  --lamp-k 24/48 无效)。网格旋钮接口约定(主 agent 稍后接线,本脚本按
  接口写):环境变量 RURIX_G31_LAMP_GRID_M(缺席=0.6 字面;在位 parse f32
  非法即 fail——RURIX_G18_AMBIENT 同律先例)。

档位(散臂 env+显式 --lamp-k,不动任何在案锚;改默认字面归 Wave3 GO 后):
  1. grid 0.6 / k 12   基线 = 零画质参数缺省(W4 翻转后缺省=full19,
                        s02 锚位 7636f72f;不设 env 不传 --lamp-k)
  2. grid 0.6 / k 24   证伪档:预期簇仍 13、kept 不变(网格不收细则 k 无效)
  3. grid 0.3 / k 24
  4. grid 0.3 / k 48
  5. grid 0.15 / k 48
  6. grid 0.15 / k 96
每档单跑(阶梯是量测不是锚;双跑自证留给定档后的 GO 档位)。

采集面:
  - stderr 灯簇统计行(lane_body apply_lamp_lights eprintln 字面:
    "[g14_3_pipeline_perf]: lamp-lights 提取 emissive_tris=… clusters=…
     kept=… dropped=…"),正则抓 clusters_total/kept/dropped。
  - evidence(real_render_frame_ms mean + stats.render_min/max_ms + digest)。
  - --profile-json(C7 面,默认关零语义变更;frame_segments[render_wall]
    的 p50_ms/max_ms 供 judge_kladder2 预算判读)。

用法:
  py -3 run_kladder.py             # 真跑(GPU 锁外部由主 agent 持有)
  py -3 run_kladder.py --dry-run   # 打印命令与 env 不执行
  py -3 run_kladder.py --selftest  # 零 GPU:伪 stderr/evidence/profile 走通解析链
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

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
WIN = ROOT / "target-night" / "release" / "g31_window_present.exe"
KL_DIR = HERE / "kladder"

FRAMES = 96      # 与 full19 s02 锚同口径(9.75/10.59ms 参考值即此口径)
WARMUP = 2
GRID_ENV = "RURIX_G31_LAMP_GRID_M"   # 旋钮接口约定(缺席=0.6 字面)

# 六档字面(tag 进目录名;grid=None ⇒ env 缺席,k=None ⇒ 不传 --lamp-k)。
LADDER: list[dict] = [
    {"tag": "s1_g060_k12_baseline", "grid": None,   "k": None},
    {"tag": "s2_g060_k24_falsify",  "grid": "0.6",  "k": 24},
    {"tag": "s3_g030_k24",          "grid": "0.3",  "k": 24},
    {"tag": "s4_g030_k48",          "grid": "0.3",  "k": 48},
    {"tag": "s5_g015_k48",          "grid": "0.15", "k": 48},
    {"tag": "s6_g015_k96",          "grid": "0.15", "k": 96},
]

# 灯簇统计行正则(lane_body eprintln 字面;弃簇通量段为全角括号,只锚定
# 前四个 key=value 段,对尾段字面变化稳健)。
LAMP_RE = re.compile(
    r"lamp-lights 提取 emissive_tris=(\d+) clusters=(\d+) kept=(\d+) dropped=(\d+)")


def env_of(grid: str | None) -> dict:
    """阶梯环境注入:REQUIRE_REAL/VALIDATION 恒注;网格旋钮 pop 后条件 set
    (RURIX_G18_AMBIENT 恒 pop 不注——full 预设 OnceLock 自供 0.004,
    显式 env 面不进基线锚位口径)。"""
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    env.pop("RURIX_G18_AMBIENT", None)
    env.pop(GRID_ENV, None)
    if grid is not None:
        env[GRID_ENV] = grid
    return env


def cmd_of(step: dict, run_dir: Path) -> list[str]:
    """单档命令:基线 = 零画质参数(缺省 full19);其余显式 --quality full
    + --lamp-k(不进 dup 表,与 full 可组合)。"""
    cmd = [str(WIN), "--frames", str(FRAMES), "--warmup", str(WARMUP), "--hidden"]
    if step["k"] is not None:
        cmd += ["--quality", "full", "--lamp-k", str(step["k"])]
    cmd += ["--evidence", str(run_dir / "ev.json"),
            "--profile-json", str(run_dir / "prof.json")]
    return cmd


def parse_lamp_line(stderr: str) -> dict:
    """从 stderr 抓灯簇统计(缺行如实登记 None,不冒充)。"""
    m = LAMP_RE.search(stderr or "")
    if not m:
        return {"emissive_tris": None, "clusters_total": None,
                "kept": None, "dropped": None, "lamp_line": None}
    line = next((ln for ln in stderr.splitlines() if "lamp-lights 提取" in ln), None)
    return {"emissive_tris": int(m.group(1)), "clusters_total": int(m.group(2)),
            "kept": int(m.group(3)), "dropped": int(m.group(4)),
            "lamp_line": line}


def run_step(step: dict, rows: list[dict]) -> dict:
    """单档执行 + 三面采集(stderr 灯簇行/evidence/profile 落盘)。"""
    run_dir = KL_DIR / step["tag"]
    run_dir.mkdir(parents=True, exist_ok=True)
    cmd = cmd_of(step, run_dir)
    t0 = time.time()
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True,
                       encoding="utf-8", errors="replace",
                       timeout=1800, env=env_of(step["grid"]))
    wall = time.time() - t0
    # stderr 全文落盘(灯簇行证据面 + 事后追溯)。
    (run_dir / "stderr.txt").write_text(r.stderr or "", encoding="utf-8")
    ev_p = run_dir / "ev.json"
    evd: dict = {}
    digest = None
    if r.returncode == 0 and ev_p.is_file():
        evd = json.loads(ev_p.read_text(encoding="utf-8"))
        digest = evd.get("digest")
    vuid = (r.stderr or "").count("VUID-")
    ok = r.returncode == 0 and vuid == 0 and digest is not None
    row = {
        "tag": step["tag"],
        "grid_env": step["grid"],        # None = env 缺席(0.6 字面)
        "k_req": step["k"],              # None = 不传(默认 12)
        "cmd": subprocess.list2cmdline(cmd),
        "rc": r.returncode,
        "vuid": vuid,
        "digest": digest,
        "real_render_frame_ms": evd.get("real_render_frame_ms"),
        "render_min_ms": (evd.get("stats") or {}).get("render_min_ms"),
        "render_max_ms": (evd.get("stats") or {}).get("render_max_ms"),
        **parse_lamp_line(r.stderr),
        "profile_json": str(run_dir / "prof.json"),
        "wall_s": round(wall, 1),
        "ok": ok,
    }
    if not ok:
        row["stderr_tail"] = (r.stderr or "").strip().splitlines()[-8:]
        row["stdout_tail"] = (r.stdout or "").strip().splitlines()[-4:]
    rows.append(row)
    print(json.dumps(row, ensure_ascii=False), flush=True)
    return row


def do_runs() -> int:
    """六档序贯单跑(任一档 fail 即停,fail-closed)。"""
    if not WIN.is_file():
        raise SystemExit(f"FAIL: 窗口 bin 不存在 {WIN}(先建 target-night release)")
    rows: list[dict] = []
    fails = 0
    for step in LADDER:
        row = run_step(step, rows)
        if not row["ok"]:
            fails += 1
            break
    out = KL_DIR / "kladder_runs.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps({
        "schema": "rurix.day0830.g38.t5.run_kladder.v1",
        "fails": fails,
        "frames": FRAMES, "warmup": WARMUP,
        "grid_env_name": GRID_ENV,
        "grid_env_note": "缺席=0.6 字面;在位 parse f32 非法即 fail"
                         "(接口约定,主 agent 接线;RURIX_G18_AMBIENT 同律)",
        "ladder": LADDER,
        "rows": rows,
    }, ensure_ascii=False, indent=1) + "\n", encoding="utf-8")
    print(f"RUN_KLADDER {'PASS' if fails == 0 else 'FAIL'} fails={fails} → {out}")
    return 0 if fails == 0 else 1


def do_dry_run() -> int:
    """打印六档命令与 env 增量字面,零执行。"""
    print("# env 恒注: RURIX_REQUIRE_REAL=1 RURIX_VK_VALIDATION=1"
          "(RURIX_G18_AMBIENT 恒缺席=预设自供 0.004)")
    for i, step in enumerate(LADDER, 1):
        g = f"{GRID_ENV}={step['grid']}" if step["grid"] else f"{GRID_ENV} 缺席(0.6)"
        print(f"[{i}] ({g}) "
              + subprocess.list2cmdline(cmd_of(step, KL_DIR / step["tag"])))
    print(f"# 共 {len(LADDER)} 档单跑;每档 {FRAMES}f/warmup{WARMUP}")
    return 0


def do_selftest() -> int:
    """零 GPU 自测:伪 stderr(真 eprintln 字面含全角括号)走正则;
    伪 evidence/profile 落盘走 row 组装;命令/env 构造断言。"""
    fake_stderr = (
        "[g14_3_pipeline_perf]: lamp-lights 提取 emissive_tris=44024 "
        "clusters=13 kept=12 dropped=1（弃簇通量峰 0.001234/弃三角 7）"
        "gain=4 k=12 contrib=0\n其他行\n")
    got = parse_lamp_line(fake_stderr)
    assert got["emissive_tris"] == 44024 and got["clusters_total"] == 13
    assert got["kept"] == 12 and got["dropped"] == 1
    assert "lamp-lights 提取" in got["lamp_line"]
    # 缺行如实 None(不冒充)。
    none = parse_lamp_line("无关输出")
    assert none["clusters_total"] is None
    # 命令构造:基线零画质参数;非基线含 --quality full --lamp-k。
    c0 = cmd_of(LADDER[0], KL_DIR / "x")
    assert "--quality" not in c0 and "--lamp-k" not in c0, "基线应零画质参数"
    assert "--profile-json" in c0 and "--evidence" in c0
    c3 = cmd_of(LADDER[3], KL_DIR / "x")
    assert c3[c3.index("--lamp-k") + 1] == "48"
    assert c3[c3.index("--quality") + 1] == "full"
    # env 构造:基线缺席旋钮;档位在位字面;恒注面在。
    e0 = env_of(None)
    assert GRID_ENV not in e0 and e0["RURIX_REQUIRE_REAL"] == "1"
    assert e0["RURIX_VK_VALIDATION"] == "1" and "RURIX_G18_AMBIENT" not in e0
    e5 = env_of("0.15")
    assert e5[GRID_ENV] == "0.15"
    print(json.dumps({
        "selftest": "run_kladder", "pass": True,
        "lamp_parse": {k: got[k] for k in
                       ("emissive_tris", "clusters_total", "kept", "dropped")},
        "ladder_steps": len(LADDER),
    }, ensure_ascii=False))
    return 0


def main() -> int:
    if hasattr(sys.stdout, "reconfigure"):
        sys.stdout.reconfigure(encoding="utf-8")
    ap = argparse.ArgumentParser(description=__doc__)
    g = ap.add_mutually_exclusive_group()
    g.add_argument("--dry-run", action="store_true", help="打印命令不执行")
    g.add_argument("--selftest", action="store_true", help="伪数据走通解析链(零 GPU)")
    args = ap.parse_args()
    if args.selftest:
        return do_selftest()
    if args.dry_run:
        return do_dry_run()
    return do_runs()


if __name__ == "__main__":
    sys.exit(main())
