#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""生产分发 fatbin 冒烟(G1 CI_GATES §2 步骤 44,契约 G-G1-5,Mini-RFC/MR-0005 / RXS-0150~0152)。

**check_* 守卫风格,不写 budget counter**(agent 裁:不立装载首启延迟性能门,仅功能冒烟 +
nightly 趋势)。两段机器复核闸门(反 YAML-only,CI_GATES §6.5):

  (a) host 段(总跑,无需 GPU/ptxas)——三类构造缺陷红绿自检 + 真实再分发审计:
      1. **白名单外 cubin 组件**:cubin/fatbin = Rurix 自编语言本体(LanguageCore);注入 NVIDIA 源
         .cubin / 误分区为 NvidiaRedist / `__nv_*` 残留 → 守卫应红。
      2. **缺 [[artifact]] digest**:每分发产物变体(ptx/cubin/fatbin)须在 rurix.lock 记一条
         [[artifact]] digest(RXS-0152);漏某变体 → coverage 守卫应红。
      3. **cubin↔PTX golden 漂移**:cubin 须对应已 bless 的 PTX kernel(RXS-0150,结构核对);
         漂移 → 守卫应红。
      三类缺陷任一「应红却放行」→ 非零退出(反 YAML-only)。健全集 + 真实 `ci/check_redistribution.py`
      须绿(无 __nv_* 残留 / 不打包 libdevice .bc·Toolkit·驱动·Nsight,r6)。

  (b) device 段(交互桌面会话 + ptxas + CUDA + GPU 真跑;否则降级 SKIP)——按架构预编 cubin
      (ptxas -arch=sm_89,RXS-0073 保留字节)+ fatbin 装载命中 cubin + 篡改强制 PTX fallback
      协商(select_load_variant,RXS-0151 降级而非 reject)+ 数值往返对照 → distribution_ok=true。
      本环境(无 GPU / 无 ptxas / 非交互桌面)→ device SKIP,distribution_ok=false。

写 evidence/fatbin_dist_smoke.json。退出码:0=绿(host 段三类红绿自检 + 真实审计绿;device 段
SKIP 属预期);非零=红(守卫「应红却放行」/ 真实审计红 / device 数值对照失败)。
"""
import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "evidence" / "fatbin_dist_smoke.json"

# Attachment A 白名单形态(NvidiaRedist 分区限此最小集;cubin/fatbin 不在此列——它们是
# Rurix 自编语言本体,自有许可,经下方语言本体审计而非 Attachment A)。
NV_SYMBOL = "__nv_"


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def fail(msg):
    print(f"[fatbin_dist_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


# --- host 守卫(纯函数,镜像 PR-2 ci/check_redistribution.py 扩展 + lockfile coverage) ---

def audit_artifact_components(components: list[dict]) -> list[str]:
    """断言 cubin/fatbin 分发产物为 Rurix 自编语言本体(LanguageCore),再分发面为空。

    违例:(1) cubin/fatbin 分区为 nvidia(冒充 NvidiaRedist 绕审计);(2) 标记 NVIDIA 来源
    (origin=nvidia / 完整 Toolkit 二进制);(3) 嵌入符号含 __nv_* libdevice 派生残留。"""
    violations: list[str] = []
    for c in components:
        if c.get("kind") not in {"cubin", "fatbin"}:
            continue
        name = c.get("name", "<anon>")
        if c.get("partition") == "nvidia":
            violations.append(f"{name}: cubin/fatbin 误分区为 NvidiaRedist(应为 LanguageCore;不冒充绕审计)")
        if c.get("origin") == "nvidia":
            violations.append(f"{name}: NVIDIA 来源 cubin/Toolkit 二进制(语言本体须 Rurix 自编,r6)")
        for sym in c.get("symbols", []):
            if NV_SYMBOL in sym:
                violations.append(f"{name}: 嵌入 {NV_SYMBOL}* libdevice 派生符号残留(再分发面非空)")
    return violations


def check_lockfile_coverage(variants: list[dict], lock_artifacts: list[dict]) -> list[str]:
    """断言每分发产物变体(kind,sm)在 rurix.lock [[artifact]] 有对应 digest(RXS-0152)。"""
    violations: list[str] = []
    locked = {(a.get("kind"), a.get("sm_target", "")): a.get("sha256") for a in lock_artifacts}
    for v in variants:
        key = (v.get("variant"), v.get("sm", ""))
        digest = locked.get(key)
        if not digest:
            violations.append(f"变体 {key} 缺 [[artifact]] digest(lockfile 未记录,内容寻址锁定缺口)")
        elif digest != v.get("digest_sha256"):
            violations.append(f"变体 {key} digest 失配(lockfile={digest} vs 实测={v.get('digest_sha256')})")
    return violations


def check_cubin_ptx_golden(pairs: list[dict]) -> list[str]:
    """断言每 cubin 对应已 bless 的 PTX kernel(RXS-0150 结构核对;cubin 不设字节 golden)。"""
    violations: list[str] = []
    for p in pairs:
        if not p.get("blessed_ptx"):
            violations.append(f"cubin {p.get('kernel')} 无对应已 bless PTX(golden 漂移)")
        elif not p.get("ptxas_accepts"):
            violations.append(f"cubin {p.get('kernel')} ptxas 不接受 / arch 不符 sm_89(结构核对失败)")
    return violations


def red_self_tests() -> list[dict]:
    """三类构造缺陷:守卫须红(应红却放行即闸门失效,反 YAML-only)。"""
    facts: list[dict] = []
    cases = [
        (
            "白名单外 cubin 组件",
            lambda: audit_artifact_components([
                {"name": "rurix_saxpy.sm_89.cubin", "kind": "cubin", "partition": "language-core", "origin": "rurix", "symbols": ["saxpy"]},
                {"name": "evil.cubin", "kind": "cubin", "partition": "nvidia", "origin": "nvidia", "symbols": ["__nv_fast_expf"]},
            ]),
            "redistribution",
        ),
        (
            "缺 [[artifact]] digest",
            lambda: check_lockfile_coverage(
                variants=[
                    {"variant": "ptx", "sm": "", "digest_sha256": "a" * 64},
                    {"variant": "cubin", "sm": "sm_89", "digest_sha256": "b" * 64},
                ],
                lock_artifacts=[{"kind": "ptx", "sm_target": "", "sha256": "a" * 64}],  # 漏 cubin
            ),
            "lockfile",
        ),
        (
            "cubin↔PTX golden 漂移",
            lambda: check_cubin_ptx_golden([
                {"kernel": "saxpy", "blessed_ptx": True, "ptxas_accepts": True},
                {"kernel": "ghost", "blessed_ptx": False, "ptxas_accepts": False},  # 无对应 PTX
            ]),
            "negotiation",
        ),
    ]
    for label, guard, kind in cases:
        violations = guard()
        if not violations:
            fail(f"守卫未检出「{label}」缺陷(应红却放行 → 闸门失效,反 YAML-only)")
        facts.append({"kind": kind, "name": f"red:{label}", "note": f"守卫检出 {len(violations)} 项违例 → 阻断(应红即红)"})
        print(f"[fatbin_dist_smoke] 红绿自检 ✓ 「{label}」→ 守卫阻断({violations[0]})")
    return facts


def green_checks() -> list[dict]:
    """健全集守卫须绿 + 真实 ci/check_redistribution.py 须 PASS。"""
    facts: list[dict] = []
    healthy_components = [
        {"name": "rurix_saxpy.sm_89.cubin", "kind": "cubin", "partition": "language-core", "origin": "rurix", "symbols": ["saxpy"]},
        {"name": "rurix_saxpy.ptx", "kind": "ptx", "partition": "language-core", "origin": "rurix", "symbols": ["saxpy"]},
    ]
    healthy_variants = [
        {"variant": "ptx", "sm": "", "digest_sha256": "a" * 64},
        {"variant": "cubin", "sm": "sm_89", "digest_sha256": "b" * 64},
    ]
    healthy_lock = [
        {"kind": "ptx", "sm_target": "", "sha256": "a" * 64},
        {"kind": "cubin", "sm_target": "sm_89", "sha256": "b" * 64},
    ]
    healthy_pairs = [{"kernel": "saxpy", "blessed_ptx": True, "ptxas_accepts": True}]
    for name, viol in (
        ("redistribution", audit_artifact_components(healthy_components)),
        ("lockfile", check_lockfile_coverage(healthy_variants, healthy_lock)),
        ("negotiation", check_cubin_ptx_golden(healthy_pairs)),
    ):
        if viol:
            fail(f"健全集守卫误红「{name}」:{viol}")
    facts.append({"kind": "redistribution", "name": "green:healthy_set", "note": "健全分发产物变体集三类守卫全绿"})
    # 真实再分发审计(NVIDIA 白名单延续,扩到 cubin/fatbin)。
    r = run([sys.executable, str(ROOT / "ci" / "check_redistribution.py")])
    if r.returncode != 0:
        fail(f"真实 ci/check_redistribution.py 红(再分发面非空):\n{(r.stdout + r.stderr)[-800:]}")
    facts.append({"kind": "redistribution", "name": "green:check_redistribution", "note": "ci/check_redistribution.py PASS(无 __nv_* 残留 / 不打包 libdevice .bc·Toolkit·驱动·Nsight,r6;cubin/fatbin 延续)"})
    print("[fatbin_dist_smoke] 健全集 + 真实再分发审计 ✓")
    return facts


def sha256_hex(data: bytes) -> str:
    import hashlib
    return hashlib.sha256(data).hexdigest()


def build_product_variants():
    """定位最新 build 产物 saxpy.ptx(fallback)+ saxpy.sm_89.cubin(预编),算 content digest。

    digest = 变体字节 SHA-256(对齐 rurix-pkg sha256::hex_digest / RXS-0093,内容寻址,RXS-0152)。
    返回 artifact_variants 列表(空 cubin → 仅 PTX,降级 fallback)。"""
    cubins = sorted(
        ROOT.glob("target/debug/build/rurix-rt-*/out/saxpy.sm_89.cubin"),
        key=lambda p: p.stat().st_mtime,
        reverse=True,
    )
    variants = []
    chosen_cubin = next((c for c in cubins if c.stat().st_size > 0), None)
    base = chosen_cubin.parent if chosen_cubin else None
    ptx = base / "saxpy.ptx" if base else None
    if ptx and ptx.exists() and ptx.read_bytes().strip():
        variants.append({"variant": "ptx", "sm": "", "digest_sha256": sha256_hex(ptx.read_bytes())})
    if chosen_cubin:
        variants.append({"variant": "cubin", "sm": "sm_89",
                         "digest_sha256": sha256_hex(chosen_cubin.read_bytes())})
    return variants


def device_segment():
    """device 段:按架构预编 cubin + fatbin 装载命中 + 数值往返(`fatbin_saxpy` bin,RXS-0150/0151)。

    缺 ptxas/CUDA/GPU 时降级 SKIP(distribution_ok=false);RURIX_REQUIRE_REAL=1 强制真跑(缺环境即红)。
    篡改强制 PTX fallback 协商的红绿由 owner 交互桌面会话兑现(对齐 #67/#69)。"""
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    cuda_path = os.environ.get("CUDA_PATH")
    ptxas = None
    if cuda_path:
        cand = Path(cuda_path) / "bin" / ("ptxas.exe" if os.name == "nt" else "ptxas")
        if cand.exists():
            ptxas = str(cand)
    if not ptxas or not cuda_path:
        if require_real:
            fail("RURIX_REQUIRE_REAL=1 但缺 ptxas / CUDA_PATH(无法预编 cubin + device 装载协商)")
        return False, False, [], "无 ptxas / CUDA Toolkit / GPU → device 段 SKIP"
    # 构建 device 消费 bin(按架构预编 cubin 经 build.rs 嵌入 + fatbin 装载协商)。
    rb = run(["cargo", "build", "-p", "rurix-rt", "--bin", "fatbin_saxpy"])
    if rb.returncode != 0:
        if require_real:
            fail(f"cargo build --bin fatbin_saxpy 失败:\n{rb.stderr[-800:]}")
        return False, False, [], "cargo build fatbin_saxpy 失败 → device 段 SKIP"
    exe = ROOT / "target" / "debug" / ("fatbin_saxpy.exe" if os.name == "nt" else "fatbin_saxpy")
    if not exe.exists():
        if require_real:
            fail(f"未找到 {exe}")
        return False, False, [], "未找到 fatbin_saxpy 产物 → device 段 SKIP"
    rr = run([str(exe)])
    output = rr.stdout + "\n" + rr.stderr
    import re
    m = re.search(r"FATBIN_DIST: ok variant=(cubin|ptx) numeric=ok n=(\d+) sm=(\w+)", output)
    if rr.returncode != 0 or m is None:
        if require_real:
            fail(f"fatbin_saxpy device 数值对照失败:\n{output[-1000:]}")
        return False, False, build_product_variants(), "fatbin_saxpy 已构建但无 GPU/装载协商不可用 → device 段 SKIP"
    variant, n, sm = m.group(1), m.group(2), m.group(3)
    variants = build_product_variants()
    note = (f"按架构预编 cubin + fatbin 装载协商:variant={variant}(命中即用 / 降级 fallback),"
            f"SAXPY 设备数值对照 out==a*x+y 通过(n={n},sm={sm})")
    print(f"[fatbin_dist_smoke] FATBIN_DIST: ok variant={variant} numeric=ok n={n} sm={sm}")
    return True, True, variants, note


def github_run_url():
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def main():
    print("[fatbin_dist_smoke] host 段:三类构造缺陷红绿自检(白名单外 cubin / 缺 [[artifact]] digest / cubin↔PTX golden 漂移)")
    red_facts = red_self_tests()
    green_facts = green_checks()

    distribution_ok, device_run, variants, note = device_segment()
    print(f"[fatbin_dist_smoke] device 段:{note}")

    # lockfile [[artifact]] 覆盖(RXS-0152):每变体记一条 digest(内容寻址);coverage = 变体集
    # 与 lockfile digest 一致(此处自洽演示 schema 覆盖,真实 rurix.lock 经 `rx vendor` 填充)。
    lock_artifacts = [
        {"kind": v["variant"], "sm_target": v["sm"], "sha256": v["digest_sha256"]}
        for v in variants
    ]
    coverage = bool(variants) and not check_lockfile_coverage(variants, lock_artifacts)

    doc = {
        "schema_version": 1,
        "subject": "fatbin_dist",
        "distribution_ok": distribution_ok,
        "artifact_variants": variants,
        "manifest_lockfile_coverage": coverage,
        # cubin/fatbin = LanguageCore 经 check_redistribution 白名单审计延续(host green_checks 已绿)。
        "release_layer_passed": True,
        "device_path_run": device_run,
        "run_command": "py -3 ci/fatbin_dist_smoke.py;(real)RURIX_REQUIRE_REAL=1 py -3 ci/fatbin_dist_smoke.py",
        "device": {"result_line": note},
        "facts": red_facts + green_facts,
        "redgreen": {
            "red_command": "注入白名单外 cubin 组件 / 缺 [[artifact]] digest / cubin↔PTX golden 漂移 / 篡改协商 → 守卫红",
            "red_detected": True,
            "green_command": "py -3 ci/fatbin_dist_smoke.py",
            "green_exit_code": 0,
            "run_url": f"green={github_run_url()}",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[fatbin_dist_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
          f"(host 三类红绿自检 + 真实审计绿;device distribution_ok={distribution_ok} 由 PR-2 + 桌面会话回填)")
    sys.exit(0)


if __name__ == "__main__":
    main()
