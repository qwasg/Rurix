#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""统一子进程看门狗包装(P0-7 nightly 僵尸进程根治;EA1 契约外并行轨道)。

背景:CI/bench 在 nightly 路径上用 subprocess 跑「编译出来的 exe」而无 timeout,
一旦 exe 内核态卡死则永久挂起;GitHub 的 job 级 `timeout-minutes` 只 cancel job、
无法回收内核态卡死的孙 exe,于是僵尸 exe 锁死 self-hosted runner(H:\\actions-runner),
需手动 `Move-Item` 隔离止血(2026-07-17 async_buffer_pipeline.exe 事故)。

本模块提供 stdlib-only 的 `guarded_run`:
  1. 分级超时:调用方按类型给额度(sanitizer 大额、exe 运行中额、cargo build 大额)。
  2. 超时杀进程树:`taskkill /PID <pid> /T /F`(/T 递归杀 cargo→exe、
     sanitizer→python→exe 的孙进程;零第三方依赖,与手动止血 Move-Item 同族)。
  3. 杀不掉时兜底解锁:把目标 exe `shutil.move` 到隔离目录(gitignore 区
     build/quarantine/;Windows 允许 rename 运行中的 exe,即使 delete 失败也能解锁路径),
     并输出隔离路径。
  4. 诚实红:超时 = 非零退出码(124,GNU timeout 约定),绝不 SKIP 充绿。
  5. 超时打印进程树快照(pid/name):经 Toolhelp32 快照(Windows)best-effort。

**为何用 Popen 而非 subprocess.run(timeout=)**:Windows 上 `run(timeout=)` 超时只
`TerminateProcess` 直接子进程,孙 exe 存活成僵尸(见 pitfalls);必须 Popen +
`taskkill /T` 按 PID 杀整树。次序硬要点:**先杀树、再收尾 communicate(短 timeout)**——
孙进程继承 stdout pipe 时,即便直接子进程已死 communicate 仍可能挂在读 pipe。

CLI(供 nightly.yml PowerShell 步骤路由 rx.exe / rurixc.exe,无需改 Rust):
  py -3 bench/proc_guard.py --timeout 1200 -- <cmd> [args...]
  py -3 bench/proc_guard.py --timeout 900 --quarantine-exe target/debug/rx.exe -- <cmd...>
"""
from __future__ import annotations

import datetime
import os
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 超时 = 诚实红的退出码(GNU coreutils `timeout` 约定 124;绝不 0/SKIP)。
TIMEOUT_EXIT = 124

# 分级超时默认额度(秒);调用方可覆盖。sanitizer 显著拖慢 kernel → 大额度。
DEFAULT_TIMEOUT = 900          # 15 min:一般兜底
EXE_RUN_TIMEOUT = 300          # 5 min:跑编译出的 bench exe(smoke 档)
CARGO_BUILD_TIMEOUT = 1800     # 30 min:cargo build(冷编译留余量)
SANITIZER_TIMEOUT = 1200       # 20 min:单 (tool, kernel) 组合(sanitizer 拖慢)


class GuardedResult:
    """subprocess.CompletedProcess 近似形态 + 看门狗附加字段。

    returncode / stdout / stderr 兼容原调用点的读取;timed_out / quarantined 供
    调用方与测试断言(超时是否触发、隔离了哪些 exe)。
    """

    __slots__ = ("returncode", "stdout", "stderr", "timed_out", "quarantined")

    def __init__(self, returncode, stdout, stderr, timed_out, quarantined):
        self.returncode = returncode
        self.stdout = stdout
        self.stderr = stderr
        self.timed_out = timed_out
        self.quarantined = quarantined


def _dec(b) -> str:
    if b is None:
        return ""
    if isinstance(b, str):
        return b
    return b.decode("utf-8", errors="replace")


def _as_list(v):
    if v is None:
        return []
    if isinstance(v, (list, tuple)):
        return [x for x in v if x is not None]
    return [v]


# ---------------------------------------------------------------------------
# 进程树快照(诊断;实际杀树靠 taskkill /T,快照失败只降级为「快照不可用」)
# ---------------------------------------------------------------------------
def _win_all_processes():
    """经 Toolhelp32 枚举全部进程 → [(pid, ppid, name), ...](stdlib ctypes,无外部依赖)。"""
    import ctypes
    from ctypes import wintypes

    TH32CS_SNAPPROCESS = 0x00000002
    INVALID = ctypes.c_void_p(-1).value

    class PROCESSENTRY32(ctypes.Structure):
        _fields_ = [
            ("dwSize", wintypes.DWORD),
            ("cntUsage", wintypes.DWORD),
            ("th32ProcessID", wintypes.DWORD),
            ("th32DefaultHeapID", ctypes.c_size_t),
            ("th32ModuleID", wintypes.DWORD),
            ("cntThreads", wintypes.DWORD),
            ("th32ParentProcessID", wintypes.DWORD),
            ("pcPriClassBase", ctypes.c_long),
            ("dwFlags", wintypes.DWORD),
            ("szExeFile", ctypes.c_char * 260),
        ]

    k32 = ctypes.windll.kernel32
    k32.CreateToolhelp32Snapshot.restype = wintypes.HANDLE
    k32.CreateToolhelp32Snapshot.argtypes = [wintypes.DWORD, wintypes.DWORD]
    k32.Process32First.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32)]
    k32.Process32Next.argtypes = [wintypes.HANDLE, ctypes.POINTER(PROCESSENTRY32)]
    k32.CloseHandle.argtypes = [wintypes.HANDLE]

    snap = k32.CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
    if not snap or snap == INVALID:
        return []
    procs = []
    try:
        entry = PROCESSENTRY32()
        entry.dwSize = ctypes.sizeof(PROCESSENTRY32)
        ok = k32.Process32First(snap, ctypes.byref(entry))
        while ok:
            name = entry.szExeFile.decode("mbcs", "replace")
            procs.append((int(entry.th32ProcessID), int(entry.th32ParentProcessID), name))
            ok = k32.Process32Next(snap, ctypes.byref(entry))
    finally:
        k32.CloseHandle(snap)
    return procs


def _descendants(root_pid, procs):
    """从 (pid, ppid, name) 平铺表提取 root_pid 的后代(含自身),BFS。"""
    children = {}
    names = {}
    for pid, ppid, name in procs:
        children.setdefault(ppid, []).append(pid)
        names[pid] = name
    out = []
    seen = set()
    stack = [root_pid]
    while stack:
        pid = stack.pop()
        if pid in seen:
            continue
        seen.add(pid)
        out.append((pid, names.get(pid, "?")))
        stack.extend(children.get(pid, []))
    return out


def _snapshot_tree(root_pid) -> str:
    """root_pid 进程树快照文本(pid/name);任何失败降级为单行说明,不抛出。"""
    try:
        if os.name == "nt":
            procs = _win_all_processes()
            if not procs:
                return f"[proc_guard] process tree snapshot unavailable (pid={root_pid})"
            tree = _descendants(root_pid, procs)
            lines = [f"[proc_guard] process tree of pid={root_pid} ({len(tree)} procs):"]
            for pid, name in tree:
                lines.append(f"[proc_guard]   pid={pid} name={name}")
            return "\n".join(lines)
        # POSIX best-effort
        r = subprocess.run(["ps", "-o", "pid,ppid,comm"], capture_output=True,
                           text=True, timeout=10)
        procs = []
        for ln in r.stdout.splitlines()[1:]:
            parts = ln.split(None, 2)
            if len(parts) >= 3 and parts[0].isdigit() and parts[1].isdigit():
                procs.append((int(parts[0]), int(parts[1]), parts[2]))
        tree = _descendants(root_pid, procs)
        return f"[proc_guard] process tree of pid={root_pid}: " + \
               ", ".join(f"{pid}:{name}" for pid, name in tree)
    except Exception as e:  # 诊断绝不影响止血主路径
        return f"[proc_guard] process tree snapshot failed (pid={root_pid}): {e!r}"


# ---------------------------------------------------------------------------
# 杀进程树 + 隔离
# ---------------------------------------------------------------------------
def _kill_tree(pid) -> None:
    """按 PID 杀整棵进程树(Windows: taskkill /T /F;POSIX: killpg)。"""
    if os.name == "nt":
        try:
            subprocess.run(["taskkill", "/PID", str(pid), "/T", "/F"],
                           capture_output=True, timeout=30, check=False)
        except Exception as e:
            sys.stderr.write(f"[proc_guard] taskkill failed for pid={pid}: {e!r}\n")
    else:
        import signal
        try:
            os.killpg(os.getpgid(pid), signal.SIGKILL)
        except Exception as e:  # ProcessLookupError / PermissionError 等
            sys.stderr.write(f"[proc_guard] killpg failed for pid={pid}: {e!r}\n")


def _quarantine(exe_paths, quarantine_dir) -> list:
    """把仍存在的目标 exe move 到隔离目录(带时间戳防覆盖);返回隔离后的绝对路径列表。

    镜像手动止血 Move-Item:即便 taskkill 杀不掉内核态卡死的 exe,rename/move 仍能
    解锁原路径(Windows 允许 move 运行中的 exe)。隔离目录默认 build/quarantine/
    (.gitignore 忽略 build/,不会误 git-add 僵尸二进制)。
    """
    moved = []
    exe_paths = [Path(p) for p in exe_paths]
    exe_paths = [p for p in exe_paths if p.exists()]
    if not exe_paths:
        return moved
    quarantine_dir = Path(quarantine_dir)
    quarantine_dir.mkdir(parents=True, exist_ok=True)
    ts = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    for p in exe_paths:
        dest = quarantine_dir / f"{p.stem}_{ts}{p.suffix}"
        n = 1
        while dest.exists():
            dest = quarantine_dir / f"{p.stem}_{ts}_{n}{p.suffix}"
            n += 1
        try:
            shutil.move(str(p), str(dest))
            moved.append(str(dest))
        except Exception as e:
            sys.stderr.write(f"[proc_guard] quarantine move failed for {p}: {e!r}\n")
    return moved


# ---------------------------------------------------------------------------
# 核心
# ---------------------------------------------------------------------------
def guarded_run(cmd, *, timeout=DEFAULT_TIMEOUT, cwd=None, capture=True,
                quarantine_exe=None, quarantine_dir=None, label=None, env=None):
    """跑子进程,带看门狗超时 + 杀进程树 + 隔离 + 诚实红。

    参数:
      cmd            命令 argv(元素会 str() 化)。
      timeout        秒;超时触发杀树 + 隔离。
      cwd            工作目录。
      capture        True=捕获 stdout/stderr(返回 str);False=继承(直出控制台,
                     用于 cargo build 保留实时进度)。
      quarantine_exe 超时后要隔离的 exe 路径(单个或列表);None=只杀树不隔离。
      quarantine_dir 隔离目录(默认 ROOT/build/quarantine;测试传临时目录)。
      label          日志标签(默认取 cmd 前段)。
      env            子进程环境。

    返回 GuardedResult。超时 → returncode=124(TIMEOUT_EXIT,诚实红),timed_out=True。
    """
    cmd = [str(c) for c in cmd]
    if quarantine_dir is None:
        quarantine_dir = ROOT / "build" / "quarantine"
    exe_list = _as_list(quarantine_exe)
    tag = label or " ".join(cmd[:3])

    popen_kw = {"cwd": (str(cwd) if cwd else None), "env": env}
    if os.name != "nt":
        popen_kw["start_new_session"] = True  # 建独立进程组,便于 killpg
    if capture:
        popen_kw["stdout"] = subprocess.PIPE
        popen_kw["stderr"] = subprocess.PIPE

    proc = subprocess.Popen(cmd, **popen_kw)
    try:
        out_b, err_b = proc.communicate(timeout=timeout)
        return GuardedResult(proc.returncode, _dec(out_b), _dec(err_b), False, [])
    except subprocess.TimeoutExpired:
        sys.stderr.write(f"\n[proc_guard] TIMEOUT after {timeout}s: {tag}\n")
        sys.stderr.write(_snapshot_tree(proc.pid) + "\n")
        # 硬次序:先杀树,再收尾 communicate(短 timeout)——孙进程死后 pipe 关闭才不挂
        _kill_tree(proc.pid)
        out_b = err_b = b""
        for settle in (30, 10):
            try:
                out_b, err_b = proc.communicate(timeout=settle)
                break
            except subprocess.TimeoutExpired:
                _kill_tree(proc.pid)  # 再补一刀
        quarantined = _quarantine(exe_list, quarantine_dir)
        for q in quarantined:
            sys.stderr.write(f"[proc_guard] quarantined stuck exe -> {q}\n")
        note = (f"[proc_guard] {tag}: TIMEOUT {timeout}s -> killed process tree "
                f"(pid={proc.pid}), quarantined={len(quarantined)} exe(s); "
                f"honest RED (exit {TIMEOUT_EXIT}, NOT skip)")
        sys.stderr.write(note + "\n")
        return GuardedResult(TIMEOUT_EXIT, _dec(out_b), _dec(err_b) + "\n" + note,
                             True, quarantined)


# ---------------------------------------------------------------------------
# CLI:供 nightly.yml PowerShell 步骤路由 rx.exe / rurixc.exe(无需改 Rust)
# ---------------------------------------------------------------------------
def _cli(argv) -> int:
    timeout = DEFAULT_TIMEOUT
    quarantine = []
    rest = None
    i = 0
    while i < len(argv):
        a = argv[i]
        if a == "--":
            rest = argv[i + 1:]
            break
        if a == "--timeout":
            timeout = float(argv[i + 1]); i += 2; continue
        if a.startswith("--timeout="):
            timeout = float(a.split("=", 1)[1]); i += 1; continue
        if a == "--quarantine-exe":
            quarantine.append(argv[i + 1]); i += 2; continue
        if a.startswith("--quarantine-exe="):
            quarantine.append(a.split("=", 1)[1]); i += 1; continue
        i += 1
    if not rest:
        sys.stderr.write(
            "usage: py -3 bench/proc_guard.py [--timeout N] "
            "[--quarantine-exe PATH]... -- <cmd> [args...]\n")
        return 2
    res = guarded_run(rest, timeout=timeout, capture=False,
                      quarantine_exe=(quarantine or None),
                      label=" ".join(rest[:3]))
    return res.returncode


if __name__ == "__main__":
    sys.exit(_cli(sys.argv[1:]))
