#!/usr/bin/env python3
"""本机 GPU/构建互斥锁(check_* 类未编号守卫,不占数字 CI 步骤)。

G9 蜂群纪律:单张 RTX 4070 Ti,多个并行 agent 的 device 真跑腿与 cargo
构建/测试必须串行(并行 cargo 会二进制互覆盖假红;并行 device 提交会互相
污染 measured 数字)。本模块提供进程级排他锁。

实现:Windows ``msvcrt.locking`` 跨进程字节锁 + 进程内 ``threading.Lock``
兜底(``msvcrt.locking`` 对同一进程内重复加锁不互斥,必须自兜)。锁文件 =
``%TEMP%\\rurix-gpu-device.lock``(跨 checkout/worktree 全局)。锁字节选在
固定偏移 0;为防「文件为空时锁范围外立即成功」,首次创建时先写入哨兵行
``rurix-gpu-device-lock v1``,持锁期间第二行写持有者自述。

* acquire 打印持有者提示与等待时长;with-lock 段内第一条输出须自报
  ``[gpu_device_lock] holder=...``,供编排者排 device 时刻表;
* ``--selftest`` 用独立临时锁文件证明:同进程双臂互斥、子进程跨进程互斥、
  release 后可再得——三臂全绿才算能红能绿。

用法(库)::

    from gpu_device_lock import gpu_device_lock
    with gpu_device_lock(purpose="g9_m94 device 腿"):
        ...  # device 真跑 / cargo 构建

用法(CLI 自检)::

    py -3 ci/gpu_device_lock.py --selftest
"""

from __future__ import annotations

import argparse
import os
import subprocess
import sys
import tempfile
import threading
import time
from contextlib import contextmanager
from pathlib import Path

if sys.platform == "win32":
    import msvcrt
else:  # pragma: no cover - 本仓 CI/本机均 Windows;POSIX 仅为可读性兜底
    msvcrt = None  # type: ignore[assignment]

LOCK_PATH = Path(tempfile.gettempdir()) / "rurix-gpu-device.lock"
_SENTINEL = b"rurix-gpu-device-lock v1\n"

# G37 收官战役健壮性补面(day_0828 HANDOVER §7 兑现):
# ① 解锁/写回段偶发 PermissionError → 有限重试,终败不抛(锁随句柄 close 释放);
# ② stale holder:持有者 pid 已死但锁仍不可得(句柄泄漏面)→ 宽限期后回收锁文件。
_UNLOCK_RETRIES = 3
_UNLOCK_RETRY_S = 0.2
_STALE_GRACE_S = 30.0


def _pid_alive(pid: int) -> bool | None:
    """Windows 下判定 pid 是否存活;非 win32 或探测失败返回 None(不可判)。"""
    if sys.platform != "win32" or pid <= 0:
        return None
    try:
        import ctypes

        PROCESS_QUERY_LIMITED_INFORMATION = 0x1000
        STILL_ACTIVE = 259
        kernel32 = ctypes.windll.kernel32
        h = kernel32.OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, False, int(pid))
        if not h:
            return False
        try:
            code = ctypes.c_ulong()
            if kernel32.GetExitCodeProcess(h, ctypes.byref(code)):
                return code.value == STILL_ACTIVE
            return None
        finally:
            kernel32.CloseHandle(h)
    except Exception:
        return None


def _holder_line(fh) -> str:
    """读锁文件末行持有者自述(失败返回空串,只作提示不作判据)。"""
    try:
        fh.seek(0)
        lines = fh.read().decode("utf-8", errors="replace").splitlines()
        for line in reversed(lines):
            if line.startswith("holder pid="):
                return line.strip()
    except OSError:
        pass
    return ""


def _holder_pid(holder: str) -> int:
    try:
        return int(holder.split("pid=")[1].split()[0])
    except (IndexError, ValueError):
        return -1

# 进程内互斥层:msvcrt.locking 仅对同一句柄同区域重锁报错;跨 open(同进程)
# 反而立即成功 = 不互斥,必须自兜。acquire()=阻塞、acquire(False)=非阻塞试锁。
_PROCESS_LOCK = threading.Lock()


@contextmanager
def gpu_device_lock(
    purpose: str,
    *,
    lock_path: Path | None = None,
    timeout_s: float = 3600.0,
    poll_s: float = 2.0,
    quiet: bool = False,
):
    """排他持有本机 GPU/构建锁;超时仍未得锁则 RuntimeError(fail-closed)。

    purpose 写入持锁期间的锁文件第二行,供其它等待者打印「谁在持锁」。
    """
    path = lock_path if lock_path is not None else LOCK_PATH
    deadline = time.monotonic() + timeout_s
    holder_desc = f"holder pid={os.getpid()} purpose={purpose} since={time.strftime('%Y-%m-%dT%H:%M:%S')}"

    if not quiet:
        print(f"[gpu_device_lock] acquire purpose={purpose!r} lock={path}", flush=True)
    t0 = time.monotonic()
    while True:
        if _PROCESS_LOCK.acquire(blocking=False):
            break
        if time.monotonic() > deadline:
            raise RuntimeError(
                f"[gpu_device_lock] FAIL: {timeout_s}s 内未得进程内锁(purpose={purpose!r});"
                "同进程另一线程持锁——编排者须排 device 时刻表"
            )
        time.sleep(min(poll_s, 0.5))
    waited_inproc = time.monotonic() - t0
    if waited_inproc > 0.05 and not quiet:
        print(
            f"[gpu_device_lock] waited {waited_inproc:.1f}s on in-process lock (purpose={purpose!r})",
            flush=True,
        )

    fh = None
    try:
        path.parent.mkdir(parents=True, exist_ok=True)
        fh = open(path, "r+b") if path.exists() else open(path, "w+b")
        # 保证文件非空:锁字节(偏移 0)必须落在文件范围内,否则立即成功 = 锁失效。
        fh.seek(0, os.SEEK_END)
        if fh.tell() == 0:
            fh.seek(0)
            fh.write(_SENTINEL)
            fh.flush()
        waited = 0.0
        stale_since: float | None = None
        while True:
            try:
                if msvcrt is not None:
                    fh.seek(0)
                    msvcrt.locking(fh.fileno(), msvcrt.LK_NBLCK, 1)
                break
            except OSError:
                if time.monotonic() > deadline:
                    fh.close()
                    _PROCESS_LOCK.release()
                    raise RuntimeError(
                        f"[gpu_device_lock] FAIL: {timeout_s}s 内未得锁(purpose={purpose!r});"
                        f"锁文件 {path} 被其它进程持有——编排者须排 device 时刻表"
                    )
                # stale holder 检测:持有者 pid 明确已死仍拿不到锁(句柄泄漏面)
                # → 宽限 _STALE_GRACE_S 后回收锁文件重建。pid 复用/不可判一律不回收。
                holder = _holder_line(fh)
                hpid = _holder_pid(holder)
                alive = _pid_alive(hpid) if hpid > 0 else None
                now = time.monotonic()
                if alive is False:
                    if stale_since is None:
                        stale_since = now
                        if not quiet:
                            print(
                                f"[gpu_device_lock] holder pid={hpid} dead but lock busy;"
                                f" grace {_STALE_GRACE_S:.0f}s before reclaim",
                                flush=True,
                            )
                    elif now - stale_since >= _STALE_GRACE_S:
                        try:
                            fh.close()
                            path.unlink()
                            if not quiet:
                                print(
                                    f"[gpu_device_lock] RECLAIM stale lock (dead holder pid={hpid}):"
                                    f" {path} removed and recreated",
                                    flush=True,
                                )
                        except OSError as e:
                            if not quiet:
                                print(
                                    f"[gpu_device_lock] reclaim failed ({e!r}); keep waiting",
                                    flush=True,
                                )
                        fh = open(path, "r+b") if path.exists() else open(path, "w+b")
                        fh.seek(0, os.SEEK_END)
                        if fh.tell() == 0:
                            fh.seek(0)
                            fh.write(_SENTINEL)
                            fh.flush()
                        stale_since = None
                else:
                    stale_since = None
                time.sleep(poll_s)
                waited += poll_s
                if not quiet and waited % 30 < poll_s:
                    state = {True: "alive", False: "DEAD", None: "?"}[alive]
                    print(
                        f"[gpu_device_lock] waiting {waited:.0f}s … current holder: "
                        f"{holder or '?'} (pid {state})",
                        flush=True,
                    )
        # 持锁:写持有者自述(追加行;不清空文件,锁字节不失效)。写回失败仅提示
        # 不致命——自述只服务等待者排障,不是互斥判据(day_0828 §7 PermissionError 面)。
        for _ in range(_UNLOCK_RETRIES):
            try:
                fh.seek(0, os.SEEK_END)
                fh.write((holder_desc + "\n").encode("utf-8"))
                fh.flush()
                break
            except OSError:
                time.sleep(_UNLOCK_RETRY_S)
        if not quiet:
            print(f"[gpu_device_lock] holder=pid:{os.getpid()} purpose={purpose!r}", flush=True)
        yield
    finally:
        if fh is not None:
            unlock_err: OSError | None = None
            if msvcrt is not None:
                for _ in range(_UNLOCK_RETRIES):
                    try:
                        fh.seek(0)
                        msvcrt.locking(fh.fileno(), msvcrt.LK_UNLCK, 1)
                        unlock_err = None
                        break
                    except OSError as e:
                        unlock_err = e
                        time.sleep(_UNLOCK_RETRY_S)
            try:
                fh.close()
            except OSError:
                pass
            if unlock_err is not None and not quiet:
                print(
                    f"[gpu_device_lock] WARN unlock retries exhausted ({unlock_err!r});"
                    " lock released via handle close",
                    flush=True,
                )
        _PROCESS_LOCK.release()
        if not quiet:
            print(f"[gpu_device_lock] released purpose={purpose!r}", flush=True)


# ---------------------------------------------------------------------------
# selftest:同进程双臂互斥 / 子进程跨进程互斥 / release 后可再得
# ---------------------------------------------------------------------------

_CHILD_SNIPPET = """
import sys, time
sys.path.insert(0, sys.argv[2])
from gpu_device_lock import gpu_device_lock
from pathlib import Path
lock = Path(sys.argv[1])
t0 = time.monotonic()
with gpu_device_lock("selftest-child", lock_path=lock, timeout_s=60, poll_s=0.2, quiet=True):
    print(f"CHILD_WAITED={time.monotonic() - t0:.2f}", flush=True)
"""


def run_selftest() -> int:
    failures = 0
    ci_dir = str(Path(__file__).resolve().parent)
    with tempfile.TemporaryDirectory(prefix="gpu_device_lock_selftest_") as td:
        lock = Path(td) / "lock.bin"

        # 臂 1:同进程嵌套 acquire 必须阻塞(超时必红 = 互斥存在的证据)
        t0 = time.monotonic()
        try:
            with gpu_device_lock("selftest-outer", lock_path=lock, quiet=True):
                try:
                    with gpu_device_lock(
                        "selftest-inner", lock_path=lock, timeout_s=1.5, poll_s=0.2, quiet=True
                    ):
                        print("  RED WRONG— 臂1:同进程嵌套 acquire 未阻塞", flush=True)
                        failures += 1
                except RuntimeError:
                    print(
                        f"  RED ok   — 臂1:同进程嵌套 acquire 超时拒(waited {time.monotonic()-t0:.1f}s)",
                        flush=True,
                    )
        except RuntimeError as e:
            print(f"  RED WRONG— 臂1:外层 acquire 异常 {e}", flush=True)
            failures += 1

        # 臂 2:父进程持锁时,子进程 acquire 必须等待至父释放后才成功
        with gpu_device_lock("selftest-parent", lock_path=lock, quiet=True):
            proc = subprocess.Popen(
                [sys.executable, "-u", "-c", _CHILD_SNIPPET, str(lock), ci_dir],
                stdout=subprocess.PIPE,
                stderr=subprocess.STDOUT,
                text=True,
                cwd=ci_dir,
            )
            time.sleep(2.0)
            still_waiting = proc.poll() is None
        out, _ = proc.communicate(timeout=90)
        if still_waiting and proc.returncode == 0 and "CHILD_WAITED=" in (out or ""):
            waited = float(out.split("CHILD_WAITED=")[1].split()[0])
            if waited >= 1.5:
                print(
                    f"  RED ok   — 臂2:子进程跨进程互斥(子进程等待 {waited:.1f}s 后才得锁)",
                    flush=True,
                )
            else:
                print(f"  RED WRONG— 臂2:子进程等待仅 {waited:.1f}s(<1.5s),锁未生效", flush=True)
                failures += 1
        else:
            print(
                f"  RED WRONG— 臂2:子进程行为异常 rc={proc.returncode} "
                f"still_waiting={still_waiting} out={out!r}",
                flush=True,
            )
            failures += 1

        # 臂 3(GREEN):释放后可再得
        try:
            with gpu_device_lock("selftest-green", lock_path=lock, timeout_s=5, quiet=True):
                pass
            print("  GREEN ok — 臂3:release 后可再得锁", flush=True)
        except RuntimeError as e:
            print(f"  GREEN MISS— 臂3:{e}", flush=True)
            failures += 1

    if failures:
        print(f"[gpu_device_lock] SELFTEST FAIL ({failures})")
        return 1
    print("[gpu_device_lock] SELFTEST PASS (2 RED + 1 GREEN)")
    return 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--selftest", action="store_true", help="证明互斥断言能红能绿")
    args = ap.parse_args()
    if args.selftest:
        return run_selftest()
    ap.error("仅支持 --selftest;生产用法为库形式 from gpu_device_lock import gpu_device_lock")
    return 2


if __name__ == "__main__":
    sys.exit(main())
