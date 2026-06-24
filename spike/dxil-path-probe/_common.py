# SPIKE(RD-010) — G2.2 Q-D131=C 双路 DXIL spike 取证共享 helper。
# 隔离于 spike/dxil-path-probe/,不入 src/ 生产路径、不随产品编译、spike 结束可弃。
# 纪律:measured-first / blocked-honest(AGENTS 硬规则 3/4)——探到记实测,探不到如实 blocked,绝不杜撰。
"""DXIL path spike 共享工具:安全 subprocess 包装 + 工具链定位 + 版本探测。

设计约束:
- 全部经 list 参数调 subprocess(shell=False),禁字符串插值,防命令注入(安全编码)。
- 每次调用带 timeout;工具缺失/超时/spawn 失败 → 返回结构化失败,不抛、不崩溃。
- 工具不存在一律降级为 'unavailable' 字符串(对齐 evidence schema 受限环境降级值)。
"""
from __future__ import annotations

import os
import shutil
import subprocess
from pathlib import Path

UNAVAILABLE = "unavailable"
DEFAULT_TIMEOUT = 30


def run(args: list[str], timeout: int = DEFAULT_TIMEOUT) -> dict:
    """安全执行(shell=False,list 参数)。返回 {ok, rc, stdout, stderr, error}。

    工具缺失/超时/spawn 失败均捕获为结构化结果,绝不抛出。
    """
    try:
        proc = subprocess.run(
            args,
            capture_output=True,
            text=True,
            encoding="utf-8",
            errors="replace",
            timeout=timeout,
            check=False,
        )
        return {
            "ok": proc.returncode == 0,
            "rc": proc.returncode,
            "stdout": proc.stdout or "",
            "stderr": proc.stderr or "",
            "error": None,
        }
    except FileNotFoundError:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": "not_found"}
    except subprocess.TimeoutExpired:
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": "timeout"}
    except OSError as e:  # spawn 失败(权限/路径等)
        return {"ok": False, "rc": None, "stdout": "", "stderr": "", "error": f"oserror:{e}"}


def locate_clang() -> tuple[str | None, str]:
    """定位 clang,复刻 toolchain.rs 探测序(D-205):
    RURIXC_CLANG > C:\\Program Files\\LLVM\\bin\\clang.exe > PATH。

    返回 (路径或 None, 来源标签)。
    """
    env = os.environ.get("RURIXC_CLANG")
    if env and Path(env).is_file():
        return env, "RURIXC_CLANG"
    default = Path("C:/Program Files/LLVM/bin/clang.exe")
    if default.is_file():
        return str(default), "default_llvm_path"
    found = shutil.which("clang")
    if found:
        return found, "PATH"
    return None, UNAVAILABLE


def locate_tool(name: str, env_var: str | None = None) -> str | None:
    """通用工具定位:可选环境变量 > PATH。"""
    if env_var:
        env = os.environ.get(env_var)
        if env and Path(env).is_file():
            return env
    return shutil.which(name)


def tool_version(path: str | None, version_args: list[str]) -> str:
    """探工具版本字符串(取首行)或 'unavailable'。"""
    if not path:
        return UNAVAILABLE
    res = run([path] + version_args, timeout=15)
    if res["error"] == "not_found":
        return UNAVAILABLE
    text = (res["stdout"] or res["stderr"]).strip()
    if not text:
        return UNAVAILABLE if not res["ok"] else "(version output empty)"
    return text.splitlines()[0].strip()
