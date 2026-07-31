#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""Rapier 第二后端对拍冒烟(步骤 90;G6.4;RFC-0017 §4.D;验收门 G-G6-5)。

host 段(**恒跑**,纯 host 门,check_* 风格;Rapier 为纯 Rust CPU 库,无 GPU 依赖):
  1. 默认 off 核验(§4.D1,`cargo metadata` 机验非 grep):rurix-physics 的
     feature `rapier` 存在且 default feature 集不含 `rapier`(生产默认 = Jolt,
     快路径不替换默认)。
  2. 无 CMake 树核验(§4.D1「纯 Rust 零 CMake」佐证):rapier-only 档
     (`--no-default-features --features rapier`)`cargo tree -e normal` 依赖树
     不含 rurix-physics-sys / Jolt / cxx / cmake(不构建 Jolt vendor)。
  3. cargo test 两腿 exit 0:rapier-only 档 / 双后端档(`--features rapier`,
     default=jolt 仍在);解析汇总计数入 evidence(计数为 0 判红,反 vacuous-green)。
  4. rapier 两 tier clippy:上述两档 `cargo clippy --all-targets -- -D warnings`
     零告警。
  5. parity 进程级重放一致(§4.D3):双后端档 tests/parity.rs
     `rapier_jolt_parity_n300_tolerance_and_contact_invariants` 以独立进程跑两次
     (PARITY_JSON 分别落系统 temp 两个临时路径,跑完清除、不留仓内)——
     ① 两进程 JSON 各后端确定性哈希逐位一致 + max 偏差逐值一致(进程级重放);
     ② 单次内 §4.D3 判据全过:变换容差(pos ≤ 0.82m / rot ≤ 93°)、Begin/End
     集合重叠率 ≥ 0.99、逐对相位 RLE 等价类一致、verdict == "pass";阈值常量值
     本门钉死核验(0.82 / 93.0 / 0.99,与 tests/parity.rs 顶部常量互锁,对拍侧
     静默放宽阈值即本门红);禁跨引擎逐位(逐位帧数仅记录,不进判据,§4.0-4)。
  6. 文档口径 grep(§4.D4):src/rurix-physics/src/lib.rs 与 rapier.rs 模块头
     含「快路径 ≠ 性能/稳定性默认」字样;rapier.rs 存在且为 src 面唯一含
     `rapier3d` 的 .rs 文件(与步骤 88 v1.4 收窄口径互锁;判定逻辑本文件自带,
     不 import 步骤 88)。
  7. 写 evidence/physics_rapier_parity_smoke_<ts>.json(UTC 时间戳;过
     ci/check_schemas.py——schema = milestones/g6/physics_rapier_parity_evidence_schema.json,
     单路由双形态:本文件 smoke evidence 与 parity 测试侧标定 evidence
     physics_rapier_parity_2*.json 同 schema 按 subject if/then 分流;计数/偏差
     数字入 checks 不进硬门,P-09 / RFC-0017 §4.0-5)。

device 段:**无**(纯 host 门;G6 CI_GATES §2.90 行,无 RURIX_REQUIRE_REAL 段)。

任一判据红 → 逐项打印定位后 exit 1(evidence 仍如实落盘,红不充绿)。

用法: py -3 ci/physics_rapier_parity_smoke.py [--selftest]
  --selftest: 反 YAML-only 红绿自检(合成数据喂纯判定层),不跑 cargo、不写 evidence。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import platform
import re
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
EVIDENCE_DIR = ROOT / "evidence"

# §4.D1「纯 Rust 零 CMake」依赖树禁出名(大小写不敏感):rapier-only 档不构建
# rurix-physics-sys / Jolt vendor,佐证无 CMake 路径(2026-07-31 实测树 = rapier3d
# + parry3d + nalgebra 等纯 Rust 链,零命中)。
TREE_FORBIDDEN_RE = re.compile(r"rurix-physics-sys|jolt|cxx|cmake", re.I)

# §4.D4 文档口径钉定字样(lib.rs 与 rapier.rs 模块头均须含)。
FASTPATH_DISCLAIMER = "快路径 ≠ 性能/稳定性默认"
PHYSICS_LIB_RS = "src/rurix-physics/src/lib.rs"
# §4.C4(v1.4 收窄):`rapier3d` 原生类型名唯一 sanctioned 消费模块。
PHYSICS_RAPIER_RS = "src/rurix-physics/src/rapier.rs"
RAPIER3D_RE = re.compile(r"rapier3d")

# §4.D3 判据期望值(与 tests/parity.rs 文件顶部常量互锁):本门钉死核验——对拍侧
# 静默放宽阈值即 [wellformed] 红,改阈值须同 PR 改本门并留痕 evidence。
PARITY_SCHEMA_ID = "physics_rapier_parity/run_v1"
EXPECTED_N_STEPS = 300
EXPECTED_MIN_REPLAYS = 5
EXPECTED_POS_TOL_M = 0.82
EXPECTED_ROT_TOL_DEG = 93.0
EXPECTED_CONTACT_OVERLAP_MIN = 0.99

# cargo test 两腿(判据 3)与 clippy 两 tier(判据 4)构建档。
TEST_LEGS = [
    ("rapier_only", ["cargo", "test", "-p", "rurix-physics",
                     "--no-default-features", "--features", "rapier"]),
    ("dual_backend", ["cargo", "test", "-p", "rurix-physics", "--features", "rapier"]),
]
CLIPPY_LEGS = [
    ("rapier_only", ["cargo", "clippy", "-p", "rurix-physics",
                     "--no-default-features", "--features", "rapier",
                     "--all-targets", "--", "-D", "warnings"]),
    ("dual_backend", ["cargo", "clippy", "-p", "rurix-physics", "--features", "rapier",
                      "--all-targets", "--", "-D", "warnings"]),
]
# parity 进程级重放(判据 5):双后端档单跑 parity 测试靶,独立进程 ×2。
PARITY_CMD = ["cargo", "test", "-p", "rurix-physics", "--features", "rapier",
              "--test", "parity"]

# cargo test 输出的通过计数行。
TEST_OK_RE = re.compile(r"test result: ok\. (\d+) passed; 0 failed")

# parity JSON 判定问题串 tag → evidence checks 键(judge_parity_doc 纯函数产出)。
PARITY_TAG_KEYS = {
    "wellformed": "parity_json_wellformed",
    "verdict": "parity_verdict_pass",
    "pos_within_tol": "parity_pos_within_tol",
    "rot_within_tol": "parity_rot_within_tol",
    "begin_overlap_ok": "parity_begin_overlap_ok",
    "end_overlap_ok": "parity_end_overlap_ok",
    "phase_class_equal": "parity_phase_class_equal",
}

# evidence checks 键序(schema additionalProperties=false,须与 g6 schema 同步)。
CHECK_KEYS = (
    "metadata_rapier_feature_present", "metadata_default_off_rapier",
    "rapier_only_tree_no_cmake",
    "rapier_only_tests_pass", "rapier_only_test_count",
    "dual_backend_tests_pass", "dual_backend_test_count",
    "rapier_only_clippy_pass", "dual_backend_clippy_pass",
    "parity_process1_pass", "parity_process2_pass",
    "parity_json_wellformed", "parity_verdict_pass",
    "parity_hashes_replay_identical", "parity_max_dev_replay_identical",
    "parity_pos_within_tol", "parity_rot_within_tol",
    "parity_begin_overlap_ok", "parity_end_overlap_ok",
    "parity_phase_class_equal",
    "parity_jolt_determinism_hash", "parity_rapier_determinism_hash",
    "parity_max_pos_dev_m", "parity_max_rot_dev_deg",
    "parity_begin_overlap", "parity_end_overlap",
    "audit_fastpath_disclaimer", "audit_rapier3d_single_module",
    "step_time_secs",
)


def _fail(msg: str) -> None:
    print(f"[physics_rapier_parity_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def run(cmd, cwd: Path = ROOT, timeout: int = 1800, env_extra: dict | None = None):
    env = dict(os.environ)
    if env_extra:
        env.update(env_extra)
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout, env=env)
    return (
        r.returncode,
        r.stdout.decode("utf-8", "replace"),
        r.stderr.decode("utf-8", "replace"),
    )


def github_run_url() -> str:
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local"


# ————————————————————— 纯判定层(selftest 直接喂合成数据)—————————————————————


def judge_default_features(features: dict) -> list[str]:
    """§4.D1:feature `rapier` 存在但默认 off(cargo metadata 机验,非 grep)。纯函数。"""
    problems: list[str] = []
    if "rapier" not in features:
        problems.append("rurix-physics features 缺 `rapier`(G6.4 快路径 feature 须在位)")
    if "rapier" in features.get("default", []):
        problems.append(
            "rurix-physics default feature 集含 `rapier`"
            "(§4.D1:rapier 默认 off,生产默认 = Jolt,不替换默认)"
        )
    return problems


def audit_tree_no_cmake(tree_text: str) -> list[str]:
    """§4.D1:rapier-only 档 cargo tree 输出零 rurix-physics-sys/Jolt/cxx/cmake。纯函数。"""
    problems: list[str] = []
    for lineno, line in enumerate(tree_text.splitlines(), 1):
        m = TREE_FORBIDDEN_RE.search(line)
        if m:
            problems.append(
                f"cargo tree:{lineno}: rapier-only 档依赖树出现禁出名 {m.group(0)!r}"
                f"(§4.D1 纯 Rust 零 CMake;命中行 {line.strip()!r})"
            )
    return problems


def judge_parity_doc(doc: dict | None) -> list[str]:
    """单次 parity 进程 JSON(run_v1)的 §4.D3 判据 + 结构面。纯函数。

    问题串前缀 tag 供 evidence 粒度化:[wellformed] = 结构面(schema/N=300/重放 ≥5/
    阈值钉定值/确定性哈希形态),[verdict]/[pos_within_tol]/[rot_within_tol]/
    [begin_overlap_ok]/[end_overlap_ok]/[phase_class_equal] = §4.D3 判据面。
    """
    problems: list[str] = []
    if not isinstance(doc, dict):
        return ["[wellformed] PARITY_JSON 不是合法 JSON 对象"]
    if doc.get("schema") != PARITY_SCHEMA_ID:
        problems.append(f"[wellformed] schema != {PARITY_SCHEMA_ID!r}(={doc.get('schema')!r})")
    if doc.get("n_steps") != EXPECTED_N_STEPS:
        problems.append(
            f"[wellformed] n_steps={doc.get('n_steps')!r} != {EXPECTED_N_STEPS}(§4.D3 钉数 N=300)"
        )
    replays = doc.get("replays")
    if not isinstance(replays, int) or replays < EXPECTED_MIN_REPLAYS:
        problems.append(
            f"[wellformed] replays={replays!r} < {EXPECTED_MIN_REPLAYS}(判据⑤ ≥5 次重放)"
        )
    th = doc.get("thresholds")
    if not isinstance(th, dict):
        problems.append("[wellformed] thresholds 缺席")
        th = {}
    for key, want in (
        ("pos_tol_m", EXPECTED_POS_TOL_M),
        ("rot_tol_deg", EXPECTED_ROT_TOL_DEG),
        ("contact_overlap_min", EXPECTED_CONTACT_OVERLAP_MIN),
    ):
        got = th.get(key)
        if not isinstance(got, (int, float)) or isinstance(got, bool) or abs(got - want) > 1e-9:
            problems.append(
                f"[wellformed] thresholds.{key}={got!r} != 钉定值 {want}"
                "(阈值静默放宽即红;改阈值须同 PR 改本门)"
            )
    pb = doc.get("per_backend")
    if not isinstance(pb, dict):
        problems.append("[wellformed] per_backend 缺席")
        pb = {}
    for backend in ("jolt", "rapier"):
        entry = pb.get(backend)
        h = entry.get("determinism_hash") if isinstance(entry, dict) else None
        if not (isinstance(h, str) and re.fullmatch(r"[0-9a-f]{16}", h)):
            problems.append(
                f"[wellformed] per_backend.{backend}.determinism_hash={h!r} 非 16 位小写 hex"
                "(§4.A7(a) 后端内确定性哈希)"
            )
        if not isinstance(entry, dict) or entry.get("replays_bitwise_identical") is not True:
            problems.append(
                f"[wellformed] per_backend.{backend}.replays_bitwise_identical != true"
                "(进程内 5 重放逐位确定性,§4.A7(a))"
            )
    cb = doc.get("cross_backend")
    if not isinstance(cb, dict):
        problems.append("[wellformed] cross_backend 缺席")
        cb = {}
    pos = cb.get("max_pos_dev_m")
    if not isinstance(pos, (int, float)) or isinstance(pos, bool) or pos > EXPECTED_POS_TOL_M:
        problems.append(
            f"[pos_within_tol] max_pos_dev_m={pos!r} 超容差 {EXPECTED_POS_TOL_M} m(§4.D3 ①)"
        )
    rot = cb.get("max_rot_dev_deg")
    if not isinstance(rot, (int, float)) or isinstance(rot, bool) or rot > EXPECTED_ROT_TOL_DEG:
        problems.append(
            f"[rot_within_tol] max_rot_dev_deg={rot!r} 超容差 {EXPECTED_ROT_TOL_DEG}°(§4.D3 ①)"
        )
    bo = cb.get("begin_overlap")
    if not isinstance(bo, (int, float)) or isinstance(bo, bool) or bo < EXPECTED_CONTACT_OVERLAP_MIN:
        problems.append(
            f"[begin_overlap_ok] begin_overlap={bo!r} < {EXPECTED_CONTACT_OVERLAP_MIN}(§4.D3 ②)"
        )
    eo = cb.get("end_overlap")
    if not isinstance(eo, (int, float)) or isinstance(eo, bool) or eo < EXPECTED_CONTACT_OVERLAP_MIN:
        problems.append(
            f"[end_overlap_ok] end_overlap={eo!r} < {EXPECTED_CONTACT_OVERLAP_MIN}(§4.D3 ②)"
        )
    if cb.get("phase_class_equal") is not True:
        problems.append("[phase_class_equal] 逐对相位 RLE 等价类不一致(§4.D3 ②)")
    if doc.get("verdict") != "pass":
        problems.append(
            f"[verdict] verdict={doc.get('verdict')!r} != 'pass'(对拍侧自判红,红不充绿)"
        )
    return problems


def judge_replay_consistency(doc1: dict, doc2: dict) -> list[str]:
    """进程级重放一致(§4.D3 ⑤ 跨进程面):两独立进程 JSON 各后端确定性哈希
    逐位一致 + max 偏差逐值一致。tag:[hash]/[max_dev]。纯函数。"""
    problems: list[str] = []
    for backend in ("jolt", "rapier"):
        h1 = ((doc1.get("per_backend") or {}).get(backend) or {}).get("determinism_hash")
        h2 = ((doc2.get("per_backend") or {}).get(backend) or {}).get("determinism_hash")
        if h1 != h2:
            problems.append(
                f"[hash] {backend} 确定性哈希跨进程不一致:{h1!r} vs {h2!r}"
                "(进程级重放须逐位一致,§4.A7(a) 跨进程面)"
            )
    for key in ("max_pos_dev_m", "max_rot_dev_deg"):
        v1 = (doc1.get("cross_backend") or {}).get(key)
        v2 = (doc2.get("cross_backend") or {}).get(key)
        if v1 != v2:
            problems.append(
                f"[max_dev] cross_backend.{key} 跨进程不一致:{v1!r} vs {v2!r}"
                "(max 偏差须逐值一致)"
            )
    return problems


def module_head(text: str) -> str:
    """提取 Rust 模块头文档注释(文件首个非 `//!` 行截断)。"""
    head: list[str] = []
    for line in text.splitlines():
        if line.startswith("//!"):
            head.append(line)
        else:
            break
    return "\n".join(head)


def audit_fastpath_disclaimer(texts: dict[str, str]) -> list[str]:
    """§4.D4:lib.rs 与 rapier.rs 模块头含「快路径 ≠ 性能/稳定性默认」字样。纯函数。"""
    problems: list[str] = []
    for path in (PHYSICS_LIB_RS, PHYSICS_RAPIER_RS):
        text = texts.get(path)
        if text is None:
            problems.append(f"{path} 缺席(§4.D4 文档口径审计面缺位)")
            continue
        if FASTPATH_DISCLAIMER not in module_head(text):
            problems.append(
                f"{path} 模块头缺「{FASTPATH_DISCLAIMER}」字样"
                "(§4.D4:快路径非性能/稳定性默认,不替换 Jolt 生产默认)"
            )
    return problems


def audit_rapier3d_single_module(files: dict[str, str]) -> list[str]:
    """§4.C4(v1.4 收窄,与步骤 88 互锁):rapier.rs 存在且为 src 面唯一含
    `rapier3d` 的 .rs 文件。纯函数。"""
    problems: list[str] = []
    if PHYSICS_RAPIER_RS not in files:
        problems.append(
            f"{PHYSICS_RAPIER_RS} 缺席(G6.4 Rapier 快路径 sanctioned 消费模块须在位)"
        )
    hits = sorted(p for p, t in files.items() if RAPIER3D_RE.search(t))
    for p in hits:
        if p != PHYSICS_RAPIER_RS:
            problems.append(
                f"{p}: 出现 `rapier3d` 名(§4.C4 v1.4:收敛于 src/rapier.rs 单一 "
                "sanctioned 消费模块,其余 src 文件零命中)"
            )
    if PHYSICS_RAPIER_RS in files and PHYSICS_RAPIER_RS not in hits:
        problems.append(
            f"{PHYSICS_RAPIER_RS} 不含 `rapier3d` 消费(sanctioned 模块名存实亡,门需校准)"
        )
    return problems


# ————————————————————— IO 采集层 —————————————————————


def collect_rs_files(dirs: list[str]) -> dict[str, str]:
    files: dict[str, str] = {}
    for d in dirs:
        base = ROOT / d
        if not base.is_dir():
            continue
        for p in sorted(base.rglob("*.rs")):
            files[p.relative_to(ROOT).as_posix()] = p.read_text(encoding="utf-8")
    return files


def rustc_version() -> str:
    try:
        code, out, _err = run(["rustc", "--version"], timeout=60)
    except FileNotFoundError:
        return "rustc 不在 PATH"
    return out.strip() if code == 0 else "rustc --version 探测失败"


# ————————————————————— red 自检(反 YAML-only)—————————————————————


def _tags(problems: list[str]) -> set[str]:
    return {
        m.group(1)
        for p in problems
        if (m := re.match(r"\[(\w+)\]", p)) is not None
    }


def _parity_doc_ok() -> dict:
    """合成合法 parity run_v1 JSON(镜像 tests/parity.rs parity_json 落盘形态)。"""
    return {
        "schema": "physics_rapier_parity/run_v1",
        "n_steps": 300,
        "replays": 5,
        "thresholds": {"pos_tol_m": 0.82, "rot_tol_deg": 93.0, "contact_overlap_min": 0.99},
        "per_backend": {
            "jolt": {"determinism_hash": "bfa449a7a5515449", "replays_bitwise_identical": True},
            "rapier": {"determinism_hash": "7e7ddda2f21e23d0", "replays_bitwise_identical": True},
        },
        "cross_backend": {
            "max_pos_dev_m": 0.541314125,
            "max_rot_dev_deg": 61.774520874,
            "bitwise_identical_frames": 0,
            "begin_overlap": 1.0,
            "end_overlap": 1.0,
            "phase_class_equal": True,
            "per_step": [[0, 0.001019716, 0.0]],
        },
        "verdict": "pass",
    }


_LIB_HEAD_OK = (
    "//! rurix-physics — Rurix 引擎物理库。\n"
    "//!\n"
    "//! **快路径 ≠ 性能/稳定性默认**(§4.D4):Rapier 路径价值 = 纯 Rust/无 CMake\n"
    "//! CI 面与第二实现交叉验证;生产默认 = Jolt。\n"
    "\n"
    "#![forbid(unsafe_code)]\n"
)
_RAPIER_HEAD_OK = (
    "//! Rapier 快路径第二后端(G6.4,RFC-0017 §4.D;验收门 G-G6-5)。\n"
    "//!\n"
    "//! **快路径 ≠ 性能/稳定性默认**:Rapier 路径价值 = 纯 Rust/无 CMake CI 面。\n"
    "\n"
    "use std::collections::BTreeMap;\n"
)


def red_self_test() -> None:
    """合成数据断言各纯判定层能区分红绿;门失效即 exit 1。"""
    # 判据 1:默认 off 核验
    good_features = {"default": ["jolt"], "jolt": ["dep:rurix-physics-sys"],
                     "rapier": ["dep:rapier3d"]}
    if judge_default_features(good_features):
        _fail("red 自检失败:合法 features(rapier 存在 + 默认 off)被误判红(门过严)")
    bad_default = {"default": ["jolt", "rapier"], "jolt": [], "rapier": ["dep:rapier3d"]}
    if not judge_default_features(bad_default):
        _fail("red 自检失败:default 集含 rapier 未判红(§4.D1,门失效)")
    missing_rapier = {"default": ["jolt"], "jolt": ["dep:rurix-physics-sys"]}
    if not judge_default_features(missing_rapier):
        _fail("red 自检失败:feature rapier 缺席未判红(门失效)")
    # 判据 2:无 CMake 树核验
    clean_tree = (
        "rurix-physics v1.0.0 (H:\\rurix\\src\\rurix-physics)\n"
        "rurix-render v1.0.0 (H:\\rurix\\src\\rurix-render)\n"
        "rapier3d v0.33.0\nparry3d v0.28.0\nnalgebra v0.35.0\n"
    )
    if audit_tree_no_cmake(clean_tree):
        _fail("red 自检失败:干净 rapier-only 依赖树被误判红(门过严)")
    for bad_line, label in (
        ("rurix-physics-sys v1.0.0 (H:\\rurix\\src\\rurix-physics-sys)", "sys crate"),
        ("jolt-physics v0.1.0", "Jolt"),
        ("JoltC vendor build", "Jolt 大写"),
        ("cxx v1.0.100", "cxx"),
        ("cmake v0.1.50", "cmake"),
    ):
        if not audit_tree_no_cmake(clean_tree + bad_line + "\n"):
            _fail(f"red 自检失败:依赖树 {label} 禁出名未判红(§4.D1,门失效)")
    # 判据 5:parity 单次 §4.D3 判据 + 进程级重放
    ok_doc = _parity_doc_ok()
    if judge_parity_doc(ok_doc):
        _fail("red 自检失败:合法 parity JSON 被误判红(门过严)")
    if judge_replay_consistency(ok_doc, _parity_doc_ok()):
        _fail("red 自检失败:两进程一致 JSON 被误判红(门过严)")
    bad_pos = _parity_doc_ok()
    bad_pos["cross_backend"]["max_pos_dev_m"] = 0.9
    if "pos_within_tol" not in _tags(judge_parity_doc(bad_pos)):
        _fail("red 自检失败:位置偏差超容差未判红(§4.D3 ①,门失效)")
    bad_rot = _parity_doc_ok()
    bad_rot["cross_backend"]["max_rot_dev_deg"] = 120.0
    if "rot_within_tol" not in _tags(judge_parity_doc(bad_rot)):
        _fail("red 自检失败:旋转偏差超容差未判红(§4.D3 ①,门失效)")
    bad_bo = _parity_doc_ok()
    bad_bo["cross_backend"]["begin_overlap"] = 0.5
    if "begin_overlap_ok" not in _tags(judge_parity_doc(bad_bo)):
        _fail("red 自检失败:Begin 重叠率 <0.99 未判红(§4.D3 ②,门失效)")
    bad_eo = _parity_doc_ok()
    bad_eo["cross_backend"]["end_overlap"] = 0.5
    if "end_overlap_ok" not in _tags(judge_parity_doc(bad_eo)):
        _fail("red 自检失败:End 重叠率 <0.99 未判红(§4.D3 ②,门失效)")
    bad_rle = _parity_doc_ok()
    bad_rle["cross_backend"]["phase_class_equal"] = False
    if "phase_class_equal" not in _tags(judge_parity_doc(bad_rle)):
        _fail("red 自检失败:相位 RLE 等价类不一致未判红(§4.D3 ②,门失效)")
    bad_verdict = _parity_doc_ok()
    bad_verdict["verdict"] = "fail"
    if "verdict" not in _tags(judge_parity_doc(bad_verdict)):
        _fail("red 自检失败:verdict=fail 未判红(红不充绿,门失效)")
    loose_th = _parity_doc_ok()
    loose_th["thresholds"]["pos_tol_m"] = 1.5
    if "wellformed" not in _tags(judge_parity_doc(loose_th)):
        _fail("red 自检失败:阈值静默放宽未判红(阈值钉定核验,门失效)")
    bad_hash = _parity_doc_ok()
    bad_hash["per_backend"]["rapier"]["determinism_hash"] = "XYZ"
    if "wellformed" not in _tags(judge_parity_doc(bad_hash)):
        _fail("red 自检失败:确定性哈希形态非法未判红(门失效)")
    few_replays = _parity_doc_ok()
    few_replays["replays"] = 3
    if "wellformed" not in _tags(judge_parity_doc(few_replays)):
        _fail("red 自检失败:replays <5 未判红(判据⑤,门失效)")
    wrong_n = _parity_doc_ok()
    wrong_n["n_steps"] = 100
    if "wellformed" not in _tags(judge_parity_doc(wrong_n)):
        _fail("red 自检失败:n_steps !=300 未判红(§4.D3 钉数,门失效)")
    if "wellformed" not in _tags(judge_parity_doc(None)):
        _fail("red 自检失败:非法 JSON(非对象)未判红(门失效)")
    drift_hash = _parity_doc_ok()
    drift_hash["per_backend"]["rapier"]["determinism_hash"] = "0000000000000000"
    if "hash" not in _tags(judge_replay_consistency(ok_doc, drift_hash)):
        _fail("red 自检失败:rapier 哈希跨进程漂移未判红(进程级重放,门失效)")
    drift_dev = _parity_doc_ok()
    drift_dev["cross_backend"]["max_pos_dev_m"] = 0.123
    if "max_dev" not in _tags(judge_replay_consistency(ok_doc, drift_dev)):
        _fail("red 自检失败:max 偏差跨进程漂移未判红(进程级重放,门失效)")
    # 判据 6:§4.D4 文档口径
    good_texts = {PHYSICS_LIB_RS: _LIB_HEAD_OK, PHYSICS_RAPIER_RS: _RAPIER_HEAD_OK}
    if audit_fastpath_disclaimer(good_texts):
        _fail("red 自检失败:合法模块头(含钉定字样)被误判红(门过严)")
    missing_phrase = {
        PHYSICS_LIB_RS: _LIB_HEAD_OK,
        PHYSICS_RAPIER_RS: "//! Rapier 快路径第二后端。\n\nuse std::collections::BTreeMap;\n",
    }
    if not audit_fastpath_disclaimer(missing_phrase):
        _fail("red 自检失败:rapier.rs 模块头缺钉定字样未判红(§4.D4,门失效)")
    missing_file = {PHYSICS_LIB_RS: _LIB_HEAD_OK}
    if not audit_fastpath_disclaimer(missing_file):
        _fail("red 自检失败:rapier.rs 缺席未判红(门失效)")
    good_files = {
        PHYSICS_RAPIER_RS: "use rapier3d::dynamics::RigidBodySet;\n",
        PHYSICS_LIB_RS: "mod rapier;\n",
        "src/rurix-physics/src/world.rs": "// 薄分派,零原生名\n",
        "src/rurix-render/src/scene.rs": "// clean render code\n",
    }
    if audit_rapier3d_single_module(good_files):
        _fail("red 自检失败:rapier3d 单模块收敛合法面被误判红(门过严)")
    extra_hit = dict(good_files)
    extra_hit["src/rurix-physics/src/world.rs"] = "let _ = core::mem::size_of::<rapier3d::math::Vector>();\n"
    if not audit_rapier3d_single_module(extra_hit):
        _fail("red 自检失败:world.rs 出现 rapier3d 名未判红(v1.4 收窄,门失效)")
    render_hit = dict(good_files)
    render_hit["src/rurix-render/src/scene.rs"] = "// rapier3d mention\n"
    if not audit_rapier3d_single_module(render_hit):
        _fail("red 自检失败:rurix-render 出现 rapier3d 名未判红(门失效)")
    no_rapier_rs = {k: v for k, v in good_files.items() if k != PHYSICS_RAPIER_RS}
    if not audit_rapier3d_single_module(no_rapier_rs):
        _fail("red 自检失败:rapier.rs 缺席未判红(门失效)")
    empty_rapier = dict(good_files)
    empty_rapier[PHYSICS_RAPIER_RS] = "// 空模块占位(零原生消费)\n"
    if not audit_rapier3d_single_module(empty_rapier):
        _fail("red 自检失败:sanctioned 模块零 rapier3d 消费未判红(名存实亡,门失效)")


# ————————————————————— 检查段 —————————————————————


def metadata_section(results: dict, failures: list[str]) -> bool:
    """判据 1:§4.D1 默认 off 核验(cargo metadata 机验,非 grep)。"""
    try:
        code, out, err = run(["cargo", "metadata", "--format-version", "1", "--no-deps"],
                             timeout=600)
    except FileNotFoundError:
        results["metadata_rapier_feature_present"] = False
        results["metadata_default_off_rapier"] = False
        failures.append("metadata 段: cargo 不在 PATH")
        return False
    if code != 0:
        results["metadata_rapier_feature_present"] = False
        results["metadata_default_off_rapier"] = False
        failures.append(f"metadata 段: cargo metadata exit {code}:{err.strip()[:400]!r}")
        return False
    try:
        meta = json.loads(out)
    except json.JSONDecodeError:
        results["metadata_rapier_feature_present"] = False
        results["metadata_default_off_rapier"] = False
        failures.append("metadata 段: cargo metadata 输出非合法 JSON")
        return False
    features = None
    for pkg in meta.get("packages", []):
        if pkg.get("name") == "rurix-physics":
            features = pkg.get("features") or {}
            break
    if features is None:
        results["metadata_rapier_feature_present"] = False
        results["metadata_default_off_rapier"] = False
        failures.append("metadata 段: cargo metadata 缺 rurix-physics 包")
        return False
    problems = judge_default_features(features)
    results["metadata_rapier_feature_present"] = "rapier" in features
    results["metadata_default_off_rapier"] = "rapier" not in features.get("default", [])
    for p in problems:
        failures.append(f"metadata 段: {p}")
    print(
        f"[physics_rapier_parity_smoke] metadata 段: feature rapier "
        f"在位={results['metadata_rapier_feature_present']}, "
        f"默认 off={results['metadata_default_off_rapier']}"
    )
    return not problems


def tree_section(results: dict, failures: list[str]) -> bool:
    """判据 2:rapier-only 档依赖树零 CMake 面(§4.D1「纯 Rust 零 CMake」佐证)。"""
    cmd = ["cargo", "tree", "-p", "rurix-physics",
           "--no-default-features", "--features", "rapier",
           "-e", "normal", "--prefix", "none"]
    try:
        code, out, err = run(cmd, timeout=600)
    except FileNotFoundError:
        results["rapier_only_tree_no_cmake"] = False
        failures.append("tree 段: cargo 不在 PATH")
        return False
    problems = audit_tree_no_cmake(out)
    if code != 0:
        problems.insert(0, f"`{' '.join(cmd)}` exit {code}:{err.strip()[:400]!r}")
    results["rapier_only_tree_no_cmake"] = not problems
    for p in problems:
        failures.append(f"tree 段: {p}")
    print(
        f"[physics_rapier_parity_smoke] tree 段: rc={code}, "
        f"{'PASS' if not problems else f'RED({len(problems)} 处)'}"
    )
    return not problems


def test_section(results: dict, failures: list[str]) -> bool:
    """判据 3:cargo test 两腿(rapier-only / 双后端)exit 0,汇总计数入 evidence。"""
    ok = True
    for leg, cmd in TEST_LEGS:
        try:
            code, out, err = run(cmd)
        except FileNotFoundError:
            results[f"{leg}_tests_pass"] = False
            results[f"{leg}_test_count"] = 0
            failures.append(f"cargo test 段: cargo 不在 PATH({leg} 档未能执行)")
            ok = False
            continue
        blob = out + err
        results[f"{leg}_test_count"] = sum(int(x) for x in TEST_OK_RE.findall(blob))
        passed = code == 0 and results[f"{leg}_test_count"] > 0
        results[f"{leg}_tests_pass"] = passed
        if code != 0:
            print(f"[physics_rapier_parity_smoke] cargo test 段 {leg} 档输出尾部:",
                  file=sys.stderr)
            print(blob[-2400:], file=sys.stderr)
            failures.append(f"cargo test 段: `{' '.join(cmd)}` exit {code}(构建/单测红)")
        elif results[f"{leg}_test_count"] == 0:
            failures.append(
                f"cargo test 段: {leg} 档通过计数为 0(测试面坍缩,反 vacuous-green)"
            )
        if not passed:
            ok = False
        print(
            f"[physics_rapier_parity_smoke] cargo test 段 {leg}: rc={code}, "
            f"全过计数={results[f'{leg}_test_count']}"
        )
    return ok


def clippy_section(results: dict, failures: list[str]) -> bool:
    """判据 4:rapier 两 tier clippy --all-targets -D warnings 零告警。"""
    ok = True
    for leg, cmd in CLIPPY_LEGS:
        try:
            code, out, err = run(cmd)
        except FileNotFoundError:
            results[f"{leg}_clippy_pass"] = False
            failures.append(f"clippy 段: cargo 不在 PATH({leg} 档未能执行)")
            ok = False
            continue
        results[f"{leg}_clippy_pass"] = code == 0
        if code != 0:
            blob = out + err
            print(f"[physics_rapier_parity_smoke] clippy 段 {leg} 档输出尾部:",
                  file=sys.stderr)
            print(blob[-2400:], file=sys.stderr)
            failures.append(f"clippy 段: `{' '.join(cmd)}` exit {code}(告警非零)")
            ok = False
        print(f"[physics_rapier_parity_smoke] clippy 段 {leg}: rc={code}")
    return ok


def parity_section(results: dict, failures: list[str]) -> bool:
    """判据 5:parity 进程级重放一致(双进程 PARITY_JSON + 单次 §4.D3 判据全过)。"""
    tmp = Path(tempfile.mkdtemp(prefix="rurix_g64_parity_"))
    docs: list[dict | None] = []
    ok = True
    try:
        for idx in (1, 2):
            pj = tmp / f"parity_run{idx}.json"
            try:
                code, out, err = run(PARITY_CMD, env_extra={"PARITY_JSON": str(pj)})
            except FileNotFoundError:
                results[f"parity_process{idx}_pass"] = False
                failures.append(f"parity 段: cargo 不在 PATH(进程 {idx} 未能执行)")
                docs.append(None)
                ok = False
                continue
            doc = None
            if pj.is_file():
                try:
                    doc = json.loads(pj.read_text(encoding="utf-8"))
                except json.JSONDecodeError:
                    doc = None
            docs.append(doc)
            results[f"parity_process{idx}_pass"] = code == 0 and doc is not None
            if code != 0:
                blob = out + err
                print(f"[physics_rapier_parity_smoke] parity 段进程 {idx} 输出尾部:",
                      file=sys.stderr)
                print(blob[-2400:], file=sys.stderr)
                failures.append(
                    f"parity 段: 进程 {idx} `{' '.join(PARITY_CMD)}` exit {code}(对拍测试红)"
                )
                ok = False
            if doc is None:
                failures.append(f"parity 段: 进程 {idx} PARITY_JSON 未落盘或非法 JSON")
                ok = False
            print(
                f"[physics_rapier_parity_smoke] parity 进程 {idx}: rc={code}, "
                f"JSON={'ok' if doc else '缺席/非法'}"
            )
    finally:
        # 临时件清进系统 temp,勿留仓内。
        shutil.rmtree(tmp, ignore_errors=True)
    # 单次 §4.D3 判据(tag 粒度化入 checks;两进程各自全过才绿)
    bad_tags: set[str] = set()
    for idx, doc in enumerate(docs, 1):
        for prob in judge_parity_doc(doc):
            m = re.match(r"\[(\w+)\]", prob)
            if m:
                bad_tags.add(m.group(1))
            failures.append(f"parity 段: 进程 {idx} {prob}")
    docs_ok = all(d is not None for d in docs)
    for tag, key in PARITY_TAG_KEYS.items():
        results[key] = docs_ok and tag not in bad_tags
    if bad_tags or not docs_ok:
        ok = False
    # 进程级重放一致(两进程各自哈希逐位 + max 偏差逐值)
    if docs_ok:
        replay_problems = judge_replay_consistency(docs[0], docs[1])
        replay_bad = _tags(replay_problems)
        results["parity_hashes_replay_identical"] = "hash" not in replay_bad
        results["parity_max_dev_replay_identical"] = "max_dev" not in replay_bad
        for p in replay_problems:
            failures.append(f"parity 段: {p}")
        if replay_problems:
            ok = False
    else:
        results["parity_hashes_replay_identical"] = False
        results["parity_max_dev_replay_identical"] = False
    # 数值留证(入 checks 不进硬门,P-09;禁跨引擎逐位 → 逐位帧数不录判据)
    if docs[0] is not None:
        cb = docs[0].get("cross_backend") or {}
        pb = docs[0].get("per_backend") or {}
        results["parity_max_pos_dev_m"] = cb.get("max_pos_dev_m")
        results["parity_max_rot_dev_deg"] = cb.get("max_rot_dev_deg")
        results["parity_begin_overlap"] = cb.get("begin_overlap")
        results["parity_end_overlap"] = cb.get("end_overlap")
        results["parity_jolt_determinism_hash"] = (pb.get("jolt") or {}).get("determinism_hash")
        results["parity_rapier_determinism_hash"] = (pb.get("rapier") or {}).get("determinism_hash")
    return ok


def audit_section(results: dict, failures: list[str]) -> bool:
    """判据 6:§4.D4 文档口径 grep(模块头钉定字样 + rapier3d 单模块收敛)。"""
    texts: dict[str, str] = {}
    for path in (PHYSICS_LIB_RS, PHYSICS_RAPIER_RS):
        p = ROOT / path
        if p.is_file():
            texts[path] = p.read_text(encoding="utf-8")
    # src 面(tests/ 不入本门面;步骤 88 另覆 crate 全 .rs 面)。
    src_files = collect_rs_files(["src/rurix-physics/src", "src/rurix-render/src"])
    checks = [
        ("audit_fastpath_disclaimer", audit_fastpath_disclaimer(texts)),
        ("audit_rapier3d_single_module", audit_rapier3d_single_module(src_files)),
    ]
    ok = True
    for key, problems in checks:
        results[key] = not problems
        for p in problems:
            failures.append(f"§4.D4 审计门: {p}")
        if problems:
            ok = False
        print(
            f"[physics_rapier_parity_smoke] 审计 {key}: "
            f"{'PASS' if not problems else f'RED({len(problems)} 处)'}"
        )
    return ok


def write_evidence(results: dict, host_ok: bool, machine: str) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now(_dt.timezone.utc).replace(microsecond=0)
    doc = {
        "schema_version": 1,
        "subject": "physics_rapier_parity_smoke",
        "milestone": "G6.4 / G-G6-5 (RFC-0017 §4.D)",
        "step": 90,
        "host_section_pass": host_ok,
        # 纯 host 门,无 device 段(G6 CI_GATES §2.90)→ null。
        "device_section_rc": None,
        "machine": machine,
        "checks": {k: results.get(k) for k in CHECK_KEYS if results.get(k) is not None},
        "physics_rapier_parity_smoke_ok": host_ok,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    ev = EVIDENCE_DIR / f"physics_rapier_parity_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n",
                  encoding="utf-8", newline="\n")
    print(f"[physics_rapier_parity_smoke] 写 evidence {ev.relative_to(ROOT)}; "
          f"run_url={doc['run_url']}")


def main() -> int:
    if "--selftest" in sys.argv:
        red_self_test()
        print("[physics_rapier_parity_smoke] selftest PASS"
              "(红绿判别有效;未跑 cargo、未写 evidence)")
        return 0
    t0 = time.monotonic()
    machine = f"{platform.platform()}; {rustc_version()}"
    results: dict = {}
    failures: list[str] = []
    meta_ok = metadata_section(results, failures)
    tree_ok = tree_section(results, failures)
    test_ok = test_section(results, failures)
    clippy_ok = clippy_section(results, failures)
    parity_ok = parity_section(results, failures)
    audits_ok = audit_section(results, failures)
    results["step_time_secs"] = round(time.monotonic() - t0, 3)
    host_ok = meta_ok and tree_ok and test_ok and clippy_ok and parity_ok and audits_ok
    write_evidence(results, host_ok, machine)
    if failures:
        print("[physics_rapier_parity_smoke] FAIL 判据红清单:", file=sys.stderr)
        for f in failures:
            print(f"  - {f}", file=sys.stderr)
        return 1
    print("[physics_rapier_parity_smoke] PASS(host 恒跑,纯 host 门)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
