#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""着色阶段条款 / 类型面拦截冒烟(G2.1 CI_GATES 步骤 45,RFC-0002;G-G2-1,
纯 host/CPU-only,check_ 守卫风格,反 YAML-only)。

经 rurixc 前端(cargo feature `shader-stages`,默认启用)对着色阶段类型面违例做
**真实红绿**核对:用 `rurixc --emit=check`(全量静态检查,无 codegen/link)对临时
`.rx` 样例逐一编译,断言——

- **green**:合法着色阶段声明 + 阶段专属 I/O 标注 + 共享接口 + Texture2D/Sampler
  作 fragment 形参 → 退出 0、零诊断;
- **red**:着色阶段误用(直接调用入口 → RX3001)/ 无标注 I/O 字段(RX3011)/ 阶段间
  接口不匹配(RX3012)/ 资源句柄违例(RX3013)→ 退出非零且 stderr 含对应错误码。

内置 red 自检:对 green 样例注入一处违例,断言由绿翻红——证明门真的在拦截(非空过)。
DXIL codegen(G2.2)/ 绑定布局推导(G2.3)/ 🔒 纹理内存模型映射(06 §4.2 禁区)不在
本 PR(纯类型面/语法面);本冒烟纯 host 编译期,无 device/GPU。
"""
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

GREEN_SHADER = """\
//@ spec: RXS-0153
struct VsOut {
    #[builtin(position)] pos: f32,
    #[interpolate(perspective)] uv: f32,
}
vertex fn vs_main() -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }
fragment fn fs_main(inp: VsOut, tex: Texture2D<f32>, samp: Sampler) -> VsOut { inp }
compute fn cs_main() {}
fn main() {}
"""

# (名称, 源, 期望退出 0=green, 期望错误码或 None)
CASES = [
    ("green_legal_stages", GREEN_SHADER, 0, None),
    (
        "red_direct_stage_call",
        "//@ spec: RXS-0153\nvertex fn vs_main() {}\nfn main() { vs_main(); }\n",
        1,
        "RX3001",
    ),
    (
        "red_unannotated_io_field",
        "//@ spec: RXS-0154\nstruct VsOut { #[builtin(position)] pos: f32, uv: f32 }\n"
        "vertex fn vs_main() -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\nfn main() {}\n",
        1,
        "RX3011",
    ),
    (
        "red_interface_mismatch",
        "//@ spec: RXS-0155\n"
        "struct VsOut { #[builtin(position)] pos: f32, #[interpolate(perspective)] uv: f32 }\n"
        "struct FsIn { #[interpolate(perspective)] color: f32 }\n"
        "vertex fn vs_main() -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\n"
        "fragment fn fs_main(inp: FsIn) -> VsOut { VsOut { pos: 0.0, uv: 0.0 } }\nfn main() {}\n",
        1,
        "RX3012",
    ),
    (
        "red_resource_handle",
        "//@ spec: RXS-0156\nfragment fn fs_main() -> Texture2D<f32> { }\nfn main() {}\n",
        1,
        "RX3013",
    ),
]


def fail(msg):
    print(f"[shader_stages] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def build_rurixc():
    print("[shader_stages] cargo build -p rurixc (feature shader-stages, default)")
    r = subprocess.run(
        ["cargo", "build", "-p", "rurixc", "--quiet"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if r.returncode != 0:
        fail(f"cargo build -p rurixc 失败:\n{r.stdout}\n{r.stderr}")
    exe = ROOT / "target" / "debug" / ("rurixc.exe" if sys.platform == "win32" else "rurixc")
    if not exe.is_file():
        fail(f"rurixc 产物缺失: {exe}")
    return exe


def run_check(exe, src):
    """rurixc --emit=check <tmp>;返回 (exit_code, stderr)。"""
    with tempfile.TemporaryDirectory() as d:
        f = Path(d) / "case.rx"
        f.write_text(src, encoding="utf-8")
        r = subprocess.run(
            [str(exe), "--emit=check", str(f)],
            cwd=ROOT, capture_output=True, text=True,
        )
        return r.returncode, r.stderr


def check_case(exe, name, src, want_exit, want_code):
    code, stderr = run_check(exe, src)
    is_green = code == 0
    if want_exit == 0:
        if not is_green:
            fail(f"{name}: 期望 green(0 诊断)却退出 {code}\n{stderr}")
    else:
        if is_green:
            fail(f"{name}: 期望 red(拦截)却退出 0(违例被放行,反 YAML-only)")
        if want_code and want_code not in stderr:
            fail(f"{name}: 期望诊断码 {want_code} 未出现\n{stderr}")
    print(f"[shader_stages] OK {name} (exit={code}{', ' + want_code if want_code else ''})")


def main():
    exe = build_rurixc()
    for name, src, want_exit, want_code in CASES:
        check_case(exe, name, src, want_exit, want_code)

    # red 自检(反 YAML-only):green 样例注入一处无标注字段 → 必须由绿翻红 RX3011。
    injected = GREEN_SHADER.replace(
        "#[interpolate(perspective)] uv: f32,", "uv: f32,"
    )
    code, stderr = run_check(exe, injected)
    if code == 0 or "RX3011" not in stderr:
        fail("red 自检失败:green 样例注入无标注 I/O 字段后未翻红 RX3011(门空过)")
    print("[shader_stages] OK red-self-test (green→inject→RX3011 red)")

    print("[shader_stages] PASS (着色阶段类型面拦截真实红绿:RX3001/3011/3012/3013 + green)")
    sys.exit(0)


if __name__ == "__main__":
    main()
