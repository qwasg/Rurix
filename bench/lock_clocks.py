"""锁频工具 + 采样前置闸门(BENCH_PROTOCOL.md §2.1 / 契约 G-M5-1)。

L0 锁频规程的可执行化:`nvidia-smi -lgc/-lmc` 锁频(需管理员)→ NVML 读回校验
→ 采样前 fail-fast 闸门(未锁频拒绝采样,避免 unlocked 证据浪费整轮 3×(50×3))。

锁频目标常量(SM/MEM 时钟、容差、显存可接受档)统一引用 env_probe(单一事实源,
BENCH_PROTOCOL.md §2.1;持久模式 Windows 不支持,沿用 -lgc/-lmc 仰角路径)。

用法:
  py -3 bench/lock_clocks.py --lock      # 锁 SM/MEM 时钟 + 读回校验(需管理员)
  py -3 bench/lock_clocks.py --check      # 仅 NVML 读回校验,未锁频退出码 1(默认动作)
  py -3 bench/lock_clocks.py --unlock    # 解锁(-rgc/-rmc,会话结束)

编程接口:
  from bench import lock_clocks; lock_clocks.require_locked()  # 未锁频抛 SystemExit(1)
"""
from __future__ import annotations

import subprocess
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from bench import env_probe

ROOT = Path(__file__).resolve().parent.parent

# 锁频目标统一引用 env_probe(单一事实源,BENCH_PROTOCOL.md §2.1)
LOCK_TARGET_SM_MHZ = env_probe.LOCK_TARGET_SM_MHZ
LOCK_TARGET_MEM_MHZ = env_probe.LOCK_TARGET_MEM_MHZ


def _run_smi(args: list[str]) -> int:
    cmd = ["nvidia-smi", *args]
    print(f"  $ {' '.join(cmd)}")
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.stdout.strip():
        print(proc.stdout.strip())
    if proc.returncode != 0 and proc.stderr.strip():
        print(proc.stderr.strip(), file=sys.stderr)
    return proc.returncode


def clocks_state() -> dict:
    """NVML 读回当前时钟与锁定判定(复用 env_probe.collect_environment)。"""
    return env_probe.collect_environment()["clocks"]


def lock() -> int:
    """锁 SM/MEM 时钟(需管理员)→ NVML 读回校验。返回退出码。"""
    print("[lock_clocks] 锁频(需管理员;持久模式 Windows 不支持,沿用 -lgc/-lmc)")
    rc1 = _run_smi(["-lgc", f"{LOCK_TARGET_SM_MHZ},{LOCK_TARGET_SM_MHZ}"])
    rc2 = _run_smi(["-lmc", f"{LOCK_TARGET_MEM_MHZ}"])
    if rc1 != 0 or rc2 != 0:
        print("[lock_clocks] FAIL — nvidia-smi 锁频命令返回非 0"
              "(需管理员权限;GeForce 驱动对 -lgc/-lmc 支持随版本波动,r11 §1.2)",
              file=sys.stderr)
        return 1
    state = clocks_state()
    if state["locked"]:
        print(f"[lock_clocks] PASS — 锁定生效(SM {state['sm_clock_mhz']} MHz / "
              f"MEM {state['mem_clock_mhz']} MHz)")
        return 0
    print(f"[lock_clocks] FAIL — 读回未达目标(SM {state['sm_clock_mhz']} MHz / "
          f"MEM {state['mem_clock_mhz']} MHz;目标 SM {LOCK_TARGET_SM_MHZ} / "
          f"MEM {LOCK_TARGET_MEM_MHZ})", file=sys.stderr)
    return 1


def unlock() -> int:
    """解锁 SM/MEM 时钟(会话结束,BENCH_PROTOCOL.md §2.1 第 5 步)。"""
    print("[lock_clocks] 解锁(-rgc/-rmc)")
    rc1 = _run_smi(["-rgc"])
    rc2 = _run_smi(["-rmc"])
    return 0 if (rc1 == 0 and rc2 == 0) else 1


def check() -> int:
    """NVML 读回校验(fail-fast 闸门)。未锁频退出码 1。"""
    state = clocks_state()
    if state["locked"]:
        print(f"[lock_clocks] PASS — 已锁频(SM {state['sm_clock_mhz']} MHz / "
              f"MEM {state['mem_clock_mhz']} MHz)")
        return 0
    print(f"[lock_clocks] FAIL — 未锁频(SM {state['sm_clock_mhz']} MHz / "
          f"MEM {state['mem_clock_mhz']} MHz);采样将产 unlocked 证据,不得回填"
          "(BENCH_PROTOCOL.md §2.1)。先运行 py -3 bench/lock_clocks.py --lock",
          file=sys.stderr)
    return 1


def require_locked() -> None:
    """采样前置闸门:未锁频抛 SystemExit(1)。

    供三次运行器在 main() 起始调用,避免跑完 3×(50×3) 采样才在聚合期发现 unlocked
    (聚合期 level 校验仍保留作二道防线)。
    """
    if check() != 0:
        raise SystemExit(1)


def main(argv: list[str]) -> int:
    if "--lock" in argv:
        return lock()
    if "--unlock" in argv:
        return unlock()
    # 默认动作为 --check
    return check()


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
