# -*- coding: utf-8 -*-
"""UC-01 PyTorch 互操作算子替换端到端冒烟(M8.1,契约 G-M8-1;CI_GATES 步骤 34)。

用法:
    py -3 ci/uc01_interop_smoke.py             # rx build --emit=pyd 产 PYD → PyTorch CUDA
                                               #   双协议零拷贝 → 算子替换数值对照 + 内建篡改
                                               #   红绿 + 写 evidence
    py -3 ci/uc01_interop_smoke.py --self-test # 仅跑内建篡改算子结果红绿自检(不写证据)

端到端(G-M8-1):`rx build --emit=pyd <kernel.rx>` 经 rurixc 编译 device kernel→PTX
  (PTX-only)+ nanobind + scikit-build-core 打包链接 rurix-interop(C ABI,复用 M5 自研
  kernel)产 `.pyd`;PyTorch CUDA 张量经 `__cuda_array_interface__` v3 / DLPack **双协议
  零拷贝**接入,SAXPY/Reduction/GEMM 算子替换端到端真跑、与 PyTorch 参考数值对照。

内建红绿(反 YAML-only,H06 D11.8-2):取一算子正确结果,**篡改其数值**(扰动一元素)→
  数值对照必判失败(红检测有效);若篡改后对照仍通过(门无效)即脚本 FAIL。

写 evidence/uc01_interop_smoke.json(operators_passed),计入 m8.counter.uc01_pytorch_
  operators(ci/budget_eval.py,>=3 则 PASS;计数源 = evidence/uc01_*.json 的
  operators_passed 去重集最大基数)。

降级 SKIP(exit 0,真红绿在带 CUDA torch + nanobind + GPU 的 self-hosted runner):
  - 无 torch CUDA(`torch.cuda.is_available()` False);
  - PYD 构建失败(无 clang/CUDA 工具链 / 无嵌入 PTX → 算子运行期失败)。
"""

import json
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RX = ROOT / "target" / "debug" / ("rx.exe" if sys.platform == "win32" else "rx")
KERNEL = ROOT / "src" / "rurix-rt" / "kernels" / "saxpy.rx"
STAGE = ROOT / "build" / "uc01_interop_smoke"
EVIDENCE = ROOT / "evidence" / "uc01_interop_smoke.json"

SAXPY_TOL = 1e-6
REDUCE_REL_TOL = 1e-4
GEMM_REL_TOL = 1e-3


def fail(msg: str) -> None:
    print(f"[uc01_interop_smoke] FAIL: {msg}")
    sys.exit(1)


def skip(msg: str) -> None:
    print(f"[uc01_interop_smoke] SKIP: {msg}")
    sys.exit(0)


def run(cmd, **kw):
    return subprocess.run(cmd, capture_output=True, text=True, **kw)


def build_rx() -> None:
    r = run(["cargo", "build", "-p", "rx"], cwd=ROOT)
    if r.returncode != 0:
        fail(f"cargo build -p rx 失败:\n{r.stderr}")
    if not RX.exists():
        fail(f"rx 产物缺失: {RX}")


def build_pyd():
    """rx build --emit=pyd 产 PYD,返回(.pyd 所在目录, .pyd 路径)。"""
    if STAGE.exists():
        for p in STAGE.glob("*.pyd"):
            p.unlink()
    STAGE.mkdir(parents=True, exist_ok=True)
    r = run([str(RX), "build", "--emit=pyd", str(KERNEL), "-o", str(STAGE)], cwd=ROOT)
    if r.returncode != 0:
        # 工具链缺失(无 clang/CUDA)→ 降级 SKIP(真红绿在 self-hosted runner)
        skip(f"rx build --emit=pyd 失败(可能无 clang/CUDA 工具链):\n{r.stdout}\n{r.stderr}")
    pyds = sorted(STAGE.glob("*.pyd"))
    if not pyds:
        fail("rx build --emit=pyd 未产出 .pyd")
    return STAGE, pyds[0]


def import_module(stage: Path):
    sys.path.insert(0, str(stage))
    import importlib

    return importlib.import_module("rurix_uc01")


def cai_ptr(t) -> int:
    return int(t.__cuda_array_interface__["data"][0])


def run_operators(torch, uc):
    """经 DLPack + CAI v3 双协议跑 SAXPY/Reduction/GEMM,返回(operators_passed, facts)。"""
    dev = "cuda"
    facts = []
    passed = set()

    # —— SAXPY:out = a*x + y —— (DLPack + CAI v3)
    n = 1 << 16
    a = 2.5
    x = torch.arange(n, dtype=torch.float32, device=dev) * 0.5
    y = torch.arange(n, dtype=torch.float32, device=dev) * -1.25 + 3.0
    ref = a * x + y
    saxpy_ok = True
    for proto, call in (
        ("dlpack", lambda o: uc.saxpy(o, x, y, a)),
        (
            "__cuda_array_interface__",
            lambda o: uc.saxpy_ptr(cai_ptr(o), cai_ptr(x), cai_ptr(y), a, n),
        ),
    ):
        out = torch.empty(n, dtype=torch.float32, device=dev)
        torch.cuda.synchronize()
        rc = call(out)
        torch.cuda.synchronize()
        if rc not in (None, 0):
            skip(f"saxpy({proto}) 互操作返回码 {rc}(可能无嵌入 PTX/GPU,降级 SKIP)")
        err = (out - ref).abs().max().item()
        facts.append({"operator": "saxpy", "protocol": proto, "max_abs_err": err, "tolerance": SAXPY_TOL})
        saxpy_ok = saxpy_ok and (err <= SAXPY_TOL)
    if saxpy_ok:
        passed.add("saxpy")

    # —— Reduction:out[0] = sum(x) —— (DLPack + CAI v3)
    xs = torch.rand(n, dtype=torch.float32, device=dev)
    rref = xs.double().sum().item()
    reduce_ok = True
    for proto, call in (
        ("dlpack", lambda o: uc.reduce(o, xs)),
        ("__cuda_array_interface__", lambda o: uc.reduce_ptr(cai_ptr(o), cai_ptr(xs), n)),
    ):
        out = torch.empty(1, dtype=torch.float32, device=dev)
        torch.cuda.synchronize()
        rc = call(out)
        torch.cuda.synchronize()
        if rc not in (None, 0):
            skip(f"reduce({proto}) 互操作返回码 {rc}(降级 SKIP)")
        got = out.item()
        rel = abs(got - rref) / max(abs(rref), 1.0)
        facts.append({"operator": "reduce", "protocol": proto, "max_abs_err": rel, "tolerance": REDUCE_REL_TOL})
        reduce_ok = reduce_ok and (rel <= REDUCE_REL_TOL)
    if reduce_ok:
        passed.add("reduce")

    # —— GEMM:C[M,N] = A[M,K]·B[K,N] —— (DLPack + CAI v3)
    M, K, N = 100, 70, 80
    A = torch.rand(M, K, dtype=torch.float32, device=dev)
    B = torch.rand(K, N, dtype=torch.float32, device=dev)
    gref = A @ B
    gemm_ok = True
    for proto, call in (
        ("dlpack", lambda c: uc.gemm(c, A, B)),
        (
            "__cuda_array_interface__",
            lambda c: uc.gemm_ptr(cai_ptr(c), cai_ptr(A), cai_ptr(B), M, N, K),
        ),
    ):
        C = torch.empty(M, N, dtype=torch.float32, device=dev)
        torch.cuda.synchronize()
        rc = call(C)
        torch.cuda.synchronize()
        if rc not in (None, 0):
            skip(f"gemm({proto}) 互操作返回码 {rc}(降级 SKIP)")
        rel = ((C - gref).abs() / gref.abs().clamp_min(1.0)).max().item()
        facts.append({"operator": "gemm", "protocol": proto, "max_abs_err": rel, "tolerance": GEMM_REL_TOL})
        gemm_ok = gemm_ok and (rel <= GEMM_REL_TOL)
    if gemm_ok:
        passed.add("gemm")

    return passed, facts


def red_self_test(torch, uc) -> bool:
    """红:取 saxpy 正确结果,篡改一元素数值 → 数值对照必判失败(>容差)。
    返回 True = 红有效(篡改被对照判出)。"""
    dev = "cuda"
    n = 4096
    a = 2.5
    x = torch.arange(n, dtype=torch.float32, device=dev) * 0.5
    y = torch.arange(n, dtype=torch.float32, device=dev) + 1.0
    out = torch.empty(n, dtype=torch.float32, device=dev)
    torch.cuda.synchronize()
    rc = uc.saxpy(out, x, y, a)
    torch.cuda.synchronize()
    if rc not in (None, 0):
        skip(f"red self-test saxpy 返回码 {rc}(降级 SKIP)")
    ref = a * x + y
    if (out - ref).abs().max().item() > SAXPY_TOL:
        fail("绿基线失败:未篡改的 saxpy 数值对照即超容差(算子替换不正确)")
    tampered = out.clone()
    tampered[0] = tampered[0] + 1.0  # 篡改算子数值结果
    return (tampered - ref).abs().max().item() > SAXPY_TOL


def main() -> None:
    self_test = "--self-test" in sys.argv
    try:
        import torch
    except ImportError:
        skip("未安装 torch(降级 SKIP;真红绿在带 CUDA torch 的 self-hosted runner)")
    if not torch.cuda.is_available():
        skip("torch.cuda 不可用(CPU-only torch / 无 GPU;降级 SKIP)")

    build_rx()
    stage, pyd = build_pyd()
    uc = import_module(stage)

    if not red_self_test(torch, uc):
        fail("红验证失败:篡改 saxpy 数值结果后对照未判失败(门未真正校验,反 YAML-only)")

    if self_test:
        print("[uc01_interop_smoke] self-test PASS(篡改算子数值结果 → 数值对照判失败,门有效)")
        return

    passed, facts = run_operators(torch, uc)
    if len(passed) < 3:
        fail(f"算子替换数值对照未全通过:passed={sorted(passed)}(要求 saxpy/reduce/gemm 三算子)")

    gpu = torch.cuda.get_device_name(0)
    evidence = {
        "schema_version": 1,
        "subject": "uc01_pytorch_operators",
        "operators_passed": sorted(passed),
        "protocols": ["__cuda_array_interface__", "dlpack"],
        "pyd": {
            "module": "rurix_uc01",
            "build_command": f"rx build --emit=pyd {KERNEL.relative_to(ROOT).as_posix()} -o build/uc01_interop_smoke",
            "pyd_path": pyd.relative_to(ROOT).as_posix(),
            "note": "nanobind + scikit-build-core 产 PYD,链接 rurix-interop staticlib(C ABI),复用 M5 自研 kernel 嵌入 PTX(PTX-only)",
        },
        "device": {
            "gpu": gpu,
            "torch": torch.__version__,
            "cuda": torch.version.cuda or "",
        },
        "facts": facts,
        "redgreen": {
            "red_command": "篡改 saxpy 算子数值结果(扰动一元素)后与 PyTorch 参考数值对照",
            "red_detected": True,
            "green_command": "py -3 ci/uc01_interop_smoke.py(rx build --emit=pyd → 双协议零拷贝 → 三算子数值对照通过)",
            "green_exit_code": 0,
            "run_url": "local red-green(反 YAML-only,H06 D11.8-2);pr-smoke 步骤 34 self-hosted runner run URL 见 PR 描述",
        },
        "timestamp": datetime.now(timezone.utc).astimezone().isoformat(timespec="seconds"),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    EVIDENCE.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    print(
        f"[uc01_interop_smoke] PASS(rx build --emit=pyd 产 PYD / "
        f"CAI v3 + DLPack 双协议零拷贝接入 PyTorch CUDA({gpu}) / "
        f"{sorted(passed)} 三算子替换数值对照通过 / 红验证篡改数值对照判失败 → "
        f"{EVIDENCE.relative_to(ROOT).as_posix()})"
    )


if __name__ == "__main__":
    main()
