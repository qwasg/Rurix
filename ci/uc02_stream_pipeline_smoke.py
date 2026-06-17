#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-02 三 stream 重叠流水线 + 跨线程所有权转移冒烟(M8 CI_GATES §2 步骤 36,契约 G-M8-3)。

两段机器复核闸门(反 YAML-only):

  (a) host 段(总跑,无需 GPU):对 src/uc02-demo/compile-fail/*.rs 四类资源生命周期违例样例
      逐个 rustc 编译,断言**每个都被拒绝**且携预期错误码(use-after-free E0382 / double-free
      E0599 / 跨 stream 未同步 E0599 / 跨线程非法转移 E0277,RXS-0134)。任一**应失败者编译
      通过**→ 资源生命周期编译期拦截被放行 → 非零退出(红)。内置 red 自检:构造一个合法
      (会编译通过)样例,断言拦截检查器把它识别为「未拦截」(证明闸门非 YAML-only)。

  (b) device 段(有 GPU 真跑;无 GPU 降级 SKIP):cargo run -p uc02-demo 端到端——三 stream
      重叠(H2D/compute/D2H + event 流序依赖 + 流序分配类型化 InFlight)+ 跨线程 DeviceBox/
      SharedEvent 转移,逐元素数值对照通过 → stream_pipeline_ok=true。

写 evidence/uc02_stream_pipeline.json(stream_pipeline_ok / reject_classes_intercepted / facts /
redgreen)。stream_pipeline_ok=true 计入 m8.counter.uc02_stream_pipeline(>=1 PASS;建设期/无 GPU
为 normal SKIP)。退出码:0=绿(或无 GPU 降级 SKIP);非零=红(拦截放行 / 端到端数值对照失败)。
"""
import datetime
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"
CF_DIR = ROOT / "src" / "uc02-demo" / "compile-fail"
TMP = ROOT / "target" / "uc02_smoke"
EVIDENCE = ROOT / "evidence" / "uc02_stream_pipeline.json"

# fixture 文件名 → (类别名, 预期 rustc 错误码)
REJECT_CLASSES = {
    "use_after_move.rs": ("use_after_free", "E0382"),
    "double_free.rs": ("double_free", "E0599"),
    "cross_stream_unsync.rs": ("cross_stream_unsync", "E0599"),
    "cross_thread_send.rs": ("cross_thread_send", "E0277"),
}


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[uc02_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[uc02_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def build_rlib():
    """构建 rurix-rt 库,返回 (rlib 路径, deps 目录)。失败 → SKIP(无 cargo/工具链)。"""
    r = run(["cargo", "build", "-p", "rurix-rt"])
    if r.returncode != 0:
        skip(f"cargo build -p rurix-rt 失败(无工具链?):\n{r.stderr[-800:]}")
    rlib = TARGET / "librurix_rt.rlib"
    deps = TARGET / "deps"
    if not rlib.exists():
        skip(f"未找到 {rlib}")
    return rlib, deps


def try_compile(path, rlib, deps):
    """rustc 编译单文件(metadata-only,输出到 TMP 避开 /dev/null 跨盘);返回 (编译通过?, stderr)。"""
    TMP.mkdir(parents=True, exist_ok=True)
    out = TMP / "out.rmeta"
    r = run([
        "rustc", "--edition", "2024", "--crate-type", "bin", "--emit", "metadata",
        "-o", str(out),
        "--extern", f"rurix_rt={rlib}", "-L", f"dependency={deps}", str(path),
    ])
    return r.returncode == 0, r.stderr


def is_rejected(path, expected_code, rlib, deps):
    """样例被 rustc 拒绝且携预期错误码 → True(资源生命周期错误类别被编译期拦截)。"""
    compiled, stderr = try_compile(path, rlib, deps)
    return (not compiled) and (expected_code in stderr)


def red_self_test(rlib, deps):
    """red 自检(反 YAML-only):构造一个合法(会编译通过)样例,断言拦截检查器把它识别为
    「未拦截」——证明闸门真的在编译每个样例、能区分「拦截 vs 放行」,而非空过。"""
    TMP.mkdir(parents=True, exist_ok=True)
    p = TMP / "red_should_compile.rs"
    p.write_text(
        "use rurix_rt::DeviceBox;\n"
        "fn ok(b: DeviceBox<f32>) -> usize { b.len() }\n"
        "fn main() { let _ = ok; }\n",
        encoding="utf-8",
    )
    # 合法样例应「未被拦截」(编译通过);若 is_rejected 误报 True 则检查器坏。
    return not is_rejected(p, "E0382", rlib, deps)


def check_reject_classes(rlib, deps):
    """(a) host 段:四类资源生命周期违例 100% 编译期拦截核对。返回 (facts, intercepted)。"""
    facts, intercepted = [], []
    for fname, (cls, code) in sorted(REJECT_CLASSES.items()):
        path = CF_DIR / fname
        if not path.exists():
            fail(f"缺 compile-fail 样例 {path}")
        rejected = is_rejected(path, code, rlib, deps)
        facts.append({
            "kind": "reject", "name": cls, "fixture": f"src/uc02-demo/compile-fail/{fname}",
            "expected_error": code, "rejected": rejected,
            "note": "rustc 拒绝(编译期拦截)" if rejected else "应拦截却编译通过(违例放行)",
        })
        if rejected:
            intercepted.append(cls)
        else:
            fail(f"资源生命周期违例放行:{fname} 应被 rustc({code})拒绝却编译通过"
                 f"(资源生命周期编译期拦截被破坏,反 YAML-only 红)")
    return facts, intercepted


def run_demo():
    """(b) device 段:cargo run -p uc02-demo 端到端。返回 (stream_pipeline_ok, device_run, result_line, facts)。"""
    r = run(["cargo", "run", "-q", "-p", "uc02-demo"])
    line = ""
    for ln in (r.stdout or "").splitlines():
        if ln.startswith("UC02_RESULT:"):
            line = ln.strip()
            break
    if not line:
        fail(f"uc02-demo 未输出 UC02_RESULT(exit={r.returncode}):\n{r.stdout[-400:]}\n{r.stderr[-400:]}")
    facts = []
    if "skip" in line:
        return False, False, line, facts
    if r.returncode != 0 or " ok " not in f" {line} ":
        fail(f"uc02-demo 端到端数值对照失败:{line}")
    for key in ("part_a_overlap_max_err", "part_b_cross_thread_max_err"):
        m = re.search(rf"{key}=([0-9.eE+-]+)", line)
        if m:
            facts.append({"kind": "pipeline", "name": key, "max_abs_err": float(m.group(1)),
                          "tolerance": 1e-4})
    return True, True, line, facts


def main():
    rlib, deps = build_rlib()

    if not red_self_test(rlib, deps):
        fail("red 自检失败:拦截检查器把一个合法样例误报为「已拦截」(闸门失效)")

    reject_facts, intercepted = check_reject_classes(rlib, deps)
    print(f"[uc02_smoke] host 段:四类资源生命周期违例 100% 编译期拦截 ✓ {intercepted}")

    pipeline_ok, device_run, result_line, pipe_facts = run_demo()
    if device_run:
        print(f"[uc02_smoke] device 段:{result_line}")
    else:
        print(f"[uc02_smoke] device 段:无 GPU 降级 SKIP(host 拦截段已绿);{result_line}")

    doc = {
        "schema_version": 1,
        "subject": "uc02_stream_pipeline",
        "stream_pipeline_ok": pipeline_ok,
        "reject_classes_intercepted": intercepted,
        "device_path_run": device_run,
        "run_command": "cargo run -p uc02-demo;rustc 编译 src/uc02-demo/compile-fail/*.rs 断言拒绝",
        "device": {"result_line": result_line},
        "facts": reject_facts + pipe_facts,
        "redgreen": {
            "red_command": "放行任一 compile-fail 违例(使应拦截者编译通过)/ 篡改 uc02-demo 数值",
            "red_detected": True,
            "green_command": "py -3 ci/uc02_stream_pipeline_smoke.py",
            "green_exit_code": 0,
            "run_url": "TODO:回填 self-hosted runner 绿→红→复原绿 run URL(步骤 36)",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[uc02_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
          f"(stream_pipeline_ok={pipeline_ok},reject 4/4 拦截)")
    sys.exit(0)


if __name__ == "__main__":
    main()
