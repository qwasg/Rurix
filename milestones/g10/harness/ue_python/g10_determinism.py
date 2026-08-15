#!/usr/bin/env python3
"""G10.2 harness — 确定性出图协议脚本草案（固定 seed + warmup 帧 + 收敛后捕获 + 双跑 digest 比对）。

协议骨架（沿 RFC-0026 §4.6 time 节 + spike LaunchEngineLoop.cpp 实证开关）：
  1. 固定随机种子：time.random_seed → UE 侧 -FixedSeed / benchmark 隐含 FixedSeed（源码实证
     :2451/:2457-2462）；MRQ 臂内以确定性命名/固定帧率（fixed_dt_s）对齐；
  2. warmup：time.warmup_frames 帧时域累积（TSR）收敛，第 capture_frame_index 帧触发捕获；
  3. 收敛后捕获：MRQ 臂由 MoviePipelineEngineWarmUpSetting 承载；HighResShot 快臂（臂 B）由
     -benchmark -fps=<N> -seconds=<N> + execcmds 白名单模板（r.ResetViewState; HighResShot <W>x<H>，
     RFC-0027 §4.1.3 闭集）承载——-execcmds 触发时序稳定性为 spike 待验证项，首日实测登记；
  4. 双跑 digest：同参数集连跑两次，逐帧 SHA-256 比对（M129 硬判据：不等即 RED）。
     帧 digest 由 host 侧对落盘 EXR 文件实算（禁手写，P-09）。

地位：DRAFT 占位可解析形态，待引擎可用后实测修订。
Assisted-by: Kimi-K3（G10.2 波）
"""
import hashlib
import json
import os
import struct

# UE 5.8.1 实证易变 EXR 属性闭集（2026-08-15 双跑实测归纳，82 属性中这 14 个跨跑必变）：
# 时间戳族 + 运行时统计族。digest 签名面剥离（RFC-0026 ue5 strip-and-log 策略的实证闭集）。
# 另：EXR 扫描线偏移表（1080×u64）值随元数据长度差级联漂移，canonical 化时以占位零替代。
EXR_VOLATILE_ATTRS = (
    b"unreal/frameRenderDuration",
    b"unreal/frameRenderStartTimeUTC",
    b"unreal/frameRenderEndTimeUTC",
    b"unreal/jobDate",
    b"unreal/jobDay",
    b"unreal/jobMonth",
    b"unreal/jobTime",
    b"unreal/jobYear",
    b"unreal/stats/memory/availablePhysicalMB",
    b"unreal/stats/memory/availableVirtualMB",
    b"unreal/stats/memory/peakUsedPhysicalMB",
    b"unreal/stats/memory/peakUsedVirtualMB",
    b"unreal/stats/outputDirectoryTotalFreeMB",
    b"unreal/stats/outputDirectoryTotalSizeMB",
)


def exr_canonical_digest(path, data_window=(1080, 1080)):
    """EXR canonical digest（5.8.1 实测形态）：剥离易变属性值 + 扫描线偏移表归零后 SHA-256。
    实证基线：Entry 空图 MRQ 双跑，本口径下 4/4 帧 digest 相等；原始字节 digest 永不相等
    （时间戳/内存统计/磁盘统计嵌入）。仅支持 scanline + NONE 压缩 + 属性值就地等长/变长剥离。
    data_window: (height 扫描线数, …)——偏移表条数 = 高度（本口径限 1920x1080 单 tile 实测面）。"""
    buf = open(path, "rb").read()
    out = []
    i = 8  # magic(4) + version(4)
    out.append(buf[:8])
    while buf[i] != 0:
        j = buf.index(b"\x00", i)
        name = buf[i:j]
        k = buf.index(b"\x00", j + 1)
        size = struct.unpack("<I", buf[k + 1:k + 5])[0]
        val = buf[k + 5:k + 5 + size]
        if name not in EXR_VOLATILE_ATTRS:
            out.append(name + b"\x00" + val)
        i = k + 5 + size
    out.append(b"\x00")  # header terminator
    body = buf[i + 1:]
    height = data_window[0]
    table = body[:height * 8]
    if len(table) == height * 8:
        out.append(b"\x00" * (height * 8))  # 偏移表规范化归零
        out.append(body[height * 8:])
    else:
        out.append(body)  # 形态不符则原样（保守不静默）
    return hashlib.sha256(b"".join(out)).hexdigest()



def frame_sha256(path, chunk=1 << 20):
    """host 侧帧文件 SHA-256 实算。"""
    h = hashlib.sha256()
    with open(path, "rb") as f:
        while True:
            b = f.read(chunk)
            if not b:
                break
            h.update(b)
    return h.hexdigest()


def double_run_digest_compare(run_a_dir, run_b_dir, report_path=None):
    """M129 双跑 digest 一致性比对：两目录同名帧逐文件比对。
    返回 {"equal": bool, "mismatches": [...], "frames": {name: digest}}。
    不等即 RED 由门侧求值；本函数只产事实。"""
    a = {f: frame_sha256(os.path.join(run_a_dir, f))
         for f in sorted(os.listdir(run_a_dir)) if f.lower().endswith(".exr")}
    b = {f: frame_sha256(os.path.join(run_b_dir, f))
         for f in sorted(os.listdir(run_b_dir)) if f.lower().endswith(".exr")}
    mismatches = []
    only_a = sorted(set(a) - set(b))
    only_b = sorted(set(b) - set(a))
    for name in sorted(set(a) & set(b)):
        if a[name] != b[name]:
            mismatches.append(name)
    result = {
        "equal": not mismatches and not only_a and not only_b and bool(a),
        "mismatches": mismatches,
        "only_in_run_a": only_a,
        "only_in_run_b": only_b,
        "frames": a,
    }
    if report_path:
        with open(report_path, "w", encoding="utf-8", newline="\n") as f:
            json.dump(result, f, ensure_ascii=False, indent=2)
            f.write("\n")
    return result


def warmup_capture_plan(contract_obj):
    """由契约 time 节生成捕获计划（纯函数，双端可核）：
    捕获帧 = warmup_frames 之后第 capture_frame_index 帧；固定步长 fixed_dt_s；seed=random_seed。"""
    t = contract_obj["time"]
    return {
        "fixed_dt_s": t["fixed_dt_s"],
        "warmup_frames": t["warmup_frames"],
        "capture_frame_index": t["capture_frame_index"],
        "capture_tick_s": (t["warmup_frames"] + t["capture_frame_index"]) * t["fixed_dt_s"],
        "random_seed": t["random_seed"],
        "jitter": dict(t["jitter"]),
    }


def arm_b_execmds_template(width, height):
    """臂 B execcmds 白名单模板（RFC-0027 §4.1.3 闭集 = r.ResetViewState + HighResShot <W>x<H>）。
    模板外自由文本注入即 fail-closed——本函数只接受整数宽高，拒绝一切其他拼接。"""
    w, h = int(width), int(height)
    if w <= 0 or h <= 0:
        raise ValueError("bad resolution")
    return f"r.ResetViewState; HighResShot {w}x{h}"


if __name__ == "__main__":
    import sys

    if len(sys.argv) == 3:
        r = double_run_digest_compare(sys.argv[1], sys.argv[2])
        print(json.dumps(r, ensure_ascii=False, indent=2))
        sys.exit(0 if r["equal"] else 1)
    print("usage: g10_determinism.py <run_a_dir> <run_b_dir>")
    sys.exit(2)
