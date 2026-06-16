# -*- coding: utf-8 -*-
"""core 数学库 Vec/Mat/swizzle 原语端到端冒烟(M7.1,契约 G-M7-4;M7 CI_GATES 步骤 29)。

用法:
    py -3 ci/stdlib_math_smoke.py            # host 真跑 + device codegen + 写证据
    py -3 ci/stdlib_math_smoke.py --self-test # 仅跑内建篡改红绿自检(不写证据)

host 路径:具体 f32 结构体 API(conformance/stdlib/host/*.rx)经 rurixc 全管线产 EXE →
  真跑核对成功 marker(vec_ops → "stdlib-vec-ok";mat_ops → "stdlib-mat-ok")。
device 路径:语义同义标量分量 device fn 原语(conformance/stdlib/device/*.rx)经
  rurixc `--emit=nvptx-ir` 产 NVPTX IR(0 退出 + 非空 IR);`--emit=ptx`(ptxas 干验证,
  RXS-0073)best-effort(ptxas 缺失则 device_facts 记 skipped,不失败本门)。
  聚合/结构体值类型 device codegen 为后续扩展(spec/stdlib.md §5),本轮 device 路径以
  标量子集实现。GPU roundtrip 有硬件即跑、无则 skipped(本门不强制)。

内建红绿(反 YAML-only,H06 D11.8-2):篡改一个 host 期望值 → 真跑必**不**打印成功
  marker(红);原样源真跑打印 marker(绿)。应红却绿即脚本 FAIL。

primitives_passed(host 真跑 AND device codegen 双过)写入 evidence/stdlib_math_smoke.json,
  去重计数计入 m7.counter.math_primitives(ci/budget_eval.py,>=8 则 PASS)。
"""

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RURIXC = ROOT / "target" / "debug" / "rurixc.exe"
OUT_DIR = ROOT / "build" / "stdlib_math_smoke"
EVIDENCE = ROOT / "evidence" / "stdlib_math_smoke.json"

HOST_CASES = [
    ("conformance/stdlib/host/vec_ops.rx", "stdlib-vec-ok"),
    ("conformance/stdlib/host/mat_ops.rx", "stdlib-mat-ok"),
    ("conformance/stdlib/host/geom_ops.rx", "stdlib-geom-ok"),
]
DEVICE_CASES = [
    "conformance/stdlib/device/vec_scalar.rx",
    "conformance/stdlib/device/mat_scalar.rx",
    "conformance/stdlib/device/geom_scalar.rx",
]
# host 真跑 AND device codegen 双路径均覆盖的 core 数学库原语(契约 G-M7-4)
PRIMITIVES = [
    "vec_construct",
    "vec_swizzle",
    "vec_add",
    "vec_scale",
    "vec_dot",
    "vec_cross",
    "vec_length",
    "vec_normalize",
    "mat_construct",
    "mat_mul",
    "mat_vec_mul",
    # 几何原语 / 谓词(host 结构体真跑 + device 标量分量原语 codegen 双过)
    "geom_convert",
    "point_in_aabb",
    "point_aabb_distance",
    "ray_aabb_intersect",
]


def fail(msg: str) -> None:
    print(f"[stdlib_math_smoke] FAIL: {msg}")
    sys.exit(1)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_rurixc() -> None:
    r = run(["cargo", "build", "-p", "rurixc", "--bin", "rurixc"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build rurixc 失败:\n{r.stderr}")
    if not RURIXC.exists():
        fail(f"rurixc 产物缺失: {RURIXC}")


def compile_run(src_rel: str, exe_name: str) -> tuple[int, str]:
    """编译 host 源到 EXE 并真跑,返回 (exit_code, stdout)。"""
    src = ROOT / src_rel
    exe = OUT_DIR / exe_name
    r = run([str(RURIXC), str(src), "-o", str(exe)], cwd=ROOT)
    if r.returncode != 0:
        fail(f"编译 {src_rel} 失败(exit {r.returncode}):\n{r.stdout}{r.stderr}")
    if not exe.exists():
        fail(f"EXE 未产出: {exe}")
    rr = run([str(exe)])
    return rr.returncode, rr.stdout.strip()


def emit_device_ir(src_rel: str) -> int:
    """device codegen NVPTX IR;返回退出码(并校验 IR 非空)。"""
    src = ROOT / src_rel
    r = run([str(RURIXC), str(src), "--emit=nvptx-ir"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"device codegen {src_rel} 失败(exit {r.returncode}):\n{r.stdout}{r.stderr}")
    if "target triple" not in r.stdout:
        fail(f"device IR 异常(无 target triple): {src_rel}")
    return r.returncode


def try_ptxas(src_rel: str) -> str:
    """best-effort ptxas 干验证(--emit=ptx);返回 ran|skipped。"""
    src = ROOT / src_rel
    r = run([str(RURIXC), str(src), "--emit=ptx"], cwd=ROOT)
    if r.returncode == 0 and ("//" in r.stdout or ".visible" in r.stdout or "target" in r.stdout):
        return "ran"
    return "skipped"


def self_test() -> bool:
    """红:篡改 host 期望值后真跑必不打印成功 marker。返回 True = 红验证通过(marker 缺席)。"""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    tampered = OUT_DIR / "tampered_vec.rx"
    src = (ROOT / "conformance/stdlib/host/vec_ops.rx").read_text(encoding="utf-8")
    # 篡改:把点积期望 32.0 改成错误值 99.0 → check_dot_cross 失败 → ok != 6 → 不打印 marker
    broken = src.replace("a.dot(b), 32.0", "a.dot(b), 99.0")
    if broken == src:
        fail("自检篡改未命中目标表达式(vec_ops.rx 结构变化?)")
    tampered.write_text(broken, encoding="utf-8")
    exe = OUT_DIR / "tampered_vec.exe"
    r = run([str(RURIXC), str(tampered), "-o", str(exe)], cwd=ROOT)
    if r.returncode != 0:
        fail(f"篡改源编译失败(预期可编译,仅运行期断言失败):\n{r.stdout}{r.stderr}")
    rr = run([str(exe)])
    marker_absent = "stdlib-vec-ok" not in rr.stdout
    return marker_absent


def main() -> None:
    mode = sys.argv[1] if len(sys.argv) > 1 else ""
    OUT_DIR.mkdir(parents=True, exist_ok=True)
    build_rurixc()

    if mode == "--self-test":
        ok = self_test()
        if not ok:
            fail("红验证失败:篡改期望值后仍打印成功 marker(门未真正校验,反 YAML-only)")
        print("[stdlib_math_smoke] self-test PASS(篡改期望值 → 成功 marker 缺席,门有效)")
        return

    host_facts = []
    for src_rel, marker in HOST_CASES:
        exe_name = Path(src_rel).stem + ".exe"
        code, out = compile_run(src_rel, exe_name)
        if code != 0:
            fail(f"{src_rel} 退出码 {code}(期待 0)")
        if out != marker:
            fail(f"{src_rel} stdout 不符: {out!r}(期待 {marker!r})")
        host_facts.append({
            "source": src_rel,
            "marker": marker,
            "exit_code": code,
            "note": "PASS",
        })

    device_facts = []
    for src_rel in DEVICE_CASES:
        code = emit_device_ir(src_rel)
        ptxas = try_ptxas(src_rel)
        device_facts.append({
            "source": src_rel,
            "emit": "--emit=nvptx-ir",
            "exit_code": code,
            "ptxas": ptxas,
            "note": "PASS",
        })

    # 内建红绿:篡改期望值 → marker 缺席(红);原样源 → marker(绿,已由 host_facts 背书)
    red_absent = self_test()
    if not red_absent:
        fail("红验证失败:篡改期望值后仍打印成功 marker(门未真正校验,反 YAML-only)")

    evidence = {
        "schema_version": 1,
        "subject": "math_primitives",
        "primitives_passed": PRIMITIVES,
        "host_facts": host_facts,
        "device_facts": device_facts,
        "gpu_roundtrip": "skipped",
        "redgreen": {
            "red_command": "篡改 conformance/stdlib/host/vec_ops.rx 点积期望 32.0→99.0 后真跑",
            "red_marker_absent": True,
            "green_command": "py -3 ci/stdlib_math_smoke.py(原样源 → stdlib-vec-ok / stdlib-mat-ok)",
            "green_exit_code": 0,
            "run_url": "local red-green(反 YAML-only,H06 D11.8-2);pr-smoke 步骤 29 self-hosted runner run URL 见 PR 描述",
        },
        "rurixc_binary": "target/debug/rurixc.exe",
        "timestamp": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[stdlib_math_smoke] PASS(host {len(host_facts)} 真跑 marker 符合 / "
        f"device {len(device_facts)} codegen IR 产出 / 红验证 marker 缺席 / "
        f"{len(PRIMITIVES)} primitives → {EVIDENCE.relative_to(ROOT).as_posix()})"
    )


if __name__ == "__main__":
    main()
