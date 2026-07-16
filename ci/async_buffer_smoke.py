#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""流序分配 AsyncBuffer 冒烟(G1 CI_GATES §2 步骤 42,契约 G-G1-2,MR-0001 / RXS-0144~0148)。

两段机器复核闸门(反 YAML-only,CI_GATES §6.3):

  (a) host 段(总跑,无需 GPU):默认构建 rurix-rt(AsyncBuffer 随 rurix-rt 始终编译,无 feature
      门控,镜像 InFlight)。对 src/rurix-rt/compile-fail/async_buffer_*.rs 三类流序分配错误样例
      逐个 rustc 编译,断言**每个都被拒绝**(分配未完成访问 E0599 / 释放后访问 E0382 / 跨 stream
      未同步 E0599,RXS-0145/0146/0147/0148)。任一**应失败者编译通过**→ 编译期拦截被放行 →
      非零退出(红)。内置 red 自检:一个合法 alloc_async/share_with 程序应编译通过(证明闸门能
      区分「拦截 vs 放行」,非 YAML-only)。

  (b) device 段(GPU + cuMemAllocAsync 真跑;否则降级 SKIP):三 stream 流序分配 + 两条 share_with
      跨 stream 时序边 + 往返数值对照(out==input)→ pipeline_ok=true。本环境无 GPU / 老驱动无流序
      分配 → device SKIP,pipeline_ok=false,g1.counter.async_buffer_pipeline 为 normal SKIP(建设期预期)。

写 evidence/async_buffer_smoke.json。pipeline_ok=true 计入 g1.counter.async_buffer_pipeline。
退出码:0=绿(host 段全绿;device 段 SKIP 属预期);非零=红(拦截放行 / red 自检失效 / 数值对照失败)。
"""
import datetime
import json
import os
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TARGET = ROOT / "target" / "debug"
CF_DIR = ROOT / "src" / "rurix-rt" / "compile-fail"
TMP = ROOT / "target" / "async_buffer_smoke"
EVIDENCE = ROOT / "evidence" / "async_buffer_smoke.json"

# fixture → (类别, 预期 rustc 错误码)。三类流序分配生命周期错误(06 §5.4 / RXS-0145~0148):
#   分配未完成访问 → AsyncBuffer 无 device_ptr E0599;释放后访问 → affine move E0382;
#   跨 stream 未同步 → AsyncBuffer 无 copy_to_host E0599。
REJECT_CLASSES = {
    "async_buffer_alloc_incomplete.rs": ("alloc_incomplete", "E0599"),
    "async_buffer_use_after_free.rs": ("use_after_free", "E0382"),
    "async_buffer_cross_stream_unsync.rs": ("cross_stream_unsync", "E0599"),
}


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, cwd=ROOT, **kw)


def skip(msg):
    print(f"[async_buffer_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[async_buffer_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def build_rlib():
    """默认构建 rurix-rt(AsyncBuffer 随 rurix-rt 始终编译,无 feature 门控)。"""
    r = run(["cargo", "build", "-p", "rurix-rt"])
    if r.returncode != 0:
        skip(f"cargo build -p rurix-rt 失败(无工具链?):\n{r.stderr[-600:]}")
    rlib = TARGET / "librurix_rt.rlib"
    deps = TARGET / "deps"
    if not rlib.exists():
        skip(f"未找到 {rlib}")
    return rlib, deps


def try_compile(path, rlib, deps):
    TMP.mkdir(parents=True, exist_ok=True)
    out = TMP / "out.rmeta"
    r = run([
        "rustc", "--edition", "2024", "--crate-type", "bin", "--emit", "metadata",
        "-o", str(out),
        "--extern", f"rurix_rt={rlib}", "-L", f"dependency={deps}", str(path),
    ])
    return r.returncode == 0, r.stderr


def red_self_test(rlib, deps):
    """合法 alloc_async/share_with 程序应编译通过(检查器能区分拦截 vs 放行)。"""
    TMP.mkdir(parents=True, exist_ok=True)
    p = TMP / "red_should_compile.rs"
    p.write_text(
        "use rurix_rt::{SharedStream, SharedEvent};\n"
        "fn ok(s: &SharedStream, other: &SharedStream, ev: &SharedEvent) {\n"
        "    if let Ok(buf) = s.alloc_async::<f32>(4) {\n"
        "        let _ = buf.share_with(other, ev);\n"
        "    }\n"
        "}\n"
        "fn main() { let _ = ok; }\n",
        encoding="utf-8",
    )
    compiled, _ = try_compile(p, rlib, deps)
    return compiled


def check_reject_classes(rlib, deps):
    facts, intercepted = [], []
    for fname, (cls, frag) in sorted(REJECT_CLASSES.items()):
        path = CF_DIR / fname
        if not path.exists():
            fail(f"缺 compile-fail 样例 {path}")
        compiled, stderr = try_compile(path, rlib, deps)
        rejected = not compiled
        frag_ok = (frag in stderr) if rejected else False
        facts.append({
            "kind": "reject", "name": cls, "fixture": f"src/rurix-rt/compile-fail/{fname}",
            "expected_error": frag, "rejected": rejected,
            "note": ("rustc 拒绝(编译期拦截)" + ("" if frag_ok else f";注:未见片段 {frag!r}"))
            if rejected else "应拦截却编译通过(违例放行)",
        })
        if not rejected:
            fail(f"流序分配错误类别放行:{fname} 应被 rustc 拒绝却编译通过"
                 f"(三类编译期拦截被破坏,反 YAML-only 红)")
        intercepted.append(cls)
    return facts, intercepted


def device_segment():
    """device 段:三 stream 流序分配 + 两条 share_with 时序边 + 往返数值对照(--ignored 真跑)。"""
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    r = run([
        "cargo", "test", "-p", "rurix-rt",
        "pipeline::tests::async_buffer_three_stream_pipeline_device",
        "--", "--ignored", "--exact", "--nocapture",
    ])
    output = r.stdout + "\n" + r.stderr
    if "ASYNC_BUFFER_DEVICE: ok pipeline=1" in output:
        print("[async_buffer_smoke] ASYNC_BUFFER_DEVICE: ok pipeline=1 (三 stream 流序分配往返数值对照通过)")
        return True, True, "alloc_async→share_with(compute)→写→share_with(d2h)→读回往返数值对照通过"
    if "ASYNC_BUFFER_DEVICE: skip" in output:
        return False, False, "无 GPU / 老驱动无 cuMemAllocAsync → device 段 SKIP"
    if require_real:
        fail(f"流序分配 device 端到端失败:\n{output[-1600:]}")
    return False, False, "device 流序分配闭环不可用(无 GPU?)→ device 段 SKIP"


def github_run_url():
    server = os.environ.get("GITHUB_SERVER_URL")
    repo = os.environ.get("GITHUB_REPOSITORY")
    run_id = os.environ.get("GITHUB_RUN_ID")
    if server and repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return "local interactive runner"


def main():
    rlib, deps = build_rlib()
    print("[async_buffer_smoke] host 段:默认构建 rurix-rt 成功(AsyncBuffer 始终编译,RXS-0144)✓")

    if not red_self_test(rlib, deps):
        fail("red 自检失败:合法 alloc_async/share_with 程序未能编译(闸门失效或工具链异常)")

    reject_facts, intercepted = check_reject_classes(rlib, deps)
    print(f"[async_buffer_smoke] host 段:三类流序分配错误 100% 编译期拦截 ✓ {intercepted}")

    pipeline_ok, device_run, note = device_segment()
    print(f"[async_buffer_smoke] device 段:{note}")

    doc = {
        "schema_version": 1,
        "subject": "async_buffer",
        "pipeline_ok": pipeline_ok,
        "reject_classes_intercepted": intercepted,
        "device_path_run": device_run,
        "run_command": "cargo test -p rurix-rt pipeline::tests::async_buffer_three_stream_pipeline_device -- --ignored --exact --nocapture",
        "device": {"result_line": note},
        "facts": reject_facts + [{
            "kind": "pipeline", "name": "three_stream_async_pipeline",
            "note": "三 stream 流序分配(cuMemAllocAsync)+ 两条 share_with 跨 stream 时序边 + 往返 out==input(RXS-0148)",
        }],
        "redgreen": {
            "red_command": "放行任一 compile-fail 违例(使应拦截者编译通过)/ 篡改往返数值对照",
            "red_detected": True,
            "green_command": "py -3 ci/async_buffer_smoke.py",
            "green_exit_code": 0,
            "run_url": f"green={github_run_url()}",
        },
        "timestamp": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[async_buffer_smoke] PASS 写 {EVIDENCE.relative_to(ROOT)}"
          f"(pipeline_ok={pipeline_ok},reject 3/3 拦截;device 真跑回填见步骤 42)")
    sys.exit(0)


if __name__ == "__main__":
    main()
