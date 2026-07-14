#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""stable channel 清单冒烟(V1 CI_GATES §2 步骤 50,契约 G-V1-3,Mini-RFC/MR-0008)。

机器复核闸门(反 YAML-only),spec/release.md RXS-0185 ~ RXS-0186:

  green —— `rurixup release`(缺省 channel=stable)产出 `channel_manifest.json`:
      channel=stable / rurix_version == workspace 版号 / `bundle_manifest_sha256` ==
      同目录 bundle.json 字节流 SHA-256(内容寻址引用)/ components 与 bundle 组件
      全集一一对应(干名字典序);**两次生成逐字节一致**(确定性,无时间戳)。

  red→绿闭合 ——
      ① 漂移注入 `--simulate-channel-drift` → Release 层第 8 子门 channel-manifest
        红 → 发布阻断(退出码 2 且 failed_gates 含 channel-manifest);放行即本脚本红。
      ② 未知 channel `--channel nightly` → 工具层用法错误(退出码 1,零新 RX 码);
        受理即本脚本红。
      ③ 复原正常路径 → 绿(红绿闭合)。

纯 host/CPU-only,无 device;不 SKIP 充绿(仅无 cargo 工具链时降级 SKIP,对齐
release_pipeline_smoke 先例)。不写 evidence(无新 budget counter,对齐步骤 49)。
退出码:0=绿;非零=红。
"""
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"
OUT = ROOT / "target" / "channel_manifest_smoke"


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[channel_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[channel_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def workspace_version() -> str:
    """从 Cargo.toml [workspace.package] 解析统一版号(stdlib-only,无 toml 依赖)。"""
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r"^\[workspace\.package\]([\s\S]*?)(?=^\[|\Z)", text, re.MULTILINE)
    if not m:
        fail("Cargo.toml 未找到 [workspace.package] 段")
    v = re.search(r'^version\s*=\s*"([^"]+)"', m.group(1), re.MULTILINE)
    if not v:
        fail("Cargo.toml [workspace.package] 未找到 version")
    return v.group(1)


def build_rurixup():
    r = run(["cargo", "build", "-q", "-p", "rurixup"])
    if r.returncode != 0:
        skip(f"cargo build -p rurixup 失败(无工具链?):\n{r.stderr[-800:]}")
    exe = TARGET / "rurixup.exe"
    if not exe.exists():
        exe = TARGET / "rurixup"  # 非 Windows 兜底
    if not exe.exists():
        skip(f"未找到 rurixup 可执行({TARGET})")
    return exe


def release(exe, version, art_path, out_dir, extra=None):
    """调用 rurixup release(单语言本体组件 + Valid 签名事实);返回 (code, summary, r)。"""
    cmd = [
        str(exe), "release", "--version", version,
        "--component", f"rurixup.exe|{version}|Apache-2.0|core|{art_path}",
        "--sign", "rurixup.exe|Valid|true|selftest",
        "--out-dir", str(out_dir),
    ] + (extra or [])
    r = run(cmd)
    summary = {}
    for ln in (r.stdout or "").splitlines():
        if ln.startswith("RURIXUP_RELEASE:"):
            for tok in ln[len("RURIXUP_RELEASE:"):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    summary[k] = v
    return r.returncode, summary, r


def main():
    exe = build_rurixup()
    wv = workspace_version()

    # —— green:channel 清单产出 + 字段语义 + 内容寻址引用 + 组件对应(RXS-0185/0186)——
    code, summary, r = release(exe, wv, exe, OUT / "green")
    if code != 0:
        fail(f"green 路径未放行(exit={code}):{summary}\n{r.stdout[-300:]}\n{r.stderr[-300:]}")
    cm_path = OUT / "green" / "channel_manifest.json"
    if not cm_path.is_file():
        fail("green 未产出 channel_manifest.json(RXS-0185 存在性)")
    cm = json.loads(cm_path.read_text(encoding="utf-8"))
    if cm.get("channel") != "stable":
        fail(f"channel != stable(得 {cm.get('channel')!r})")
    if cm.get("rurix_version") != wv:
        fail(f"channel 清单版号 {cm.get('rurix_version')!r} != workspace 版号 {wv!r}(RXS-0186)")
    bundle_bytes = (OUT / "green" / "bundle.json").read_bytes()
    want_digest = hashlib.sha256(bundle_bytes).hexdigest()
    if cm.get("bundle_manifest_sha256") != want_digest:
        fail(f"bundle_manifest_sha256 失配(内容寻址引用,RXS-0185):清单 "
             f"{cm.get('bundle_manifest_sha256')} != 实测 {want_digest}")
    bundle = json.loads(bundle_bytes.decode("utf-8"))
    bundle_names = sorted(c["name"] for c in bundle["components"])
    cm_names = [c["name"] for c in cm.get("components", [])]
    if cm_names != bundle_names:
        fail(f"components 与 bundle 组件全集不对应(RXS-0186):{cm_names} != {bundle_names}")
    if summary.get("channel") != "stable" or summary.get("channel_ok") != "true":
        fail(f"摘要行 channel token 缺失/不符:{summary}")
    print(f"[channel_smoke] green ✓ channel=stable rurix_version={wv} "
          f"digest={want_digest[:12]}… components={cm_names}")

    # —— 确定性:同一输入两次生成逐字节一致(RXS-0185,无时间戳)——
    code2, _, _ = release(exe, wv, exe, OUT / "green2")
    if code2 != 0:
        fail(f"确定性复跑未放行(exit={code2})")
    if (OUT / "green2" / "channel_manifest.json").read_bytes() != cm_path.read_bytes():
        fail("两次生成 channel_manifest.json 字节不一致(确定性破坏,RXS-0185)")
    print("[channel_smoke] 确定性 ✓ 两次生成逐字节一致")

    # —— red ①:漂移注入 → 第 8 子门 channel-manifest 红 → 发布阻断(RXS-0186)——
    code, summary, r = release(exe, wv, exe, OUT / "red", ["--simulate-channel-drift"])
    if not (code == 2 and summary.get("allow_upload") == "false"
            and "channel-manifest" in summary.get("failed_gates", "")):
        fail(f"漂移注入未阻断(应 exit 2 + failed_gates 含 channel-manifest,得 exit={code} "
             f"summary={summary})——发布门失效(反 YAML-only 红)")
    print(f"[channel_smoke] red ✓ 漂移注入 → 发布门阻断(failed_gates={summary.get('failed_gates')})")

    # —— red ②:未知 channel → 工具层用法错误 exit 1(RXS-0185,零新 RX 码)——
    code, _, r = release(exe, wv, exe, OUT / "red2", ["--channel", "nightly"])
    if code != 1:
        fail(f"未知 channel `nightly` 未拒(应 exit 1,得 exit={code})——合法集守卫失效")
    print("[channel_smoke] red ✓ 未知 channel nightly → 用法错误拒(exit 1)")

    # —— 复原绿(红绿闭合)——
    code, summary, _ = release(exe, wv, exe, OUT / "restore")
    if code != 0 or summary.get("channel_ok") != "true":
        fail(f"复原绿失败(exit={code} summary={summary})")
    print("[channel_smoke] PASS 复原绿(红绿闭合;green + 确定性 + 漂移红 + 未知 channel 红)")
    sys.exit(0)


if __name__ == "__main__":
    main()
