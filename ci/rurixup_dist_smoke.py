#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""rurixup 真实分发冒烟(EA1.1a+EA1.1b / RFC-0012,CI 步骤 59 前半+后半,RXS-0214 ~ RXS-0217)。

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
EA1.1b 后半(hermetic 环回 HTTP,RXS-0216/0217):Python `http.server`(stdlib,零第三方)
起本地 fixture 于 127.0.0.1 随机端口,served = 真实构建产物组件 + 真算 digest 清单链;
`RURIXUP_TEST_ALLOW_LOOPBACK_HTTP=1` 下经系统 curl.exe 全链网络 install 物化绿;RED 四路
各自独立见证——①组件坏一字节(级④)②清单坏哈希(级①锚失配)③截断传输(curl 部分)
④协议降级(缺 env 拒 http)——+ 端点不可达(fixture 关)诚实 network 错误 + 系统 0-byte;
每路断言 RURIXUP_INSTALL_ERROR kind。**pr-smoke 零真实外呼**——fixture 进程为唯一网络面。
"""
import hashlib
import http.server
import json
import os
import re
import socketserver
import subprocess
import sys
import tempfile
import threading
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


def has_kind(stdout: str, kind: str) -> bool:
    """机器 token 判据:stdout 含 `RURIXUP_INSTALL_ERROR: kind=<kind>`。纯函数。"""
    return f"RURIXUP_INSTALL_ERROR: kind={kind}" in (stdout or "")


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
    # (e) 网络机器 token(EA1.1b):kind 提取正/反。
    if not has_kind("RURIXUP_INSTALL_ERROR: kind=network\n其它", "network"):
        fail("red 自检失败:network token 未被识别(门失效)")
    if has_kind("RURIXUP_INSTALL: version=1.1.0", "network"):
        fail("red 自检失败:成功摘要被误判含 network token(门过松)")
    if has_kind("RURIXUP_INSTALL_ERROR: kind=integrity", "network"):
        fail("red 自检失败:integrity 被误判为 network(kind 串扰,门失效)")


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


def install_env(home: Path, loopback: bool = False) -> dict:
    env = dict(os.environ)
    env["RURIX_HOME"] = str(home)
    if loopback:
        env["RURIXUP_TEST_ALLOW_LOOPBACK_HTTP"] = "1"
    else:
        env.pop("RURIXUP_TEST_ALLOW_LOOPBACK_HTTP", None)
    return env


# ————————————————— EA1.1b hermetic 环回 HTTP fixture(RXS-0216/0217)—————————————————

# 截断标记:{相对路径: True} → 该资源声明完整 Content-Length 但只发半个 body(curl 部分传输)。
_TRUNCATE: dict = {}


def _make_handler(served_dir: Path):
    class Handler(http.server.BaseHTTPRequestHandler):
        def log_message(self, *_a):  # 静默(pr-smoke 干净输出)
            pass

        def do_GET(self):
            rel = self.path.lstrip("/").split("?", 1)[0]
            fp = served_dir / rel
            if not fp.is_file():
                self.send_error(404, "not found")
                return
            data = fp.read_bytes()
            if _TRUNCATE.get(rel):
                # 声明完整长度但只发一半 → 提前关闭 → curl 报部分传输(exit 18)。
                self.send_response(200)
                self.send_header("Content-Length", str(len(data)))
                self.end_headers()
                try:
                    self.wfile.write(data[: max(1, len(data) // 2)])
                except Exception:
                    pass
                return
            self.send_response(200)
            self.send_header("Content-Length", str(len(data)))
            self.end_headers()
            self.wfile.write(data)

    return Handler


def start_fixture(served_dir: Path):
    """起本地 http.server 于 127.0.0.1 随机端口(daemon 线程),返回 (httpd, base_url)。"""
    httpd = socketserver.TCPServer(("127.0.0.1", 0), _make_handler(served_dir))
    httpd.timeout = 5
    port = httpd.server_address[1]
    t = threading.Thread(target=httpd.serve_forever, daemon=True)
    t.start()
    return httpd, f"http://127.0.0.1:{port}/"


def write_anchor(anchor_path: Path, ver: str, base_url: str, channel_manifest_bytes: bytes) -> None:
    """写 repo 锚 channels/stable.json(本地文件,base_url 指向 fixture;级① digest 真算)。"""
    digest = hashlib.sha256(channel_manifest_bytes).hexdigest()
    anchor = {
        "schema_version": 1,
        "channel": "stable",
        "releases": [
            {"version": ver, "channel_manifest_sha256": digest, "base_url": base_url}
        ],
        "latest": ver,
    }
    # 规范缩进(与 fetch.rs line-scan 解析形态对齐:每字段独立行)。
    lines = ["{", '  "schema_version": 1,', '  "channel": "stable",', '  "releases": [', "    {",
             f'      "version": "{ver}",',
             f'      "channel_manifest_sha256": "{digest}",',
             f'      "base_url": "{base_url}"',
             "    }", "  ],", f'  "latest": "{ver}"', "}", ""]
    anchor_path.write_bytes("\n".join(lines).encode("utf-8"))
    # anchor 变量仅用于交叉核对 digest 一致(防手写 JSON 与结构漂移)。
    assert json.loads(anchor_path.read_text(encoding="utf-8"))["releases"][0]["channel_manifest_sha256"] == digest
    assert anchor["latest"] == ver


def net_install(rurixup: Path, ver: str, anchor: Path, home: Path, reg: Path, loopback: bool):
    """跑网络 install:`rurixup install <ver> --channel-file <anchor> --registry <reg> --home <home>`。"""
    return run(
        [str(rurixup), "install", ver, "--channel-file", str(anchor),
         "--registry", str(reg), "--home", str(home), "--max-time", "30"],
        env=install_env(home, loopback=loopback),
    )


def net_half(rurixup: Path, rx: Path, ver: str, wd: Path):
    """EA1.1b 后半:hermetic 环回 HTTP 全链 install 绿 + 四路 RED + 不可达。"""
    served = wd / "served"
    served.mkdir(parents=True, exist_ok=True)
    # served 资产 = 真实构建产物 + 真算 digest 清单链(复用 make_fixture 布局)。
    fx = make_fixture(rurixup, rx, wd / "netfx", ver)
    for name in ("rx.exe", "rurixup.exe", "bundle.json", "channel_manifest.json"):
        (served / name).write_bytes((fx / name).read_bytes())

    httpd, base_url = start_fixture(served)
    try:
        anchor = wd / "stable.json"
        write_anchor(anchor, ver, base_url, (served / "channel_manifest.json").read_bytes())

        # —————————————————— GREEN(全链网络物化)——————————————————
        home = wd / "nethome"
        reg = home / "toolchains.json"
        r = net_install(rurixup, ver, anchor, home, reg, loopback=True)
        tdir = home / "toolchains" / ver
        if not install_succeeded(r.returncode, tdir.is_dir(), reg.is_file()):
            fail(f"后半 GREEN 网络 install 未成功(exit={r.returncode} dir={tdir.is_dir()} "
                 f"reg={reg.is_file()}):{r.stdout[-400:]}\n{r.stderr[-400:]}")
        s = tokens(r.stdout, "RURIXUP_INSTALL:")
        if s.get("version") != ver or s.get("digest_levels_verified") != "4" or s.get("components") != "2":
            fail(f"后半 GREEN install 摘要不符:{s}")
        # 磁盘逐字节 == 源(经 curl 真下载 + 四级校验 + 物化)。
        if (tdir / "bin" / "rx.exe").read_bytes() != rx.read_bytes():
            fail("后半 GREEN 物化 rx.exe 与源逐字节不等(防降级:非真实下载物化)")
        print(f"[dist_smoke] 后半 GREEN ✓ 环回 HTTP 全链 install {ver} 物化(curl→四级校验→rename)+ 逐字节==源")

        # —————————————————— RED① 组件坏一字节(级④)——————————————————
        good_rx = (served / "rx.exe").read_bytes()
        b = bytearray(good_rx)
        b[100] ^= 0xFF
        (served / "rx.exe").write_bytes(bytes(b))
        home1 = wd / "nethome_r1"
        reg1 = home1 / "toolchains.json"
        r = net_install(rurixup, ver, anchor, home1, reg1, loopback=True)
        if not install_rejected_cleanly(r.returncode, (home1 / "toolchains" / ver).exists(), reg1.is_file()):
            fail(f"RED① 坏组件未干净拒装(exit={r.returncode}):{r.stdout[-300:]}\n{r.stderr[-300:]}")
        if not has_kind(r.stdout, "integrity"):
            fail(f"RED① 缺 integrity token:{r.stdout[-300:]}")
        _assert_no_staging(home1)
        (served / "rx.exe").write_bytes(good_rx)  # 复原
        print("[dist_smoke] RED① ✓ 组件坏字节 → 级④ 内容寻址拒(kind=integrity)+ 零半装 + staging 零残留")

        # —————————————————— RED② 清单坏哈希(级①锚失配)——————————————————
        good_mf = (served / "channel_manifest.json").read_bytes()
        # 保持合法 UTF-8/JSON 前提下改字节(改 channel 值一字母)→ 级① 哈希失配
        # (级① 为纯字节 sha256 对比,先于任何解析;不制造无效 UTF-8 令 read 早失败)。
        mf_text = good_mf.decode("utf-8")
        tampered_text = mf_text.replace('"stable"', '"stablx"', 1)
        if tampered_text == mf_text:
            fail("RED② 构造失败:channel_manifest 未含预期 stable 令牌")
        (served / "channel_manifest.json").write_bytes(tampered_text.encode("utf-8"))
        home2 = wd / "nethome_r2"
        reg2 = home2 / "toolchains.json"
        r = net_install(rurixup, ver, anchor, home2, reg2, loopback=True)
        if not install_rejected_cleanly(r.returncode, (home2 / "toolchains" / ver).exists(), reg2.is_file()):
            fail(f"RED② 坏清单哈希未干净拒装(exit={r.returncode}):{r.stdout[-300:]}")
        if not has_kind(r.stdout, "integrity"):
            fail(f"RED② 缺 integrity token(级①锚失配):{r.stdout[-300:]}")
        (served / "channel_manifest.json").write_bytes(good_mf)  # 复原
        print("[dist_smoke] RED② ✓ 清单坏哈希 → 级①锚→channel 失配拒(kind=integrity)")

        # —————————————————— RED③ 截断传输(curl 部分)——————————————————
        _TRUNCATE["rx.exe"] = True
        home3 = wd / "nethome_r3"
        reg3 = home3 / "toolchains.json"
        r = net_install(rurixup, ver, anchor, home3, reg3, loopback=True)
        _TRUNCATE.pop("rx.exe", None)
        if not install_rejected_cleanly(r.returncode, (home3 / "toolchains" / ver).exists(), reg3.is_file()):
            fail(f"RED③ 截断未干净拒装(exit={r.returncode}):{r.stdout[-300:]}")
        if not has_kind(r.stdout, "network"):
            fail(f"RED③ 缺 network token(curl 部分传输):{r.stdout[-300:]}\n{r.stderr[-300:]}")
        _assert_no_staging(home3)
        print("[dist_smoke] RED③ ✓ 截断传输 → curl 部分传输非零退出(kind=network)+ 零半装")

        # —————————————————— RED④ 协议降级(缺 env 拒 http)——————————————————
        home4 = wd / "nethome_r4"
        reg4 = home4 / "toolchains.json"
        r = net_install(rurixup, ver, anchor, home4, reg4, loopback=False)  # 无 env 标志
        if not install_rejected_cleanly(r.returncode, (home4 / "toolchains" / ver).exists(), reg4.is_file()):
            fail(f"RED④ 协议降级未干净拒装(exit={r.returncode}):{r.stdout[-300:]}")
        if not has_kind(r.stdout, "network"):
            fail(f"RED④ 缺 network token(默认态 https-only 拒 http):{r.stdout[-300:]}")
        print("[dist_smoke] RED④ ✓ 协议降级 → 缺 env 默认态拒 http://127.0.0.1(kind=network,https-only fail-closed)")

    finally:
        httpd.shutdown()
        httpd.server_close()

    # —————————————————— 端点不可达(fixture 已关)——————————————————
    home5 = wd / "nethome_r5"
    reg5 = home5 / "toolchains.json"
    r = net_install(rurixup, ver, anchor, home5, reg5, loopback=True)
    if not install_rejected_cleanly(r.returncode, (home5 / "toolchains" / ver).exists(), reg5.is_file()):
        fail(f"不可达未干净拒装(exit={r.returncode}):{r.stdout[-300:]}")
    if not has_kind(r.stdout, "network"):
        fail(f"不可达缺 network token(curl 连接失败):{r.stdout[-300:]}\n{r.stderr[-300:]}")
    print("[dist_smoke] 不可达 ✓ fixture 关闭 → curl 连接失败(kind=network)+ 系统 0-byte,不 fake success")
    print("[dist_smoke] 后半 PASS(hermetic 环回 HTTP:GREEN 全链 / RED①坏字节 ②坏哈希 ③截断 ④协议降级 / 不可达;零真实外呼)")


def _assert_no_staging(home: Path):
    """断言 home\\tmp 下无 .staging- / .download- 残留(零半装辅助)。"""
    tmp = home / "tmp"
    if tmp.is_dir():
        for e in tmp.iterdir():
            nm = e.name
            if nm.startswith(".staging-") or nm.startswith(".download-"):
                fail(f"staging/download 残留未清:{e}")


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
        print("[dist_smoke] 前半 PASS 复原绿(红绿闭合;GREEN+切换探针+幂等 / RED①篡改 / RED②错向)")

        # —————————————————— EA1.1b 后半:hermetic 环回 HTTP 网络拉取 ——————————————————
        net_half(rurixup, rx, ver, wd)
    print("[dist_smoke] PASS 前半(离线 --from-dir)+ 后半(hermetic 环回 HTTP)全绿")
    sys.exit(0)


if __name__ == "__main__":
    main()
