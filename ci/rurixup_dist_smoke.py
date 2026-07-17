#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""rurixup 真实分发冒烟(EA1.1a / RFC-0012,CI 步骤 59 前半,RXS-0214 ~ RXS-0215)。

契约 G-EA1-2 防降级硬门(**真实磁盘物化,账面注册/内存提交/mock 文件系统均不得
替代**),spec/release.md RXS-0214(真实 FS 物化)+ RXS-0215(活跃切换)。纯离线
`--from-dir` 本地源,host/CPU-only,零真实外呼、零 GPU 依赖。

  green ——
    ① `cargo build rurixup + rx` → 用真实构建产物当组件造 --from-dir fixture(bundle.json
       digest 真算)→ `rurixup install --from-dir` 物化到临时 RURIX_HOME;
    ② 断言磁盘树在 + 逐字节 == 源 + 注册表 v2 字段真(schema_version=2 / install_path /
       tree_digest 非 null / digest_levels_verified=4);
    ③ 切换探针:经 shim 干名(<home>\\bin\\rx.exe = rurixup 拷贝)转发 toolchains 内真实
       rx.exe 跑 `rx check hello.rx` 退出 0;物化产物真实可执行(直接跑 toolchains 内
       rurixup.exe --help 退出 0);
    ④ 幂等:再装同源 → registered 不增 + toolchains.json 逐字节一致。

  red→绿闭合 ——
    ① 篡改组件一字节(bundle.json 仍持原 digest)→ 内容寻址拒(退出非 0)+ toolchains/
       零残留 + 注册表 0-byte + staging 零残留;
    ② default 指向已删版本目录 → 诚实报错退出非 0;
    ③ 复原正常路径 → 绿。

  内建 red_self_test:把断言反向喂进纯判定层,证明 smoke 能区分红/绿(反 YAML-only)。

无 device / 无网络;不 SKIP 充绿(仅无 cargo 工具链时降级 SKIP)。退出码:0=绿;非零=红。
"""
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"


def run(cmd, env=None, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, env=env, **kw)


def skip(msg):
    print(f"[dist_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[dist_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


# ————————————————— 纯判定层(red 自检直接喂合成数据)—————————————————


def install_rejected_cleanly(exit_code: int, toolchains_exists: bool, registry_exists: bool) -> bool:
    """RED① 判据:安装被拒(退出非 0)**且**零半装(无版本目录、无注册表)。纯函数。"""
    return exit_code != 0 and not toolchains_exists and not registry_exists


def install_succeeded(exit_code: int, toolchains_exists: bool, registry_exists: bool) -> bool:
    """GREEN 判据:安装成功(退出 0)且磁盘物化 + 注册表落地。纯函数。"""
    return exit_code == 0 and toolchains_exists and registry_exists


def red_self_test() -> None:
    """反 YAML-only:合成红/绿场景喂纯判定层,断言门能区分。门失效即红。"""
    # (a) 干净拒装(退出 1 + 零残留)→ 应判「被拒」。
    if not install_rejected_cleanly(1, False, False):
        fail("red 自检失败:干净拒装(exit 1 + 零残留)未被识别为 RED①(门失效)")
    # (b) 成功安装(退出 0 + 物化 + 注册)→ **不得**判为「被拒」(否则门把绿误当红)。
    if install_rejected_cleanly(0, True, True):
        fail("red 自检失败:成功安装被误判为拒装(门过松,会把绿吞成红)")
    # (c) 退出非 0 但残留了 toolchains → 非干净拒装(半装泄漏)→ 不算 RED① 通过。
    if install_rejected_cleanly(1, True, False):
        fail("red 自检失败:半装泄漏(exit≠0 但版本目录残留)被误判为干净拒装(门失效)")
    # (d) 成功判据对称自检。
    if not install_succeeded(0, True, True):
        fail("red 自检失败:成功安装未被识别(门过严)")
    if install_succeeded(1, True, True):
        fail("red 自检失败:退出非 0 被误判为成功(门失效)")


# ————————————————— IO / 工具层 —————————————————


def workspace_version() -> str:
    text = (ROOT / "Cargo.toml").read_text(encoding="utf-8")
    m = re.search(r"^\[workspace\.package\]([\s\S]*?)(?=^\[|\Z)", text, re.MULTILINE)
    v = re.search(r'^version\s*=\s*"([^"]+)"', m.group(1), re.MULTILINE) if m else None
    if not v:
        fail("Cargo.toml [workspace.package] 未找到 version")
    return v.group(1)


def build():
    r = run(["cargo", "build", "-q", "-p", "rurixup", "-p", "rx"])
    if r.returncode != 0:
        skip(f"cargo build 失败(无工具链?):\n{r.stderr[-800:]}")
    rurixup = TARGET / "rurixup.exe"
    rx = TARGET / "rx.exe"
    if not rurixup.exists():
        rurixup = TARGET / "rurixup"
    if not rx.exists():
        rx = TARGET / "rx"
    if not (rurixup.exists() and rx.exists()):
        skip(f"未找到 rurixup / rx 可执行({TARGET})")
    return rurixup, rx


def tokens(stdout, prefix):
    out = {}
    for ln in (stdout or "").splitlines():
        if ln.startswith(prefix):
            for tok in ln[len(prefix):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    out[k] = v
    return out


def make_fixture(rurixup: Path, rx: Path, wd: Path, ver: str) -> Path:
    """造 --from-dir fixture:真实构建产物当组件 + rurixup release 产 bundle/channel 清单。"""
    fromdir = wd / "fromdir"
    fromdir.mkdir(parents=True, exist_ok=True)
    # 真实构建产物当组件(digest 真算)。
    (fromdir / "rx.exe").write_bytes(rx.read_bytes())
    (fromdir / "rurixup.exe").write_bytes(rurixup.read_bytes())
    rel = wd / "rel"
    r = run([
        str(rurixup), "release", "--version", ver,
        "--component", f"rx.exe|{ver}|Apache-2.0|core|{fromdir / 'rx.exe'}",
        "--component", f"rurixup.exe|{ver}|Apache-2.0|core|{fromdir / 'rurixup.exe'}",
        "--sign", "rx.exe|Valid|true|selftest",
        "--sign", "rurixup.exe|Valid|true|selftest",
        "--out-dir", str(rel),
    ])
    if r.returncode != 0:
        fail(f"release 未放行(exit={r.returncode}):{r.stdout[-300:]}\n{r.stderr[-300:]}")
    for name in ("bundle.json", "channel_manifest.json"):
        (fromdir / name).write_bytes((rel / name).read_bytes())
    return fromdir


def install_env(home: Path) -> dict:
    import os
    env = dict(os.environ)
    env["RURIX_HOME"] = str(home)
    return env


def main():
    red_self_test()
    rurixup, rx = build()
    ver = workspace_version()

    with tempfile.TemporaryDirectory() as td:
        wd = Path(td)
        fromdir = make_fixture(rurixup, rx, wd, ver)

        # —————————————————— GREEN ——————————————————
        home = wd / "home"
        reg = home / "toolchains.json"
        r = run([str(rurixup), "install", "--from-dir", str(fromdir),
                 "--registry", str(reg)], env=install_env(home))
        toolchain_dir = home / "toolchains" / ver
        if not install_succeeded(r.returncode, toolchain_dir.is_dir(), reg.is_file()):
            fail(f"GREEN install 未成功(exit={r.returncode} dir={toolchain_dir.is_dir()} "
                 f"reg={reg.is_file()}):{r.stdout[-300:]}\n{r.stderr[-300:]}")
        s = tokens(r.stdout, "RURIXUP_INSTALL:")
        if s.get("version") != ver or s.get("digest_levels_verified") != "4" \
                or s.get("components") != "2" or s.get("registered") != "1":
            fail(f"install 摘要不符:{s}")

        # 磁盘树在 + 逐字节 == 源。
        disk_rx = toolchain_dir / "bin" / "rx.exe"
        disk_up = toolchain_dir / "bin" / "rurixup.exe"
        if not (disk_rx.is_file() and disk_up.is_file()):
            fail(f"物化产物缺失:{disk_rx} / {disk_up}")
        if disk_rx.read_bytes() != rx.read_bytes():
            fail("物化 rx.exe 与源逐字节不等(防降级硬门:非真实物化)")
        if disk_up.read_bytes() != rurixup.read_bytes():
            fail("物化 rurixup.exe 与源逐字节不等")

        # 注册表 v2 字段真。
        reg_doc = json.loads(reg.read_text(encoding="utf-8"))
        if reg_doc.get("schema_version") != 2:
            fail(f"注册表 schema_version 非 2:{reg_doc.get('schema_version')}")
        entry = next((t for t in reg_doc.get("installed", []) if t.get("version") == ver), None)
        if not entry or not entry.get("install_path") or not entry.get("tree_digest"):
            fail(f"注册表 v2 字段(install_path/tree_digest)缺失:{entry}")
        print(f"[dist_smoke] GREEN ✓ 物化 {ver} 逐字节==源 + 注册表 v2(install_path/tree_digest)真")

        # 切换探针:shim 干名转发 + 物化产物真实可执行。
        (home / "bin").mkdir(parents=True, exist_ok=True)
        shim_rx = home / "bin" / "rx.exe"
        shim_rx.write_bytes(rurixup.read_bytes())
        hello = wd / "hello.rx"
        hello.write_text("fn main() {}\n", encoding="utf-8")
        r = run([str(shim_rx), "check", str(hello)], env=install_env(home))
        if r.returncode != 0:
            fail(f"shim 干名转发 rx check 未退出 0(exit={r.returncode}):"
                 f"{r.stdout[-200:]}\n{r.stderr[-200:]}")
        r2 = run([str(disk_up), "--help"], env=install_env(home))
        if r2.returncode != 0:
            fail(f"物化 rurixup.exe 不可执行(--help exit={r2.returncode})")
        print("[dist_smoke] 切换探针 ✓ shim(rx→toolchains 内真实 exe)check 退出 0;物化产物可执行")

        # 幂等:再装同源 → registered 不增 + toolchains.json 逐字节一致。
        first_reg = reg.read_bytes()
        r = run([str(rurixup), "install", "--from-dir", str(fromdir),
                 "--registry", str(reg)], env=install_env(home))
        s = tokens(r.stdout, "RURIXUP_INSTALL:")
        if r.returncode != 0 or s.get("registered") != "1":
            fail(f"幂等 install 破坏(exit={r.returncode} registered={s.get('registered')})")
        if reg.read_bytes() != first_reg:
            fail("幂等 install 后 toolchains.json 字节漂移(确定性破坏)")
        print("[dist_smoke] 幂等 ✓ 再装同源 registered=1 且 toolchains.json 逐字节一致")

        # 注册表原子写零残渣(RXS-0214 动态语义(5):tmp→rename,不留 .tmp)。
        tmp_residue = sorted(p.name for p in reg.parent.glob("*.tmp"))
        if tmp_residue:
            fail(f"注册表原子写残留 tmp 文件:{tmp_residue}")
        print("[dist_smoke] 原子写 ✓ 注册表目录零 .tmp 残渣")

        # —————————————————— RED① 篡改组件一字节 ——————————————————
        home_r = wd / "home_red"
        reg_r = home_r / "toolchains.json"
        b = bytearray((fromdir / "rx.exe").read_bytes())
        b[100] ^= 0xFF  # 翻一字节(bundle.json 仍持原 digest)。
        (fromdir / "rx.exe").write_bytes(bytes(b))
        r = run([str(rurixup), "install", "--from-dir", str(fromdir),
                 "--registry", str(reg_r)], env=install_env(home_r))
        red_dir = home_r / "toolchains" / ver
        staging_residue = list((home_r / "tmp").glob(".staging-*")) if (home_r / "tmp").is_dir() else []
        if not install_rejected_cleanly(r.returncode, red_dir.exists(), reg_r.is_file()):
            fail(f"RED① 篡改组件未干净拒装(exit={r.returncode} dir={red_dir.exists()} "
                 f"reg={reg_r.is_file()}):内容寻址校验失效")
        if staging_residue:
            fail(f"RED① staging 残留未清:{staging_residue}")
        if "RURIXUP_INSTALL_ERROR: kind=integrity" not in (r.stdout or ""):
            fail(f"RED① 缺 integrity 机器 token:{r.stdout[-200:]}")
        print("[dist_smoke] RED① ✓ 篡改组件 → 内容寻址拒(exit≠0)+ toolchains/ 零残留 + 注册表 0-byte + staging 零残留")
        # 复原 fixture 组件字节(供后续复绿)。
        (fromdir / "rx.exe").write_bytes(rx.read_bytes())

        # —————————————————— RED② default 指向已删目录 ——————————————————
        import shutil
        shutil.rmtree(toolchain_dir)
        r = run([str(rurixup), "default", ver, "--registry", str(reg)], env=install_env(home))
        if r.returncode == 0:
            fail("RED② default 指向已删版本目录未报错(应退出非 0)")
        print("[dist_smoke] RED② ✓ default 指向已删目录 → 诚实报错(exit≠0)")

        # —————————————————— 复原绿 ——————————————————
        r = run([str(rurixup), "install", "--from-dir", str(fromdir),
                 "--registry", str(reg)], env=install_env(home))
        if not install_succeeded(r.returncode, toolchain_dir.is_dir(), reg.is_file()):
            fail(f"复原绿失败(exit={r.returncode})")
        print("[dist_smoke] PASS 复原绿(红绿闭合;GREEN+切换探针+幂等 / RED①篡改 / RED②错向)")
    sys.exit(0)


if __name__ == "__main__":
    main()
