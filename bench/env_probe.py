"""L0 环境画像探测器(契约 G-M0-3 / BENCH_PROTOCOL.md §2.3 / 08 §2.3)。

NVML 优先(nvml.dll ctypes 直调),nvidia-smi 仅人工后备(r6);
TDR 与 HAGS 读注册表;CUDA 驱动版本经 nvcuda.dll cuDriverGetVersion。
受限环境字段降级取 "unavailable",schema 不变(14 §5)。

用法:
  py -3 bench/env_probe.py             # 打印环境画像 JSON
  py -3 bench/env_probe.py --validate  # 同时对 evidence_schema 的 environment 子 schema 校验
"""
from __future__ import annotations

import ctypes
import json
import sys
import winreg
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# 锁频目标(BENCH_PROTOCOL.md §2.1;2610 = RTX 4070 Ti 官方 Boost Clock,
# 来源:NVIDIA 4070 Ti 规格页;10501 = 本机唯一支持显存档,
# 来源:nvidia-smi -q -d SUPPORTED_CLOCKS 输出,2026-06-11)
LOCK_TARGET_SM_MHZ = 2610
# GDDR6X 实测特性:-lmc 10501 锁定生效后,负载下 NVML 报告值为 10251(低一档),
# 空闲读回 10501;两值均视为锁定(来源:本机 nvmlDeviceGetClockInfo 实测,2026-06-11)
LOCK_TARGET_MEM_MHZ = 10501
LOCK_MEM_ACCEPTED_MHZ = (10501, 10251)
CLOCK_TOLERANCE_MHZ = 15  # 相邻档位步进(SUPPORTED_CLOCKS 输出为 15MHz 步进)

NVML_CLOCK_SM = 1
NVML_CLOCK_MEM = 2
NVML_TEMPERATURE_GPU = 0


class NvmlError(RuntimeError):
    pass


def _check(nvml, ret: int, fn: str) -> None:
    if ret != 0:
        raise NvmlError(f"{fn} failed: NVML error {ret}")


def collect_environment(device_index: int = 0) -> dict:
    env: dict = {}

    # --- NVML ---
    nvml = ctypes.WinDLL("nvml.dll")
    _check(nvml, nvml.nvmlInit_v2(), "nvmlInit_v2")
    try:
        buf = ctypes.create_string_buffer(96)

        _check(nvml, nvml.nvmlSystemGetDriverVersion(buf, 96), "nvmlSystemGetDriverVersion")
        env["driver_version"] = buf.value.decode()

        _check(nvml, nvml.nvmlSystemGetNVMLVersion(buf, 96), "nvmlSystemGetNVMLVersion")
        env["nvml_version"] = buf.value.decode()

        handle = ctypes.c_void_p()
        _check(nvml, nvml.nvmlDeviceGetHandleByIndex_v2(device_index, ctypes.byref(handle)),
               "nvmlDeviceGetHandleByIndex_v2")

        _check(nvml, nvml.nvmlDeviceGetName(handle, buf, 96), "nvmlDeviceGetName")
        env["gpu_name"] = buf.value.decode()

        major, minor = ctypes.c_int(), ctypes.c_int()
        _check(nvml, nvml.nvmlDeviceGetCudaComputeCapability(handle, ctypes.byref(major), ctypes.byref(minor)),
               "nvmlDeviceGetCudaComputeCapability")
        env["compute_capability"] = f"{major.value}.{minor.value}"

        # 驱动模型:0=WDDM,1=WDM(TCC),2=MCDM
        cur, pending = ctypes.c_int(), ctypes.c_int()
        try:
            _check(nvml, nvml.nvmlDeviceGetDriverModel_v2(handle, ctypes.byref(cur), ctypes.byref(pending)),
                   "nvmlDeviceGetDriverModel_v2")
            env["driver_model"] = {0: "WDDM", 1: "TCC", 2: "MCDM"}.get(cur.value, "unavailable")
        except (NvmlError, AttributeError):
            env["driver_model"] = "unavailable"

        sm_clk, mem_clk = ctypes.c_uint(), ctypes.c_uint()
        _check(nvml, nvml.nvmlDeviceGetClockInfo(handle, NVML_CLOCK_SM, ctypes.byref(sm_clk)),
               "nvmlDeviceGetClockInfo(SM)")
        _check(nvml, nvml.nvmlDeviceGetClockInfo(handle, NVML_CLOCK_MEM, ctypes.byref(mem_clk)),
               "nvmlDeviceGetClockInfo(MEM)")
        locked = (
            abs(sm_clk.value - LOCK_TARGET_SM_MHZ) <= CLOCK_TOLERANCE_MHZ
            and any(abs(mem_clk.value - t) <= CLOCK_TOLERANCE_MHZ for t in LOCK_MEM_ACCEPTED_MHZ)
        )
        env["clocks"] = {
            "locked": locked,
            "sm_clock_mhz": sm_clk.value,
            "mem_clock_mhz": mem_clk.value,
            "lock_method": "nvidia-smi -lgc/-lmc (elevated); persistence mode unsupported on Windows",
        }

        temp = ctypes.c_uint()
        _check(nvml, nvml.nvmlDeviceGetTemperature(handle, NVML_TEMPERATURE_GPU, ctypes.byref(temp)),
               "nvmlDeviceGetTemperature")
        env["thermal"] = {"temp_start_c": temp.value, "temp_end_c": temp.value, "steady_state": False}

        # 进程隔离:计算进程枚举
        count = ctypes.c_uint(0)
        ret = nvml.nvmlDeviceGetComputeRunningProcesses_v3(handle, ctypes.byref(count), None)
        # ret 7 = INSUFFICIENT_SIZE(有进程),0 = 无进程
        n_procs = count.value if ret in (0, 7) else "unavailable"
        env["isolation_check"] = {"other_compute_processes": n_procs if isinstance(n_procs, int) else 0}
    finally:
        nvml.nvmlShutdown()

    # --- CUDA Driver API ---
    try:
        cuda = ctypes.WinDLL("nvcuda.dll")
        ver = ctypes.c_int()
        if cuda.cuDriverGetVersion(ctypes.byref(ver)) == 0:
            env["cuda_driver_version"] = f"{ver.value // 1000}.{(ver.value % 1000) // 10}"
        else:
            env["cuda_driver_version"] = "unavailable"
    except OSError:
        env["cuda_driver_version"] = "unavailable"

    # --- 注册表:TDR 与 HAGS ---
    tdr: dict = {}
    hags: object = "unavailable"
    try:
        with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE,
                            r"SYSTEM\CurrentControlSet\Control\GraphicsDrivers") as key:
            for name, target in (("TdrDelay", "tdr_delay"), ("TdrLevel", "tdr_level")):
                try:
                    tdr[target], _ = winreg.QueryValueEx(key, name)
                except FileNotFoundError:
                    tdr[target] = "not_set(os_default)"
            try:
                hsm, _ = winreg.QueryValueEx(key, "HwSchMode")
                hags = (hsm == 2)
            except FileNotFoundError:
                hags = "unavailable"
    except OSError:
        tdr = {"tdr_delay": "unavailable", "tdr_level": "unavailable"}
    env["tdr"] = tdr
    env["hags_enabled"] = hags

    # --- OS ---
    import platform
    env["os_build"] = f"{platform.system()} {platform.version()}"

    return env


def validate(env: dict) -> list[str]:
    import jsonschema
    schema = json.loads((ROOT / "milestones/m0/evidence_schema.json").read_text(encoding="utf-8"))
    sub = schema["properties"]["environment"]
    return [f"{'/'.join(str(p) for p in e.path)}: {e.message}"
            for e in jsonschema.Draft7Validator(sub).iter_errors(env)]


if __name__ == "__main__":
    profile = collect_environment()
    print(json.dumps(profile, ensure_ascii=False, indent=2))
    if "--validate" in sys.argv:
        errors = validate(profile)
        if errors:
            print("[env_probe] schema FAIL", file=sys.stderr)
            for e in errors:
                print(f"  - {e}", file=sys.stderr)
            sys.exit(1)
        print("[env_probe] schema PASS", file=sys.stderr)
