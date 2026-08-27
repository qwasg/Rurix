#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude（G35 GPU 粒子系统波 1 G35-1）
"""G35-1：GPU 粒子基元库门冒烟（g35.wave1.primitives；24 位键 3-pass 稳定
LSD radix sort + compact_u32——host 金标准 src/rurix-render/src/particles/
primitives.rs 与 device 七 kernel〔scan 三件 kernels/g35_scan_{seg_sum,spine,
seg_apply}.rx 冻结面只读消费 + sort 三件 kernels/g35_sort_{hist,spine,
scatter}.rx + kernels/g35_compact_u32.rx〕逐字同源；harness =
src/rurix-render/src/bin/g35_primitives_device.rs 经 vk::run_compute 真跑,
SEG=256 分段、线程=段、段内串行=确定序、禁原子抢槽、禁 shared memory,
纯整数域 device↔host 零容差位级——mod.rs G35-P v1 契约；Onesweep/
decoupled-lookback 评估窗登记不实现,RFC-0049 §9 Q1）。

八面判据（facts 闭集）：
1. **kernels_spv_valid**：rurixc 现编七件（scan 三 + sort 三 + compact）SPV
   + spirv-val 全绿；kernel 源与 SPV sha256 记 evidence（后续波漂移守护
   基线）。
2. **scan_bitexact**：scan 三 kernel 全链 device 输出与 host
   `scan::exclusive_scan_segmented` 逐规模**位级相等**（整数域零容差协议）。
3. **sort_bitexact**：sort 全链 device 输出与 host `sort_pairs_u24` 位级
   相等 AND 与独立参考 `sort_pairs_reference`（std 稳定 sort）互核相等
   （防同一错误两处照抄）。
4. **sort_stability**：同键 payload 保序（host 侧验证：payload = 原下标 ⇒
   稳定 ⇔ 同键段 payload 严格递增）+ 判据咬合前提（窗内重复键对 ≥ 1,
   固定 seed 夹具确定值）。
5. **compact_bitexact**：compact device 输出与 host `compact_u32` 位级相等
   （槽位 = 分段稳定 scan 推导,禁原子抢槽协议兑现面）。
6. **determinism_double_run**：device 全链双跑逐流 memcmp 逐规模位级一致
   （确定性门）。
7. **red_arm_effective**：--red-arm tamper 构造性注入（pass 0 spine 产 off
   缓冲某槽 +1）后输出必异于 host——判据非摆设的机器证明（honest 前置臂
   必须先绿；红臂命中 → probe 退 0 且 evidence 记 red_arm_effective=true）。
8. **throughput_measured**：keys/秒 measured_local 诚实登记**不设通过线**
   （probe 车道逐 dispatch 建/毁 instance+device 口径如实标注）。

三态：无 Vulkan loader/设备/SDK（spirv-val 缺）→ DEV_ENV_DEGRADE 退 0
（不冒充 PASS）；RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL（禁 mock
充真跑）。

evidence 纪律：门裁决件落 evidence/g35_primitives_gate_<ts>.json
（check_schemas 前缀路由 g35_primitives_ 仅门裁决件）；probe 真跑件
（rurix.g35.primitives_probe.v1）留 .tmp/g35_gates/ 工作区不注册
check_schemas,数字经门裁决件蒸馏登记。

用法：
  py -3 ci/g35_primitives_smoke.py --selftest
  py -3 ci/g35_primitives_smoke.py --gate g35.wave1.primitives [--scale 65536]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g35.wave1.primitives"
SUBJECT = "g35_primitives"
WAVE = "G35.1"
TAG = "g35_primitives"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_primitives_gate_evidence_schema.json"
SCHEMA_ID = "rurix.g35.primitives_gate_evidence.v1"
# probe 真跑件 schema 字面（.tmp 工作区件——不注册 check_schemas，无 schema 文件）。
PROBE_SCHEMA_ID = "rurix.g35.primitives_probe.v1"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
WORK = ROOT / ".tmp" / "g35_gates"
# 七 kernel 现编面（源 → WORK 内 SPV；scan 三件 = G35-P 冻结面只读消费,
# sort 三件 + compact = G35-1 本波交付）。三元组 = (evidence 键, 源名, SPV 名)。
KERNEL_SPECS = (
    ("scan_seg_sum", "g35_scan_seg_sum.rx", "g35_scan_seg_sum.spv"),
    ("scan_spine", "g35_scan_spine.rx", "g35_scan_spine.spv"),
    ("scan_seg_apply", "g35_scan_seg_apply.rx", "g35_scan_seg_apply.spv"),
    ("sort_hist", "g35_sort_hist.rx", "g35_sort_hist.spv"),
    ("sort_spine", "g35_sort_spine.rx", "g35_sort_spine.spv"),
    ("sort_scatter", "g35_sort_scatter.rx", "g35_sort_scatter.spv"),
    ("compact_u32", "g35_compact_u32.rx", "g35_compact_u32.spv"),
)
# probe bin SPV 旗标（KERNEL_SPECS 同序;harness parse_args 字面）。
SPV_FLAGS = (
    "--spv-scan-seg-sum",
    "--spv-scan-spine",
    "--spv-scan-seg-apply",
    "--spv-sort-hist",
    "--spv-sort-spine",
    "--spv-sort-scatter",
    "--spv-compact",
)
GREEN_EVIDENCE = WORK / "probe_green.json"
RED_EVIDENCE = WORK / "probe_red.json"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN_PROBE = ROOT / "target" / "debug" / f"g35_primitives_device{EXE_SUFFIX}"
SCALE_DEFAULT = 65536
SCALE_MAX = 1048576  # PARTICLE_CAP_MAX = SEG × NSEG_MAX（mod.rs 契约）

FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "scan_bitexact",
    "sort_bitexact",
    "sort_stability",
    "compact_bitexact",
    "determinism_double_run",
    "red_arm_effective",
    "throughput_measured",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


# ---------------------------------------------------------------------------
# 判读器（selftest 红绿两臂消费面；纯函数零 GPU 依赖）
# ---------------------------------------------------------------------------


def rows_all_true(rows: list[dict], key: str) -> bool:
    """逐规模布尔全真判（行非空 + 字段严格 is True——字符串/1 冒充必红）。"""
    return len(rows) > 0 and all(r.get(key) is True for r in rows)


def scan_bitexact_ok(rows: list[dict]) -> bool:
    """② scan 位级判：全规模 scan_bitexact == true。"""
    return rows_all_true(rows, "scan_bitexact")


def sort_bitexact_ok(rows: list[dict]) -> bool:
    """③ sort 位级判：全规模 host 位级 AND 独立参考互核（两臂合取,防
    同一错误两处照抄——单臂真不许绿）。"""
    return rows_all_true(rows, "sort_bitexact") and rows_all_true(rows, "sort_reference_equal")


def sort_stability_ok(rows: list[dict]) -> bool:
    """④ 稳定性判：全规模同键 payload 保序 + 窗内重复键对 ≥ 1（咬合前提——
    零重复键 = 判据空转必红）。"""
    if not rows_all_true(rows, "sort_stable"):
        return False
    dups = [r.get("duplicate_pairs") for r in rows]
    if not all(isinstance(d, int) and not isinstance(d, bool) and d >= 0 for d in dups):
        return False
    return max(dups) >= 1


def compact_bitexact_ok(rows: list[dict]) -> bool:
    """⑤ compact 位级判：全规模 compact_bitexact == true。"""
    return rows_all_true(rows, "compact_bitexact")


def determinism_ok(rows: list[dict]) -> bool:
    """⑥ 确定性判：全规模 device 双跑逐流 memcmp 位级一致。"""
    return rows_all_true(rows, "double_run_bitexact")


def red_arm_ok(doc: dict) -> bool:
    """⑦ RED 臂判：red-arm 臂 probe 状态 pass 且 red_arm_effective 严格 true。"""
    return (
        doc.get("status") == "pass"
        and doc.get("mode") == "red-arm"
        and doc.get("red_arm_effective") is True
    )


def throughput_ok(tp: dict) -> bool:
    """⑧ throughput 登记面健全判：keys_per_sec 有限正数 + measured 口径字
    符串非空（诚实登记非阈门）。"""
    kps = tp.get("keys_per_sec")
    if not isinstance(kps, (int, float)) or isinstance(kps, bool) or kps != kps or not kps > 0:
        return False
    return isinstance(tp.get("measured"), str) and len(tp["measured"]) > 0


def probe_doc_sane(doc: dict, mode: str) -> bool:
    """probe evidence 形态判（schema/gate/mode/status 四面互核）。"""
    return (
        doc.get("schema") == PROBE_SCHEMA_ID
        and doc.get("gate") == GATE_KEY
        and doc.get("mode") == mode
        and doc.get("status") == "pass"
    )


def build_gate_doc(
    fact_rows: list[dict],
    all_pass: bool,
    kernels_block: dict,
    green_doc: dict,
    red_doc: dict,
    env_info: dict,
    ts: str,
) -> dict:
    """门裁决件构造（--gate 与 --selftest 合成正例共用——selftest 校验的
    即真实构造器,防 schema 与产件漂移）。"""
    rows = green_doc.get("scales") or []
    tp = green_doc.get("throughput") or {}
    return {
        "schema": SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": kernels_block,
        "determinism": {
            "double_run_bitexact": determinism_ok(rows),
            "scales": [int(r.get("n", 0)) for r in rows],
        },
        "throughput": {
            "keys_per_sec": tp.get("keys_per_sec", -1.0),
            "sort_n": int(tp.get("sort_n", 0)),
            "sort_ms": tp.get("sort_ms", -1.0),
            "measured": tp.get("measured", ""),
        },
        "probe": {
            "green_evidence": str(GREEN_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
            "red_evidence": str(RED_EVIDENCE.relative_to(ROOT)).replace("\\", "/"),
            "scales": [int(r.get("n", 0)) for r in rows],
            "stability_duplicate_pairs_max": int(green_doc.get("stability_duplicate_pairs_max", -1)),
            "red_arm_detail": red_doc.get("red_arm_detail", ""),
        },
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-1 GPU 粒子基元库：24 位键 3-pass 稳定 LSD radix sort（段直方图 g35_sort_hist → "
            "digit-major 单串行 spine g35_sort_spine → 段内串行稳定散射 g35_sort_scatter,pass 间 "
            "ping-pong = host 侧缓冲交换）+ compact_u32（槽位 = 分段稳定 scan 三 kernel 推导,禁原子"
            "抢槽）。host 金标准 particles/primitives.rs 与 device 七 kernel 逐字同源,纯 u32/usize "
            "整数算术零浮点 ⇒ device↔host 零容差位级（mod.rs G35-P v1 整数域协议）；SEG=256 分段、"
            "线程=段、段内串行=确定序,无 shared memory / 无原子 / 无 lookback（Vulkan 前进保证缺位,"
            "保守分段臂 = 生产形态；Onesweep/decoupled-lookback 评估窗登记不实现,RFC-0049 §9 Q1）。"
            "判据 = 规模闭集 {256, 4096, 65536, --scale} 逐档:scan/sort/compact 位级 + 独立参考互核"
            "（std 稳定 sort / iter filter,防同一错误两处照抄）+ 同键 payload 保序（咬合前提:窗内"
            "重复键对 ≥ 1,固定 seed 夹具确定值）+ device 全链双跑逐流 memcmp + tamper RED 臂"
            "（off 缓冲某槽 +1 构造性注入,散射双射保多重集 ⇒ 末端必异,honest 前置臂先绿）+ "
            "throughput measured_local 诚实登记不设通过线（probe 车道逐 dispatch 建/毁 instance+"
            "device 口径如实标注,非生产 DeviceFrameSession 口径）。scan 三 kernel 与 scan.rs/mod.rs "
            "为 G35-P 冻结面只读消费 0-byte 不动。"
        ),
    }


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def load_json(path: Path) -> dict | None:
    if not path.is_file():
        return None
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except json.JSONDecodeError:
        return None


def run_gate(scale: int) -> int:
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行（前置失败）"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:180]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1

    # ── 构建（rurixc + harness；子进程 env 继承）──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan",
         "--bin", "g35_primitives_device", "--quiet"],
        "g35_primitives_device",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面：rurixc 现编七件 + spirv-val（kernel 源 sha256 记 evidence）──
    WORK.mkdir(parents=True, exist_ok=True)
    degrade: list[str] = []
    if shutil.which("spirv-val") is None:
        degrade.append("spirv-val 不可用（Vulkan SDK 缺）")
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    kernel_entries: dict[str, dict] = {}
    for key, src_name, dst_name in KERNEL_SPECS:
        src = KERNEL_DIR / src_name
        dst = WORK / dst_name
        entry = {
            "path": str(dst.relative_to(ROOT)).replace("\\", "/"),
            "sha256": "sha256:" + "0" * 64,
            "source": str(src.relative_to(ROOT)).replace("\\", "/"),
            "source_sha256": "sha256:" + "0" * 64,
        }
        kernel_entries[key] = entry
        if not src.is_file():
            spv_ok = False
            note(f"kernel 源缺失: {src}")
            continue
        entry["source_sha256"] = "sha256:" + hashlib.sha256(src.read_bytes()).hexdigest()
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        entry["sha256"] = "sha256:" + hashlib.sha256(dst.read_bytes()).hexdigest()
        if not degrade:  # spirv-val 在 PATH 才可核（缺 = dev-env 降级面）
            val = run(["spirv-val", str(dst)], timeout=600)
            if val.returncode != 0:
                spv_ok = False
                note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    set_fact(
        "kernels_spv_valid",
        spv_ok and not degrade,
        f"rurixc 现编七件（scan 三 g35_scan_seg_sum/spine/seg_apply〔冻结面只读消费〕+ sort 三 "
        f"g35_sort_hist/spine/scatter + g35_compact_u32）+ spirv-val={'全绿' if spv_ok else '红'}"
        f"{'（spirv-val 缺 = dev-env 降级）' if degrade else ''}；kernel 源/SPV sha256 在档",
    )
    if not spv_ok:
        fail("G35-1 kernel SPV 编译/spirv-val 未过（本波交付面红,非 dev-env 降级）")

    # ── probe 真跑（绿臂 + red-arm 臂;ci.gpu_device_lock 独占窗）──
    green_doc: dict | None = None
    red_doc: dict | None = None
    if not degrade and spv_ok:
        spv_args: list[str] = []
        for (_key, _src, dst_name), flag in zip(KERNEL_SPECS, SPV_FLAGS):
            spv_args += [flag, str(WORK / dst_name)]
        env = dict(os.environ)
        with gpu_device_lock(purpose=f"{TAG} probe 真跑（绿臂 + red-arm 臂）"):
            g = run(
                [str(BIN_PROBE), *spv_args, "--scale", str(scale),
                 "--evidence-out", str(GREEN_EVIDENCE)],
                timeout=3600, env=env,
            )
            gout = (g.stdout or "") + (g.stderr or "")
            if "skipped_dev_env" in gout:
                degrade.append(f"probe skipped_dev_env: {gout.strip()[-200:]}")
            else:
                if g.returncode != 0:
                    fail(f"绿臂真跑退 {g.returncode}: {gout[-300:]}")
                green_doc = load_json(GREEN_EVIDENCE)
                if green_doc is None:
                    fail(f"绿臂 evidence 缺失/非法: {GREEN_EVIDENCE}")
                r = run(
                    [str(BIN_PROBE), *spv_args, "--red-arm", "tamper",
                     "--evidence-out", str(RED_EVIDENCE)],
                    timeout=3600, env=env,
                )
                rout = (r.stdout or "") + (r.stderr or "")
                if "skipped_dev_env" in rout:
                    degrade.append(f"red-arm skipped_dev_env: {rout.strip()[-200:]}")
                else:
                    if r.returncode != 0:
                        fail(f"red-arm 臂退 {r.returncode}（漏检即红）: {rout[-300:]}")
                    red_doc = load_json(RED_EVIDENCE)
                    if red_doc is None:
                        fail(f"red-arm evidence 缺失/非法: {RED_EVIDENCE}")

    if degrade:
        doc = {
            "schema": "rurix.g35.primitives.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for dg in degrade:
            note(f"DEV_ENV_DEGRADE {dg}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE（三态之 SKIP,非 PASS 非 FAIL）")
        return 0

    # ── ②~⑧ 逐 fact 判定（probe evidence 蒸馏;判读器 = selftest 同一纯函数面）──
    if green_doc is not None:
        sane = probe_doc_sane(green_doc, "device")
        if not sane:
            fail(f"绿臂 evidence 形态破（schema/gate/mode/status 互核）: {str(green_doc)[:200]}")
        rows = green_doc.get("scales") or []
        ns = [int(r.get("n", 0)) for r in rows]
        set_fact(
            "scan_bitexact",
            sane and scan_bitexact_ok(rows),
            f"② scan 三 kernel 全链 vs host exclusive_scan_segmented 逐规模位级（scales={ns}；"
            "整数域零容差协议）",
        )
        set_fact(
            "sort_bitexact",
            sane and sort_bitexact_ok(rows),
            f"③ sort 全链 vs host sort_pairs_u24 位级 AND vs 独立参考 sort_pairs_reference 互核"
            f"（scales={ns}；防同一错误两处照抄）",
        )
        dup_max = max((int(r.get("duplicate_pairs", 0)) for r in rows), default=0)
        set_fact(
            "sort_stability",
            sane and sort_stability_ok(rows),
            f"④ 同键 payload 保序全规模 + 咬合前提窗内重复键对 max={dup_max} ≥ 1"
            "（payload = 原下标,host 侧验证函数）",
        )
        set_fact(
            "compact_bitexact",
            sane and compact_bitexact_ok(rows),
            f"⑤ compact vs host compact_u32 逐规模位级（scales={ns}；槽位 = 分段稳定 scan 推导）",
        )
        set_fact(
            "determinism_double_run",
            sane and determinism_ok(rows),
            f"⑥ device 全链双跑逐流 memcmp 逐规模位级一致（scales={ns}；确定性门）",
        )
        tp = green_doc.get("throughput") or {}
        set_fact(
            "throughput_measured",
            sane and throughput_ok(tp),
            f"⑧ throughput measured 登记:keys_per_sec={tp.get('keys_per_sec')!r} "
            f"sort_n={tp.get('sort_n')!r} sort_ms={tp.get('sort_ms')!r}"
            "（诚实登记不设通过线;probe 车道口径如实标注）",
        )
    if red_doc is not None:
        set_fact(
            "red_arm_effective",
            red_arm_ok(red_doc),
            f"⑦ tamper RED 臂:off 缓冲某槽 +1 构造性注入检出 — {red_doc.get('red_arm_detail', '')!r}"
            "（honest 前置臂先绿;判据非摆设机器证明）",
        )

    # ── evidence 落盘（门裁决件;jsonschema 自校验硬门）──
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti（本机单卡 measured_local）",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    kernels_block = dict(kernel_entries)
    kernels_block["spirv_val_all"] = bool(facts["kernels_spv_valid"]["status"] == "PASS")
    gate_doc = build_gate_doc(
        fact_rows, all_pass, kernels_block, green_doc or {}, red_doc or {}, env_info, ts
    )
    import jsonschema  # 自校验硬门（schema 漂移即 RED）

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
    gate_path = ROOT / "evidence" / f"g35_primitives_gate_{ts}.json"
    gate_path.parent.mkdir(parents=True, exist_ok=True)
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_path.relative_to(ROOT)}（probe 真跑件留 {WORK.relative_to(ROOT)}）")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest（判读器红绿两臂穷举,零 GPU/零构建依赖）
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

    good_row = {
        "n": 65536, "nseg": 256,
        "scan_bitexact": True, "sort_bitexact": True, "sort_reference_equal": True,
        "sort_stable": True, "duplicate_pairs": 106,
        "compact_bitexact": True, "compact_kept": 33188,
        "double_run_bitexact": True, "sort_ms": 547.0,
    }
    small_row = {**good_row, "n": 256, "nseg": 1, "duplicate_pairs": 0}

    # 红绿臂②:scan 位级判。
    expect(scan_bitexact_ok([small_row, good_row]), "GREEN:② scan 全规模位级正例")
    expect(not scan_bitexact_ok([good_row, {**good_row, "scan_bitexact": False}]),
           "RED:② 任一规模 scan 非位级必红")
    expect(not scan_bitexact_ok([]), "RED:② 空规模列必红")
    expect(not scan_bitexact_ok([{**good_row, "scan_bitexact": "true"}]),
           "RED:② 字符串冒充布尔必红")
    # 红绿臂③:sort 位级 + 互核双臂合取判。
    expect(sort_bitexact_ok([small_row, good_row]), "GREEN:③ sort 双臂正例")
    expect(not sort_bitexact_ok([{**good_row, "sort_bitexact": False}]),
           "RED:③ host 位级破必红")
    expect(not sort_bitexact_ok([{**good_row, "sort_reference_equal": False}]),
           "RED:③ 独立参考互核破必红（host 位级真亦不许绿）")
    expect(not sort_bitexact_ok([]), "RED:③ 空规模列必红")
    # 红绿臂④:稳定性 + 咬合前提判。
    expect(sort_stability_ok([small_row, good_row]), "GREEN:④ 保序 + 重复键对 ≥1 正例")
    expect(not sort_stability_ok([{**good_row, "sort_stable": False}]),
           "RED:④ 同键 payload 逆序必红")
    expect(not sort_stability_ok([small_row, {**good_row, "duplicate_pairs": 0}]),
           "RED:④ 全规模零重复键对（判据空转）必红")
    expect(not sort_stability_ok([{**good_row, "duplicate_pairs": True}]),
           "RED:④ 布尔冒充计数必红")
    expect(not sort_stability_ok([]), "RED:④ 空规模列必红")
    # 红绿臂⑤:compact 位级判。
    expect(compact_bitexact_ok([small_row, good_row]), "GREEN:⑤ compact 正例")
    expect(not compact_bitexact_ok([{**good_row, "compact_bitexact": False}]),
           "RED:⑤ compact 非位级必红")
    expect(not compact_bitexact_ok([]), "RED:⑤ 空规模列必红")
    # 红绿臂⑥:确定性双跑判。
    expect(determinism_ok([small_row, good_row]), "GREEN:⑥ 双跑位级正例")
    expect(not determinism_ok([{**good_row, "double_run_bitexact": False}]),
           "RED:⑥ 双跑漂移必红")
    expect(not determinism_ok([]), "RED:⑥ 空规模列必红")
    # 红绿臂⑦:RED 臂判。
    good_red = {"schema": PROBE_SCHEMA_ID, "status": "pass", "mode": "red-arm",
                "gate": GATE_KEY, "red_arm_effective": True,
                "red_arm_detail": "off[16] +1 注入检出:keys 异 812 槽"}
    expect(red_arm_ok(good_red), "GREEN:⑦ RED 臂命中正例")
    expect(not red_arm_ok({**good_red, "red_arm_effective": False}),
           "RED:⑦ 注入漏检必红")
    expect(not red_arm_ok({**good_red, "status": "fail"}), "RED:⑦ 臂状态 fail 必红")
    expect(not red_arm_ok({**good_red, "mode": "device"}), "RED:⑦ 臂模式冒充必红")
    expect(not red_arm_ok({}), "RED:⑦ 空件必红")
    # 红绿臂⑧:throughput 登记面健全判。
    good_tp = {"keys_per_sec": 119814.9, "sort_n": 65536, "sort_ms": 547.0,
               "measured": "measured_local(probe 车道)"}
    expect(throughput_ok(good_tp), "GREEN:⑧ throughput 正例")
    expect(not throughput_ok({**good_tp, "keys_per_sec": 0.0}), "RED:⑧ 0 keys/s 必红")
    expect(not throughput_ok({**good_tp, "keys_per_sec": float("nan")}), "RED:⑧ NaN 必红")
    expect(not throughput_ok({**good_tp, "measured": ""}), "RED:⑧ 口径字符串空必红")
    expect(not throughput_ok({}), "RED:⑧ 空件必红")
    # probe 形态互核判。
    good_green = {
        "schema": PROBE_SCHEMA_ID, "status": "pass", "mode": "device", "gate": GATE_KEY,
        "scales": [small_row, good_row], "stability_duplicate_pairs_max": 106,
        "throughput": good_tp, "red_arm_effective": None,
    }
    expect(probe_doc_sane(good_green, "device"), "GREEN:probe 形态正例")
    expect(not probe_doc_sane({**good_green, "schema": "rurix.other.v1"}, "device"),
           "RED:probe schema 漂移必红")
    expect(not probe_doc_sane({**good_green, "gate": "g35.other"}, "device"),
           "RED:probe gate 键漂移必红")
    expect(not probe_doc_sane({**good_green, "status": "fail"}, "device"),
           "RED:probe status fail 必红")
    # schema 互核:在树 + Draft7 合法 + facts enum == FACT_IDS + 闭集基数。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        import jsonschema as _js

        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法（check_schema 绿）")
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(enum == FACT_IDS, f"gate schema facts enum == FACT_IDS（{len(FACT_IDS)},逐位同序）")
        expect(
            gs["properties"]["facts"]["minItems"] == 8 and gs["properties"]["facts"]["maxItems"] == 8,
            "gate schema facts minItems == maxItems == 8",
        )
        expect(gs["properties"]["schema"]["const"] == SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate schema 门键 const 互核")
        # 合成正例经真实构造器过 schema（绿臂）+ 破形件必拒（红臂）。
        fact_rows = [{"id": fid, "status": "PASS", "detail": "selftest 合成正例"} for fid in FACT_IDS]
        kernels_block = {
            key: {
                "path": f".tmp/g35_gates/{dst}",
                "sha256": "sha256:" + "0" * 64,
                "source": f"src/rurix-render/kernels/{src}",
                "source_sha256": "sha256:" + "0" * 64,
            }
            for key, src, dst in KERNEL_SPECS
        }
        kernels_block["spirv_val_all"] = True
        doc = build_gate_doc(
            fact_rows, True, kernels_block, good_green, good_red,
            {"gpu": "selftest", "os": "windows"}, "19700101T000000Z",
        )
        v = _js.Draft7Validator(gs)
        expect(not list(v.iter_errors(doc)), "GREEN:真实构造器合成正例过 gate schema（0 err）")
        expect(bool(list(v.iter_errors({**doc, "facts": doc["facts"][:7]}))),
               "RED:facts 7 项（闭集破）schema 必拒")
        expect(bool(list(v.iter_errors({**doc, "verdict": "MAYBE"}))),
               "RED:verdict 非 PASS/FAIL schema 必拒")
        expect(bool(list(v.iter_errors({**doc, "extra_key": 1}))),
               "RED:additionalProperties schema 必拒")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS（facts=8；判读器红绿两臂穷举 + schema Draft7 双臂 + enum 互核）")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--scale", type=int, default=SCALE_DEFAULT)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}（闭集 {GATE_KEY}）", file=sys.stderr)
            return 1
        if not 1 <= args.scale <= SCALE_MAX:
            print(f"[{TAG}] FAIL: --scale {args.scale} 越域（1 ..= {SCALE_MAX}）", file=sys.stderr)
            return 1
        return run_gate(args.scale)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
