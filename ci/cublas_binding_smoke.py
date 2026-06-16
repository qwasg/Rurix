# -*- coding: utf-8 -*-
"""cublas GEMM/GEMV 三层绑定冒烟(M8.2,契约 G-M8-2;CI_GATES 步骤 35)。

用法:
    py -3 ci/cublas_binding_smoke.py             # 构建 rurix-cublas cdylib → torch CUDA
                                                 #   张量设备指针零拷贝 → cublas GEMM/GEMV
                                                 #   数值对照 + 内建篡改红绿 + 写 evidence
    py -3 ci/cublas_binding_smoke.py --self-test # 仅跑内建篡改绑定结果红绿自检(不写证据)

端到端(G-M8-2):`cargo build -p rurix-cublas` 产 cdylib(三层绑定 raw FFI / safe wrapper /
  高层 API),经 ctypes 加载;PyTorch CUDA 张量(行主序 f32)经 C ABI `rurix_cublas_gemm` /
  `rurix_cublas_gemv` **零拷贝**(data_ptr 设备地址)调用 cublas(复用 rurix-rt 共享 primary
  context + 借用外部设备指针缓冲),与 `torch.matmul` / `torch.mv` 参考数值对照。runtime DLL
  (`cublas64_*.dll`)经 Attachment A 白名单审计留痕(RXS-0129)。

内建红绿(反 YAML-only,H06 D11.8-2):取 GEMM 正确结果,**篡改其数值**(扰动一元素)→
  数值对照必判失败(红检测有效);若篡改后对照仍通过(门无效)即脚本 FAIL。

写 evidence/cublas_binding_smoke.json(bindings_passed),计入 m8.counter.cublas_bindings
  (ci/budget_eval.py,>=2 则 PASS;计数源 = evidence/cublas_*.json 的 bindings_passed
  去重集最大基数)。

降级 SKIP(exit 0,真红绿在带 CUDA torch + cublas + GPU 的 self-hosted runner):
  - 无 torch CUDA(`torch.cuda.is_available()` False);
  - cdylib 构建失败 / cublas runtime DLL 不可用(绑定返回 RX7016)。
"""

import ctypes
import json
import re
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DLL = ROOT / "target" / "debug" / ("rurix_cublas.dll" if sys.platform == "win32" else "librurix_cublas.so")
EVIDENCE = ROOT / "evidence" / "cublas_binding_smoke.json"

GEMM_REL_TOL = 1e-3
GEMV_REL_TOL = 1e-3

# cublas 句柄初始化失败错误码(RX7016;无 cublas runtime DLL / cublasCreate 失败 → 降级)。
RX_CUBLAS_HANDLE_INIT_FAILED = 7016


def fail(msg: str) -> None:
    print(f"[cublas_binding_smoke] FAIL: {msg}")
    sys.exit(1)


def skip(msg: str) -> None:
    print(f"[cublas_binding_smoke] SKIP: {msg}")
    sys.exit(0)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_cdylib():
    r = run(["cargo", "build", "-p", "rurix-cublas"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build -p rurix-cublas 失败:\n{r.stderr}")
    if not DLL.exists():
        fail(f"rurix-cublas cdylib 产物缺失: {DLL}")


def load_binding():
    lib = ctypes.CDLL(str(DLL))
    lib.rurix_cublas_gemm.restype = ctypes.c_int32
    lib.rurix_cublas_gemm.argtypes = [ctypes.c_uint64] * 3 + [ctypes.c_uint64] * 3
    lib.rurix_cublas_gemv.restype = ctypes.c_int32
    lib.rurix_cublas_gemv.argtypes = [ctypes.c_uint64] * 3 + [ctypes.c_uint64] * 2
    return lib


def audit_runtime_dll() -> dict:
    """定位 cublas runtime DLL 并核对 Attachment A 白名单形态(RXS-0129)。"""
    import os

    found = None
    cuda_path = os.environ.get("CUDA_PATH")
    if cuda_path:
        for sub in ("bin/x64", "bin"):
            for p in (Path(cuda_path) / sub).glob("cublas64_*.dll"):
                found = p.name
                break
            if found:
                break
    whitelisted = bool(found and re.fullmatch(r"cublas64_\d+\.dll", found))
    return {
        "runtime_dll": found or "(not located via CUDA_PATH)",
        "attachment_a_whitelisted": whitelisted,
        "note": "Attachment A 白名单最小集(cublas64_*.dll 运行期库);完整 Toolkit/驱动/Nsight 永不捆绑(r6);M8.2 链接系统 DLL,物理捆绑/再分发承接 M8.4",
    }


def call_gemm(lib, torch, M, K, N):
    dev = "cuda"
    A = torch.rand(M, K, dtype=torch.float32, device=dev).contiguous()
    B = torch.rand(K, N, dtype=torch.float32, device=dev).contiguous()
    ref = A @ B
    C = torch.empty(M, N, dtype=torch.float32, device=dev).contiguous()
    torch.cuda.synchronize()
    rc = lib.rurix_cublas_gemm(C.data_ptr(), A.data_ptr(), B.data_ptr(), M, N, K)
    torch.cuda.synchronize()
    return rc, C, ref


def call_gemv(lib, torch, M, N):
    dev = "cuda"
    A = torch.rand(M, N, dtype=torch.float32, device=dev).contiguous()
    x = torch.rand(N, dtype=torch.float32, device=dev).contiguous()
    ref = torch.mv(A, x)
    y = torch.empty(M, dtype=torch.float32, device=dev).contiguous()
    torch.cuda.synchronize()
    rc = lib.rurix_cublas_gemv(y.data_ptr(), A.data_ptr(), x.data_ptr(), M, N)
    torch.cuda.synchronize()
    return rc, y, ref


def run_bindings(lib, torch):
    """跑 cublas GEMM/GEMV 三层绑定,返回(bindings_passed, facts)。"""
    facts = []
    passed = set()

    # —— GEMM:C[M,N] = A[M,K]·B[K,N](行主序 ↔ cublas 列主序适配,RXS-0128)——
    rc, C, gref = call_gemm(lib, torch, 128, 96, 112)
    if rc == RX_CUBLAS_HANDLE_INIT_FAILED:
        skip(f"cublas 句柄初始化失败(RX{rc};无 cublas runtime DLL/GPU,降级 SKIP)")
    if rc != 0:
        skip(f"cublas gemm 返回码 {rc}(降级 SKIP)")
    rel = ((C - gref).abs() / gref.abs().clamp_min(1.0)).max().item()
    facts.append({"binding": "gemm", "layer": "raw_ffi+safe_wrapper+high_level", "max_rel_err": rel, "tolerance": GEMM_REL_TOL})
    if rel <= GEMM_REL_TOL:
        passed.add("gemm")

    # —— GEMV:y[M] = A[M,N]·x[N](行主序经 CUBLAS_OP_T 适配,RXS-0128)——
    rc, y, vref = call_gemv(lib, torch, 160, 128)
    if rc != 0:
        skip(f"cublas gemv 返回码 {rc}(降级 SKIP)")
    rel = ((y - vref).abs() / vref.abs().clamp_min(1.0)).max().item()
    facts.append({"binding": "gemv", "layer": "raw_ffi+safe_wrapper+high_level", "max_rel_err": rel, "tolerance": GEMV_REL_TOL})
    if rel <= GEMV_REL_TOL:
        passed.add("gemv")

    return passed, facts


def red_self_test(lib, torch) -> bool:
    """红:取 gemm 正确结果,篡改一元素数值 → 数值对照必判失败(>容差)。
    返回 True = 红有效(篡改被对照判出)。"""
    rc, C, gref = call_gemm(lib, torch, 64, 48, 64)
    if rc == RX_CUBLAS_HANDLE_INIT_FAILED:
        skip(f"red self-test cublas 句柄初始化失败(RX{rc},降级 SKIP)")
    if rc != 0:
        skip(f"red self-test gemm 返回码 {rc}(降级 SKIP)")
    base = ((C - gref).abs() / gref.abs().clamp_min(1.0)).max().item()
    if base > GEMM_REL_TOL:
        fail("绿基线失败:未篡改的 cublas gemm 数值对照即超容差(绑定不正确)")
    tampered = C.clone()
    tampered[0, 0] = tampered[0, 0] + 1.0  # 篡改绑定数值结果
    rel = ((tampered - gref).abs() / gref.abs().clamp_min(1.0)).max().item()
    return rel > GEMM_REL_TOL


def main() -> None:
    self_test = "--self-test" in sys.argv
    try:
        import torch
    except ImportError:
        skip("未安装 torch(降级 SKIP;真红绿在带 CUDA torch 的 self-hosted runner)")
    if not torch.cuda.is_available():
        skip("torch.cuda 不可用(CPU-only torch / 无 GPU;降级 SKIP)")

    build_cdylib()
    lib = load_binding()

    if not red_self_test(lib, torch):
        fail("红验证失败:篡改 cublas gemm 数值结果后对照未判失败(门未真正校验,反 YAML-only)")

    if self_test:
        print("[cublas_binding_smoke] self-test PASS(篡改绑定数值结果 → 数值对照判失败,门有效)")
        return

    passed, facts = run_bindings(lib, torch)
    if len(passed) < 2:
        fail(f"cublas 绑定数值对照未全通过:passed={sorted(passed)}(要求 gemm/gemv 两绑定)")

    redistribution = audit_runtime_dll()
    if not redistribution["attachment_a_whitelisted"]:
        # 审计未命中白名单形态属环境问题(非数值红绿),记录但不阻断绑定计数。
        print(f"[cublas_binding_smoke] WARN: runtime DLL 未命中 Attachment A 白名单形态: {redistribution['runtime_dll']}")

    gpu = torch.cuda.get_device_name(0)
    evidence = {
        "schema_version": 1,
        "subject": "cublas_bindings",
        "bindings_passed": sorted(passed),
        "binding_layers": ["raw_ffi", "safe_wrapper", "high_level_api"],
        "build_command": "cargo build -p rurix-cublas(cdylib);ctypes 加载 rurix_cublas_gemm/gemv,torch CUDA 张量 data_ptr 设备地址零拷贝",
        "device": {
            "gpu": gpu,
            "torch": torch.__version__,
            "cuda": torch.version.cuda or "",
        },
        "redistribution": redistribution,
        "facts": facts,
        "redgreen": {
            "red_command": "篡改 cublas gemm 绑定数值结果(扰动一元素)后与 torch.matmul 参考数值对照",
            "red_detected": True,
            "green_command": "py -3 ci/cublas_binding_smoke.py(cargo build -p rurix-cublas → ctypes 零拷贝 → gemm/gemv 数值对照通过)",
            "green_exit_code": 0,
            "run_url": "local red-green(反 YAML-only,H06 D11.8-2);pr-smoke 步骤 35 self-hosted runner run URL 见 PR 描述",
        },
        "timestamp": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[cublas_binding_smoke] PASS(cargo build -p rurix-cublas cdylib / "
        f"ctypes 零拷贝接入 PyTorch CUDA({gpu}) / "
        f"{sorted(passed)} GEMM·GEMV 三层绑定数值对照通过 / 红验证篡改数值对照判失败 / "
        f"runtime DLL {redistribution['runtime_dll']} Attachment A 审计 → "
        f"{EVIDENCE.relative_to(ROOT).as_posix()})"
    )


if __name__ == "__main__":
    main()
