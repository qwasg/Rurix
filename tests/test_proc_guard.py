"""proc_guard 看门狗红绿测试(P0-7 nightly 僵尸进程根治)。

绿:正常子进程照常返回 0、无隔离。
红:故意挂死的子进程(且 spawn 一个孙进程 sleeper)→ guarded_run 小超时内:
    (a) 返回诚实红退出码 124(非 SKIP、非无限挂起);
    (b) 杀掉整棵进程树(孙进程 PID 事后查不到 —— 这是 vs 纯 subprocess.run(timeout=)
        只杀直接子进程的分水岭);
    (c) 目标 exe 被 Move 进隔离目录(临时目录)。

用 Python 脚本充当「子进程/孙进程」,避免测试依赖编译器;隔离用 pytest tmp_path。
纳入 pr-smoke 的 `pytest tests/ -q` 门,使看门盘自带 CI 强制红绿(反 YAML-only)。
"""
from __future__ import annotations

import os
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench.proc_guard import TIMEOUT_EXIT, guarded_run


# 孙进程 sleeper:P1 spawn 一个独立 python 孙进程 sleep,把孙 PID 写盘,然后自己也 sleep
# (使整树挂死并触发超时)。tree-kill 必须连孙进程一并回收。
_HANG_WITH_GRANDCHILD = (
    "import subprocess, sys, time\n"
    "g = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(9999)'])\n"
    "with open(sys.argv[1], 'w') as f:\n"
    "    f.write(str(g.pid))\n"
    "    f.flush()\n"
    "sys.stdout.flush()\n"
    "time.sleep(9999)\n"
)


def _pid_alive(pid: int) -> bool:
    if os.name == "nt":
        out = subprocess.run(["tasklist", "/FI", f"PID eq {pid}", "/NH"],
                             capture_output=True, text=True).stdout
        return "No tasks" not in out and str(pid) in out
    try:
        os.kill(pid, 0)
        return True
    except OSError:
        return False


def _wait_pid_gone(pid: int, timeout: float = 10.0) -> bool:
    """轮询等待 pid 消失(taskkill 后 OS 回收有延迟);返回 True=已消失。"""
    deadline = time.time() + timeout
    while time.time() < deadline:
        if not _pid_alive(pid):
            return True
        time.sleep(0.2)
    return not _pid_alive(pid)


def test_guarded_run_green_normal_completes(tmp_path):
    """绿:正常子进程返回 0,无隔离产物。"""
    r = guarded_run([sys.executable, "-c", "print('green ok')"],
                    timeout=30, capture=True, quarantine_dir=tmp_path / "q")
    assert r.returncode == 0
    assert r.timed_out is False
    assert "green ok" in r.stdout
    assert r.quarantined == []
    # 未触发隔离 → 隔离目录不应被创建
    assert not (tmp_path / "q").exists()


def test_guarded_run_green_nonzero_exit_passthrough(tmp_path):
    """绿(非零):正常退出的非零码原样透传,不误判为超时。"""
    r = guarded_run([sys.executable, "-c", "import sys; sys.exit(7)"],
                    timeout=30, capture=True, quarantine_dir=tmp_path / "q")
    assert r.returncode == 7
    assert r.timed_out is False
    assert r.quarantined == []


def test_guarded_run_red_timeout_kills_tree_and_quarantines(tmp_path):
    """红:挂死子进程 → 超时杀整树(含孙进程)+ 隔离目标 exe + 诚实红 124。"""
    pidfile = tmp_path / "grandchild_pid.txt"
    fake_exe = tmp_path / "fake_stuck.exe"
    fake_exe.write_bytes(b"MZ\x90\x00 fake stuck exe")
    qdir = tmp_path / "quarantine"

    t0 = time.time()
    r = guarded_run(
        [sys.executable, "-c", _HANG_WITH_GRANDCHILD, str(pidfile)],
        timeout=3, capture=True, quarantine_exe=fake_exe, quarantine_dir=qdir,
    )
    elapsed = time.time() - t0

    # (诚实红)超时 = 非零退出码 124,timed_out=True,绝非 SKIP/绿
    assert r.timed_out is True
    assert r.returncode == TIMEOUT_EXIT != 0
    # 不无限挂起:大致 = timeout + 杀树收尾开销,给宽松上界
    assert elapsed < 60, f"guarded_run 耗时 {elapsed:.1f}s,疑似未按超时返回"

    # (树杀分水岭)孙进程 PID 事后查不到 —— 证 taskkill /T 杀了整树而非只杀直接子
    assert pidfile.exists(), "子进程未写出孙进程 PID(测试前提失败)"
    gc_pid = int(pidfile.read_text().strip())
    assert _wait_pid_gone(gc_pid), f"孙进程 pid={gc_pid} 仍存活 —— 进程树未被杀干净"

    # (隔离)目标 exe 被 Move 进隔离目录,原路径解锁,隔离路径在返回值里
    assert not fake_exe.exists(), "目标 exe 未从原路径 move 走(未解锁)"
    moved = list(qdir.glob("fake_stuck_*.exe"))
    assert len(moved) == 1, f"隔离目录内产物数异常: {moved}"
    assert str(moved[0]) in r.quarantined


def test_cli_green_exit_code_passthrough():
    """CLI 绿:子进程退出码原样透传(供 nightly PowerShell $LASTEXITCODE 判定)。"""
    from bench import proc_guard
    rc = proc_guard._cli(["--timeout", "30", "--",
                          sys.executable, "-c", "import sys; sys.exit(0)"])
    assert rc == 0
    rc = proc_guard._cli(["--timeout", "30", "--",
                          sys.executable, "-c", "import sys; sys.exit(5)"])
    assert rc == 5


def test_cli_no_command_usage_error():
    """CLI:缺 `--` 命令段 → 退出 2(usage),不静默成 0。"""
    from bench import proc_guard
    assert proc_guard._cli(["--timeout", "30"]) == 2
