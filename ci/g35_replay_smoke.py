#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor:Claude(G35 GPU 粒子系统 G35-9 确定性回放/回滚)
"""G35-9:确定性回放/回滚门冒烟(g35.wave9.replay;粒子输入 journal +
回放重仿真位级 + 回滚 N 帧重仿真位级——兑现「确定性 GPU 粒子」总口径,
反打 Niagara 网络不复制粒子本体、GPU 模拟不可回放。journal/检查点结构 =
src/rurix-render/src/particles/replay.rs〔"G35J"/"G35C" v1 冻结布局,
全域小端手写 to_le_bytes〕;probe = src/rurix-render/src/bin/
g35_replay_device.rs 经 vk::run_compute 消费 W2 七 kernel 真跑,device
帧链/digest 链式与 g35_particle_core_device 逐字同模;RFC-0049 §4.12)。

八面判据(facts 闭集;本门全位级——整数/digest 域零容差,无 f32 budget
条目如实登记〔f32 容差面归 g35.wave2 门,digest 已按字节覆盖 f32 流〕):
1. **kernels_spv_valid**:rurixc 现编 7 kernel 消费面(g35_sim/
   g35_particle_compact/g35_emit/g35_indirect_args + scan 三件,W1/W2 冻结
   面只消费不修改)+ spirv-val 全绿 + 冻结消费面 sha256 快照在档。
2. **journal_record_replay_bitexact**:录制腿(确定性脚本 64 帧,journal +
   digest 链 + 检查点落 .tmp)→ 回放腿**仅凭 journal 重建输入**(seed/
   emitter/dt/emit 序列)GPU 重仿真,逐帧 digest 与录制链位级全等(首异帧
   = -1;record/replay 双件 journal_sha256 与链尾 digest 三方互核)。
3. **checkpoint_restore_bitexact**:回滚腿从检查点 k=16 上传恢复 device
   缓冲,恢复帧自身 digest 与录制链位级全等(digest 链种子 = 录制链
   digest[k−1])。
4. **rollback_resim_bitexact**:恢复后重仿真至帧 j=48 逐帧位级全等且
   digest[48] 全等(frames_resimmed == 33;网络回滚语义 = 检查点 + 输入
   重放)。
5. **first_divergence_frame_witness**:红臂篡改 journal 帧 32 emit_count
   (+1)→ 回放 digest 链首异帧精确 == 32(分歧可定位见证——确定性系统
   独有性质,Niagara GPU sim 做不到)。
6. **determinism_double_run**:同 journal 输入 GPU 重仿真双跑 digest 链
   位级一致。
7. **red_arm_effective**:journal-tamper 臂检出(篡改后链尾 digest 必异
   ——digest 判据对输入敏感性证明,防镂空 digest 冒充)。
8. **frame_ms_measured**:device 7 dispatch 链逐帧墙钟均值 measured_local
   诚实登记(record/replay/rollback 三腿;含 run_compute 逐 dispatch 会话
   重建开销,非帧率对标)。

host 金标准平行对拍维持:各腿 host 整数流(flags/scan_out/seg_offsets/
pid/args)零容差全帧位级(host_parallel_bitexact),破则腿 state=fail。

三态:无 Vulkan loader/设备/SPV → DEV_ENV_DEGRADE 退 0(不冒充 PASS);
RURIX_REQUIRE_REAL=1 下 DEV_ENV 降级翻硬 FAIL(禁 mock 充真跑)。

用法:
  py -3 ci/g35_replay_smoke.py --selftest
  py -3 ci/g35_replay_smoke.py --gate g35.wave9.replay [--frames 64] [--cap 65536] [--seed 42]
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

from gpu_device_lock import gpu_device_lock  # noqa: E402

GATE_KEY = "g35.wave9.replay"
SUBJECT = "g35_replay"
WAVE = "G35.9"
TAG = "g35_replay"
GATE_SCHEMA_PATH = ROOT / "milestones" / "g35" / "g35_replay_gate_evidence_schema.json"
GATE_SCHEMA_ID = "rurix.g35.replay_gate_evidence.v1"
KERNEL_DIR = ROOT / "src" / "rurix-render" / "kernels"
# 消费面 7 kernel(W2 四件 + W1 scan 三件;G35-9 只消费不修改)。
CONSUMED_KERNELS = (
    "g35_sim", "g35_particle_compact", "g35_emit", "g35_indirect_args",
    "g35_scan_seg_sum", "g35_scan_spine", "g35_scan_seg_apply",
)
FROZEN_CONSUMED_PATHS = [
    # G35-9 消费不修改承诺面(七 kernel + host 契约/帧协议/scan 金标准)——
    # sha256 快照在档 = 漂移守护基线(g35_particle_core FROZEN 同律)。
    "src/rurix-render/kernels/g35_sim.rx",
    "src/rurix-render/kernels/g35_particle_compact.rx",
    "src/rurix-render/kernels/g35_emit.rx",
    "src/rurix-render/kernels/g35_indirect_args.rx",
    "src/rurix-render/kernels/g35_scan_seg_sum.rx",
    "src/rurix-render/kernels/g35_scan_spine.rx",
    "src/rurix-render/kernels/g35_scan_seg_apply.rx",
    "src/rurix-render/src/particles/mod.rs",
    "src/rurix-render/src/particles/core.rs",
    "src/rurix-render/src/particles/scan.rs",
]
WORK = ROOT / ".tmp" / "g35_gates" / "replay"
EXE_SUFFIX = ".exe" if sys.platform == "win32" else ""
BIN = ROOT / "target" / "debug" / f"g35_replay_device{EXE_SUFFIX}"
JOURNAL = WORK / "journal.g35j"
DIGEST_CHAIN = WORK / "digest_chain.txt"

# 冻结腿参数(RFC-0049 §4.12 交付面字面)。
ROLLBACK_K = 16
ROLLBACK_TO = 48
TAMPER_FRAME = 32
CHECKPOINT_INTERVAL = 16

DIGEST_RE = re.compile(r"^sha256:[0-9a-f]{64}$")
FAILURES: list[str] = []

FACT_IDS = [
    "kernels_spv_valid",
    "journal_record_replay_bitexact",
    "checkpoint_restore_bitexact",
    "rollback_resim_bitexact",
    "first_divergence_frame_witness",
    "determinism_double_run",
    "red_arm_effective",
    "frame_ms_measured",
]


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def run(cmd: list[str], timeout: int = 7200, env: dict | None = None) -> subprocess.CompletedProcess:
    note(f"$ {' '.join(str(c) for c in cmd)}")
    return subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True, timeout=timeout, env=env)


def device_env() -> dict[str, str]:
    env = dict(os.environ)
    env["RURIX_REQUIRE_REAL"] = "1"
    env["RURIX_VK_VALIDATION"] = "1"
    return env


# ---------------------------------------------------------------------------
# 判读器(selftest 红绿两臂消费面;全纯函数零 GPU)
# ---------------------------------------------------------------------------


def _int(v) -> bool:
    return isinstance(v, int) and not isinstance(v, bool)


def _digest(v) -> bool:
    return isinstance(v, str) and DIGEST_RE.match(v) is not None


def record_ok(rec: dict) -> bool:
    """录制腿健全判:state=pass + host 整数平行位级 + digest/帧数形态 +
    检查点面(K=16 在档且含回滚腿消费的 k=16 帧)。"""
    return (
        rec.get("state") == "pass"
        and rec.get("host_parallel_bitexact") is True
        and _digest(rec.get("digest_final"))
        and _int(rec.get("frames"))
        and rec["frames"] >= 1
        and rec.get("checkpoint_interval") == CHECKPOINT_INTERVAL
        and isinstance(rec.get("checkpoint_frames"), list)
        and ROLLBACK_K in rec.get("checkpoint_frames", [])
        and _digest(rec.get("journal_sha256"))
    )


def record_replay_ok(rec: dict, rep: dict) -> bool:
    """② journal 录制/回放位级判:回放旗标 + 首异帧 -1 + 链尾三方互核
    (record.digest_final == replay.digest_recorded_final ==
    digest_replay_final)+ journal_sha256 双件互核 + host 平行位级。"""
    return (
        record_ok(rec)
        and rep.get("record_replay_bitexact") is True
        and rep.get("first_divergence") == -1
        and _int(rep.get("first_divergence"))
        and _digest(rep.get("digest_recorded_final"))
        and _digest(rep.get("digest_replay_final"))
        and rep["digest_replay_final"] == rep["digest_recorded_final"]
        and rep["digest_recorded_final"] == rec.get("digest_final")
        and rep.get("journal_sha256") == rec.get("journal_sha256")
        and rep.get("host_parallel_bitexact") is True
    )


def checkpoint_restore_ok(rb: dict) -> bool:
    """③ 检查点恢复判:恢复帧自身 digest 全等旗标 + 实际消费检查点帧 ==
    k == 16(冻结)+ host 平行位级。"""
    return (
        rb.get("restore_frame_digest_match") is True
        and rb.get("k") == ROLLBACK_K
        and rb.get("checkpoint_frame") == ROLLBACK_K
        and rb.get("host_parallel_bitexact") is True
    )


def rollback_ok(rb: dict) -> bool:
    """④ 回滚重仿真判:③ 合取 + 逐帧全等旗标 + digest[j] 全等(旗标 +
    digest 对互核)+ j == 48 + frames_resimmed == 33。"""
    return (
        checkpoint_restore_ok(rb)
        and rb.get("resim_bitexact") is True
        and rb.get("digest_at_j_match") is True
        and rb.get("to") == ROLLBACK_TO
        and rb.get("frames_resimmed") == ROLLBACK_TO - ROLLBACK_K + 1
        and _digest(rb.get("digest_recorded_at_j"))
        and _digest(rb.get("digest_resim_at_j"))
        and rb["digest_recorded_at_j"] == rb["digest_resim_at_j"]
    )


def witness_ok(red: dict) -> bool:
    """⑤ 首异帧见证判:journal-tamper 臂首异帧精确 == 篡改帧 == 32
    (分歧可定位——早异/晚异/漏检皆红)。"""
    return (
        red.get("arm") == "journal-tamper"
        and red.get("first_divergence") == TAMPER_FRAME
        and _int(red.get("first_divergence"))
        and red.get("expected_divergence") == TAMPER_FRAME
        and red.get("tampered_frame") == TAMPER_FRAME
    )


def determinism_ok(rep: dict) -> bool:
    """⑥ 双跑位级判:旗标 + run1/run2 链尾 digest 形态合法且全等互核。"""
    a, b = rep.get("digest_replay_final"), rep.get("digest_run2_final")
    return (
        rep.get("determinism_double_run") is True
        and _digest(a)
        and _digest(b)
        and a == b
    )


def red_ok(red: dict) -> bool:
    """⑦ RED 臂判:journal-tamper 检出 + 录制/篡改链尾 digest 形态合法且
    必异(防镂空 digest 冒充)。"""
    g, r = red.get("digest_recorded_final"), red.get("digest_tampered_final")
    return (
        red.get("arm") == "journal-tamper"
        and red.get("detected") is True
        and _digest(g)
        and _digest(r)
        and g != r
    )


def frame_ms_sane(v) -> bool:
    """⑧ frame_ms 登记面健全判:有限正数(诚实登记非阈门)。"""
    return (
        isinstance(v, (int, float))
        and not isinstance(v, bool)
        and math.isfinite(v)
        and v > 0
    )


# ---------------------------------------------------------------------------
# gate 腿
# ---------------------------------------------------------------------------


def build_or_fail(argv: list[str], what: str) -> bool:
    r = run(argv)
    if r.returncode != 0:
        fail(f"{what} 构建失败: {(r.stdout + r.stderr)[-400:]}")
        return False
    return True


def sha256_of(p: Path) -> str:
    return "sha256:" + hashlib.sha256(p.read_bytes()).hexdigest()


def spv_args() -> list[str]:
    return [
        "--spv-sim", str(WORK / "g35_sim.spv"),
        "--spv-compact", str(WORK / "g35_particle_compact.spv"),
        "--spv-emit", str(WORK / "g35_emit.spv"),
        "--spv-indirect-args", str(WORK / "g35_indirect_args.spv"),
        "--spv-scan-seg-sum", str(WORK / "g35_scan_seg_sum.spv"),
        "--spv-scan-spine", str(WORK / "g35_scan_spine.spv"),
        "--spv-scan-seg-apply", str(WORK / "g35_scan_seg_apply.spv"),
    ]


def run_probe(label: str, extra: list[str], env: dict) -> tuple[subprocess.CompletedProcess, dict | None, Path]:
    ev_path = WORK / f"probe_{label}.json"
    argv = [str(BIN), *spv_args(), "--evidence-out", str(ev_path), *extra]
    r = run(argv, timeout=3600, env=env)
    doc = None
    if ev_path.is_file():
        try:
            doc = json.loads(ev_path.read_text(encoding="utf-8"))
        except json.JSONDecodeError:
            doc = None
    return r, doc, ev_path


def run_gate(frames: int, cap: int, seed: int) -> int:
    os.environ.setdefault("RURIX_REQUIRE_REAL", "1")
    os.environ.setdefault("RURIX_VK_VALIDATION", "1")
    facts: dict[str, dict] = {
        fid: {"id": fid, "status": "FAIL", "detail": "未执行(前置失败)"} for fid in FACT_IDS
    }

    def set_fact(fid: str, ok: bool, detail: str) -> None:
        facts[fid] = {"id": fid, "status": "PASS" if ok else "FAIL", "detail": detail}
        note(f"  fact {fid}: {'PASS' if ok else 'FAIL'} — {detail[:200]}")

    if not GATE_SCHEMA_PATH.is_file():
        fail(f"gate schema 缺失: {GATE_SCHEMA_PATH}")
        return 1

    # ── 构建(probe vulkan bin + rurixc SPV 面)──
    ok = build_or_fail(
        ["cargo", "build", "-p", "rurix-render", "--features", "vulkan",
         "--bin", "g35_replay_device", "--quiet"],
        "probe bin",
    )
    ok &= build_or_fail(
        ["cargo", "build", "-p", "rurixc", "--features", "vulkan-backend", "--bin", "rurixc", "--quiet"],
        "rurixc",
    )
    if not ok:
        return 1

    # ── ① kernel SPV 面:现编 7 件消费面 + spirv-val + 冻结消费面快照 ──
    WORK.mkdir(parents=True, exist_ok=True)
    rurixc = ROOT / "target" / "debug" / f"rurixc{EXE_SUFFIX}"
    spv_ok = True
    for name in CONSUMED_KERNELS:
        src = KERNEL_DIR / f"{name}.rx"
        dst = WORK / f"{name}.spv"
        r = run([str(rurixc), str(src), "--target", "vulkan", "-o", str(dst)], timeout=1800)
        if r.returncode != 0 or not dst.is_file():
            spv_ok = False
            note(f"rurixc 编译失败 {src.name}: {(r.stdout + r.stderr)[-200:]}")
            continue
        val = run(["spirv-val", str(dst)], timeout=600)
        if val.returncode != 0:
            spv_ok = False
            note(f"spirv-val 未过 {dst.name}: {(val.stdout + val.stderr)[-200:]}")
    frozen_snapshot: dict[str, str] = {}
    snapshot_ok = True
    for p in FROZEN_CONSUMED_PATHS:
        fp = ROOT / p
        if fp.is_file():
            frozen_snapshot[p] = sha256_of(fp)
        else:
            snapshot_ok = False
            frozen_snapshot[p] = "MISSING"
    set_fact(
        "kernels_spv_valid",
        spv_ok and snapshot_ok,
        f"rurixc 现编 7 kernel 消费面(g35_sim/g35_particle_compact/g35_emit/g35_indirect_args + "
        f"scan 三件,W1/W2 冻结面只消费不修改)+ spirv-val={'绿' if spv_ok else '红'};冻结消费面"
        f"(七 .rx + mod.rs/core.rs/scan.rs)sha256 快照在档={snapshot_ok}(漂移守护基线)",
    )

    degrade: list[str] = []
    if not spv_ok:
        degrade.append("G35-9 消费面 kernel SPV 编译/spirv-val 未过")

    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    gate_path = ROOT / "evidence" / f"g35_replay_gate_{ts}.json"
    gate_rel = str(gate_path.relative_to(ROOT)).replace("\\", "/")
    doc_rec: dict | None = None
    doc_rep: dict | None = None
    doc_rb: dict | None = None
    doc_red: dict | None = None
    probe_evidence: list[str] = []

    def leg_out(rc, doc, label: str) -> tuple[dict | None, bool]:
        """腿出参统一判读:skipped_dev_env → degrade;rc≠0 → fail 登记
        (doc 仍供 facts 判读——红即红,不静默)。返回 (doc, 是否降级)。"""
        out = (rc.stdout or "") + (rc.stderr or "")
        if (doc or {}).get("state") == "skipped_dev_env" or '"skipped_dev_env"' in out:
            degrade.append(f"probe skipped_dev_env({label}): {out.strip()[-200:]}")
            return None, True
        if rc.returncode != 0 or doc is None:
            fail(f"{label}腿真跑失败 rc={rc.returncode}: {out[-300:]}")
        if "Validation Error" in out or "VUID-" in out:
            fail(f"{label}腿 validation 应静默却报错")
        return doc, False

    if not degrade:
        env = device_env()
        with gpu_device_lock(purpose=f"{TAG} record/replay/rollback/red 四腿 device 真跑"):
            # ── 录制腿(journal + digest 链 + 检查点落 .tmp)──
            rc, doc_rec, ev = run_probe("record", [
                "--record", "--frames", str(frames), "--cap", str(cap), "--seed", str(seed),
                "--journal-out", str(JOURNAL), "--digest-out", str(DIGEST_CHAIN),
            ], env)
            doc_rec, deg = leg_out(rc, doc_rec, "录制")
            if doc_rec is not None:
                probe_evidence.append(str(ev.relative_to(ROOT)).replace("\\", "/"))
            # ── 回放腿(仅凭 journal 重建输入,双跑)──
            if not deg and JOURNAL.is_file() and DIGEST_CHAIN.is_file():
                rc, doc_rep, ev = run_probe("replay", [
                    "--replay", "--journal", str(JOURNAL), "--digest", str(DIGEST_CHAIN),
                ], env)
                doc_rep, deg = leg_out(rc, doc_rep, "回放")
                if doc_rep is not None:
                    probe_evidence.append(str(ev.relative_to(ROOT)).replace("\\", "/"))
            # ── 回滚腿(k=16 恢复 → 重仿真至 j=48)──
            if not deg and JOURNAL.is_file() and DIGEST_CHAIN.is_file():
                rc, doc_rb, ev = run_probe("rollback", [
                    "--rollback", str(ROLLBACK_K), "--to", str(ROLLBACK_TO),
                    "--journal", str(JOURNAL), "--digest", str(DIGEST_CHAIN),
                ], env)
                doc_rb, deg = leg_out(rc, doc_rb, "回滚")
                if doc_rb is not None:
                    probe_evidence.append(str(ev.relative_to(ROOT)).replace("\\", "/"))
            # ── 红臂(journal-tamper:帧 32 emit_count+1 首异帧见证)──
            if not deg and JOURNAL.is_file() and DIGEST_CHAIN.is_file():
                rc, doc_red, ev = run_probe("red", [
                    "--red-arm", "journal-tamper",
                    "--journal", str(JOURNAL), "--digest", str(DIGEST_CHAIN),
                ], env)
                doc_red, _ = leg_out(rc, doc_red, "红臂")
                if doc_red is not None:
                    probe_evidence.append(str(ev.relative_to(ROOT)).replace("\\", "/"))

    if degrade:
        doc = {
            "schema": "rurix.g35.replay.skip.v1",
            "state": "DEV_ENV_DEGRADE",
            "reasons": degrade,
        }
        print(json.dumps(doc, ensure_ascii=False))
        for dg in degrade:
            note(f"DEV_ENV_DEGRADE {dg}")
        if os.environ.get("RURIX_REQUIRE_REAL") == "1":
            print(f"[{TAG}] FAIL RURIX_REQUIRE_REAL=1 但 device 面降级", file=sys.stderr)
            return 1
        note("SKIP DEV_ENV_DEGRADE(三态之 SKIP,非 PASS 非 FAIL)")
        return 0

    # ── ②~⑧ facts(四腿 evidence 判读)──
    rec = doc_rec or {}
    rep = doc_rep or {}
    rb = doc_rb or {}
    red = doc_red or {}
    set_fact(
        "journal_record_replay_bitexact",
        record_replay_ok(rec, rep),
        f"仅凭 journal 重建输入 GPU 重仿真:逐帧 digest 与录制链位级全等 = "
        f"{rep.get('record_replay_bitexact')!r}(首异帧 = {rep.get('first_divergence')!r};"
        f"链尾 record/replay 三方互核 {str(rec.get('digest_final'))[:23]}…;journal "
        f"{rec.get('journal_bytes')!r} B sha 双件互核;host 平行整数位级 = "
        f"record {rec.get('host_parallel_bitexact')!r} / replay {rep.get('host_parallel_bitexact')!r})",
    )
    set_fact(
        "checkpoint_restore_bitexact",
        checkpoint_restore_ok(rb),
        f"检查点 k={ROLLBACK_K} 上传恢复 device 缓冲:恢复帧自身 digest 与录制链位级全等 = "
        f"{rb.get('restore_frame_digest_match')!r}(消费检查点帧 = {rb.get('checkpoint_frame')!r};"
        f"digest 链种子 = 录制链 digest[k−1];九流+pid_base+n_curr 帧开始前捕获)",
    )
    set_fact(
        "rollback_resim_bitexact",
        rollback_ok(rb),
        f"回滚重仿真 k={ROLLBACK_K}→j={ROLLBACK_TO}:逐帧位级全等 = {rb.get('resim_bitexact')!r} + "
        f"digest[{ROLLBACK_TO}] 全等 = {rb.get('digest_at_j_match')!r}"
        f"(frames_resimmed = {rb.get('frames_resimmed')!r};网络回滚语义 = 检查点 + 输入重放)",
    )
    set_fact(
        "first_divergence_frame_witness",
        witness_ok(red),
        f"分歧可定位见证:篡改 journal 帧 {TAMPER_FRAME} emit_count(+1)→ 回放 digest 链"
        f"首异帧 = {red.get('first_divergence')!r}(要求精确 == {TAMPER_FRAME};篡改帧前逐帧"
        f"全等——确定性系统独有性质,Niagara GPU sim 做不到)",
    )
    set_fact(
        "determinism_double_run",
        determinism_ok(rep),
        f"同 journal 输入 GPU 重仿真双跑 digest 链位级一致 = {rep.get('determinism_double_run')!r}"
        f"(run1 链尾 = {str(rep.get('digest_replay_final'))[:23]}…;digest = 全流字节 sha256 "
        f"逐帧链式,g35_particle_core_device 同式)",
    )
    set_fact(
        "red_arm_effective",
        red_ok(red),
        f"RED 臂 journal-tamper:篡改后链尾 digest 必异 detected={red.get('detected')!r}"
        f"(recorded={str(red.get('digest_recorded_final'))[:23]}… "
        f"tampered={str(red.get('digest_tampered_final'))[:23]}…)",
    )
    ms_rec, ms_rep, ms_rb = (rec.get("frame_ms_mean"), rep.get("frame_ms_mean"), rb.get("frame_ms_mean"))
    set_fact(
        "frame_ms_measured",
        frame_ms_sane(ms_rec) and frame_ms_sane(ms_rep) and frame_ms_sane(ms_rb),
        f"device 7 dispatch 链逐帧墙钟均值 record={ms_rec!r} / replay={ms_rep!r} / "
        f"rollback={ms_rb!r} ms(measured_local 诚实登记;含 run_compute 逐 dispatch "
        f"instance/device 会话重建开销,登记语义非帧率对标)",
    )

    # ── evidence 落盘(门裁决件;jsonschema 自校验硬门)──
    fact_rows = [facts[fid] for fid in FACT_IDS]
    all_pass = all(f["status"] == "PASS" for f in fact_rows) and not FAILURES
    env_info = {
        "gpu": "RTX 4070 Ti(本机单卡 measured_local)",
        "os": "windows",
        "rustc": subprocess.run(["rustc", "--version"], capture_output=True, text=True).stdout.strip(),
        "base_commit": subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT,
                                      capture_output=True, text=True).stdout.strip(),
    }
    spv_entry = lambda name: {
        "path": str((WORK / f"{name}.spv").relative_to(ROOT)).replace("\\", "/"),
        "sha256": sha256_of(WORK / f"{name}.spv") if (WORK / f"{name}.spv").is_file() else "sha256:" + "0" * 64,
    }
    zero_digest = "sha256:" + "0" * 64
    gate_doc = {
        "schema": GATE_SCHEMA_ID,
        "subject": SUBJECT,
        "symbolic_gate_key": GATE_KEY,
        "wave": WAVE,
        "facts": fact_rows,
        "verdict": "PASS" if all_pass else "FAIL",
        "kernels": {
            "sim_spv": spv_entry("g35_sim"),
            "compact_spv": spv_entry("g35_particle_compact"),
            "emit_spv": spv_entry("g35_emit"),
            "indirect_args_spv": spv_entry("g35_indirect_args"),
            "scan_seg_sum_spv": spv_entry("g35_scan_seg_sum"),
            "scan_spine_spv": spv_entry("g35_scan_spine"),
            "scan_seg_apply_spv": spv_entry("g35_scan_seg_apply"),
            "spirv_val_all": bool(facts["kernels_spv_valid"]["status"] == "PASS"),
            "frozen_consumed_snapshot": frozen_snapshot,
        },
        "journal_layout": {
            "magic": "G35J",
            "version": 1,
            "byte_order": "little_endian",
            "header_bytes": 88,
            "record_bytes": 4,
            "header_fields": (
                "magic[4]=\"G35J\" version:u32=1 seed:u64 cap:u32 frames:u32 dt:f32 "
                "gravity_y:f32 emitter{pos[3],spread[3],vel_base[3],vel_spread[3],"
                "life_base,gravity_y}:f32x14"
            ),
            "record_fields": "emit_count:u32",
            "journal_path": str(JOURNAL.relative_to(ROOT)).replace("\\", "/"),
            "journal_bytes": rec.get("journal_bytes", 0),
            "journal_sha256": rec.get("journal_sha256", zero_digest),
            "frames": rec.get("frames", frames),
            "cap": rec.get("cap", cap),
            "seed": rec.get("seed", seed),
            "dt": rec.get("dt", 1.0 / 60.0),
            "emit_schedule": "min(64 + frame*17 % 192, cap - n_curr)",
            "checkpoint_interval": CHECKPOINT_INTERVAL,
        },
        "record": {
            "digest_final": rec.get("digest_final", zero_digest),
            "checkpoint_frames": rec.get("checkpoint_frames", []),
            "host_parallel_bitexact": rec.get("host_parallel_bitexact", False),
            "n_final": rec.get("n_final", 0),
            "pids_issued": rec.get("pids_issued", 0),
        },
        "replay": {
            "record_replay_bitexact": rep.get("record_replay_bitexact", False),
            "first_divergence": rep.get("first_divergence", -1),
            "determinism_double_run": rep.get("determinism_double_run", False),
            "digest_recorded_final": rep.get("digest_recorded_final", zero_digest),
            "digest_replay_final": rep.get("digest_replay_final", zero_digest),
            "digest_run2_final": rep.get("digest_run2_final", zero_digest),
            "host_parallel_bitexact": rep.get("host_parallel_bitexact", False),
        },
        "rollback": {
            "k": ROLLBACK_K,
            "to": ROLLBACK_TO,
            "checkpoint_frame": rb.get("checkpoint_frame", -1),
            "restore_frame_digest_match": rb.get("restore_frame_digest_match", False),
            "resim_bitexact": rb.get("resim_bitexact", False),
            "digest_at_j_match": rb.get("digest_at_j_match", False),
            "frames_resimmed": rb.get("frames_resimmed", 0),
            "digest_recorded_at_j": rb.get("digest_recorded_at_j", zero_digest),
            "digest_resim_at_j": rb.get("digest_resim_at_j", zero_digest),
            "host_parallel_bitexact": rb.get("host_parallel_bitexact", False),
        },
        "red_arm": {
            "arm": "journal-tamper",
            "tamper": "emit_count+1",
            "tampered_frame": TAMPER_FRAME,
            "expected_divergence": TAMPER_FRAME,
            "detected": red.get("detected", False),
            "first_divergence": red.get("first_divergence", -1),
            "digest_recorded_final": red.get("digest_recorded_final", zero_digest),
            "digest_tampered_final": red.get("digest_tampered_final", zero_digest),
        },
        "frame_ms": {
            "record_mean_ms": ms_rec if frame_ms_sane(ms_rec) else 1e-9,
            "replay_mean_ms": ms_rep if frame_ms_sane(ms_rep) else 1e-9,
            "rollback_mean_ms": ms_rb if frame_ms_sane(ms_rb) else 1e-9,
            "measured": "measured_local",
            "note": (
                "device 7 dispatch 链逐帧墙钟均值(vk::run_compute 每 dispatch 重建 "
                "instance/device,该会话开销如实计入;登记语义非帧率对标,生产车道"
                "届时走 DeviceFrameSession 持久车道 + DispatchSpec::Indirect)"
            ),
        },
        "probe_evidence": probe_evidence + ["(probe evidence 缺失)"] * max(0, 4 - len(probe_evidence)),
        "environment": env_info,
        "timestamp": ts,
        "notes": (
            "G35-9 确定性回放/回滚:粒子输入 journal(v1 冻结布局 \"G35J\":header 88 B = "
            "magic+version+seed u64+cap u32+frames u32+dt f32+gravity_y f32+emitter EmitterDesc "
            "全字段 f32×14;逐帧记录 4 B = emit_count u32;全域小端手写 to_le_bytes 零外部 "
            "crate,写/读往返位级)+ 回放重仿真位级(仅凭 journal 重建输入〔seed→随机带/"
            "emitter/dt/emit 序列〕在 GPU 上重跑 W2 七 kernel 链——非 host 数据回放;逐帧 "
            "digest 与录制链位级全等)+ 回滚 N 帧重仿真位级(录制腿每 K=16 帧帧开始前 "
            "readback 九流+pid_base+n_curr 存检查点〔\"G35C\" v1〕;回滚 = 检查点 k=16 上传"
            "恢复 → 重仿真至 j=48,digest 链种子 = 录制链 digest[k−1] ⇒ 恢复帧自身与逐帧至 "
            "j 位级全等——网络回滚语义 = 检查点 + 输入重放)+ 首异帧见证(篡改帧 32 "
            "emit_count+1 → 首异帧精确 == 32,分歧可定位)。兑现「确定性 GPU 粒子」总口径,"
            "反打 Niagara(网络不复制粒子本体、GPU 模拟不可回放、GPU sim 非确定不可定位分歧)。"
            "device 帧链/digest 链式(全流字节 sha256 逐帧链式)与 g35_particle_core_device "
            "逐字同模;host 金标准平行对拍维持(整数流 flags/scan_out/seg_offsets/pid/args "
            "零容差)。本门全位级:整数/digest 域零容差,无 f32 budget 条目如实登记(f32 "
            "容差面归 g35.wave2 门,digest 已按字节覆盖 f32 流)。probe 真跑件与 journal/"
            "digest 链/检查点文件留 .tmp 工作区不入 evidence/。"
        ),
    }
    import jsonschema  # 自校验硬门(schema 漂移即 RED)

    errs = list(jsonschema.Draft7Validator(
        json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
    ).iter_errors(gate_doc))
    if errs:
        fail("gate evidence schema 自校验红: " + "; ".join(
            f"{'/'.join(str(p) for p in e.path)}: {e.message}" for e in errs[:3]))
        all_pass = False
        gate_doc["verdict"] = "FAIL"
    gate_path.parent.mkdir(parents=True, exist_ok=True)
    gate_path.write_text(json.dumps(gate_doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    note(f"evidence: {gate_rel}(probe 件 {len(probe_evidence)} 份与 journal/digest 链/检查点留 .tmp 工作区)")

    note(f"GATE {'PASS' if all_pass else 'FAIL'} {GATE_KEY}")
    return 0 if all_pass else 1


# ---------------------------------------------------------------------------
# selftest(判读器红绿穷举 + schema 校验 + FACT_IDS 互核;零 GPU 零构建)
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

    d0 = "sha256:" + "a" * 64
    d1 = "sha256:" + "b" * 64
    d2 = "sha256:" + "c" * 64
    # 红绿臂①:录制腿健全判。
    rec_good = {"state": "pass", "host_parallel_bitexact": True, "digest_final": d0,
                "frames": 64, "checkpoint_interval": 16, "checkpoint_frames": [0, 16, 32, 48],
                "journal_sha256": d2, "journal_bytes": 344, "frame_ms_mean": 850.0}
    expect(record_ok(rec_good), "GREEN:录制腿正例")
    expect(not record_ok({**rec_good, "state": "fail"}), "RED:录制腿 state=fail 必红")
    expect(not record_ok({**rec_good, "host_parallel_bitexact": False}), "RED:host 平行整数破必红")
    expect(not record_ok({**rec_good, "checkpoint_frames": [0, 32, 48]}), "RED:k=16 检查点缺失必红")
    expect(not record_ok({**rec_good, "checkpoint_interval": 8}), "RED:检查点间隔非 16 必红")
    expect(not record_ok({**rec_good, "digest_final": "xx"}), "RED:digest 形态破必红")
    expect(not record_ok({**rec_good, "frames": 0}), "RED:零帧必红")
    expect(not record_ok({**rec_good, "frames": True}), "RED:bool 冒充帧数必红")
    # 红绿臂②:journal 录制/回放位级判。
    rep_good = {"record_replay_bitexact": True, "first_divergence": -1,
                "determinism_double_run": True, "digest_recorded_final": d0,
                "digest_replay_final": d0, "digest_run2_final": d0,
                "journal_sha256": d2, "host_parallel_bitexact": True, "frame_ms_mean": 860.0}
    expect(record_replay_ok(rec_good, rep_good), "GREEN:录制/回放位级正例")
    expect(not record_replay_ok(rec_good, {**rep_good, "record_replay_bitexact": False}),
           "RED:回放非位级必红")
    expect(not record_replay_ok(rec_good, {**rep_good, "first_divergence": 32}),
           "RED:首异帧非 -1 必红")
    expect(not record_replay_ok(rec_good, {**rep_good, "digest_replay_final": d1}),
           "RED:旗标真但链尾异(自相矛盾)必红")
    expect(not record_replay_ok({**rec_good, "digest_final": d1}, rep_good),
           "RED:record/replay 链尾互核破必红")
    expect(not record_replay_ok(rec_good, {**rep_good, "journal_sha256": d1}),
           "RED:journal sha 双件互核破必红")
    expect(not record_replay_ok(rec_good, {**rep_good, "host_parallel_bitexact": False}),
           "RED:回放腿 host 平行破必红")
    expect(not record_replay_ok({**rec_good, "state": "fail"}, rep_good),
           "RED:录制腿不健全传染必红")
    expect(not record_replay_ok(rec_good, {**rep_good, "record_replay_bitexact": "true"}),
           "RED:字符串冒充 bool 必红")
    # 红绿臂③④:检查点恢复/回滚重仿真判。
    rb_good = {"k": 16, "to": 48, "checkpoint_frame": 16, "restore_frame_digest_match": True,
               "resim_bitexact": True, "digest_at_j_match": True, "frames_resimmed": 33,
               "digest_recorded_at_j": d0, "digest_resim_at_j": d0,
               "host_parallel_bitexact": True, "frame_ms_mean": 870.0}
    expect(checkpoint_restore_ok(rb_good), "GREEN:检查点恢复正例")
    expect(rollback_ok(rb_good), "GREEN:回滚重仿真正例")
    expect(not checkpoint_restore_ok({**rb_good, "restore_frame_digest_match": False}),
           "RED:恢复帧 digest 异必红")
    expect(not checkpoint_restore_ok({**rb_good, "checkpoint_frame": 0}),
           "RED:消费检查点帧 ≠ k 必红")
    expect(not checkpoint_restore_ok({**rb_good, "k": 8}), "RED:k 非冻结 16 必红")
    expect(not checkpoint_restore_ok({**rb_good, "host_parallel_bitexact": False}),
           "RED:回滚腿 host 平行破必红")
    expect(not rollback_ok({**rb_good, "resim_bitexact": False}), "RED:重仿真非位级必红")
    expect(not rollback_ok({**rb_good, "digest_at_j_match": False}), "RED:digest[j] 异必红")
    expect(not rollback_ok({**rb_good, "to": 47}), "RED:j 非冻结 48 必红")
    expect(not rollback_ok({**rb_good, "frames_resimmed": 32}), "RED:重仿真帧数 ≠ 33 必红")
    expect(not rollback_ok({**rb_good, "digest_resim_at_j": d1}),
           "RED:旗标真但 j 处 digest 对异(自相矛盾)必红")
    expect(not rollback_ok({**rb_good, "digest_recorded_at_j": "bad"}), "RED:digest 形态破必红")
    # 红绿臂⑤:首异帧见证判。
    red_good = {"arm": "journal-tamper", "detected": True, "first_divergence": 32,
                "expected_divergence": 32, "tampered_frame": 32,
                "digest_recorded_final": d0, "digest_tampered_final": d1}
    expect(witness_ok(red_good), "GREEN:首异帧见证正例")
    expect(not witness_ok({**red_good, "first_divergence": 33}), "RED:晚异(错位)必红")
    expect(not witness_ok({**red_good, "first_divergence": 31}), "RED:早异(链污染)必红")
    expect(not witness_ok({**red_good, "first_divergence": -1}), "RED:漏检必红")
    expect(not witness_ok({**red_good, "arm": "seed-change"}), "RED:臂名不符必红")
    expect(not witness_ok({**red_good, "expected_divergence": 31}), "RED:期望锚漂移必红")
    expect(not witness_ok({**red_good, "tampered_frame": 31}), "RED:篡改帧锚漂移必红")
    # 红绿臂⑥:双跑位级判。
    expect(determinism_ok(rep_good), "GREEN:双跑位级正例")
    expect(not determinism_ok({**rep_good, "digest_run2_final": d1}),
           "RED:旗标真但 run2 链尾异(自相矛盾)必红")
    expect(not determinism_ok({**rep_good, "determinism_double_run": False}), "RED:旗标假必红")
    expect(not determinism_ok({**rep_good, "digest_run2_final": "zz"}), "RED:digest 形态破必红")
    # 红绿臂⑦:RED 臂判。
    expect(red_ok(red_good), "GREEN:RED 臂正例")
    expect(not red_ok({**red_good, "detected": False}), "RED:漏检必红")
    expect(not red_ok({**red_good, "digest_tampered_final": d0}),
           "RED:篡改后 digest 未变(镂空 digest)必红")
    expect(not red_ok({**red_good, "arm": "tamper"}), "RED:臂名不符必红")
    expect(not red_ok({**red_good, "digest_recorded_final": "bad"}), "RED:digest 形态破必红")
    # 红绿臂⑧:frame_ms 健全判。
    expect(frame_ms_sane(845.8), "GREEN:frame_ms 正例")
    expect(not frame_ms_sane(0.0), "RED:0ms 必红")
    expect(not frame_ms_sane(float("nan")), "RED:NaN 必红")
    expect(not frame_ms_sane(float("inf")), "RED:inf 必红")
    expect(not frame_ms_sane(None), "RED:缺失必红")
    expect(not frame_ms_sane(True), "RED:bool 冒充数值必红")
    # schema 互核:gate schema 在树 + Draft7 合法 + facts enum == FACT_IDS +
    # const 互核(门键/journal 冻结布局面/回滚冻结参数/红臂冻结锚)。
    expect(GATE_SCHEMA_PATH.is_file(), "gate schema 在树")
    if GATE_SCHEMA_PATH.is_file():
        gs = json.loads(GATE_SCHEMA_PATH.read_text(encoding="utf-8"))
        enum = gs["properties"]["facts"]["items"]["properties"]["id"]["enum"]
        expect(sorted(enum) == sorted(FACT_IDS), f"gate schema facts enum == FACT_IDS({len(FACT_IDS)})")
        expect(gs["properties"]["schema"]["const"] == GATE_SCHEMA_ID, "gate schema const 互核")
        expect(gs["properties"]["symbolic_gate_key"]["const"] == GATE_KEY, "gate schema 门键 const 互核")
        expect(gs["properties"]["subject"]["const"] == SUBJECT, "gate schema subject const 互核")
        expect(gs["properties"]["wave"]["const"] == WAVE, "gate schema wave const 互核")
        jl = gs["properties"]["journal_layout"]["properties"]
        expect(jl["magic"]["const"] == "G35J" and jl["version"]["const"] == 1,
               "journal 冻结布局 const 互核(magic/version)")
        expect(jl["header_bytes"]["const"] == 88 and jl["record_bytes"]["const"] == 4,
               "journal 冻结布局 const 互核(header 88 B / record 4 B)")
        expect(jl["checkpoint_interval"]["const"] == CHECKPOINT_INTERVAL,
               "检查点间隔 const 互核(K=16)")
        rbp = gs["properties"]["rollback"]["properties"]
        expect(rbp["k"]["const"] == ROLLBACK_K and rbp["to"]["const"] == ROLLBACK_TO,
               "回滚腿冻结参数 const 互核(k=16 j=48)")
        rap = gs["properties"]["red_arm"]["properties"]
        expect(rap["arm"]["const"] == "journal-tamper"
               and rap["tampered_frame"]["const"] == TAMPER_FRAME
               and rap["expected_divergence"]["const"] == TAMPER_FRAME,
               "红臂冻结锚 const 互核(journal-tamper 帧 32)")
        import jsonschema as _js
        _js.Draft7Validator.check_schema(gs)
        print("  ok   — gate schema Draft7 合法(check_schema 绿)")
    expect(len(FACT_IDS) == 8, "facts 闭集 = 8")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(facts=8;8 红绿臂组 + journal/回滚/红臂冻结面 schema 互核)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--gate", default="")
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--frames", type=int, default=64)
    ap.add_argument("--cap", type=int, default=65536)
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.gate:
        if args.gate != GATE_KEY:
            print(f"[{TAG}] FAIL: 未知门键 {args.gate}(闭集 {GATE_KEY})", file=sys.stderr)
            return 1
        if args.frames < 64:
            print(f"[{TAG}] FAIL: --frames {args.frames} < 64(回滚腿 j={ROLLBACK_TO}/红臂"
                  f"篡改帧 {TAMPER_FRAME} 须窗内;64 = 冻结默认窗)", file=sys.stderr)
            return 1
        if args.cap <= 0 or args.cap % 256 != 0:
            print(f"[{TAG}] FAIL: --cap {args.cap} 须为 SEG=256 正整倍数", file=sys.stderr)
            return 1
        return run_gate(args.frames, args.cap, args.seed)
    ap.print_help()
    return 1


if __name__ == "__main__":
    sys.exit(main())
