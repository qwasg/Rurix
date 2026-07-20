#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 最小 RHI 冒烟(步骤 72;EI1.3 / RFC-0014 Part B;RXS-0256~0265;验收门 G-EI1-3）。

std::gpu `Rhi` compute-pass render graph 的端到端见证——编译期 100% 拦截(I1/I2/I6/I7/I8)
+ 图装配期确定性拦(I3/I5,`submit()` 时 host 侧 rxrt_trap)+ apps/uc05-rhi 零 .rs 主语言
判据(RFC-0014 §9.2)。

  host 段（**恒跑**,反 YAML-only,无 GPU / 无 link）:
    1. conformance/uc05 corpus 批跑(`cargo test -p rurixc --test uc05_corpus`):accept 0 诊断 +
       reject 编译期 I1/I2/I6/I7/I8 全拦截 + assembly 编译期 CLEAN + I1~I10 矩阵三方一致。
       **纯 rust test,无工具链亦恒跑**(反 YAML-only 底线)。
    2. apps/uc05-rhi 零 .rs 主语言审计(仅 .rx + rurix.toml;镜像 ci/uc07_offline_golden_smoke.py
       :90-113 零 .rs 审计先例,RFC-0014 §9.2)。
    3. `rurixc --emit=check`(不 link)编译 demo.rx + assembly/*.rx:demo 0 诊断 / assembly 编译期
       CLEAN(图装配期性质,--emit=check 不拦)——**host 恒跑证 I3/I5 非编译期**。

  device / toolchain 段（**gate real**:link 工具链〔MSVC/SDK,d3d12 stub 链接〕+ GPU〔CUDA
  driver:Context::create 经 from_primary〕在位;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP
  退 0 打 dev-env-degrade):
    4. **GREEN**:`rx build apps/uc05-rhi/src/demo.rx` → EXE,run → exit 0 + stdout 含
       `UC05_RHI_OK`(合法图装配核验通过 + submit 成功 + **真派发 compute pass** + **真 D2H**)
       + **I9 数值对照**:`UC05_SUM` == `UC05_REF`(device 求和 vs host 闭式参考,EI1.4)。
    5. **RED**:`rx build` 每个 conformance/uc05/assembly/*.rx → EXE,run → **退非零** + stderr 含
       `rhi_submit` + **该语料头声明的装配期类别** `[structure]` / `[reflection]`(图装配期库层
       状态值 Err → RXRT_FAIL → rxrt_trap;I3 依赖环 / **I4 未声明访问** / I5 写写冲突 / 空图
       生命周期,确定性拦非运行期概率性)。
    6. 落 evidence JSON(`evidence/uc05_rhi_smoke_<ts>.json`;schema
       milestones/ei1/uc05_rhi_smoke_evidence_schema.json)。

**SKIP 纪律**:无 link 工具链 / 无 CUDA → SKIP = dev-env-degrade(**非 fake pass**,退 0);
`RURIX_REQUIRE_REAL=1` 翻**硬红**。装配期确定性拦的**纯 host 无 GPU 见证**另由 rurix-rt
rhi.rs 库单测(`rejects_read_before_write_i3` / `rejects_write_write_conflict_i5` /
`rejects_lifecycle_misuse`)+ 步骤 73 承担(EXE red-green 为 device 段 e2e 加证)。

**主循环登记提示**:步骤号 = 72;门 = G-EI1-3;条款 = RXS-0256~0265;host 段恒跑(步骤 1~3)
vs device/toolchain 段 gated(rx build + GPU run),镜像 export_c_smoke / uc07 双态。
"""
from __future__ import annotations

import datetime as _dt
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
RURIXC = ROOT / "target" / "debug" / ("rurixc.exe" if os.name == "nt" else "rurixc")
APP = ROOT / "apps" / "uc05-rhi"
DEMO = APP / "src" / "demo.rx"
ASSEMBLY_DIR = ROOT / "conformance" / "uc05" / "assembly"
EVIDENCE_DIR = ROOT / "evidence"


def fail(msg: str) -> int:
    print(f"[uc05_rhi_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "（RURIX_REQUIRE_REAL=1 不许 SKIP）")
    print(f"[uc05_rhi_smoke] SKIP {msg}（dev-env-degrade,退出 0）")
    return 0


def run(cmd, cwd: Path = ROOT, timeout: int = 900):
    r = subprocess.run(cmd, capture_output=True, cwd=str(cwd), timeout=timeout)
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


def probe_gpu() -> bool:
    """device 可用性探测(抄 ci/uc07_offline_golden_smoke.py:CUDA_PATH + ptxas)。
    Context::create 经 from_primary 需 CUDA driver;PTX 产物嵌入需 rurixc(无需 ptxas 产 cubin
    亦可 PTX fallback,ptxas 在位为完整档)。"""
    cuda_path = os.environ.get("CUDA_PATH")
    if not cuda_path:
        return False
    ptxas = Path(cuda_path) / "bin" / ("ptxas.exe" if os.name == "nt" else "ptxas")
    return ptxas.exists()


# ─────────────────────────── host 段（恒跑） ───────────────────────────


def audit_zero_rs() -> bool:
    """apps/uc05-rhi 零 .rs 主语言审计(仅 .rx + rurix.toml)。"""
    if not APP.is_dir():
        fail(f"apps/uc05-rhi 不存在: {APP}")
        return False
    violations, rx_files = [], []
    for p in sorted(APP.rglob("*")):
        if p.is_dir():
            continue
        rel = p.relative_to(APP).as_posix()
        if rel == "rurix.toml":
            continue
        if p.suffix == ".rx":
            rx_files.append(rel)
            continue
        violations.append(rel)
    if violations:
        fail(
            "零 .rs 审计违例——apps/uc05-rhi 存在非 .rx 源(G-EI1-3,RFC-0014 §9.2):\n  "
            + "\n  ".join(violations)
        )
        return False
    if not rx_files:
        fail("apps/uc05-rhi 无任何 .rx 源(应用不存在?)")
        return False
    print(
        f"[uc05_rhi_smoke] host 步骤 2 PASS: 零 .rs 审计（apps/uc05-rhi 仅 {len(rx_files)} 个 .rx"
        " + rurix.toml,零 .rs/.cpp/.c/.py）"
    )
    return True


def host_section(results: dict) -> bool:
    # 1) corpus 批跑（纯 rust test,恒跑,反 YAML-only）。
    code, out, err = run(["cargo", "test", "-q", "-p", "rurixc", "--test", "uc05_corpus"])
    if code != 0:
        print((out + err)[-2400:], file=sys.stderr)
        results["corpus_pass"] = False
        fail("host 段: uc05_corpus 批跑未过")
        return False
    results["corpus_pass"] = True
    print(
        "[uc05_rhi_smoke] host 步骤 1 PASS: uc05_corpus 批跑（accept 0 诊断 + reject 编译期"
        " I1/I2/I6/I7/I8 全拦截 + assembly 编译期 CLEAN + I1~I10 矩阵三方一致）"
    )

    # 2) 零 .rs 审计。
    if not audit_zero_rs():
        results["zero_rs_audit"] = False
        return False
    results["zero_rs_audit"] = True

    # 3) rurixc --emit=check（不 link,host 恒跑）:demo 0 诊断 + assembly 编译期 CLEAN。
    if not RURIXC.is_file():
        code, out, err = run(["cargo", "build", "-q", "-p", "rurixc", "--bin", "rurixc"])
        if code != 0 or not RURIXC.is_file():
            print((out + err)[-1200:], file=sys.stderr)
            fail("host 段: rurixc 构建失败")
            return False
    dc, do, de = run([str(RURIXC), str(DEMO), "--emit=check"])
    demo_clean = dc == 0 and "RX" not in (do + de)
    if not demo_clean:
        print((do + de)[-1000:], file=sys.stderr)
        fail("host 段: demo.rx --emit=check 非 0 诊断")
        return False
    for f in sorted(ASSEMBLY_DIR.glob("*.rx")):
        ac, ao, ae = run([str(RURIXC), str(f), "--emit=check"])
        if ac != 0 or "error" in (ao + ae).lower() or "RX" in (ao + ae):
            print((ao + ae)[-1000:], file=sys.stderr)
            fail(f"host 段: assembly/{f.name} 应编译期 CLEAN（图装配期性质）")
            return False
    print(
        "[uc05_rhi_smoke] host 步骤 3 PASS: --emit=check（不 link）demo 0 诊断 + assembly"
        " 编译期 CLEAN（证 I3/I5 非编译期,图装配期确定性拦）"
    )
    results["compile_demo"] = True
    results["compile_assembly"] = True
    return True


# ─────────────────────────── device / toolchain 段（gate real） ───────────────────────────


def rx_build(src: Path, exe: Path):
    return run([str(RX), "build", str(src), "-o", str(exe)])


def _numeric_tokens(stdout: str) -> dict:
    """抽取 demo 的机器可核数值 token(`UC05_SUM=<u32>` / `UC05_REF=<u32>`,EI1.4 I9 对照)。"""
    out: dict = {}
    for m in re.finditer(r"^(UC05_(?:SUM|REF))=(\d+)\s*$", stdout, re.MULTILINE):
        out[m.group(1)] = int(m.group(2))
    return out


def _assembly_reject_category(src: Path) -> str | None:
    """读语料头 `//@ assembly-reject: <category>`(structure / reflection;rhi.rs 库层状态值族)。"""
    for line in src.read_text(encoding="utf-8").splitlines():
        m = re.search(r"//@\s*assembly-reject:\s*(\w+)", line)
        if m:
            return m.group(1)
    return None


def device_section(results: dict, workdir: Path) -> int:
    if not RX.is_file():
        code, out, err = run(["cargo", "build", "-q", "-p", "rurixc", "-p", "rx"])
        if code != 0 or not RX.is_file():
            if "error[" in err or "error:" in err:
                return fail(f"rx 构建失败:\n{err[-900:]}")
            return skip("rx 构建失败（无工具链?）")

    if not probe_gpu():
        results["demo_run_green"] = "SKIP"
        results["assembly_redgreen"] = "SKIP"
        results["toolchain_skip"] = "no-gpu"
        return skip("device 段:无 CUDA_PATH / ptxas（Context::create 需 GPU driver;host 段已恒跑）")

    workdir.mkdir(parents=True, exist_ok=True)

    # GREEN:demo → EXE → run → exit 0 + UC05_RHI_OK + **数值对照**(I9)。
    demo_exe = workdir / "uc05_demo.exe"
    bc, bo, be = rx_build(DEMO, demo_exe)
    if bc != 0 or not demo_exe.is_file():
        # 区分编译错误(红)vs link 工具链缺(SKIP)。
        if "error[" in be or "error:" in be:
            return fail(f"demo rx build 编译失败:\n{be[-900:]}")
        results["demo_run_green"] = "SKIP"
        results["assembly_redgreen"] = "SKIP"
        results["toolchain_skip"] = "no-link"
        return skip(f"demo rx build 失败（link 工具链缺?）:\n{be[-500:]}")
    rc, ro, re_ = run([str(demo_exe)], cwd=workdir)
    # I9 数值对照(EI1.4):demo 打印 device 求和与 host 闭式参考,须精确相等(demo 自身不等即
    # 退 2;此处**独立复核** token,防 demo 打印与判定脱节)。
    sums = _numeric_tokens(ro)
    green_ok = (
        rc == 0
        and "UC05_RHI_OK" in ro
        and sums.get("UC05_SUM") is not None
        and sums.get("UC05_SUM") == sums.get("UC05_REF")
    )
    results["demo_run_green"] = green_ok
    results["demo_numeric"] = (
        f"UC05_SUM={sums.get('UC05_SUM')} UC05_REF={sums.get('UC05_REF')}"
    )
    if not green_ok:
        print((ro + re_)[-800:], file=sys.stderr)
        return fail(
            f"GREEN 失败: demo EXE rc={rc}, stdout 缺 UC05_RHI_OK 或数值对照不等 "
            f"(UC05_SUM={sums.get('UC05_SUM')} vs UC05_REF={sums.get('UC05_REF')})"
        )
    print(
        "[uc05_rhi_smoke] device 步骤 4 PASS: GREEN demo EXE exit 0 + UC05_RHI_OK"
        f"（合法图 submit 通过 + 真派发 + 真 D2H;I9 数值对照 {results['demo_numeric']} 相等）"
    )

    # RED:每个 assembly → EXE → run → 退非零 + stderr 含 rhi_submit + **该语料声明的装配期类别**
    # (`//@ assembly-reject: <structure|reflection>` 头;类别 = rhi.rs 库层状态值族)。
    cases = []
    for src in sorted(ASSEMBLY_DIR.glob("*.rx")):
        category = _assembly_reject_category(src)
        if category is None:
            return fail(f"assembly/{src.name} 缺 //@ assembly-reject: <category> 头")
        exe = workdir / f"uc05_{src.stem}.exe"
        rbc, rbo, rbe = rx_build(src, exe)
        if rbc != 0 or not exe.is_file():
            if "error[" in rbe or "error:" in rbe:
                return fail(f"assembly/{src.name} rx build 编译失败:\n{rbe[-700:]}")
            return skip(f"assembly/{src.name} rx build 失败（link 工具链缺?）")
        arc, aro, are = run([str(exe)], cwd=workdir)
        blob = aro + are
        red_ok = arc != 0 and "rhi_submit" in blob and f"[{category}]" in blob
        cases.append(f"{src.stem}:{category}:{'RED_OK' if red_ok else 'RED_FAIL'}")
        if not red_ok:
            print(blob[-800:], file=sys.stderr)
            return fail(
                f"RED 失败: assembly/{src.name} EXE rc={arc},stderr 缺装配 [{category}] Err"
                f"（图装配期确定性拦应退非零 + rhi_submit [{category}]）"
            )
        print(
            f"[uc05_rhi_smoke] device 步骤 5 PASS: RED assembly/{src.stem} EXE 退非零"
            f"（rc={arc}）+ stderr 含 rhi_submit [{category}]（I3/I4/I5/生命周期装配期确定性拦）"
        )
    results["assembly_redgreen"] = True
    results["assembly_cases"] = cases
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = results.get("assembly_redgreen") == "SKIP"
    checks = {
        k: results.get(k)
        for k in (
            "corpus_pass",
            "zero_rs_audit",
            "compile_demo",
            "compile_assembly",
            "demo_run_green",
            "demo_numeric",
            "assembly_redgreen",
        )
        if results.get(k) is not None
    }
    doc = {
        "schema_version": 1,
        "subject": "uc05_rhi_smoke",
        "milestone": "EI1.3 / G-EI1-3 (RFC-0014 Part B; RXS-0256~0265)",
        "step": 72,
        "host_section_pass": host_ok,
        "device_section_rc": device_rc,
        "checks": checks,
        "toolchain_skip": results.get("toolchain_skip"),
        "dev_env_degrade": device_skipped or results.get("toolchain_skip") is not None,
        "run_url": github_run_url(),
        "timestamp": ts.isoformat(),
    }
    if results.get("assembly_cases"):
        doc["assembly_cases"] = results["assembly_cases"]
    ev = EVIDENCE_DIR / f"uc05_rhi_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[uc05_rhi_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    import tempfile

    results: dict = {}
    host_ok = host_section(results)
    if not host_ok:
        write_evidence(results, host_ok, 1)
        return 1
    with tempfile.TemporaryDirectory(prefix="uc05_rhi_smoke_") as td:
        device_rc = device_section(results, Path(td))
        write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
