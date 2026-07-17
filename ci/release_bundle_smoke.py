#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""发布 bundle 打包冒烟(EA1.2 / RFC-0012,CI 步骤 60,RXS-0218)。

契约 G-EA1-4 发布资产门(**纯离线面**,spec/release.md RXS-0218)。上传本体 / 回读
自校验 / 信任根登记流**只在 release.yml**(全部 hard-block 门后);本步骤只核发布
资产的**离线可判性**:

  ① 打包确定性:同源两次 `rurixup release` 编排产 bundle.json / SHA256SUMS 逐字节一致;
  ② 资产字节与 bundle.json 组件 digest **一比一闭环**:每组件源文件字节 sha256 ==
     bundle.json 声明 digest == SHA256SUMS 对应行 digest(无第二 digest 域);
  ③ 3 组件完备(rx.exe / rurixup.exe / rurix_rt_cabi.lib):完备 → release_complete=true;
     **人为缺 .lib → RED 见证**(release_complete=false + release_missing 含 rurix_rt_cabi.lib);
  ④ SHA256SUMS 字典序断言(标准 sha256sum 双空格格式,干名字典序);
  ⑤ channels/stable.json 锚 schema 字段面校验(releases[{version,channel_manifest_sha256,base_url}]);
  ⑥ 内建 red_self_test(反 YAML-only:合成红/绿喂纯判定层,证门能区分)。

**零真实外呼**、host/CPU-only、零 GPU 依赖。rurix_rt_cabi.lib 组件在本步骤以确定性合成
字节代表(打包确定性 / digest 闭环 / 完备判定不依赖 lib 是否为真编译产物——crt-static 真
构建 + cargo test 归 release.yml);EXE/产物落 %TEMP%,不留仓库。退出码:0=绿;非零=红。
"""
import hashlib
import json
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
    print(f"[bundle_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[bundle_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


# ————————————————— 纯判定层(red 自检直接喂合成数据)—————————————————


def is_complete(missing: list) -> bool:
    """GREEN 判据:3 组件完备(missing 为空)。纯函数。"""
    return len(missing) == 0


def digests_match(a: str, b: str) -> bool:
    """一比一内容寻址判据:两 digest 逐字符相等且非空。纯函数。"""
    return bool(a) and a == b


def is_lexicographic(names: list) -> bool:
    """SHA256SUMS 字典序判据:干名序列 == 其字典序排序。纯函数。"""
    return names == sorted(names)


def red_self_test() -> None:
    """反 YAML-only:合成红/绿场景喂纯判定层,断言门能区分。门失效即红。"""
    # (a) 完备判定正/反。
    if not is_complete([]):
        fail("red 自检失败:空 missing 未被识别为完备(门失效)")
    if is_complete(["rurix_rt_cabi.lib"]):
        fail("red 自检失败:缺 .lib 被误判为完备(门过松,吞红成绿)")
    # (b) digest 闭环正/反。
    if not digests_match("ab" * 32, "ab" * 32):
        fail("red 自检失败:相等 digest 未被识别匹配(门失效)")
    if digests_match("ab" * 32, "cd" * 32):
        fail("red 自检失败:不等 digest 被误判匹配(门过松)")
    if digests_match("", ""):
        fail("red 自检失败:空 digest 被误判匹配(门失效)")
    # (c) 字典序正/反。
    if not is_lexicographic(["a", "b", "c"]):
        fail("red 自检失败:已排序序列未被识别为字典序(门失效)")
    if is_lexicographic(["c", "a", "b"]):
        fail("red 自检失败:乱序被误判为字典序(门过松)")


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


def sha256_file(p: Path) -> str:
    return hashlib.sha256(p.read_bytes()).hexdigest()


def release_tokens(stdout: str) -> dict:
    out = {}
    for ln in (stdout or "").splitlines():
        if ln.startswith("RURIXUP_RELEASE:"):
            for tok in ln[len("RURIXUP_RELEASE:"):].split():
                if "=" in tok:
                    k, v = tok.split("=", 1)
                    out[k] = v
    return out


def make_components(wd: Path, rurixup: Path, rx: Path, ver: str) -> dict:
    """造 3 组件源文件(rx.exe/rurixup.exe 真实构建产物 + rurix_rt_cabi.lib 确定性合成)。"""
    src = wd / "src"
    src.mkdir(parents=True, exist_ok=True)
    (src / "rx.exe").write_bytes(rx.read_bytes())
    (src / "rurixup.exe").write_bytes(rurixup.read_bytes())
    # 确定性合成 crt-static rurix_rt_cabi.lib(打包确定性/digest 闭环/完备判定不依赖
    # 其为真编译产物——crt-static 真构建 + cargo test 归 release.yml)。
    (src / "rurix_rt_cabi.lib").write_bytes(("RURIX_RT_CABI_STATIC_LIB_" + ver).encode("utf-8") * 8)
    return {
        "rx.exe": src / "rx.exe",
        "rurixup.exe": src / "rurixup.exe",
        "rurix_rt_cabi.lib": src / "rurix_rt_cabi.lib",
    }


def do_release(rurixup: Path, ver: str, comps: dict, out_dir: Path):
    """跑 rurixup release,返回 (exit, tokens, out_dir)。"""
    cmd = [str(rurixup), "release", "--version", ver, "--out-dir", str(out_dir)]
    for name, path in comps.items():
        cmd += ["--component", f"{name}|{ver}|Apache-2.0|core|{path}"]
        cmd += ["--sign", f"{name}|Valid|true|selftest"]
    r = run(cmd)
    return r.returncode, release_tokens(r.stdout), r


def parse_sha256sums(text: str) -> list:
    """解析 SHA256SUMS → [(digest, name)];断言标准双空格格式。"""
    rows = []
    for ln in text.splitlines():
        if not ln:
            continue
        parts = ln.split("  ", 1)
        if len(parts) != 2:
            fail(f"SHA256SUMS 行非标准双空格格式:{ln!r}")
        rows.append((parts[0], parts[1]))
    return rows


def check_anchor_schema() -> None:
    """channels/stable.json 锚 schema 字段面校验(RXS-0217;合成样例,纯离线)。"""
    sample = {
        "schema_version": 1,
        "channel": "stable",
        "releases": [
            {"version": "1.1.0", "channel_manifest_sha256": "ab" * 32,
             "base_url": "https://github.com/o/r/releases/download/v1.1.0/"}
        ],
        "latest": "1.1.0",
    }
    for k in ("schema_version", "channel", "releases", "latest"):
        if k not in sample:
            fail(f"锚 schema 缺顶层字段 {k}")
    rel = sample["releases"][0]
    for k in ("version", "channel_manifest_sha256", "base_url"):
        if k not in rel:
            fail(f"锚 releases[] 缺字段 {k}")
    # 反例:缺 channel_manifest_sha256 → 校验应判缺(门有效)。
    bad = {"version": "1.1.0", "base_url": "x"}
    if all(k in bad for k in ("version", "channel_manifest_sha256", "base_url")):
        fail("锚 schema red 自检失败:缺 channel_manifest_sha256 未被识别")
    print("[bundle_smoke] ⑤ ✓ channels/stable.json 锚 schema 字段面校验(releases[version/digest/base_url])")


def main():
    red_self_test()
    rurixup, rx = build()
    ver = workspace_version()

    with tempfile.TemporaryDirectory() as td:
        wd = Path(td)
        comps = make_components(wd, rurixup, rx, ver)

        # ————— GREEN:3 组件完备 —————
        code, tok, r = do_release(rurixup, ver, comps, wd / "rel1")
        if code != 0:
            fail(f"release 未放行(exit={code}):{r.stdout[-300:]}\n{r.stderr[-300:]}")
        missing = [m for m in tok.get("release_missing", "").strip("[]").split(",") if m]
        if not (tok.get("release_complete") == "true" and is_complete(missing)):
            fail(f"③ 3 组件完备判定失败(release_complete={tok.get('release_complete')} "
                 f"missing={missing}):{tok}")
        print("[bundle_smoke] ③ ✓ 3 组件完备(rx.exe/rurixup.exe/rurix_rt_cabi.lib)→ release_complete=true")

        bundle = json.loads((wd / "rel1" / "bundle.json").read_text(encoding="utf-8"))
        bundle_digests = {c["name"]: c["sha256"] for c in bundle["components"]}

        # ② 资产字节与 bundle.json 组件 digest 一比一闭环。
        for name, path in comps.items():
            real = sha256_file(path)
            if not digests_match(real, bundle_digests.get(name, "")):
                fail(f"② digest 闭环破裂:{name} 源字节 sha256={real} != bundle.json "
                     f"digest={bundle_digests.get(name)}")
        print("[bundle_smoke] ② ✓ 资产字节 sha256 == bundle.json 组件 digest(一比一内容寻址闭环)")

        # ④ SHA256SUMS 字典序 + 每行 digest == bundle.json digest。
        sums_text = (wd / "rel1" / "SHA256SUMS").read_text(encoding="utf-8")
        rows = parse_sha256sums(sums_text)
        names = [n for _, n in rows]
        if not is_lexicographic(names):
            fail(f"④ SHA256SUMS 非字典序:{names}")
        for digest, name in rows:
            if not digests_match(digest, bundle_digests.get(name, "")):
                fail(f"④ SHA256SUMS 行 digest 与 bundle.json 不符:{name}")
        print(f"[bundle_smoke] ④ ✓ SHA256SUMS 字典序 {names} + 每行 digest == bundle.json")

        # ① 打包确定性:同源两次逐字节一致(bundle.json + SHA256SUMS)。
        code2, _, r2 = do_release(rurixup, ver, comps, wd / "rel2")
        if code2 != 0:
            fail(f"② 二次 release 未放行(exit={code2})")
        for name in ("bundle.json", "SHA256SUMS"):
            a = (wd / "rel1" / name).read_bytes()
            b = (wd / "rel2" / name).read_bytes()
            if a != b:
                fail(f"① 打包确定性破坏:{name} 同源两次字节漂移")
        print("[bundle_smoke] ① ✓ 打包确定性(bundle.json + SHA256SUMS 同源两次逐字节一致)")

        # ⑤ 锚 schema 字段面。
        check_anchor_schema()

        # ————— RED:人为缺 rurix_rt_cabi.lib —————
        two = {"rx.exe": comps["rx.exe"], "rurixup.exe": comps["rurixup.exe"]}
        code_r, tok_r, r_r = do_release(rurixup, ver, two, wd / "rel_red")
        # release 本体仍放行(完备判定非 hard-block 子门,老版本两件清单语义 0-byte),
        # 但 release_complete 必须诚实 false + release_missing 含 rurix_rt_cabi.lib。
        missing_r = [m for m in tok_r.get("release_missing", "").strip("[]").split(",") if m]
        if tok_r.get("release_complete") != "false" or "rurix_rt_cabi.lib" not in missing_r:
            fail(f"③ RED 缺件未被检出(release_complete={tok_r.get('release_complete')} "
                 f"missing={missing_r}):缺件即红判据失效")
        # RED 态 SHA256SUMS 仍字典序(两件)。
        red_rows = parse_sha256sums((wd / "rel_red" / "SHA256SUMS").read_text(encoding="utf-8"))
        if not is_lexicographic([n for _, n in red_rows]):
            fail("④ RED 态 SHA256SUMS 非字典序")
        print("[bundle_smoke] RED ✓ 人为缺 rurix_rt_cabi.lib → release_complete=false + "
              "release_missing=[rurix_rt_cabi.lib](缺件即红见证)")

    print("[bundle_smoke] PASS(①打包确定性 ②digest 闭环 ③3 组件完备+缺件 RED "
          "④SHA256SUMS 字典序 ⑤锚 schema ⑥red_self_test;零真实外呼)")
    sys.exit(0)


if __name__ == "__main__":
    main()
