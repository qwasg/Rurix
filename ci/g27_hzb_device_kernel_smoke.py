#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G27.2 实现波)
"""G27.2 M-a HZB device kernel 兑现门冒烟
(g27.p0.m_a.hzb_device_kernel;G27_CONTRACT §4.2 M-a 行判据逐字;
rfcs/0044-geometry-device-realization.md §1 判据事实源;
G27_ACCEPTANCE_MAP §1 M-a 行)。

硬判据:kernels/g27_hzb_reduce.rx + g27_hzb_test.rx(rurixc --target vulkan 产
SPV + spirv-val 各自通过)经 vk::run_compute 派发——g20_hzb_probe 夹具逐字同源
(193×117 非 2 幂确定性深度场 + det_rects(800) + reverse-Z/standard-Z 双臂):
① device 金字塔 vs host HzbPyramid::build 逐级位级相等(零容差协议——纯 min/max
选择归约零舍入,RFC-0044 §1.1;**本门无标定腿无 budget 条目**,G26 标定容差协议
不适用本面)+ ② 800 rect × 双约定判定序列 vs host test_rect 逐 rect 逐字节全等
+ ③ 零假阳性硬不变量(device 判 Occluded ⇒ exact_rect_occluded 独立复核同判,
F3 纵深防御)+ ④ device 双跑位级(digest = sha256(判定位序列 ‖ 金字塔 f32 LE),
F11 字面)+ ⑤ tamper RED 臂(构造性注入单一金字塔纹素 → 逐 rect 序列必异 +
假阳性哨兵必检出,F4)+ ⑥ host 冻结面 0-byte 机核(geometry/ hzb.rs + cull.rs +
visbuffer.rs vs g26-closed,git-diff + 工作树双面)。

三态:无 Vulkan loader/设备 → harness skipped_dev_env(退 0 非 fake pass);
本脚本默认 RURIX_REQUIRE_REAL=1(setdefault),该态下 SKIP → 硬红如实登记
FAIL,不假绿。

用法:
  py -3 ci/g27_hzb_device_kernel_smoke.py --gate g27.p0.m_a.hzb_device_kernel
  py -3 ci/g27_hzb_device_kernel_smoke.py --verify-latest
  py -3 ci/g27_hzb_device_kernel_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g27.p0.m_a.hzb_device_kernel"
NUMERIC_STEP = 464
SUBJECT = "g27_m_a_hzb_device_kernel"
WAVE = "G27.2"
SCHEMA_PATH = ROOT / "milestones/g27/g27_m_a_hzb_device_kernel_evidence_schema.json"
SOURCE_REF = (
    "G27_CONTRACT §4.2 M-a;rfcs/0044-geometry-device-realization.md §1;"
    "G27_ACCEPTANCE_MAP §1 M-a 行"
)
TAG = "g27_m_a"

KERNEL_REDUCE = ROOT / "src/rurix-render/kernels/g27_hzb_reduce.rx"
KERNEL_TEST = ROOT / "src/rurix-render/kernels/g27_hzb_test.rx"
WORK_DIR = ROOT / ".tmp/g27_gates"
SPV_REDUCE = WORK_DIR / "g27_hzb_reduce.spv"
SPV_TEST = WORK_DIR / "g27_hzb_test.spv"
HARNESS_BIN = "g27_hzb_device"
FROZEN_BASE = "g26-closed"
# RFC-0044 §1.5 host 参考臂冻结:对拍承重面 + 生产剔除链面三文件。
FROZEN_PATHS = [
    "src/rurix-render/src/geometry/hzb.rs",
    "src/rurix-render/src/geometry/cull.rs",
    "src/rurix-render/src/geometry/visbuffer.rs",
]
ARM_NAMES = ["reverse_z", "standard_z"]
RECTS = 800

# facts 闭集(≥7;schema extra_facts minItems 6)。
FACT_IDS = [
    "spv_compile_spirv_val_pass",
    "mips_bitexact_all_levels",
    "rect_verdict_sequence_equal_800x2",
    "zero_false_positive_vs_exact",
    "device_double_run_bitexact",
    "tamper_red_arm_detected",
    "geometry_frozen_0byte",
]


def run(cmd: list[str], **kw) -> subprocess.CompletedProcess:
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, **kw)


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


def target_dir() -> Path:
    alt = os.environ.get("CARGO_TARGET_DIR")
    return (ROOT / alt) if alt else (ROOT / "target")


def build_rurixc() -> Path | None:
    print(f"[{TAG}] cargo build -p rurixc --features vulkan-backend --bin rurixc")
    r = run(["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc"])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    return exe if exe.is_file() else None


def build_harness() -> Path | None:
    print(f"[{TAG}] cargo build -p rurix-render --features vulkan --bin {HARNESS_BIN}")
    r = run(["cargo", "build", "-p", "rurix-render", "--features", "vulkan", "--bin", HARNESS_BIN])
    if r.returncode != 0:
        print(r.stderr[-2000:])
        return None
    exe = target_dir() / "debug" / (f"{HARNESS_BIN}.exe" if sys.platform == "win32" else HARNESS_BIN)
    return exe if exe.is_file() else None


def compile_spv(rurixc: Path, kernel: Path, spv: Path) -> tuple[bool, str]:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    print(f"[{TAG}] rurixc {kernel.name} --target vulkan -o {spv.relative_to(ROOT)}")
    r = run([str(rurixc), str(kernel), "--target", "vulkan", "-o", str(spv)])
    if r.returncode != 0 or not spv.is_file():
        return False, f"{kernel.name} SPV 编译失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}"
    # spirv-val 独立校验(rurixc 内建校验之外的第二判读面;缺工具即 RED——
    # M-a 行「spirv-val 通过」为硬判据,不 SKIP)。
    val = run(["spirv-val", str(spv)])
    if val.returncode != 0:
        return False, f"{kernel.name} spirv-val 未过: {(val.stdout + val.stderr)[-300:]}"
    return True, f"{kernel.name} rurixc --target vulkan 产 SPV + spirv-val 通过"


def json_line(stdout: str, schema_token: str) -> str | None:
    for line in stdout.splitlines():
        if schema_token in line:
            return line.strip()
    return None


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面)
# ---------------------------------------------------------------------------


def _harness_state(line: str) -> str:
    try:
        return json.loads(line).get("state", "")
    except json.JSONDecodeError:
        return ""


def _arms_field_all_true(doc: dict, field: str) -> bool:
    """双臂布尔字段全真判读(arms 缺臂/缺字段/非 True 均拒)。"""
    arms = doc.get("arms") or []
    by_conv = {a.get("conv"): a for a in arms if isinstance(a, dict)}
    return all(
        (by_conv.get(name) or {}).get(field) is True for name in ARM_NAMES
    )


def _arms_zero_fp_cull_nonzero(doc: dict) -> bool:
    """③+⑤ 判读:双臂 false_positives==0 且 occluded>0(剔除数非零)。"""
    arms = doc.get("arms") or []
    by_conv = {a.get("conv"): a for a in arms if isinstance(a, dict)}
    for name in ARM_NAMES:
        a = by_conv.get(name) or {}
        if a.get("false_positives") != 0:
            return False
        occ = a.get("occluded")
        if not isinstance(occ, int) or occ <= 0:
            return False
    return True


def _arms_rects_800(doc: dict) -> bool:
    arms = doc.get("arms") or []
    by_conv = {a.get("conv"): a for a in arms if isinstance(a, dict)}
    return all((by_conv.get(name) or {}).get("rects") == RECTS for name in ARM_NAMES)


def _red_arm_detected(line: str, rc: int) -> bool:
    try:
        doc = json.loads(line) if line else {}
    except json.JSONDecodeError:
        doc = {}
    return rc == 0 and doc.get("detected") is True and doc.get("state") != "skipped_dev_env"


# ---------------------------------------------------------------------------
# host 冻结面 0-byte 机核(⑥:vs g26-closed + 工作树双面;g26 temporal 同模)
# ---------------------------------------------------------------------------


def geometry_frozen_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", FROZEN_BASE, "--", *FROZEN_PATHS])
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", FROZEN_BASE, "--", *FROZEN_PATHS])
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"geometry 冻结面有差分 vs {FROZEN_BASE}(触碰即 RED): {changed[:3]}"
    u = run(["git", "status", "--porcelain", "--", *FROZEN_PATHS])
    if u.stdout.strip():
        dirty = [x for x in u.stdout.splitlines() if x.strip()]
        return False, f"geometry 冻结面工作树未提交面: {dirty[:3]}"
    return True, (
        f"git diff --quiet {FROZEN_BASE} -- hzb.rs cull.rs visbuffer.rs 0-byte"
        "(提交面 + 工作树双面)"
    )


# ---------------------------------------------------------------------------
# gate
# ---------------------------------------------------------------------------


def run_gate() -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}

    with gpu_device_lock(purpose=f"{TAG} 构建+SPV+全档双约定+tamper RED 臂"):
        rurixc = build_rurixc()
        if rurixc is None:
            set_fact("spv_compile_spirv_val_pass", False, "rurixc 构建失败")
        else:
            ok_r, det_r = compile_spv(rurixc, KERNEL_REDUCE, SPV_REDUCE)
            ok_t, det_t = compile_spv(rurixc, KERNEL_TEST, SPV_TEST)
            set_fact("spv_compile_spirv_val_pass", ok_r and ok_t, f"{det_r};{det_t}")
        harness = build_harness()
        if harness is None:
            set_fact("mips_bitexact_all_levels", False, "harness 构建失败")
        elif facts["spv_compile_spirv_val_pass"]["status"] == "PASS":
            # ── 全档验证(双约定逐臂真跑;REQUIRE_REAL+VK_VALIDATION)──
            print(f"[{TAG}] 全档验证: --spv-reduce --spv-test(双约定 800 rect)")
            r = run(
                [
                    str(harness),
                    "--spv-reduce", str(SPV_REDUCE),
                    "--spv-test", str(SPV_TEST),
                ],
                env=device_env(), timeout=3600,
            )
            line = json_line(r.stdout, "rurix.g27hzb.harness.v1")
            doc = json.loads(line) if line else {}
            state = doc.get("state", "")
            device_facts = (
                "mips_bitexact_all_levels",
                "rect_verdict_sequence_equal_800x2",
                "zero_false_positive_vs_exact",
                "device_double_run_bitexact",
            )
            if state == "skipped_dev_env":
                for fid in device_facts:
                    set_fact(fid, False, "device SKIP(skipped_dev_env;RURIX_REQUIRE_REAL=1 下如实 FAIL 不假绿)")
            elif not doc:
                for fid in device_facts:
                    set_fact(fid, False, f"harness 无 evidence 行 rc={r.returncode}: {(r.stdout + r.stderr)[-200:]}")
            else:
                arms = {a.get("conv"): a for a in (doc.get("arms") or []) if isinstance(a, dict)}
                set_fact(
                    "mips_bitexact_all_levels",
                    _arms_field_all_true(doc, "mips_bitexact"),
                    "device 金字塔 vs host HzbPyramid::build 逐级位级相等(零容差);"
                    + ";".join(f"{n} mips={(arms.get(n) or {}).get('mips')}" for n in ARM_NAMES),
                )
                set_fact(
                    "rect_verdict_sequence_equal_800x2",
                    state == "pass"
                    and r.returncode == 0
                    and _arms_field_all_true(doc, "verdict_sequence_equal")
                    and _arms_rects_800(doc),
                    "state=pass;800 rect × 双约定判定序列 vs host test_rect 逐 rect 全等;"
                    + ";".join(
                        f"{n} occluded={(arms.get(n) or {}).get('occluded')}" for n in ARM_NAMES
                    ),
                )
                set_fact(
                    "zero_false_positive_vs_exact",
                    _arms_zero_fp_cull_nonzero(doc),
                    "device Occluded ⇒ exact_rect_occluded 同判(独立复核)且剔除数>0;"
                    + ";".join(
                        f"{n} fp={(arms.get(n) or {}).get('false_positives')}"
                        f" occluded={(arms.get(n) or {}).get('occluded')}"
                        for n in ARM_NAMES
                    ),
                )
                set_fact(
                    "device_double_run_bitexact",
                    _arms_field_all_true(doc, "double_run_bitexact"),
                    "双臂固定输入全链双跑 digest 位级相等(判定位序列 ‖ 金字塔 f32 LE,F11)",
                )
            # ── tamper RED 臂真跑(构造性注入必检出)──
            print(f"[{TAG}] RED 臂: --red-arm tamper")
            ra = run(
                [
                    str(harness),
                    "--red-arm", "tamper",
                    "--spv-reduce", str(SPV_REDUCE),
                    "--spv-test", str(SPV_TEST),
                ],
                env=device_env(), timeout=3600,
            )
            rl = json_line(ra.stdout, "rurix.g27hzb.red_arm.v1")
            try:
                rdoc = json.loads(rl) if rl else {}
            except json.JSONDecodeError:
                rdoc = {}
            ok = _red_arm_detected(rl or "", ra.returncode)
            set_fact(
                "tamper_red_arm_detected",
                ok,
                f"tamper:{'检出' if ok else 'FAIL'}({rdoc.get('detail', rdoc.get('reason', ''))[:120]})",
            )

    ok, detail = geometry_frozen_0byte()
    set_fact("geometry_frozen_0byte", ok, detail)

    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows)
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=fact_rows,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            "G27.2 M-a:HZB device kernel 兑现——g27_hzb_reduce.rx + g27_hzb_test.rx 经 "
            "vk::run_compute 真跑;g20_hzb_probe 夹具逐字同源 193×117 × 800 rect × 双约定:"
            "device 金字塔逐级位级相等(零容差,无标定腿无 budget 条目)+ 判定序列逐 rect "
            "全等 + 零假阳性独立复核 + device 双跑位级 + tamper RED 臂(构造性注入)+ "
            "geometry/(hzb+cull+visbuffer)vs g26-closed 0-byte;"
            "RURIX_REQUIRE_REAL=1 + RURIX_VK_VALIDATION=1"
        ),
        host_section_pass=all_pass,
    )
    return 0 if (all_pass and code == 0) else 1


# ---------------------------------------------------------------------------
# selftest(反 YAML-only:判读器红绿两臂,无 GPU 依赖)
# ---------------------------------------------------------------------------


def run_selftest() -> int:
    failures = 0

    def expect(cond: bool, name: str) -> None:
        nonlocal failures
        if cond:
            print(f"  ok   — {name}")
        else:
            print(f"  MISS — {name}", file=sys.stderr)
            failures += 1

    def arm(conv: str, **kw) -> dict:
        base = {
            "conv": conv,
            "rects": RECTS,
            "mips": 9,
            "mips_bitexact": True,
            "verdict_sequence_equal": True,
            "occluded": 231,
            "false_positives": 0,
            "double_run_bitexact": True,
        }
        base.update(kw)
        return base

    good = {"state": "pass", "arms": [arm("reverse_z"), arm("standard_z")]}

    # facts 闭集 ≥7 且 schema 在树(extra_facts minItems 6 被满足)。
    expect(len(FACT_IDS) >= 7, f"facts 闭集 {len(FACT_IDS)} ≥ 7")
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    min_items = schema.get("properties", {}).get("extra_facts", {}).get("minItems", 99)
    expect(len(FACT_IDS) >= min_items, f"facts {len(FACT_IDS)} ≥ schema minItems {min_items}")
    # 红臂①:harness 态判读——skipped_dev_env/fail 不得判 pass。
    expect(_harness_state('{"state":"skipped_dev_env"}') != "pass", "RED:SKIP 态非 pass")
    expect(_harness_state('{"state":"fail"}') != "pass", "RED:fail 态非 pass")
    expect(_harness_state('{"state":"pass"}') == "pass", "GREEN:pass 态正例")
    # 红臂②:双臂布尔判读——单臂 false / 缺臂必拒。
    bad_one = {"state": "pass", "arms": [arm("reverse_z", mips_bitexact=False), arm("standard_z")]}
    expect(not _arms_field_all_true(bad_one, "mips_bitexact"), "RED:单臂 mips 非位级必拒")
    missing = {"state": "pass", "arms": [arm("reverse_z")]}
    expect(not _arms_field_all_true(missing, "verdict_sequence_equal"), "RED:缺 standard_z 臂必拒")
    expect(_arms_field_all_true(good, "mips_bitexact"), "GREEN:双臂 mips 位级正例")
    expect(_arms_field_all_true(good, "double_run_bitexact"), "GREEN:双臂双跑位级正例")
    # 红臂③:零假阳性+剔除数判读——fp>0 / occluded=0 必拒。
    fp_doc = {"arms": [arm("reverse_z", false_positives=1), arm("standard_z")]}
    expect(not _arms_zero_fp_cull_nonzero(fp_doc), "RED:假阳性 1 必拒")
    cull0 = {"arms": [arm("reverse_z", occluded=0), arm("standard_z")]}
    expect(not _arms_zero_fp_cull_nonzero(cull0), "RED:剔除数 0 必拒")
    expect(_arms_zero_fp_cull_nonzero(good), "GREEN:零假阳性+剔除非零正例")
    # 红臂④:rects 口径判读——非 800 必拒。
    r799 = {"arms": [arm("reverse_z", rects=799), arm("standard_z")]}
    expect(not _arms_rects_800(r799), "RED:rects=799 必拒")
    expect(_arms_rects_800(good), "GREEN:rects=800 正例")
    # 红臂⑤:tamper 判读——detected=false / SKIP 态 / rc≠0 必拒。
    expect(not _red_arm_detected('{"detected":false}', 0), "RED:tamper 漏检必拒")
    expect(
        not _red_arm_detected('{"detected":true,"state":"skipped_dev_env"}', 0),
        "RED:tamper SKIP 态冒充必拒",
    )
    expect(not _red_arm_detected('{"detected":true}', 1), "RED:tamper rc=1 必拒")
    expect(_red_arm_detected('{"detected":true,"detail":"x"}', 0), "GREEN:tamper 检出正例")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts={len(FACT_IDS)};5 红臂组 + 正例组)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=None)
    ap.add_argument("--verify-latest", action="store_true")
    ap.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        p = wel.load_latest_evidence(SUBJECT)
        return 0 if p and wel.load_json(p).get("host_section_pass") else 1
    if args.gate is not None and args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
