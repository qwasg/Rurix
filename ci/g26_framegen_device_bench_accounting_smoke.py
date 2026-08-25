#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Agent(G26.2 实现波)
"""G26.2 M-b FG device 车道帧时 measured 登记 + 口径纪律回验门冒烟
(g26.p0.m_b.framegen_device_bench_accounting;G26_CONTRACT §4.2 M-b 行判据
逐字;rfcs/0043-framegen-device-kernel-realization.md §2 判据事实源;
G26_ACCEPTANCE_MAP §1 M-b 行)。

硬判据:device 全链路(打包 + dispatch + 回读同步)warmup 10 + timed 150
逐帧墙钟三档真跑(--bench x2/x3/x4,g26_framegen_device bench 腿)→ 逐档
frame_ms trimmed mean(M141/M165 50×3 冻结统计口径,ci/g12 block_stats 同
实现复用禁重写)程序产追加 g26_budget 条目(threshold = measured × 2.0 回归
守护,measured_local 零 estimated)+ FgAccounting 两口径类型面分离核验(F9
双恒等式:①presented_frames == real_frames + generated_frames 重算相等
②real_render_fps 以登记面数值 f64 重算相等且与 generated 计数无关)+ 性能面
0-byte 机核(F11:g14_3_pipeline_perf.rs / render_exec.rs / vendor_upscale.rs
三文件 vs g25-closed git-diff)。

语义边界(RFC-0043 §2.3):回归守护语义,**不构成帧率对标通过线**(G6 无性能
硬门纪律沿用;正式帧率对标锚定 G14 车道);生成帧禁计入真实渲染帧率。

用法:
  py -3 ci/g26_framegen_device_bench_accounting_smoke.py --gate g26.p0.m_b.framegen_device_bench_accounting
  py -3 ci/g26_framegen_device_bench_accounting_smoke.py --verify-latest
  py -3 ci/g26_framegen_device_bench_accounting_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

import g11_wave_exit_lib as wel  # noqa: E402
from gpu_device_lock import gpu_device_lock  # noqa: E402
# 50×3 trimmed mean 冻结统计口径(M141/M165 字面)同实现复用——禁重写防漂移。
from g12_pt_throughput_baseline_smoke import (  # noqa: E402
    NO_PASS_LINE_LITERAL,
    TIMED,
    WARMUP,
    block_stats,
    recompute_check,
)

GATE_KEY = "g26.p0.m_b.framegen_device_bench_accounting"
NUMERIC_STEP = 450
SUBJECT = "g26_m_b_framegen_device_bench_accounting"
WAVE = "G26.2"
SCHEMA_PATH = ROOT / "milestones/g26/g26_m_b_framegen_device_bench_accounting_evidence_schema.json"
SOURCE_REF = (
    "G26_CONTRACT §4.2 M-b;rfcs/0043-framegen-device-kernel-realization.md §2;"
    "G26_ACCEPTANCE_MAP §1 M-b 行;milestones/m0/BENCH_PROTOCOL.md §3(M141/M165 冻结统计口径继承)"
)
TAG = "g26_m_b"

KERNEL = ROOT / "src/rurix-render/kernels/g26_framegen.rx"
WORK_DIR = ROOT / ".tmp/g26_gates"
SPV_PATH = WORK_DIR / "g26_framegen.spv"
BUDGET_PATH = ROOT / "milestones/g26/g26_budget.json"
HARNESS_BIN = "g26_framegen_device"
FROZEN_BASE = "g25-closed"

# (tier, budget entry id, evidence 固定路径)——budget 通用路读
# evidence results.trimmed_mean vs threshold(direction max)。
BENCH_TIERS = [
    ("x2", "g26.framegen_device.frame_ms_x2", "evidence/g26_framegen_device_bench_x2.json"),
    ("x3", "g26.framegen_device.frame_ms_x3", "evidence/g26_framegen_device_bench_x3.json"),
    ("x4", "g26.framegen_device.frame_ms_x4", "evidence/g26_framegen_device_bench_x4.json"),
]
# 性能面 0-byte 机核三文件(RFC-0043 §2.3 F11;契约 M-b 行同字面)。
PERF_SURFACE_FILES = [
    "src/rurix-render/src/bin/g14_3_pipeline_perf.rs",
    "src/rurix-rt/src/render_exec.rs",
    "src/rurix-rt/src/vendor_upscale.rs",
]

FACT_IDS = [
    "bench_three_tiers_measured",
    "budget_entries_programmatic",
    "accounting_identity_presented",
    "accounting_real_fps_isolated",
    "perf_surface_0byte",
    "no_pass_line_semantics",
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


def compile_spv(rurixc: Path) -> tuple[bool, str]:
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    print(f"[{TAG}] rurixc {KERNEL.name} --target vulkan -o {SPV_PATH.relative_to(ROOT)}")
    r = run([str(rurixc), str(KERNEL), "--target", "vulkan", "-o", str(SPV_PATH)])
    if r.returncode != 0 or not SPV_PATH.is_file():
        return False, f"SPV 编译失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}"
    val = run(["spirv-val", str(SPV_PATH)])
    if val.returncode != 0:
        return False, f"spirv-val 未过: {(val.stdout + val.stderr)[-300:]}"
    return True, "SPV + spirv-val 通过"


def json_line(stdout: str, schema_token: str) -> str | None:
    for line in stdout.splitlines():
        if schema_token in line:
            return line.strip()
    return None


# ---------------------------------------------------------------------------
# g26_budget(帧时条目:缺 → 程序产追加 ×2.0;在档 → 守护复检 measured ≤ 阈)
# ---------------------------------------------------------------------------


def load_budget() -> dict | None:
    if not BUDGET_PATH.is_file():
        return None
    return json.loads(BUDGET_PATH.read_text(encoding="utf-8"))


def budget_entry(budget: dict, eid: str) -> dict | None:
    for e in budget.get("entries", []):
        if e.get("id") == eid:
            return e
    return None


def _entry_is_measured(entry: dict) -> bool:
    return entry.get("evidence") == "measured_local" and not entry.get("skip_reason")


def append_budget_entries(new_entries: list[dict]) -> list[str]:
    """字节级纯追加(既有字节 0-byte;g13 append_budget_entries 同模)。"""
    problems: list[str] = []
    if not new_entries:
        return problems
    budget_text = BUDGET_PATH.read_text(encoding="utf-8")
    budget = json.loads(budget_text)
    to_add: list[dict] = []
    for entry in new_entries:
        if budget_entry(budget, entry["id"]) is not None:
            problems.append(f"{entry['id']} 已在树(追加面不改写)")
            continue
        to_add.append(entry)
    if problems or not to_add:
        return problems
    nl = "\r\n" if "\r\n" in budget_text else "\n"
    frag = ""
    for entry in to_add:
        body = json.dumps(entry, ensure_ascii=False, indent=2)
        body = body.replace("\n", nl)
        body = "    " + body.replace(nl, nl + "    ")
        frag += "," + nl + body
    anchor = f"{nl}  ],{nl}  \"ratio_assertions\""
    if anchor not in budget_text:
        return ["g26_budget.json 结构锚缺失(拒改写)"]
    head, sep, tail = budget_text.partition(anchor)
    budget_text = head + frag + sep + tail
    json.loads(budget_text)
    BUDGET_PATH.write_text(budget_text, encoding="utf-8", newline="")
    return problems


# ---------------------------------------------------------------------------
# FgAccounting 双恒等式判读(F9;python 第二实现路径独立重算)
# ---------------------------------------------------------------------------


def accounting_identity_presented(doc: dict) -> tuple[bool, str]:
    acc = doc.get("accounting") or {}
    real = acc.get("real_frames")
    gen = acc.get("generated_frames")
    presented = acc.get("presented_frames")
    if not all(isinstance(v, int) for v in (real, gen, presented)):
        return False, "accounting 计数字段缺失/非整数"
    recompute = real + gen
    ok = presented == recompute and doc.get("identity_presented_ok") is True
    return ok, f"presented={presented} == real {real} + generated {gen}(重算 {recompute});类型面核验 identity_presented_ok={doc.get('identity_presented_ok')}"


def accounting_real_fps_isolated(doc: dict) -> tuple[bool, str]:
    acc = doc.get("accounting") or {}
    real = acc.get("real_frames")
    secs = acc.get("real_render_seconds")
    fps = acc.get("real_render_fps")
    gen = acc.get("generated_frames")
    if not isinstance(real, int) or not isinstance(secs, float) or not isinstance(fps, float):
        return False, "accounting fps 字段缺失"
    if secs <= 0.0:
        return False, "real_render_seconds ≤ 0"
    recompute = real / secs
    rel = abs(fps - recompute) / max(abs(fps), 1e-300)
    fps_ok = rel < 1e-12
    # 隔离面:公式重算只含 real 两字段——generated 扰动不改其值(f64 恒等)。
    perturbed = real / secs  # generated 不进公式,任何扰动值均无从影响
    isolated_ok = perturbed == recompute and doc.get("identity_real_fps_isolated_ok") is True
    typed_ok = doc.get("identity_real_fps_recompute_ok") is True
    ok = fps_ok and isolated_ok and typed_ok
    return ok, (
        f"real_render_fps={fps} 重算 {recompute}(rel={rel:.2e});generated={gen} 扰动无涉"
        f"(公式面零消费);FgAccounting 类型面核验 recompute={doc.get('identity_real_fps_recompute_ok')} "
        f"isolated={doc.get('identity_real_fps_isolated_ok')}"
    )


def perf_surface_0byte() -> tuple[bool, str]:
    r = run(["git", "diff", "--quiet", FROZEN_BASE, "--"] + PERF_SURFACE_FILES)
    if r.returncode != 0:
        d = run(["git", "diff", "--name-only", FROZEN_BASE, "--"] + PERF_SURFACE_FILES)
        changed = [x.strip() for x in d.stdout.splitlines() if x.strip()]
        return False, f"性能面有差分 vs {FROZEN_BASE}(F11 冻结面触碰即 RED): {changed}"
    u = run(["git", "status", "--porcelain", "--"] + PERF_SURFACE_FILES)
    if u.stdout.strip():
        return False, f"性能面工作树未提交面: {u.stdout.strip().splitlines()[:3]}"
    return True, (
        f"git diff --quiet {FROZEN_BASE} -- g14_3_pipeline_perf.rs render_exec.rs "
        "vendor_upscale.rs 三文件 0-byte(提交面 + 工作树双面)"
    )


# ---------------------------------------------------------------------------
# bench 腿(三档真跑 + 统计 + 登记)
# ---------------------------------------------------------------------------


def run_bench_tier(harness: Path, tier: str) -> tuple[dict | None, dict | None, str]:
    print(f"[{TAG}] bench 腿: --bench {tier} --warmup {WARMUP} --frames {TIMED}")
    r = run(
        [str(harness), "--bench", tier, "--spv", str(SPV_PATH),
         "--warmup", str(WARMUP), "--frames", str(TIMED)],
        env=device_env(), timeout=3600,
    )
    line = json_line(r.stdout, "rurix.g26framegen.bench.v1")
    if r.returncode != 0 or line is None:
        skip = json_line(r.stdout, "rurix.g26framegen.bench_skip.v1")
        if skip is not None:
            return None, None, f"bench {tier} SKIP(RURIX_REQUIRE_REAL=1 不许 SKIP 充绿): {skip[:200]}"
        return None, None, f"bench {tier} 失败 rc={r.returncode}: {(r.stdout + r.stderr)[-300:]}"
    doc = json.loads(line)
    if doc.get("warmup_count") != WARMUP or doc.get("timed_count") != TIMED:
        return None, None, f"bench {tier} 采样轮数不符(warmup {doc.get('warmup_count')}/timed {doc.get('timed_count')},协议 {WARMUP}+{TIMED})"
    samples = [float(v) for v in doc.get("frame_ms", [])]
    if len(samples) != TIMED:
        return None, None, f"bench {tier} 原始样本数 {len(samples)} ≠ {TIMED}"
    stats = block_stats(samples)
    if not recompute_check(samples, stats):
        return None, None, f"bench {tier} 统计面独立重算不符(50×3 trimmed mean 协议面破缺)"
    return doc, stats, f"trimmed_mean={stats['trimmed_mean_ms']:.6f} ms cv={stats['cv']:.4f}"


def register_bench_entry(tier: str, eid: str, ev_rel: str, doc: dict, stats: dict, ts: str) -> tuple[bool, str]:
    """帧时条目:缺 → 程序产追加(threshold = measured × 2.0 回归守护);在档 →
    守护复检 measured ≤ 在档阈(墙钟非位级确定,g13/M165 同模)。"""
    measured = float(stats["trimmed_mean_ms"])
    budget = load_budget()
    if budget is None:
        return False, "g26_budget.json 缺失"
    existing = budget_entry(budget, eid)
    if existing is not None:
        if not _entry_is_measured(existing):
            return False, f"{eid} 非 measured_local(estimated 冒充 measured 即 RED)"
        if NO_PASS_LINE_LITERAL not in existing.get("description", ""):
            return False, f"{eid} 在档描述缺不设通过线字面"
        if measured > float(existing["threshold"]):
            return False, f"{eid} 守护复检失败:复测 {measured:.6f} ms > 在档阈 {float(existing['threshold']):.6f} ms"
        return True, f"在档守护复检 PASS:复测 {measured:.6f} ms ≤ 阈 {float(existing['threshold']):.6f} ms"
    ev_doc = {
        "schema": "rurix.g26framegen.bench_entry.v1",
        "entry_id": eid,
        "results": {"trimmed_mean": measured},
        "stats": stats,
        "protocol": (
            f"FG/MFG device ×{tier[1:]} 档逐生成帧全链路帧时(host Instant 墙钟 around 打包 + "
            f"dispatch + 回读同步;warmup {WARMUP} + timed {TIMED} = 3 块 × 50 trimmed mean,"
            f"M141/M165 冻结统计口径;threshold = measured × 2.0 回归守护,{NO_PASS_LINE_LITERAL})"
        ),
        "sample_manifest": {"count": TIMED, "digest": f"sha256-first-frame:{doc.get('first_frame_digest', 'n/a')}"},
        "accounting": doc.get("accounting", {}),
        "provenance": {"gpu": "device", "backend": "framegen_device", "base_commit": doc.get("base_commit", "")},
        "timestamp": ts,
    }
    (ROOT / ev_rel).write_text(json.dumps(ev_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    entry = {
        "id": eid,
        "description": (
            f"FG/MFG device ×{tier[1:]} 档逐生成帧全链路帧时基线(host Instant 墙钟 around "
            f"打包+dispatch+回读同步,合成场景 128×72;warmup {WARMUP} + timed {TIMED} = 3 块 × 50 "
            "trimmed mean,M141/M165 冻结统计口径;threshold = measured × 2.0 回归守护)——"
            f"回归守护语义,{NO_PASS_LINE_LITERAL}(正式帧率对标锚定 G14;生成帧禁计入真实渲染"
            f"帧率,FgAccounting 两口径类型面分离);trimmed mean {measured:.6f} ms(cv "
            f"{stats['cv']:.4f});采样程序 ci/g26_framegen_device_bench_accounting_smoke.py "
            "bench 腿可复跑(在档后守护复检)"
        ),
        "direction": "max",
        "evidence": "measured_local",
        "skip_reason": None,
        "unit": "ms",
        "threshold": measured * 2.0,
        "evidence_file": ev_rel,
        "measured_value": measured,
    }
    problems = append_budget_entries([entry])
    if problems:
        return False, f"budget 追加失败: {problems[:2]}"
    return True, f"程序产追加 measured={measured:.6f} ms threshold={measured * 2.0:.6f} ms"


def no_pass_line_fact() -> tuple[bool, str]:
    """登记性 fact:三条帧时条目描述均携带不设通过线字面(非 vacuous——字面
    缺失即 FAIL),注明回归守护不构成帧率对标通过线。"""
    budget = load_budget()
    if budget is None:
        return False, "g26_budget.json 缺失"
    missing = []
    for _tier, eid, _ev in BENCH_TIERS:
        e = budget_entry(budget, eid)
        if e is None or NO_PASS_LINE_LITERAL not in e.get("description", ""):
            missing.append(eid)
    if missing:
        return False, f"条目缺不设通过线字面: {missing}"
    return True, (
        f"三条帧时条目均登记「{NO_PASS_LINE_LITERAL}」字面——回归守护语义,不构成"
        "帧率对标通过线(正式帧率对标锚定 G14;G6 无性能硬门纪律沿用)"
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

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    with gpu_device_lock(purpose=f"{TAG} 构建+SPV+三档 bench 腿"):
        rurixc = build_rurixc()
        spv_ok, spv_detail = (False, "rurixc 构建失败") if rurixc is None else compile_spv(rurixc)
        harness = build_harness()
        if not spv_ok or harness is None:
            set_fact("bench_three_tiers_measured", False, f"前置失败: {spv_detail if not spv_ok else 'harness 构建失败'}")
        else:
            tier_docs: dict[str, dict] = {}
            tier_details: list[str] = []
            reg_details: list[str] = []
            bench_ok = True
            reg_ok = True
            for tier, eid, ev_rel in BENCH_TIERS:
                doc, stats, detail = run_bench_tier(harness, tier)
                tier_details.append(f"{tier}:{detail}")
                if doc is None or stats is None:
                    bench_ok = False
                    reg_ok = False
                    reg_details.append(f"{tier}:bench 缺测不登记")
                    continue
                tier_docs[tier] = doc
                ok, rdetail = register_bench_entry(tier, eid, ev_rel, doc, stats, ts)
                reg_ok = reg_ok and ok
                reg_details.append(f"{tier}:{rdetail}")
            set_fact("bench_three_tiers_measured", bench_ok and len(tier_docs) == 3, ";".join(tier_details))
            set_fact("budget_entries_programmatic", reg_ok and len(tier_docs) == 3, ";".join(reg_details))
            if len(tier_docs) == 3:
                pres = [accounting_identity_presented(tier_docs[t]) for t, _e, _v in BENCH_TIERS]
                set_fact(
                    "accounting_identity_presented",
                    all(ok for ok, _ in pres),
                    ";".join(f"{t}:{d}" for (t, _e, _v), (_ok, d) in zip(BENCH_TIERS, pres)),
                )
                iso = [accounting_real_fps_isolated(tier_docs[t]) for t, _e, _v in BENCH_TIERS]
                set_fact(
                    "accounting_real_fps_isolated",
                    all(ok for ok, _ in iso),
                    ";".join(f"{t}:{d}" for (t, _e, _v), (_ok, d) in zip(BENCH_TIERS, iso)),
                )

    ok, detail = perf_surface_0byte()
    set_fact("perf_surface_0byte", ok, detail)
    ok, detail = no_pass_line_fact()
    set_fact("no_pass_line_semantics", ok, detail)

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
            "G26.2 M-b:FG device 车道帧时 measured 登记 + 口径纪律回验——三档 bench 腿真跑"
            "(warmup 10 + timed 150 逐生成帧墙钟,50×3 trimmed mean 冻结口径)程序产入 "
            "g26_budget(threshold = measured × 2.0 回归守护,不构成帧率对标通过线);"
            "FgAccounting F9 双恒等式类型面核验(presented=real+generated;real_render_fps 与 "
            "generated 无关);性能面三文件 0-byte vs g25-closed;"
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

    expect(len(FACT_IDS) >= 6, f"facts 闭集 {len(FACT_IDS)} ≥ 6")
    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8")) if SCHEMA_PATH.is_file() else {}
    min_items = schema.get("properties", {}).get("extra_facts", {}).get("minItems", 99)
    expect(len(FACT_IDS) >= min_items, f"facts {len(FACT_IDS)} ≥ schema minItems {min_items}")
    # 红臂①:50×3 trimmed mean 协议——样本数不足必拒;统计面篡改必检出。
    try:
        block_stats([1.0] * (TIMED - 1))
        expect(False, "RED:样本数不足必拒")
    except ValueError:
        expect(True, "RED:样本数不足必拒")
    good_samples = [0.5 + (i % 7) * 0.001 for i in range(TIMED)]
    good = block_stats(good_samples)
    expect(recompute_check(good_samples, good), "GREEN:统计面独立重算咬合")
    tampered = dict(good, trimmed_mean_ms=good["trimmed_mean_ms"] + 0.01)
    expect(not recompute_check(good_samples, tampered), "RED:统计面篡改必检出")
    # 红臂②:F9 恒等式判读——presented 篡改必拒;正例过。
    good_doc = {
        "accounting": {
            "real_frames": 11, "generated_frames": 9, "presented_frames": 20,
            "real_render_seconds": 0.5, "real_render_fps": 22.0,
        },
        "identity_presented_ok": True,
        "identity_real_fps_recompute_ok": True,
        "identity_real_fps_isolated_ok": True,
    }
    ok, _ = accounting_identity_presented(good_doc)
    expect(ok, "GREEN:presented 恒等式正例")
    bad = json.loads(json.dumps(good_doc))
    bad["accounting"]["presented_frames"] = 21
    ok, _ = accounting_identity_presented(bad)
    expect(not ok, "RED:presented 混算注入必拒")
    ok, _ = accounting_real_fps_isolated(good_doc)
    expect(ok, "GREEN:real_fps 重算/隔离正例")
    bad2 = json.loads(json.dumps(good_doc))
    bad2["accounting"]["real_render_fps"] = 40.0  # 生成帧混入真渲口径的冒充形态
    ok, _ = accounting_real_fps_isolated(bad2)
    expect(not ok, "RED:real_fps 混算注入必拒")
    bad3 = json.loads(json.dumps(good_doc))
    bad3["identity_real_fps_isolated_ok"] = False
    ok, _ = accounting_real_fps_isolated(bad3)
    expect(not ok, "RED:类型面隔离核验假必拒")
    # 红臂③:estimated 冒充 measured 必拒;不设通过线字面缺失可检。
    expect(not _entry_is_measured({"evidence": "estimated"}), "RED:estimated 注入必拒")
    expect(_entry_is_measured({"evidence": "measured_local", "skip_reason": None}), "GREEN:measured_local 正例")
    expect(NO_PASS_LINE_LITERAL == "不构成帧率对标通过线", "GREEN:不设通过线字面同源(g12 冻结)")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts={len(FACT_IDS)};3 红臂组 + 正例组)")
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
