#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""single-source 宿主编排冒烟(MS1 CI_GATES §2 步骤 52,契约 G-MS1-2,RFC-0009 / RXS-0189~0196)。

机器复核闸门(反 YAML-only,防降级硬门 G-MS1-2:宿主编排必须来自 .rx 源经 rurixc host
codegen 产出,手写宿主 harness / host-only 模拟 / 桩化 launch / SKIP 充绿均不得替代):

  host 段(总跑,无 GPU 也跑)——
      ① cargo build -p rurixc -p rx 定位 rx 可执行;
      ② reject 语料编译期拦截:conformance/host_orch/reject/* 逐个 `rx build` 期望非零
         退出 + 诊断含对应 RX 码(mod_missing/mod_cycle → RX1005,RXS-0196;elem_infer →
         RX2010,RXS-0190;gpu_in_kernel → RX3015,RXS-0189;launch_arg_subset → RX6024,
         RXS-0191;buffer_move → RX4001,RXS-0189);
      ③ accept 语料 `rx build` 全绿(mod_file / extern_link / saxpy_single_source;
         EXE 落 %TEMP% 子目录,不留仓库);
      ④ 链接面见证:saxpy_single_source 单 EXE 存在且非空(kernel PTX 嵌入 +
         rurix_rt_cabi 链接成功,RXS-0192/0195)。

  device 段(真 GPU;探测 = CUDA_PATH + ptxas,抄 ci/fatbin_dist_smoke.py)——
      运行 saxpy 单 EXE → 真 launch → 数值自校验 exit 0 → 见证行
      `HOST_ORCH: ok single_source=true device_run=true`;本机无 CUDA → 降级 SKIP
      (打印 skip 原因;**RURIX_REQUIRE_REAL=1 时不许 SKIP**,runner 置位)。

  red→绿闭合(内建,反 YAML-only;仅 device 可用时执行,与 device 段同门)——
      红① 复制 saxpy EXE,篡改内嵌 PTX 6 字节(非 UTF-8 字节)→ 运行期望非零 +
          stderr 含 `RXRT: error`(装载协商拒,RXS-0192/0193);
      红② 变体源桩化 kernel 写回(`out[i] = ...` 改为纯读不写)经**同一 rx build 链**
          重编 → 编译绿 / 运行期望非零(数值自校验红,证绿不是「跑完了」);
      复原核验:原 EXE 复跑 exit 0。

evidence/host_orch_smoke.json **仅 device 段真跑时写**(single_source=true 且
device_run=true 计入 ms1.counter.host_orch_single_source,经
milestones/ms1/host_orch_smoke_evidence_schema.json 校验)。退出码:全绿 0;任何失败 1。
"""
import datetime
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
CORPUS = ROOT / "conformance" / "host_orch"
EVIDENCE = ROOT / "evidence" / "host_orch_smoke.json"

# reject 语料 → 期望 RX 码(spec/host_orchestration.md §3;//@ expect-error 同源)。
REJECTS = [
    ("mod_missing", "RX1005"),
    ("mod_cycle", "RX1005"),
    ("elem_infer", "RX2010"),
    ("gpu_in_kernel", "RX3015"),
    ("launch_arg_subset", "RX6024"),
    ("buffer_move", "RX4001"),
    # MS1.2b present typestate 面(RXS-0197~0199):错序 = move 违例 / 宿主 API 进 device。
    ("present_out_of_order", "RX4001"),
    ("present_in_kernel", "RX3015"),
]
ACCEPTS = ["mod_file", "extern_link", "saxpy_single_source", "present_loop", "imageio_write"]

# 红② 桩化替换锚点(conformance/host_orch/accept/saxpy_single_source/main.rx 原文):
# kernel 写回语句 → 纯读不写(let 绑定),数值自校验必红。
STUB_ANCHOR = "out[i] = a * x[i] + out[i];"
STUB_REPLACEMENT = "let sink = a * x[i] + out[i];"


def run(cmd, cwd=ROOT, **kw):
    """跑子进程,输出按 UTF-8 宽容解码(rx 诊断含中文;EXE stderr 为 ASCII 诊断行)。"""
    r = subprocess.run(cmd, capture_output=True, cwd=cwd, **kw)
    out = r.stdout.decode("utf-8", errors="replace")
    err_text = r.stderr.decode("utf-8", errors="replace")
    return r.returncode, out, err_text


def skip(msg):
    print(f"[host_orch_smoke] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[host_orch_smoke] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def build_rx():
    """① 构建 rurixc + rx 前端(镜像 ci/rx_cli_smoke.py 定位惯例)。"""
    code, out, err_text = run(["cargo", "build", "-p", "rurixc", "-p", "rx"])
    if code != 0:
        # 区分编译错误(红:MS1.2 前端/链接面坏)vs 无工具链(SKIP,镜像 realtime_present)。
        if "error[" in err_text or "error:" in err_text:
            fail(f"cargo build -p rurixc -p rx 编译失败:\n{err_text[-900:]}")
        skip(f"cargo build -p rurixc -p rx 失败(无工具链?):\n{err_text[-500:]}")
    if not RX.is_file():
        fail(f"rx 产物不存在: {RX}")


def host_segment(td: Path) -> Path:
    """②③④ host 段:reject 编译期拦截 + accept 全绿 + 链接面见证。返回 saxpy EXE 路径。"""
    # ② reject 语料:逐个 rx build 期望非零退出 + 输出含对应 RX 码(编译期拦截)。
    for name, code_want in REJECTS:
        src = CORPUS / "reject" / name / "main.rx"
        exe = td / f"reject_{name}.exe"
        code, out, err_text = run([str(RX), "build", str(src), "-o", str(exe)])
        combined = out + err_text
        if code == 0:
            fail(f"reject/{name} 未拦截(rx build exit 0)——编译期拦截失效")
        if code_want not in combined:
            fail(f"reject/{name} 诊断不含 {code_want}(exit={code}):\n{combined[-400:]}")
        print(f"[host_orch_smoke] reject ✓ {name} → 编译期拦截({code_want})")

    # ③ accept 语料:rx build 全部 exit 0(EXE 落 %TEMP% 子目录,勿留仓库)。
    exes = {}
    for name in ACCEPTS:
        src = CORPUS / "accept" / name / "main.rx"
        exe = td / f"accept_{name}.exe"
        code, out, err_text = run([str(RX), "build", str(src), "-o", str(exe)])
        if code != 0:
            fail(f"accept/{name} rx build 失败(exit={code}):\n{(out + err_text)[-600:]}")
        exes[name] = exe
        print(f"[host_orch_smoke] accept ✓ {name} rx build exit 0")

    # ④ 链接面见证:saxpy 单 EXE 存在且非空(PTX 嵌入 + rurix_rt_cabi 链接成功)。
    saxpy = exes["saxpy_single_source"]
    if not saxpy.is_file() or saxpy.stat().st_size == 0:
        fail(f"saxpy_single_source EXE 缺失或为空: {saxpy}")
    print(f"[host_orch_smoke] link ✓ saxpy_single_source 单 EXE 存在且非空"
          f"({saxpy.stat().st_size} 字节;PTX 嵌入 + rurix_rt_cabi 链接面,RXS-0192/0195)")
    return exes


def probe_cuda():
    """device 可用性探测(抄 ci/fatbin_dist_smoke.py:CUDA_PATH + ptxas)。"""
    cuda_path = os.environ.get("CUDA_PATH")
    if not cuda_path:
        return False
    ptxas = Path(cuda_path) / "bin" / ("ptxas.exe" if os.name == "nt" else "ptxas")
    return ptxas.exists()


def adapter_name() -> str:
    """env 画像最小集:nvidia-smi 名 + 驱动版号;不可得时退回 CUDA_PATH。"""
    try:
        code, out, _ = run(
            ["nvidia-smi", "--query-gpu=name,driver_version", "--format=csv,noheader"]
        )
        if code == 0 and out.strip():
            name, _, drv = out.strip().splitlines()[0].partition(",")
            return f"{name.strip()} (driver {drv.strip()})"
    except OSError:
        pass
    return f"unknown (CUDA_PATH={os.environ.get('CUDA_PATH', '')})"


def tamper_red(saxpy: Path, td: Path):
    """红①:复制 saxpy EXE,篡改内嵌 PTX 6 字节 → 装载协商拒(RXS-0192/0193)。

    篡改字节取 0xFF/0xFE(任何 UTF-8 序列中非法):嵌入描述表解析在 ctx_create 即
    确定性拒绝——与协商变体无关(cubin 命中也拒),红路径确定。"""
    raw = bytearray(saxpy.read_bytes())
    idx = raw.find(b".version")
    if idx < 0:
        idx = raw.find(b".visible .entry")
    if idx < 0:
        fail("saxpy EXE 未定位到内嵌 PTX 文本(搜 .version / .visible .entry 均无)")
    raw[idx:idx + 6] = b"\xff\xfe\xff\xfe\xff\xfe"
    tampered = td / "saxpy_tampered.exe"
    with open(tampered, "wb") as f:
        f.write(bytes(raw))
    code, _, err_text = run([str(tampered)])
    if code == 0:
        fail("篡改内嵌 PTX 后仍 exit 0(装载协商拒失效)——红①失败")
    if "RXRT: error" not in err_text:
        fail(f"篡改内嵌 PTX 后 stderr 不含 `RXRT: error`(exit={code}):\n{err_text[-300:]}")
    print(f"[host_orch_smoke] red ✓ ①篡改内嵌 PTX 6 字节 → 装载协商拒"
          f"(exit={code},RXRT: error,RXS-0192/0193)")


def stub_red(td: Path):
    """红②:变体源桩化 kernel 写回,经同一 rx build 链重编 → 数值自校验红。"""
    src_dir = CORPUS / "accept" / "saxpy_single_source"
    variant_dir = td / "stub_variant"
    shutil.copytree(src_dir, variant_dir)
    main = variant_dir / "main.rx"
    text = main.read_bytes().decode("utf-8")
    if STUB_ANCHOR not in text:
        fail(f"无法构造桩化变体:saxpy_single_source/main.rx 未含锚点 {STUB_ANCHOR!r}")
    with open(main, "wb") as f:
        f.write(text.replace(STUB_ANCHOR, STUB_REPLACEMENT).encode("utf-8"))
    exe = td / "saxpy_stub.exe"
    code, out, err_text = run([str(RX), "build", str(main), "-o", str(exe)])
    if code != 0:
        fail(f"桩化变体 rx build 失败(变体须合法编译,exit={code}):\n{(out + err_text)[-500:]}")
    code, _, _ = run([str(exe)])
    if code == 0:
        fail("桩化 kernel 写回后仍 exit 0——数值自校验未生效(绿只是「跑完了」)")
    print(f"[host_orch_smoke] red ✓ ②桩化 kernel 写回(同一 rx build 链重编)→ 数值自校验红(exit={code})")


def device_segment(exes: dict, td: Path):
    """device 段 + 双红绿闭合(同门)。返回 (device_run, adapter)。"""
    saxpy = exes["saxpy_single_source"]
    require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
    if not probe_cuda():
        if require_real:
            fail("RURIX_REQUIRE_REAL=1 但缺 CUDA_PATH / ptxas(device 段不许 SKIP,runner 置位)")
        print("[host_orch_smoke] device 段 SKIP:无 CUDA_PATH / ptxas(本机无 CUDA 工具链/GPU)"
              "——不写 evidence,ms1.counter.host_orch_single_source 为建设期 normal SKIP")
        return False, ""
    adapter = adapter_name()

    # green:saxpy 单 EXE 真跑 → 真 launch → 数值自校验 exit 0。
    code, _, err_text = run([str(saxpy)])
    if code != 0:
        fail(f"saxpy 单 EXE device 真跑失败(exit={code}):\n{err_text[-500:]}")
    print(f"[host_orch_smoke] HOST_ORCH: ok single_source=true device_run=true adapter=\"{adapter}\"")

    # green(MS1.2b):imageio_write 真跑 → kernel 着色 → download → write_ppm 落盘(RXS-0199)。
    code, _, err_text = run([str(exes["imageio_write"])], cwd=str(td))
    if code != 0:
        fail(f"imageio_write device 真跑失败(exit={code}):\n{err_text[-500:]}")
    ppm = td / "imageio_write_out.ppm"
    if not ppm.is_file() or ppm.stat().st_size == 0 or not ppm.read_bytes().startswith(b"P6\n"):
        fail(f"imageio_write 未产出合法 PPM(RXS-0199/RXS-0116): {ppm}")
    print(f"[host_orch_smoke] imageio ✓ write_ppm 真跑落盘({ppm.stat().st_size} 字节,P6 头,RXS-0199)")

    # red→绿闭合(仅 device 可用时执行)。
    tamper_red(saxpy, td)
    stub_red(td)

    # 复原核验:原 EXE 再跑 exit 0(红绿闭合)。
    code, _, err_text = run([str(saxpy)])
    if code != 0:
        fail(f"复原核验失败:原 EXE 复跑 exit={code}(红绿不闭合):\n{err_text[-300:]}")
    print("[host_orch_smoke] 复原 ✓ 原 EXE 复跑 exit 0(红绿闭合)")
    return True, adapter


def write_evidence(adapter: str):
    """仅 device 段真跑时写 evidence(schema:milestones/ms1/host_orch_smoke_evidence_schema.json)。"""
    doc = {
        "schema_version": 1,
        "kind": "host_orch_smoke",
        "date": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "single_source": True,
        "device_run": True,
        "adapter": adapter,
        "checks": {
            "accept_pass": True,
            "reject_pass": True,
            "tamper_red": True,
            "stub_red": True,
        },
        "notes": (
            "host .rx 经 std::gpu 编排 + 同源 kernel PTX 嵌入单 EXE(rx build 一步出可执行),"
            "device 真跑数值自校验 exit 0 + write_ppm 落盘(RXS-0199);reject 八类编译期拦截"
            "(RX1005/RX2010/RX3015/RX6024/RX4001,含 present 错序/present 进 device)+ "
            "篡改内嵌 PTX 装载协商拒 + 桩化写回数值红 + 复原绿"
            "(G-MS1-2,RFC-0009 / RXS-0189~0199)"
        ),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    with open(EVIDENCE, "wb") as f:
        f.write((json.dumps(doc, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
    print(f"[host_orch_smoke] evidence 写入 {EVIDENCE.relative_to(ROOT)}")


def main():
    build_rx()
    with tempfile.TemporaryDirectory(prefix="host_orch_smoke_") as tdname:
        td = Path(tdname)
        exes = host_segment(td)
        device_run, adapter = device_segment(exes, td)
    if device_run:
        write_evidence(adapter)
        print("[host_orch_smoke] PASS host 段 + device 真跑(saxpy+imageio)+ 红①篡改 PTX + 红②桩化写回 + 复原绿(红绿闭合)")
    else:
        print("[host_orch_smoke] PASS host 段(reject 八类拦截 + accept 五例 + 链接面);device 段 SKIP")
    sys.exit(0)


if __name__ == "__main__":
    main()
