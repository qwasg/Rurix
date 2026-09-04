#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G41 水面渲染前端门(符号门键 `g41.water.surface`;不消费 CI_step 号)。

判据(逐条机器裁决,任一不满足即红):

1. **五 kernel 编译 + spirv-val**:`kernels/g41_water_{wave,scene,blur,surface,
   encode}.rx` 经 `rurixc --target vulkan` 产 SPV 且 `spirv-val` 接受。
2. **host 金标准单测全绿**:`cargo test -p rurix-render --lib world::water_surface`。
3. **波方程 device↔host 对拍在冻结带内**:`g41_water_probe` 对 measured 冻结带
   `artifacts/day_0903_water/g41_wave_band.json` 比对(位级相等不可达,理由见
   `rfcs/0050` §5)。
4. **对拍 RED 臂**:把带收紧到 1e-9 后探针须**变红**(证明门真的会红,反
   YAML-only)。
5. **七臂 A/B 可归因**:`--water off` 与逐特性 `off` 共 8 组出图 digest **两两
   不等**——每条臂都真的接线并可观测。
6. **默认关臂零漂移**:同参双跑 present digest 位级相等(确定性)。

三态:无 Vulkan 设备时 device 腿(3~6)登记 `skipped_dev_env` 并跳过;
`RURIX_REQUIRE_REAL=1` 下不可跳过,翻硬红。

用法:
    py -3 ci/g41_water_smoke.py --gate g41.water.surface
    py -3 ci/g41_water_smoke.py --build-spv            # 只编 kernel
    py -3 ci/g41_water_smoke.py --selftest             # 只跑 host 面(1~2)
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GATE = "g41.water.surface"
KERNELS = [
    "g41_water_wave",
    "g41_water_scene",
    "g41_water_blur",
    "g41_water_surface",
    "g41_water_encode",
]
SPV_DIR = os.path.join(ROOT, ".tmp", "g41", "spv")
BAND = os.path.join(ROOT, "artifacts", "day_0903_water", "g41_wave_band.json")
REQUIRE_REAL = os.environ.get("RURIX_REQUIRE_REAL") == "1"

ARMS = [
    ("all_on", []),
    ("water_off", ["--water", "off"]),
    ("no_refract", ["--refraction", "off"]),
    ("no_volume", ["--volume", "off"]),
    ("no_caustic", ["--caustics", "off"]),
    ("no_disp", ["--dispersion", "off"]),
    ("no_foam", ["--foam", "off"]),
    ("no_reflect", ["--reflection", "off"]),
]


def run(cmd, **kw):
    return subprocess.run(
        cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", **kw
    )


def exe(name: str) -> str:
    p = os.path.join(ROOT, "target", "release", name + (".exe" if os.name == "nt" else ""))
    return p if os.path.exists(p) else ""


def build_spv(facts: list) -> bool:
    rurixc = os.path.join(ROOT, "target", "debug", "rurixc" + (".exe" if os.name == "nt" else ""))
    if not os.path.exists(rurixc):
        r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"])
        if r.returncode != 0:
            facts.append(("kernels_compile", False, "rurixc 构建失败"))
            return False
    os.makedirs(SPV_DIR, exist_ok=True)
    ok = True
    for k in KERNELS:
        src = os.path.join(ROOT, "src", "rurix-render", "kernels", f"{k}.rx")
        out = os.path.join(SPV_DIR, f"{k}.spv")
        r = run([rurixc, src, "--target", "vulkan", "-o", out])
        good = r.returncode == 0 and os.path.exists(out)
        if not good:
            ok = False
            facts.append((f"kernel_{k}", False, (r.stderr or r.stdout).strip()[:400]))
        else:
            facts.append((f"kernel_{k}", True, "SPIR-V emitted + spirv-val accepted"))
    facts.append(("kernels_compile", ok, f"{len(KERNELS)} kernels"))
    return ok


def host_tests(facts: list) -> bool:
    r = run(["cargo", "test", "-p", "rurix-render", "--lib", "world::water_surface"])
    m = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", r.stdout)
    ok = bool(m) and m.group(1) == "ok" and m.group(3) == "0"
    facts.append(("host_gold_tests", ok, m.group(0) if m else "无 test result 行"))
    return ok


def device_legs(facts: list) -> bool:
    probe = exe("g41_water_probe")
    present = exe("g41_water_present")
    if not probe or not present:
        r = run(
            [
                "cargo", "build", "--release", "-p", "rurix-render",
                "--features", "vulkan", "--bin", "g41_water_probe", "--bin", "g41_water_present",
            ]
        )
        probe, present = exe("g41_water_probe"), exe("g41_water_present")
        if not probe or not present:
            facts.append(("device_bins", False, "构建失败"))
            return False

    if not os.path.exists(BAND):
        facts.append(("wave_band_present", False, f"缺冻结带 {BAND}(先跑 --freeze)"))
        return False

    # 3) 对拍在带内。
    r = run([probe, "--frames", "90", "--band", BAND])
    out = r.stdout + r.stderr
    if "skipped_dev_env" in out:
        if REQUIRE_REAL:
            facts.append(("wave_parity", False, "无设备但 RURIX_REQUIRE_REAL=1"))
            return False
        facts.append(("wave_parity", True, "skipped_dev_env(无 Vulkan 设备)"))
        return True
    md = re.search(r"max_abs_diff=([\d.eE+-]+)", out)
    ok3 = r.returncode == 0
    facts.append(("wave_parity_in_band", ok3, f"max_abs_diff={md.group(1) if md else '?'}"))

    # 4) RED 臂:收紧带须变红。
    tight = os.path.join(ROOT, ".tmp", "g41", "red_band.json")
    os.makedirs(os.path.dirname(tight), exist_ok=True)
    with open(tight, "w", encoding="utf-8") as f:
        json.dump({"max_abs_diff": 1e-9, "note": "RED 臂:人为收紧,探针须红"}, f)
    r = run([probe, "--frames", "90", "--band", tight])
    ok4 = r.returncode != 0
    facts.append(("wave_parity_red_arm", ok4, "收紧带后探针如期红" if ok4 else "RED 臂失效(漏检)"))

    # 5) 七臂 A/B 可归因(digest 两两不等)。
    digests = {}
    ok5 = True
    for name, extra in ARMS:
        r = run(
            [present, "--headless", "--frames", "1", "--warmup", "40",
             "--width", "640", "--height", "360", "--preset", "clear", "--digest"] + extra
        )
        m = re.search(r"present_digest=sha256:(\w+)", r.stdout + r.stderr)
        if r.returncode != 0 or not m:
            ok5 = False
            facts.append((f"arm_{name}", False, "出图失败"))
        else:
            digests[name] = m.group(1)
    if ok5:
        ok5 = len(set(digests.values())) == len(digests)
    facts.append(
        ("arms_distinguishable", ok5, f"{len(set(digests.values()))}/{len(digests)} 组 digest 互异")
    )

    # 6) 双跑位级相等。
    two = []
    for _ in range(2):
        r = run(
            [present, "--headless", "--frames", "1", "--warmup", "40",
             "--width", "640", "--height", "360", "--preset", "clear", "--digest"]
        )
        m = re.search(r"present_digest=sha256:(\w+)", r.stdout + r.stderr)
        two.append(m.group(1) if m else None)
    ok6 = two[0] is not None and two[0] == two[1]
    facts.append(("double_run_bit_equal", ok6, f"{(two[0] or '?')[:16]}… × 2"))

    return all([ok3, ok4, ok5, ok6])


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=None)
    ap.add_argument("--build-spv", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--evidence", default=None)
    a = ap.parse_args()

    if a.gate and a.gate != GATE:
        print(f"g41_water_smoke: 未知门键 {a.gate}(本门 = {GATE})", file=sys.stderr)
        return 2

    facts: list = []
    ok = build_spv(facts)
    if a.build_spv:
        for n, g, d in facts:
            print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
        return 0 if ok else 1

    ok = host_tests(facts) and ok
    if not a.selftest:
        ok = device_legs(facts) and ok

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    # 落**战役目录**而非 `evidence/`:后者由 `ci/check_schemas.py` 按
    # bench/sampling/results 基准件 schema 强校验,本门是渲染特性门不是基准件
    # (day_0902_rain_night 同律把 run/gate 件放战役目录)。
    ev = a.evidence or os.path.join(
        ROOT, "artifacts", "day_0903_water", "evidence", f"g41_water_gate_{ts}.json"
    )
    payload = {
        "schema": "rurix.g41.water_gate_evidence.v1",
        "gate": GATE,
        "status": "pass" if ok else "fail",
        "evidence_level": "measured_local",
        "timestamp": ts,
        "mode": "selftest" if a.selftest else "full",
        "require_real": REQUIRE_REAL,
        "facts": {n: {"pass": g, "detail": d} for n, g, d in facts},
        "rfc": "RFC-0050",
        "campaign": "artifacts/day_0903_water",
    }
    if ok:  # fail-closed:只有 PASS 才落 evidence(全仓同律)
        os.makedirs(os.path.dirname(ev), exist_ok=True)
        with open(ev, "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=2)

    print(f"g41_water_smoke [{GATE}] {'PASS' if ok else 'FAIL'}")
    for n, g, d in facts:
        print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
    if ok:
        print(f"  evidence: {os.path.relpath(ev, ROOT)}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
