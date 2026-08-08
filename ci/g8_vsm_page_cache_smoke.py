#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G8.5a M19 vsm_page_cache 硬门冒烟(g8.p0.m19.vsm_page_cache)。

host:shadow:: 回归 + g8_m19_probe 16 帧事件序列(vs golden sha)。
device:uc06-renderer --m19-vsm-page-cache
  ⓪ 逐帧 `vsm_page_mark_project`(主相机深度 → 反投影 → 选级 → 出窗回退 → 原子
     位图)dispatch + 位图 readback,与 host 镜像 `Vsm::page_mark_bits` 逐位/逐槽
     对拍(A2.1);
  ① 逐帧 page_table / depth_pool / sample digest —— **原像取自 device readback**,
     与 golden `tests/vsm_page_cache/golden/m19_digests.json` 逐帧逐字比对;
  ② multi-view depth 对拍(G7.5 冻结 1e-6);
  ③ RED 四轴:stale 页表 / local 页不入批 / 驱逐序扰动 / mark 位图冒充。

A2.1 清零记录(2026-08-07):
  * fixture 的标记源曾是 `vsm.mark_slot(l,x,y)` —— host **预知 page id** 直接标页,
    既无主相机深度也不做反投影/选级;`vsm_page_mark_project.rx` 编进 SPV 却零消费
    ⇒ 设计 §2.1 帧循环第一行 / §2.3 第一核在 device 面为空。现改为深度网格驱动,
    device 逐帧真跑该核并逐位对拍(RED:`--m19-red-skip-mark` / `--m19-red-host-mark`)。
  * device JSON 的 `validation_errors` 曾是 `let validation_errors = 0u32;` 字面量,
    现取 `rurix_rt::render_exec::validation_error_total()` 实数 + messenger 装载位。
RD-038 raster/VSM 接入:空集(已 closed,见 deferred.json + G7 evidence 指针)。

A2 清零记录(2026-08-07):此门此前三处 host 代绿/自指,已删并以 golden 对拍替代——
  * device JSON 的 `page_table_digest`/`sample_digest` 曾是 host fixture digest 直填,
    smoke 侧只做 `bool(...)` truthiness 判定 ⇒ 灌任意垃圾串仍全绿
    (证伪记录 `.a2_evidence/02_falsify_garbage_digest.txt`、`03_falsify_device_json.txt`);
  * host probe 的 `page_table_digest_match` 等三位是 `digests.len()==16` / `len()==64`
    这类自指臂,已从 `M19HostChecks` 删除(判据只能由 device 段给);
  * `red_wrong_eviction` 曾是恒真式(`real != "0"*64 and 序列位`),现改为真扰动臂:
    池预算 6→12 改变 LRU 受害者集合(evict 4→2)⇒ 事件序列 sha 必红。

用法:
  py -3 ci/g8_vsm_page_cache_smoke.py --gate g8.p0.m19.vsm_page_cache
  py -3 ci/g8_vsm_page_cache_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import datetime as _dt
import json
import os
import platform
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"
GOLDEN_DIR = ROOT / "tests" / "vsm_page_cache" / "golden"
SCHEMA_PATH = ROOT / "milestones" / "g8" / "g8_m19_vsm_page_cache_evidence_schema.json"
FRAME_COUNT = 16

GATE_KEY = "g8.p0.m19.vsm_page_cache"
NUMERIC_STEP = 115
SOURCE_REF = (
    "G8_ACCEPTANCE_MAP §2 M19;G8.5_RENDERING_COMPLETION_DESIGN §2;"
    "RD-038 closed → G8.5a 接入空集"
    "(evidence/renderer_raster_diff_smoke_20260804T170945.json)"
)
TAG = "g8_m19"
WAVE = "G8.5a"

CHECK_KEYS = [
    "host_oracle_regression",
    "event_sequence_matches_golden",
    "cross_frame_cache_hit",
    "invalidation_reasons_exhaustive",
    "clipmap_scroll_hit",
    "local_light_page_hit",
    "non_virtual_caster_hit",
    "multi_view_batch",
    "page_table_digest_match",
    "depth_readback_digest_match",
    "sample_digest_match",
    # A2.1:mark 段(设计 §2.1 帧循环第一行 / §2.3 第一核)真上 device。
    "device_page_mark_project",
    "red_stale_page",
    "red_wrong_eviction",
    "red_missing_local_page",
    "red_fake_page_mark",
    "validation_zero",
    "not_satisfiable_by_g7",
]

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


def require_real() -> bool:
    return os.environ.get("RURIX_REQUIRE_REAL") == "1"


def _tool_version(tool: str) -> str:
    try:
        r = subprocess.run([tool, "--version"], capture_output=True, text=True)
        return r.stdout.strip().splitlines()[0] if r.stdout else "unknown"
    except Exception:
        return "unknown"


def extract_json(stdout: str) -> dict | None:
    text = stdout.strip()
    if not text:
        return None
    # 优先:含 subject 的整行 JSON(忽略前后日志行)。
    for line in reversed(text.splitlines()):
        line = line.strip()
        if line.startswith("{") and "subject" in line:
            try:
                return json.loads(line)
            except Exception:
                continue
    try:
        return json.loads(text)
    except Exception:
        return None


def run_probe(extra: list[str] | None = None) -> dict | None:
    args = [
        "cargo",
        "run",
        "-q",
        "-p",
        "rurix-render",
        "--bin",
        "g8_m19_probe",
        "--",
        "--golden-dir",
        str(GOLDEN_DIR),
    ]
    if extra:
        args.extend(extra)
    print(f"[{TAG}] probe: {' '.join(args[-3:])}")
    r = subprocess.run(args, cwd=ROOT, capture_output=True, text=True)
    if r.returncode != 0:
        check(False, f"g8_m19_probe 失败 rc={r.returncode}\n{r.stderr}")
    return extract_json(r.stdout)


def jint(doc: dict | None, key: str, missing: int = -1) -> int:
    """取整型字段;**缺字段**才落 `missing`(不可用 `or`:0 是合法值且为假值)。"""
    v = (doc or {}).get(key)
    if v is None:
        return missing
    try:
        return int(v)
    except (TypeError, ValueError):
        return missing


def jfloat(doc: dict | None, key: str, missing: float = -1.0) -> float:
    v = (doc or {}).get(key)
    if v is None:
        return missing
    try:
        return float(v)
    except (TypeError, ValueError):
        return missing


def load_golden_digests() -> list[dict]:
    """golden 逐帧 digest(device readback digest 的**唯一**比对基准)。"""
    p = GOLDEN_DIR / "m19_digests.json"
    if not p.is_file():
        check(False, f"缺 golden {p}(先 --write-golden)")
        return []
    frames = json.loads(p.read_text(encoding="utf-8")).get("frames") or []
    if len(frames) != FRAME_COUNT:
        check(False, f"golden 帧数 {len(frames)} ≠ {FRAME_COUNT}")
    return frames


def digest_frames_match(doc: dict, golden: list[dict], key: str) -> tuple[bool, int]:
    """device 逐帧 digest 与 golden 逐帧比对;返回(全等, 相等帧数)。

    只信 `device_frames[*]` 里 device readback 出的值;device 自报的
    `*_frames_matched` 计数仅作交叉校验(不一致 → 判失败)。
    """
    dev = doc.get("device_frames") or []
    if len(dev) != FRAME_COUNT or len(golden) != FRAME_COUNT:
        return False, 0
    matched = 0
    for d, g in zip(dev, golden):
        dv = d.get(key) or ""
        gv = g.get(key) or ""
        if len(dv) != 64 or not set(dv) <= set("0123456789abcdef"):
            return False, matched
        if d.get("frame") != g.get("frame"):
            return False, matched
        if dv == gv:
            matched += 1
    return matched == FRAME_COUNT, matched


def mark_section_ok(doc: dict | None) -> tuple[bool, list[str]]:
    """A2.1 mark 段判据:位图必须是 device `vsm_page_mark_project` 真跑 readback。

    逐条独立、全部硬比对(无 truthiness):
      * `mark_provenance == "device_readback"` 且 kernel 名对得上;
      * 16 帧各一次 dispatch(跳 dispatch 的臂 `mark_dispatches` 会掉);
      * 逐位对拍零字失配、逐槽对拍零槽失配、`levels*512` 之后零越界写;
      * 位图置位总数 > 0(全零位图 = 段空转);
      * 位图去重数 ≥2 —— **深度驱动**的位图必随帧变化;host 预知 page id 的
        冒充位图逐帧恒定(去重数 = 1),此条即为其证伪面。
    """
    d = doc or {}
    bad: list[str] = []
    if d.get("mark_provenance") != "device_readback":
        bad.append(f"mark_provenance={d.get('mark_provenance')!r}")
    if d.get("mark_kernel") != "vsm_page_mark_project":
        bad.append(f"mark_kernel={d.get('mark_kernel')!r}")
    if jint(d, "mark_dispatches") != FRAME_COUNT:
        bad.append(f"mark_dispatches={d.get('mark_dispatches')}")
    if jint(d, "mark_frames_matched") != FRAME_COUNT:
        bad.append(f"mark_frames_matched={d.get('mark_frames_matched')}")
    if jint(d, "mark_word_mismatches", 1) != 0:
        bad.append(f"mark_word_mismatches={d.get('mark_word_mismatches')}")
    if jint(d, "mark_slot_mismatches", 1) != 0:
        bad.append(f"mark_slot_mismatches={d.get('mark_slot_mismatches')}")
    if jint(d, "mark_tail_dirty", 1) != 0:
        bad.append(f"mark_tail_dirty={d.get('mark_tail_dirty')}")
    if jint(d, "mark_bits_total", 0) <= 0:
        bad.append(f"mark_bits_total={d.get('mark_bits_total')}")
    if jint(d, "mark_distinct_bitmaps", 0) < 2:
        bad.append(f"mark_distinct_bitmaps={d.get('mark_distinct_bitmaps')}")
    if jint(d, "mark_pixels_per_frame", 0) <= 0:
        bad.append(f"mark_pixels_per_frame={d.get('mark_pixels_per_frame')}")
    # 「位图由深度驱动」的结构性证据:F12→F13 唯一输入差异是深度缓冲,而 device
    # 位图不同 ⇒ 位图不可能从 lparams/常量凑出。device 侧逐字段比对后给出本位。
    if not d.get("mark_depth_is_causal"):
        bad.append("mark_depth_is_causal=false")
    if not d.get("mark_all_match"):
        bad.append("mark_all_match=false")
    frames = d.get("mark_frames") or []
    if len(frames) != FRAME_COUNT:
        bad.append(f"mark_frames={len(frames)}帧")
    elif not all(f.get("match") and f.get("dispatched") for f in frames):
        bad.append("逐帧 mark_frames[*].match/dispatched 有假")
    return (not bad), bad


def run_device(extra: list[str] | None = None) -> tuple[str, dict | None]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    cmd = [
        "cargo",
        "run",
        "-q",
        "-p",
        "uc06-renderer",
        "--features",
        "vulkan",
        "--",
        "--m19-vsm-page-cache",
    ]
    if extra:
        cmd.extend(extra)
    print(f"[{TAG}] device: {' '.join(cmd[-4:])}")
    r = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, env=env, timeout=600)
    doc = extract_json(r.stdout)
    if doc is None and "SKIP" in (r.stdout + r.stderr):
        if require_real():
            check(False, f"device SKIP(RURIX_REQUIRE_REAL=1):\n{r.stdout}\n{r.stderr}")
        return "skipped_dev_env", None
    if doc is None:
        check(False, f"device JSON 缺失 rc={r.returncode}\n{r.stderr[-1500:]}\n{r.stdout[-1500:]}")
        return "fail", None
    if r.returncode != 0 and not (doc.get("pass") or doc.get("red_ok")):
        check(False, f"device 失败 rc={r.returncode} pass/red_ok 皆假")
        return "fail", doc
    return "executed", doc


def run_selftest() -> int:
    assert SCHEMA_PATH.is_file()
    assert len(CHECK_KEYS) >= 16
    # 反假绿:仅 G7 page-mark/单帧 depth 不得满足本门 checks 全集
    fake = {k: False for k in CHECK_KEYS}
    fake["host_oracle_regression"] = True
    assert not all(fake.values())

    golden = [
        {"frame": i, "page_table": f"{i:064x}", "depth_pool": f"{i:064x}", "sample": f"{i:064x}"}
        for i in range(FRAME_COUNT)
    ]
    good = {"device_frames": [dict(g) for g in golden]}
    for key in ("page_table", "depth_pool", "sample"):
        ok, n = digest_frames_match(good, golden, key)
        assert ok and n == FRAME_COUNT, key
    # A2 反假绿:垃圾串 / 空串 / 少一帧 / 单帧漂移 一律不得判绿
    for bad_doc in (
        {"device_frames": [{**g, "page_table": "FALSIFY_not_a_real_hash"} for g in golden]},
        {"device_frames": [{**g, "page_table": ""} for g in golden]},
        {"device_frames": [dict(g) for g in golden[:-1]]},
        {
            "device_frames": [
                {**g, "page_table": ("f" * 64) if g["frame"] == 7 else g["page_table"]}
                for g in golden
            ]
        },
        {},
    ):
        ok, _ = digest_frames_match(bad_doc, golden, "page_table")
        assert not ok, f"篡改样本被判绿: {str(bad_doc)[:60]}"

    # A2.1 mark 段反假绿:好文档判绿,六种冒充/退化一律判红。
    good_mark = {
        "mark_provenance": "device_readback",
        "mark_kernel": "vsm_page_mark_project",
        "mark_dispatches": FRAME_COUNT,
        "mark_frames_matched": FRAME_COUNT,
        "mark_word_mismatches": 0,
        "mark_slot_mismatches": 0,
        "mark_tail_dirty": 0,
        "mark_bits_total": 76,
        "mark_distinct_bitmaps": 2,
        "mark_pixels_per_frame": 64,
        "mark_depth_is_causal": True,
        "mark_all_match": True,
        "mark_frames": [
            {"frame": i, "match": True, "dispatched": True} for i in range(FRAME_COUNT)
        ],
    }
    ok, bad = mark_section_ok(good_mark)
    assert ok, bad
    falsify = [
        # ① 不 dispatch(核编进 SPV 但零消费的旧态)
        {"mark_dispatches": 0, "mark_bits_total": 0, "mark_frames_matched": 0},
        # ② host 预知 page id 冒充:位图逐帧恒定
        {"mark_distinct_bitmaps": 1, "mark_frames_matched": 13},
        # ③ 位图全零(段空转)
        {"mark_bits_total": 0},
        # ④ 逐位失配
        {"mark_word_mismatches": 3},
        # ⑤ 逐槽失配
        {"mark_slot_mismatches": 1},
        # ⑥ provenance 自述被改
        {"mark_provenance": "host_fixture"},
        # ⑦ 越界写
        {"mark_tail_dirty": 1},
        # ⑧ 逐帧位有假(聚合数好看,明细掉)
        {"mark_frames": [{"frame": i, "match": i != 7, "dispatched": True} for i in range(FRAME_COUNT)]},
        # ⑨ 位图与深度无因果(F12→F13 只差深度却出同一位图)
        {"mark_depth_is_causal": False},
    ]
    for patch in falsify:
        ok, _ = mark_section_ok({**good_mark, **patch})
        assert not ok, f"mark 冒充样本被判绿: {patch}"
    ok, _ = mark_section_ok({})
    assert not ok, "空 device 文档被判绿"
    print(
        f"[{TAG}] selftest PASS checks={len(CHECK_KEYS)} digest-falsify=5/5 "
        f"mark-falsify={len(falsify)}/{len(falsify)}"
    )
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default=GATE_KEY)
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--write-golden", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate != GATE_KEY:
        print(f"unknown gate {args.gate}", file=sys.stderr)
        return 2

    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    checks = {k: False for k in CHECK_KEYS}

    if args.write_golden:
        print(f"[{TAG}] write golden → {GOLDEN_DIR}")
        subprocess.run(
            [
                "cargo",
                "run",
                "-q",
                "-p",
                "rurix-render",
                "--bin",
                "g8_m19_probe",
                "--",
                "--golden-dir",
                str(GOLDEN_DIR),
                "--write-golden",
            ],
            cwd=ROOT,
            check=False,
        )

    # host units
    print(f"[{TAG}] cargo test -p rurix-render shadow::")
    tr = subprocess.run(
        ["cargo", "test", "-q", "-p", "rurix-render", "shadow::"],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    checks["host_oracle_regression"] = tr.returncode == 0
    if tr.returncode != 0:
        check(False, f"shadow:: tests 失败:\n{tr.stdout}\n{tr.stderr}")

    golden = load_golden_digests()

    probe = run_probe()
    if probe:
        # host 段只判 host 能判的:事件序列 vs golden sha + 六条事件语义位。
        # 三条 digest 判据**不在此处**(host 自指臂已从 M19HostChecks 删除)。
        for k in [
            "event_sequence_matches_golden",
            "cross_frame_cache_hit",
            "invalidation_reasons_exhaustive",
            "clipmap_scroll_hit",
            "local_light_page_hit",
            "non_virtual_caster_hit",
            "multi_view_batch",
        ]:
            ok = bool(probe.get(k))
            checks[k] = ok
            if not ok:
                check(False, f"probe.{k} 为假")
        for leaked in ("page_table_digest_match", "depth_readback_digest_match", "sample_digest_match"):
            if leaked in probe:
                check(False, f"probe 又冒出 host 代绿位 {leaked}(A2 已清零,不得回流)")
        # host probe 的逐帧 digest 必须逐字等于 golden 文件(golden 未漂移)。
        pdig = probe.get("digests") or []
        if len(pdig) != FRAME_COUNT or len(golden) != FRAME_COUNT:
            check(False, f"probe digests {len(pdig)} / golden {len(golden)} 帧数不符")
        elif any(
            pdig[i].get(k) != golden[i].get(k)
            for i in range(FRAME_COUNT)
            for k in ("page_table", "depth_pool", "sample", "dirty_depth")
        ):
            check(False, "host probe 逐帧 digest 与 golden 文件不一致(golden 已漂移)")
        note(f"events_sha256={probe.get('events_sha256')}")
        note(f"max_view_count={probe.get('max_view_count')}")
        note(f"evict_count={probe.get('evict_count')} pool_pages={probe.get('pool_pages')}")
    else:
        check(False, "probe JSON 缺失")

    # golden events sha 必须真实存在且非退化(序列判据的锚)
    sha_path = GOLDEN_DIR / "m19_events.sha256"
    if not sha_path.is_file():
        check(False, "缺 golden m19_events.sha256(先 --write-golden)")
    else:
        real = sha_path.read_text(encoding="utf-8").strip()
        if len(real) != 64 or real == "0" * 64:
            check(False, f"golden events sha 退化: {real!r}")

    # RED: wrong eviction — 真扰动臂:池预算 6→12 改 LRU 受害者集合(evict 4→2),
    # 事件序列 sha 必与 golden 不同。恒真式已删。
    red_evict = run_probe(["--red-wrong-eviction"])
    if red_evict:
        checks["red_wrong_eviction"] = (
            bool(red_evict.get("red_ok"))
            and bool(red_evict.get("evict_order_changed"))
            and bool(red_evict.get("event_sequence_red"))
            and jint(red_evict, "base_evict_count", 0) > 0
            and jint(red_evict, "red_evict_count", 0) > 0
            and jint(red_evict, "base_evict_count") != jint(red_evict, "red_evict_count")
            and red_evict.get("base_events_sha256") != red_evict.get("red_events_sha256")
        )
        note(
            f"red_evict pool {red_evict.get('base_pool_pages')}→{red_evict.get('red_pool_pages')} "
            f"evict {red_evict.get('base_evict_count')}→{red_evict.get('red_evict_count')}"
        )
    if not checks["red_wrong_eviction"]:
        check(False, "RED wrong-eviction 未翻红")

    # device 段:三条 digest 判据全部由 device readback digest vs golden 逐帧比对给出
    pt_matched = pool_matched = sample_matched = 0
    device_state, doc = run_device()
    if device_state == "executed" and doc:
        if doc.get("digest_provenance") != "device_readback":
            check(False, f"device digest provenance 非 device_readback: {doc.get('digest_provenance')!r}")
        pt_ok, pt_n = digest_frames_match(doc, golden, "page_table")
        pool_ok, pool_n = digest_frames_match(doc, golden, "depth_pool")
        smp_ok, smp_n = digest_frames_match(doc, golden, "sample")
        pt_matched, pool_matched, sample_matched = pt_n, pool_n, smp_n
        mism = jint(doc, "sample_value_mismatches")
        checks["page_table_digest_match"] = pt_ok
        checks["depth_readback_digest_match"] = pool_ok and bool(doc.get("depth_match"))
        checks["sample_digest_match"] = smp_ok and mism == 0
        if not pt_ok:
            check(False, f"逐帧 page_table digest vs golden 仅 {pt_n}/{FRAME_COUNT} 帧相等")
        if not pool_ok:
            check(False, f"逐帧 depth_pool digest vs golden 仅 {pool_n}/{FRAME_COUNT} 帧相等")
        if not smp_ok:
            check(False, f"逐帧 sample digest vs golden 仅 {smp_n}/{FRAME_COUNT} 帧相等")
        if mism != 0:
            check(False, f"device 采样值与 host oracle 不一致 {mism} 个(0/1 二值应零容差)")
        if not doc.get("depth_match"):
            check(False, "multi-view depth 对拍未过")
        # device 自报计数与 smoke 独立比对结果必须一致(防 device 侧自说自话)
        for key, n in (
            ("page_table_digest_frames_matched", pt_n),
            ("depth_pool_digest_frames_matched", pool_n),
            ("sample_digest_frames_matched", smp_n),
        ):
            if jint(doc, key) != n:
                check(False, f"device 自报 {key}={doc.get(key)} ≠ smoke 独立比对 {n}")
        checks["multi_view_batch"] = (
            jint(doc, "view_count", 0) >= 5 and jint(doc, "dispatch_count", 0) >= 1
        )
        # A2.1:mark 段。`validation_zero` 不再只看 `validation_errors == 0` ——
        # device 侧此前是写死的 `0u32` 字面量,现取 messenger 进程实数;messenger
        # 未装上时 0 不可信,故 `validation_messenger` 必须为真。
        mark_ok, mark_bad = mark_section_ok(doc)
        checks["device_page_mark_project"] = mark_ok
        if not mark_ok:
            check(False, f"mark 段(device page_mark_project)未过: {mark_bad}")
        checks["validation_zero"] = jint(doc, "validation_errors") == 0 and bool(
            doc.get("validation_messenger")
        )
        if not checks["validation_zero"]:
            check(
                False,
                f"validation 非零或 messenger 未装(errors={doc.get('validation_errors')},"
                f" messenger={doc.get('validation_messenger')})",
            )
        # 反假绿:本门规模不是 G7 的单帧腿 —— 16 帧 mark + 16 帧逐帧 digest +
        # 4 帧 local 臂 + ≥5 视图;dispatch 数 ≥ 2×16(mark 段 + 采样段)。
        checks["not_satisfiable_by_g7"] = (
            jint(doc, "frames_checked", 0) == FRAME_COUNT
            and jint(doc, "frames_with_local", 0) >= 4
            and jint(doc, "view_count", 0) >= 5
            and jint(doc, "mark_dispatches", 0) == FRAME_COUNT
            and jint(doc, "dispatch_count", 0) >= 2 * FRAME_COUNT
        )
        if not checks["not_satisfiable_by_g7"]:
            check(
                False,
                "规模判据不成立(frames/local/view/mark/dispatch:"
                f"{doc.get('frames_checked')}/{doc.get('frames_with_local')}/"
                f"{doc.get('view_count')}/{doc.get('mark_dispatches')}/"
                f"{doc.get('dispatch_count')})",
            )
        if not doc.get("pass"):
            check(False, "device pass=false")
        note(
            f"device pages={doc.get('page_count')} bitexact={doc.get('bitexact_texels')}"
            f"/{doc.get('depth_texels')} depth_max_abs={doc.get('measured_depth_max_abs')}"
        )
        note(f"device dispatches={doc.get('dispatch_count')} frames={doc.get('frames_checked')}")
    elif device_state == "skipped_dev_env":
        checks["validation_zero"] = False

    # RED axes。**逐臂**判红(不只看 red_ok):篡改落在 device 上传面,若三个 digest
    # 仍与 golden 全等,说明它们不是 device readback 出来的(= host 代填复辟)。
    # 这是本门「digest 非 host 代填」的结构性证据,不靠 provenance 字段的自述。
    _, red_stale = run_device(["--m19-red-stale"])
    stale_pool_red = jint(red_stale, "depth_pool_digest_frames_matched") < FRAME_COUNT
    checks["red_stale_page"] = (
        bool(red_stale and red_stale.get("red_ok"))
        and stale_pool_red
        and not red_stale.get("depth_match")
    )
    if not checks["red_stale_page"]:
        check(
            False,
            "RED stale 未翻红(depth_pool 匹配帧="
            f"{jint(red_stale, 'depth_pool_digest_frames_matched')}/{FRAME_COUNT},"
            f"depth_match={(red_stale or {}).get('depth_match')})",
        )

    _, red_local = run_device(["--m19-red-missing-local"])
    local_pt_red = jint(red_local, "page_table_digest_frames_matched") < FRAME_COUNT
    local_smp_red = jint(red_local, "sample_digest_frames_matched") < FRAME_COUNT
    checks["red_missing_local_page"] = (
        bool(red_local and red_local.get("red_ok")) and local_pt_red and local_smp_red
    )
    if not checks["red_missing_local_page"]:
        check(
            False,
            "RED missing-local 未翻红(page_table 匹配帧="
            f"{jint(red_local, 'page_table_digest_frames_matched')}/{FRAME_COUNT},"
            f"sample 匹配帧={jint(red_local, 'sample_digest_frames_matched')}/{FRAME_COUNT})",
        )
    # A2.1 RED:mark 位图的两种「不是 device 产的」冒充路径都必须翻红。
    #   ① skip:不 dispatch,零位图冒充 —— 即「核编进 SPV 但零消费」的旧态;
    #   ② host impostor:host 预知 page id(A2.1 前 `mark_slot` 的四页硬编码)
    #      生成位图冒充 —— 位图逐帧恒定,与深度驱动的真位图在 F13+(压力页)分叉。
    #      ② 是关键一枪:它在 F0–F12 与真位图**完全一致**,只有深度真被消费时
    #      才抓得住 ⇒ 证明本门判的是「device 反投影结果」而非「有个位图就行」。
    _, red_skip = run_device(["--m19-red-skip-mark"])
    _, red_host = run_device(["--m19-red-host-mark"])
    skip_red = (
        bool(red_skip and red_skip.get("red_ok"))
        and not (red_skip or {}).get("mark_all_match")
        and jint(red_skip, "mark_dispatches", 1) == 0
        and jint(red_skip, "mark_bits_total", 1) == 0
        and jint(red_skip, "mark_frames_matched", FRAME_COUNT) < FRAME_COUNT
    )
    host_red = (
        bool(red_host and red_host.get("red_ok"))
        and not (red_host or {}).get("mark_all_match")
        and jint(red_host, "mark_slot_mismatches", 0) > 0
        and jint(red_host, "mark_distinct_bitmaps", 2) == 1
        and jint(red_host, "mark_frames_matched", FRAME_COUNT) < FRAME_COUNT
    )
    checks["red_fake_page_mark"] = skip_red and host_red
    if not checks["red_fake_page_mark"]:
        check(
            False,
            "RED fake-page-mark 未翻红(skip:"
            f"dispatch={jint(red_skip, 'mark_dispatches')} bits={jint(red_skip, 'mark_bits_total')}"
            f" matched={jint(red_skip, 'mark_frames_matched')};host-impostor:"
            f"slot_mism={jint(red_host, 'mark_slot_mismatches')}"
            f" distinct={jint(red_host, 'mark_distinct_bitmaps')}"
            f" matched={jint(red_host, 'mark_frames_matched')})",
        )
    note(
        f"mark device bits={jint(doc, 'mark_bits_total', 0)} dispatch={jint(doc, 'mark_dispatches', 0)} "
        f"distinct={jint(doc, 'mark_distinct_bitmaps', 0)} px/frame={jint(doc, 'mark_pixels_per_frame', 0)}"
    )
    note(
        f"red_fake_mark skip_matched={jint(red_skip, 'mark_frames_matched')}/{FRAME_COUNT} "
        f"host_impostor_matched={jint(red_host, 'mark_frames_matched')}/{FRAME_COUNT}"
    )

    note(
        f"red_stale pool_digest_red={FRAME_COUNT - jint(red_stale, 'depth_pool_digest_frames_matched', 0)}帧; "
        f"red_missing_local pt_red={FRAME_COUNT - jint(red_local, 'page_table_digest_frames_matched', 0)}帧 "
        f"smp_red={FRAME_COUNT - jint(red_local, 'sample_digest_frames_matched', 0)}帧"
    )

    note(
        "RD-038 closed empty-set for G8.5a;"
        "closed-ptr=evidence/renderer_raster_diff_smoke_20260804T170945.json"
    )

    # host 段 = host 能自证的判据。三条 digest 判据与 multi_view_batch /
    # not_satisfiable_by_g7 属 device 段(A2:不得由 host 自指臂顶包)。
    host_keys = [
        "host_oracle_regression",
        "event_sequence_matches_golden",
        "cross_frame_cache_hit",
        "invalidation_reasons_exhaustive",
        "clipmap_scroll_hit",
        "local_light_page_hit",
        "non_virtual_caster_hit",
        "red_wrong_eviction",
    ]
    host_pass = all(checks[k] for k in host_keys)

    all_pass = all(checks.values()) and not FAILURES
    if device_state == "skipped_dev_env" and require_real():
        all_pass = False
        device_section = "skipped_dev_env"
    elif all_pass and device_state == "executed":
        device_section = "pass"
    else:
        device_section = device_state if device_state else "fail"

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    evidence = {
        "schema_version": 1,
        "subject": "g8_m19_vsm_page_cache",
        "symbolic_gate_key": GATE_KEY,
        "matrix_row": "M19",
        "wave": WAVE,
        "numeric_step": NUMERIC_STEP,
        "source_ref": SOURCE_REF,
        "host_section_pass": host_pass,
        "device_section_state": device_section,
        "checks": {k: bool(checks[k]) for k in CHECK_KEYS},
        "device_digest_section": {
            "provenance": (doc or {}).get("digest_provenance") or "device_readback",
            "golden_ref": "tests/vsm_page_cache/golden/m19_digests.json",
            "frames_checked": jint(doc, "frames_checked", 0),
            "frames_with_local": jint(doc, "frames_with_local", 0),
            "page_table_frames_matched": pt_matched,
            "depth_pool_frames_matched": pool_matched,
            "sample_frames_matched": sample_matched,
            "sample_value_mismatches": jint(doc, "sample_value_mismatches", 0),
            "device_dispatches": jint(doc, "dispatch_count", 0),
            "measured_depth_max_abs": jfloat(doc, "measured_depth_max_abs", 0.0),
            "tol_depth": jfloat(doc, "tol_depth", 0.0),
        },
        "device_mark_section": {
            "kernel": (doc or {}).get("mark_kernel") or "vsm_page_mark_project",
            "provenance": (doc or {}).get("mark_provenance") or "device_readback",
            "host_mirror": "rurix_render::shadow::vsm::Vsm::page_mark_bits",
            "dispatches": jint(doc, "mark_dispatches", 0),
            "frames_matched": jint(doc, "mark_frames_matched", 0),
            "bits_set_total": jint(doc, "mark_bits_total", 0),
            "word_mismatches": jint(doc, "mark_word_mismatches", -1),
            "slot_mismatches": jint(doc, "mark_slot_mismatches", -1),
            "tail_dirty_words": jint(doc, "mark_tail_dirty", -1),
            "distinct_bitmaps": jint(doc, "mark_distinct_bitmaps", 0),
            "pixels_per_frame": jint(doc, "mark_pixels_per_frame", 0),
            "depth_is_causal": bool((doc or {}).get("mark_depth_is_causal")),
            "depth_is_causal_basis": (
                "F12→F13 的唯一输入差异是深度缓冲(inv_vp/lparams/cam/灯基/base_radius/"
                "levels 逐字相同),而两帧 device readback 位图不同 ⇒ 位图由深度反投影产生"
            ),
            "local_spot_mark_scope": (
                "spot 单级透视页表不在 vsm_page_mark_project 输入布局内(设计 §2.3 "
                "第一核只列方向光 clipmap mark),故 local 臂 mark 仍 host 白盒,"
                "不计入本段 device 判据"
            ),
        },
        "validation_section": {
            "errors": jint(doc, "validation_errors", -1),
            "messenger_installed": bool((doc or {}).get("validation_messenger")),
            "source": "rurix_rt::render_exec::validation_error_total()",
        },
        "red_axes": {
            "stale_page": (
                "device 上传面改上传**上一帧**物理池(抑制一次失效 ⇒ 页内容 stale)"
                f"+ mv 段 z 区间偏移;depth_pool digest 红 "
                f"{FRAME_COUNT - jint(red_stale, 'depth_pool_digest_frames_matched', 0)} 帧"
            ),
            "missing_local_page": (
                "device 上传面 local 页表段清零(local 页不入批)+ mv 段 local tri_count=0;"
                f"page_table digest 红 {FRAME_COUNT - jint(red_local, 'page_table_digest_frames_matched', 0)} 帧,"
                f"sample digest 红 {FRAME_COUNT - jint(red_local, 'sample_digest_frames_matched', 0)} 帧"
            ),
            "wrong_eviction": (
                f"池预算 {(red_evict or {}).get('base_pool_pages')}→"
                f"{(red_evict or {}).get('red_pool_pages')} 改 LRU 受害者集合;"
                f"evict {(red_evict or {}).get('base_evict_count')}→"
                f"{(red_evict or {}).get('red_evict_count')};事件序列 sha 变"
            ),
            "fake_page_mark": (
                "① --m19-red-skip-mark:不 dispatch vsm_page_mark_project,零位图冒充 ⇒ "
                f"mark 匹配 {jint(red_skip, 'mark_frames_matched', -1)}/{FRAME_COUNT}、"
                f"置位 {jint(red_skip, 'mark_bits_total', -1)};"
                "② --m19-red-host-mark:host 预知 page id(A2.1 前 mark_slot 四页硬编码)"
                f"冒充 ⇒ 位图去重数 {jint(red_host, 'mark_distinct_bitmaps', -1)}(恒定)、"
                f"mark 匹配 {jint(red_host, 'mark_frames_matched', -1)}/{FRAME_COUNT}"
                f"(F13+ 压力页分叉)、逐槽失配 {jint(red_host, 'mark_slot_mismatches', -1)}"
            ),
        },
        "evidence_level": "measured_local",
        "run_url": "",
        "timestamp": ts,
        "environment": {
            "os": platform.platform(),
            "python_version": sys.version.split()[0],
            "cargo_version": _tool_version("cargo"),
            "rustc_version": _tool_version("rustc"),
        },
        "rd038_raster_vsm_ingress": {
            "status": "empty_set",
            "reason": "RD-038 closed on G7.7 path; G8.5a ingress empty-set (design §5)",
            "closed_evidence_pointers": [
                "milestones/g7/G7_CONTRACT.md §8.1",
                "evidence/renderer_raster_diff_smoke_20260804T170945.json",
                "milestones/g7/RD038_LITERAL_MATRIX.md §7",
            ],
        },
        "notes": "; ".join(NOTES + FAILURES[:8]),
    }
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    out = EVIDENCE_DIR / f"g8_m19_vsm_page_cache_{ts}.json"
    out.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[{TAG}] evidence → {out}")

    schema = json.loads(SCHEMA_PATH.read_text(encoding="utf-8"))
    for k in schema.get("required", []):
        check(k in evidence, f"evidence 缺字段 {k}")

    passed = sum(1 for v in checks.values() if v)
    print(f"[{TAG}] checks {passed}/{len(CHECK_KEYS)} device={device_section}")
    if all_pass and not FAILURES:
        print(f"[{TAG}] PASS")
        return 0
    print(f"[{TAG}] FAIL: {FAILURES}", file=sys.stderr)
    return 1


if __name__ == "__main__":
    sys.exit(main())
