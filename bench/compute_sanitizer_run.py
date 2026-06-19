"""Compute Sanitizer 表驱动隔离运行器(M5.4 第 3 步,契约 G-M5-4)。

用 `compute-sanitizer --tool racecheck|memcheck` 包裹各 kernel 的 `--smoke` 真跑,
每 (tool, kernel) 组合起独立子进程(隔离;崩溃不连坐 harness,14 §6 / M5_PLAN §5),
解析退出码与 SUMMARY → 产 `evidence/compute_sanitizer_<tool>_<kernel>_<yyyymmdd>.json`
(clean=true/false + 摘要)。

**纪律**:Sanitizer 只跑「正确性」维度,显著拖慢 kernel —— 不锁频、不做性能采样,
严禁污染 measured 基准(M5_PLAN §5)。

工具定位:禁硬编码版本文件名(沿用 M4 CI_GATES §1 r6 教训),经环境变量 / where /
CUDA 安装目录探测,优先 compute-sanitizer.exe(避免 .bat + 含空格路径的 cmd 引号陷阱)。

用法:
  py -3 bench/compute_sanitizer_run.py                       # 全 kernel × racecheck+memcheck
  py -3 bench/compute_sanitizer_run.py --kernel reduce       # 子集(可多次)
  py -3 bench/compute_sanitizer_run.py --tool racecheck      # 单 tool
  py -3 bench/compute_sanitizer_run.py --fixture race        # 红绿验证:red(应检出竞争)
  py -3 bench/compute_sanitizer_run.py --fixture clean       # 红绿验证:green(应转绿)
"""
from __future__ import annotations

import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT))

# kernel 干名 → 包裹的真跑命令(相对 ROOT;均为 --smoke 闭环,return 0=成功)
KERNELS: dict[str, list[str]] = {
    "reduce": ["bench/reduce_bench.py", "--smoke"],
    "scan": ["bench/scan_bench.py", "--smoke"],
    "transpose": ["bench/transpose_bench.py", "--smoke"],
    "gemm_tile": ["bench/gemm_tile_bench.py", "--smoke"],
    "saxpy": ["bench/saxpy_bench.py", "--smoke"],  # M4 SAXPY 回归
    # M7.3 G0 软光栅 device kernel(全 safe,atomics-free;spec/softraster.md
    # RXS-0118~0121,D-M7-3)纳入既有 Compute Sanitizer nightly(M5.4 机制延续,
    # M7 CI_GATES §4 第 5 项;激活经真实 GPU 验证)
    "sr_binning": ["bench/sr_binning_bench.py", "--smoke"],
    "sr_raster_tile": ["bench/sr_raster_tile_bench.py", "--smoke"],
    "sr_depth": ["bench/sr_depth_bench.py", "--smoke"],
    "sr_tonemap": ["bench/sr_tonemap_bench.py", "--smoke"],
    # M8.3 UC-02 三 stream 重叠 + 跨线程所有权转移 device 路径(spec/pipeline.md
    # RXS-0130~0134,D-M8-3)纳入既有 Compute Sanitizer nightly(M5.4 机制延续,
    # M8 CI_GATES §4 / M8_CONTRACT §5;运行期无数据竞争佐证编译期拦截;经 --target-processes
    # all 跟随 uc02-demo 子 exe)
    "uc02_stream": ["bench/uc02_stream_bench.py", "--smoke"],
    # G1.2 流序分配 AsyncBuffer device 路径(spec/async_buffer.md RXS-0144~0148,MR-0001)
    # 纳入既有 Compute Sanitizer nightly(M5.4 机制延续,G1 CI_GATES §4;CUDA.jl #780
    # use-after-free 事故类**永久回归项**;经 --target-processes all 跟随 async_buffer_pipeline
    # 示例 exe 的流序分配 cuMemAllocAsync / 拷贝 / cuMemFreeAsync)
    "async_buffer": ["bench/async_buffer_bench.py", "--smoke"],
}
TOOLS = ("racecheck", "memcheck")

# 红绿验证夹具(非生产 kernel 目录):race 应被检出竞争,clean(补 bar.sync)应转绿
FIXTURES: dict[str, list[str]] = {
    "race": ["bench/sanitizer_fixtures/fixture_runner.py", "--variant", "race"],
    "clean": ["bench/sanitizer_fixtures/fixture_runner.py", "--variant", "clean"],
}


def git_commit() -> str:
    out = subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT,
                         capture_output=True, text=True, check=False)
    return out.stdout.strip() or "unknown"


def find_sanitizer() -> str:
    """定位 compute-sanitizer 可执行文件(禁硬编码版本号,r6 教训)。

    顺序:COMPUTE_SANITIZER 环境变量 → CUDA 安装目录探测(优先 .exe)→ PATH。
    """
    env = os.environ.get("COMPUTE_SANITIZER")
    if env and Path(env).is_file():
        return env

    roots: list[Path] = []
    seen: set[str] = set()

    def add_root(p: Path | None) -> None:
        if p and str(p) not in seen and p.is_dir():
            seen.add(str(p))
            roots.append(p)

    cuda_path = os.environ.get("CUDA_PATH")
    if cuda_path:
        add_root(Path(cuda_path))
    for pf_var in ("ProgramFiles", "ProgramW6432"):
        pf = os.environ.get(pf_var)
        if pf:
            base = Path(pf) / "NVIDIA GPU Computing Toolkit" / "CUDA"
            # 版本目录倒序(新版优先);不写死具体版本号
            if base.is_dir():
                for vdir in sorted(base.glob("v*"), reverse=True):
                    add_root(vdir)
    nvcc = shutil.which("nvcc")
    if nvcc:
        add_root(Path(nvcc).resolve().parent.parent)  # <toolkit>/bin/nvcc -> <toolkit>

    exe_rels = (
        "compute-sanitizer/compute-sanitizer.exe",
        "extras/compute-sanitizer/compute-sanitizer.exe",
        "bin/compute-sanitizer.exe",
    )
    bat_rels = ("bin/compute-sanitizer.bat",)
    for rels in (exe_rels, bat_rels):  # .exe 优先于 .bat
        for r in roots:
            for rel in rels:
                cand = r / rel
                if cand.is_file():
                    return str(cand)

    which = shutil.which("compute-sanitizer")
    if which:
        return which
    raise FileNotFoundError(
        "compute-sanitizer 未找到:设置环境变量 COMPUTE_SANITIZER=<完整路径>,"
        "或安装 CUDA Toolkit(compute-sanitizer 随 Toolkit 分发)"
    )


def _wrap(sanitizer: str, sani_args: list[str]) -> list[str]:
    """组装调用命令;.bat 经 COMSPEC /c 调起(.exe 直接 CreateProcess)。"""
    if sanitizer.lower().endswith(".bat"):
        comspec = os.environ.get("COMSPEC", "cmd.exe")
        return [comspec, "/c", sanitizer, *sani_args]
    return [sanitizer, *sani_args]


def sanitizer_version(sanitizer: str) -> str:
    try:
        proc = subprocess.run(_wrap(sanitizer, ["--version"]), cwd=ROOT,
                              capture_output=True, text=True, timeout=60)
        for line in (proc.stdout + proc.stderr).splitlines():
            if line.strip():
                return line.strip()
    except Exception:
        pass
    return "unavailable"


_SUMMARY_KEYS = ("SUMMARY", "hazard", "error", "race", "leak")


def _extract_summary(output: str) -> str:
    lines = []
    for line in output.splitlines():
        low = line.lower()
        if "=========" in line and any(k.lower() in low for k in _SUMMARY_KEYS):
            lines.append(line.strip())
    text = "\n".join(lines)
    if not text:
        text = "no sanitizer diagnostics captured"
    return text[:4000]


def run_one(sanitizer: str, version: str, tool: str, label: str,
            target_cmd: list[str], out_dir: Path) -> dict:
    """跑一个 (tool, label) 组合,产证据 dict 并写盘。"""
    abs_cmd = [sys.executable, *[str(ROOT / a) if a.endswith(".py") else a for a in target_cmd]]
    # --target-processes all:跟随子进程(M8.3 UC-02 device 路径 kernel/拷贝在 uc02-demo
    # 子 exe 内执行,须随进程检查;对既有 in-process python ctypes kernel 为无害扩展——
    # 它们不 spawn GPU 子进程,行为不变)。
    sani_args = ["--tool", tool, "--target-processes", "all", "--error-exitcode", "1",
                 "--", *abs_cmd]
    cmd = _wrap(sanitizer, sani_args)
    print(f"\n[sanitizer] >>> --tool {tool} -- {' '.join(target_cmd)}")
    proc = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    output = (proc.stdout or "") + "\n" + (proc.stderr or "")

    errors = 0
    m = re.search(r"ERROR SUMMARY:\s*(\d+)\s+error", output)
    if m:
        errors = int(m.group(1))
    hazards = 0
    mh = re.search(r"RACECHECK SUMMARY:\s*(\d+)\s+hazard", output)
    if mh:
        hazards = int(mh.group(1))
    # racecheck 在括号内也报 error 计数:"(N errors, M warnings)"
    mr = re.search(r"RACECHECK SUMMARY:.*?\((\d+)\s+error", output)
    if mr:
        errors = max(errors, int(mr.group(1)))

    clean = (proc.returncode == 0) and (errors == 0) and (hazards == 0)
    summary = _extract_summary(output)
    print(f"[sanitizer] {tool}/{label}: exit={proc.returncode} errors={errors} "
          f"hazards={hazards} -> clean={clean}")
    print(f"[sanitizer]   {summary.splitlines()[0] if summary else ''}")

    doc = {
        "schema_version": 1,
        "tool": tool,
        "kernel": label,
        "clean": clean,
        "exit_code": proc.returncode,
        "errors_count": errors,
        "hazards_count": hazards,
        "report_summary": summary,
        "sanitizer_version": version,
        "command": " ".join(target_cmd),
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "harness_commit": git_commit(),
    }
    date = datetime.datetime.now().strftime("%Y%m%d")
    out_path = out_dir / f"compute_sanitizer_{tool}_{label}_{date}.json"
    out_dir.mkdir(parents=True, exist_ok=True)
    out_path.write_text(json.dumps(doc, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(f"[sanitizer] evidence written: {out_path.relative_to(ROOT)} (clean={clean})")
    return doc


def main() -> int:
    ap = argparse.ArgumentParser(description="Compute Sanitizer 隔离运行器(M5.4 G-M5-4)")
    ap.add_argument("--kernel", action="append", choices=sorted(KERNELS),
                    help="只跑指定 kernel(可多次);默认全跑")
    ap.add_argument("--tool", action="append", choices=list(TOOLS),
                    help="只跑指定 tool(可多次);默认 racecheck+memcheck")
    ap.add_argument("--fixture", choices=sorted(FIXTURES),
                    help="红绿验证夹具:race(应报红)/ clean(应转绿),仅 racecheck")
    ap.add_argument("--out-dir", default="evidence", help="证据输出目录(默认 evidence/)")
    args = ap.parse_args()

    out_dir = (ROOT / args.out_dir) if not os.path.isabs(args.out_dir) else Path(args.out_dir)
    sanitizer = find_sanitizer()
    version = sanitizer_version(sanitizer)
    print(f"[sanitizer] tool path: {sanitizer}")
    print(f"[sanitizer] version: {version}")

    if args.fixture:
        # 红绿验证:race 期望 clean=false(检出竞争),clean 期望 clean=true
        label = f"fixture-{args.fixture}"
        doc = run_one(sanitizer, version, "racecheck", label,
                      FIXTURES[args.fixture], out_dir)
        if args.fixture == "race":
            ok = doc["clean"] is False
            verdict = "红态达标(检出竞争)" if ok else "红态未达标(竞争未被检出!)"
        else:
            ok = doc["clean"] is True
            verdict = "绿态达标(全 clean)" if ok else "绿态未达标(仍有竞争!)"
        print(f"\n[sanitizer] 红绿验证 {label}: {verdict}")
        return 0 if ok else 1

    kernels = args.kernel or sorted(KERNELS)
    tools = args.tool or list(TOOLS)
    docs = []
    for tool in tools:
        for kernel in kernels:
            docs.append(run_one(sanitizer, version, tool, kernel, KERNELS[kernel], out_dir))

    dirty = [f"{d['tool']}/{d['kernel']}" for d in docs if not d["clean"]]
    clean_n = sum(1 for d in docs if d["clean"])
    print(f"\n[sanitizer] 汇总:{clean_n}/{len(docs)} clean")
    if dirty:
        print(f"[sanitizer] FAIL(非 clean):{', '.join(dirty)}", file=sys.stderr)
        return 1
    print("[sanitizer] 全部 clean")
    return 0


if __name__ == "__main__":
    sys.exit(main())
