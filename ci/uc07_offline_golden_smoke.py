#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""UC-07 离线 golden 冒烟(MS1 CI_GATES §2 步骤 53,契约 G-MS1-3/G-MS1-4,RFC-0010 §4.1/§4.4)。

机器复核闸门(反 YAML-only,主语言判据机器面 G-MS1-3 + 三层 golden G-MS1-4:
宿主帧循环/资源编排/出图落盘与 kernel 全部 .rx 同包经 `rx build` 产 EXE——
手写 Rust/C++ 宿主 harness / host-only 模拟 / 桩化 launch / SKIP 充绿均不得替代):

  前置审计(G-MS1-3 机器面,host 段总跑,无 GPU 也跑)——
      ① 零杂源:遍历 apps/ruridrop,文件集仅 .rx + rurix.toml(发现 .rs/.py/.cpp/
         .c/.h 等任何其他源即红,列出违例;RFC-0010 §4.1 ①);
      ② 同源单包:src/*.rx 集合中存在 `kernel fn` 定义(SPH/渲染 kernel 与宿主
         编排同包,RFC-0010 §4.1 ②)+ offline_smoke.rx / refcpu.rx 双入口在位;
      ③ 产物链路:cargo build -p rurixc -p rx 后,两 EXE 均经 `rx build` 产出
         (语言基础设施白名单之外零 native 胶水,RFC-0010 §4.1 ③④)。

  device 段(真 GPU;探测 = CUDA_PATH + ptxas,抄 ci/host_orch_smoke.py;冒烟档
  N=4096 / 160×120 / 8spp / 2 帧)——
      ① 确定性硬门:offline_smoke 独立 tmp 子目录跑两次 → 逐帧量化 PPM 字节
         SHA-256 相等(G-MS1-4 ①);
      ② 参考容差硬门:refcpu 入口(同一 .rx device fn host 重放)跑一次 →
         GPU 帧 vs ref 帧逐像素字节比较:|Δ|≤1 占比 ≥99.5% 且 max ≤2
         (P6 头解析后纯像素字节,G-MS1-4 ②);
      ③ blessed 哈希软门:逐帧 SHA-256 == tests/uc07/golden_manifest;失配红并
         提示重 bless 路径(`RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py`
         重写 manifest + tests/uc07/bless_log.md 留痕;bless 后仍走完全部硬门);
      ④ 数据流红绿(内建,反 YAML-only):拷贝 src 到 tmp,篡改 params_smoke.rx
         传给 sim_forces 的重力常数 GRAVITY 10.0→2.5 → 同一 rx build 链重编 →
         跑 → 帧 digest ≠ golden(变红)→ 丢弃变体(原树 0-byte 未动,run1
         digest == golden 即复原绿;镜像步骤 48 数据流红绿先例)。
      本机无 CUDA → 降级 SKIP(打印 skip 原因;**RURIX_REQUIRE_REAL=1 时不许
      SKIP**,runner 置位)。

evidence/uc07_offline_golden_smoke.json **仅 device 段真跑全绿时写**(digest_match=true
计入 ms1.counter.uc07_offline_golden_frames,经
milestones/ms1/uc07_offline_golden_evidence_schema.json 校验)。退出码:全绿 0;任何失败 1。
"""
import datetime
import hashlib
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RX = ROOT / "target" / "debug" / ("rx.exe" if os.name == "nt" else "rx")
APP = ROOT / "apps" / "ruridrop"
SRC = APP / "src"
MANIFEST = ROOT / "tests" / "uc07" / "golden_manifest"
EVIDENCE = ROOT / "evidence" / "uc07_offline_golden_smoke.json"

# 冒烟档帧清单(offline_smoke.rx 写 cwd;refcpu.rx 同档产 ref_*)。
FRAMES = ["frame_0000.ppm", "frame_0001.ppm"]
REF_FRAMES = ["ref_0000.ppm", "ref_0001.ppm"]

# ④ 数据流红绿篡改锚点(apps/ruridrop/src/params_smoke.rx 原文):
# offline_smoke.rx 把 params_smoke::GRAVITY 经标量实参传给 sim::sim_forces,
# 篡改为明显不同值 → 力场变 → 帧 digest 必变(证 golden 键于数据流非「跑完了」)。
TAMPER_ANCHOR = "pub const GRAVITY: f32 = 10.0;"
TAMPER_REPLACEMENT = "pub const GRAVITY: f32 = 2.5;"

# 参考容差硬门阈值(G-MS1-4 ②,RFC-0010 §4.4)。
TOL_RATIO = 0.995
TOL_MAX = 2


def run(cmd, cwd=ROOT, timeout=1800, **kw):
    """跑子进程,输出按 UTF-8 宽容解码(rx 诊断含中文;EXE stdout 为 ASCII 见证行)。

    timeout 兜底防僵尸进程锁 runner(nightly 无 timeout 事故先例)。"""
    r = subprocess.run(cmd, capture_output=True, cwd=cwd, timeout=timeout, **kw)
    out = r.stdout.decode("utf-8", errors="replace")
    err_text = r.stderr.decode("utf-8", errors="replace")
    return r.returncode, out, err_text


def skip(msg):
    print(f"[uc07_offline_golden] SKIP {msg}(降级 SKIP,退出 0)")
    sys.exit(0)


def fail(msg):
    print(f"[uc07_offline_golden] FAIL {msg}", file=sys.stderr)
    sys.exit(1)


def audit_main_language():
    """前置审计(G-MS1-3 机器面):零杂源 + kernel 同包 + 双入口在位。"""
    # ① 零杂源:apps/ruridrop 文件集仅 .rx + rurix.toml(任何其他源即红)。
    violations = []
    rx_files = []
    for p in sorted(APP.rglob("*")):
        if p.is_dir():
            continue
        rel = p.relative_to(APP).as_posix()
        if rel == "rurix.toml":
            continue
        if p.suffix == ".rx":
            rx_files.append(rel)
            continue
        violations.append(rel)
    if violations:
        fail(
            "零 .rs 审计违例——apps/ruridrop 存在非 .rx 源(G-MS1-3,RFC-0010 §4.1 ①):\n  "
            + "\n  ".join(violations)
        )
    if not rx_files:
        fail("apps/ruridrop 无任何 .rx 源(应用不存在?)")
    print(f"[uc07_offline_golden] audit ✓ ①零杂源:apps/ruridrop 文件集仅 .rx + rurix.toml"
          f"({len(rx_files)} 个 .rx,零 .rs/.cpp/.c/.py)")

    # ② 同源单包:src/*.rx 集合中存在 kernel fn 定义 + 双入口在位。
    kernel_files = []
    n_kernels = 0
    for p in sorted(SRC.glob("*.rx")):
        text = p.read_bytes().decode("utf-8")
        cnt = text.count("kernel fn ")
        if cnt:
            kernel_files.append(p.name)
            n_kernels += cnt
    if not kernel_files:
        fail("src/*.rx 集合中不存在 kernel fn 定义(入口与 kernel 不同包?G-MS1-3)")
    for entry in ("offline_smoke.rx", "refcpu.rx"):
        if not (SRC / entry).is_file():
            fail(f"入口缺失: apps/ruridrop/src/{entry}")
    print(f"[uc07_offline_golden] audit ✓ ②kernel 同包:src/*.rx 含 {n_kernels} 处 kernel fn"
          f"({', '.join(kernel_files)});offline_smoke.rx/refcpu.rx 双入口在位")


def build_exes(td: Path) -> tuple[Path, Path]:
    """③ 产物链路 = rx build(镜像 ci/host_orch_smoke.py 定位惯例)。"""
    code, out, err_text = run(["cargo", "build", "-p", "rurixc", "-p", "rx"])
    if code != 0:
        # 区分编译错误(红)vs 无工具链(SKIP,镜像 host_orch_smoke)。
        if "error[" in err_text or "error:" in err_text:
            fail(f"cargo build -p rurixc -p rx 编译失败:\n{err_text[-900:]}")
        skip(f"cargo build -p rurixc -p rx 失败(无工具链?):\n{err_text[-500:]}")
    if not RX.is_file():
        fail(f"rx 产物不存在: {RX}")
    exes = {}
    for entry in ("offline_smoke", "refcpu"):
        src = SRC / f"{entry}.rx"
        exe = td / f"{entry}.exe"
        code, out, err_text = run([str(RX), "build", str(src), "-o", str(exe)])
        if code != 0:
            fail(f"rx build {entry}.rx 失败(exit={code}):\n{(out + err_text)[-600:]}")
        if not exe.is_file() or exe.stat().st_size == 0:
            fail(f"{entry} EXE 缺失或为空: {exe}")
        exes[entry] = exe
    print(f"[uc07_offline_golden] audit ✓ ③产物链路:offline_smoke/refcpu 两 EXE 均经"
          f" rx build 产出(防降级硬门 G-MS1-3;kernel PTX 嵌入单 EXE,RFC-0009)")
    return exes["offline_smoke"], exes["refcpu"]


def probe_cuda():
    """device 可用性探测(抄 ci/host_orch_smoke.py:CUDA_PATH + ptxas)。"""
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


def run_frames(exe: Path, workdir: Path, names: list[str], want: str) -> dict[str, bytes]:
    """在独立子目录跑入口 EXE,校验见证行 + 帧齐备,返回 name → 帧字节。"""
    workdir.mkdir(parents=True, exist_ok=True)
    code, out, err_text = run([str(exe)], cwd=str(workdir))
    if code != 0:
        fail(f"{exe.name} 真跑失败(exit={code}):\n{(out + err_text)[-500:]}")
    if want not in out:
        fail(f"{exe.name} stdout 缺见证行 {want!r}:\n{out[-300:]}")
    frames = {}
    for name in names:
        p = workdir / name
        if not p.is_file() or p.stat().st_size == 0:
            fail(f"{exe.name} 未产出帧 {name}(cwd={workdir})")
        frames[name] = p.read_bytes()
    return frames


def parse_p6(name: str, raw: bytes) -> tuple[int, int, bytes]:
    """解析 PPM P6 确定性头(RXS-0116:`P6\\n<w> <h>\\n255\\n` + 纯像素字节)。"""
    if not raw.startswith(b"P6\n"):
        fail(f"{name} 非 P6 头(RXS-0116): {raw[:16]!r}")
    nl2 = raw.index(b"\n", 3)
    nl3 = raw.index(b"\n", nl2 + 1)
    dims = raw[3:nl2].split()
    if len(dims) != 2 or raw[nl2 + 1:nl3] != b"255":
        fail(f"{name} P6 头字段非法: {raw[:nl3 + 1]!r}")
    w, h = int(dims[0]), int(dims[1])
    pixels = raw[nl3 + 1:]
    if len(pixels) != w * h * 3:
        fail(f"{name} 像素字节数 {len(pixels)} != {w}*{h}*3")
    return w, h, pixels


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def determinism_gate(offline: Path, td: Path) -> dict[str, str]:
    """① 确定性硬门:两次独立运行逐帧 SHA-256 相等。返回 name → digest。"""
    run1 = run_frames(offline, td / "run1", FRAMES, "RENDER_OK frames=2")
    run2 = run_frames(offline, td / "run2", FRAMES, "RENDER_OK frames=2")
    digests = {}
    for name in FRAMES:
        d1, d2 = sha256(run1[name]), sha256(run2[name])
        if d1 != d2:
            fail(f"确定性硬门红:{name} 两次运行 digest 不等\n  run1={d1}\n  run2={d2}")
        digests[name] = d1
        print(f"[uc07_offline_golden] determinism ✓ ①{name} 两跑 SHA-256 一致 {d1}")
    return digests


def tolerance_gate(offline_dir: Path, refcpu: Path, td: Path) -> tuple[float, int]:
    """② 参考容差硬门:GPU 帧 vs refcpu 帧,|Δ|≤1 占比 ≥99.5% 且 max ≤2。"""
    refs = run_frames(refcpu, td / "ref", REF_FRAMES, "REFCPU_OK")
    worst_ratio, worst_max = 1.0, 0
    for gpu_name, ref_name in zip(FRAMES, REF_FRAMES):
        gw, gh, gpix = parse_p6(gpu_name, (offline_dir / gpu_name).read_bytes())
        rw, rh, rpix = parse_p6(ref_name, refs[ref_name])
        if (gw, gh) != (rw, rh):
            fail(f"容差硬门红:{gpu_name} {gw}x{gh} vs {ref_name} {rw}x{rh} 尺寸不等")
        total = len(gpix)
        within = 0
        maxd = 0
        for a, b in zip(gpix, rpix):
            d = a - b if a >= b else b - a
            if d <= 1:
                within += 1
            if d > maxd:
                maxd = d
        ratio = within / total
        if ratio < TOL_RATIO or maxd > TOL_MAX:
            fail(f"容差硬门红:{gpu_name} vs {ref_name} |Δ|≤1 占比 {ratio:.6f}"
                 f"(要求 ≥{TOL_RATIO})/ max {maxd}(要求 ≤{TOL_MAX})")
        print(f"[uc07_offline_golden] tolerance ✓ ②{gpu_name} vs {ref_name}:"
              f"|Δ|≤1 占比 {ratio:.6f}(≥{TOL_RATIO}),max {maxd}(≤{TOL_MAX})")
        worst_ratio = min(worst_ratio, ratio)
        worst_max = max(worst_max, maxd)
    return worst_ratio, worst_max


def manifest_gate(digests: dict[str, str]) -> None:
    """③ blessed 哈希软门:逐帧 SHA-256 == tests/uc07/golden_manifest。

    RURIX_BLESS_UC07=1 → 以本次 device 真跑 digest 重写 manifest(bless 留痕须同 PR
    在 tests/uc07/bless_log.md 追加;bless 后仍走完全部硬门)。"""
    if os.environ.get("RURIX_BLESS_UC07") == "1":
        MANIFEST.parent.mkdir(parents=True, exist_ok=True)
        lines = "".join(f"{digests[name]}  {name}\n" for name in FRAMES)
        with open(MANIFEST, "wb") as f:
            f.write(lines.encode("utf-8"))
        print(f"[uc07_offline_golden] bless ✓ RURIX_BLESS_UC07=1 重写"
              f" {MANIFEST.relative_to(ROOT)}({len(FRAMES)} 帧;留痕 tests/uc07/bless_log.md)")
    if not MANIFEST.is_file():
        fail("tests/uc07/golden_manifest 缺失——首次 bless:"
             "`RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py`(留痕 bless_log.md)")
    blessed = {}
    for lineno, line in enumerate(MANIFEST.read_bytes().decode("utf-8").splitlines(), start=1):
        if not line.strip():
            continue
        parts = line.split()
        if len(parts) != 2:
            fail(f"golden_manifest 第 {lineno} 行格式非法(应为 `<sha256>  <frame>`): {line!r}")
        blessed[parts[1]] = parts[0]
    for name in FRAMES:
        if name not in blessed:
            fail(f"golden_manifest 缺帧 {name}——重 bless:"
                 "`RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py`")
        if blessed[name] != digests[name]:
            fail(f"blessed 哈希软门红:{name} 漂移\n  blessed={blessed[name]}\n"
                 f"  actual ={digests[name]}\n  确认漂移合法(如驱动升级)后重 bless:"
                 "`RURIX_BLESS_UC07=1 py -3 ci/uc07_offline_golden_smoke.py`"
                 "(bless_log.md 同 PR 追加留痕)")
        print(f"[uc07_offline_golden] golden ✓ ③{name} SHA-256 == golden_manifest")


def dataflow_red(td: Path, digests: dict[str, str]) -> None:
    """④ 数据流红绿(内建):篡改 GRAVITY 经同一 rx build 链重编 → digest 必变红。"""
    variant_dir = td / "tamper_src"
    shutil.copytree(SRC, variant_dir)
    params = variant_dir / "params_smoke.rx"
    text = params.read_bytes().decode("utf-8")
    if TAMPER_ANCHOR not in text:
        fail(f"无法构造篡改变体:params_smoke.rx 未含锚点 {TAMPER_ANCHOR!r}")
    with open(params, "wb") as f:
        f.write(text.replace(TAMPER_ANCHOR, TAMPER_REPLACEMENT).encode("utf-8"))
    exe = td / "offline_tampered.exe"
    code, out, err_text = run([str(RX), "build", str(variant_dir / "offline_smoke.rx"),
                               "-o", str(exe)])
    if code != 0:
        fail(f"篡改变体 rx build 失败(变体须合法编译,exit={code}):\n{(out + err_text)[-500:]}")
    tampered = run_frames(exe, td / "tamper_run", FRAMES, "RENDER_OK frames=2")
    for name in FRAMES:
        td_digest = sha256(tampered[name])
        if td_digest == digests[name]:
            fail(f"数据流红绿失效:篡改 GRAVITY 后 {name} digest 仍 == golden({td_digest})"
                 "——golden 未键于物理数据流(仅「跑完了」不接受)")
    print(f"[uc07_offline_golden] red ✓ ④篡改 sim_forces 重力常数(GRAVITY 10.0→2.5,"
          f"同一 rx build 链重编)→ 逐帧 digest ≠ golden(变红);变体已弃,原树 0-byte"
          f" 未动 → run1 digest == golden 即复原绿")


def write_evidence(adapter: str, ratio: float, maxd: int):
    """仅 device 段真跑全绿时写 evidence(schema:milestones/ms1/uc07_offline_golden_evidence_schema.json)。"""
    doc = {
        "schema_version": 1,
        "kind": "uc07_offline_golden",
        "date": datetime.datetime.now().astimezone().replace(microsecond=0).isoformat(),
        "zero_rs_audit": True,
        "frames": len(FRAMES),
        "determinism_pass": True,
        "tolerance": {"ratio": ratio, "max": maxd},
        "digest_match": True,
        "dataflow_red": True,
        "adapter": adapter,
        "notes": (
            "apps/ruridrop 全 .rx 主语言审计(零 .rs/.cpp/.c/.py + kernel 同包 + 两 EXE 经"
            " rx build 产出,G-MS1-3)+ 离线 golden 三层(冒烟档 N=4096/160×120/8spp/2 帧:"
            "①同机两跑逐帧 SHA-256 一致 ②GPU vs refcpu 量化域 |Δ|≤1 占比与 max 达标 "
            "③逐帧 SHA-256 == tests/uc07/golden_manifest)+ ④篡改 sim_forces 重力常数经"
            "同一 rx build 链重编 digest 变红、原树复原绿"
            "(G-MS1-4,RFC-0010 §4.1/§4.4)"
        ),
    }
    EVIDENCE.parent.mkdir(parents=True, exist_ok=True)
    with open(EVIDENCE, "wb") as f:
        f.write((json.dumps(doc, ensure_ascii=False, indent=2) + "\n").encode("utf-8"))
    print(f"[uc07_offline_golden] evidence 写入 {EVIDENCE.relative_to(ROOT)}")


def main():
    audit_main_language()
    with tempfile.TemporaryDirectory(prefix="uc07_offline_golden_") as tdname:
        td = Path(tdname)
        offline, refcpu = build_exes(td)
        require_real = os.environ.get("RURIX_REQUIRE_REAL") == "1"
        if not probe_cuda():
            if require_real:
                fail("RURIX_REQUIRE_REAL=1 但缺 CUDA_PATH / ptxas(device 段不许 SKIP,runner 置位)")
            print("[uc07_offline_golden] device 段 SKIP:无 CUDA_PATH / ptxas(本机无 CUDA"
                  " 工具链/GPU)——不写 evidence,ms1.counter.uc07_offline_golden_frames"
                  " 为建设期 normal SKIP")
            print("[uc07_offline_golden] PASS 前置审计(零杂源 + kernel 同包 + rx build 链);device 段 SKIP")
            sys.exit(0)
        adapter = adapter_name()
        digests = determinism_gate(offline, td)
        ratio, maxd = tolerance_gate(td / "run1", refcpu, td)
        manifest_gate(digests)
        dataflow_red(td, digests)
        print(f"[uc07_offline_golden] UC07_GOLDEN: ok zero_rs=true frames={len(FRAMES)}"
              f" digest_match=true adapter=\"{adapter}\"")
    write_evidence(adapter, ratio, maxd)
    print("[uc07_offline_golden] PASS 前置审计 + device 真跑三层 golden(①确定性 ②参考容差"
          " ③blessed 哈希)+ ④数据流红绿(篡改重力 → digest 红 → 原树复原绿)")
    sys.exit(0)


if __name__ == "__main__":
    main()
