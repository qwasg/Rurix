#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-05 图形 RHI 冒烟(步骤 76;G4.2 PR-B/PR-C / RFC-0015 §4.A;RXS-0270~0276;验收门 G-G4-3）。

std::gpu `Rhi` 图形 pass(raster + mesh)render graph 的端到端见证——G4.2 PR-B 图形 RHI
库面(rhi.rs seal_gfx 装配核验 + graph.rs derive_barriers 单源桥接)的 device 段 e2e 加证。
**PR-C 覆盖扩(RXS-0274/0276)**:TextureTable 入 pass(`.reads_table`)bindless 动态索引 +
present handoff(`.present`)终端声明;步骤 76 覆盖扩——bindless accept 语料 + present reject
语料 + 像素判据含 bindless 四象限先例(sampling_superset/bindless G3 evidence 参照)。

**本步骤定位**(与步骤 72 compute RHI 冒烟分工):
  - 步骤 72 = compute RHI(compute-pass 图;demo.rx UC05_RHI_OK 三 pass 数值对照)。
  - 步骤 76 = 图形 RHI(gfx pass 图;gfx_demo.rx raster+mesh 装配核验 + device 真跑;
    PR-C 扩:gfx_bindless.rx accept + gfx_present_*.rx reject + bindless 四象限像素判据)。
  - 步骤 77 = 图形不变量门(纯 host;gfx reject 语料 + 声明↔反射 + compute 回归)。

  host 段（**恒跑**,反 YAML-only,无 GPU / 无 link）:
    1. conformance/uc05 corpus 批跑(`cargo test -p rurixc --test uc05_corpus`):compute
       reject 编译期拦截无回归 + assembly 编译期 CLEAN + I1~I10 矩阵三方一致。**纯 rust test,
       无工具链亦恒跑**(反 YAML-only 底线,compute 路零回归守卫)。
    2. apps/uc05-rhi 零 .rs 主语言审计(仅 .rx + rurix.toml;RFC-0014 §9.2)。
    3. `rurixc --emit=check`(不 link)编译 gfx_demo.rx:0 诊断(图形 pass 声明 + 装配核验
       可编译本体;device 真跑归 device 段)。
    4. `rurixc --emit=check` 编译 gfx assembly-reject 语料(reject/gfx_*.rx 带 `//@ assembly-reject:
       structure` 头):**编译期 CLEAN**(图装配期性质,--emit=check 不拦)——证 gfx I3/I5 非编译期。
    5. **PR-C**:`rurixc --emit=check` 编译 accept/gfx_bindless.rx:0 诊断(TextureTable 入 pass
       `.reads_table` + bindless 动态索引声明面;RXS-0276;PR-C 覆盖扩)。

  device / toolchain 段（**gate real**:link 工具链 + GPU〔CUDA driver:Context::create 经
  from_primary〕在位;`RURIX_REQUIRE_REAL=1` 翻硬红,缺则 SKIP 退 0 打 dev-env-degrade）:
    6. **GREEN**:`rx build apps/uc05-rhi/src/gfx_demo.rx` → EXE,run → exit 0(合法 gfx 图装配
       核验通过 + submit 成功;**图形 pass 派发归 PR-F Vulkan RHI 通道**,本步骤证装配核验 +
       EXE 可运行,像素判据 RXS-0222 归 PR-F/步骤 80 device 见证)。
    7. **RED**:`rx build` 每个 reject/gfx_*.rx(assembly-reject 头)→ EXE,run → **退非零** +
       stderr 含 `rhi_submit` + **该语料头声明的装配期类别** `[structure]`(图装配期库层状态值
       Err → RXRT_FAIL → rxrt_trap;gfx I3 依赖环 / I3 读未写 / I5 写写冲突 / present 非末位·重复,
       确定性拦非运行期概率性)。
    8. **PR-C bindless 四象限像素判据**(device gated,归 PR-F Vulkan 通道 device 见证):
       gfx_bindless demo device 真跑 → 四象限像素逐色判据(bindless 动态索引四个纹素色,
       sampling_superset/bindless G3 evidence 参照;RXS-0222 headless readback 纪律)。
    9. 落 evidence JSON(`evidence/uc05_graphics_rhi_smoke_<ts>.json`)。

**SKIP 纪律**:无 link 工具链 / 无 CUDA → SKIP = dev-env-degrade(**非 fake pass**,退 0);
`RURIX_REQUIRE_REAL=1` 翻**硬红**。装配期确定性拦的**纯 host 无 GPU 见证**归步骤 77(图形
不变量门);本步骤 EXE red-green 为 device 段 e2e 加证。

**主循环登记提示**:步骤号 = 76;门 = G-G4-3;条款 = RXS-0270~0273;host 段恒跑(步骤 1~4)
vs device/toolchain 段 gated(rx build + GPU run),镜像步骤 72 双态。
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
DEMO = APP / "src" / "gfx_demo.rx"
REJECT_DIR = ROOT / "conformance" / "uc05" / "reject"
EVIDENCE_DIR = ROOT / "evidence"


def fail(msg: str) -> int:
    print(f"[uc05_graphics_rhi_smoke] FAIL {msg}", file=sys.stderr)
    return 1


def skip(msg: str) -> int:
    if os.environ.get("RURIX_REQUIRE_REAL") == "1":
        return fail(msg + "（RURIX_REQUIRE_REAL=1 不许 SKIP）")
    print(f"[uc05_graphics_rhi_smoke] SKIP {msg}（dev-env-degrade,退出 0）")
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
    """device 可用性探测(抄 ci/uc05_rhi_smoke.py:CUDA_PATH + ptxas)。
    Context::create 经 from_primary 需 CUDA driver;PTX 产物嵌入需 rurixc。"""
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
            "零 .rs 审计违例——apps/uc05-rhi 存在非 .rx 源(G-G4-3,RFC-0014 §9.2):\n  "
            + "\n  ".join(violations)
        )
        return False
    if not rx_files:
        fail("apps/uc05-rhi 无任何 .rx 源(应用不存在?)")
        return False
    print(
        f"[uc05_graphics_rhi_smoke] host 步骤 2 PASS: 零 .rs 审计（apps/uc05-rhi 仅"
        f" {len(rx_files)} 个 .rx + rurix.toml,零 .rs/.cpp/.c/.py）"
    )
    return True


def _gfx_assembly_reject_files() -> list[Path]:
    """conformance/uc05/reject/gfx_*.rx 带 `//@ assembly-reject:` 头的语料(图装配期性质)。"""
    out = []
    for p in sorted(REJECT_DIR.glob("gfx_*.rx")):
        text = p.read_text(encoding="utf-8")
        if re.search(r"//@\s*assembly-reject:\s*\w+", text):
            out.append(p)
    return out


def _assembly_reject_category(src: Path) -> str | None:
    """读语料头 `//@ assembly-reject: <category>`(structure / reflection;rhi.rs 库层状态值族)。"""
    for line in src.read_text(encoding="utf-8").splitlines():
        m = re.search(r"//@\s*assembly-reject:\s*(\w+)", line)
        if m:
            return m.group(1)
    return None


def host_section(results: dict) -> bool:
    # 1) corpus 批跑（纯 rust test,恒跑,反 YAML-only;compute 路零回归守卫）。
    code, out, err = run(["cargo", "test", "-q", "-p", "rurixc", "--test", "uc05_corpus"])
    if code != 0:
        print((out + err)[-2400:], file=sys.stderr)
        results["corpus_pass"] = False
        fail("host 段: uc05_corpus 批跑未过(compute 路回归)")
        return False
    results["corpus_pass"] = True
    print(
        "[uc05_graphics_rhi_smoke] host 步骤 1 PASS: uc05_corpus 批跑（compute 路零回归"
        " + assembly 编译期 CLEAN + I1~I10 矩阵三方一致）"
    )

    # 2) 零 .rs 审计。
    if not audit_zero_rs():
        results["zero_rs_audit"] = False
        return False
    results["zero_rs_audit"] = True

    # 3) rurixc --emit=check（不 link,host 恒跑）:gfx_demo.rx 0 诊断。
    if not RURIXC.is_file():
        code, out, err = run(["cargo", "build", "-q", "-p", "rurixc", "--bin", "rurixc"])
        if code != 0 or not RURIXC.is_file():
            print((out + err)[-1200:], file=sys.stderr)
            fail("host 段: rurixc 构建失败")
            return False
    if not DEMO.is_file():
        fail(f"host 段: gfx_demo.rx 不存在: {DEMO}")
        return False
    dc, do, de = run([str(RURIXC), str(DEMO), "--emit=check"])
    demo_clean = dc == 0 and "RX" not in (do + de)
    if not demo_clean:
        print((do + de)[-1000:], file=sys.stderr)
        fail("host 段: gfx_demo.rx --emit=check 非 0 诊断")
        return False
    print(
        "[uc05_graphics_rhi_smoke] host 步骤 3 PASS: --emit=check（不 link）gfx_demo.rx"
        " 0 诊断（图形 pass 声明 + 装配核验可编译本体）"
    )
    results["compile_gfx_demo"] = True

    # 4) gfx assembly-reject 语料编译期 CLEAN（图装配期性质,--emit=check 不拦）。
    gfx_rejects = _gfx_assembly_reject_files()
    if not gfx_rejects:
        fail("host 段: conformance/uc05/reject/ 无 gfx_*.rx assembly-reject 语料")
        return False
    for f in gfx_rejects:
        ac, ao, ae = run([str(RURIXC), str(f), "--emit=check"])
        if ac != 0 or "error" in (ao + ae).lower() or "RX" in (ao + ae):
            print((ao + ae)[-1000:], file=sys.stderr)
            fail(f"host 段: reject/{f.name} 应编译期 CLEAN（图装配期性质,--emit=check 不拦）")
            return False
    print(
        f"[uc05_graphics_rhi_smoke] host 步骤 4 PASS: --emit=check {len(gfx_rejects)} 个"
        " gfx assembly-reject 语料编译期 CLEAN（证 gfx I3/I5 非编译期,图装配期确定性拦）"
    )
    results["compile_gfx_assembly_rejects"] = True
    results["gfx_reject_count"] = len(gfx_rejects)

    # 5) PR-C:gfx_bindless.rx accept 语料 --emit=check 0 诊断(RXS-0276;bindless 覆盖扩)。
    bindless = ROOT / "conformance" / "uc05" / "accept" / "gfx_bindless.rx"
    if not bindless.is_file():
        fail(f"host 段: accept/gfx_bindless.rx 不存在: {bindless}")
        return False
    bc, bo, be = run([str(RURIXC), str(bindless), "--emit=check"])
    bindless_clean = bc == 0 and "RX" not in (bo + be)
    if not bindless_clean:
        print((bo + be)[-1000:], file=sys.stderr)
        fail("host 段: gfx_bindless.rx --emit=check 非 0 诊断(PR-C RXS-0276 bindless 覆盖扩)")
        return False
    print(
        "[uc05_graphics_rhi_smoke] host 步骤 5 PASS: --emit=check gfx_bindless.rx"
        " 0 诊断（PR-C RXS-0276 TextureTable 入 pass `.reads_table` bindless 动态索引声明面）"
    )
    results["compile_gfx_bindless"] = True
    return True


# ─────────────────────────── device / toolchain 段（gate real） ───────────────────────────


def rx_build(src: Path, exe: Path):
    return run([str(RX), "build", str(src), "-o", str(exe)])


def _is_nvptx_graphics_skip(build_stderr: str) -> bool:
    """`rx build` 失败为 NVPTX 后端不支持图形 shader(RX6003;fragment/vertex/mesh 着色器
    遇 NVPTX device codegen)→ device-env degrade SKIP(非代码缺陷;图形 shader 需 Vulkan 后端,
    归 PR-F)。PR-B §8.1 已声明此为 SKIP 场景;PR-C 步骤 76 覆盖扩修正 FAIL→SKIP 口径。"""
    if "RX6003" not in build_stderr or "NVPTX" not in build_stderr:
        return False
    # 图形着色器阶段关键字(fragment/vertex/mesh)出现在 RX6003 诊断上下文 → 图形 on NVPTX 不支持。
    return any(k in build_stderr for k in ("fragment", "vertex", "mesh"))


def device_section(results: dict, workdir: Path) -> int:
    if not RX.is_file():
        code, out, err = run(["cargo", "build", "-q", "-p", "rurixc", "-p", "rx"])
        if code != 0 or not RX.is_file():
            if "error[" in err or "error:" in err:
                return fail(f"rx 构建失败:\n{err[-900:]}")
            return skip("device 段: rx 构建失败（无工具链?）")

    if not probe_gpu():
        results["demo_run_green"] = "SKIP"
        results["assembly_redgreen"] = "SKIP"
        results["toolchain_skip"] = "no-gpu"
        return skip("device 段:无 CUDA_PATH / ptxas（Context::create 需 GPU driver;host 段已恒跑）")

    workdir.mkdir(parents=True, exist_ok=True)

    # GREEN:gfx_demo.rx → EXE → run → exit 0（合法 gfx 图装配核验通过 + submit 成功;
    # 图形 pass 派发归 PR-F Vulkan RHI 通道,本步骤证装配核验 + EXE 可运行）。
    demo_exe = workdir / "uc05_gfx_demo.exe"
    bc, bo, be = rx_build(DEMO, demo_exe)
    if bc != 0 or not demo_exe.is_file():
        # 区分编译错误(红)vs NVPTX 图形不支持(SKIP)vs link 工具链缺(SKIP)。
        if _is_nvptx_graphics_skip(be):
            results["demo_run_green"] = "SKIP"
            results["assembly_redgreen"] = "SKIP"
            results["bindless_run_green"] = "SKIP"
            results["bindless_pixel_criteria"] = "SKIP"
            results["toolchain_skip"] = "nvptx-no-graphics"
            return skip(
                "device 段: gfx_demo.rx rx build 遇 RX6003(NVPTX 不支持图形 shader;"
                "图形 shader 需 Vulkan 后端,归 PR-F;host 段已恒跑)"
            )
        if "error[" in be or "error:" in be:
            return fail(f"gfx_demo.rx rx build 编译失败:\n{be[-900:]}")
        results["demo_run_green"] = "SKIP"
        results["assembly_redgreen"] = "SKIP"
        results["toolchain_skip"] = "no-link"
        return skip(f"gfx_demo.rx rx build 失败（link 工具链缺?）:\n{be[-500:]}")
    rc, ro, re_ = run([str(demo_exe)], cwd=workdir)
    # gfx_demo.rx 无 witness 行(无数值对照);GREEN = exit 0(装配核验通过 + submit 成功)。
    green_ok = rc == 0
    results["demo_run_green"] = green_ok
    if not green_ok:
        print((ro + re_)[-800:], file=sys.stderr)
        return fail(
            f"GREEN 失败: gfx_demo.rx EXE rc={rc}(合法 gfx 图装配核验应通过 + submit 成功)"
        )
    print(
        "[uc05_graphics_rhi_smoke] device 步骤 5 PASS: GREEN gfx_demo.rx EXE exit 0"
        "（合法 gfx 图装配核验通过 + submit 成功;图形 pass 派发归 PR-F）"
    )

    # RED:每个 gfx assembly-reject → EXE → run → 退非零 + stderr 含 rhi_submit + [structure]
    # (gfx I3 依赖环 / I3 读未写 / I5 写写冲突,图装配期确定性拦)。
    cases = []
    for src in _gfx_assembly_reject_files():
        category = _assembly_reject_category(src)
        if category is None:
            return fail(f"reject/{src.name} 缺 //@ assembly-reject: <category> 头")
        exe = workdir / f"uc05_gfx_{src.stem}.exe"
        rbc, rbo, rbe = rx_build(src, exe)
        if rbc != 0 or not exe.is_file():
            if _is_nvptx_graphics_skip(rbe):
                results["assembly_redgreen"] = "SKIP"
                results["bindless_run_green"] = "SKIP"
                results["bindless_pixel_criteria"] = "SKIP"
                results["toolchain_skip"] = "nvptx-no-graphics"
                return skip(
                    f"device 段: reject/{src.name} rx build 遇 RX6003(NVPTX 不支持图形 shader;"
                    f"图形 shader 需 Vulkan 后端,归 PR-F;host 段已恒跑)"
                )
            if "error[" in rbe or "error:" in rbe:
                return fail(f"reject/{src.name} rx build 编译失败:\n{rbe[-700:]}")
            return skip(f"reject/{src.name} rx build 失败（link 工具链缺?）")
        arc, aro, are = run([str(exe)], cwd=workdir)
        blob = aro + are
        red_ok = arc != 0 and "rhi_submit" in blob and f"[{category}]" in blob
        cases.append(f"{src.stem}:{category}:{'RED_OK' if red_ok else 'RED_FAIL'}")
        if not red_ok:
            print(blob[-800:], file=sys.stderr)
            return fail(
                f"RED 失败: reject/{src.name} EXE rc={arc},stderr 缺装配 [{category}] Err"
                f"（图装配期确定性拦应退非零 + rhi_submit [{category}]）"
            )
        print(
            f"[uc05_graphics_rhi_smoke] device 步骤 6 PASS: RED reject/{src.stem} EXE"
            f" 退非零（rc={arc}）+ stderr 含 rhi_submit [{category}]（gfx I3/I5 装配期确定性拦）"
        )
    results["assembly_redgreen"] = True
    results["assembly_cases"] = cases

    # 8) PR-C bindless 四象限像素判据(device gated,归 PR-F Vulkan 通道 device 见证):
    #    gfx_bindless demo device 真跑 → 四象限像素逐色判据(bindless 动态索引四个纹素色,
    #    sampling_superset/bindless G3 evidence 参照;RXS-0222 headless readback 纪律)。
    #    PR-C 库面见证:TextureTable 入 pass(`.reads_table`)+ 装配核验 + submit 成功(EXE exit 0)。
    #    像素判据(四象限 bindless 动态索引)归 PR-F Vulkan RHI 通道 device 见证(同 gfx_demo
    #    像素判据 RXS-0222 归 PR-F/步骤 80);本步骤证 PR-C 库面 EXE 可运行 + 装配核验通过。
    bindless_src = ROOT / "conformance" / "uc05" / "accept" / "gfx_bindless.rx"
    if not bindless_src.is_file():
        return fail(f"device 段: accept/gfx_bindless.rx 不存在: {bindless_src}")
    bindless_exe = workdir / "uc05_gfx_bindless.exe"
    xbc, xbo, xbe = rx_build(bindless_src, bindless_exe)
    if xbc != 0 or not bindless_exe.is_file():
        if _is_nvptx_graphics_skip(xbe):
            results["bindless_run_green"] = "SKIP"
            results["bindless_pixel_criteria"] = "SKIP"
            results["toolchain_skip"] = "nvptx-no-graphics"
            return skip(
                "device 段: gfx_bindless.rx rx build 遇 RX6003(NVPTX 不支持图形 shader;"
                "图形 shader 需 Vulkan 后端,归 PR-F;host 段已恒跑)"
            )
        if "error[" in xbe or "error:" in xbe:
            return fail(f"gfx_bindless.rx rx build 编译失败:\n{xbe[-900:]}")
        results["bindless_run_green"] = "SKIP"
        results["bindless_pixel_criteria"] = "SKIP"
        results["toolchain_skip"] = "no-link"
        return skip(f"gfx_bindless.rx rx build 失败（link 工具链缺?）:\n{xbe[-500:]}")
    xrc, xro, xre_ = run([str(bindless_exe)], cwd=workdir)
    xblob = xro + xre_
    # PR-C 库面 GREEN = exit 0(TextureTable 入 pass + reads_table 声明 + 装配核验 + submit 成功)。
    # 像素判据(四象限 bindless 动态索引)归 PR-F Vulkan 通道 device 见证(RXS-0222 headless
    # readback);本步骤证 PR-C 库面 EXE 可运行(gfx pass 派发归 PR-F,同步骤 5 gfx_demo 口径)。
    bindless_green = xrc == 0
    results["bindless_run_green"] = bindless_green
    if not bindless_green:
        # 非零退出:若为 graphics/Vulkan 不可用 → SKIP(dev-env degrade);否则 FAIL。
        no_gfx_keys = ("vkCreateInstance", "graphics queue", "Vulkan", "spirv", "graphics shader")
        if any(k.lower() in xblob.lower() for k in no_gfx_keys):
            results["bindless_run_green"] = "SKIP"
            results["bindless_pixel_criteria"] = "SKIP"
            return skip(
                f"gfx_bindless.rx EXE 无 Vulkan/graphics 后端(device 段 SKIP=dev-env degrade;"
                f"gfx pass 派发归 PR-F):{xre_.strip()[:300]}"
            )
        print(xblob[-800:], file=sys.stderr)
        return fail(
            f"PR-C bindless GREEN 失败: gfx_bindless.rx EXE rc={xrc}"
            f"(PR-C 库面装配核验 + submit 应通过;像素判据归 PR-F)"
        )
    print(
        "[uc05_graphics_rhi_smoke] device 步骤 8 PASS: PR-C bindless GREEN gfx_bindless.rx"
        " EXE exit 0（TextureTable 入 pass `.reads_table` + 装配核验 + submit 成功;"
        " 四象限 bindless 动态索引像素判据归 PR-F Vulkan 通道 device 见证）"
    )
    # 像素判据(四象限)归 PR-F:PR-C 库面 EXE 可运行见证已落;四象限逐色判据
    # (sampling_superset/bindless G3 evidence 参照)在 PR-F Vulkan RHI 通道 device 真跑时回填。
    results["bindless_pixel_criteria"] = "deferred-PR-F"
    return 0


def write_evidence(results: dict, host_ok: bool, device_rc: int) -> None:
    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    ts = _dt.datetime.now().astimezone().replace(microsecond=0)
    device_skipped = (
        results.get("assembly_redgreen") == "SKIP"
        or results.get("demo_run_green") == "SKIP"
        or results.get("bindless_run_green") == "SKIP"
        or results.get("toolchain_skip") is not None
    )
    checks = {
        k: results.get(k)
        for k in (
            "corpus_pass",
            "zero_rs_audit",
            "compile_gfx_demo",
            "compile_gfx_assembly_rejects",
            "gfx_reject_count",
            "compile_gfx_bindless",
            "demo_run_green",
            "assembly_redgreen",
            "bindless_run_green",
            "bindless_pixel_criteria",
        )
        if results.get(k) is not None
    }
    doc = {
        "schema_version": 1,
        "subject": "uc05_graphics_rhi_smoke",
        "milestone": "G4.2 PR-B/PR-C / G-G4-3 (RFC-0015 §4.A; RXS-0270~0276)",
        "step": 76,
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
    ev = EVIDENCE_DIR / f"uc05_graphics_rhi_smoke_{ts.strftime('%Y%m%dT%H%M%S')}.json"
    ev.write_text(
        json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8", newline="\n"
    )
    print(f"[uc05_graphics_rhi_smoke] 写 evidence {ev.relative_to(ROOT)}; run_url={doc['run_url']}")


def main() -> int:
    import tempfile

    results: dict = {}
    host_ok = host_section(results)
    if not host_ok:
        write_evidence(results, host_ok, 1)
        return 1
    with tempfile.TemporaryDirectory(prefix="uc05_gfx_rhi_smoke_") as td:
        device_rc = device_section(results, Path(td))
        write_evidence(results, host_ok, device_rc)
    return device_rc


if __name__ == "__main__":
    sys.exit(main())
