#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G40 体积云展示车道门(符号门键 `g40.clouds.present`;不消费 CI_step 号)。

判据(逐条机器裁决,任一不满足即红):

1. **两 kernel 编译 + spirv-val**:`kernels/g40_{volumetric_cloud,cloud_encode}.rx`
   经 `rurixc --target vulkan` 产 SPV 且 `spirv-val` **真调**接受(g41 门只声称
   不调用,本门补齐;调用形态同 `ci/g35_render_wiring_smoke.py`)。
2. **host 金标准单测全绿**:`cargo test -p rurix-render --lib world::clouds`
   与 `world::sky` 两组。
3. **五臂可归因**:`--preset {noon,clear,golden,sunset}` 四天空档 + `--preset
   clear --phi-fwd off` 关臂,共 5 组出图 digest **两两不等**——每条臂都真的
   接线并可观测(反「旋钮不接线」)。
4. **默认臂零漂移**:`--preset clear` 同参双跑 digest 位级相等(确定性)。
5. **RED 臂**:把 `g40_volumetric_cloud.rx` **复制**到 `.tmp/g40/red/` 后注入
   语法错,rurixc 须**拒**(rc ≠ 0)——证明门真的会红(反 YAML-only)。
   只动临时副本,树内 kernel 0-byte。

三态:无 Vulkan 设备时 device 腿(3~4)登记 `skipped_dev_env` 并跳过;
`RURIX_REQUIRE_REAL=1` 下不可跳过,翻硬红。device 腿全程持
`gpu_device_lock`(单卡蜂群纪律:并行 device 提交会互相污染 measured 数字)。

用法:
    py -3 ci/g40_cloud_smoke.py --gate g40.clouds.present
    py -3 ci/g40_cloud_smoke.py --build-spv            # 只编 kernel
    py -3 ci/g40_cloud_smoke.py --selftest             # 只跑 host 面(1、2、5)
"""

from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE = "g40.clouds.present"
KERNELS = [
    "g40_volumetric_cloud",
    "g40_cloud_encode",
]
# bin 的内建默认即此目录;门仍显式传 `--spv-cloud/--spv-encode`(自持性:
# 门不吃 bin 的默认值,默认值漂移不会静默改判)。
SPV_DIR = os.path.join(ROOT, ".tmp", "g40", "spv")
RED_DIR = os.path.join(ROOT, ".tmp", "g40", "red")
KERNEL_DIR = os.path.join(ROOT, "src", "rurix-render", "kernels")
REQUIRE_REAL = os.environ.get("RURIX_REQUIRE_REAL") == "1"

# host 金标准单测的两个过滤面(world::clouds = 云物理,world::sky = 解析天空)。
TEST_FILTERS = ["world::clouds", "world::sky"]

# 出图臂:四天空档 + phi_fwd 关臂。digest 须两两不等。
ARMS = [
    ("preset_noon", ["--preset", "noon"]),
    ("preset_clear", ["--preset", "clear"]),
    ("preset_golden", ["--preset", "golden"]),
    ("preset_sunset", ["--preset", "sunset"]),
    ("clear_phifwd_off", ["--preset", "clear", "--phi-fwd", "off"]),
]
# 默认臂(双跑位级相等腿)。
DEFAULT_ARM = ["--preset", "clear"]
# headless 出图构型(640×360 单帧)。bin **无** `--warmup`(参见
# `src/rurix-render/src/bin/g40_cloud_present.rs` 的 parse_args 闭集),
# 未知参数会被 bin 直接 fail,故此处一个不多传。
HEADLESS = ["--headless", "--frames", "1", "--width", "640", "--height", "360", "--digest"]

DIGEST_RE = re.compile(r"digest=sha256:([0-9a-f]{64})")
# RED 臂注入体:未闭合形参表 + 括号错配,任何解析器都须拒。
RED_INJECT = "\n\nfn red_arm_injected_syntax_error( { ) }\n"


def run(cmd, **kw):
    return subprocess.run(
        cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8", errors="replace", **kw
    )


def exe(name: str) -> str:
    p = os.path.join(ROOT, "target", "release", name + (".exe" if os.name == "nt" else ""))
    return p if os.path.exists(p) else ""


def rurixc_path() -> str:
    return os.path.join(ROOT, "target", "debug", "rurixc" + (".exe" if os.name == "nt" else ""))


def spv_args() -> list:
    """显式点名两件 SPV(与 bin 内建默认同路径,但门不吃默认值)。"""
    return [
        "--spv-cloud", os.path.join(SPV_DIR, "g40_volumetric_cloud.spv"),
        "--spv-encode", os.path.join(SPV_DIR, "g40_cloud_encode.spv"),
    ]


def spirv_val(out: str) -> tuple:
    """真调 spirv-val(缺工具 = fail-closed,不静默放行)。"""
    try:
        v = run(["spirv-val", out])
    except FileNotFoundError:
        return False, "spirv-val 不在 PATH(fail-closed:不静默放行)"
    if v.returncode != 0:
        return False, (v.stderr or v.stdout).strip()[:400]
    return True, ""


def build_spv(facts: list) -> bool:
    rurixc = rurixc_path()
    if not os.path.exists(rurixc):
        r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"])
        if r.returncode != 0:
            facts.append(("kernels_compile", False, "rurixc 构建失败"))
            return False
    os.makedirs(SPV_DIR, exist_ok=True)
    ok = True
    passed = 0
    for k in KERNELS:
        src = os.path.join(KERNEL_DIR, f"{k}.rx")
        out = os.path.join(SPV_DIR, f"{k}.spv")
        r = run([rurixc, src, "--target", "vulkan", "-o", out])
        if r.returncode != 0 or not os.path.exists(out):
            ok = False
            facts.append((f"kernel_{k}", False,
                          "rurixc 编译失败: " + (r.stderr or r.stdout).strip()[:400]))
            continue
        good, why = spirv_val(out)
        if not good:
            ok = False
            facts.append((f"kernel_{k}", False, f"spirv-val 未过: {why}"))
        else:
            passed += 1
            facts.append((f"kernel_{k}", True, "SPIR-V emitted + spirv-val accepted"))
    facts.append(("kernels_compile", ok, f"{passed}/{len(KERNELS)} kernels"))
    return ok


def host_tests(facts: list) -> bool:
    """host 金标准两组过滤面(world::clouds + world::sky)全绿。"""
    ok = True
    detail = []
    for filt in TEST_FILTERS:
        r = run(["cargo", "test", "-p", "rurix-render", "--lib", filt])
        m = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", r.stdout)
        good = bool(m) and m.group(1) == "ok" and m.group(3) == "0"
        ok = ok and good
        detail.append(f"{filt}: {m.group(0) if m else '无 test result 行'}")
    facts.append(("host_gold_tests", ok, "; ".join(detail)))
    return ok


def red_arm(facts: list) -> bool:
    """RED 臂:临时副本注入语法错,rurixc 须拒。树内 kernel **0-byte 不动**。"""
    rurixc = rurixc_path()
    if not os.path.exists(rurixc):
        facts.append(("red_arm_kernel_syntax", False, "rurixc 不在位(先跑 --build-spv)"))
        return False
    os.makedirs(RED_DIR, exist_ok=True)
    src = os.path.join(KERNEL_DIR, "g40_volumetric_cloud.rx")
    red_src = os.path.join(RED_DIR, "g40_volumetric_cloud_red.rx")
    red_out = os.path.join(RED_DIR, "g40_volumetric_cloud_red.spv")
    # 复制 → 追加注入(只读原文件,绝不回写树内 kernel)。
    shutil.copyfile(src, red_src)
    with open(red_src, "a", encoding="utf-8") as f:
        f.write(RED_INJECT)
    if os.path.exists(red_out):
        os.remove(red_out)
    r = run([rurixc, red_src, "--target", "vulkan", "-o", red_out])
    ok = r.returncode != 0 and not os.path.exists(red_out)
    facts.append((
        "red_arm_kernel_syntax", ok,
        f"注入语法错后 rurixc rc={r.returncode}"
        + ("(如期红)" if ok else "(RED 臂失效:门漏检,语法错竟被接受)"),
    ))
    return ok


def arm_digest(present: str, extra: list) -> tuple:
    """跑一条 headless 臂,返回 (digest|None, 是否 skipped_dev_env, 合并输出)。"""
    r = run([present] + HEADLESS + spv_args() + extra)
    out = (r.stdout or "") + (r.stderr or "")
    if "skipped_dev_env" in out:
        return None, True, out
    m = DIGEST_RE.search(out)
    if r.returncode != 0 or not m:
        return None, False, out
    return m.group(1), False, out


def device_legs(facts: list) -> bool:
    present = exe("g40_cloud_present")
    if not present:
        r = run([
            "cargo", "build", "--release", "-p", "rurix-render",
            "--features", "vulkan", "--bin", "g40_cloud_present",
        ])
        present = exe("g40_cloud_present")
        if not present:
            facts.append(("device_bins", False,
                          "构建失败: " + (r.stderr or r.stdout).strip()[:400]))
            return False

    ok3 = True
    ok4 = False
    # 单卡蜂群纪律:全部 device 腿在一把锁内串行(G35/G36 门同律;并行
    # device 提交会互相污染 measured 数字与 digest)。
    with gpu_device_lock(purpose="g40_cloud_smoke 五臂出图 + 默认臂双跑 device 真跑"):
        # 3) 五臂 digest 两两不等。
        digests = {}
        for name, extra in ARMS:
            d, skipped, out = arm_digest(present, extra)
            if skipped:
                if REQUIRE_REAL:
                    facts.append(("arms_distinguishable", False, "无设备但 RURIX_REQUIRE_REAL=1"))
                    facts.append(("double_run_bit_equal", False, "无设备但 RURIX_REQUIRE_REAL=1"))
                    return False
                facts.append(("arms_distinguishable", True, "skipped_dev_env(无 Vulkan 设备)"))
                facts.append(("double_run_bit_equal", True, "skipped_dev_env(无 Vulkan 设备)"))
                return True
            if d is None:
                ok3 = False
                facts.append((f"arm_{name}", False, f"出图失败: {out.strip()[-300:]}"))
            else:
                digests[name] = d
        if ok3:
            ok3 = len(set(digests.values())) == len(digests) == len(ARMS)
        facts.append((
            "arms_distinguishable", ok3,
            f"{len(set(digests.values()))}/{len(ARMS)} 组 digest 互异"
            f"(四天空档 + phi_fwd 关臂)",
        ))

        # 4) 默认臂(--preset clear)双跑位级相等。
        two = []
        for _ in range(2):
            d, _skipped, _out = arm_digest(present, DEFAULT_ARM)
            two.append(d)
        ok4 = two[0] is not None and two[0] == two[1]
        facts.append(("double_run_bit_equal", ok4, f"{(two[0] or '?')[:16]}… × 2"))

    return ok3 and ok4


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=None)
    ap.add_argument("--build-spv", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--evidence", default=None)
    a = ap.parse_args()

    if a.gate and a.gate != GATE:
        print(f"g40_cloud_smoke: 未知门键 {a.gate}(本门 = {GATE})", file=sys.stderr)
        return 2

    facts: list = []
    ok = build_spv(facts)
    if a.build_spv:
        for n, g, d in facts:
            print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
        return 0 if ok else 1

    ok = red_arm(facts) and ok
    ok = host_tests(facts) and ok
    if not a.selftest:
        ok = device_legs(facts) and ok

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    # 落**战役目录**而非 `evidence/`:后者由 `ci/check_schemas.py` 按
    # bench/sampling/results 基准件 schema 强校验,本门是渲染特性门不是基准件
    # (day_0902_rain_night / day_0903_water 同律把 run/gate 件放战役目录)。
    ev = a.evidence or os.path.join(
        ROOT, "artifacts", "day_0903_clouds", "evidence", f"g40_cloud_gate_{ts}.json"
    )
    payload = {
        "schema": "rurix.g40.cloud_gate_evidence.v1",
        "gate": GATE,
        "status": "pass" if ok else "fail",
        "evidence_level": "measured_local",
        "timestamp": ts,
        "mode": "selftest" if a.selftest else "full",
        "require_real": REQUIRE_REAL,
        "facts": {n: {"pass": g, "detail": d} for n, g, d in facts},
        "campaign": "artifacts/day_0903_clouds",
    }
    if ok:  # fail-closed:只有 PASS 才落 evidence(全仓同律)
        os.makedirs(os.path.dirname(ev), exist_ok=True)
        with open(ev, "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=2)

    print(f"g40_cloud_smoke [{GATE}] {'PASS' if ok else 'FAIL'}")
    for n, g, d in facts:
        print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
    if ok:
        print(f"  evidence: {os.path.relpath(ev, ROOT)}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
