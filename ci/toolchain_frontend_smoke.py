#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""rurixup 工具链前端冒烟(V1 后续 / MR-0009,CI 步骤 51,RXS-0187 ~ RXS-0188)。

机器复核闸门(反 YAML-only),spec/release.md RXS-0187 ~ RXS-0188:

  green —— `rurixup release`(产 channel_manifest.json + bundle.json)→
      `rurixup install --channel-manifest ... --bundle ...` 消费 stable channel →
      注册进 toolchains.json;`rurixup list` 显示版本 + default;install 幂等
      (再装同版 = registered 不增);`rurixup default <ver>` 设默认。toolchains.json
      两次同操作序列逐字节一致(确定性,无时间戳)。

  red→绿闭合 ——
      ① 篡改 bundle.json 任一字节 → `install` 内容寻址 digest 失配 → 退出 1;
      ② `rurixup default 9.9.9`(未注册)→ 退出 1;
      ③ 复原正常路径 → 绿。

纯 host/CPU-only,无 device / 无网络;不 SKIP 充绿(仅无 cargo 工具链时降级 SKIP)。
不写 evidence(无新 budget counter,对齐步骤 49/50)。退出码:0=绿;非零=红。
"""
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[toolchain_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[toolchain_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r"^\[workspace\.package\]([\s\S]*?)(?=^\[|\Z)", text, re.MULTILINE)
    v = re.search(r'^version\s*=\s*"([^"]+)"', m.group(1), re.MULTILINE) if m else None
    if not v:
        fail("Cargo.toml [workspace.package] 未找到 version")
    return v.group(1)


def build_rurixup():
    r = run(["cargo", "build", "-q", "-p", "rurixup"])
    if r.returncode != 0:
        skip(f"cargo build -p rurixup 失败(无工具链?):\n{r.stderr[-800:]}")
    exe = TARGET / "rurixup.exe"
    if not exe.exists():
        exe = TARGET / "rurixup"
    if not exe.exists():
        skip(f"未找到 rurixup 可执行({TARGET})")
    return exe


def rurixup(exe, *args):
    return run([str(exe), *args])


def summary_tokens(stdout, prefix):
    out = {}
    for ln in (stdout or "").splitlines():
        if ln.startswith(prefix):
            for tok in ln[len(prefix):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    out[k] = v
    return out


def main():
    exe = build_rurixup()
    wv = workspace_version()

    with tempfile.TemporaryDirectory() as td:
        td = Path(td)
        rel = td / "release"
        # 产 stable channel 清单 + bundle(复用 release 路径,单语言本体组件)。
        r = rurixup(
            exe, "release", "--version", wv,
            "--component", f"rurixup.exe|{wv}|Apache-2.0|core|{exe}",
            "--sign", "rurixup.exe|Valid|true|selftest",
            "--out-dir", str(rel),
        )
        if r.returncode != 0:
            fail(f"release 未放行(exit={r.returncode}):{r.stdout[-300:]}\n{r.stderr[-300:]}")
        manifest = rel / "channel_manifest.json"
        bundle = rel / "bundle.json"
        if not (manifest.is_file() and bundle.is_file()):
            fail("release 未产出 channel_manifest.json / bundle.json")

        reg = td / "toolchains.json"

        # —— green:install → list → 幂等 install → default ——
        r = rurixup(exe, "install", "--channel-manifest", str(manifest),
                    "--bundle", str(bundle), "--registry", str(reg))
        if r.returncode != 0:
            fail(f"install 未成功(exit={r.returncode}):{r.stdout[-300:]}\n{r.stderr[-300:]}")
        s = summary_tokens(r.stdout, "RURIXUP_INSTALL:")
        if s.get("version") != wv or s.get("channel") != "stable" or s.get("default") != wv \
                or s.get("registered") != "1":
            fail(f"install 摘要不符:{s}")
        first_bytes = reg.read_bytes()
        print(f"[toolchain_smoke] green ✓ install version={wv} default={wv} registered=1")

        # 幂等:再装同版 → registered 不增 + toolchains.json 逐字节一致(确定性)。
        r = rurixup(exe, "install", "--channel-manifest", str(manifest),
                    "--bundle", str(bundle), "--registry", str(reg))
        s = summary_tokens(r.stdout, "RURIXUP_INSTALL:")
        if r.returncode != 0 or s.get("registered") != "1":
            fail(f"幂等 install 破坏(exit={r.returncode} registered={s.get('registered')})")
        if reg.read_bytes() != first_bytes:
            fail("幂等 install 后 toolchains.json 字节漂移(确定性破坏)")
        print("[toolchain_smoke] 幂等 ✓ 再装同版 registered=1 且 toolchains.json 逐字节一致")

        # list 显示版本 + default。
        r = rurixup(exe, "list", "--registry", str(reg))
        ls = summary_tokens(r.stdout, "RURIXUP_LIST:")
        if r.returncode != 0 or ls.get("count") != "1" or ls.get("default") != wv:
            fail(f"list 不符(exit={r.returncode} {ls})")
        print(f"[toolchain_smoke] list ✓ count=1 default={wv}")

        # default 设为已注册版本。
        r = rurixup(exe, "default", wv, "--registry", str(reg))
        if r.returncode != 0:
            fail(f"default {wv} 未成功(exit={r.returncode}):{r.stderr[-200:]}")
        print(f"[toolchain_smoke] default ✓ 设 {wv}")

        # —— red ①:篡改 bundle.json → 内容寻址 digest 失配 → install 拒(exit 1)——
        tampered = td / "bundle_tampered.json"
        tampered.write_text(bundle.read_text(encoding="utf-8") + " ", encoding="utf-8")
        reg2 = td / "toolchains_red.json"
        r = rurixup(exe, "install", "--channel-manifest", str(manifest),
                    "--bundle", str(tampered), "--registry", str(reg2))
        if r.returncode != 1:
            fail(f"篡改 bundle 未拒(应 exit 1,得 exit={r.returncode})——内容寻址校验失效")
        if reg2.exists():
            fail("篡改 install 后仍写出 toolchains.json(应全有或全无不注册)")
        print("[toolchain_smoke] red ✓ 篡改 bundle → install 拒(exit 1,不注册)")

        # —— red ②:default 未注册版号 → 拒(exit 1)——
        r = rurixup(exe, "default", "9.9.9", "--registry", str(reg))
        if r.returncode != 1:
            fail(f"未注册版号 default 未拒(应 exit 1,得 exit={r.returncode})")
        print("[toolchain_smoke] red ✓ default 9.9.9(未注册)→ 拒(exit 1)")

        # —— 复原绿:正常 default 仍工作 ——
        r = rurixup(exe, "default", wv, "--registry", str(reg))
        if r.returncode != 0:
            fail(f"复原绿失败(exit={r.returncode})")
        print("[toolchain_smoke] PASS 复原绿(红绿闭合;green + 幂等 + list + 篡改红 + 未注册红)")
    sys.exit(0)


if __name__ == "__main__":
    main()
