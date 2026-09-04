#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G42 湿街渲染门(符号门键 `g42.wet.street`;不消费 CI_step 号)。

湿地面镜面 + 积水 = `day_0902_rain_night` REPORT §7.3 自列三缺口之第一条。
车道形态 = **换载 fork 内核**(`artifacts/day_0904_wet_street/recon/R0.md` §1):
`--wet on` 时 scene pass 装 `.tmp/g42/spv/g42_direct_gi_wet.spv`,`--wet off`
/ 缺省仍装冻结件 `g14_3_direct_gi.spv` ⇒ presented digest 位级等于雨夜锚。

判据(R0 §9 八条闭集,逐条机器裁决,任一不满足即红):

1. **kernel_spv_valid**:`kernels/g42_direct_gi_wet.rx` 经 `rurixc --target
   vulkan` 产 SPV 且 `spirv-val` 接受(真调 spirv-val,不口称)。
2. **host_gold_tests**:`cargo test -p rurix-render --lib world::wet_ground`
   全绿(host 金标准 = device 公式面逐字同源,R0 §5)。
3. **wet_off_anchor_match**:`--wet off` 在 C2/C1 上跑雨夜终版定帧命令
   (96 帧 warmup 100),两条 presented digest 须分别等于雨夜锚
   `7a5ec1bc…` / `0985ebb8…`。`--wet off` 装冻结 SPV ⇒ 本条证明换载真的被
   旗标闸住(off 面零漂移)。
4. **neutral_superset_bitexact**:`--wet on --wet-dark 1.0 --wet-spec 0
   --puddle off` 的 presented digest 须与同帧数 `--wet off` 腿**位级相同**。
   这是最强的 0-语义漂移证据 —— 见 R0 §5.1:中性参下 dark_f ≡ 1.0 恰、
   pud ≡ 0.0 恰(nv ∈ [0,1) ⇒ smoothstep 参量恒 < 0)、spec/refl 各乘 0.0 恰
   ⇒ fork 输出与冻结件逐位相同,即 fork 是冻结件的**纯超集**而非改写。
5. **wet_on_double_run**:`--wet on` 缺省参双跑 presented digest 位级相等
   (确定性;逐像素独立求值禁 atomic)。
6. **arms_distinguishable**:`wet_off` / `wet_on` / `wet_on_no_puddle`
   (`--puddle off`)三臂 digest 两两互异 —— 每条臂都真接线并可观测。
7. **red_arm_closed_set**:闭集 fail-closed 四臂 device 真跑须各 rc != 0 ——
   (a) `--puddle on` 随 `--wet off`;(b) `--wet-dark 0.5` 随 `--wet off`;
   (c) `--wet-dark 1.5` 越 (0.2, 1.0] 闭集(随 `--wet on`);
   (d) `--wet on` 不随 `--particles on`。另加 kernel 级 RED 臂:湿 kernel
   拷进 `.tmp/g42/red/` 注入语法错,rurixc 须变红。
   **判读加防伪门**:rc != 0 之外还要求输出**不含**「未知参数」——否则湿旗标
   未接线时四臂会以「未知参数 --puddle」这种错原因假绿(本门最易被骗的面)。
8. **vuid_zero**:全部 device 腿 stdout+stderr 合并面无 `Validation Error`
   / `VUID-` 子串,且每腿 `RURIX_VK_VALIDATION` 均为 `1`。

帧时:`frame_ms_measured` 键**只登记不判红**(measured_local),口径同雨夜
REPORT §5 —— 湿面加 GGX + 1 条反射射线,升幅如实登记,不凑绿、不关雨、
不降分辨率。

三态:某腿打印 `skipped_dev_env` 且门层 `RURIX_REQUIRE_REAL` 非 `1` 时,
device 面(3~8)登记 skip 而非翻红;门层置 1 则硬红(禁 mock 充真跑)。

用法:
    py -3 ci/g42_wet_street_smoke.py --gate g42.wet.street
    py -3 ci/g42_wet_street_smoke.py --build-spv     # 只编湿 kernel + spirv-val
    py -3 ci/g42_wet_street_smoke.py --selftest      # 纯 CPU 判读器红绿臂
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
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, os.path.join(ROOT, "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

TAG = "g42_wet_street"
GATE = "g42.wet.street"
SCHEMA_ID = "rurix.g42.wet_street_gate_evidence.v1"
EXE = ".exe" if os.name == "nt" else ""

# 门层三态旗标(读环境;device_env() 会给子进程恒置 1,见该函数头注)。
REQUIRE_REAL = os.environ.get("RURIX_REQUIRE_REAL") == "1"

CAMPAIGN = os.path.join(ROOT, "artifacts", "day_0904_wet_street")
RAIN = os.path.join("artifacts", "day_0902_rain_night")  # 相对 ROOT(子进程 cwd)

KERNEL_NAME = "g42_direct_gi_wet"
KERNEL_SRC = os.path.join(ROOT, "src", "rurix-render", "kernels", f"{KERNEL_NAME}.rx")
SPV_DIR = os.path.join(ROOT, ".tmp", "g42", "spv")
WET_SPV = os.path.join(".tmp", "g42", "spv", f"{KERNEL_NAME}.spv")  # 相对路径进 argv
RED_DIR = os.path.join(ROOT, ".tmp", "g42", "red")

# 车道 bin 与 rurixc:**本门不自建**。本役构建由外部编排(GPU 门套件持 target
# 目录锁,门内再起 cargo 会二进制互覆盖假红,g9 蜂群纪律)⇒ 只查在位并
# fail-closed 提示操作者先手工构建。唯一在门时刻合法的 cargo 调用 = fact 2
# 的 host 单测(纯 host 面,且自持 gpu_device_lock 串行)。
LANE_BIN = os.path.join("target", "release", f"g35_particle_lane{EXE}")
RURIXC = os.path.join(ROOT, "target", "debug", f"rurixc{EXE}")
LANE_BUILD_HINT = (
    "cargo build --release -p rurix-render --features vendor-upscale --bin g35_particle_lane"
)
RURIXC_BUILD_HINT = "cargo build -p rurixc --features vulkan-backend --bin rurixc"

# 车道五件冻结 SPV(雨夜展示与 g35.wave3.render 同一现编产物目录;本门只读)。
LANE_SPV_DIR = os.path.join(".tmp", "g35_gates", "render")
SPV_ARGS = [
    "--spv-scene", os.path.join(LANE_SPV_DIR, "g14_3_direct_gi.spv"),
    "--spv-mv", os.path.join(LANE_SPV_DIR, "g14_mv.spv"),
    "--spv-resample", os.path.join(LANE_SPV_DIR, "g14_8_tsr_resample.spv"),
    "--spv-resolve", os.path.join(LANE_SPV_DIR, "g14_8_tsr_resolve.spv"),
    "--spv-encode", os.path.join(LANE_SPV_DIR, "g31_display_encode.spv"),
]

GLTF = r"K:\rurix_g10_cache\bistro-orca\v5_2\derived\BistroExterior\BistroExterior.gltf"

# 雨夜终版雨参(REPORT §5 逐字;任一数字变动即失锚,禁改)。
RAIN_FX = [
    "--rain-shutter", "1.0", "--rain-occlusion", "on", "--r-world", "0.0015",
    "--particle-tint", "0.40,0.44,0.52", "--particle-alpha-scale", "0.45",
    "--emit-max", "640",
    "--emitter-vel", "0.4,-9.0,0.2", "--emitter-vel-spread", "0.3,0.5,0.3",
    "--emitter-gravity", "-3", "--emitter-life", "1.659",
    "--gltf", GLTF,
    "--g10-dir", os.path.join(RAIN, "g10_corpus"),
    "--headless",
]

# 机位面(契约 + 契约 digest + 紧凑发射盒;REPORT §5/§8 + render_runs.jsonl
# still_C*_final 逐字)。--expect-digest 非可选:共享体 prelude 缺该旗标时按
# FROZEN_CONTRACT_DIGEST 比,雨夜借壳契约必不等 ⇒ 拒出图。
CAMERAS = {
    "C2": {
        "contract": os.path.join(RAIN, "contract_rain_night_C2_cd050.json"),
        "expect_digest": "sha256:37eea8257bc0faaf64bb8082d810fb3764bcb98f0ec3a0789252add6edb3e681",
        "emitter_pos": "6.492,10.304,-30.620",
        "emitter_spread": "15.417,1.500,13.587",
        "anchor": "7a5ec1bc48b49fb06dd5c3c2353fb05ed113fb56b06161f3e8f92367bfff0ced",
    },
    "C1": {
        "contract": os.path.join(RAIN, "contract_rain_night_C1_cd050.json"),
        "expect_digest": "sha256:5a5e8f70e823d6a104b11f93f81bc94b385e42b97b868109f2f624d425cc5f75",
        "emitter_pos": "17.916,10.328,-45.121",
        "emitter_spread": "14.757,1.500,12.659",
        "anchor": "0985ebb84663188e63761c0131d7b6eade53dc4d4647ab4d32bed1120c400dc5",
    },
}

# 锚腿帧窗 = 雨夜终版定帧命令逐字(96/100);其余腿降到 8 帧省时。
# warmup 100 **必须**保留:life 1.659 s ⇒ 雨柱落地约 1 s,warmup 不足时雨面
# 未达稳态(首探 16 帧只落 2.5 m 是雨夜的判读教训,REPORT §5)⇒ digest 无意义。
ANCHOR_FRAMES, ANCHOR_WARMUP = 96, 100
CHEAP_FRAMES, CHEAP_WARMUP = 8, 100

# presented digest 判读正则。源格式串(g35_particle_lane.rs L4396-4399 eprintln!):
#   "{G35L_TAG}: PASS on 面 oit={} frames={total} render={r_mean:.3}ms \
#    particle_gpu={pg_mean:.3}ms oit_gpu={og_mean:.3}ms presented={presented_digest}"
# 其中 G35L_TAG = "[g35_particle_lane]"(L180),presented_digest 由
# g35l_bgra_digest 产 = format!("sha256:{}", sha256_hex(..)) (L637-643) ⇒ 带
# `sha256:` 前缀 + 64 位小写 hex。REPORT §6 的锚为裸 hex,故本正则只捕 hex 体。
PRESENTED_RE = re.compile(r"presented=sha256:([0-9a-f]{64})\b")
# 同一 PASS 行的 render={r_mean:.3}ms == evidence frame_ms.real_render_frame_ms
# (同一 r_mean,行上截到 3 位小数)。
RENDER_MS_RE = re.compile(r"\brender=([0-9]+\.[0-9]+)ms\b")
# validation 面扫描字面(g35 车道门同律)。
VUID_MARKERS = ("Validation Error", "VUID-")
# 湿旗标未接线时车道会以「未知参数 --wet」退非零 —— RED 臂据此会假绿,故
# 把该字面本身列为防伪判据。
UNKNOWN_ARG_MARKER = "未知参数"

FACT_IDS = [
    "kernel_spv_valid",
    "host_gold_tests",
    "wet_off_anchor_match",
    "neutral_superset_bitexact",
    "wet_on_double_run",
    "arms_distinguishable",
    "red_arm_closed_set",
    "vuid_zero",
]
DEVICE_FACT_IDS = FACT_IDS[2:]  # 3~8 = device 面(三态可 skip)


# ---------------------------------------------------------------------------
# 执行原语
# ---------------------------------------------------------------------------


def run(cmd, timeout: int = 7200, env: dict | None = None):
    print(f"[{TAG}] $ {' '.join(str(c) for c in cmd)}", flush=True)
    return subprocess.run(
        cmd, cwd=ROOT, capture_output=True, text=True, encoding="utf-8",
        errors="replace", timeout=timeout, env=env,
    )


def device_env() -> dict:
    """device 腿子进程环境(g35_render_wiring_smoke.device_env 同律)。

    `RURIX_REQUIRE_REAL=1` 禁 mock 充真跑;它要求 `RURIX_VK_VALIDATION=1`
    成对(validation ERROR count 不可 unavailable,VUID=0 为 fact 8 判据)。
    诚实说明:子进程恒置 1 ⇒ 车道自身遇缺设备/缺资产会直接硬红而**不会**打印
    `skipped_dev_env`,故本门的 skip 支路是防御网(捕获不守该配对的 bin 变体),
    是否降级由**门层**环境的 RURIX_REQUIRE_REAL 裁决。
    """
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


# ---------------------------------------------------------------------------
# 判读器(纯函数;--selftest 红绿两臂消费面,零 GPU 零 cargo)
# ---------------------------------------------------------------------------


def parse_presented(text: str) -> str | None:
    """从 PASS 行取 presented digest 的裸 hex 体(形态不合即 None)。"""
    m = PRESENTED_RE.search(text or "")
    return m.group(1) if m else None


def parse_render_ms(text: str) -> float | None:
    """从 PASS 行取 render={r_mean:.3}ms(== real_render_frame_ms)。"""
    m = RENDER_MS_RE.search(text or "")
    if not m:
        return None
    try:
        v = float(m.group(1))
    except ValueError:
        return None
    return v if v == v and v > 0.0 else None


def anchor_ok(measured: str | None, expected: str) -> bool:
    """锚判:形态合法 + 位级相等(一字之差即红)。"""
    return (
        isinstance(measured, str)
        and re.fullmatch(r"[0-9a-f]{64}", measured) is not None
        and re.fullmatch(r"[0-9a-f]{64}", expected or "") is not None
        and measured == expected
    )


def bit_equal(a: str | None, b: str | None) -> bool:
    """位级相等判(任一侧缺失/形态破即红,不给「都是 None 所以相等」的假绿)。"""
    return (
        isinstance(a, str)
        and re.fullmatch(r"[0-9a-f]{64}", a) is not None
        and a == b
    )


def pairwise_distinct(digests: dict) -> bool:
    """三臂两两互异判(任一缺失/形态破/重复即红)。"""
    if not digests:
        return False
    vals = list(digests.values())
    if any(not isinstance(v, str) or re.fullmatch(r"[0-9a-f]{64}", v) is None for v in vals):
        return False
    return len(set(vals)) == len(vals)


def vuid_hits(text: str) -> list:
    """validation 面扫描:命中的字面(空 = 干净)。"""
    return [m for m in VUID_MARKERS if m in (text or "")]


def red_arm_ok(rc: int, out: str) -> bool:
    """RED 臂判:rc != 0 **且**不是因「未知参数」退出(旗标未接线的假绿面)
    **且**没有走到 PASS 行(没出图)。"""
    return (
        isinstance(rc, int)
        and rc != 0
        and UNKNOWN_ARG_MARKER not in (out or "")
        and parse_presented(out or "") is None
    )


def cargo_test_ok(stdout: str) -> tuple:
    """host 单测判:`test result: ok. N passed; 0 failed`(N ≥ 1)。"""
    m = re.search(r"test result: (\w+)\. (\d+) passed; (\d+) failed", stdout or "")
    if not m:
        return False, "无 test result 行"
    ok = m.group(1) == "ok" and m.group(3) == "0" and int(m.group(2)) >= 1
    return ok, m.group(0)


# ---------------------------------------------------------------------------
# 前置在位核验(fail-closed;本门不自建 bin —— 本役构建由外部编排)
# ---------------------------------------------------------------------------


def preflight(facts: list) -> bool:
    ok = True
    if not os.path.exists(os.path.join(ROOT, LANE_BIN)):
        facts.append((
            "preflight_lane_bin", False,
            f"缺车道 bin {LANE_BIN}:本门**不自建**(本役构建由外部编排,"
            f"门内起 cargo 会与在跑的 GPU 门套件互抢 target 目录锁 ⇒ 假红)。"
            f"请操作者先跑:{LANE_BUILD_HINT}",
        ))
        ok = False
    if not os.path.exists(RURIXC):
        facts.append((
            "preflight_rurixc", False,
            f"缺 rurixc {os.path.relpath(RURIXC, ROOT)}:同律本门不自建,"
            f"请操作者先跑:{RURIXC_BUILD_HINT}",
        ))
        ok = False
    if not os.path.exists(KERNEL_SRC):
        facts.append(("preflight_kernel_src", False, f"缺湿 kernel 源件 {KERNEL_SRC}"))
        ok = False
    missing = [p for p in SPV_ARGS[1::2] if not os.path.exists(os.path.join(ROOT, p))]
    if missing:
        facts.append((
            "preflight_lane_spv", False,
            f"缺车道冻结 SPV {missing}(雨夜锚的消费面,须与雨夜同一现编产物);"
            f"请先跑 py -3 ci/g35_render_wiring_smoke.py --gate g35.wave3.render 的 SPV 段",
        ))
        ok = False
    for c in CAMERAS.values():
        if not os.path.exists(os.path.join(ROOT, c["contract"])):
            facts.append(("preflight_contract", False, f"缺契约 {c['contract']}"))
            ok = False
    if shutil.which("spirv-val") is None:
        facts.append((
            "preflight_spirv_val", False,
            "PATH 上无 spirv-val(fact 1 须真调 spirv-val 而非口称);"
            "请把 Vulkan SDK 的 Bin 目录加入 PATH",
        ))
        ok = False
    return ok


# ---------------------------------------------------------------------------
# fact 1:湿 kernel 现编 + spirv-val
# ---------------------------------------------------------------------------


def build_spv(facts: list) -> bool:
    os.makedirs(SPV_DIR, exist_ok=True)
    out = os.path.join(ROOT, WET_SPV)
    r = run([RURIXC, KERNEL_SRC, "--target", "vulkan", "-o", out], timeout=1800)
    if r.returncode != 0 or not os.path.exists(out):
        facts.append((
            "kernel_spv_valid", False,
            f"rurixc 编译 {KERNEL_NAME}.rx 红: {(r.stderr or r.stdout).strip()[-400:]}",
        ))
        return False
    val = run(["spirv-val", out], timeout=600)
    if val.returncode != 0:
        facts.append((
            "kernel_spv_valid", False,
            f"spirv-val 未过 {WET_SPV}: {(val.stdout + val.stderr).strip()[-400:]}",
        ))
        return False
    facts.append((
        "kernel_spv_valid", True,
        f"{KERNEL_NAME}.rx rurixc 现编 → {WET_SPV}({os.path.getsize(out)} B)+ "
        f"spirv-val 接受(真调 {shutil.which('spirv-val')})",
    ))
    return True


def kernel_red_arm() -> tuple:
    """kernel 级 RED 臂:湿 kernel 拷进 .tmp/g42/red/ 注入语法错,rurixc 须红
    (证明 fact 1 的编译判据真的会红,反 YAML-only)。"""
    os.makedirs(RED_DIR, exist_ok=True)
    red_src = os.path.join(RED_DIR, f"{KERNEL_NAME}_red.rx")
    with open(KERNEL_SRC, "r", encoding="utf-8") as f:
        text = f.read()
    with open(red_src, "w", encoding="utf-8") as f:
        f.write(text)
        f.write("\n// RED 臂:人为注入语法错,rurixc 须拒编。\nfn (((\n")
    r = run([RURIXC, red_src, "--target", "vulkan", "-o",
             os.path.join(RED_DIR, f"{KERNEL_NAME}_red.spv")], timeout=1800)
    ok = r.returncode != 0
    return ok, ("注入语法错后 rurixc 如期红" if ok else "RED 臂失效:rurixc 接受了语法错件(漏检)")


# ---------------------------------------------------------------------------
# fact 2:host 金标准单测
# ---------------------------------------------------------------------------


def host_tests(facts: list) -> bool:
    # 门时刻唯一合法的 cargo 调用(纯 host 面)。自持 gpu_device_lock:该锁在本仓
    # 兼作**构建锁**(gpu_device_lock 头注:device 真跑腿与 cargo 构建/测试必须
    # 串行,否则并行 cargo 二进制互覆盖假红)。与 device 腿锁段分开、顺序不重叠。
    with gpu_device_lock(purpose=f"{TAG} host 金标准单测(world::wet_ground;cargo 构建锁)"):
        r = run(["cargo", "test", "-p", "rurix-render", "--lib", "world::wet_ground"], timeout=3600)
    ok, detail = cargo_test_ok(r.stdout)
    if not ok:
        detail = f"{detail};尾部输出 {(r.stdout + r.stderr).strip()[-300:]}"
    facts.append(("host_gold_tests", ok, f"cargo test -p rurix-render --lib world::wet_ground → {detail}"))
    return ok


# ---------------------------------------------------------------------------
# device 腿(argv 构造 + 结果缓存)
# ---------------------------------------------------------------------------


def lane_argv(cam: str, frames: int, warmup: int, wet: list, ev_rel: str,
              particles: str = "on", minimal: bool = False) -> list:
    """一腿 argv。minimal=True 时**只带**车道 SPV + 契约 + 帧窗 + 湿旗标 ——
    供 RED 臂 (d)(`--wet on` 不随 `--particles on`)用:雨参 `--emitter-*` /
    `--r-world` 等本身也有「须随 --particles on」的 fail-closed 面
    (g35_particle_lane.rs L3053/L3061),全带上会让该臂**以错原因变红**。"""
    c = CAMERAS[cam]
    argv = [LANE_BIN, *SPV_ARGS, "--particles", particles]
    if not minimal:
        argv += RAIN_FX
        argv += ["--emitter-pos", c["emitter_pos"], "--emitter-spread", c["emitter_spread"]]
    else:
        argv += ["--headless"]
    argv += [
        "--contract", c["contract"],
        "--expect-digest", c["expect_digest"],
        "--frames", str(frames), "--warmup", str(warmup),
    ]
    if ev_rel:
        argv += ["--evidence", ev_rel]
    return argv + list(wet)


# 湿旗标臂(`--spv-scene-wet` 在每条 `--wet on` 腿上显式给,不吃 bin 缺省 ⇒
# 门自持 hermetic:换载的到底是哪个 SPV 由门说了算)。
WET_OFF = ["--wet", "off"]
WET_ON = ["--wet", "on", "--spv-scene-wet", WET_SPV]
WET_ON_NEUTRAL = WET_ON + ["--wet-dark", "1.0", "--wet-spec", "0", "--puddle", "off"]
WET_ON_NO_PUDDLE = WET_ON + ["--puddle", "off"]

_LEG_CACHE: dict = {}
_LEG_LOG: list = []


def leg(label: str, cam: str, frames: int, warmup: int, wet: list, *,
        particles: str = "on", minimal: bool = False, fresh: bool = False) -> dict:
    """跑一腿并缓存:同一语义 argv 决不跑两次(`wet_off` 8 帧腿同时供 fact 4
    与 fact 6 消费)。fresh=True 强制重跑 —— fact 5 的双跑臂必须是两次真跑。"""
    key = json.dumps([cam, frames, warmup, wet, particles, minimal], ensure_ascii=False)
    if not fresh and key in _LEG_CACHE:
        hit = dict(_LEG_CACHE[key])
        hit["reused_by"] = label
        print(f"[{TAG}] 腿 {label} 命中缓存({hit['label']};同语义 argv 不重跑)", flush=True)
        return hit
    # 逐腿 harness 真跑件落 .tmp 工作区(战役 evidence/ 只收门裁决件)。
    ev_rel = os.path.join(".tmp", "g42", "legs", f"lane_{label}.json")
    os.makedirs(os.path.join(ROOT, os.path.dirname(ev_rel)), exist_ok=True)
    argv = lane_argv(cam, frames, warmup, wet, ev_rel, particles=particles, minimal=minimal)
    env = device_env()
    t0 = time.monotonic()
    try:
        r = run(argv, timeout=5400, env=env)
        rc, out = r.returncode, (r.stdout or "") + (r.stderr or "")
    except subprocess.TimeoutExpired as e:
        rc, out = 124, f"TimeoutExpired: {e}"
    wall_s = time.monotonic() - t0
    res = {
        "label": label,
        "camera": cam,
        "frames": frames,
        "warmup": warmup,
        "wet_args": list(wet),
        "particles": particles,
        "rc": rc,
        "presented": parse_presented(out),
        "render_frame_ms": parse_render_ms(out),
        "wall_s": round(wall_s, 2),
        "skipped_dev_env": "skipped_dev_env" in out,
        "vuid_hits": vuid_hits(out),
        "vk_validation": env.get("RURIX_VK_VALIDATION", ""),
        "require_real": env.get("RURIX_REQUIRE_REAL", ""),
        "tail": out.strip()[-600:],
        "argv": argv,
        # 全量输出只留内存(RED 臂「未知参数」防伪判读须扫全串);evidence 只落 tail。
        "out": out,
    }
    _LEG_CACHE[key] = res
    _LEG_LOG.append(res)
    print(
        f"[{TAG}] 腿 {label} rc={rc} wall={res['wall_s']}s "
        f"presented={(res['presented'] or '(无)')[:16]}… render={res['render_frame_ms']}ms",
        flush=True,
    )
    return res


def device_legs(facts: list, degrade: list) -> dict:
    """全部 device 腿(单一 gpu_device_lock 段内串行)。返回 arms/frame_ms 登记面。"""
    arms: dict = {}
    frame_ms: dict = {}
    red: dict = {}

    with gpu_device_lock(
        purpose="g42 湿街 off 锚腿 + 中性超集臂 + on 双跑 + 三臂判别 + RED device 真跑"
    ):
        # ── fact 3:off 锚腿 ×2(雨夜终版定帧命令 96/100 逐字;帧窗一变即失锚)──
        off_anchor = {
            cam: leg(f"{cam.lower()}_wet_off_anchor", cam, ANCHOR_FRAMES, ANCHOR_WARMUP, WET_OFF)
            for cam in ("C2", "C1")
        }
        # ── fact 4/6:C2 8 帧 off 腿(两处共用,缓存保证只跑一次)──
        off_cheap = leg("c2_wet_off_8", "C2", CHEAP_FRAMES, CHEAP_WARMUP, WET_OFF)
        # ── fact 4:中性超集臂(R0 §5.1:中性参下 fork 输出与冻结件逐位相同)──
        neutral = leg("c2_wet_on_neutral_8", "C2", CHEAP_FRAMES, CHEAP_WARMUP, WET_ON_NEUTRAL)
        # ── fact 5/6:on 缺省臂双跑(fresh=True 绕缓存 —— 双跑必须是两次真跑)──
        on_a = leg("c2_wet_on_8_a", "C2", CHEAP_FRAMES, CHEAP_WARMUP, WET_ON)
        on_b = leg("c2_wet_on_8_b", "C2", CHEAP_FRAMES, CHEAP_WARMUP, WET_ON, fresh=True)
        # ── fact 6:on 无积水臂 ──
        on_np = leg("c2_wet_on_no_puddle_8", "C2", CHEAP_FRAMES, CHEAP_WARMUP, WET_ON_NO_PUDDLE)
        # ── fact 7:device RED 四臂(全在 CLI 闭集裁决面变红 ⇒ 不进设备,近乎零耗)──
        red["puddle_with_wet_off"] = leg(
            "red_a_puddle_with_wet_off", "C2", CHEAP_FRAMES, CHEAP_WARMUP,
            ["--wet", "off", "--puddle", "on"])
        red["wet_dark_with_wet_off"] = leg(
            "red_b_wetdark_with_wet_off", "C2", CHEAP_FRAMES, CHEAP_WARMUP,
            ["--wet", "off", "--wet-dark", "0.5"])
        red["wet_dark_out_of_set"] = leg(
            "red_c_wetdark_1p5", "C2", CHEAP_FRAMES, CHEAP_WARMUP,
            WET_ON + ["--wet-dark", "1.5"])
        red["wet_on_without_particles"] = leg(
            "red_d_wet_on_particles_off", "C2", CHEAP_FRAMES, CHEAP_WARMUP,
            WET_ON, particles="off", minimal=True)

    # ── 三态检出(任一腿打印 skipped_dev_env)──
    for res in _LEG_LOG:
        if res["skipped_dev_env"]:
            degrade.append(f"腿 {res['label']} 打印 skipped_dev_env: {res['tail'][-200:]}")
    if degrade:
        return {"arms": {}, "frame_ms": {}, "red": {}}

    # ── fact 3 判读 ──
    ok3 = all(anchor_ok(off_anchor[cam]["presented"], CAMERAS[cam]["anchor"]) for cam in ("C2", "C1"))
    ok3 = ok3 and all(off_anchor[cam]["rc"] == 0 for cam in ("C2", "C1"))
    facts.append((
        "wet_off_anchor_match", ok3,
        "; ".join(
            f"{cam} --wet off {ANCHOR_FRAMES}/{ANCHOR_WARMUP} presented="
            f"{(off_anchor[cam]['presented'] or '(无)')[:16]}… == 锚 "
            f"{CAMERAS[cam]['anchor'][:16]}… → "
            f"{anchor_ok(off_anchor[cam]['presented'], CAMERAS[cam]['anchor'])}"
            for cam in ("C2", "C1")
        ) + "(off 面装冻结 SPV ⇒ 本条 = 换载真被旗标闸住的机器证明)",
    ))

    # ── fact 4 判读(位级,非近似)──
    ok4 = (
        off_cheap["rc"] == 0 and neutral["rc"] == 0
        and bit_equal(neutral["presented"], off_cheap["presented"])
    )
    facts.append((
        "neutral_superset_bitexact", ok4,
        f"--wet on --wet-dark 1.0 --wet-spec 0 --puddle off presented="
        f"{(neutral['presented'] or '(无)')[:16]}… vs --wet off "
        f"{(off_cheap['presented'] or '(无)')[:16]}…(同 {CHEAP_FRAMES}/{CHEAP_WARMUP} 帧窗)"
        f"位级等={bit_equal(neutral['presented'], off_cheap['presented'])};"
        f"R0 §5.1:中性参下 dark_f ≡ 1.0 恰、pud ≡ 0.0 恰、spec/refl 各乘 0.0 恰 ⇒ "
        f"fork 是冻结件的**纯超集**而非改写(本门最强 0-语义漂移证据)",
    ))

    # ── fact 5 判读 ──
    ok5 = on_a["rc"] == 0 and on_b["rc"] == 0 and bit_equal(on_a["presented"], on_b["presented"])
    facts.append((
        "wet_on_double_run", ok5,
        f"--wet on 缺省参双跑 presented {(on_a['presented'] or '(无)')[:16]}… × 2 "
        f"位级等={bit_equal(on_a['presented'], on_b['presented'])}(逐像素独立求值禁 atomic)",
    ))

    # ── fact 6 判读 ──
    arms = {
        "wet_off": off_cheap["presented"],
        "wet_on": on_a["presented"],
        "wet_on_no_puddle": on_np["presented"],
    }
    ok6 = on_np["rc"] == 0 and pairwise_distinct(arms)
    facts.append((
        "arms_distinguishable", ok6,
        f"三臂 presented 两两互异={pairwise_distinct(arms)}:"
        + "; ".join(f"{k}={(v or '(无)')[:16]}…" for k, v in arms.items()),
    ))

    # ── fact 7 device 四臂 + kernel 臂 ──
    red_ok = {k: red_arm_ok(v["rc"], v["out"]) for k, v in red.items()}
    kr_ok, kr_detail = kernel_red_arm()
    red_ok["kernel_syntax_error"] = kr_ok
    ok7 = all(red_ok.values())
    facts.append((
        "red_arm_closed_set", ok7,
        "; ".join(f"{k}={'红如期' if v else '失效(漏检)'}" for k, v in red_ok.items())
        + f";kernel 臂:{kr_detail}"
        + f";判读 = rc != 0 ∧ 无「{UNKNOWN_ARG_MARKER}」∧ 未走到 PASS 行"
        + "(防湿旗标未接线时以错原因假绿)"
        + ";" + "; ".join(
            f"{k} rc={v['rc']} 尾={v['tail'][-120:]}" for k, v in red.items()
        ),
    ))

    # ── fact 8:VUID 面 ──
    all_hits = {r["label"]: r["vuid_hits"] for r in _LEG_LOG if r["vuid_hits"]}
    vk_all_one = all(r["vk_validation"] == "1" for r in _LEG_LOG) and bool(_LEG_LOG)
    ok8 = not all_hits and vk_all_one
    facts.append((
        "vuid_zero", ok8,
        f"{len(_LEG_LOG)} 条 device 腿 stdout+stderr 合并面 "
        f"{'无' if not all_hits else '命中 ' + json.dumps(all_hits, ensure_ascii=False)} "
        f"{VUID_MARKERS} 子串;每腿 RURIX_VK_VALIDATION=1={vk_all_one}",
    ))

    # ── 帧时登记(measured,**不判红**)──
    frame_ms = {
        "evidence_level": "measured_local",
        "note": (
            "逐腿 real_render_frame_ms 取自车道 PASS 行 render={r_mean:.3}ms(== evidence "
            "frame_ms.real_render_frame_ms 同一 r_mean,行上截 3 位小数);解析不到时退 "
            "wall_clock 墙钟秒(含 glTF 装配约 50 s 固定成本,不可与帧时同域比较)。"
            "measured_local 诚实登记,**非帧率对标门** —— 湿面加 GGX + 1 条反射射线,升幅"
            "如实登记,不凑绿、不关雨、不降分辨率(雨夜 REPORT §5 同律;干面基线 C2 6.367 "
            "ms / C1 7.484 ms)。"
        ),
        "legs": {
            r["label"]: {
                "real_render_frame_ms": r["render_frame_ms"],
                "wall_s": r["wall_s"],
                "source": "pass_line_render_ms" if r["render_frame_ms"] is not None else "wall_clock_only",
                "camera": r["camera"],
                "frames": r["frames"],
                "warmup": r["warmup"],
                "wet_args": r["wet_args"],
            }
            for r in _LEG_LOG
        },
    }
    return {"arms": arms, "frame_ms": frame_ms, "red": red_ok}


# ---------------------------------------------------------------------------
# selftest(纯 CPU:判读器能红也能绿 + fact 闭集互核;无 cargo 无 GPU)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    misses = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal misses
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            misses += 1

    d0 = "a" * 64
    d1 = "b" * 64
    pass_line = (
        "[g35_particle_lane]: PASS on 面 oit=off frames=196 render=6.367ms "
        f"particle_gpu=2.295ms oit_gpu=0.000ms presented=sha256:{d0}"
    )
    # ① digest 正则:合法 PASS 行绿,畸形行红。
    expect(parse_presented(pass_line) == d0, "GREEN:digest 正则命中合成 PASS 行")
    expect(parse_render_ms(pass_line) == 6.367, "GREEN:render_ms 正则命中同一 PASS 行")
    expect(parse_presented("presented=sha256:" + "a" * 63) is None, "RED:63 位 hex 畸形行必拒")
    expect(parse_presented("presented=sha256:" + "A" * 64) is None, "RED:大写 hex 畸形行必拒")
    expect(parse_presented("presented=" + d0) is None, "RED:缺 sha256: 前缀必拒")
    expect(parse_presented("") is None, "RED:空输出必拒")
    expect(parse_render_ms("render=6ms") is None, "RED:无小数位 render 形态必拒")
    expect(parse_render_ms("render=0.000ms") is None, "RED:0ms 必拒")
    # ② 两两互异助手:重复即红。
    expect(pairwise_distinct({"a": d0, "b": d1, "c": "c" * 64}), "GREEN:三臂互异正例")
    expect(not pairwise_distinct({"a": d0, "b": d1, "c": d0}), "RED:重复 digest 必红")
    expect(not pairwise_distinct({"a": d0, "b": None, "c": d1}), "RED:某臂缺 digest 必红")
    expect(not pairwise_distinct({}), "RED:空臂集必红")
    # ③ 锚比较:一字之差即红。
    expect(anchor_ok(d0, d0), "GREEN:锚位级等正例")
    expect(not anchor_ok("b" + d0[1:], d0), "RED:锚一字之差必红")
    expect(not anchor_ok(None, d0), "RED:锚侧 digest 缺失必红")
    expect(not anchor_ok(d0, "zz"), "RED:期望锚形态破必红")
    expect(bit_equal(d0, d0) and not bit_equal(d0, d1), "GREEN/RED:位级等助手两臂")
    expect(not bit_equal(None, None), "RED:双侧缺失不得算相等(假绿面)")
    # ④ VUID 扫描:合成 VUID 行必被捕。
    expect(vuid_hits("… VUID-vkCmdDispatch-None-02699 …") == ["VUID-"], "RED:合成 VUID 行必捕")
    expect(vuid_hits("Validation Error: [ x ]") == ["Validation Error"], "RED:Validation Error 必捕")
    expect(vuid_hits(pass_line) == [], "GREEN:干净 PASS 行零命中")
    # ⑤ RED 臂判读:未接线的「未知参数」假绿必被拒。
    expect(red_arm_ok(1, "FAIL: --puddle on 须随 --wet on"), "GREEN:RED 臂以正确原因红")
    expect(not red_arm_ok(0, "ok"), "RED:rc=0 的 RED 臂必红(漏检)")
    expect(not red_arm_ok(1, f"FAIL: {UNKNOWN_ARG_MARKER} --puddle"),
           "RED:「未知参数」= 旗标未接线,不得算 RED 臂成立")
    expect(not red_arm_ok(1, pass_line), "RED:走到 PASS 行(已出图)不得算 RED 臂成立")
    # ⑥ host 单测判读。
    expect(cargo_test_ok("test result: ok. 20 passed; 0 failed; 0 ignored")[0],
           "GREEN:host 单测 20 绿正例")
    expect(not cargo_test_ok("test result: FAILED. 19 passed; 1 failed")[0], "RED:1 failed 必红")
    expect(not cargo_test_ok("test result: ok. 0 passed; 0 failed")[0],
           "RED:0 passed(过滤器打空)必红")
    expect(not cargo_test_ok("")[0], "RED:无 test result 行必红")
    # ⑦ fact 名闭集互核(R0 §9 八条,顺序即报告序)。
    expect(FACT_IDS == [
        "kernel_spv_valid", "host_gold_tests", "wet_off_anchor_match",
        "neutral_superset_bitexact", "wet_on_double_run", "arms_distinguishable",
        "red_arm_closed_set", "vuid_zero",
    ], "fact 闭集 == R0 §9 文档八条(名与序)")
    expect(len(FACT_IDS) == 8 and len(DEVICE_FACT_IDS) == 6, "facts=8;device 面=6(三态可 skip)")
    # ⑧ argv 构造互核(锚腿 = 雨夜终版字面;RED 臂 (d) 须为 minimal 面)。
    a = lane_argv("C2", ANCHOR_FRAMES, ANCHOR_WARMUP, WET_OFF, "")
    expect("--emitter-pos" in a and a[a.index("--emitter-pos") + 1] == CAMERAS["C2"]["emitter_pos"],
           "GREEN:C2 锚腿携紧凑发射盒")
    expect(a[a.index("--frames") + 1] == "96" and a[a.index("--warmup") + 1] == "100",
           "GREEN:锚腿帧窗 = 96/100(雨夜终版定帧命令)")
    expect("--expect-digest" in a, "GREEN:每腿携契约 --expect-digest(缺则按 FROZEN 比 ⇒ 拒出图)")
    m = lane_argv("C2", CHEAP_FRAMES, CHEAP_WARMUP, WET_ON, "", particles="off", minimal=True)
    expect("--emitter-pos" not in m and "--r-world" not in m,
           "GREEN:RED 臂 (d) minimal argv 不携雨参(否则以错原因变红)")
    expect("--spv-scene-wet" in WET_ON and WET_ON[WET_ON.index("--spv-scene-wet") + 1] == WET_SPV,
           "GREEN:每条 --wet on 腿显式给 --spv-scene-wet(hermetic)")
    expect("--spv-scene-wet" not in WET_OFF, "GREEN:--wet off 腿不携湿 SPV 旗标")

    if misses:
        print(f"[{TAG}] selftest FAIL ({misses})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(8 组红绿臂 + fact 闭集/argv 互核;纯 CPU 零 cargo 零 GPU)")
    return 0


# ---------------------------------------------------------------------------
# main
# ---------------------------------------------------------------------------


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=None)
    ap.add_argument("--build-spv", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--evidence", default=None)
    a = ap.parse_args()

    if a.gate and a.gate != GATE:
        print(f"{TAG}: 未知门键 {a.gate}(本门 = {GATE})", file=sys.stderr)
        return 2

    if a.selftest:
        return run_selftest()

    facts: list = []
    degrade: list = []

    if not preflight(facts):
        print(f"{TAG} [{GATE}] FAIL(前置在位核验红)")
        for n, g, d in facts:
            print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
        return 1

    ok = build_spv(facts)
    if a.build_spv:
        for n, g, d in facts:
            print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
        return 0 if ok else 1

    ok = host_tests(facts) and ok

    arms: dict = {}
    frame_ms: dict = {}
    red: dict = {}
    if ok:
        out = device_legs(facts, degrade)
        arms, frame_ms, red = out["arms"], out["frame_ms"], out["red"]
    else:
        degrade.append("前置 fact(kernel SPV / host 单测)红,device 腿不启(不烧 GPU 时段)")

    if degrade:
        # 三态:门层 RURIX_REQUIRE_REAL=1 下 SKIP 翻硬红(禁 mock 充真跑);
        # 否则 device 面(3~8)登记 skip,非 PASS 非 FAIL 的第三态。
        if REQUIRE_REAL or not ok:
            for fid in DEVICE_FACT_IDS:
                if fid not in {n for n, _, _ in facts}:
                    facts.append((fid, False, f"未裁决:{degrade[0][:200]}"))
            ok = False
        else:
            for fid in DEVICE_FACT_IDS:
                if fid not in {n for n, _, _ in facts}:
                    facts.append((fid, True, f"skipped_dev_env(门层 RURIX_REQUIRE_REAL≠1):{degrade[0][:200]}"))

    ok = ok and all(g for _, g, _ in facts)

    ts = datetime.datetime.now(datetime.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    # 落**战役目录**而非仓根 `evidence/`:后者由 `ci/check_schemas.py` 按
    # bench/sampling/results 基准件 schema 强校验,本门是渲染特性门不是基准件
    # (day_0902_rain_night / day_0903_water 同律把 run/gate 件放战役目录)。
    ev = a.evidence or os.path.join(CAMPAIGN, "evidence", f"g42_wet_street_gate_{ts}.json")
    payload = {
        "schema": SCHEMA_ID,
        "gate": GATE,
        "verdict": "PASS" if ok else "FAIL",
        "generated_utc": ts,
        "mode": "full",
        "campaign": "artifacts/day_0904_wet_street",
        "rfc": "R0.md §9(侦察交接单验收口径)",
        "facts": [{"name": n, "ok": g, "detail": d} for n, g, d in facts],
        "anchors": {
            "source": "artifacts/day_0902_rain_night/REPORT.md §6(终版定帧 presented)",
            "command": f"--particles on --frames {ANCHOR_FRAMES} --warmup {ANCHOR_WARMUP}",
            "C2_presented": CAMERAS["C2"]["anchor"],
            "C1_presented": CAMERAS["C1"]["anchor"],
            "C2_contract_expect_digest": CAMERAS["C2"]["expect_digest"],
            "C1_contract_expect_digest": CAMERAS["C1"]["expect_digest"],
        },
        "arms": arms,
        "red_arms": red,
        "kernel": {
            "src": f"src/rurix-render/kernels/{KERNEL_NAME}.rx",
            "spv": WET_SPV.replace("\\", "/"),
        },
        "frame_ms_measured": frame_ms or {
            "evidence_level": "measured_local",
            "note": "device 面未裁决(前置红或三态 skip),无帧时登记",
            "legs": {},
        },
        "env": {
            "gate_require_real": REQUIRE_REAL,
            "child_require_real": "1",
            "child_vk_validation": "1",
            "note": (
                "每条 device 腿子进程恒置 RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1"
                "(二者成对:validation ERROR count 不可 unavailable,VUID=0 为 fact 8 判据)"
            ),
        },
        "legs": [
            {k: v for k, v in r.items() if k != "out"}
            for r in _LEG_LOG
        ],
        "degrade": degrade,
    }
    if ok:  # fail-closed:只有 PASS 才落 evidence(全仓同律)
        os.makedirs(os.path.dirname(ev), exist_ok=True)
        with open(ev, "w", encoding="utf-8") as f:
            json.dump(payload, f, ensure_ascii=False, indent=2)

    print(f"{TAG} [{GATE}] GATE {'PASS' if ok else 'FAIL'}")
    for n, g, d in facts:
        print(f"  [{'PASS' if g else 'FAIL'}] {n}: {d}")
    if ok:
        print(f"  evidence: {os.path.relpath(ev, ROOT)}")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
