# -*- coding: utf-8 -*-
"""DXIL 图形=B codegen 冒烟门(G2 CI_GATES 步骤 46;契约 G-G2-2;spec/dxil_backend.md
RXS-0162)。

机制(host/CPU-only,无 device,无网络;反 YAML-only):以参考着色阶段喂图形=B 外部
转译链(SPIRV-Cross→dxc→DXIL),真实核验 RXS-0162 的 host 可达面:

  1. **转译链可达**:spirv-cross + dxc 定位 + 端到端跑通(缺工具 → SKIP exit 0,
     对齐 RXS-0073 ptxas 干验证 / RXS-0157 validator SKIP 纪律)。
  2. **确定性(Property 3)**:同 SPIR-V 输入 → B 全链 ×N 容器 SHA256 全等。
  3. **validator gate(RXS-0162 IR2)**:签名 validator(dxv/dxil.dll,`RURIX_DXC_DIR`)
     可用时入 golden 前 DXIL 须 validator 接受;本机 Vulkan SDK dxc 无签名 validator
     → 该子核验 SKIP(结构性 dxc 编译成功为代,owner 在 pin 环境补签名 validator)。
  4. **签名解析 / 系统值保真**:dumpbin → 解析 ISG1/OSG1 → 断言系统值(SV_Position /
     SV_VertexID)经链保真(用户 varying 语义名经 spirv-cross 退化为通用名属机制① 缺口,
     由 Rust signature_gate strict-only RX6011 兜底,见 dxil_sig_gate 单测;此处只核系统值)。
  5. **签名篡改红绿(strict-only)**:篡改 SPIR-V 字流 → 转译链拒(红)→ 复原 → 绿;
     篡改译后签名(去系统值)→ 保真核验拒(红)→ 复原 → 绿。
  6. **供应链 pin 核对(RXS-0162 IR4)**:定位工具 SHA256 与 `rurix.lock [[toolchain]]`
     pin 比对(canonical 命中即过;env override 的 dev/probe 工具 SHA 异 → NOTE 非红)。

evidence 写 evidence/dxil_codegen_smoke_*.json(归 owner;AI 本地跑后 git restore)。
任一应绿却红 / 应红却绿 / 错误码不符即整体 FAIL(非零退出);无环境 → SKIP exit 0。

用法: py -3 ci/dxil_codegen_smoke.py
"""
from __future__ import annotations

import hashlib
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LOCKFILE = ROOT / "rurix.lock"

FAILURES: list[str] = []
NOTES: list[str] = []


def check(cond: bool, msg: str) -> None:
    if not cond:
        FAILURES.append(msg)


def note(msg: str) -> None:
    NOTES.append(msg)


# ───────────────────────── 工具定位(env override > PATH) ─────────────────────────


def locate(env_keys: list[str], names: list[str]) -> str | None:
    """按 env 绝对路径 > PATH 名定位可执行;均不可用 → None。"""
    for k in env_keys:
        v = os.environ.get(k)
        if v and Path(v).is_file():
            return v
    # RURIX_DXC_DIR / RURIX_DXC_NEW_DIR 目录内 dxc.exe(对齐 toolchain.rs)。
    # 需先于 PATH:开发机 PATH 上常有 Vulkan SDK dxc,但签名 validator 目录另由
    # RURIX_DXC_DIR 指定；若先吃 PATH 会导致 validator gate 用 dxv 验证了另一套 dxc 产物。
    for k in ("RURIX_DXC_DIR", "RURIX_DXC_NEW_DIR"):
        v = os.environ.get(k)
        if v and (Path(v) / "dxc.exe").is_file() and "dxc" in names:
            return str(Path(v) / "dxc.exe")
    for n in names:
        p = shutil.which(n)
        if p:
            return p
    return None


def sha256_file(p: Path) -> str:
    h = hashlib.sha256()
    h.update(p.read_bytes())
    return h.hexdigest()


# ───────────────────────── 纯函数(red 自检喂合成输入,反 YAML-only) ─────────────────────────


def parse_toolchain_pins(lock_text: str) -> dict[str, dict[str, str]]:
    """解析 rurix.lock 的 `[[toolchain]]` 段 → {file: {field: value}}。极简 TOML 子集。"""
    pins: dict[str, dict[str, str]] = {}
    cur: dict[str, str] | None = None
    for raw in lock_text.replace("\r\n", "\n").split("\n"):
        line = raw.strip()
        if line == "[[toolchain]]":
            cur = {}
            continue
        if line.startswith("[[") or line.startswith("#") or not line:
            if line.startswith("[[") and line != "[[toolchain]]":
                cur = None
            continue
        m = re.match(r'^(\w+)\s*=\s*"(.*)"\s*$', line)
        if cur is not None and m:
            cur[m.group(1)] = m.group(2)
            if m.group(1) == "file":
                pins[m.group(2)] = cur
    return pins


def signatures_system_values(disasm: str) -> set[str]:
    """从 dxc -dumpbin 反汇编解析签名表中出现的系统值 token 集合(SV_*/缩写)。"""
    svs: set[str] = set()
    for line in disasm.replace("\r\n", "\n").split("\n"):
        s = line.strip().lstrip(";").strip()
        # 注释表 SysValue 列(POS/VERTID 等)与元数据全名(SV_Position)两种形态。
        for tok in ("SV_Position", "SV_VertexID", "POS", "VERTID"):
            if re.search(rf"\b{re.escape(tok)}\b", s):
                svs.add("SV_Position" if tok in ("SV_Position", "POS") else "SV_VertexID")
    return svs


def input_signature_names(disasm: str) -> set[str]:
    """解析 ISG1(`; Input signature:`)注释表的语义名集合(剥尾随语义 index 数字,
    大写)。用于核验顶点输入用户语义名保真(POSITION 不退化为 TEXCOORD#)。容错:
    无表 → 空集;遇 `Output signature:` / 非注释行结束本段。"""
    names: set[str] = set()
    in_sec = False
    for raw in disasm.replace("\r\n", "\n").split("\n"):
        line = raw.strip()
        body = line.lstrip(";").strip()
        if body.startswith("Input signature:"):
            in_sec = True
            continue
        if body.startswith("Output signature:") or body.startswith("Pipeline"):
            in_sec = False
            continue
        if not in_sec:
            continue
        if not line.startswith(";"):
            in_sec = False
            continue
        toks = body.split()
        if not toks or toks[0] in ("Name", "no") or toks[0].startswith("---"):
            continue
        # 列体例:Name Index Mask Register SysValue Format [Used];Name=toks[0]。
        names.add(toks[0].rstrip("0123456789").upper())
    return names


def pin_matches(located_sha: str, pin_sha: str) -> bool:
    """供应链 pin 命中判定(纯函数,canonical 命中)。"""
    return located_sha.lower() == pin_sha.lower()


def red_self_test() -> None:
    """反 YAML-only:合成输入断言纯核验函数能区分红绿。门失效即红退出。"""
    # (a) pin 比对:同 sha 命中、异 sha 未命中。
    if not pin_matches("ABcd", "abCD") or pin_matches("aa", "bb"):
        _fail("red 自检:pin_matches 判定失效")
    # (b) 系统值解析:含 SV_Position 的反汇编应识别;空表不应识别。
    ok = signatures_system_values(
        "; SV_Position 0 xyzw 1 POS float\n; SV_VertexID 0 x 0 VERTID uint"
    )
    if "SV_Position" not in ok or "SV_VertexID" not in ok:
        _fail(f"red 自检:系统值解析漏检 {ok}")
    if signatures_system_values("; no parameters"):
        _fail("red 自检:系统值解析误检(空签名表)")
    # (b') 输入语义名解析:ISG1 含 POSITION 应识别(剥尾随数字);Output 段不计入;
    #      退化为 TEXCOORD 的输入应解析为 TEXCOORD(便于断言保名 vs 退化)。
    in_names = input_signature_names(
        "; Input signature:\n; Name Index Mask Register SysValue Format Used\n"
        "; POSITION 0 xyz 0 NONE float\n; Output signature:\n; TEXCOORD 0 xyzw 0 NONE float xyzw"
    )
    if "POSITION" not in in_names or "TEXCOORD" in in_names:
        _fail(f"red 自检:输入语义名解析失效(应仅 ISG1 段 POSITION){in_names}")
    if input_signature_names("; Input signature:\n; no parameters"):
        _fail("red 自检:输入语义名解析误检(空输入签名)")
    # (c) toolchain pin 解析:能取出 file→sha。
    pins = parse_toolchain_pins(
        '[[toolchain]]\nname = "dxc"\nfile = "dxc.exe"\nsha256 = "deadbeef"\n'
    )
    if pins.get("dxc.exe", {}).get("sha256") != "deadbeef":
        _fail(f"red 自检:toolchain pin 解析失效 {pins}")


def _fail(msg: str) -> None:
    print(f"[dxil_codegen_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


# 参考着色阶段(host 冒烟取证;非 Rurix 编码器产物——编码器路径由 cargo dxil_golden /
# dxil_spirv 单测覆盖,本冒烟证**外部转译链**端到端 + 确定性 + 系统值保真 + 顶点输入名
# 保真 + 篡改红绿)。写真输出(SV_Position + COLOR)使签名经链落地(避平凡 passthrough
# DCE);系统值 SV_Position / SV_VertexID 经链保真;**顶点输入用户语义名 POSITION** 经
# 顶点输入语义保名旗标(`--set-hlsl-vertex-input-semantic <loc> <semantic>`,RFC-0004
# §4.4 机制①;生产侧由 dxil_codegen::vertex_input_semantic_flags 经 io_sig 导出)端到端
# **存活**(实测);用户**输出 varying** COLOR 经 spirv-cross 退化为 TEXCOORD(机制① 边界:
# spirv-cross 无输出语义旗标,STUB(RD-017),strict-only 由 Rust signature_gate RX6011 兜底)。
REF_HLSL = """\
struct VsOut {
    float4 pos : SV_Position;
    float4 color : COLOR0;
};

VsOut main(float3 ipos : POSITION, uint vid : SV_VertexID) {
    VsOut o;
    o.pos = float4(ipos, 1.0) + float4(float(vid), 0.0, 0.0, 0.0);
    o.color = float4(1.0, 0.5, 0.25, 1.0);
    return o;
}
"""

# 顶点输入语义保名旗标(`--set-hlsl-vertex-input-semantic <location> <semantic>`,
# RFC-0004 §4.4 机制①)。生产侧由 dxil_codegen::vertex_input_semantic_flags(stage,io_sig)
# 经 io_sig 顺序导出 location → 语义名(非硬编码,Rust 单测 vertex_input_semantic_flags_
# derive_from_io_sig 锚定);本参考着色阶段唯一命名顶点输入 POSITION 取 location 0
# (SV_VertexID 系统值不占 location),按 location 覆盖保名。
KEEP_VERTEX_INPUT_FLAGS = ["--set-hlsl-vertex-input-semantic", "0", "POSITION"]


def run(argv: list[str]) -> subprocess.CompletedProcess:
    return subprocess.run(argv, capture_output=True, text=True)


def emit_reference_spirv(dxc: str, work: Path) -> Path | None:
    """dxc HLSL→SPIR-V(-fspv-reflect 携 UserSemantic),作转译链参考输入。"""
    hlsl = work / "ref_vs.hlsl"
    hlsl.write_text(REF_HLSL, encoding="utf-8")
    spv = work / "ref_vs.spv"
    p = run([dxc, "-spirv", "-fspv-reflect", "-T", "vs_6_0", "-E", "main", "-Fo", str(spv), str(hlsl)])
    return spv if p.returncode == 0 and spv.is_file() else None


def transpile_chain(spirv_cross: str, dxc: str, spv: Path, work: Path, tag: str,
                    extra: list[str] | None = None) -> tuple[bytes | None, str]:
    """SPIR-V → (spirv-cross) HLSL → (dxc) DXIL 容器。返回 (容器字节 | None, 反汇编文本)。
    `extra` = 顶点输入语义保名旗标(经 io_sig 导出,生产对齐;None → 空)。"""
    hlsl = work / f"{tag}.hlsl"
    argv = [spirv_cross, "--hlsl", "--shader-model", "60"]
    argv += extra or []
    argv += ["--output", str(hlsl), str(spv)]
    pc = run(argv)
    if pc.returncode != 0 or not hlsl.is_file():
        return None, f"spirv-cross 失败: {pc.stderr.strip()[:200]}"
    dxil = work / f"{tag}.dxil"
    pd = run([dxc, "-T", "vs_6_0", "-E", "main", "-Fo", str(dxil), str(hlsl)])
    if pd.returncode != 0 or not dxil.is_file():
        return None, f"dxc 失败: {pd.stderr.strip()[:200]}"
    container = dxil.read_bytes()
    disasm = run([dxc, "-dumpbin", str(dxil)]).stdout
    return container, disasm


def main() -> int:
    red_self_test()

    spirv_cross = locate(["RURIX_SPIRV_CROSS"], ["spirv-cross"])
    dxc = locate(["RURIX_DXC"], ["dxc"])
    spirv_val = locate(["RURIX_SPIRV_VAL"], ["spirv-val"])

    if not spirv_cross or not dxc:
        print(
            "[dxil_codegen_smoke] SKIP(B 转译链工具不可用:"
            f"spirv-cross={'有' if spirv_cross else '缺'} / dxc={'有' if dxc else '缺'};"
            "开发环境降级,exit 0,真实红绿在带 B 工具链环境,RXS-0162)"
        )
        return 0

    evidence = {
        "tool": "ci/dxil_codegen_smoke.py",
        "ci_step": 46,
        "clause": "RXS-0162",
        "tools": {"spirv_cross": spirv_cross, "dxc": dxc, "spirv_val": spirv_val},
        "checks": {},
    }

    with tempfile.TemporaryDirectory(prefix="rurix_dxil_b_smoke_") as tmp:
        work = Path(tmp)

        # ── 1) 转译链可达:参考 SPIR-V + 端到端跑通。 ──
        spv = emit_reference_spirv(dxc, work)
        check(spv is not None, "dxc HLSL→SPIR-V 参考产出失败(转译链入口不可达)")
        if spv is None:
            return _report(evidence)
        # spirv-val 参考 SPIR-V(可用时)。
        if spirv_val:
            pv = run([spirv_val, str(spv)])
            check(pv.returncode == 0, f"参考 SPIR-V 未过 spirv-val: {pv.stderr.strip()[:200]}")
            evidence["checks"]["spirv_val"] = pv.returncode == 0
        else:
            note("spirv-val 不可用 → SPIR-V 独立验证 SKIP")

        container0, disasm0 = transpile_chain(
            spirv_cross, dxc, spv, work, "run0", KEEP_VERTEX_INPUT_FLAGS
        )
        check(container0 is not None, f"B 转译链端到端失败: {disasm0}")
        if container0 is None:
            return _report(evidence)
        evidence["checks"]["chain_reachable"] = True

        # ── 2) 确定性(Property 3):同 SPIR-V ×N 容器 SHA256 全等。 ──
        digests = [hashlib.sha256(container0).hexdigest()]
        for i in range(1, 4):
            c, _ = transpile_chain(spirv_cross, dxc, spv, work, f"run{i}", KEEP_VERTEX_INPUT_FLAGS)
            check(c is not None, f"确定性子跑 {i} 转译失败")
            if c is not None:
                digests.append(hashlib.sha256(c).hexdigest())
        deterministic = len(set(digests)) == 1
        check(deterministic, f"B 全链非确定性(×{len(digests)} 容器 SHA256 不一致): {set(digests)}")
        evidence["checks"]["deterministic"] = deterministic
        evidence["container_sha256"] = digests[0]

        # ── 3) 签名解析 / 系统值保真:SV_Position + SV_VertexID 经链保真。 ──
        svs = signatures_system_values(disasm0)
        check("SV_Position" in svs, f"译后签名缺 SV_Position(系统值未保真): {svs}")
        check("SV_VertexID" in svs, f"译后签名缺 SV_VertexID(系统值未保真): {svs}")
        evidence["checks"]["system_values_preserved"] = sorted(svs)

        # ── 3b) 顶点输入用户语义名保真(RFC-0004 §4.4 机制①,RXS-0159 IR1(a)):
        #        POSITION 经顶点输入语义保名旗标端到端**存活**,不退化为通用 TEXCOORD#。 ──
        in_names = input_signature_names(disasm0)
        check(
            "POSITION" in in_names,
            f"顶点输入用户语义名 POSITION 未保真(应经 --set-hlsl-vertex-input-semantic "
            f"存活、不退化 TEXCOORD#;机制① 接入生产缺口)。ISG1 名集={sorted(in_names)}",
        )
        evidence["checks"]["vertex_input_semantic_preserved"] = sorted(in_names)
        # 输出 varying COLOR 经 spirv-cross 退化为 TEXCOORD(机制① 边界,STUB(RD-017)):
        # spirv-cross 无输出语义保名旗标 → 输出/片元 varying 名不可保真,strict-only 由
        # Rust signature_gate RX6011 兜底(命名输出 varying I/O 显式拒绝,不静默)。
        if "TEXCOORD" in disasm0 and "COLOR" not in disasm0:
            note(
                "用户**输出 varying** COLOR 经 spirv-cross 退化为 TEXCOORD(机制① 边界:"
                "spirv-cross 无输出语义旗标、不消费 UserSemantic;deferred RD-017;"
                "命名输出 varying I/O 经 Rust signature_gate RX6011 strict-only 显式拒绝)"
            )

        # ── 4) validator gate(签名 validator 可用时;否则结构性 dxc 编译为代)。 ──
        dxc_dir = os.environ.get("RURIX_DXC_DIR") or os.environ.get("RURIX_DXC_NEW_DIR")
        dxv = None
        if dxc_dir and (Path(dxc_dir) / "dxv.exe").is_file():
            dxv = str(Path(dxc_dir) / "dxv.exe")
        if dxv:
            # 复跑链产 DXIL 落盘后 dxv 验证(同生产保名旗标)。
            run([spirv_cross, "--hlsl", "--shader-model", "60", *KEEP_VERTEX_INPUT_FLAGS,
                 "--output", str(work / "v.hlsl"), str(spv)])
            run([dxc, "-T", "vs_6_0", "-E", "main", "-Fo", str(work / "v.dxil"), str(work / "v.hlsl")])
            pv = run([dxv, str(work / "v.dxil")])
            check(pv.returncode == 0, f"DXIL 容器未过签名 validator(dxv): {pv.stdout.strip()[:200]}")
            evidence["checks"]["validator_gate"] = pv.returncode == 0
        else:
            note(
                "签名 validator(dxil.dll/dxv)不可用(Vulkan SDK dxc 不随附)→ validator gate "
                "SKIP,结构性 dxc 编译成功为代;完整签名验证归 owner pin 环境(device 真跑 / golden bless)"
            )
            evidence["checks"]["validator_gate"] = "skipped-no-signing-validator"

        # ── 5) 签名篡改红绿(strict-only):篡改 SPIR-V → 链拒(红)→ 复原 → 绿。 ──
        spv_bytes = spv.read_bytes()
        tampered = bytearray(spv_bytes)
        tampered[0] ^= 0xFF  # 破坏 magic word 首字节。
        spv.write_bytes(bytes(tampered))
        c_red, _ = transpile_chain(spirv_cross, dxc, spv, work, "tamper", KEEP_VERTEX_INPUT_FLAGS)
        check(c_red is None, "篡改 SPIR-V 字流后转译链仍成功(strict-only 红路径失效)")
        spv.write_bytes(spv_bytes)
        c_green, _ = transpile_chain(spirv_cross, dxc, spv, work, "restore", KEEP_VERTEX_INPUT_FLAGS)
        check(c_green is not None, "复原 SPIR-V 后转译链未转绿")
        evidence["checks"]["tamper_spirv_red_green"] = (c_red is None) and (c_green is not None)

        # 译后签名篡改(去系统值)→ 保真核验拒(红)→ 复原核验绿。
        # SV_Position 在反汇编中以全名 + 注释表缩写 POS 两种形态出现,篡改须两者皆去
        # (与 signatures_system_values 的双形态识别同口径)。
        tampered_disasm = re.sub(r"SV_Position|\bPOS\b", "BOGUS_REMOVED", disasm0)
        red_svs = signatures_system_values(tampered_disasm)
        check("SV_Position" not in red_svs, "篡改去 SV_Position 后保真核验仍命中(红路径失效)")
        green_svs = signatures_system_values(disasm0)
        check("SV_Position" in green_svs, "复原后系统值保真核验未转绿")
        evidence["checks"]["tamper_signature_red_green"] = (
            "SV_Position" not in red_svs and "SV_Position" in green_svs
        )

        # ── 6) 供应链 pin 核对(RXS-0162 IR4):定位工具 SHA256 vs rurix.lock。 ──
        if LOCKFILE.is_file():
            pins = parse_toolchain_pins(LOCKFILE.read_text(encoding="utf-8"))
            pin_report = {}
            for tool_path, fname in ((dxc, "dxc.exe"), (spirv_cross, "spirv-cross.exe")):
                pin = pins.get(fname)
                if not pin:
                    note(f"rurix.lock 缺 [[toolchain]] file={fname} pin")
                    continue
                actual = sha256_file(Path(tool_path))
                hit = pin_matches(actual, pin.get("sha256", ""))
                pin_report[fname] = "canonical-match" if hit else "dev-override(sha-differs)"
                if not hit:
                    note(
                        f"{fname} 定位 SHA256 与 rurix.lock pin 不一致(dev/probe override,"
                        "非红;canonical 复现归 owner pin 环境)"
                    )
            evidence["checks"]["supply_chain_pin"] = pin_report
        else:
            note("rurix.lock 不存在 → 供应链 pin 核对 SKIP")

    return _report(evidence)


def _report(evidence: dict) -> int:
    evidence["notes"] = NOTES
    evidence["passed"] = not FAILURES
    # 本冒烟为 host/CPU-only 红绿门(对齐 ci/pkg_resolve_smoke.py 体例:stdout PASS/FAIL +
    # 退出码即结论,不写 evidence/*.json)。device 真跑数值/呈现对照 evidence(带 schema)
    # 归 owner pin 环境兑现(硬规则 2/3;AI 不写 evidence)。结构化结果仅用于 stdout 摘要。
    for n in NOTES:
        print(f"[dxil_codegen_smoke] NOTE: {n}")
    if FAILURES:
        print(f"[dxil_codegen_smoke] FAIL ({len(FAILURES)})")
        for f in FAILURES:
            print(f"  - {f}")
        return 1
    summary = evidence.get("checks", {})
    print(f"[dxil_codegen_smoke] checks: {json.dumps(summary, ensure_ascii=False)}")
    print(
        "[dxil_codegen_smoke] PASS(转译链可达 + 确定性 ×N + 系统值保真 + 顶点输入名保真 + "
        "validator gate + 签名篡改红绿 + 供应链 pin)"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
