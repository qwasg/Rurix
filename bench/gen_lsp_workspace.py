# -*- coding: utf-8 -*-
"""确定性生成 ~10k 行合法 .rx 样例工程(M6.5,契约 G-M6-2)。

LSP 10k 行交互延迟实测需要一个"代表性 10k 行工程"。为避免向仓库提交大文件,
本生成器在采样时把工程写入临时/build 目录;来源(生成器 commit + 模板规格 +
行数 + 函数数)写入 evidence/lsp_latency_*.json 留痕(BENCH_PROTOCOL 环境画像纪律)。

模板:零依赖前向调用链(只用已验证语法,见 conformance/toolchain/lsp_mvp/sample.rx):
  fn f0() -> i32 { 0 }
  fn fN() -> i32 { let v = f{N-1}(); v }   # let 调用 + 尾标识符,均为已验证构造
  fn main() { let anchor = f{LAST}(); let _ = anchor; }
每函数体含真实调用点,使 definition/completion 在 10k 行尺度上可解析、可计时。
生成完全确定(同 target_lines → 逐字节同输出),便于复采与趋势对齐。
"""
from __future__ import annotations

import argparse
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

DEFAULT_LINES = 10_000
HEADER = [
    "// AUTO-GENERATED LSP latency workspace (deterministic). DO NOT EDIT.",
    "// provenance: bench/gen_lsp_workspace.py — forwarding-call fn chain.",
]


def git_commit() -> str:
    out = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=ROOT, capture_output=True, text=True, check=False,
    )
    return out.stdout.strip() or "unknown"


def build_lines(target_lines: int) -> tuple[list[str], int, dict]:
    """构造 .rx 行列表,直到 >= target_lines 后收口 main;返回 (lines, n_functions, anchors)。

    anchors 为 0-based LSP 坐标(line/character),指向 main 中 `f{LAST}` 调用点 token,
    供 bench 发 completion/definition 请求定位。
    """
    lines: list[str] = list(HEADER)
    # f0:字面量尾表达式(与 helper() 同构)
    lines += ["fn f0() -> i32 {", "    0", "}"]
    n = 1
    # 预留 main 收口 4 行 + 收口前留 1 行余量
    while len(lines) < target_lines - 5:
        lines += [
            f"fn f{n}() -> i32 {{",
            f"    let v = f{n - 1}();",
            "    v",
            "}",
        ]
        n += 1
    last = n - 1
    call_line = f"    let anchor = f{last}();"
    main_start = len(lines)
    lines += [
        "fn main() {",
        call_line,
        "    let _ = anchor;",
        "}",
    ]
    anchor_line_idx = main_start + 1  # call_line 的 0-based 行号
    anchor_char = call_line.index(f"f{last}")  # `f{last}` token 起始列
    anchors = {
        "completion": {"line": anchor_line_idx, "character": anchor_char},
        "definition": {"line": anchor_line_idx, "character": anchor_char},
    }
    return lines, n, anchors


def generate(target_lines: int, out_path: Path) -> dict:
    """生成工程写入 out_path;返回来源/锚点元数据(供 evidence 留痕)。"""
    lines, n_functions, anchors = build_lines(target_lines)
    text = "\n".join(lines) + "\n"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    out_path.write_text(text, encoding="utf-8")
    return {
        "path": out_path,
        "lines": len(lines),
        "n_functions": n_functions,
        "anchors": anchors,
        "generator": f"bench/gen_lsp_workspace.py@{git_commit()} "
                     f"(forwarding-call fn chain; deterministic; target={target_lines})",
        "template_spec": "fn f0(){0}; fn fN(){let v=f{N-1}(); v}; fn main(){let anchor=f{LAST}();}",
    }


def main() -> int:
    ap = argparse.ArgumentParser(description="生成确定性 10k 行 LSP 样例工程")
    ap.add_argument("--lines", type=int, default=DEFAULT_LINES, help="目标行数(默认 10000)")
    ap.add_argument("--out", default="build/lsp_latency/workspace_10k.rx",
                    help="输出路径(默认 build/lsp_latency/workspace_10k.rx)")
    args = ap.parse_args()
    meta = generate(args.lines, ROOT / args.out)
    print(f"[gen_lsp_workspace] {meta['lines']} lines, {meta['n_functions']} fns -> {args.out}")
    print(f"[gen_lsp_workspace] anchors={meta['anchors']}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
