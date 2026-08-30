#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""G37 W5:SDK bundle 一键幂等重打(候选打包链;禁 cargo/禁 GPU 面)。

与全门 ci/g31_sdk_dist_smoke.py 的分工:
- 全门 = 从源重建全链验证(cargo 构建 + rurixc emit dll + SPV 现编 + MSVC 离线
  可建 + GPU canonical 真跑),W6 终验消费;
- 本脚本 = **已就绪工件**的纯打包/校验面(零 cargo、零 GPU、零 MSVC):锚定输入
  13 件(sha256 硬核对,fail-closed)+ 树内活面 11 件 → 24 组件 staging →
  `rurixup release` ×2(确定性断言)→ digest 一比一闭环 → SBOM 双视图覆盖断言
  → 签名清单断言(selftest 声明面,见下)→ `rurixup install --from-dir` 四级校验
  物化 + 幂等再装(纯文件面,无 GPU)→ 产物落 dist/sdk_bundle/<ver>/。
  W4 验收若触发代码重建,更新 inputs 快照与锚后重跑本脚本即为重打终版。

组件闭集(24)/版号(sdk-1.1.0)/license 字面均 import 自升级后门脚本——单一事实源,
两脚本闭集不漂。

签名机制现状(如实):生产后端 Azure Artifact Signing 经 CI secret + 人工门控,
本机不可达;本脚本按门先例回填 selftest(self-signed-test)声明 → signing_manifest
是「声明性签名清单 + 组件 content digest」,分发完整性信任根 = SHA256SUMS/四级
内容寻址,非生产 Authenticode。CANDIDATE_MANIFEST.json 登记此降级。

用法:
  py -3 ci/g37_sdk_bundle_repack.py --selftest
  py -3 ci/g37_sdk_bundle_repack.py                 # 候选打包 → dist/sdk_bundle/sdk-1.1.0/
  py -3 ci/g37_sdk_bundle_repack.py --status final  # W6 终验后重打(仅登记字段变化)
"""
from __future__ import annotations

import argparse
import datetime as _dt
import hashlib
import io
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "ci"))

# 单一事实源:组件闭集/版号/license 字面/许可件路径随门脚本(import 无副作用面)。
import g31_sdk_dist_smoke as gate  # noqa: E402

TAG = "g37_sdk_repack"
RURIXUP = ROOT / "target" / "debug" / ("rurixup.exe" if os.name == "nt" else "rurixup")
INPUTS = ROOT / "artifacts" / "day_0830_delivery" / "w5_commercial" / "bundle" / "inputs"
ANCHORS_JSON = INPUTS / "INPUT_ANCHORS.json"
WORK = ROOT / ".tmp" / "g37_w5_repack"
STAGE = WORK / "staging"
OUT_DEFAULT = ROOT / "dist" / "sdk_bundle" / gate.SDK_VERSION

# 锚定输入 13 件(runtime 5 + canonical SPV 4 + G37 SPV 4):自 inputs 快照取件,
# sha256 == INPUT_ANCHORS.json 硬核对。其余 11 件 = 树内活面(契约/示例/
# API_VERSIONING/文档 4/许可 4),树是事实源。
ANCHORED = [
    "rurix_renderer.dll", "rurix_renderer.lib", "rurix_renderer.h",
    "rurix_renderer_sdk.dll", "rurix_renderer_sdk.lib",
    "g14_3_direct_gi.spv", "g14_mv.spv", "g14_8_tsr_resample.spv", "g14_8_tsr_resolve.spv",
    "g31_realism_transp.spv", "g31_realism_ris.spv",
    "g31_display_encode_lut.spv", "g34_unified_primary_skin.spv",
]
TREE_SOURCES = {
    gate.CONTRACT.name: gate.CONTRACT,
    gate.HOST_CPP.name: gate.HOST_CPP,
    gate.VERSIONING_MD.name: gate.VERSIONING_MD,
    **{n: p for n, p in gate.DOCS.items()},
    **{n: p for n, (p, _lic) in gate.LICENSE_COMPONENTS.items()},
}
RELEASE_FILES = ["bundle.json", "channel_manifest.json", "signing_manifest.json",
                 "sbom.spdx.json", "sbom.cdx.json", "SHA256SUMS", "gate_decision.json"]

FAILURES: list[str] = []


def note(msg: str) -> None:
    print(f"[{TAG}] {msg}", flush=True)


def fail(msg: str) -> None:
    FAILURES.append(msg)
    print(f"[{TAG}] FAIL: {msg}", file=sys.stderr, flush=True)


def sha256_bytes(b: bytes) -> str:
    return hashlib.sha256(b).hexdigest()


# ---------------------------------------------------------------------------
# 判读器(selftest 消费面;纯函数)
# ---------------------------------------------------------------------------


def anchor_ok(data: bytes, anchor: str) -> bool:
    """锚定输入判据:字节 sha256 == 锚(64 hex)且锚形状合法。纯函数。"""
    return bool(anchor) and len(anchor) == 64 and sha256_bytes(data) == anchor


def component_partition_complete(anchored, tree, expected) -> bool:
    """组件来源二分完备判据:锚定集 ∪ 树内集 == 闭集,零交集零缺漏。纯函数。"""
    a, t, e = set(anchored), set(tree), set(expected)
    return not (a & t) and (a | t) == e


def parse_sums(text: str) -> dict:
    """SHA256SUMS 行解析(digest 两空格 name;EA1 干名字典序确定性)。纯函数。"""
    rows = {}
    for ln in (text or "").splitlines():
        if ln:
            d, n = ln.split("  ", 1)
            rows[n] = d
    return rows


def release_deterministic(dir_a: Path, dir_b: Path) -> bool:
    """同源两次 release 七产物逐字节一致。"""
    return all((dir_a / n).read_bytes() == (dir_b / n).read_bytes() for n in RELEASE_FILES)


def signing_selftest_declared(doc: dict, signed_dlls) -> bool:
    """签名清单判据:两 DLL Valid+timestamped+verified,backend=self-signed-test,
    upload_permitted=true(声明性 selftest 面,非生产 Authenticode)。纯函数。"""
    by_name = {a.get("name"): a for a in doc.get("artifacts", [])}
    for n in signed_dlls:
        a = by_name.get(n)
        if not (a and a.get("status") == "Valid" and a.get("timestamped") is True
                and a.get("verified") is True and a.get("backend") == "self-signed-test"):
            return False
    return doc.get("upload_permitted") is True


# ---------------------------------------------------------------------------
# 打包链
# ---------------------------------------------------------------------------


def run(cmd: list[str], env: dict | None = None) -> subprocess.CompletedProcess:
    note("$ " + " ".join(str(c) for c in cmd[:6]) + (" …" if len(cmd) > 6 else ""))
    return subprocess.run(cmd, cwd=str(ROOT), capture_output=True, text=True,
                          timeout=600, env=env)


def assemble_staging(anchors: dict) -> bool:
    if STAGE.is_dir():
        shutil.rmtree(STAGE)
    STAGE.mkdir(parents=True)
    for name in ANCHORED:
        src = INPUTS / name
        if not src.is_file():
            fail(f"锚定输入缺件 {name}: {src}(重建后须重新快照 inputs/ 并更新 INPUT_ANCHORS.json)")
            return False
        data = src.read_bytes()
        if not anchor_ok(data, anchors.get(name, "")):
            fail(f"锚定输入 {name} sha256 失配(实 {sha256_bytes(data)[:16]}… ≠ 锚 "
                 f"{anchors.get(name, '')[:16]}…)——fail-closed 拒打包不冒充")
            return False
        (STAGE / name).write_bytes(data)
    for name, p in TREE_SOURCES.items():
        if not p.is_file():
            fail(f"树内组件缺件 {name}: {p}")
            return False
        (STAGE / name).write_bytes(p.read_bytes())
    staged = sorted(q.name for q in STAGE.iterdir())
    if staged != gate.EXPECTED_COMPONENTS:
        fail(f"staging 闭集不符({len(staged)} 件): {staged}")
        return False
    note(f"staging 24 组件齐(锚定 {len(ANCHORED)} + 树内 {len(TREE_SOURCES)})")
    return True


def do_release(out_dir: Path) -> subprocess.CompletedProcess:
    cmd = [str(RURIXUP), "release", "--version", gate.SDK_VERSION,
           "--channel", gate.SDK_CHANNEL, "--out-dir", str(out_dir)]
    for name in gate.EXPECTED_COMPONENTS:
        cmd += ["--component",
                f"{name}|{gate.SDK_VERSION}|{gate.component_license(name)}|core|{STAGE / name}"]
    for dll in gate.SIGNED_DLLS:
        cmd += ["--sign", f"{dll}|Valid|true|selftest"]
    return run(cmd)


def install_env(home: Path) -> dict:
    env = dict(os.environ)
    env["RURIX_HOME"] = str(home)
    env.pop("RURIXUP_TEST_ALLOW_LOOPBACK_HTTP", None)
    return env


def run_repack(out_dir: Path, status: str) -> int:
    if not RURIXUP.is_file():
        fail(f"rurixup 二进制缺失 {RURIXUP}(纪律禁 cargo build——须已就绪)")
        return 1
    if not ANCHORS_JSON.is_file():
        fail(f"输入锚表缺失 {ANCHORS_JSON}")
        return 1
    anchors = json.loads(ANCHORS_JSON.read_text(encoding="utf-8"))
    WORK.mkdir(parents=True, exist_ok=True)

    # ── staging 24 组件(锚定 13 + 树内 11)──
    if not component_partition_complete(ANCHORED, TREE_SOURCES, gate.EXPECTED_COMPONENTS):
        fail("组件来源二分不完备(锚定集∪树内集 ≠ 闭集)")
        return 1
    if not assemble_staging(anchors):
        return 1

    # ── release ×2(打包确定性)──
    rel1, rel2 = WORK / "rel1", WORK / "rel2"
    for d in (rel1, rel2):
        if d.is_dir():
            shutil.rmtree(d)
    r1 = do_release(rel1)
    tok1 = gate.release_tokens(r1.stdout)
    if r1.returncode != 0 or tok1.get("allow_upload") != "true":
        fail(f"release 未放行(exit={r1.returncode}): {r1.stdout[-300:]}\n{r1.stderr[-300:]}")
        return 1
    r2 = do_release(rel2)
    if r2.returncode != 0:
        fail(f"二次 release 未放行(exit={r2.returncode})")
        return 1
    det_ok = release_deterministic(rel1, rel2)
    note(f"release ×2 七产物逐字节一致 = {det_ok}")

    # ── digest 一比一闭环(staging sha == bundle.json == SHA256SUMS)──
    bundle = json.loads((rel1 / "bundle.json").read_text(encoding="utf-8"))
    bundle_digests = {c["name"]: c["sha256"] for c in bundle["components"]}
    sums_rows = parse_sums((rel1 / "SHA256SUMS").read_text(encoding="utf-8"))
    closure_ok = gate.component_set_ok(bundle_digests)
    for name in gate.EXPECTED_COMPONENTS:
        real = sha256_bytes((STAGE / name).read_bytes())
        if not (real == bundle_digests.get(name, "") == sums_rows.get(name, "")):
            closure_ok = False
            fail(f"digest 闭环破缺 {name}")
    note(f"digest 一比一闭环(24 组件) = {closure_ok}")

    # ── SBOM 双视图覆盖 ──
    spdx = (rel1 / "sbom.spdx.json").read_text(encoding="utf-8")
    cdx = (rel1 / "sbom.cdx.json").read_text(encoding="utf-8")
    sbom_ok = (gate.sbom_covers(spdx, gate.EXPECTED_COMPONENTS, gate.SDK_VERSION)
               and gate.sbom_covers(cdx, gate.EXPECTED_COMPONENTS, gate.SDK_VERSION))
    note(f"SBOM SPDX+CycloneDX 覆盖 24 组件 + {gate.SDK_VERSION} = {sbom_ok}")

    # ── 签名清单(selftest 声明面)──
    signing = json.loads((rel1 / "signing_manifest.json").read_text(encoding="utf-8"))
    sign_ok = signing_selftest_declared(signing, gate.SIGNED_DLLS)
    note(f"签名清单两 DLL selftest 声明 + upload_permitted = {sign_ok}"
         "(生产 Authenticode 后端不可达,降级登记见 CANDIDATE_MANIFEST)")

    # ── bundle.json / channel_manifest.json 入 staging(分发/from-dir 布局)──
    for n in ("bundle.json", "channel_manifest.json"):
        (STAGE / n).write_bytes((rel1 / n).read_bytes())

    # ── install 验证腿(纯文件面:四级校验物化 + 布局 + 逐字节 + 幂等)──
    home = WORK / "home"
    if home.is_dir():
        shutil.rmtree(home)
    reg = home / "toolchains.json"
    ri = run([str(RURIXUP), "install", "--from-dir", str(STAGE),
              "--registry", str(reg)], env=install_env(home))
    s = gate.tokens(ri.stdout, "RURIXUP_INSTALL:")
    tdir = home / "toolchains" / gate.SDK_VERSION
    rel_map = gate.expected_rel_paths(gate.EXPECTED_COMPONENTS)
    layout_ok = tdir.is_dir() and all((tdir / rel).is_file() for rel in rel_map.values())
    byte_ok = layout_ok and all(
        (tdir / rel).read_bytes() == (STAGE / name).read_bytes()
        for name, rel in rel_map.items())
    first_reg = reg.read_bytes() if reg.is_file() else b""
    ri2 = run([str(RURIXUP), "install", "--from-dir", str(STAGE),
               "--registry", str(reg)], env=install_env(home))
    s2 = gate.tokens(ri2.stdout, "RURIXUP_INSTALL:")
    idem_ok = (ri2.returncode == 0 and s2.get("registered") == "1"
               and reg.is_file() and reg.read_bytes() == first_reg)
    install_ok = (ri.returncode == 0 and s.get("components") == "24"
                  and s.get("digest_levels_verified") == "4" and layout_ok and byte_ok
                  and idem_ok)
    note(f"install --from-dir components={s.get('components')} digest_levels="
         f"{s.get('digest_levels_verified')} 布局={layout_ok} 逐字节={byte_ok} 幂等={idem_ok}")

    all_ok = det_ok and closure_ok and sbom_ok and sign_ok and install_ok and not FAILURES
    if not all_ok:
        fail("打包链断言未全绿,不落 dist 产物(fail-closed)")
        return 1

    # ── 产物落盘(分发布局 = 24 组件平铺 + release 七产物;幂等覆盖写)──
    if out_dir.is_dir():
        shutil.rmtree(out_dir)
    out_dir.mkdir(parents=True)
    for q in sorted(STAGE.iterdir()):
        (out_dir / q.name).write_bytes(q.read_bytes())
    for n in RELEASE_FILES:
        if n not in ("bundle.json", "channel_manifest.json"):  # 已随 staging 平铺
            (out_dir / n).write_bytes((rel1 / n).read_bytes())

    files = sorted(p for p in out_dir.iterdir() if p.is_file() and p.name != "CANDIDATE_MANIFEST.json")
    total = sum(p.stat().st_size for p in files)
    ts = _dt.datetime.now(_dt.timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    manifest = {
        "schema": "rurix.g37.sdk_bundle_candidate.v1",
        "status": status,
        "version": gate.SDK_VERSION,
        "channel": gate.SDK_CHANNEL,
        "component_count": len(gate.EXPECTED_COMPONENTS),
        "total_bytes": total,
        "file_count": len(files),
        "timestamp": ts,
        "inputs": {
            "anchored": {n: anchors[n] for n in ANCHORED},
            "tree": {n: str(p.relative_to(ROOT)) for n, p in TREE_SOURCES.items()},
        },
        "signing_degradation": (
            "生产 Authenticode(Azure Artifact Signing)经 CI secret + 人工门控,本机"
            "不可达;signing_manifest.json 为 self-signed-test 声明面 + 组件 content "
            "digest。分发完整性信任根 = SHA256SUMS + rurixup 四级内容寻址(级①锚 "
            "channel_manifest digest ②bundle digest ③树 digest ④逐文件 sha256)。"
        ),
        "rebuild_note": (
            "W4/W6 若触发代码重建:重收割 runtime/SPV 工件 → 更新 "
            "artifacts/day_0830_delivery/w5_commercial/bundle/inputs/ 快照与 "
            "INPUT_ANCHORS.json → 重跑本脚本(--status final)即重打终版;"
            "全链验证(MSVC 离线可建 + GPU canonical 真跑)走升级后 "
            "ci/g31_sdk_dist_smoke.py --gate g31.g37w5.dist。"
        ),
        "assertions": {
            "release_deterministic_x2": det_ok,
            "digest_closure_24": closure_ok,
            "sbom_dual_covers": sbom_ok,
            "signing_selftest_declared": sign_ok,
            "install_from_dir_verified": install_ok,
        },
    }
    io.open(out_dir / "CANDIDATE_MANIFEST.json", "w", encoding="utf-8", newline="\n").write(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")

    note(f"bundle 候选落盘 {out_dir.relative_to(ROOT)}({len(files)} 件 + 登记件,"
         f"共 {total:,} B ≈ {total / 1024 / 1024:.2f} MiB)")
    note("SHA256SUMS 前 6 行:")
    for ln in (out_dir / "SHA256SUMS").read_text(encoding="utf-8").splitlines()[:6]:
        note(f"  {ln}")
    note(f"REPACK {'PASS' if all_ok else 'FAIL'} status={status}")
    return 0


# ---------------------------------------------------------------------------
# selftest(纯函数红绿;零 rurixup/零文件依赖)
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

    h = sha256_bytes(b"payload")
    expect(anchor_ok(b"payload", h), "GREEN:锚核对正例")
    expect(not anchor_ok(b"payloadX", h), "RED:字节漂移必红")
    expect(not anchor_ok(b"payload", h[:32]), "RED:锚形状破缺拒判")
    expect(not anchor_ok(b"payload", ""), "RED:空锚拒判")
    expect(component_partition_complete(ANCHORED, TREE_SOURCES, gate.EXPECTED_COMPONENTS),
           "GREEN:锚定 13 + 树内 11 二分完备 == 闭集 24")
    expect(not component_partition_complete(ANCHORED[:-1], TREE_SOURCES, gate.EXPECTED_COMPONENTS),
           "RED:缺一件二分必红")
    expect(not component_partition_complete(ANCHORED + [next(iter(TREE_SOURCES))],
                                            TREE_SOURCES, gate.EXPECTED_COMPONENTS),
           "RED:交集非空必红")
    sums = parse_sums(f"{h}  a.dll\n{h}  b.spv\n")
    expect(sums == {"a.dll": h, "b.spv": h}, "GREEN:SHA256SUMS 行解析")
    expect(parse_sums("") == {}, "GREEN:空 SUMS 空表")
    good_art = {"name": "rurix_renderer.dll", "status": "Valid", "timestamped": True,
                "verified": True, "backend": "self-signed-test"}
    good2 = dict(good_art, name="rurix_renderer_sdk.dll")
    expect(signing_selftest_declared({"artifacts": [good_art, good2], "upload_permitted": True},
                                     gate.SIGNED_DLLS), "GREEN:签名声明两 DLL 正例")
    expect(not signing_selftest_declared({"artifacts": [good_art], "upload_permitted": True},
                                         gate.SIGNED_DLLS), "RED:缺一 DLL 必红")
    expect(not signing_selftest_declared(
        {"artifacts": [good_art, dict(good2, status="Unsigned")], "upload_permitted": True},
        gate.SIGNED_DLLS), "RED:Unsigned 必红")
    expect(not signing_selftest_declared(
        {"artifacts": [good_art, good2], "upload_permitted": False},
        gate.SIGNED_DLLS), "RED:upload_permitted=false 必红")
    expect(len(gate.EXPECTED_COMPONENTS) == 24, "组件闭集 24(门脚本单一事实源)")
    expect(gate.SDK_VERSION == "sdk-1.1.0", "版号 sdk-1.1.0(门脚本单一事实源)")
    if failures:
        print(f"[{TAG}] selftest FAIL ({failures})", file=sys.stderr)
        return 1
    print(f"[{TAG}] selftest PASS(锚核对/二分完备/SUMS/签名声明 红绿闭合)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--selftest", action="store_true")
    ap.add_argument("--out-dir", default=str(OUT_DEFAULT))
    ap.add_argument("--status", default="candidate", choices=["candidate", "final"])
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    return run_repack(Path(args.out_dir), args.status)


if __name__ == "__main__":
    sys.exit(main())
