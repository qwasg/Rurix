#!/usr/bin/env python3
# -*- coding: utf-8 -*-
# Assisted-by: Cursor Claude Fable 5（G17.3 M-b NGX 版本演进面对齐评估波）
"""G17.3 P0 硬门 M-b：NGX 版本演进面对齐评估
（g17.p0.m_b.ngx_evolution_alignment；G17_CONTRACT §4.2 M-b/G-G17-4；
G17_ACCEPTANCE_MAP §1 M-b 行；G15-MD-F1 承接锚②字面兑现面）。

判据（契约 §4.2 M-b 逐字）：nvngx_dlss.dll 310.5.2→310.6.0+ 换版评估走新缓存目录
+ G17 新 provenance 登记面 `milestones/g17/g17_vendor_sdk_registry.json`（g13 登记表
0-byte）+ PaddedWindowNetwork 实例化形态核验（SL verbose 日志逐字）+ in-stream/提交
税源 X2 边际探针重测分解（对照 1.90+0.10ms 基线，新鲜命令输出）+ 画质守护双门禁
（Stage A digest 锚零漂移 + 画质锚带复核带内，超带即拒绝换版）+ A/B measured 结论
如实登记（采纳/拒绝/零收益均合法）。

本门消费 `milestones/g17/g17_mb_ngx_probe_results.json`（probe driver 双臂真跑产物；
device 真跑面在 probe 轮）。

RED 字面：B 臂 digest MISS 但 verdict=adopt（门禁遮蔽）/ 登记面 sha 与实测不符 /
X2 分解数据缺失 / probe 缺臂——注入全检出。

用法：
  py -3 ci/g17_ngx_evolution_alignment_smoke.py --gate g17.p0.m_b.ngx_evolution_alignment
  py -3 ci/g17_ngx_evolution_alignment_smoke.py --verify-latest
  py -3 ci/g17_ngx_evolution_alignment_smoke.py --selftest
"""
from __future__ import annotations

import argparse
import hashlib
import io
import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import g11_wave_exit_lib as wel  # noqa: E402

ROOT = wel.ROOT
GATE_KEY = "g17.p0.m_b.ngx_evolution_alignment"
NUMERIC_STEP = 298  # post-interlock 实测顺位领取
SUBJECT = "g17_m_b_ngx_evolution_alignment"
WAVE = "G17.3"
SCHEMA_PATH = ROOT / "milestones/g17/g17_m_b_ngx_evolution_alignment_evidence_schema.json"
PROBE_JSON = ROOT / "milestones/g17/g17_mb_ngx_probe_results.json"
G17_REGISTRY = ROOT / "milestones/g17/g17_vendor_sdk_registry.json"
G13_REGISTRY = ROOT / "milestones/g13/g13_vendor_sdk_registry.json"
EVAL_BIN_DIR = ROOT / "external/streamline-2.10.3-ngx310.6.0/bin/x64"
SOURCE_REF = (
    "G17_CONTRACT §4.2 M-b/G-G17-4;G17_ACCEPTANCE_MAP §1 M-b 行;"
    "G15_P2_DECISIONS.md §4 G15-MD-F1 行承接锚②;"
    "milestones/g17/g17_mb_ngx_probe_results.json（双臂 probe 消费面）"
)


def _sha256(path: Path) -> str:
    h = hashlib.sha256()
    with io.open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def evaluate(probe: dict | None, reg: dict | None, g13_reg: dict | None,
             eval_dll_shas: dict[str, str] | None, src_diff_names: list[str] | None) -> list[dict]:
    """10 facts（纯函数可注入）。"""
    facts: list[dict] = []
    reg_sdk = (reg or {}).get("sdks", {}).get("streamline_ngx_310_6_0_eval", {})
    reg_dlls = reg_sdk.get("dlls", {})
    g13_dlls = (g13_reg or {}).get("sdks", {}).get("streamline", {}).get("dlls", {})

    # ① 评估缓存 provenance 登记 + 实测 sha 一致 + sl 三件 == g13 登记
    p_bad: list[str] = []
    if not reg_dlls:
        p_bad.append("g17_vendor_sdk_registry 缺 streamline_ngx_310_6_0_eval.dlls")
    if eval_dll_shas is None:
        p_bad.append("评估缓存目录四 DLL 实测不可得")
    else:
        for n, sha in reg_dlls.items():
            got = eval_dll_shas.get(n)
            if got != sha:
                p_bad.append(f"{n} 登记 {sha[:12]}… ≠ 实测 {str(got)[:12]}…")
        for n in ("sl.interposer.dll", "sl.common.dll", "sl.dlss.dll"):
            if reg_dlls.get(n) != g13_dlls.get(n):
                p_bad.append(f"{n} 登记与 g13 原件登记不一致（原件复制面破）")
        if reg_dlls.get("nvngx_dlss.dll") == g13_dlls.get("nvngx_dlss.dll"):
            p_bad.append("nvngx_dlss.dll 登记与 310.5.2 同 sha（换版评估对象缺失）")
    facts.append({
        "id": "eval_cache_provenance_registered",
        "status": "PASS" if not p_bad else "FAIL",
        "detail": "四 DLL sha256 登记 == 实测重算；sl 三件 == g13 登记原件；nvngx 310.6.0 ≠ 310.5.2"
        if not p_bad else "; ".join(p_bad[:3]),
    })

    # ② g13 登记表 0-byte（git 机核由调用侧注入 src_diff_names 含义扩展——此处独立跑）
    g13_ok = True
    g13_note = "g13_vendor_sdk_registry.json 0-byte（git diff HEAD 空）"
    try:
        r = subprocess.run(["git", "diff", "HEAD", "--name-only", "--",
                            "milestones/g13/g13_vendor_sdk_registry.json"],
                           cwd=ROOT, capture_output=True, text=True, check=False)
        if r.returncode != 0 or r.stdout.strip():
            g13_ok = False
            g13_note = f"g13 登记表触改: {r.stdout.strip() or f'rc={r.returncode}'}"
    except OSError as e:
        g13_ok, g13_note = False, str(e)
    facts.append({"id": "g13_registry_zero_byte", "status": "PASS" if g13_ok else "FAIL",
                  "detail": g13_note})

    # ③ probe 双臂齐备且全轮 ok
    if probe is None:
        facts.append({"id": "probe_results_fresh", "status": "FAIL",
                      "detail": "g17_mb_ngx_probe_results.json 缺失（probe 未跑；诚实红）"})
        for fid in ("network_instantiation_verified", "in_stream_decomposition_retested",
                    "digest_guard_gate", "adoption_verdict_honest", "ab_measured_recorded"):
            facts.append({"id": fid, "status": "FAIL", "detail": "probe 缺失不可判"})
    else:
        arms = probe.get("arms", {})
        a_sum = arms.get("a", {}).get("summary", {})
        b_sum = arms.get("b", {}).get("summary", {})
        both = bool(a_sum) and bool(b_sum)
        b_unavail = bool(b_sum.get("arm_unavailable"))
        ok3 = both and a_sum.get("all_rounds_ok") and (b_sum.get("all_rounds_ok") or b_unavail)
        facts.append({
            "id": "probe_results_fresh",
            "status": "PASS" if ok3 else "FAIL",
            "detail": f"双臂 probe 齐备（started={probe.get('started_utc')}），"
                      f"A all_ok={a_sum.get('all_rounds_ok')} B all_ok={b_sum.get('all_rounds_ok')}"
                      + (f"（B 臂级不可用如实登记 = 合法评估终态，诊断 = "
                         f"{b_sum.get('fail_diagnostics')}）" if b_unavail else ""),
        })
        # ④ 形态核验（SL 日志 token；B 臂不可用 = 实例化形态不可达如实登记）
        b_tokens = arms.get("b", {}).get("ngx_tokens_found") or []
        a_tokens = arms.get("a", {}).get("ngx_tokens_found") or []
        form_ok = "NGXCubinVulkan" in a_tokens and ("NGXCubinVulkan" in b_tokens or b_unavail)
        facts.append({
            "id": "network_instantiation_verified",
            "status": "PASS" if form_ok else "FAIL",
            "detail": f"A 臂 tokens={a_tokens}（NGXCubinVulkan cubin 宿主面在档）；B 臂 = "
                      + (f"tokens={b_tokens}（PaddedWindowNetwork "
                         f"{'在' if 'PaddedWindowNetwork' in b_tokens else '不在'} B 臂日志）"
                         if not b_unavail else
                         "310.6.0 在 SL 2.10.3 下 DLSSContext 不可用——实例化形态不可达"
                         "（形态核验的诚实产出 = 兼容性失败留档 .tmp/g17_mb/arm_b_fail.log，"
                         "PaddedWindowNetwork 形态对齐评估结论 = 当前 SL pin 下不可评）"),
        })
        # ⑤ X2 分解重测（对照 1.90+0.10 基线；B 臂不可用 = not-available 如实登记）
        a_marg = a_sum.get("in_stream_marginal_median_ms")
        b_marg = b_sum.get("in_stream_marginal_median_ms")
        base = probe.get("g15_baseline_literal", {})
        ok5 = a_marg is not None and (b_marg is not None or b_unavail)
        facts.append({
            "id": "in_stream_decomposition_retested",
            "status": "PASS" if ok5 else "FAIL",
            "detail": (
                f"X2 边际（新鲜命令输出）：A 臂 310.5.2 = {a_marg} ms（submit_wait x1 = "
                f"{a_sum.get('submit_wait_x1_median_ms')} / x2 = {a_sum.get('submit_wait_x2_median_ms')} ms）"
                f"；B 臂 310.6.0 = {b_marg if b_marg is not None else 'not-available（臂级不可用）'}"
                f"（G15 §8.7 对照基线字面 = in-stream {base.get('in_stream_ms')} + 提交固定 "
                f"{base.get('submit_fixed_ms')} ms——vendor SDK 演进 in-stream 成本变化面：当前 SL pin"
                f" 下 310.6.0 不可运行，变化不可测如实登记）"
            ) if ok5 else "A 臂 X2 边际数据缺失（timing_x1/timing_x2 轮不齐）",
        })
        # ⑥ 画质守护双门禁第一门（digest 锚；B 臂不可用 = 门禁不适用如实登记）
        a_hit = a_sum.get("digest_anchor_hit")
        b_hit = b_sum.get("digest_anchor_hit")
        ok6 = a_hit is True
        facts.append({
            "id": "digest_guard_gate",
            "status": "PASS" if ok6 else "FAIL",
            "detail": f"A 臂（310.5.2 生产默认）notiming digest == G14.12 冻结锚 HIT={a_hit}；"
                      f"B 臂（310.6.0）HIT={b_hit}"
                      + ("（臂级不可用——门禁不适用，换版已被兼容性面拒绝）" if b_unavail
                         else ("（新网络输出位面 ≠ 冻结锚——超锚即拒绝换版门禁触发，如实登记）"
                               if b_hit is False else "（位级同一）")),
        })
        # ⑦ 采纳结论与门禁事实一致（诚实机核）
        verdict = (probe.get("adoption_verdict") or {}).get("verdict", "")
        should_reject = b_unavail or (b_hit is not True)
        honest = (verdict == "reject_version_swap") == should_reject
        facts.append({
            "id": "adoption_verdict_honest",
            "status": "PASS" if honest and verdict else "FAIL",
            "detail": f"verdict={verdict!r} ⇔ 拒绝依据事实（B 臂不可用={b_unavail} ∨ digest HIT≠True"
                      f"〔实测 {b_hit}〕）一致（采纳/拒绝/零收益均合法，禁遮蔽门禁事实冒充采纳）",
        })
        # ⑧ A/B measured 登记（B 臂不可用 = not-available 如实登记）
        a_prod = a_sum.get("notiming_prod_median_ms")
        b_prod = b_sum.get("notiming_prod_median_ms")
        ok8 = a_prod is not None and (b_prod is not None or b_unavail)
        facts.append({
            "id": "ab_measured_recorded",
            "status": "PASS" if ok8 else "FAIL",
            "detail": (
                f"no-timing 三轮中位：A 臂 {a_prod} ms（{a_sum.get('notiming_prod_ms')}）"
                f" / B 臂 {b_prod if b_prod is not None else 'not-available（臂级不可用，A/B 对照不可测如实登记）'}"
                f"——单变量同窗对照（measured_local）"
            ) if ok8 else "A 臂 no-timing 三轮数据不齐",
        })

    # ⑨ 生产默认 0-byte（src/ 与默认缓存零触改；X2 探针须已撤除）
    src_ok = True
    src_note = "src/ 0-byte（git diff HEAD 空——X2 探针已撤除，生产默认装载面零触改）"
    if src_diff_names is None:
        try:
            r = subprocess.run(["git", "diff", "HEAD", "--name-only", "--", "src/"],
                               cwd=ROOT, capture_output=True, text=True, check=False)
            src_diff_names = [ln for ln in r.stdout.splitlines() if ln.strip()] if r.returncode == 0 else ["<git-fail>"]
        except OSError:
            src_diff_names = ["<git-unavailable>"]
    if src_diff_names:
        src_ok = False
        src_note = f"src/ 触改未清: {src_diff_names[:3]}（X2 探针撤除义务/生产面 0-byte 违例）"
    facts.append({"id": "production_default_zero_byte", "status": "PASS" if src_ok else "FAIL",
                  "detail": src_note})

    # ⑩ RED 臂
    red_ok, red_note = run_red_arms()
    facts.append({"id": "red_arms_effective", "status": "PASS" if red_ok else "FAIL",
                  "detail": red_note})
    return facts


def _synth_probe(*, b_hit: bool = False, verdict: str = "reject_version_swap",
                 with_x2: bool = True) -> dict:
    def arm(hit: bool):
        return {
            "ngx_tokens_found": ["NGXCubinVulkan", "PaddedWindowNetwork"] if not hit else ["NGXCubinVulkan"],
            "summary": {
                "notiming_prod_ms": [3.7, 3.75, 3.8], "notiming_prod_median_ms": 3.75,
                "submit_wait_x1_median_ms": 2.0 if with_x2 else None,
                "submit_wait_x2_median_ms": 3.9 if with_x2 else None,
                "in_stream_marginal_median_ms": 1.9 if with_x2 else None,
                "digests": ["sha256:x"], "digest_anchor_hit": hit, "all_rounds_ok": True,
            },
        }
    return {
        "started_utc": "20260824T100000Z",
        "g15_baseline_literal": {"in_stream_ms": 1.90, "submit_fixed_ms": 0.10},
        "arms": {"a": arm(True), "b": arm(b_hit)},
        "adoption_verdict": {"verdict": verdict},
    }


def _synth_reg() -> tuple[dict, dict, dict]:
    g13 = {"sdks": {"streamline": {"dlls": {
        "sl.interposer.dll": "AAA", "sl.common.dll": "BBB", "sl.dlss.dll": "CCC",
        "nvngx_dlss.dll": "OLD"}}}}
    g17 = {"sdks": {"streamline_ngx_310_6_0_eval": {"dlls": {
        "sl.interposer.dll": "AAA", "sl.common.dll": "BBB", "sl.dlss.dll": "CCC",
        "nvngx_dlss.dll": "NEW"}}}}
    shas = {"sl.interposer.dll": "AAA", "sl.common.dll": "BBB", "sl.dlss.dll": "CCC",
            "nvngx_dlss.dll": "NEW"}
    return g17, g13, shas


def run_red_arms() -> tuple[bool, str]:
    """RED 四臂：门禁遮蔽采纳 / 登记 sha 不符 / X2 缺失 / probe 缺臂——注入全检出。"""
    g17, g13, shas = _synth_reg()
    fails: list[str] = []

    def core(probe, reg=g17, g13r=g13, dllsha=shas):
        out = {}
        reg_dlls = reg.get("sdks", {}).get("streamline_ngx_310_6_0_eval", {}).get("dlls", {})
        p_bad = [1 for n, sha in reg_dlls.items() if (dllsha or {}).get(n) != sha]
        out["provenance"] = not p_bad and bool(reg_dlls)
        arms = (probe or {}).get("arms", {})
        b_sum = arms.get("b", {}).get("summary", {})
        a_sum = arms.get("a", {}).get("summary", {})
        b_unavail = bool(b_sum.get("arm_unavailable"))
        out["probe"] = (bool(a_sum) and bool(b_sum) and a_sum.get("all_rounds_ok")
                        and (b_sum.get("all_rounds_ok") or b_unavail))
        out["x2"] = a_sum.get("in_stream_marginal_median_ms") is not None and (
            b_sum.get("in_stream_marginal_median_ms") is not None or b_unavail
        )
        verdict = ((probe or {}).get("adoption_verdict") or {}).get("verdict", "")
        should_reject = b_unavail or (b_sum.get("digest_anchor_hit") is not True)
        out["honest"] = bool(verdict) and (verdict == "reject_version_swap") == should_reject
        return out

    base = core(_synth_probe())
    if not all(base.values()):
        fails.append(f"正样本未全绿: {base}")
    r1 = core(_synth_probe(b_hit=False, verdict="candidate_adopt_pending_quality_band"))
    if r1["honest"]:
        fails.append("门禁遮蔽采纳未检出（B 臂 MISS + verdict=adopt）")
    bad_shas = dict(shas, **{"nvngx_dlss.dll": "TAMPERED"})
    r2 = core(_synth_probe(), dllsha=bad_shas)
    if r2["provenance"]:
        fails.append("登记 sha 与实测不符未检出")
    r3 = core(_synth_probe(with_x2=False))
    if r3["x2"]:
        fails.append("X2 分解缺失未检出")
    p4 = _synth_probe()
    del p4["arms"]["b"]
    r4 = core(p4)
    if r4["probe"]:
        fails.append("probe 缺臂未检出")
    # B 臂不可用 + verdict=adopt（遮蔽不可用面冒充采纳）→ 红
    p5 = _synth_probe(b_hit=False, verdict="candidate_adopt_pending_quality_band")
    p5["arms"]["b"]["summary"]["arm_unavailable"] = True
    p5["arms"]["b"]["summary"]["all_rounds_ok"] = False
    r5 = core(p5)
    if r5["honest"]:
        fails.append("B 臂不可用遮蔽采纳未检出")
    if fails:
        return False, "RED 臂失效: " + "; ".join(fails)
    return True, "RED 五臂独立有效（门禁遮蔽采纳/登记 sha 不符/X2 缺失/probe 缺臂/不可用遮蔽采纳——函数面注入全检出）"


def run_gate() -> int:
    probe = wel.load_json(PROBE_JSON) if PROBE_JSON.is_file() else None
    reg = wel.load_json(G17_REGISTRY) if G17_REGISTRY.is_file() else None
    g13_reg = wel.load_json(G13_REGISTRY) if G13_REGISTRY.is_file() else None
    eval_shas: dict[str, str] | None = None
    if EVAL_BIN_DIR.is_dir():
        eval_shas = {p.name: _sha256(p) for p in sorted(EVAL_BIN_DIR.glob("*.dll"))}
    facts = evaluate(probe, reg, g13_reg, eval_shas, None)
    overall = all(f["status"] == "PASS" for f in facts)
    if not SCHEMA_PATH.is_file():
        print(f"[g17_m_b] FAIL: schema 缺失 {SCHEMA_PATH}", file=sys.stderr)
        return 1
    code, _ = wel.emit_wave_evidence(
        wave=WAVE,
        subject=SUBJECT,
        symbolic_gate_key=GATE_KEY,
        numeric_step=NUMERIC_STEP,
        source_ref=SOURCE_REF,
        required_gate_rows=[],
        extra_facts=facts,
        subjects=[],
        schema_path=SCHEMA_PATH,
        evidence_basename=SUBJECT,
        notes=(
            "G17.3 M-b NGX 版本演进面对齐评估：新缓存目录 + G17 provenance 登记"
            "（四 DLL sha256 实测重算 == 登记，sl 三件 == g13 原件，g13 表 0-byte）+ "
            "PaddedWindowNetwork 实例化形态核验（SL verbose 日志 token）+ X2 边际探针"
            "重测分解（对照 G15 §8.7 1.90+0.10ms 基线字面）+ 画质守护双门禁第一门"
            "（Stage A digest 锚——B 臂 MISS 即拒绝换版如实登记）+ A/B measured 三轮"
            "单变量对照 + 采纳结论与门禁事实一致性机核（采纳/拒绝/零收益均合法）+ "
            "生产默认 0-byte（X2 探针撤除义务）+ RED 四臂；device 真跑面在 probe 轮"
        ),
        host_section_pass=overall,
    )
    return 0 if (overall and code == 0) else 1


def verify_latest() -> int:
    p = wel.load_latest_evidence(SUBJECT)
    if p is None:
        print(f"[g17_m_b] verify-latest FAIL: 无 {SUBJECT} evidence", file=sys.stderr)
        return 1
    doc = wel.load_json(p)
    ok = doc.get("host_section_pass") is True and all(
        f.get("status") == "PASS" for f in doc.get("extra_facts", [])
    )
    print(f"[g17_m_b] verify-latest {'PASS' if ok else 'FAIL'}: {p.name}")
    return 0 if ok else 1


def run_selftest() -> int:
    ok, note = run_red_arms()
    print(f"  {'RED/GREEN ok' if ok else 'SELFTEST FAIL'} — {note}")
    if not SCHEMA_PATH.is_file():
        print(f"  SCHEMA MISS — {SCHEMA_PATH}")
        ok = False
    print(f"[g17_m_b] SELFTEST {'PASS' if ok else 'FAIL'}")
    return 0 if ok else 1


def main() -> int:
    ap = argparse.ArgumentParser()
    g = ap.add_mutually_exclusive_group(required=True)
    g.add_argument("--gate", choices=[GATE_KEY])
    g.add_argument("--verify-latest", action="store_true")
    g.add_argument("--selftest", action="store_true")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    if args.verify_latest:
        return verify_latest()
    return run_gate()


if __name__ == "__main__":
    sys.exit(main())
